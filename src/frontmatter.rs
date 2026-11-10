//! Front matter detection.

/// Split a leading front matter block off `markdown`.
///
/// Front matter opens on the very first line (after an optional BOM) with
/// `---` (YAML) or `+++` (TOML) and closes with the same fence on a line of its
/// own; YAML may also close with `...`. Trailing spaces and tabs on fence lines
/// are allowed. Returns `(raw, body)`: the text between the fences, unparsed,
/// and the Markdown after the closing fence. Returns `None` when there is no
/// complete block.
///
/// # Examples
///
/// ```
/// use ironmark::split_frontmatter;
///
/// let (raw, body) = split_frontmatter("---\ntitle: Hi\n---\n# Body").unwrap();
/// assert_eq!(raw, "title: Hi\n");
/// assert_eq!(body, "# Body");
/// assert!(split_frontmatter("# No front matter").is_none());
/// ```
pub fn split_frontmatter(markdown: &str) -> Option<(&str, &str)> {
    let src = markdown.strip_prefix('\u{feff}').unwrap_or(markdown);
    let fence = if src.starts_with("---") {
        "---"
    } else if src.starts_with("+++") {
        "+++"
    } else {
        return None;
    };
    let (first, mut rest) = line(src);
    if fence_text(first) != fence || !first.ends_with('\n') {
        return None;
    }
    let start = src.len() - rest.len();
    while !rest.is_empty() {
        let at = src.len() - rest.len();
        let (current, after) = line(rest);
        let trimmed = fence_text(current);
        if trimmed == fence || fence == "---" && trimmed == "..." {
            return Some((&src[start..at], after));
        }
        rest = after;
    }
    None
}

/// A line without its line ending and trailing spaces or tabs.
fn fence_text(line: &str) -> &str {
    line.trim_end_matches(['\r', '\n'])
        .trim_end_matches([' ', '\t'])
}

/// The first line of `s` including its line ending, and the remainder.
fn line(s: &str) -> (&str, &str) {
    let end = memchr::memchr(b'\n', s.as_bytes()).map_or(s.len(), |n| n + 1);
    s.split_at(end)
}

#[cfg(test)]
mod tests {
    use super::split_frontmatter;

    #[test]
    fn yaml_and_toml() {
        assert_eq!(
            split_frontmatter("---\na: 1\n---\nbody"),
            Some(("a: 1\n", "body"))
        );
        assert_eq!(
            split_frontmatter("+++\na = 1\n+++\n"),
            Some(("a = 1\n", ""))
        );
        assert_eq!(
            split_frontmatter("---\na: 1\n...\nx"),
            Some(("a: 1\n", "x"))
        );
        assert_eq!(split_frontmatter("---\n---"), Some(("", "")));
        assert_eq!(
            split_frontmatter("\u{feff}---  \r\na: 1\r\n---\r\nbody"),
            Some(("a: 1\r\n", "body"))
        );
    }

    #[test]
    fn rejects_non_front_matter() {
        assert_eq!(split_frontmatter("---"), None);
        assert_eq!(split_frontmatter("---\na: 1\n"), None);
        assert_eq!(split_frontmatter("----\na\n----\n"), None);
        assert_eq!(split_frontmatter("+++\na\n...\n"), None);
        assert_eq!(split_frontmatter(" ---\na\n---\n"), None);
        assert_eq!(split_frontmatter("# x\n---\na\n---\n"), None);
    }
}
