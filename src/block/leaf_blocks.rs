use super::*;

#[inline(always)]
pub(super) fn memchr_newline(bytes: &[u8], start: usize) -> usize {
    match memchr::memchr(b'\n', &bytes[start..]) {
        Some(offset) => start + offset,
        None => bytes.len(),
    }
}

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

pub(super) fn parse_atx_heading(line: &str) -> Option<(u8, &str)> {
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
        return None;
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
        return s; // no trailing hashes
    }
    if end == 0 {
        return ""; // all hashes
    }
    if bytes[end - 1] == b' ' || bytes[end - 1] == b'\t' {
        let result = &s[..end];
        result.trim_end()
    } else {
        s
    }
}

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
pub(super) fn is_closing_fence(line: &str, fence_char: u8, fence_len: usize) -> bool {
    let bytes = line.as_bytes();
    let len = bytes.len();
    let mut i = 0;
    while i < len && i < 3 && bytes[i] == b' ' {
        i += 1;
    }
    if i < len && bytes[i] == b'\t' && i < 4 {
        let tab_width = 4 - (i % 4);
        if i + tab_width > 3 {
            return false;
        }
        i += 1;
    }
    if i >= len || bytes[i] != fence_char {
        return false;
    }
    let fence_start = i;
    while i < len && bytes[i] == fence_char {
        i += 1;
    }
    if i - fence_start < fence_len {
        return false;
    }
    while i < len {
        if bytes[i] != b' ' && bytes[i] != b'\t' {
            return false;
        }
        i += 1;
    }
    true
}

/// Parse a GFM table separator line like `| --- | :---: | ---: |`
/// Returns alignments if valid, None otherwise.
pub(super) fn parse_table_separator(line: &str) -> Option<Vec<TableAlignment>> {
    let trimmed = line.trim();
    if trimmed.is_empty() {
        return None;
    }

    let inner = trimmed.strip_prefix('|').unwrap_or(trimmed);
    let inner = inner.strip_suffix('|').unwrap_or(inner);

    if inner.trim().is_empty() {
        return None;
    }

    let cells: Vec<&str> = inner.split('|').collect();
    if cells.is_empty() {
        return None;
    }

    let mut alignments = Vec::new();
    for cell in &cells {
        let c = cell.trim();
        if c.is_empty() {
            return None;
        }
        let left = c.starts_with(':');
        let right = c.ends_with(':');
        let dashes = if left && right {
            &c[1..c.len() - 1]
        } else if left {
            &c[1..]
        } else if right {
            &c[..c.len() - 1]
        } else {
            c
        };
        if dashes.is_empty() || !dashes.bytes().all(|b| b == b'-') {
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

    if !trimmed.contains('|') {
        return None;
    }

    Some(alignments)
}

/// Parse a table row into cells, trimming each cell and padding/truncating to `num_cols`.
pub(super) fn parse_table_row(line: &str, num_cols: usize) -> Vec<String> {
    let trimmed = line.trim();

    let inner = trimmed.strip_prefix('|').unwrap_or(trimmed);
    let inner = inner.strip_suffix('|').unwrap_or(inner);
    let has_escaped_pipe = {
        let bytes = inner.as_bytes();
        let mut j = 0;
        let mut found = false;
        while j + 1 < bytes.len() {
            if bytes[j] == b'\\' && bytes[j + 1] == b'|' {
                found = true;
                break;
            }
            j += 1;
        }
        found
    };

    if !has_escaped_pipe {
        let mut cells: Vec<String> = inner.split('|').map(|s| s.trim().to_string()).collect();
        cells.resize(num_cols, String::new());
        cells.truncate(num_cols);
        return cells;
    }

    let mut cells = Vec::new();
    let mut current = String::new();
    let bytes = inner.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'\\' && i + 1 < bytes.len() && bytes[i + 1] == b'|' {
            current.push('\\');
            current.push('|');
            i += 2;
        } else if bytes[i] == b'|' {
            cells.push(current.trim().to_string());
            current = String::new();
            i += 1;
        } else {
            current.push(bytes[i] as char);
            i += 1;
        }
    }
    cells.push(current.trim().to_string());

    cells.resize(num_cols, String::new());
    cells.truncate(num_cols);
    cells
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
            let is_empty = if bytes.len() <= 1 {
                true
            } else {
                let mut j = 1;
                loop {
                    if j >= bytes.len() {
                        break true;
                    }
                    match bytes[j] {
                        b' ' | b'\t' => j += 1,
                        _ => break false,
                    }
                }
            };
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
                    for j in 0..i {
                        n = n * 10 + (bytes[j] - b'0') as u32;
                    }
                    n
                } else {
                    match line[..i].parse::<u32>() {
                        Ok(n) => n,
                        Err(_) => return None,
                    }
                };
                let is_empty = if i + 1 >= bytes.len() {
                    true
                } else {
                    let mut j = i + 1;
                    loop {
                        if j >= bytes.len() {
                            break true;
                        }
                        match bytes[j] {
                            b' ' | b'\t' => j += 1,
                            _ => break false,
                        }
                    }
                };
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

pub(super) fn can_interrupt_paragraph(marker: &ListMarkerInfo) -> bool {
    if marker.is_empty_item {
        return false;
    }
    match marker.kind {
        ListKind::Bullet(_) => true,
        ListKind::Ordered(_) => marker.start_num == 1,
    }
}
