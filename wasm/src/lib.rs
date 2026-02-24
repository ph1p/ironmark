use ironmark::{parse as ironmark_parse, ParseOptions};
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
pub fn parse(
    markdown: &str,
    hard_breaks: Option<bool>,
    enable_highlight: Option<bool>,
    enable_strikethrough: Option<bool>,
    enable_underline: Option<bool>,
    enable_tables: Option<bool>,
    enable_autolink: Option<bool>,
    enable_task_lists: Option<bool>,
) -> String {
    let options = ParseOptions {
        hard_breaks: hard_breaks.unwrap_or(true),
        enable_highlight: enable_highlight.unwrap_or(true),
        enable_strikethrough: enable_strikethrough.unwrap_or(true),
        enable_underline: enable_underline.unwrap_or(true),
        enable_tables: enable_tables.unwrap_or(true),
        enable_autolink: enable_autolink.unwrap_or(true),
        enable_task_lists: enable_task_lists.unwrap_or(true),
    };
    ironmark_parse(markdown, &options)
}
