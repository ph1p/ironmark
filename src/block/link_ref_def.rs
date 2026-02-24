use super::*;

pub(super) fn parse_link_ref_def(input: &str) -> Option<(String, String, Option<String>, usize)> {
    let bytes = input.as_bytes();
    if bytes.is_empty() || bytes[0] != b'[' {
        return None;
    }

    // Parse label - preserve backslashes (matching uses raw label text)
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

    // Must be followed by :
    if i >= bytes.len() || bytes[i] != b':' {
        return None;
    }
    i += 1;

    // Skip optional whitespace (including up to one newline)
    i = skip_spaces_and_optional_newline(bytes, i);

    // Parse destination
    let (dest, dest_end) = parse_link_destination(bytes, i)?;
    i = dest_end;

    // Check for title
    let before_title = i;
    // Skip spaces (and optional newline)
    let title_start = skip_spaces_and_optional_newline(bytes, i);

    let mut title = None;

    if title_start < bytes.len() && title_start > before_title {
        if let Some((t, t_end)) = parse_link_title(bytes, title_start) {
            // After title, rest of line must be blank
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

    // No title - rest of line after dest must be blank
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

/// Resolve HTML entity references and backslash escapes in a string
pub(super) fn resolve_entities_and_escapes(s: &str) -> String {
    let bytes = s.as_bytes();
    let mut out = String::with_capacity(s.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'\\' && i + 1 < bytes.len() && is_ascii_punctuation(bytes[i + 1]) {
            out.push(bytes[i + 1] as char);
            i += 2;
        } else if bytes[i] == b'&' {
            // Try to resolve entity
            if let Some((resolved, end)) = try_resolve_entity_in_bytes(bytes, i) {
                out.push_str(&resolved);
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

pub(super) fn try_resolve_entity_in_bytes(bytes: &[u8], start: usize) -> Option<(String, usize)> {
    let mut i = start + 1; // skip &
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
        i += 1; // skip ;
        entities::resolve_numeric_ref(value, hex).map(|r| (r, i))
    } else {
        let ns = i;
        while i < bytes.len() && bytes[i].is_ascii_alphanumeric() {
            i += 1;
        }
        if i == ns || i >= bytes.len() || bytes[i] != b';' {
            return None;
        }
        let name = std::str::from_utf8(&bytes[ns..i]).ok()?;
        i += 1; // skip ;
        entities::lookup_entity(name).map(|r| (r, i))
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
        // Angle-bracket destination
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
        // Regular destination - balanced parentheses
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
                // Handle multi-byte UTF-8
                let ch_start = i;
                i += utf8_char_len(b);
                dest.push_str(std::str::from_utf8(&bytes[ch_start..i]).unwrap_or("\u{FFFD}"));
            }
        }
        if paren_depth != 0 {
            return None;
        }
        if dest.is_empty() && start < bytes.len() && bytes[start] != b'<' {
            // Empty destination without angle brackets is not valid
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
        if bytes[i] == close_quote && quote != b'(' {
            return Some((title, i + 1));
        }
        if bytes[i] == b')' && quote == b'(' {
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

#[inline(always)]
pub(super) fn is_ascii_punctuation(b: u8) -> bool {
    matches!(b, b'!'..=b'/' | b':'..=b'@' | b'['..=b'`' | b'{'..=b'~')
}

#[inline(always)]
pub(super) fn utf8_char_len(first: u8) -> usize {
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
