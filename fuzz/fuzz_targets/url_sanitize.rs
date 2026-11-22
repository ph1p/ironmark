//! Wrap the input in every link-producing construct so the fuzzer spends
//! its time on destinations, then check no script-capable URL survives.
#![no_main]

mod common;

use ironmark::ParseOptions;
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    let Some((&pick, rest)) = data.split_first() else { return };
    let Ok(s) = std::str::from_utf8(rest) else { return };
    let md = match pick % 8 {
        0 => format!("[x]({s})"),
        1 => format!("![x]({s})"),
        2 => format!("<{s}>"),
        3 => format!("[x]\n\n[x]: {s}"),
        4 => format!("[[{s}]]"),
        5 => format!("[[{s}|x]]"),
        6 => format!("x {s} x"),
        _ => s.to_owned(),
    };
    let opts = ParseOptions {
        disable_raw_html: true,
        enable_wiki_links: true,
        enable_autolink: true,
        enable_heading_anchors: true,
        ..Default::default()
    };
    let html = ironmark::render_html(&md, &opts);
    common::assert_no_dangerous_urls(&html);
    common::assert_no_raw_tags(&html);
});
