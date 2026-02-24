#[cfg(test)]
pub(crate) fn escape_html(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    escape_html_into(&mut out, input);
    out
}

/// Fast HTML escaping that scans bytes and flushes plain segments in bulk.
/// Only 4 chars need escaping per CommonMark: & < > "
#[inline]
pub(crate) fn escape_html_into(out: &mut String, input: &str) {
    let bytes = input.as_bytes();
    let len = bytes.len();

    // Use a lookup table for O(1) per-byte check
    static NEEDS_ESCAPE: [bool; 256] = {
        let mut t = [false; 256];
        t[b'&' as usize] = true;
        t[b'<' as usize] = true;
        t[b'>' as usize] = true;
        t[b'"' as usize] = true;
        t
    };

    let mut last = 0;
    let mut i = 0;

    while i < len {
        if !NEEDS_ESCAPE[bytes[i] as usize] {
            i += 1;
            continue;
        }
        let replacement = match bytes[i] {
            b'&' => "&amp;",
            b'<' => "&lt;",
            b'>' => "&gt;",
            b'"' => "&quot;",
            _ => unreachable!(),
        };
        // SAFETY: `last` and `i` are always at valid UTF-8 boundaries because
        // we only split on ASCII bytes (< 0x80), which are never continuation bytes.
        if last < i {
            out.push_str(unsafe { input.get_unchecked(last..i) });
        }
        out.push_str(replacement);
        i += 1;
        last = i;
    }

    if last < len {
        out.push_str(unsafe { input.get_unchecked(last..len) });
    }
}

/// Percent-encode a URL for use in href/src attributes.
/// Preserves already-encoded %XX sequences, and encodes characters that need it.
pub(crate) fn encode_url(out: &mut String, url: &str) {
    // Lookup table: true means the byte is safe and can pass through
    static SAFE: [bool; 256] = {
        let mut t = [false; 256];
        let mut i = b'A';
        while i <= b'Z' {
            t[i as usize] = true;
            i += 1;
        }
        let mut i = b'a';
        while i <= b'z' {
            t[i as usize] = true;
            i += 1;
        }
        let mut i = b'0';
        while i <= b'9' {
            t[i as usize] = true;
            i += 1;
        }
        t[b'-' as usize] = true;
        t[b'_' as usize] = true;
        t[b'.' as usize] = true;
        t[b'~' as usize] = true;
        t[b'!' as usize] = true;
        t[b'*' as usize] = true;
        t[b'\'' as usize] = true;
        t[b'(' as usize] = true;
        t[b')' as usize] = true;
        t[b';' as usize] = true;
        t[b'/' as usize] = true;
        t[b'?' as usize] = true;
        t[b':' as usize] = true;
        t[b'@' as usize] = true;
        t[b'&' as usize] = true;
        t[b'=' as usize] = true;
        t[b'+' as usize] = true;
        t[b'$' as usize] = true;
        t[b',' as usize] = true;
        t[b'#' as usize] = true;
        t
    };

    let bytes = url.as_bytes();
    let len = bytes.len();
    let mut last = 0;
    let mut i = 0;

    while i < len {
        let b = bytes[i];
        if SAFE[b as usize] {
            i += 1;
            continue;
        }
        // Already percent-encoded sequence
        if b == b'%'
            && i + 2 < len
            && bytes[i + 1].is_ascii_hexdigit()
            && bytes[i + 2].is_ascii_hexdigit()
        {
            i += 3;
            continue;
        }
        // Flush safe segment
        if last < i {
            out.push_str(&url[last..i]);
        }
        // Encode this byte (or multi-byte sequence)
        let ch_len = if b < 0x80 {
            1
        } else if b < 0xE0 {
            2
        } else if b < 0xF0 {
            3
        } else {
            4
        };
        for j in 0..ch_len {
            if i + j < len {
                out.push('%');
                out.push(HEX_CHARS[(bytes[i + j] >> 4) as usize] as char);
                out.push(HEX_CHARS[(bytes[i + j] & 0xF) as usize] as char);
            }
        }
        i += ch_len;
        last = i;
    }

    if last < len {
        out.push_str(&url[last..len]);
    }
}

static HEX_CHARS: &[u8; 16] = b"0123456789ABCDEF";

#[inline(always)]
pub(crate) fn trim_cr(line: &str) -> &str {
    let bytes = line.as_bytes();
    if !bytes.is_empty() && bytes[bytes.len() - 1] == b'\r' {
        unsafe { line.get_unchecked(..bytes.len() - 1) }
    } else {
        line
    }
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
