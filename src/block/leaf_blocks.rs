use super::*;
use compact_str::CompactString;
use smallvec::SmallVec;

#[inline(always)]
pub(super) fn memchr_newline(bytes: &[u8], start: usize) -> usize {
    match memchr::memchr(b'\n', &bytes[start..]) {
        Some(offset) => start + offset,
        None => bytes.len(),
    }
}

#[inline]
pub(super) fn is_thematic_break(line: &str) -> bool {
    let bytes = line.as_bytes();
    let mut marker: u8 = 0;
    let mut count: u32 = 0;
    for &b in bytes {
        match b {
            b' ' | b'\t' => continue,
            b'*' | b'-' | b'_' => {
                if marker == 0 {
                    marker = b;
                } else if b != marker {
                    return false;
                }
                count += 1;
            }
            _ => return false,
        }
    }
    count >= 3
}

#[inline]
pub(super) fn parse_atx_heading(line: &str, permissive: bool) -> Option<(u8, &str)> {
    let bytes = line.as_bytes();
    if bytes.is_empty() || bytes[0] != b'#' {
        return None;
    }
    let mut level = 0u8;
    let mut i = 0;
    while i < bytes.len() && bytes[i] == b'#' && level < 7 {
        level += 1;
        i += 1;
    }
    if level > 6 {
        return None;
    }
    if i < bytes.len() && bytes[i] != b' ' && bytes[i] != b'\t' {
        if !permissive {
            return None;
        }
        // permissive: `#heading` is valid — content starts immediately after the hashes
        let raw_content = line[i..].trim();
        let content = strip_closing_hashes(raw_content);
        return Some((level, content));
    }
    let content = if i >= bytes.len() {
        ""
    } else {
        let raw_content = &line[i..].trim();
        strip_closing_hashes(raw_content)
    };
    Some((level, content))
}

pub(super) fn strip_closing_hashes(s: &str) -> &str {
    let bytes = s.as_bytes();
    if bytes.is_empty() {
        return s;
    }
    let mut end = bytes.len();
    while end > 0 && bytes[end - 1] == b'#' {
        end -= 1;
    }
    if end == bytes.len() {
        return s;
    }
    if end == 0 {
        return "";
    }
    if bytes[end - 1] == b' ' || bytes[end - 1] == b'\t' {
        let result = &s[..end];
        result.trim_end()
    } else {
        s
    }
}

#[inline]
pub(super) fn parse_setext_underline(line: &str) -> Option<u8> {
    let trimmed = line.trim();
    if trimmed.is_empty() {
        return None;
    }
    let bytes = trimmed.as_bytes();
    let ch = bytes[0];
    if ch != b'=' && ch != b'-' {
        return None;
    }
    if !bytes.iter().all(|&b| b == ch) {
        return None;
    }
    Some(if ch == b'=' { 1 } else { 2 })
}

#[inline]
pub(super) fn parse_fence_start(line: &str) -> Option<(u8, usize, &str)> {
    let bytes = line.as_bytes();
    if bytes.is_empty() {
        return None;
    }
    let ch = bytes[0];
    if ch != b'`' && ch != b'~' {
        return None;
    }
    let mut count = 0;
    let mut i = 0;
    while i < bytes.len() && bytes[i] == ch {
        count += 1;
        i += 1;
    }
    if count < 3 {
        return None;
    }
    let info = line[i..].trim();
    if ch == b'`' && info.contains('`') {
        return None;
    }
    Some((ch, count, info))
}

#[inline]
pub(super) fn is_closing_fence(line: &[u8], fence_char: u8, fence_len: usize) -> bool {
    let len = line.len();
    if len == 0 {
        return false;
    }
    let mut i = 0;
    let b0 = line[0];
    if b0 != b' ' && b0 != b'\t' && b0 != fence_char {
        return false;
    }
    while i < len && i < 3 && line[i] == b' ' {
        i += 1;
    }
    if i < len && line[i] == b'\t' && i < 4 {
        let tab_width = 4 - (i % 4);
        if i + tab_width > 3 {
            return false;
        }
        i += 1;
    }
    if i >= len || line[i] != fence_char {
        return false;
    }
    let fence_start = i;
    while i < len && line[i] == fence_char {
        i += 1;
    }
    if i - fence_start < fence_len {
        return false;
    }
    while i < len {
        if line[i] != b' ' && line[i] != b'\t' {
            return false;
        }
        i += 1;
    }
    true
}

#[inline]
pub(super) fn parse_table_separator(line: &str) -> Option<SmallVec<[TableAlignment; 8]>> {
    // Runs on every paragraph continuation line; most fail on their first letter.
    if !line
        .bytes()
        .all(|b| matches!(b, b'|' | b':' | b'-' | b' ' | b'\t'))
    {
        return None;
    }
    let trimmed = trim_space_tab(line);
    if trimmed.is_empty() {
        return None;
    }

    let inner = trimmed.strip_prefix('|').unwrap_or(trimmed);
    let inner = inner.strip_suffix('|').unwrap_or(inner);

    if trim_space_tab(inner).is_empty() {
        return None;
    }

    memchr::memchr(b'|', trimmed.as_bytes())?;

    let mut alignments = SmallVec::new();
    for cell in inner.split('|') {
        let c = trim_space_tab(cell);
        if c.is_empty() {
            return None;
        }
        let bytes = c.as_bytes();
        let left = bytes[0] == b':';
        let right = bytes[bytes.len() - 1] == b':';
        let start = if left { 1 } else { 0 };
        let end = if right { bytes.len() - 1 } else { bytes.len() };
        if start >= end {
            return None;
        }
        if !bytes[start..end].iter().all(|&b| b == b'-') {
            return None;
        }
        let alignment = match (left, right) {
            (true, true) => TableAlignment::Center,
            (true, false) => TableAlignment::Left,
            (false, true) => TableAlignment::Right,
            (false, false) => TableAlignment::None,
        };
        alignments.push(alignment);
    }

    if alignments.is_empty() {
        return None;
    }

    Some(alignments)
}

pub(crate) fn parse_table_row(line: &str, num_cols: usize) -> SmallVec<[CompactString; 8]> {
    let trimmed = trim_space_tab(line);

    let inner = trimmed.strip_prefix('|').unwrap_or(trimmed);
    let inner = inner.strip_suffix('|').unwrap_or(inner);

    let has_escaped_pipe = memchr::memchr(b'\\', inner.as_bytes()).is_some_and(|pos| {
        let bytes = inner.as_bytes();
        let mut p = pos;
        loop {
            if p + 1 < bytes.len() && bytes[p + 1] == b'|' {
                return true;
            }
            match memchr::memchr(b'\\', &bytes[p + 1..]) {
                Some(offset) => p = p + 1 + offset,
                None => return false,
            }
        }
    });

    if !has_escaped_pipe {
        let mut cells: SmallVec<[CompactString; 8]> = SmallVec::with_capacity(num_cols);
        let bytes = inner.as_bytes();
        let mut start = 0;
        while cells.len() < num_cols {
            match memchr::memchr(b'|', &bytes[start..]) {
                Some(offset) => {
                    let end = start + offset;
                    cells.push(CompactString::new(trim_space_tab(&inner[start..end])));
                    start = end + 1;
                }
                None => {
                    cells.push(CompactString::new(trim_space_tab(&inner[start..])));
                    break;
                }
            }
        }
        while cells.len() < num_cols {
            cells.push(CompactString::const_new(""));
        }
        return cells;
    }

    let mut cells: SmallVec<[CompactString; 8]> = SmallVec::with_capacity(num_cols);
    let mut current = CompactString::default();
    let bytes = inner.as_bytes();
    let mut i = 0;
    let mut seg_start = 0;
    while i < bytes.len() {
        if bytes[i] == b'\\' && i + 1 < bytes.len() && bytes[i + 1] == b'|' {
            current.push_str(&inner[seg_start..i + 2]);
            i += 2;
            seg_start = i;
        } else if bytes[i] == b'|' {
            current.push_str(&inner[seg_start..i]);
            cells.push(CompactString::new(trim_space_tab(&current)));
            current.clear();
            i += 1;
            seg_start = i;
        } else {
            i += 1;
        }
    }
    current.push_str(&inner[seg_start..]);
    cells.push(CompactString::new(trim_space_tab(&current)));

    while cells.len() < num_cols {
        cells.push(CompactString::const_new(""));
    }
    cells.truncate(num_cols);
    cells
}

#[inline(always)]
fn trim_space_tab(s: &str) -> &str {
    let bytes = s.as_bytes();
    let mut start = 0;
    let mut end = bytes.len();
    while start < end && (bytes[start] == b' ' || bytes[start] == b'\t') {
        start += 1;
    }
    while end > start && (bytes[end - 1] == b' ' || bytes[end - 1] == b'\t') {
        end -= 1;
    }
    &s[start..end]
}

#[inline(always)]
fn rest_is_blank(bytes: &[u8], from: usize) -> bool {
    let mut i = from;
    while i < bytes.len() {
        let b = bytes[i];
        if b != b' ' && b != b'\t' {
            return false;
        }
        i += 1;
    }
    true
}

#[derive(Debug, Clone)]
pub(super) struct ListMarkerInfo {
    pub kind: ListKind,
    pub marker_len: usize, // bytes consumed by the marker itself (e.g., "- " = 2, "1. " = 3)
    pub start_num: u32,
    pub is_empty_item: bool, // marker followed by nothing or only blanks
}

#[inline]
pub(super) fn parse_list_marker(line: &str) -> Option<ListMarkerInfo> {
    let bytes = line.as_bytes();
    if bytes.is_empty() {
        return None;
    }

    let b0 = bytes[0];

    if b0 == b'-' || b0 == b'*' || b0 == b'+' {
        if bytes.len() == 1 || bytes[1] == b' ' || bytes[1] == b'\t' {
            let is_empty = bytes.len() <= 2
                || ((bytes[2] == b' ' || bytes[2] == b'\t') && rest_is_blank(bytes, 2));
            return Some(ListMarkerInfo {
                kind: ListKind::Bullet(b0),
                marker_len: 1,
                start_num: 0,
                is_empty_item: is_empty,
            });
        }
        return None;
    }

    if b0.is_ascii_digit() {
        let mut i = 1;
        while i < bytes.len() && i < 9 && bytes[i].is_ascii_digit() {
            i += 1;
        }
        if i < bytes.len() && (bytes[i] == b'.' || bytes[i] == b')') {
            let delim = bytes[i];
            if i + 1 >= bytes.len() || bytes[i + 1] == b' ' || bytes[i + 1] == b'\t' {
                let num = if i <= 4 {
                    let mut n = 0u32;
                    for &digit in bytes.iter().take(i) {
                        n = n * 10 + (digit - b'0') as u32;
                    }
                    n
                } else {
                    match line[..i].parse::<u32>() {
                        Ok(n) => n,
                        Err(_) => return None,
                    }
                };
                let is_empty = i + 2 >= bytes.len()
                    || ((bytes[i + 2] == b' ' || bytes[i + 2] == b'\t')
                        && rest_is_blank(bytes, i + 2));
                return Some(ListMarkerInfo {
                    kind: ListKind::Ordered(delim),
                    marker_len: i + 1,
                    start_num: num,
                    is_empty_item: is_empty,
                });
            }
        }
    }

    None
}

#[inline]
pub(super) fn can_interrupt_paragraph(marker: &ListMarkerInfo) -> bool {
    if marker.is_empty_item {
        return false;
    }
    match marker.kind {
        ListKind::Bullet(_) => true,
        ListKind::Ordered(_) => marker.start_num == 1,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn atx_heading_basic() {
        assert_eq!(parse_atx_heading("# foo", false), Some((1, "foo")));
        assert_eq!(parse_atx_heading("## foo", false), Some((2, "foo")));
        assert_eq!(parse_atx_heading("###### foo", false), Some((6, "foo")));
        assert_eq!(parse_atx_heading("####### foo", false), None);
    }

    #[test]
    fn atx_heading_closing() {
        assert_eq!(parse_atx_heading("# foo ##", false), Some((1, "foo")));
        assert_eq!(parse_atx_heading("## foo ##", false), Some((2, "foo")));
        assert_eq!(parse_atx_heading("# foo #", false), Some((1, "foo")));
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
