/// Generate a URL-safe slug from heading raw markdown text.
/// Strips markdown syntax, lowercases, replaces spaces/hyphens/dots with `-`.
/// Uses inline buffer for short slugs to avoid heap allocation.
pub(crate) fn heading_slug_into(slug: &mut String, raw: &str) {
    let bytes = raw.as_bytes();
    let len = bytes.len();
    slug.clear();

    // Fast path: check if heading is already slug-safe (common for simple headings).
    // A slug-safe heading has only ASCII alphanumerics and dashes (no leading/trailing dash).
    if len <= 64 {
        let mut is_safe = true;
        let mut has_content = false;
        let mut all_lower = true;
        for &b in bytes {
            if b.is_ascii_lowercase() {
                has_content = true;
            } else if b.is_ascii_uppercase() {
                has_content = true;
                all_lower = false;
            } else if b == b'-' {
                // A dash before any content (i.e. leading) is not slug-safe.
                if !has_content {
                    is_safe = false;
                    break;
                }
            } else {
                is_safe = false;
                break;
            }
        }
        if is_safe && has_content && bytes[len - 1] != b'-' {
            slug.push_str(raw);
            if !all_lower {
                slug.make_ascii_lowercase();
            }
            return;
        }
    }

    // Slow path: process character by character
    slug.reserve(len.saturating_sub(slug.capacity()));
    let mut i = 0;
    let mut prev_dash = true; // start true to avoid leading dash

    while i < len {
        let b = bytes[i];
        match b {
            b'*' | b'_' | b'~' | b'=' | b'+' | b'`' => {
                i += 1;
            }
            b'<' => {
                if let Some(close) = memchr::memchr(b'>', &bytes[i..]) {
                    i += close + 1;
                } else {
                    i += 1;
                }
            }
            b'\\' => {
                i += 1;
            }
            b'[' | b']' | b'!' | b'(' | b')' => {
                i += 1;
            }
            // Spaces, hyphens, dots → single dash separator
            b' ' | b'\t' | b'-' | b'.' => {
                if !prev_dash && !slug.is_empty() {
                    slug.push('-');
                    prev_dash = true;
                }
                i += 1;
            }
            // ASCII alphanumeric → lowercase
            b if b.is_ascii_alphanumeric() => {
                slug.push(b.to_ascii_lowercase() as char);
                prev_dash = false;
                i += 1;
            }
            // Multi-byte UTF-8
            b if b >= 0x80 => {
                let char_len = crate::utf8_char_len(b);
                let c = raw[i..].chars().next().unwrap_or(' ');
                if c.is_alphanumeric() {
                    for lc in c.to_lowercase() {
                        slug.push(lc);
                    }
                    prev_dash = false;
                } else {
                    // Non-alphanumeric Unicode → dash separator
                    if !prev_dash && !slug.is_empty() {
                        slug.push('-');
                        prev_dash = true;
                    }
                }
                i += char_len;
            }
            // Other ASCII punctuation → skip
            _ => {
                i += 1;
            }
        }
    }

    // Trim trailing dash
    while slug.ends_with('-') {
        slug.pop();
    }
}

pub fn benchmark_heading_slug(raw: &str) -> String {
    let mut slug = String::with_capacity(raw.len());
    heading_slug_into(&mut slug, raw);
    slug
}
