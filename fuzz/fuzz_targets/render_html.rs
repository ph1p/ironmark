//! Markdown -> HTML under every option combination: no panics, no hangs,
//! and with raw HTML disabled no script-capable URLs or raw tags.
#![no_main]

mod common;

use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    let (opts, rest) = common::split_options(data);
    let Ok(md) = std::str::from_utf8(rest) else { return };
    let html = ironmark::render_html(md, &opts);
    if opts.disable_raw_html {
        common::assert_no_dangerous_urls(&html);
        common::assert_no_raw_tags(&html);
    }
});
