//! HTML -> Markdown (and that Markdown back to HTML): no panics, no hangs.
#![no_main]

use ironmark::{HtmlParseOptions, ParseOptions, UnknownInlineHandling};
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    let Some((&flags, rest)) = data.split_first() else { return };
    let Ok(html) = std::str::from_utf8(rest) else { return };
    let opts = HtmlParseOptions {
        max_nesting_depth: match flags & 0x0f {
            0 => 128,
            n => n as usize,
        },
        unknown_inline_handling: if flags & 0x10 != 0 {
            UnknownInlineHandling::PreserveAsHtml
        } else {
            UnknownInlineHandling::StripTags
        },
        max_input_size: if flags & 0x20 != 0 { 64 } else { 0 },
    };
    let md = ironmark::html_to_markdown(html, &opts);
    let _ = ironmark::render_html(&md, &ParseOptions::default());
});
