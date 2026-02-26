use super::*;
use crate::{is_ascii_punctuation, utf8_char_len};

pub(super) fn parse_link_ref_def(input: &str) -> Option<(String, String, Option<String>, usize)> {
    let bytes = input.as_bytes();
    if bytes.is_empty() || bytes[0] != b'[' {
        return None;
    }

    let mut i = 1;
    let mut label = String::new();
    let mut found_close = false;
    while i < bytes.len() {
        if bytes[i] == b']' {
            found_close = true;
            i += 1;
            break;
        }
        if bytes[i] == b'[' {
            return None;
        }
        if bytes[i] == b'\\' && i + 1 < bytes.len() {
            label.push('\\');
            let ch_len = utf8_char_len(bytes[i + 1]);
            label
                .push_str(std::str::from_utf8(&bytes[i + 1..i + 1 + ch_len]).unwrap_or("\u{FFFD}"));
            i += 1 + ch_len;
        } else {
            let ch_len = utf8_char_len(bytes[i]);
            label.push_str(std::str::from_utf8(&bytes[i..i + ch_len]).unwrap_or("\u{FFFD}"));
            i += ch_len;
        }
    }
    if !found_close || label.trim().is_empty() || label.len() > 999 {
        return None;
    }

    if i >= bytes.len() || bytes[i] != b':' {
        return None;
    }
    i += 1;

    i = skip_spaces_and_optional_newline(bytes, i);

    let (dest, dest_end) = parse_link_destination(bytes, i)?;
    i = dest_end;

    let before_title = i;
    let title_start = skip_spaces_and_optional_newline(bytes, i);

    let mut title = None;

    if title_start < bytes.len() && title_start > before_title {
        if let Some((t, t_end)) = parse_link_title(bytes, title_start) {
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

pub(super) fn resolve_entities_and_escapes(s: &str) -> String {
    let bytes = s.as_bytes();
    if memchr::memchr2(b'\\', b'&', bytes).is_none() {
        return s.to_string();
    }
    let mut out = String::with_capacity(s.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'\\' && i + 1 < bytes.len() && is_ascii_punctuation(bytes[i + 1]) {
            out.push(bytes[i + 1] as char);
            i += 2;
        } else if bytes[i] == b'&' {
            if let Some(end) = resolve_entity_in_bytes(bytes, i, &mut out) {
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
    out
}

pub(super) fn resolve_entity_in_bytes(
    bytes: &[u8],
    start: usize,
    out: &mut String,
) -> Option<usize> {
    let mut i = start + 1;
    if i >= bytes.len() {
        return None;
    }

    if bytes[i] == b'#' {
        i += 1;
        let hex = i < bytes.len() && matches!(bytes[i], b'x' | b'X');
        if hex {
            i += 1;
        }
        let ns = i;
        if hex {
            while i < bytes.len() && bytes[i].is_ascii_hexdigit() {
                i += 1;
            }
        } else {
            while i < bytes.len() && bytes[i].is_ascii_digit() {
                i += 1;
            }
        }
        if i == ns || i - ns > 7 || i >= bytes.len() || bytes[i] != b';' {
            return None;
        }
        let value = std::str::from_utf8(&bytes[ns..i]).ok()?;
        i += 1;
        if entities::resolve_numeric_ref_into(value, hex, out) {
            Some(i)
        } else {
            None
        }
    } else {
        let ns = i;
        while i < bytes.len() && bytes[i].is_ascii_alphanumeric() {
            i += 1;
        }
        if i == ns || i >= bytes.len() || bytes[i] != b';' {
            return None;
        }
        let name = std::str::from_utf8(&bytes[ns..i]).ok()?;
        i += 1;
        if entities::lookup_entity_into(name, out) {
            Some(i)
        } else {
            None
        }
    }
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

pub(super) fn parse_link_destination(bytes: &[u8], start: usize) -> Option<(String, usize)> {
    if start >= bytes.len() {
        return None;
    }

    if bytes[start] == b'<' {
        let mut i = start + 1;
        let mut dest = String::new();
        while i < bytes.len() {
            if bytes[i] == b'>' {
                return Some((dest, i + 1));
            }
            if bytes[i] == b'<' || bytes[i] == b'\n' {
                return None;
            }
            if bytes[i] == b'\\' && i + 1 < bytes.len() {
                let ch_len = utf8_char_len(bytes[i + 1]);
                dest.push_str(
                    std::str::from_utf8(&bytes[i + 1..i + 1 + ch_len]).unwrap_or("\u{FFFD}"),
                );
                i += 1 + ch_len;
            } else {
                let ch_len = utf8_char_len(bytes[i]);
                dest.push_str(std::str::from_utf8(&bytes[i..i + ch_len]).unwrap_or("\u{FFFD}"));
                i += ch_len;
            }
        }
        None
    } else {
        let mut i = start;
        let mut paren_depth = 0i32;
        let mut dest = String::new();
        while i < bytes.len() {
            let b = bytes[i];
            if b == b' ' || b == b'\t' || b == b'\n' || (b < 0x20 && b != b'\t') {
                break;
            }
            if b == b'(' {
                paren_depth += 1;
                if paren_depth > 32 {
                    return None;
                }
                dest.push('(');
                i += 1;
            } else if b == b')' {
                if paren_depth == 0 {
                    break;
                }
                paren_depth -= 1;
                dest.push(')');
                i += 1;
            } else if b == b'\\' && i + 1 < bytes.len() && is_ascii_punctuation(bytes[i + 1]) {
                dest.push(bytes[i + 1] as char);
                i += 2;
            } else {
                let ch_start = i;
                i += utf8_char_len(b);
                dest.push_str(std::str::from_utf8(&bytes[ch_start..i]).unwrap_or("\u{FFFD}"));
            }
        }
        if paren_depth != 0 {
            return None;
        }
        if dest.is_empty() && start < bytes.len() && bytes[start] != b'<' {
            return None;
        }
        Some((dest, i))
    }
}

pub(super) fn parse_link_title(bytes: &[u8], start: usize) -> Option<(String, usize)> {
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
    let mut title = String::new();
    while i < bytes.len() {
        if bytes[i] == close_quote {
            return Some((title, i + 1));
        }
        if bytes[i] == b'(' && quote == b'(' {
            return None;
        }
        if bytes[i] == b'\\' && i + 1 < bytes.len() && is_ascii_punctuation(bytes[i + 1]) {
            title.push(bytes[i + 1] as char);
            i += 2;
        } else if bytes[i] == b'\n' {
            title.push('\n');
            i += 1;
        } else {
            let ch_start = i;
            i += utf8_char_len(bytes[i]);
            title.push_str(std::str::from_utf8(&bytes[ch_start..i]).unwrap_or("\u{FFFD}"));
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn atx_heading_basic() {
        assert_eq!(parse_atx_heading("# foo"), Some((1, "foo")));
        assert_eq!(parse_atx_heading("## foo"), Some((2, "foo")));
        assert_eq!(parse_atx_heading("###### foo"), Some((6, "foo")));
        assert_eq!(parse_atx_heading("####### foo"), None);
    }

    #[test]
    fn atx_heading_closing() {
        assert_eq!(parse_atx_heading("# foo ##"), Some((1, "foo")));
        assert_eq!(parse_atx_heading("## foo ##"), Some((2, "foo")));
        assert_eq!(parse_atx_heading("# foo #"), Some((1, "foo")));
    }

    #[test]
    fn thematic_break_basic() {
        assert!(is_thematic_break("***"));
        assert!(is_thematic_break("---"));
        assert!(is_thematic_break("___"));
        assert!(is_thematic_break(" * * *"));
        assert!(!is_thematic_break("--"));
    }

    #[test]
    fn fence_start_basic() {
        assert_eq!(parse_fence_start("```"), Some((b'`', 3, "")));
        assert_eq!(parse_fence_start("```rust"), Some((b'`', 3, "rust")));
        assert_eq!(parse_fence_start("~~~"), Some((b'~', 3, "")));
        assert_eq!(parse_fence_start("``"), None);
    }

    #[test]
    fn list_marker_basic() {
        let m = parse_list_marker("- foo");
        assert!(m.is_some());
        let m = m.unwrap();
        assert_eq!(m.kind, ListKind::Bullet(b'-'));

        let m = parse_list_marker("1. foo");
        assert!(m.is_some());
        let m = m.unwrap();
        assert_eq!(m.kind, ListKind::Ordered(b'.'));
        assert_eq!(m.start_num, 1);
    }
}
