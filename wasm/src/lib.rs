use ironmark::{
    AnsiOptions, Block, HtmlParseOptions, ParseOptions, UnknownInlineHandling, parse_markdown,
    render_ansi_terminal, render_html,
};
use std::borrow::Cow;
use wasm_bindgen::prelude::*;

/// Markdown arrives as UTF-8 bytes: wasm-bindgen copies `&[u8]` with one memcpy,
/// while `&str` goes through a per-character JS loop that dominated large inputs.
/// Invalid sequences become U+FFFD, like `TextDecoder`; strict `from_utf8` runs
/// first because its ASCII fast path beats the lossy iterator.
fn utf8(bytes: &[u8]) -> Cow<'_, str> {
    match std::str::from_utf8(bytes) {
        Ok(s) => Cow::Borrowed(s),
        Err(_) => String::from_utf8_lossy(bytes),
    }
}

/// Maximum input size for WASM: 10 MB
const WASM_MAX_INPUT_SIZE: usize = 10 * 1024 * 1024;

fn build_options(
    hard_breaks: Option<bool>,
    enable_highlight: Option<bool>,
    enable_strikethrough: Option<bool>,
    enable_underline: Option<bool>,
    enable_tables: Option<bool>,
    enable_autolink: Option<bool>,
    enable_task_lists: Option<bool>,
    disable_raw_html: Option<bool>,
    enable_heading_ids: Option<bool>,
    enable_heading_anchors: Option<bool>,
    enable_indented_code_blocks: Option<bool>,
    no_html_blocks: Option<bool>,
    no_html_spans: Option<bool>,
    tag_filter: Option<bool>,
    collapse_whitespace: Option<bool>,
    permissive_atx_headers: Option<bool>,
    enable_wiki_links: Option<bool>,
    enable_latex_math: Option<bool>,
) -> ParseOptions {
    ParseOptions {
        hard_breaks: hard_breaks.unwrap_or(true),
        enable_highlight: enable_highlight.unwrap_or(true),
        enable_strikethrough: enable_strikethrough.unwrap_or(true),
        enable_underline: enable_underline.unwrap_or(true),
        enable_tables: enable_tables.unwrap_or(true),
        enable_autolink: enable_autolink.unwrap_or(true),
        enable_task_lists: enable_task_lists.unwrap_or(true),
        disable_raw_html: disable_raw_html.unwrap_or(false),
        max_nesting_depth: 128,
        max_input_size: WASM_MAX_INPUT_SIZE,
        enable_heading_ids: enable_heading_ids.unwrap_or(false),
        enable_heading_anchors: enable_heading_anchors.unwrap_or(false),
        enable_indented_code_blocks: enable_indented_code_blocks.unwrap_or(true),
        no_html_blocks: no_html_blocks.unwrap_or(false),
        no_html_spans: no_html_spans.unwrap_or(false),
        tag_filter: tag_filter.unwrap_or(false),
        collapse_whitespace: collapse_whitespace.unwrap_or(false),
        permissive_atx_headers: permissive_atx_headers.unwrap_or(false),
        enable_wiki_links: enable_wiki_links.unwrap_or(false),
        enable_latex_math: enable_latex_math.unwrap_or(false),
    }
}

#[wasm_bindgen]
pub fn parse(
    markdown: &[u8],
    hard_breaks: Option<bool>,
    enable_highlight: Option<bool>,
    enable_strikethrough: Option<bool>,
    enable_underline: Option<bool>,
    enable_tables: Option<bool>,
    enable_autolink: Option<bool>,
    enable_task_lists: Option<bool>,
    disable_raw_html: Option<bool>,
    enable_heading_ids: Option<bool>,
    enable_heading_anchors: Option<bool>,
    enable_indented_code_blocks: Option<bool>,
    no_html_blocks: Option<bool>,
    no_html_spans: Option<bool>,
    tag_filter: Option<bool>,
    collapse_whitespace: Option<bool>,
    permissive_atx_headers: Option<bool>,
    enable_wiki_links: Option<bool>,
    enable_latex_math: Option<bool>,
) -> String {
    render_html(
        &utf8(markdown),
        &build_options(
            hard_breaks,
            enable_highlight,
            enable_strikethrough,
            enable_underline,
            enable_tables,
            enable_autolink,
            enable_task_lists,
            disable_raw_html,
            enable_heading_ids,
            enable_heading_anchors,
            enable_indented_code_blocks,
            no_html_blocks,
            no_html_spans,
            tag_filter,
            collapse_whitespace,
            permissive_atx_headers,
            enable_wiki_links,
            enable_latex_math,
        ),
    )
}

#[wasm_bindgen(js_name = "parseDefault")]
pub fn parse_default(markdown: &[u8]) -> String {
    render_html(
        &utf8(markdown),
        &ParseOptions {
            max_input_size: WASM_MAX_INPUT_SIZE,
            ..ParseOptions::default()
        },
    )
}

#[wasm_bindgen(js_name = "parseToAst")]
pub fn parse_to_ast(
    markdown: &[u8],
    hard_breaks: Option<bool>,
    enable_highlight: Option<bool>,
    enable_strikethrough: Option<bool>,
    enable_underline: Option<bool>,
    enable_tables: Option<bool>,
    enable_autolink: Option<bool>,
    enable_task_lists: Option<bool>,
    disable_raw_html: Option<bool>,
    enable_heading_ids: Option<bool>,
    enable_heading_anchors: Option<bool>,
    enable_indented_code_blocks: Option<bool>,
    no_html_blocks: Option<bool>,
    no_html_spans: Option<bool>,
    tag_filter: Option<bool>,
    collapse_whitespace: Option<bool>,
    permissive_atx_headers: Option<bool>,
    enable_wiki_links: Option<bool>,
    enable_latex_math: Option<bool>,
) -> Result<String, JsValue> {
    let ast = parse_markdown(
        &utf8(markdown),
        &build_options(
            hard_breaks,
            enable_highlight,
            enable_strikethrough,
            enable_underline,
            enable_tables,
            enable_autolink,
            enable_task_lists,
            disable_raw_html,
            enable_heading_ids,
            enable_heading_anchors,
            enable_indented_code_blocks,
            no_html_blocks,
            no_html_spans,
            tag_filter,
            collapse_whitespace,
            permissive_atx_headers,
            enable_wiki_links,
            enable_latex_math,
        ),
    );
    serde_json::to_string(&ast)
        .map_err(|err| JsValue::from_str(&format!("AST serialization failed: {err}")))
}

/// Render Markdown as ANSI-coloured terminal output.
///
/// Produces a string containing ANSI 256-colour escape codes suitable for
/// display in a terminal emulator. Use `color: false` to get plain text
/// (all ANSI codes stripped).
///
/// @param markdown - Markdown source string.
/// @param options - Optional parse options (same as `parse()`).
/// @param width - Terminal column width for word-wrap and underlines (default: 80, 0 = use default).
/// @param color - Emit ANSI colour codes (default: true).
/// @param lineNumbers - Show line numbers in fenced code blocks (default: false).
#[wasm_bindgen(js_name = "renderAnsi")]
pub fn render_ansi(
    markdown: &[u8],
    hard_breaks: Option<bool>,
    enable_highlight: Option<bool>,
    enable_strikethrough: Option<bool>,
    enable_underline: Option<bool>,
    enable_tables: Option<bool>,
    enable_autolink: Option<bool>,
    enable_task_lists: Option<bool>,
    disable_raw_html: Option<bool>,
    enable_heading_ids: Option<bool>,
    enable_heading_anchors: Option<bool>,
    enable_indented_code_blocks: Option<bool>,
    no_html_blocks: Option<bool>,
    no_html_spans: Option<bool>,
    tag_filter: Option<bool>,
    collapse_whitespace: Option<bool>,
    permissive_atx_headers: Option<bool>,
    enable_wiki_links: Option<bool>,
    enable_latex_math: Option<bool>,
    // Plain u32, not Option<u32>: Option<u32> generates i32.trunc_sat_f64_u
    // (nontrapping-fptoint) which wasm-opt rejects without --enable-nontrapping-fptoint.
    // 0 means "use default (80)".
    width: u32,
    color: Option<bool>,
    line_numbers: Option<bool>,
    padding: u32,
) -> String {
    let parse_opts = build_options(
        hard_breaks,
        enable_highlight,
        enable_strikethrough,
        enable_underline,
        enable_tables,
        enable_autolink,
        enable_task_lists,
        disable_raw_html,
        enable_heading_ids,
        enable_heading_anchors,
        enable_indented_code_blocks,
        no_html_blocks,
        no_html_spans,
        tag_filter,
        collapse_whitespace,
        permissive_atx_headers,
        enable_wiki_links,
        enable_latex_math,
    );
    let ansi_opts = AnsiOptions {
        width: if width == 0 { 80 } else { width as usize },
        color: color.unwrap_or(true),
        line_numbers: line_numbers.unwrap_or(false),
        padding: padding as usize,
    };
    render_ansi_terminal(&utf8(markdown), &parse_opts, Some(&ansi_opts))
}

fn build_html_parse_options(preserve_unknown_as_html: Option<bool>) -> HtmlParseOptions {
    HtmlParseOptions {
        max_nesting_depth: 128,
        unknown_inline_handling: if preserve_unknown_as_html.unwrap_or(false) {
            UnknownInlineHandling::PreserveAsHtml
        } else {
            UnknownInlineHandling::StripTags
        },
        max_input_size: WASM_MAX_INPUT_SIZE,
    }
}

/// Parse an HTML string and return the AST as JSON.
///
/// This converts HTML back into the same AST structure used by the Markdown parser,
/// enabling HTML-to-Markdown conversion.
///
/// @param html - HTML source string.
/// @param preserveUnknownAsHtml - If true, unknown HTML tags are preserved as raw HTML.
///                                If false (default), unknown tags are stripped but text is kept.
/// @returns JSON string representing the AST.
#[wasm_bindgen(js_name = "parseHtmlToAst")]
pub fn parse_html_to_ast(
    html: &str,
    preserve_unknown_as_html: Option<bool>,
) -> Result<String, JsValue> {
    let ast =
        ironmark::parse_html_to_ast(html, &build_html_parse_options(preserve_unknown_as_html));
    serde_json::to_string(&ast)
        .map_err(|err| JsValue::from_str(&format!("AST serialization failed: {err}")))
}

/// Convert HTML to Markdown.
///
/// This parses HTML and renders it as Markdown syntax.
///
/// @param html - HTML source string.
/// @param preserveUnknownAsHtml - If true, unknown HTML tags are preserved as raw HTML in output.
///                                If false (default), unknown tags are stripped but text is kept.
/// @returns Markdown string.
#[wasm_bindgen(js_name = "htmlToMarkdown")]
pub fn html_to_markdown(html: &str, preserve_unknown_as_html: Option<bool>) -> String {
    ironmark::html_to_markdown(html, &build_html_parse_options(preserve_unknown_as_html))
}

/// Render an AST (as JSON) to Markdown.
///
/// This takes a JSON string representing an ironmark AST and renders it as Markdown.
///
/// @param astJson - JSON string representing the AST (as returned by `parseToAst` or `parseHtmlToAst`).
/// @returns Markdown string.
#[wasm_bindgen(js_name = "renderMarkdown")]
pub fn render_markdown(ast_json: &str) -> Result<String, JsValue> {
    let ast: Block = serde_json::from_str(ast_json)
        .map_err(|err| JsValue::from_str(&format!("Invalid AST JSON: {err}")))?;
    Ok(ironmark::render_markdown(&ast))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn limited_defaults() -> ParseOptions {
        ParseOptions {
            max_input_size: WASM_MAX_INPUT_SIZE,
            ..ParseOptions::default()
        }
    }

    fn fenced_input(bytes: usize) -> String {
        let prefix = "ordinary paragraph\n\n```\n";
        format!("{prefix}{}", "x".repeat(bytes - prefix.len()))
    }

    fn assert_limited_default(input: &str) {
        let expected = render_html(input, &limited_defaults());
        let actual = parse_default(input.as_bytes());
        // Avoid dumping a ten-megabyte HTML assertion on a regression.
        assert!(
            actual == expected,
            "default output differs from limited rendering: input={} actual={} expected={} bytes",
            input.len(),
            actual.len(),
            expected.len()
        );
    }

    #[test]
    fn wasm_default_limit_matches_configured_policy() {
        assert_eq!(WASM_MAX_INPUT_SIZE, 10 * 1024 * 1024);
        let configured = build_options(
            None, None, None, None, None, None, None, None, None, None, None, None, None, None,
            None, None, None, None,
        );
        assert_eq!(configured.max_input_size, WASM_MAX_INPUT_SIZE);
        let input = "ordinary & **bold** *em* ~~del~~ ==mark== ++u++ `code`\nnext";
        let expected = "<p>ordinary &amp; <strong>bold</strong> <em>em</em> <del>del</del> <mark>mark</mark> <u>u</u> <code>code</code><br />\nnext</p>\n";
        assert_eq!(parse_default(input.as_bytes()), expected);
        assert_eq!(render_html(input, &configured), expected);
    }

    #[test]
    fn wasm_default_limit_discards_oversized_suffix() {
        let input = format!(
            "{}SUFFIX_BEYOND_WASM_LIMIT",
            fenced_input(WASM_MAX_INPUT_SIZE)
        );
        let actual = parse_default(input.as_bytes());
        assert!(
            !actual.contains("SUFFIX_BEYOND_WASM_LIMIT"),
            "default rendering retained suffix beyond the 10 MiB input limit"
        );
        assert_limited_default(&input);
    }

    #[test]
    fn wasm_default_limit_handles_exact_and_adjacent_sizes() {
        for bytes in [
            WASM_MAX_INPUT_SIZE - 1,
            WASM_MAX_INPUT_SIZE,
            WASM_MAX_INPUT_SIZE + 1,
        ] {
            assert_limited_default(&fenced_input(bytes));
        }
    }

    #[test]
    fn wasm_default_limit_preserves_utf8_boundary() {
        let input = format!("{}éSUFFIX", fenced_input(WASM_MAX_INPUT_SIZE - 1));
        assert_limited_default(&input);
        let actual = parse_default(input.as_bytes());
        assert!(!actual.contains('é'));
        assert!(!actual.contains('\u{fffd}'));
        assert!(!actual.contains("SUFFIX"));
    }
}
