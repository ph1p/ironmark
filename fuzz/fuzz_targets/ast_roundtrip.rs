//! Markdown -> AST -> Markdown -> AST: no panics, and the second render
//! is a fixed point (rendering its own output changes nothing).
#![no_main]

mod common;

use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    let (mut opts, rest) = common::split_options(data);
    let Ok(md) = std::str::from_utf8(rest) else { return };
    // Truncation would make the comparison meaningless.
    opts.max_input_size = 0;
    let once = ironmark::render_markdown(&ironmark::parse_markdown(md, &opts));
    let twice = ironmark::render_markdown(&ironmark::parse_markdown(&once, &opts));
    if std::env::var_os("IRONMARK_FUZZ_ROUNDTRIP").is_some() {
        let thrice = ironmark::render_markdown(&ironmark::parse_markdown(&twice, &opts));
        assert_eq!(twice, thrice, "render_markdown is not idempotent\ninput: {md:?}");
    }
});
