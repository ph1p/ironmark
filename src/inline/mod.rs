mod em_delims;
mod fast_path;
mod links;
mod render;
mod scanner;

use em_delims::*;
use fast_path::*;

use crate::ParseOptions;
use crate::entities;
use crate::html::{encode_url_escaped_into, escape_html_into, is_dangerous_url};
use rustc_hash::FxHashMap;
use std::borrow::Cow;
use std::rc::Rc;

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct LinkReference {
    pub href: Rc<str>,
    pub title: Option<Rc<str>>,
}

pub(crate) type LinkRefMap = FxHashMap<String, LinkReference>;

#[inline]
pub(crate) fn normalize_reference_label(label: &str) -> Cow<'_, str> {
    let trimmed = label.trim();
    let bytes = trimmed.as_bytes();

    {
        let mut simple = true;
        let mut prev_space = false;
        for &b in bytes {
            if b.is_ascii_uppercase() {
                simple = false;
                break;
            }
            if b == b' ' {
                if prev_space {
                    simple = false;
                    break;
                }
                prev_space = true;
            } else if b == b'\t' || b == b'\n' || b == b'\r' || b >= 0x80 {
                simple = false;
                break;
            } else {
                prev_space = false;
            }
        }
        if simple {
            return Cow::Borrowed(trimmed);
        }
    }

    let mut out = String::with_capacity(trimmed.len());
    let mut in_space = false;
    let mut i = 0;
    while i < bytes.len() {
        let b = bytes[i];
        if b < 0x80 {
            if b == b' ' || b == b'\t' || b == b'\n' || b == b'\r' {
                if !in_space {
                    out.push(' ');
                    in_space = true;
                }
                i += 1;
            } else {
                out.push(if b.is_ascii_uppercase() {
                    (b + 32) as char
                } else {
                    b as char
                });
                in_space = false;
                i += 1;
            }
        } else {
            let c = trimmed[i..].chars().next().unwrap();
            let clen = c.len_utf8();
            if c.is_whitespace() {
                if !in_space {
                    out.push(' ');
                    in_space = true;
                }
            } else {
                match c {
                    'ß' | 'ẞ' => out.push_str("ss"),
                    _ => {
                        for lc in c.to_lowercase() {
                            out.push(lc);
                        }
                    }
                }
                in_space = false;
            }
            i += clen;
        }
    }
    Cow::Owned(out)
}

const SPECIAL_ANY: u8 = 1;
const SPECIAL_COMPLEX: u8 = 2;
const SCAN_FULL: u8 = 1;
const SCAN_IS_BACKTICK: u8 = 2;
const SCAN_EMPH: u8 = 4;
const SCAN_BREAK: u8 = 8;
const SCAN_ESCAPE: u8 = 16;

static SPECIAL: [u8; 256] = {
    let mut t = [0u8; 256];
    t[b'*' as usize] = SPECIAL_ANY;
    t[b'_' as usize] = SPECIAL_ANY;
    let both = SPECIAL_ANY | SPECIAL_COMPLEX;
    t[b'\\' as usize] = both;
    t[b'`' as usize] = both;
    t[b'!' as usize] = both;
    t[b'[' as usize] = both;
    t[b']' as usize] = both;
    t[b'<' as usize] = both;
    t[b'&' as usize] = both;
    t[b'\n' as usize] = both;
    t[b'~' as usize] = both;
    t[b'=' as usize] = both;
    t[b'+' as usize] = both;
    t[b':' as usize] = both;
    t[b'@' as usize] = both;
    t[b'$' as usize] = both;
    t
};

#[inline]
pub(crate) fn parse_inline_pass(
    out: &mut String,
    raw: &str,
    refs: &LinkRefMap,
    opts: &ParseOptions,
    bufs: &mut InlineBuffers,
) {
    let bytes = raw.as_bytes();
    let scan_table = &bufs.scan_table;

    let mut has_emphasis = false;
    let mut has_breaks = false;
    let mut needs_html_escape = false;
    let mut requires_full_inline = false;
    let mut backtick_hint = 0u8;
    let mut rescan_from = bytes.len();
    let mut full_only_backticks = true;

    // Fast scan: process 8 bytes at a time, ORing flags together. If no byte
    // triggers SCAN_FULL|SCAN_BREAK|SCAN_EMPH|SCAN_ESCAPE in a chunk we skip it cheaply.
    let mut i = 0;
    // Process 8 bytes per iteration: OR all flags; if any FULL bit set, fall to per-byte.
    const CHUNK: usize = 8;
    while i + CHUNK <= bytes.len() {
        let chunk = &bytes[i..i + CHUNK];
        // OR flags for all 8 bytes
        let flags = scan_table[chunk[0] as usize]
            | scan_table[chunk[1] as usize]
            | scan_table[chunk[2] as usize]
            | scan_table[chunk[3] as usize]
            | scan_table[chunk[4] as usize]
            | scan_table[chunk[5] as usize]
            | scan_table[chunk[6] as usize]
            | scan_table[chunk[7] as usize];
        if flags == 0 {
            i += CHUNK;
            continue;
        }
        // Something interesting in this chunk — process per byte.
        for (offset, &b) in chunk.iter().enumerate() {
            let f = scan_table[b as usize];
            if f & SCAN_FULL != 0 {
                requires_full_inline = true;
                if f & SCAN_IS_BACKTICK == 0 {
                    full_only_backticks = false;
                }
                if f & SCAN_IS_BACKTICK != 0 && backtick_hint < u8::MAX {
                    backtick_hint = backtick_hint.saturating_add(1);
                }
                rescan_from = i + offset + 1;
                i = bytes.len(); // signal done
                break;
            }
            if f & SCAN_EMPH != 0 {
                has_emphasis = true;
            }
            if f & SCAN_BREAK != 0 {
                has_breaks = true;
            }
            if f & SCAN_ESCAPE != 0 {
                needs_html_escape = true;
            }
        }
        if requires_full_inline {
            break;
        }
        i += CHUNK;
    }
    // Handle remaining bytes (tail < 8).
    if !requires_full_inline {
        while i < bytes.len() {
            let f = scan_table[bytes[i] as usize];
            if f & SCAN_FULL != 0 {
                requires_full_inline = true;
                if f & SCAN_IS_BACKTICK == 0 {
                    full_only_backticks = false;
                }
                if f & SCAN_IS_BACKTICK != 0 && backtick_hint < u8::MAX {
                    backtick_hint = backtick_hint.saturating_add(1);
                }
                rescan_from = i + 1;
                break;
            }
            if f & SCAN_EMPH != 0 {
                has_emphasis = true;
            }
            if f & SCAN_BREAK != 0 {
                has_breaks = true;
            }
            if f & SCAN_ESCAPE != 0 {
                needs_html_escape = true;
            }
            i += 1;
        }
    }

    if !requires_full_inline {
        if has_breaks {
            emit_breaks_and_emphasis(out, raw, bytes, opts, &mut bufs.em_delims);
        } else if has_emphasis {
            emit_emphasis_only(out, raw, bytes, &mut bufs.em_delims);
        } else if needs_html_escape || opts.collapse_whitespace {
            if opts.collapse_whitespace {
                crate::html::collapse_and_escape_into(out, raw);
            } else {
                escape_html_into(out, raw);
            }
        } else {
            out.push_str(raw);
        }
        return;
    }

    if try_emit_common_inline(&mut bufs.scratch, raw, opts) {
        out.push_str(&bufs.scratch);
        bufs.scratch.clear();
        return;
    }

    if backtick_hint != 0 && backtick_hint < 3 {
        backtick_hint = count_backtick_hint_from(bytes, rescan_from, backtick_hint);
    }

    // The fast scan stopped at the first FULL byte, so the flags only cover the
    // prefix. Finish them over the rest; any other FULL byte needs the full parser.
    if full_only_backticks && backtick_hint != 0 {
        for &b in &bytes[rescan_from..] {
            let f = scan_table[b as usize];
            if f & (SCAN_FULL | SCAN_IS_BACKTICK) == SCAN_FULL {
                full_only_backticks = false;
                break;
            }
            has_emphasis |= f & SCAN_EMPH != 0;
            has_breaks |= f & SCAN_BREAK != 0;
            needs_html_escape |= f & SCAN_ESCAPE != 0;
        }
    }

    if full_only_backticks && backtick_hint != 0 && !has_matching_backtick_runs(bytes) {
        if has_breaks {
            emit_breaks_and_emphasis(out, raw, bytes, opts, &mut bufs.em_delims);
        } else if has_emphasis {
            emit_emphasis_only(out, raw, bytes, &mut bufs.em_delims);
        } else if needs_html_escape || opts.collapse_whitespace {
            if opts.collapse_whitespace {
                crate::html::collapse_and_escape_into(out, raw);
            } else {
                escape_html_into(out, raw);
            }
        } else {
            out.push_str(raw);
        }
        return;
    }

    out.reserve(raw.len());
    if bufs.items.capacity() == 0 {
        bufs.items.reserve(raw.len() / 20 + 4);
    }
    let mut p = InlineScanner::new_with_bufs(raw, refs, opts, bufs, backtick_hint);
    p.scan_all();
    if !p.delims.is_empty() {
        p.process_emphasis(0);
    }
    p.render_to_html(out, opts);
}

#[cfg(feature = "html")]
pub fn benchmark_parse_inline(raw: &str, opts: &ParseOptions) -> String {
    let mut out = String::with_capacity(raw.len() + 16);
    let mut bufs = InlineBuffers::new();
    bufs.prepare(opts);
    parse_inline_pass(&mut out, raw, &LinkRefMap::default(), opts, &mut bufs);
    out
}

pub(crate) struct InlineBuffers {
    items: Vec<InlineItem>,
    delims: Vec<usize>,
    brackets: Vec<BracketInfo>,
    links: Vec<LinkInfo>,
    em_delims: Vec<EmDelim>,
    pub(crate) scan_table: [u8; 256],
    scan_key: u8,
    pub(crate) scratch: String,
}

impl InlineBuffers {
    pub(crate) fn new() -> Self {
        Self {
            items: Vec::with_capacity(64),
            delims: Vec::with_capacity(16),
            brackets: Vec::with_capacity(8),
            links: Vec::with_capacity(8),
            em_delims: Vec::with_capacity(16),
            scan_table: build_scan_table(DEFAULT_SCAN_KEY),
            scan_key: DEFAULT_SCAN_KEY,
            scratch: String::new(),
        }
    }

    #[inline]
    pub(crate) fn prepare(&mut self, opts: &ParseOptions) {
        let key = scan_key(opts);
        if self.scan_key != key {
            self.scan_table = build_scan_table(key);
            self.scan_key = key;
        }
    }
}

#[inline(always)]
fn scan_key(opts: &ParseOptions) -> u8 {
    (opts.enable_strikethrough as u8)
        | ((opts.enable_highlight as u8) << 1)
        | ((opts.enable_underline as u8) << 2)
        | ((opts.enable_autolink as u8) << 3)
        | ((opts.enable_latex_math as u8) << 4)
}

const DEFAULT_SCAN_KEY: u8 = 0b0_1111;

fn build_scan_table(key: u8) -> [u8; 256] {
    let mut t = [0u8; 256];
    t[b'*' as usize] = SCAN_EMPH;
    t[b'_' as usize] = SCAN_EMPH;
    t[b'\\' as usize] = SCAN_BREAK;
    t[b'\n' as usize] = SCAN_BREAK;
    t[b'>' as usize] = SCAN_ESCAPE;
    t[b'"' as usize] = SCAN_ESCAPE;
    t[b'`' as usize] = SCAN_FULL | SCAN_IS_BACKTICK;
    t[b'!' as usize] = SCAN_FULL;
    t[b'[' as usize] = SCAN_FULL;
    t[b']' as usize] = SCAN_FULL;
    t[b'<' as usize] = SCAN_FULL;
    t[b'&' as usize] = SCAN_FULL;
    if key & 1 != 0 {
        t[b'~' as usize] = SCAN_FULL;
    }
    if key & 2 != 0 {
        t[b'=' as usize] = SCAN_FULL;
    }
    if key & 4 != 0 {
        t[b'+' as usize] = SCAN_FULL;
    }
    if key & 8 != 0 {
        t[b':' as usize] = SCAN_FULL;
        t[b'@' as usize] = SCAN_FULL;
    }
    if key & 16 != 0 {
        t[b'$' as usize] = SCAN_FULL;
    }
    t
}

#[derive(Clone, Debug)]
struct SmallEmVec {
    data: [u8; 4],
    len: u8,
}

impl SmallEmVec {
    #[inline(always)]
    const fn new() -> Self {
        Self {
            data: [0; 4],
            len: 0,
        }
    }
    #[inline(always)]
    fn push(&mut self, val: u8) {
        if (self.len as usize) < 4 {
            self.data[self.len as usize] = val;
            self.len += 1;
        }
    }
    #[inline(always)]
    fn as_slice(&self) -> &[u8] {
        &self.data[..self.len as usize]
    }
}

#[derive(Clone, Debug)]
enum LinkDest {
    Range(u32, u32),
    Owned(Rc<str>),
}

/// Link title — either a byte range into the original input (no escapes/entities)
/// or an owned string (escape/entity processing was needed).
#[derive(Clone, Debug)]
enum LinkTitle {
    Range(u32, u32),
    Owned(Rc<str>),
}

#[derive(Clone, Debug)]
struct LinkInfo {
    dest: LinkDest,
    title: Option<LinkTitle>,
    is_image: bool,
}

#[derive(Clone, Debug)]
enum InlineItem {
    TextRange(usize, usize),
    TextOwned(Box<str>),
    TextStatic(&'static str),
    TextInline {
        buf: [u8; 8],
        len: u8,
    },
    RawHtml(usize, usize),
    Autolink(u32, u32, bool),
    Code(Box<str>),
    /// Code span stored as byte range into input (unescaped). Escaped at render time.
    CodeRange(u32, u32),
    HardBreak,
    SoftBreak,
    DelimRun {
        kind: u8,
        count: u16,
        can_open: bool,
        can_close: bool,
        open_em: SmallEmVec,
        close_em: SmallEmVec,
    },
    BracketOpen {
        is_image: bool,
    },
    LinkStart(u16),
    LinkEnd,
    /// `[[text]]` wiki link. The range covers the text between the `[[` and `]]`.
    WikiLink(usize, usize),
    /// `$...$` inline math. Range covers content (excluding delimiters).
    MathInline(usize, usize),
    /// `$$...$$` display math. Range covers content (excluding delimiters).
    MathDisplay(usize, usize),
}

#[derive(Clone, Debug)]
struct BracketInfo {
    item_idx: usize,
    is_image: bool,
    delim_bottom: usize,
    active: bool,
    text_pos: usize,
}

struct InlineScanner<'a> {
    input: &'a str,
    bytes: &'a [u8],
    pos: usize,
    refs: &'a LinkRefMap,
    opts: &'a ParseOptions,
    items: &'a mut Vec<InlineItem>,
    delims: &'a mut Vec<usize>,
    brackets: &'a mut Vec<BracketInfo>,
    links: &'a mut Vec<LinkInfo>,
    /// `backtick_last[n]` = last byte offset of a run of length `n` (`u32::MAX` = absent).
    /// Covers lengths 1–63; longer runs go into the overflow map (only allocated when needed).
    /// Pre-populated at construction for O(1) bail in `scan_code_span`.
    has_backtick_index: bool,
    backtick_last: [u32; 64],
    backtick_last_long: Option<rustc_hash::FxHashMap<u32, u32>>,
}

impl<'a> InlineScanner<'a> {
    fn new_with_bufs(
        input: &'a str,
        refs: &'a LinkRefMap,
        opts: &'a ParseOptions,
        bufs: &'a mut InlineBuffers,
        backtick_hint: u8,
    ) -> Self {
        bufs.items.clear();
        bufs.delims.clear();
        bufs.brackets.clear();
        bufs.links.clear();
        // Only build the backtick index when the pre-scan saw several backticks.
        // Tiny one-code-span paragraphs are cheaper to handle with local memchr scans.
        let has_backtick_index = backtick_hint >= 3;
        let (backtick_last, backtick_last_long) = if has_backtick_index {
            Self::build_backtick_last(input.as_bytes())
        } else {
            ([u32::MAX; 64], None)
        };
        Self {
            input,
            bytes: input.as_bytes(),
            pos: 0,
            refs,
            opts,
            items: &mut bufs.items,
            delims: &mut bufs.delims,
            brackets: &mut bufs.brackets,
            links: &mut bufs.links,
            has_backtick_index,
            backtick_last,
            backtick_last_long,
        }
    }

    /// Single forward pass using memchr: record the last byte offset of each backtick run length.
    fn build_backtick_last(bytes: &[u8]) -> ([u32; 64], Option<rustc_hash::FxHashMap<u32, u32>>) {
        let mut last = [u32::MAX; 64];
        let mut long: Option<rustc_hash::FxHashMap<u32, u32>> = None;
        let mut i = 0;
        while let Some(rel) = memchr::memchr(b'`', &bytes[i..]) {
            let start = i + rel;
            i = start;
            let mut run = 0usize;
            while i < bytes.len() && bytes[i] == b'`' {
                run += 1;
                i += 1;
            }
            if run < 64 {
                last[run] = start as u32;
            } else {
                long.get_or_insert_with(rustc_hash::FxHashMap::default)
                    .insert(run as u32, start as u32);
            }
        }
        (last, long)
    }
}

pub(super) use crate::is_ascii_punctuation;
pub(super) use crate::utf8_char_len;

static EMAIL_LOCAL: [bool; 256] = {
    let mut t = [false; 256];
    let ranges: &[(u8, u8)] = &[(b'0', b'9'), (b'A', b'Z'), (b'a', b'z')];
    let mut r = 0;
    while r < 3 {
        let mut i = ranges[r].0;
        while i <= ranges[r].1 {
            t[i as usize] = true;
            i += 1;
        }
        r += 1;
    }
    let extra = b".!#$%&'*+/=?^_`{|}~-";
    let mut j = 0;
    while j < extra.len() {
        t[extra[j] as usize] = true;
        j += 1;
    }
    t
};

#[inline(always)]
fn is_email_local_char(b: u8) -> bool {
    EMAIL_LOCAL[b as usize]
}

#[inline(always)]
fn is_punctuation_char(c: char) -> bool {
    if c.is_ascii() {
        return is_ascii_punctuation(c as u8);
    }
    matches!(c as u32,
        0x00A0..=0x00BF | 0x2000..=0x206F | 0x2E00..=0x2E7F |
        0x3000..=0x303F | 0xFE30..=0xFE6F | 0xFF01..=0xFF0F |
        0xFF1A..=0xFF20 | 0xFF3B..=0xFF40 | 0xFF5B..=0xFF65 |
        0x2100..=0x214F | 0x2190..=0x21FF | 0x2200..=0x22FF |
        0x2300..=0x23FF | 0x2500..=0x257F | 0x25A0..=0x25FF |
        0x2600..=0x26FF | 0x2700..=0x27BF | 0x20A0..=0x20CF
    )
}

#[inline(always)]
fn flanking(marker: u8, before: char, after: char) -> (bool, bool) {
    let left_flanking = !after.is_whitespace()
        && (!is_punctuation_char(after) || before.is_whitespace() || is_punctuation_char(before));
    let right_flanking = !before.is_whitespace()
        && (!is_punctuation_char(before) || after.is_whitespace() || is_punctuation_char(after));
    if marker == b'_' {
        (
            left_flanking && (!right_flanking || is_punctuation_char(before)),
            right_flanking && (!left_flanking || is_punctuation_char(after)),
        )
    } else {
        (left_flanking, right_flanking)
    }
}

#[inline(always)]
fn char_before(s: &str, byte_pos: usize) -> char {
    if byte_pos == 0 {
        return ' ';
    }
    let bytes = s.as_bytes();
    let prev = bytes[byte_pos - 1];
    if prev < 0x80 {
        return prev as char;
    }
    let mut i = byte_pos - 1;
    while i > 0 && (bytes[i] & 0xC0) == 0x80 {
        i -= 1;
    }
    s[i..byte_pos].chars().next().unwrap_or(' ')
}

#[inline(always)]
fn char_at(s: &str, byte_pos: usize) -> char {
    if byte_pos >= s.len() {
        return ' ';
    }
    let b = s.as_bytes()[byte_pos];
    if b < 0x80 {
        return b as char;
    }
    let len = utf8_char_len(b);
    let end = (byte_pos + len).min(s.len());
    s[byte_pos..end].chars().next().unwrap_or(' ')
}
