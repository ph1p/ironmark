//! Shared helpers for the fuzz targets.
#![allow(dead_code)]

use ironmark::ParseOptions;

/// Take the first 4 bytes as option bits; the rest is the input.
/// Covers every flag combination plus small nesting / size limits.
pub fn split_options(data: &[u8]) -> (ParseOptions, &[u8]) {
    let (head, rest) = data.split_at(data.len().min(4));
    let mut b = [0u8; 4];
    b[..head.len()].copy_from_slice(head);
    let bits = u32::from_le_bytes(b);
    let on = |n: u32| bits & (1 << n) != 0;
    let opts = ParseOptions {
        hard_breaks: on(0),
        enable_highlight: on(1),
        enable_strikethrough: on(2),
        enable_underline: on(3),
        enable_tables: on(4),
        enable_autolink: on(5),
        enable_task_lists: on(6),
        disable_raw_html: on(7),
        enable_heading_ids: on(8),
        enable_heading_anchors: on(9),
        enable_indented_code_blocks: on(10),
        no_html_blocks: on(11),
        no_html_spans: on(12),
        tag_filter: on(13),
        collapse_whitespace: on(14),
        permissive_atx_headers: on(15),
        enable_wiki_links: on(16),
        enable_latex_math: on(17),
        enable_mdx: on(18),
        enable_frontmatter: on(19),
        // Bits 20-23: tiny nesting limit (0 keeps the default 128).
        max_nesting_depth: match (bits >> 20) & 0xf {
            0 => 128,
            n => n as usize,
        },
        // Bits 24-31: input truncation (0 = unlimited); may cut mid-UTF-8.
        max_input_size: ((bits >> 24) & 0xff) as usize,
    };
    (opts, rest)
}

/// Decode the few entities the renderer emits inside attribute values.
fn decode_attr(v: &str) -> String {
    v.replace("&quot;", "\"")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&#x27;", "'")
        .replace("&#39;", "'")
        .replace("&amp;", "&")
}

/// What a browser would treat as the URL scheme: WHATWG URL parsing drops
/// ASCII tab/newline anywhere and leading C0 controls/spaces.
fn browser_view(url: &str) -> String {
    url.chars()
        .filter(|c| !matches!(c, '\t' | '\n' | '\r'))
        .collect::<String>()
        .trim_start_matches(|c: char| c <= ' ')
        .to_ascii_lowercase()
}

/// Panic if any `href`/`src` in `html` carries a script-capable scheme.
/// Only meaningful when raw HTML is disabled (otherwise the input can
/// write any attribute it likes).
pub fn assert_no_dangerous_urls(html: &str) {
    for attr in [" href=\"", " src=\""] {
        let mut rest = html;
        while let Some(i) = rest.find(attr) {
            rest = &rest[i + attr.len()..];
            let end = rest.find('"').unwrap_or(rest.len());
            let url = browser_view(&decode_attr(&rest[..end]));
            let bad = url.starts_with("javascript:")
                || url.starts_with("vbscript:")
                || (url.starts_with("data:") && !url.starts_with("data:image/"));
            assert!(!bad, "dangerous URL survived: {:?}\nhtml: {html}", &rest[..end]);
        }
    }
}

/// Panic if raw markup leaked through while raw HTML is disabled.
pub fn assert_no_raw_tags(html: &str) {
    let lower = html.to_ascii_lowercase();
    for tag in ["<script", "<iframe", "<style", "<object", "<embed"] {
        assert!(!lower.contains(tag), "raw `{tag}` leaked\nhtml: {html}");
    }
}
