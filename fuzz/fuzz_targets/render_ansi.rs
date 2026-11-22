//! Markdown -> ANSI under every option combination and small widths.
#![no_main]

mod common;

use ironmark::AnsiOptions;
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    let Some((&a, rest)) = data.split_first() else { return };
    let (opts, rest) = common::split_options(rest);
    let Ok(md) = std::str::from_utf8(rest) else { return };
    let aopts = AnsiOptions {
        width: [0, 1, 2, 5, 20, 80][(a & 0x7) as usize % 6],
        color: a & 0x08 != 0,
        line_numbers: a & 0x10 != 0,
        padding: ((a >> 5) & 0x7) as usize,
    };
    let _ = ironmark::render_ansi_terminal(md, &opts, Some(&aopts));
});
