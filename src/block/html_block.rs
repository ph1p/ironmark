use super::*;

pub(super) static HTML_BLOCK_TYPE6_TAGS: &[&str] = &[
    "address",
    "article",
    "aside",
    "base",
    "basefont",
    "blockquote",
    "body",
    "caption",
    "center",
    "col",
    "colgroup",
    "dd",
    "details",
    "dialog",
    "dir",
    "div",
    "dl",
    "dt",
    "fieldset",
    "figcaption",
    "figure",
    "footer",
    "form",
    "frame",
    "frameset",
    "h1",
    "h2",
    "h3",
    "h4",
    "h5",
    "h6",
    "head",
    "header",
    "hr",
    "html",
    "iframe",
    "legend",
    "li",
    "link",
    "main",
    "menu",
    "menuitem",
    "nav",
    "noframes",
    "ol",
    "optgroup",
    "option",
    "p",
    "param",
    "search",
    "section",
    "summary",
    "table",
    "tbody",
    "td",
    "template",
    "tfoot",
    "th",
    "thead",
    "title",
    "tr",
    "track",
    "ul",
];

pub(super) fn starts_with_tag_ci(bytes: &[u8], tag: &[u8]) -> bool {
    if bytes.len() < 1 + tag.len() {
        return false;
    }
    if bytes[0] != b'<' {
        return false;
    }
    for i in 0..tag.len() {
        if bytes[1 + i].to_ascii_lowercase() != tag[i] {
            return false;
        }
    }
    let after = bytes.get(1 + tag.len()).copied();
    matches!(
        after,
        None | Some(b' ') | Some(b'\t') | Some(b'>') | Some(b'\n')
    )
}

pub(super) fn parse_html_block_start(line: &str, in_paragraph: bool) -> Option<HtmlBlockEnd> {
    let bytes = line.as_bytes();
    if bytes.is_empty() || bytes[0] != b'<' {
        return None;
    }

    // Type 1: <pre, <script, <style, <textarea (case-insensitive)
    if starts_with_tag_ci(bytes, b"pre") {
        return Some(HtmlBlockEnd::EndTag("</pre>"));
    }
    if starts_with_tag_ci(bytes, b"script") {
        return Some(HtmlBlockEnd::EndTag("</script>"));
    }
    if starts_with_tag_ci(bytes, b"style") {
        return Some(HtmlBlockEnd::EndTag("</style>"));
    }
    if starts_with_tag_ci(bytes, b"textarea") {
        return Some(HtmlBlockEnd::EndTag("</textarea>"));
    }

    // Type 2: <!-- (HTML comment)
    if bytes.len() >= 4 && &bytes[..4] == b"<!--" {
        return Some(HtmlBlockEnd::Comment);
    }

    // Type 3: <?
    if bytes.len() >= 2 && &bytes[..2] == b"<?" {
        return Some(HtmlBlockEnd::ProcessingInstruction);
    }

    // Type 4: <! followed by ASCII letter
    if bytes.len() >= 2 && bytes[0] == b'<' && bytes[1] == b'!' {
        if bytes.len() > 2 && bytes[2].is_ascii_alphabetic() {
            return Some(HtmlBlockEnd::Declaration);
        }
    }

    // Type 5: <![CDATA[
    if bytes.len() >= 9 && &bytes[..9] == b"<![CDATA[" {
        return Some(HtmlBlockEnd::Cdata);
    }

    // Type 6: block-level tag (open or close)
    if let Some(_) = check_html_block_type6(line) {
        return Some(HtmlBlockEnd::BlankLine);
    }

    // Type 7: any complete open/close tag alone on a line (cannot interrupt paragraph)
    if !in_paragraph {
        if is_html_block_type7(line) {
            return Some(HtmlBlockEnd::BlankLine);
        }
    }

    None
}

#[inline]
pub(super) fn check_html_block_type6(line: &str) -> Option<()> {
    let bytes = line.as_bytes();
    if bytes.len() < 2 || bytes[0] != b'<' {
        return None;
    }
    let start = if bytes[1] == b'/' { 2 } else { 1 };
    let mut end = start;
    while end < bytes.len() && bytes[end].is_ascii_alphanumeric() {
        end += 1;
    }
    if end == start {
        return None;
    }
    if end < bytes.len() {
        let next = bytes[end];
        if !(next == b' ' || next == b'\t' || next == b'>' || next == b'/' || next == b'\n') {
            return None;
        }
    }
    let tag_len = end - start;
    if tag_len > 10 {
        return None;
    }
    let mut buf = [0u8; 10];
    for i in 0..tag_len {
        buf[i] = bytes[start + i].to_ascii_lowercase();
    }
    let lc_tag = &buf[..tag_len];
    if HTML_BLOCK_TYPE6_TAGS
        .binary_search_by(|t| t.as_bytes().cmp(lc_tag))
        .is_ok()
    {
        Some(())
    } else {
        None
    }
}

pub(super) fn is_html_block_type7(line: &str) -> bool {
    let bytes = line.as_bytes();
    if bytes.len() < 3 || bytes[0] != b'<' {
        return false;
    }

    let is_close = bytes[1] == b'/';
    let start = if is_close { 2 } else { 1 };

    let mut i = start;
    if i >= bytes.len() || !bytes[i].is_ascii_alphabetic() {
        return false;
    }
    while i < bytes.len() && (bytes[i].is_ascii_alphanumeric() || bytes[i] == b'-') {
        i += 1;
    }

    if is_close {
        while i < bytes.len() && (bytes[i] == b' ' || bytes[i] == b'\t') {
            i += 1;
        }
        if i >= bytes.len() || bytes[i] != b'>' {
            return false;
        }
        i += 1;
    } else {
        loop {
            let had_space = {
                let before = i;
                while i < bytes.len() && (bytes[i] == b' ' || bytes[i] == b'\t') {
                    i += 1;
                }
                i > before
            };
            if i >= bytes.len() {
                return false;
            }
            if bytes[i] == b'>' {
                i += 1;
                break;
            }
            if bytes[i] == b'/' {
                i += 1;
                if i >= bytes.len() || bytes[i] != b'>' {
                    return false;
                }
                i += 1;
                break;
            }
            if !had_space {
                return false;
            }
            if !bytes[i].is_ascii_alphabetic() && bytes[i] != b'_' && bytes[i] != b':' {
                return false;
            }
            while i < bytes.len()
                && (bytes[i].is_ascii_alphanumeric()
                    || matches!(bytes[i], b'_' | b':' | b'.' | b'-'))
            {
                i += 1;
            }
            while i < bytes.len() && (bytes[i] == b' ' || bytes[i] == b'\t') {
                i += 1;
            }
            if i < bytes.len() && bytes[i] == b'=' {
                i += 1;
                while i < bytes.len() && (bytes[i] == b' ' || bytes[i] == b'\t') {
                    i += 1;
                }
                if i >= bytes.len() {
                    return false;
                }
                if bytes[i] == b'\'' || bytes[i] == b'"' {
                    let quote = bytes[i];
                    i += 1;
                    while i < bytes.len() && bytes[i] != quote {
                        i += 1;
                    }
                    if i >= bytes.len() {
                        return false;
                    }
                    i += 1;
                } else {
                    while i < bytes.len()
                        && !matches!(
                            bytes[i],
                            b' ' | b'\t' | b'"' | b'\'' | b'=' | b'<' | b'>' | b'`'
                        )
                    {
                        i += 1;
                    }
                }
            }
        }
    }

    while i < bytes.len() {
        if bytes[i] != b' ' && bytes[i] != b'\t' {
            return false;
        }
        i += 1;
    }
    true
}

pub(super) fn contains_ci(haystack: &[u8], needle: &[u8]) -> bool {
    if needle.len() > haystack.len() {
        return false;
    }
    let end = haystack.len() - needle.len() + 1;
    'outer: for i in 0..end {
        for j in 0..needle.len() {
            if haystack[i + j].to_ascii_lowercase() != needle[j] {
                continue 'outer;
            }
        }
        return true;
    }
    false
}

pub(super) fn html_block_ends(condition: &HtmlBlockEnd, line: &str) -> bool {
    match condition {
        HtmlBlockEnd::EndTag(tag) => contains_ci(line.as_bytes(), tag.as_bytes()),
        HtmlBlockEnd::Comment => line.contains("-->"),
        HtmlBlockEnd::ProcessingInstruction => line.contains("?>"),
        HtmlBlockEnd::Declaration => line.contains('>'),
        HtmlBlockEnd::Cdata => line.contains("]]>"),
        HtmlBlockEnd::BlankLine => false,
    }
}
