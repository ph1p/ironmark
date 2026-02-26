mod links;
mod render;
mod scanner;

use crate::ParseOptions;
use crate::entities;
use crate::html::escape_html_into;
use rustc_hash::FxHashMap;
use std::borrow::Cow;
use std::rc::Rc;

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct LinkReference {
    pub href: Rc<str>,
    pub title: Option<Rc<str>>,
}

pub(crate) type LinkRefMap = FxHashMap<String, LinkReference>;

pub(crate) fn normalize_reference_label(label: &str) -> Cow<'_, str> {
    let trimmed = label.trim();
    let bytes = trimmed.as_bytes();

    {
        let mut simple = true;
        let mut prev_space = false;
        for &b in bytes {
            if b >= b'A' && b <= b'Z' {
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
                out.push(if b >= b'A' && b <= b'Z' {
                    (b + 32) as char
                } else {
                    b as char
                });
                in_space = false;
                i += 1;
            }
        } else {
            let ch = &trimmed[i..];
            let c = ch.chars().next().unwrap();
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

    // Fast check: find first complex byte (not * or _) using memchr3 on the most
    // common complex triggers. If none found, we only need emphasis-only or plain.
    let first_complex = memchr::memchr3(b'[', b'`', b'<', bytes)
        .or_else(|| memchr::memchr3(b'&', b'!', b'~', bytes))
        .or_else(|| memchr::memchr3(b'=', b'+', b':', bytes))
        .or_else(|| memchr::memchr(b'@', bytes));

    let has_newline_or_backslash = if first_complex.is_none() {
        memchr::memchr2(b'\n', b'\\', bytes)
    } else {
        None
    };

    if first_complex.is_none() && has_newline_or_backslash.is_none() {
        // No complex bytes. Check if there are emphasis markers at all.
        if let Some(first_em) = memchr::memchr2(b'*', b'_', bytes) {
            emit_emphasis_only(out, raw, bytes, &mut bufs.em_delims);
            let _ = first_em;
            return;
        }
        escape_html_into(out, raw);
        return;
    }

    if first_complex.is_none() {
        // Only \n and/or \\ — breaks + emphasis path.
        emit_breaks_and_emphasis(out, raw, bytes, opts, &mut bufs.em_delims);
        return;
    }

    out.reserve(raw.len());
    if bufs.items.capacity() == 0 {
        bufs.items.reserve(raw.len() / 20 + 4);
    }
    let mut p = InlineScanner::new_with_bufs(raw, refs, opts, bufs);
    p.scan_all();
    if !p.delims.is_empty() {
        p.process_emphasis(0);
    }
    p.render_to_html(out, opts);
}

#[derive(Clone, Copy)]
struct EmDelim {
    orig_start: u32,
    orig_end: u32,
    cur_start: u32,
    cur_end: u32,
    marker: u8,
    can_open: bool,
    can_close: bool,
    active: bool,
    open_em: [u8; 4],
    open_em_len: u8,
    close_em: [u8; 4],
    close_em_len: u8,
}

#[inline(never)]
fn process_em_delims(delims: &mut [EmDelim]) {
    let mut closer_idx = 0usize;
    while closer_idx < delims.len() {
        if !delims[closer_idx].active
            || !delims[closer_idx].can_close
            || delims[closer_idx].cur_end == delims[closer_idx].cur_start
        {
            closer_idx += 1;
            continue;
        }
        let cmarker = delims[closer_idx].marker;
        let ccount = (delims[closer_idx].cur_end - delims[closer_idx].cur_start) as u16;

        let mut found = false;
        let mut oi = closer_idx;
        while oi > 0 {
            oi -= 1;
            if !delims[oi].active || delims[oi].marker != cmarker || !delims[oi].can_open {
                continue;
            }
            let ocount = (delims[oi].cur_end - delims[oi].cur_start) as u16;
            if ocount == 0 {
                continue;
            }
            if (delims[oi].can_close || delims[closer_idx].can_open)
                && (ocount + ccount) % 3 == 0
                && (ocount % 3 != 0 || ccount % 3 != 0)
            {
                continue;
            }
            let use_count: u16 = if ocount >= 2 && ccount >= 2 { 2 } else { 1 };
            let tag = use_count as u8;

            delims[oi].cur_end -= use_count as u32;
            let idx = delims[oi].open_em_len as usize;
            if idx < 4 {
                delims[oi].open_em[idx] = tag;
                delims[oi].open_em_len += 1;
            }

            delims[closer_idx].cur_start += use_count as u32;
            let idx = delims[closer_idx].close_em_len as usize;
            if idx < 4 {
                delims[closer_idx].close_em[idx] = tag;
                delims[closer_idx].close_em_len += 1;
            }

            for k in (oi + 1)..closer_idx {
                delims[k].active = false;
            }

            if delims[oi].cur_end == delims[oi].cur_start {
                delims[oi].active = false;
            }
            if delims[closer_idx].cur_end == delims[closer_idx].cur_start {
                delims[closer_idx].active = false;
            }

            found = true;
            break;
        }
        if !found {
            closer_idx += 1;
        }
    }
}

#[inline(always)]
fn render_em_delim(out: &mut String, d: &EmDelim) {
    for j in 0..d.close_em_len as usize {
        out.push_str(if d.close_em[j] == 2 {
            "</strong>"
        } else {
            "</em>"
        });
    }
    if d.cur_start < d.cur_end {
        let marker = d.marker as char;
        for _ in 0..(d.cur_end - d.cur_start) {
            out.push(marker);
        }
    }
    for j in (0..d.open_em_len as usize).rev() {
        out.push_str(if d.open_em[j] == 2 {
            "<strong>"
        } else {
            "<em>"
        });
    }
}

fn scan_em_delims(raw: &str, bytes: &[u8], skip_escapes: bool, buf: &mut Vec<EmDelim>) {
    buf.clear();
    let len = bytes.len();
    let mut i = 0;
    while i < len {
        // Use memchr to skip to the next interesting byte.
        let next = if skip_escapes {
            memchr::memchr3(b'*', b'_', b'\\', &bytes[i..])
        } else {
            memchr::memchr2(b'*', b'_', &bytes[i..])
        };
        let Some(off) = next else { break };
        i += off;
        let b = bytes[i];
        if b == b'\\' {
            // skip_escapes must be true if we got here
            if i + 1 < len && crate::is_ascii_punctuation(bytes[i + 1]) {
                i += 2;
            } else {
                i += 1;
            }
            continue;
        }
        let run_start = i;
        i += 1;
        while i < len && bytes[i] == b {
            i += 1;
        }
        let before = if run_start > 0 {
            char_before(raw, run_start)
        } else {
            ' '
        };
        let after = if i < len { char_at(raw, i) } else { ' ' };
        let (can_open, can_close) = flanking(b, before, after);
        buf.push(EmDelim {
            orig_start: run_start as u32,
            orig_end: i as u32,
            cur_start: run_start as u32,
            cur_end: i as u32,
            marker: b,
            can_open,
            can_close,
            active: true,
            open_em: [0; 4],
            open_em_len: 0,
            close_em: [0; 4],
            close_em_len: 0,
        });
    }
}

fn emit_emphasis_only(out: &mut String, raw: &str, bytes: &[u8], em_buf: &mut Vec<EmDelim>) {
    scan_em_delims(raw, bytes, false, em_buf);
    if em_buf.is_empty() {
        escape_html_into(out, raw);
        return;
    }
    process_em_delims(em_buf);

    let mut text_pos = 0usize;
    for d in em_buf.iter() {
        if text_pos < d.orig_start as usize {
            escape_html_into(out, &raw[text_pos..d.orig_start as usize]);
        }
        render_em_delim(out, d);
        text_pos = d.orig_end as usize;
    }
    if text_pos < bytes.len() {
        escape_html_into(out, &raw[text_pos..]);
    }
}

fn emit_breaks_and_emphasis(
    out: &mut String,
    raw: &str,
    bytes: &[u8],
    opts: &ParseOptions,
    em_buf: &mut Vec<EmDelim>,
) {
    scan_em_delims(raw, bytes, true, em_buf);
    if !em_buf.is_empty() {
        process_em_delims(em_buf);
    }

    let len = bytes.len();
    let mut text_pos = 0usize;
    let mut di = 0usize;

    while text_pos < len {
        let next_delim_pos = if di < em_buf.len() {
            em_buf[di].orig_start as usize
        } else {
            len
        };
        emit_text_with_breaks(out, raw, bytes, &mut text_pos, next_delim_pos, opts);

        if di < em_buf.len() && text_pos == em_buf[di].orig_start as usize {
            render_em_delim(out, &em_buf[di]);
            text_pos = em_buf[di].orig_end as usize;
            di += 1;
        }
    }
}

#[inline]
fn emit_text_with_breaks(
    out: &mut String,
    raw: &str,
    bytes: &[u8],
    text_pos: &mut usize,
    end: usize,
    opts: &ParseOptions,
) {
    let mut seg_start = *text_pos;
    let mut i = *text_pos;
    while i < end {
        let b = bytes[i];
        if b == b'\n' {
            let mut text_end = i;
            let is_hard = (i >= seg_start + 2 && bytes[i - 1] == b' ' && bytes[i - 2] == b' ')
                || (i > seg_start && bytes[i - 1] == b'\\');
            if is_hard && i > seg_start && bytes[i - 1] == b'\\' {
                text_end = i - 1;
            } else {
                while text_end > seg_start && bytes[text_end - 1] == b' ' {
                    text_end -= 1;
                }
            }
            if seg_start < text_end {
                escape_html_into(out, &raw[seg_start..text_end]);
            }
            if is_hard || opts.hard_breaks {
                out.push_str("<br />\n");
            } else {
                out.push('\n');
            }
            i += 1;
            seg_start = i;
        } else if b == b'\\' && i + 1 < end {
            let next = bytes[i + 1];
            if next == b'\n' {
                i += 1;
                continue;
            }
            if crate::is_ascii_punctuation(next) {
                if seg_start < i {
                    escape_html_into(out, &raw[seg_start..i]);
                }
                match next {
                    b'&' => out.push_str("&amp;"),
                    b'<' => out.push_str("&lt;"),
                    b'>' => out.push_str("&gt;"),
                    b'"' => out.push_str("&quot;"),
                    _ => out.push_str(&raw[i + 1..i + 2]),
                }
                i += 2;
                seg_start = i;
            } else {
                i += 1;
            }
        } else {
            i += 1;
        }
    }
    if seg_start < end {
        escape_html_into(out, &raw[seg_start..end]);
    }
    *text_pos = end;
}

pub(crate) struct InlineBuffers {
    items: Vec<InlineItem>,
    delims: Vec<usize>,
    brackets: Vec<BracketInfo>,
    links: Vec<LinkInfo>,
    em_delims: Vec<EmDelim>,
}

impl InlineBuffers {
    pub(crate) fn new() -> Self {
        Self {
            items: Vec::new(),
            delims: Vec::new(),
            brackets: Vec::new(),
            links: Vec::new(),
            em_delims: Vec::new(),
        }
    }
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

#[derive(Clone, Debug)]
struct LinkInfo {
    dest: LinkDest,
    title: Option<Rc<str>>,
    is_image: bool,
}

#[derive(Clone, Debug)]
enum InlineItem {
    TextRange(usize, usize),
    TextOwned(String),
    TextStatic(&'static str),
    TextInline {
        buf: [u8; 8],
        len: u8,
    },
    RawHtml(usize, usize),
    Autolink(u32, u32, bool),
    Code(String),
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
    /// Bitfield: bit N set means no closing backtick run of length N exists.
    backtick_no_match: u64,
}

impl<'a> InlineScanner<'a> {
    fn new_with_bufs(
        input: &'a str,
        refs: &'a LinkRefMap,
        opts: &'a ParseOptions,
        bufs: &'a mut InlineBuffers,
    ) -> Self {
        bufs.items.clear();
        bufs.delims.clear();
        bufs.brackets.clear();
        bufs.links.clear();
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
            backtick_no_match: 0,
        }
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
