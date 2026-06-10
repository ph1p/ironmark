use super::*;

#[inline]
pub(super) fn count_backtick_hint_from(bytes: &[u8], mut start: usize, mut count: u8) -> u8 {
    while start < bytes.len() && count < 3 {
        let Some(off) = memchr::memchr(b'`', &bytes[start..]) else {
            break;
        };
        count += 1;
        start += off + 1;
    }
    count
}

pub(super) fn has_matching_backtick_runs(bytes: &[u8]) -> bool {
    let mut seen_short = [false; 64];
    let mut seen_long: Option<FxHashMap<u32, ()>> = None;
    let mut i = 0usize;

    while i < bytes.len() {
        let Some(off) = memchr::memchr(b'`', &bytes[i..]) else {
            break;
        };
        i += off;
        let run_start = i;
        i += 1;
        while i < bytes.len() && bytes[i] == b'`' {
            i += 1;
        }
        let run_len = i - run_start;
        if run_len < seen_short.len() {
            let slot = &mut seen_short[run_len];
            if *slot {
                return true;
            }
            *slot = true;
        } else {
            let map = seen_long.get_or_insert_with(FxHashMap::default);
            let key = run_len as u32;
            if map.contains_key(&key) {
                return true;
            }
            map.insert(key, ());
        }
    }

    false
}

pub(super) fn try_emit_common_inline(out: &mut String, raw: &str, opts: &ParseOptions) -> bool {
    if raw.len() > 256 || raw.is_empty() || opts.collapse_whitespace || raw.contains('\n') {
        return false;
    }

    let bytes = raw.as_bytes();
    let len = bytes.len();
    let mut text_start = 0usize;
    let mut seg_needs_escape = false;
    let mut i = 0usize;

    out.clear();
    out.reserve(raw.len() + 16);

    macro_rules! flush_text {
        ($at:expr) => {
            if text_start < $at {
                if seg_needs_escape {
                    escape_html_into(out, &raw[text_start..$at]);
                } else {
                    out.push_str(&raw[text_start..$at]);
                }
            }
        };
    }

    while i < len {
        match bytes[i] {
            b'\\' | b'&' | b'<' | b'!' | b'_' => return false,
            b'>' => {
                seg_needs_escape = true;
                i += 1;
            }
            b':' | b'@' if opts.enable_autolink => return false,
            b'$' if opts.enable_latex_math => return false,
            b'[' => {
                flush_text!(i);
                let Some(next_i) = emit_simple_link(out, raw, opts, i, i) else {
                    return false;
                };
                i = next_i;
                text_start = i;
                seg_needs_escape = false;
            }
            b'`' => {
                flush_text!(i);
                let Some(next_i) = emit_simple_code(out, raw, i, i) else {
                    return false;
                };
                i = next_i;
                text_start = i;
                seg_needs_escape = false;
            }
            b'*' => {
                flush_text!(i);
                let Some(next_i) = emit_simple_delim(
                    out,
                    raw,
                    i,
                    i,
                    b'*',
                    "<em>",
                    "</em>",
                    "<strong>",
                    "</strong>",
                ) else {
                    return false;
                };
                i = next_i;
                text_start = i;
                seg_needs_escape = false;
            }
            b'~' if opts.enable_strikethrough => {
                flush_text!(i);
                let Some(next_i) =
                    emit_simple_double_delim(out, raw, i, i, b'~', "<del>", "</del>")
                else {
                    return false;
                };
                i = next_i;
                text_start = i;
                seg_needs_escape = false;
            }
            b'=' if opts.enable_highlight => {
                flush_text!(i);
                let Some(next_i) =
                    emit_simple_double_delim(out, raw, i, i, b'=', "<mark>", "</mark>")
                else {
                    return false;
                };
                i = next_i;
                text_start = i;
                seg_needs_escape = false;
            }
            b'+' if opts.enable_underline => {
                flush_text!(i);
                let Some(next_i) = emit_simple_double_delim(out, raw, i, i, b'+', "<u>", "</u>")
                else {
                    return false;
                };
                i = next_i;
                text_start = i;
                seg_needs_escape = false;
            }
            _ => {
                i += 1;
            }
        }
    }

    if text_start < len {
        if seg_needs_escape {
            escape_html_into(out, &raw[text_start..]);
        } else {
            out.push_str(&raw[text_start..]);
        }
    }
    true
}

pub(super) fn emit_simple_code(
    out: &mut String,
    raw: &str,
    text_start: usize,
    at: usize,
) -> Option<usize> {
    let bytes = raw.as_bytes();
    if at + 1 < bytes.len() && bytes[at + 1] == b'`' {
        return None;
    }
    let close_rel = memchr::memchr(b'`', &bytes[at + 1..])?;
    let close = at + 1 + close_rel;
    let content = &raw[at + 1..close];
    if content.is_empty()
        || content.contains('\n')
        || content.as_bytes()[0] == b' '
        || content.as_bytes()[content.len() - 1] == b' '
    {
        return None;
    }

    if text_start < at {
        escape_html_into(out, &raw[text_start..at]);
    }
    out.push_str("<code>");
    escape_html_into(out, content);
    out.push_str("</code>");
    Some(close + 1)
}

pub(super) fn emit_simple_link(
    out: &mut String,
    raw: &str,
    opts: &ParseOptions,
    text_start: usize,
    at: usize,
) -> Option<usize> {
    let bytes = raw.as_bytes();
    let close_bracket_rel = memchr::memchr(b']', &bytes[at + 1..])?;
    let close_bracket = at + 1 + close_bracket_rel;
    if close_bracket + 1 >= bytes.len() || bytes[close_bracket + 1] != b'(' {
        return None;
    }
    let close_paren_rel = memchr::memchr(b')', &bytes[close_bracket + 2..])?;
    let close_paren = close_bracket + 2 + close_paren_rel;
    let label = &raw[at + 1..close_bracket];
    let dest = &raw[close_bracket + 2..close_paren];

    if !is_simple_link_label(label) || !is_simple_link_dest(dest, opts) {
        return None;
    }

    if text_start < at {
        escape_html_into(out, &raw[text_start..at]);
    }
    out.push_str("<a href=\"");
    if !is_dangerous_url(dest) {
        encode_url_escaped_into(out, dest);
    }
    out.push_str("\">");
    escape_html_into(out, label);
    out.push_str("</a>");
    Some(close_paren + 1)
}

#[allow(clippy::too_many_arguments)]
pub(super) fn emit_simple_delim(
    out: &mut String,
    raw: &str,
    text_start: usize,
    at: usize,
    marker: u8,
    single_open: &str,
    single_close: &str,
    double_open: &str,
    double_close: &str,
) -> Option<usize> {
    let bytes = raw.as_bytes();
    let count = if at + 1 < bytes.len() && bytes[at + 1] == marker {
        2
    } else {
        1
    };

    let before = if at > 0 { char_before(raw, at) } else { ' ' };
    let after = if at + count < bytes.len() {
        char_at(raw, at + count)
    } else {
        ' '
    };
    let (can_open, _) = flanking(marker, before, after);
    if !can_open {
        return None;
    }

    let close = find_simple_delim_close(raw, at + count, marker, count)?;
    let content = &raw[at + count..close];
    if !is_simple_delim_content(content) {
        return None;
    }

    if text_start < at {
        escape_html_into(out, &raw[text_start..at]);
    }
    let (open, close_tag) = if count == 2 {
        (double_open, double_close)
    } else {
        (single_open, single_close)
    };
    out.push_str(open);
    escape_html_into(out, content);
    out.push_str(close_tag);
    Some(close + count)
}

pub(super) fn emit_simple_double_delim(
    out: &mut String,
    raw: &str,
    text_start: usize,
    at: usize,
    marker: u8,
    open: &str,
    close_tag: &str,
) -> Option<usize> {
    let bytes = raw.as_bytes();
    if at + 1 >= bytes.len() || bytes[at + 1] != marker {
        return None;
    }
    let close = find_simple_delim_close(raw, at + 2, marker, 2)?;
    let content = &raw[at + 2..close];
    if !is_simple_delim_content(content) {
        return None;
    }

    if text_start < at {
        escape_html_into(out, &raw[text_start..at]);
    }
    out.push_str(open);
    escape_html_into(out, content);
    out.push_str(close_tag);
    Some(close + 2)
}

pub(super) fn find_simple_delim_close(
    raw: &str,
    mut from: usize,
    marker: u8,
    count: usize,
) -> Option<usize> {
    let bytes = raw.as_bytes();
    while from < bytes.len() {
        let rel = memchr::memchr(marker, &bytes[from..])?;
        let idx = from + rel;
        if count == 2 && (idx + 1 >= bytes.len() || bytes[idx + 1] != marker) {
            from = idx + 1;
            continue;
        }
        let before = char_before(raw, idx);
        let after = if idx + count < bytes.len() {
            char_at(raw, idx + count)
        } else {
            ' '
        };
        let (_, can_close) = flanking(marker, before, after);
        if can_close {
            return Some(idx);
        }
        from = idx + 1;
    }
    None
}

/// Build a 256-entry byte lookup table marking each byte in `forbidden` true.
const fn forbidden_table(forbidden: &[u8]) -> [bool; 256] {
    let mut t = [false; 256];
    let mut i = 0;
    while i < forbidden.len() {
        t[forbidden[i] as usize] = true;
        i += 1;
    }
    t
}

/// Fast lookup: true for forbidden bytes in simple delimiter content.
static SIMPLE_DELIM_FORBIDDEN: [bool; 256] = forbidden_table(br"\&<![]`*_~=+");

/// Fast lookup: true for forbidden bytes in simple link labels
/// (the delimiter set plus `:` `@` `$`).
static SIMPLE_LABEL_FORBIDDEN: [bool; 256] = forbidden_table(br"\&<![]`*_~=+:@$");

/// Fast lookup: true for forbidden bytes in simple link destinations.
/// `$` is only meaningful when latex math is enabled — checked at the use site.
static SIMPLE_DEST_FORBIDDEN: [bool; 256] = forbidden_table(b" \t\n<>&\\()$");

#[inline]
pub(super) fn is_simple_delim_content(content: &str) -> bool {
    !content.is_empty()
        && !content
            .as_bytes()
            .iter()
            .any(|&b| SIMPLE_DELIM_FORBIDDEN[b as usize])
}

#[inline]
pub(super) fn is_simple_link_label(label: &str) -> bool {
    !label.is_empty()
        && !label
            .as_bytes()
            .iter()
            .any(|&b| SIMPLE_LABEL_FORBIDDEN[b as usize])
}

#[inline]
pub(super) fn is_simple_link_dest(dest: &str, opts: &ParseOptions) -> bool {
    !dest.is_empty()
        && !dest
            .as_bytes()
            .iter()
            .any(|&b| SIMPLE_DEST_FORBIDDEN[b as usize] && (b != b'$' || opts.enable_latex_math))
}
