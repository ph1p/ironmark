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

    if memchr::memchr3(b'&', b'<', b'>', bytes).is_none() && memchr::memchr(b'"', bytes).is_none() {
        out.push_str(input);
        return;
    }

    let mut last = 0;
    for i in 0..len {
        let idx = HTML_ESCAPE[bytes[i] as usize];
        if idx != 0 {
            if last < i {
                // SAFETY: `last` and `i` are loop-derived byte offsets within `input`
                // and only advanced forward, so the subslice is in-bounds.
                out.push_str(unsafe { input.get_unchecked(last..i) });
            }
            out.push_str(HTML_ESCAPE_STRS[idx as usize]);
            last = i + 1;
        }
    }
    if last < len {
        // SAFETY: `last <= len` and both are computed from `input.len()`.
        out.push_str(unsafe { input.get_unchecked(last..len) });
    }
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

pub(crate) fn encode_url_escaped_into(out: &mut String, url: &str) {
    let bytes = url.as_bytes();
    let len = bytes.len();
    let mut last = 0;
    let mut i = 0;

    while i < len {
        let b = bytes[i];
        if URL_HTML_SAFE[b as usize] {
            i += 1;
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
                let enc: [u8; 3] = [
                    b'%',
                    HEX_CHARS[(b >> 4) as usize],
                    HEX_CHARS[(b & 0xF) as usize],
                ];

                // SAFETY: `enc` is always ASCII (`%` + hex digits), therefore valid UTF-8.
                out.push_str(unsafe { std::str::from_utf8_unchecked(&enc) });
            }
        }
        i += ch_len;
        last = i;
    }

    if last < len {
        out.push_str(&url[last..len]);
    }
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
