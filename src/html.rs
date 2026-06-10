#[cfg(test)]
pub(crate) fn escape_html(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    escape_html_into(&mut out, input);
    out
}

/// Lookup table: maps byte → escape string index (0 = no escape needed).
static HTML_ESCAPE: [u8; 256] = {
    let mut t = [0u8; 256];
    t[b'&' as usize] = 1;
    t[b'<' as usize] = 2;
    t[b'>' as usize] = 3;
    t[b'"' as usize] = 4;
    t
};

static HTML_ESCAPE_STRS: [&str; 5] = ["", "&amp;", "&lt;", "&gt;", "&quot;"];

#[inline]
pub(crate) fn escape_html_into(out: &mut String, input: &str) {
    let bytes = input.as_bytes();
    let len = bytes.len();

    if len <= 64 {
        escape_html_short(out, input, bytes, len);
    } else {
        escape_html_long(out, input, bytes, len);
    }
}

#[inline(always)]
fn escape_html_short(out: &mut String, input: &str, bytes: &[u8], len: usize) {
    // Single-pass byte-loop with lookup table: cheaper than two memchr calls for short strings.
    let mut last = 0;
    let mut i = 0;
    while i < len {
        let idx = HTML_ESCAPE[bytes[i] as usize];
        if idx != 0 {
            out.push_str(&input[last..i]);
            out.push_str(HTML_ESCAPE_STRS[idx as usize]);
            last = i + 1;
        }
        i += 1;
    }
    out.push_str(&input[last..len]);
}

#[inline]
fn escape_html_long(out: &mut String, input: &str, bytes: &[u8], len: usize) {
    // Two scan cursors carried across iterations (nearest `&<>` hit and nearest
    // `"` hit) so each byte region is scanned at most once per cursor — the
    // initial probes double as the nothing-to-escape fast exit.
    let mut next3 = memchr::memchr3(b'&', b'<', b'>', bytes);
    let mut nextq = memchr::memchr(b'"', bytes);
    if next3.is_none() && nextq.is_none() {
        out.push_str(input);
        return;
    }
    let mut last = 0;
    loop {
        let i = match (next3, nextq) {
            (Some(a), Some(q)) => a.min(q),
            (Some(a), None) => a,
            (None, Some(q)) => q,
            (None, None) => break,
        };
        out.push_str(&input[last..i]);
        out.push_str(HTML_ESCAPE_STRS[HTML_ESCAPE[bytes[i] as usize] as usize]);
        last = i + 1;
        if next3 == Some(i) {
            next3 = memchr::memchr3(b'&', b'<', b'>', &bytes[last..]).map(|o| last + o);
        } else {
            nextq = memchr::memchr(b'"', &bytes[last..]).map(|o| last + o);
        }
    }
    out.push_str(&input[last..len]);
}

static HEX_CHARS: &[u8; 16] = b"0123456789ABCDEF";

static URL_HTML_SAFE: [bool; 256] = {
    let mut t = [false; 256];
    let ranges: &[(u8, u8)] = &[(b'A', b'Z'), (b'a', b'z'), (b'0', b'9')];
    let mut r = 0;
    while r < 3 {
        let mut i = ranges[r].0;
        while i <= ranges[r].1 {
            t[i as usize] = true;
            i += 1;
        }
        r += 1;
    }
    let extra = b"-_.~!*'();/?:@=+$,#";
    let mut j = 0;
    while j < extra.len() {
        t[extra[j] as usize] = true;
        j += 1;
    }
    t
};

#[inline]
pub(crate) fn encode_url_escaped_into(out: &mut String, url: &str) {
    let bytes = url.as_bytes();
    let len = bytes.len();

    if bytes.iter().all(|&b| URL_HTML_SAFE[b as usize]) {
        out.push_str(url);
        return;
    }

    let mut last = 0;
    let mut i = 0;

    while i < len {
        let b = bytes[i];
        if URL_HTML_SAFE[b as usize] {
            i += 1;
            while i < len && URL_HTML_SAFE[bytes[i] as usize] {
                i += 1;
            }
            continue;
        }
        if b == b'%'
            && i + 2 < len
            && bytes[i + 1].is_ascii_hexdigit()
            && bytes[i + 2].is_ascii_hexdigit()
        {
            i += 3;
            continue;
        }
        if b == b'&' {
            if last < i {
                out.push_str(&url[last..i]);
            }
            out.push_str("&amp;");
            i += 1;
            last = i;
            continue;
        }
        if last < i {
            out.push_str(&url[last..i]);
        }
        let ch_len = crate::utf8_char_len(b);
        for j in 0..ch_len {
            if i + j < len {
                let b = bytes[i + j];
                out.push('%');
                out.push(HEX_CHARS[(b >> 4) as usize] as char);
                out.push(HEX_CHARS[(b & 0xF) as usize] as char);
            }
        }
        i += ch_len;
        last = i;
    }

    if last < len {
        out.push_str(&url[last..len]);
    }
}

/// Collapse runs of spaces/tabs in `input` to a single space, then HTML-escape the result,
/// in one pass. Newlines are left untouched (they are handled as `SoftBreak`/`HardBreak`
/// inline items before this function is ever called).
#[inline]
pub(crate) fn collapse_and_escape_into(out: &mut String, input: &str) {
    let bytes = input.as_bytes();
    let len = bytes.len();
    let mut i = 0;
    let mut last = 0; // start of the current verbatim-copy slice

    while i < len {
        let b = bytes[i];
        if b == b' ' || b == b'\t' {
            if last < i {
                out.push_str(&input[last..i]);
            }
            while i < len && (bytes[i] == b' ' || bytes[i] == b'\t') {
                i += 1;
            }
            // Emit a single space (skip leading/trailing whitespace runs).
            if !out.is_empty() && i < len {
                out.push(' ');
            }
            last = i;
        } else {
            let idx = HTML_ESCAPE[b as usize];
            if idx != 0 {
                if last < i {
                    out.push_str(&input[last..i]);
                }
                out.push_str(HTML_ESCAPE_STRS[idx as usize]);
                last = i + 1;
            }
            i += 1;
        }
    }
    if last < len {
        out.push_str(&input[last..len]);
    }
}

/// GFM tag filter: the 9 tags that GitHub escapes even when raw HTML is allowed.
/// See: <https://github.github.com/gfm/#disallowed-raw-html-extension->
static GFM_FILTERED_TAGS: [&[u8]; 9] = [
    b"title",
    b"textarea",
    b"style",
    b"xmp",
    b"iframe",
    b"noembed",
    b"noframes",
    b"script",
    b"plaintext",
];

/// Returns `true` when `html` starts with a tag whose name is in the GFM filter list.
/// Handles both opening (`<script`) and closing (`</script`) tags.
/// Allocation-free: compares bytes directly.
#[inline]
pub(crate) fn gfm_tag_is_filtered(html: &str) -> bool {
    let bytes = html.as_bytes();
    if bytes.is_empty() || bytes[0] != b'<' {
        return false;
    }
    let start = if bytes.len() > 1 && bytes[1] == b'/' {
        2
    } else {
        1
    };
    // collect tag name bytes (ASCII alpha only)
    let mut name_end = start;
    while name_end < bytes.len() {
        let b = bytes[name_end];
        if b.is_ascii_alphabetic() || (name_end > start && b.is_ascii_alphanumeric()) {
            name_end += 1;
        } else {
            break;
        }
    }
    if name_end == start {
        return false;
    }
    let name = &bytes[start..name_end];
    GFM_FILTERED_TAGS
        .iter()
        .any(|tag| name.eq_ignore_ascii_case(tag))
}

/// Returns `true` if the URL uses a dangerous scheme (`javascript:`, `vbscript:`, `data:`).
/// Data URIs with an image MIME type (`data:image/...`) are allowed.
#[inline]
pub(crate) fn is_dangerous_url(url: &str) -> bool {
    let bytes = url.trim().as_bytes();
    if bytes.len() < 5 {
        return false;
    }
    if bytes.len() >= 11
        && bytes[..11]
            .iter()
            .zip(b"javascript:")
            .all(|(a, b)| a.to_ascii_lowercase() == *b)
    {
        return true;
    }
    if bytes.len() >= 9
        && bytes[..9]
            .iter()
            .zip(b"vbscript:")
            .all(|(a, b)| a.to_ascii_lowercase() == *b)
    {
        return true;
    }
    if bytes.len() >= 5
        && bytes[..5]
            .iter()
            .zip(b"data:")
            .all(|(a, b)| a.to_ascii_lowercase() == *b)
    {
        // Allow data:image/
        if bytes.len() >= 11
            && bytes[5..11]
                .iter()
                .zip(b"image/")
                .all(|(a, b)| a.to_ascii_lowercase() == *b)
        {
            return false;
        }
        return true;
    }
    false
}

#[inline(always)]
pub(crate) fn trim_cr(line: &str) -> &str {
    line.strip_suffix('\r').unwrap_or(line)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn escapes_all_html_specials() {
        assert_eq!(escape_html("<>&\"'"), "&lt;&gt;&amp;&quot;'");
    }

    #[test]
    fn escapes_into_existing_buffer() {
        let mut out = String::from("x=");
        escape_html_into(&mut out, "<>");
        assert_eq!(out, "x=&lt;&gt;");
    }

    #[test]
    fn trims_windows_cr() {
        assert_eq!(trim_cr("abc\r"), "abc");
        assert_eq!(trim_cr("abc"), "abc");
    }

    #[test]
    fn plain_text_no_copy() {
        assert_eq!(escape_html("hello world"), "hello world");
    }

    #[test]
    fn mixed_content() {
        assert_eq!(escape_html("a < b & c > d"), "a &lt; b &amp; c &gt; d");
    }
}
