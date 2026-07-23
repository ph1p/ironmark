use super::*;
use crate::{is_ascii_punctuation, utf8_char_len};

/// (label, destination, title, bytes consumed) — label/dest/title borrow from the input
/// wherever no escape or entity rewriting was needed.
pub(super) type LinkRefDefParts<'a> = (&'a str, Cow<'a, str>, Option<Cow<'a, str>>, usize);

/// Builds a string that stays borrowed from `input` until the first rewrite,
/// then switches to an owned buffer and bulk-copies the clean segments.
struct CowBuilder<'a> {
    input: &'a str,
    seg_start: usize,
    buf: Option<String>,
}

impl<'a> CowBuilder<'a> {
    #[inline]
    fn new(input: &'a str, start: usize) -> Self {
        Self {
            input,
            seg_start: start,
            buf: None,
        }
    }

    /// Flush `[seg_start..seg_end)`, append `replacement`, resume at `resume`.
    #[inline]
    fn replace(&mut self, seg_end: usize, replacement: &str, resume: usize) {
        let buf = self
            .buf
            .get_or_insert_with(|| String::with_capacity(seg_end - self.seg_start + 8));
        buf.push_str(&self.input[self.seg_start..seg_end]);
        buf.push_str(replacement);
        self.seg_start = resume;
    }

    #[inline]
    fn finish(self, end: usize) -> Cow<'a, str> {
        match self.buf {
            None => Cow::Borrowed(&self.input[self.seg_start..end]),
            Some(mut b) => {
                b.push_str(&self.input[self.seg_start..end]);
                Cow::Owned(b)
            }
        }
    }
}

pub(super) fn parse_link_ref_def(input: &str) -> Option<LinkRefDefParts<'_>> {
    let bytes = input.as_bytes();
    if bytes.is_empty() || bytes[0] != b'[' {
        return None;
    }

    // The label is kept verbatim (escapes included), so it is exactly a subslice.
    let mut i = 1;
    let mut found_close = false;
    while i < bytes.len() {
        match bytes[i] {
            b']' => {
                found_close = true;
                i += 1;
                break;
            }
            b'[' => return None,
            b'\\' if i + 1 < bytes.len() => i += 2,
            _ => i += 1,
        }
    }
    if !found_close {
        return None;
    }
    let label = &input[1..i - 1];
    // CommonMark limits labels to 999 characters. char count <= byte count, so only
    // pay the O(n) char walk when the byte length already exceeds the limit.
    if label.trim().is_empty() || (label.len() > 999 && label.chars().count() > 999) {
        return None;
    }

    if i >= bytes.len() || bytes[i] != b':' {
        return None;
    }
    i += 1;

    i = skip_spaces_and_optional_newline(bytes, i);

    let (dest, dest_end) = parse_link_destination(input, i)?;
    i = dest_end;

    let before_title = i;
    let title_start = skip_spaces_and_optional_newline(bytes, i);

    let mut title = None;

    if title_start < bytes.len()
        && title_start > before_title
        && let Some((t, t_end)) = parse_link_title(input, title_start)
    {
        let after = skip_line_spaces(bytes, t_end);
        if after >= bytes.len() || bytes[after] == b'\n' {
            title = Some(t);
            let consumed = if after < bytes.len() {
                after + 1
            } else {
                after
            };
            return Some((label, dest, title, consumed));
        }
    }

    let after_dest = skip_line_spaces(bytes, before_title);
    if after_dest < bytes.len() && bytes[after_dest] != b'\n' {
        return None;
    }
    let consumed = if after_dest < bytes.len() {
        after_dest + 1
    } else {
        after_dest
    };
    Some((label, dest, title, consumed))
}

pub(super) fn resolve_entities_and_escapes(s: &str) -> std::borrow::Cow<'_, str> {
    let bytes = s.as_bytes();
    if memchr::memchr2(b'\\', b'&', bytes).is_none() {
        return std::borrow::Cow::Borrowed(s);
    }
    let mut out = String::with_capacity(s.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'\\' && i + 1 < bytes.len() && is_ascii_punctuation(bytes[i + 1]) {
            out.push(bytes[i + 1] as char);
            i += 2;
        } else if bytes[i] == b'&' {
            if let Some(end) = crate::entities::resolve_entity_in_bytes(bytes, i, &mut out) {
                i = end;
            } else {
                out.push('&');
                i += 1;
            }
        } else {
            let ch_len = utf8_char_len(bytes[i]);
            out.push_str(&s[i..i + ch_len]);
            i += ch_len;
        }
    }
    std::borrow::Cow::Owned(out)
}

pub(super) fn skip_spaces_and_optional_newline(bytes: &[u8], mut i: usize) -> usize {
    while i < bytes.len() && (bytes[i] == b' ' || bytes[i] == b'\t') {
        i += 1;
    }
    if i < bytes.len() && bytes[i] == b'\n' {
        i += 1;
        while i < bytes.len() && (bytes[i] == b' ' || bytes[i] == b'\t') {
            i += 1;
        }
    }
    i
}

pub(super) fn skip_line_spaces(bytes: &[u8], mut i: usize) -> usize {
    while i < bytes.len() && (bytes[i] == b' ' || bytes[i] == b'\t') {
        i += 1;
    }
    i
}

pub(super) fn parse_link_destination(input: &str, start: usize) -> Option<(Cow<'_, str>, usize)> {
    let bytes = input.as_bytes();
    if start >= bytes.len() {
        return None;
    }

    if bytes[start] == b'<' {
        let mut i = start + 1;
        let mut dest = CowBuilder::new(input, i);
        while i < bytes.len() {
            match bytes[i] {
                b'>' => return Some((dest.finish(i), i + 1)),
                b'<' | b'\n' => return None,
                b'\\' if i + 1 < bytes.len() => {
                    // Drop the backslash, keep the escaped char.
                    let ch_len = utf8_char_len(bytes[i + 1]);
                    dest.replace(i, &input[i + 1..i + 1 + ch_len], i + 1 + ch_len);
                    i += 1 + ch_len;
                }
                _ => i += 1,
            }
        }
        None
    } else {
        let mut i = start;
        let mut dest = CowBuilder::new(input, start);
        let mut paren_depth = 0i32;
        while i < bytes.len() {
            let b = bytes[i];
            if b <= b' ' {
                break;
            }
            if b == b'(' {
                paren_depth += 1;
                if paren_depth > 32 {
                    return None;
                }
                i += 1;
            } else if b == b')' {
                if paren_depth == 0 {
                    break;
                }
                paren_depth -= 1;
                i += 1;
            } else if b == b'\\' && i + 1 < bytes.len() && is_ascii_punctuation(bytes[i + 1]) {
                dest.replace(i, &input[i + 1..i + 2], i + 2);
                i += 2;
            } else {
                i += 1;
            }
        }
        if paren_depth != 0 {
            return None;
        }
        let out = dest.finish(i);
        if out.is_empty() {
            return None;
        }
        Some((out, i))
    }
}

pub(super) fn parse_link_title(input: &str, start: usize) -> Option<(Cow<'_, str>, usize)> {
    let bytes = input.as_bytes();
    if start >= bytes.len() {
        return None;
    }
    let quote = bytes[start];
    let close_quote = match quote {
        b'"' => b'"',
        b'\'' => b'\'',
        b'(' => b')',
        _ => return None,
    };
    let mut i = start + 1;
    let mut title = CowBuilder::new(input, i);
    while i < bytes.len() {
        let b = bytes[i];
        if b == close_quote {
            return Some((title.finish(i), i + 1));
        }
        if b == b'(' && quote == b'(' {
            return None;
        }
        if b == b'\\' && i + 1 < bytes.len() && is_ascii_punctuation(bytes[i + 1]) {
            title.replace(i, &input[i + 1..i + 2], i + 2);
            i += 2;
        } else {
            i += 1;
        }
    }
    None
}
