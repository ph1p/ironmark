#[cfg(test)]
pub(crate) fn escape_html(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    escape_html_into(&mut out, input);
    out
}

#[inline(always)]
fn escape_quote_only(out: &mut String, input: &str, bytes: &[u8]) {
    let len = bytes.len();
    let mut last = 0;
    while let Some(q_off) = memchr::memchr(b'"', &bytes[last..]) {
        let q = last + q_off;
        if last < q {
            out.push_str(unsafe { input.get_unchecked(last..q) });
        }
        out.push_str("&quot;");
        last = q + 1;
    }
    if last < len {
        out.push_str(unsafe { input.get_unchecked(last..len) });
    }
}

#[inline]
pub(crate) fn escape_html_into(out: &mut String, input: &str) {
    let bytes = input.as_bytes();
    let len = bytes.len();
    let mut last = 0;
    if memchr::memchr3(b'&', b'<', b'>', bytes).is_none() {
        escape_quote_only(out, input, bytes);
        return;
    }
    while last < len {
        let next = memchr::memchr3(b'&', b'<', b'>', &bytes[last..]);
        let boundary = next.map_or(len, |o| last + o);
        if let Some(q_off) = memchr::memchr(b'"', &bytes[last..boundary]) {
            let q = last + q_off;
            if last < q {
                out.push_str(unsafe { input.get_unchecked(last..q) });
            }
            out.push_str("&quot;");
            last = q + 1;
            continue;
        }
        if last < boundary {
            out.push_str(unsafe { input.get_unchecked(last..boundary) });
        }
        if let Some(offset) = next {
            let i = last + offset;
            out.push_str(match bytes[i] {
                b'&' => "&amp;",
                b'<' => "&lt;",
                _ => "&gt;",
            });
            last = i + 1;
        } else {
            return;
        }
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
