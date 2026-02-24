mod links;
mod render;
mod scanner;

use crate::entities;
use crate::html::{encode_url, escape_html_into};
use crate::ParseOptions;
use std::collections::HashMap;

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct LinkReference {
    pub href: String,
    pub title: Option<String>,
}

pub(crate) type LinkRefMap = HashMap<String, LinkReference>;

pub(crate) fn normalize_reference_label(label: &str) -> String {
    let trimmed = label.trim();
    let bytes = trimmed.as_bytes();

    // Super-fast path: if all ASCII lowercase/digits/single-spaces, return as-is
    // This covers most real-world reference labels like "1", "foo", "my-link"
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
            return trimmed.to_string();
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
            // Non-ASCII: decode char
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
    out
}

/// Static lookup table: maps each ASCII byte to a &'static str containing that single character.
/// Avoids heap allocation when resolving entities to single ASCII chars.
static ASCII_CHAR_STRS: [&str; 128] = {
    const fn make() -> [&'static str; 128] {
        // We build this with a macro-like approach, but since we can't iterate in const,
        // we use a pre-built table.
        [
            "\x00", "\x01", "\x02", "\x03", "\x04", "\x05", "\x06", "\x07", "\x08", "\x09", "\x0A",
            "\x0B", "\x0C", "\x0D", "\x0E", "\x0F", "\x10", "\x11", "\x12", "\x13", "\x14", "\x15",
            "\x16", "\x17", "\x18", "\x19", "\x1A", "\x1B", "\x1C", "\x1D", "\x1E", "\x1F", " ",
            "!", "\"", "#", "$", "%", "&", "'", "(", ")", "*", "+", ",", "-", ".", "/", "0", "1",
            "2", "3", "4", "5", "6", "7", "8", "9", ":", ";", "<", "=", ">", "?", "@", "A", "B",
            "C", "D", "E", "F", "G", "H", "I", "J", "K", "L", "M", "N", "O", "P", "Q", "R", "S",
            "T", "U", "V", "W", "X", "Y", "Z", "[", "\\", "]", "^", "_", "`", "a", "b", "c", "d",
            "e", "f", "g", "h", "i", "j", "k", "l", "m", "n", "o", "p", "q", "r", "s", "t", "u",
            "v", "w", "x", "y", "z", "{", "|", "}", "~", "\x7F",
        ]
    }
    make()
};

/// Parse inline content and write HTML to `out`.
#[inline]
pub(crate) fn parse_inline_pass(
    out: &mut String,
    raw: &str,
    refs: &LinkRefMap,
    opts: &ParseOptions,
    bufs: &mut InlineBuffers,
) {
    let bytes = raw.as_bytes();

    // Lookup tables for fast character classification.
    static ANY_SPECIAL: [bool; 256] = {
        let mut t = [false; 256];
        t[b'\\' as usize] = true;
        t[b'`' as usize] = true;
        t[b'*' as usize] = true;
        t[b'_' as usize] = true;
        t[b'!' as usize] = true;
        t[b'[' as usize] = true;
        t[b']' as usize] = true;
        t[b'<' as usize] = true;
        t[b'&' as usize] = true;
        t[b'\n' as usize] = true;
        t[b'~' as usize] = true;
        t[b'=' as usize] = true;
        t[b'+' as usize] = true;
        t[b':' as usize] = true;
        t[b'@' as usize] = true;
        t
    };
    static COMPLEX: [bool; 256] = {
        let mut t = [false; 256];
        t[b'\\' as usize] = true;
        t[b'`' as usize] = true;
        t[b'!' as usize] = true;
        t[b'[' as usize] = true;
        t[b']' as usize] = true;
        t[b'<' as usize] = true;
        t[b'&' as usize] = true;
        t[b'\n' as usize] = true;
        t[b'~' as usize] = true;
        t[b'=' as usize] = true;
        t[b'+' as usize] = true;
        t[b':' as usize] = true;
        t[b'@' as usize] = true;
        t
    };

    // Fast path: find the first special char. Short-circuits for plain text.
    let first_special = bytes.iter().position(|b| ANY_SPECIAL[*b as usize]);
    if first_special.is_none() {
        escape_html_into(out, raw);
        return;
    }

    // Classify: emphasis-only or complex?
    let first_pos = first_special.unwrap();
    let first_byte = bytes[first_pos];
    if first_byte == b'*' || first_byte == b'_' {
        let has_complex = bytes[first_pos + 1..].iter().any(|b| COMPLEX[*b as usize]);
        if !has_complex {
            emit_emphasis_only(out, raw, bytes);
            return;
        }
    }

    // Ensure items vec has reasonable capacity on first use
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

/// Fast single-pass emphasis-only renderer.
/// Handles text containing only * and _ emphasis markers (no links, entities, code spans, etc.).
/// Emits HTML directly without building an InlineItem Vec.
fn emit_emphasis_only(out: &mut String, raw: &str, bytes: &[u8]) {
    // We reuse the full InlineScanner machinery but with emphasis-only items,
    // avoiding alloc overhead for links/entities/etc. Actually, the real cost
    // is the Vec allocations themselves. Let's use a compact representation.

    // Each delimiter run: original byte range, marker, flags, and open/close tag lists
    struct Delim {
        orig_start: u32, // original run start in bytes
        orig_end: u32,   // original run end in bytes
        cur_start: u32,  // current start (after close_em consumption)
        cur_end: u32,    // current end (after open_em consumption)
        marker: u8,
        can_open: bool,
        can_close: bool,
        // Open/close emphasis tags stored as SmallEmVec-like
        open_em: [u8; 4],
        open_em_len: u8,
        close_em: [u8; 4],
        close_em_len: u8,
    }

    let len = bytes.len();
    let mut delims: Vec<Delim> = Vec::new();

    // Pass 1: find all delimiter runs
    let mut i = 0;
    while i < len {
        let b = bytes[i];
        if b != b'*' && b != b'_' {
            i += 1;
            continue;
        }
        let run_start = i;
        while i < len && bytes[i] == b {
            i += 1;
        }

        let before = if run_start > 0 {
            char_before(raw, run_start)
        } else {
            ' '
        };
        let after = if i < len { char_at(raw, i) } else { ' ' };

        let left_flanking = !after.is_whitespace()
            && (!is_punctuation_char(after)
                || before.is_whitespace()
                || is_punctuation_char(before));
        let right_flanking = !before.is_whitespace()
            && (!is_punctuation_char(before)
                || after.is_whitespace()
                || is_punctuation_char(after));

        let (can_open, can_close) = if b == b'*' {
            (left_flanking, right_flanking)
        } else {
            (
                left_flanking && (!right_flanking || is_punctuation_char(before)),
                right_flanking && (!left_flanking || is_punctuation_char(after)),
            )
        };

        delims.push(Delim {
            orig_start: run_start as u32,
            orig_end: i as u32,
            cur_start: run_start as u32,
            cur_end: i as u32,
            marker: b,
            can_open,
            can_close,
            open_em: [0; 4],
            open_em_len: 0,
            close_em: [0; 4],
            close_em_len: 0,
        });
    }

    if delims.is_empty() {
        escape_html_into(out, raw);
        return;
    }

    // Pass 2: match emphasis (same algorithm as process_emphasis)
    let num_delims = delims.len();
    // Track which delimiter indices are still "active" (not fully consumed)
    // We use a simple index list like the main parser
    let mut active: Vec<usize> = (0..num_delims).collect();

    let mut closer_ai = 0; // index into active[]
    while closer_ai < active.len() {
        let ci = active[closer_ai];
        let ccount = (delims[ci].cur_end - delims[ci].cur_start) as u16;
        if !delims[ci].can_close || ccount == 0 {
            closer_ai += 1;
            continue;
        }
        let cmarker = delims[ci].marker;

        // Search backward for opener
        let mut found = false;
        let mut opener_ai = closer_ai;
        while opener_ai > 0 {
            opener_ai -= 1;
            let oi = active[opener_ai];
            let ocount = (delims[oi].cur_end - delims[oi].cur_start) as u16;
            if delims[oi].marker != cmarker || !delims[oi].can_open || ocount == 0 {
                continue;
            }
            // Rule of three
            if (delims[oi].can_close || delims[ci].can_open)
                && (ocount + ccount) % 3 == 0
                && (ocount % 3 != 0 || ccount % 3 != 0)
            {
                continue;
            }
            // Match found
            let use_count: u16 = if ocount >= 2 && ccount >= 2 { 2 } else { 1 };
            let tag = use_count as u8;

            // Record tags
            {
                let d = &mut delims[oi];
                d.cur_end -= use_count as u32;
                let idx = d.open_em_len as usize;
                if idx < 4 {
                    d.open_em[idx] = tag;
                    d.open_em_len += 1;
                }
            }
            {
                let d = &mut delims[ci];
                d.cur_start += use_count as u32;
                let idx = d.close_em_len as usize;
                if idx < 4 {
                    d.close_em[idx] = tag;
                    d.close_em_len += 1;
                }
            }

            // Remove fully consumed delimiters between opener and closer
            let remove_start = opener_ai + 1;
            let remove_end = closer_ai;
            if remove_start < remove_end {
                active.drain(remove_start..remove_end);
                closer_ai = remove_start;
            }

            // Remove opener if fully consumed
            let new_ocount =
                delims[active[opener_ai]].cur_end - delims[active[opener_ai]].cur_start;
            if new_ocount == 0 {
                active.remove(opener_ai);
                closer_ai -= 1;
            }

            // Check if closer is fully consumed
            let new_ccount =
                delims[active[closer_ai]].cur_end - delims[active[closer_ai]].cur_start;
            if new_ccount == 0 {
                active.remove(closer_ai);
            }

            found = true;
            break;
        }

        if !found {
            closer_ai += 1;
        }
    }

    // Pass 3: emit HTML — iterate delims in original order
    let mut text_pos = 0usize;
    for d in &delims {
        let orig_start = d.orig_start as usize;
        let orig_end = d.orig_end as usize;
        let cur_start = d.cur_start as usize;
        let cur_end = d.cur_end as usize;

        // Text before this delimiter's original position
        if text_pos < orig_start {
            escape_html_into(out, &raw[text_pos..orig_start]);
        }

        // Close tags (from close_em)
        for j in 0..d.close_em_len as usize {
            let tag = d.close_em[j];
            if tag == 2 {
                out.push_str("</strong>");
            } else {
                out.push_str("</em>");
            }
        }

        // Remaining literal delimiters
        if cur_start < cur_end {
            let marker = d.marker as char;
            for _ in 0..(cur_end - cur_start) {
                out.push(marker);
            }
        }

        // Open tags (in reverse order, innermost first)
        for j in (0..d.open_em_len as usize).rev() {
            let tag = d.open_em[j];
            if tag == 2 {
                out.push_str("<strong>");
            } else {
                out.push_str("<em>");
            }
        }

        text_pos = orig_end;
    }

    // Remaining text after last delimiter
    if text_pos < len {
        escape_html_into(out, &raw[text_pos..len]);
    }
}

// ── Inline items (flat list, no intermediate AST) ────────────────────

pub(crate) struct InlineBuffers {
    items: Vec<InlineItem>,
    delims: Vec<usize>,
    brackets: Vec<BracketInfo>,
    links: Vec<LinkInfo>,
}

impl InlineBuffers {
    pub(crate) fn new() -> Self {
        Self {
            items: Vec::new(),
            delims: Vec::new(),
            brackets: Vec::new(),
            links: Vec::new(),
        }
    }
}

/// Small inline array for emphasis open/close markers (avoids Vec heap allocation).
/// Emphasis processing pushes at most a few 1/2 values per delimiter run.
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

/// Compact link destination that avoids heap allocation when dest is a simple input slice.
#[derive(Clone, Debug)]
enum LinkDest {
    /// Byte range into the input (no escapes, entities, or special chars)
    Range(u32, u32),
    /// Owned string (resolved escapes, entities, etc.)
    Owned(String),
}

#[derive(Clone, Debug)]
struct LinkInfo {
    dest: LinkDest,
    title: Option<String>,
    is_image: bool,
}

#[derive(Clone, Debug)]
enum InlineItem {
    /// Range into input that needs HTML escaping when rendered
    TextRange(usize, usize),
    /// Pre-built text (already escaped or special content) - heap allocated
    TextOwned(String),
    /// Static text (like "&lt;", "&amp;", "]") - no allocation
    TextStatic(&'static str),
    /// Small inline text (up to 8 bytes) - no heap allocation
    TextInline {
        buf: [u8; 8],
        len: u8,
    },
    /// Raw HTML pass-through (no escaping)
    RawHtml(usize, usize), // range into input
    RawHtmlOwned(String), // owned (for constructed HTML like autolinks)
    /// Code span (content is HTML-escaped)
    Code(String),
    HardBreak,
    SoftBreak,
    /// A delimiter run for emphasis
    DelimRun {
        kind: u8,
        count: u16,
        can_open: bool,
        can_close: bool,
        open_em: SmallEmVec,
        close_em: SmallEmVec,
    },
    /// Opening [ or ![
    BracketOpen {
        is_image: bool,
    },
    /// Resolved link/image start marker (index into links vec)
    LinkStart(u16),
    /// Closing marker for a resolved link
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
        }
    }
}

// ── Helper functions ─────────────────────────────────────────────────

#[inline(always)]
fn is_ascii_punctuation(b: u8) -> bool {
    matches!(b, b'!'..=b'/' | b':'..=b'@' | b'['..=b'`' | b'{'..=b'~')
}

#[inline(always)]
fn is_punctuation_char(c: char) -> bool {
    if c.is_ascii() {
        is_ascii_punctuation(c as u8)
    } else {
        let cat = unicode_general_category(c);
        matches!(cat, 'P' | 'S')
    }
}

fn unicode_general_category(c: char) -> char {
    if c.is_ascii() {
        if is_ascii_punctuation(c as u8) {
            return 'P';
        }
        return 'L';
    }
    match c as u32 {
        0x00A0..=0x00BF => 'P',
        0x2000..=0x206F => 'P',
        0x2E00..=0x2E7F => 'P',
        0x3000..=0x303F => 'P',
        0xFE30..=0xFE6F => 'P',
        0xFF01..=0xFF0F => 'P',
        0xFF1A..=0xFF20 => 'P',
        0xFF3B..=0xFF40 => 'P',
        0xFF5B..=0xFF65 => 'P',
        0x2100..=0x214F => 'S',
        0x2190..=0x21FF => 'S',
        0x2200..=0x22FF => 'S',
        0x2300..=0x23FF => 'S',
        0x2500..=0x257F => 'S',
        0x25A0..=0x25FF => 'S',
        0x2600..=0x26FF => 'S',
        0x2700..=0x27BF => 'S',
        0x20A0..=0x20CF => 'S',
        _ => 'L',
    }
}

#[inline(always)]
fn char_before(s: &str, byte_pos: usize) -> char {
    if byte_pos == 0 {
        return ' ';
    }
    let bytes = s.as_bytes();
    // Fast path: ASCII (most common case in markdown)
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
    // Fast path: ASCII (most common case in markdown)
    if b < 0x80 {
        return b as char;
    }
    let len = utf8_char_len(b);
    let end = (byte_pos + len).min(s.len());
    s[byte_pos..end].chars().next().unwrap_or(' ')
}

fn is_email_autolink(s: &str) -> bool {
    let bytes = s.as_bytes();
    let at = bytes.iter().position(|&b| b == b'@');
    let Some(at) = at else {
        return false;
    };
    if at == 0 || at + 1 >= bytes.len() {
        return false;
    }
    for &b in &bytes[..at] {
        if !(b.is_ascii_alphanumeric()
            || matches!(
                b,
                b'.' | b'!'
                    | b'#'
                    | b'$'
                    | b'%'
                    | b'&'
                    | b'\''
                    | b'*'
                    | b'+'
                    | b'/'
                    | b'='
                    | b'?'
                    | b'^'
                    | b'_'
                    | b'`'
                    | b'{'
                    | b'|'
                    | b'}'
                    | b'~'
                    | b'-'
            ))
        {
            return false;
        }
    }
    for &b in &bytes[at + 1..] {
        if !(b.is_ascii_alphanumeric() || b == b'-' || b == b'.') {
            return false;
        }
    }
    true
}

#[inline(always)]
fn utf8_char_len(first: u8) -> usize {
    if first < 0x80 {
        1
    } else if first < 0xE0 {
        2
    } else if first < 0xF0 {
        3
    } else {
        4
    }
}
