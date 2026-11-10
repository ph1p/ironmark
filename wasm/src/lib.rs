#[cfg(feature = "ansi")]
use ironmark::{AnsiOptions, render_ansi_terminal};
#[cfg(feature = "ast")]
use ironmark::{Block, parse_markdown};
#[cfg(feature = "html-parser")]
use ironmark::{HtmlParseOptions, UnknownInlineHandling};
use ironmark::{ParseOptions, render_html};
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

/// Parse options cross the boundary as one `u32`: bit `i` is the `i`-th key of
/// `DEFAULT_OPTIONS` in `wasm/shared.js`, already resolved against the defaults.
/// One integer replaces an `Option<bool>` argument per option on every call.
fn build_options(flags: u32) -> ParseOptions {
    let on = |bit: u32| flags & (1 << bit) != 0;
    ParseOptions {
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
        max_nesting_depth: 128,
        max_input_size: WASM_MAX_INPUT_SIZE,
    }
}

#[wasm_bindgen]
pub fn parse(markdown: &[u8], flags: u32) -> String {
    render_html(&utf8(markdown), &build_options(flags))
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

/// Render Markdown as plain text (see `ironmark::render_text`).
#[wasm_bindgen(js_name = "renderText")]
pub fn render_text(markdown: &[u8], flags: u32) -> String {
    let options = build_options(flags);
    ironmark::render_text(&utf8(markdown), &options)
}

/// A document parsed once (see `ironmark::ParsedDocument`): the chainable JS
/// `Document` keeps one of these while it renders several outputs, so text,
/// AST, Markdown and ANSI share one parse instead of parsing again per output.
#[wasm_bindgen]
pub struct Parsed(ironmark::ParsedDocument);

#[wasm_bindgen(js_name = "parseDocument")]
pub fn parse_document(markdown: &[u8], flags: u32) -> Parsed {
    Parsed(ironmark::ParsedDocument::parse(
        &utf8(markdown),
        &build_options(flags),
    ))
}

#[wasm_bindgen]
impl Parsed {
    /// Text, headings, links and front matter (see `ironmark::analyze`), as one
    /// flat list that crosses the boundary without a JSON dependency:
    /// `[text, frontmatter | null, headingCount, (level, id, text)…, (image, url, title | null, text)…]`.
    pub fn analyze(&self) -> Vec<JsValue> {
        let doc = self.0.analyze();
        let mut out = Vec::with_capacity(3 + doc.headings.len() * 3 + doc.links.len() * 4);
        out.push(JsValue::from(doc.text));
        out.push(doc.frontmatter.map_or(JsValue::NULL, JsValue::from));
        out.push(JsValue::from(doc.headings.len() as u32));
        for h in doc.headings {
            out.extend([
                JsValue::from(h.level),
                JsValue::from(h.id),
                JsValue::from(h.text),
            ]);
        }
        for link in doc.links {
            out.extend([
                JsValue::from_bool(link.image),
                JsValue::from(link.url),
                link.title.map_or(JsValue::NULL, JsValue::from),
                JsValue::from(link.text),
            ]);
        }
        out
    }

    /// The AST as JSON, like `parseToAst`.
    #[cfg(feature = "ast")]
    pub fn ast(&self) -> Result<String, JsValue> {
        ast_json(self.0.block())
    }

    /// Normalized Markdown, without the AST JSON round trip
    /// `renderMarkdown(parseToAst(…))` would take.
    #[cfg(feature = "ast")]
    pub fn markdown(&self) -> String {
        self.0.markdown()
    }

    /// ANSI terminal output; the arguments are those of `renderAnsi`.
    #[cfg(feature = "ansi")]
    pub fn ansi(
        &self,
        width: u32,
        color: Option<bool>,
        line_numbers: Option<bool>,
        padding: u32,
    ) -> String {
        self.0
            .ansi(Some(&ansi_options(width, color, line_numbers, padding)))
    }
}

#[cfg(feature = "ast")]
fn ast_json(block: &Block) -> Result<String, JsValue> {
    serde_json::to_string(block)
        .map_err(|err| JsValue::from_str(&format!("AST serialization failed: {err}")))
}

#[cfg(feature = "ast")]
#[wasm_bindgen(js_name = "parseToAst")]
pub fn parse_to_ast(markdown: &[u8], flags: u32) -> Result<String, JsValue> {
    ast_json(&parse_markdown(&utf8(markdown), &build_options(flags)))
}

#[cfg(feature = "ansi")]
fn ansi_options(
    width: u32,
    color: Option<bool>,
    line_numbers: Option<bool>,
    padding: u32,
) -> AnsiOptions {
    AnsiOptions {
        width: if width == 0 { 80 } else { width as usize },
        color: color.unwrap_or(true),
        line_numbers: line_numbers.unwrap_or(false),
        padding: padding as usize,
    }
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
#[cfg(feature = "ansi")]
#[wasm_bindgen(js_name = "renderAnsi")]
pub fn render_ansi(
    markdown: &[u8],
    flags: u32,
    // Plain u32, not Option<u32>: Option<u32> generates i32.trunc_sat_f64_u
    // (nontrapping-fptoint) which wasm-opt rejects without --enable-nontrapping-fptoint.
    // 0 means "use default (80)".
    width: u32,
    color: Option<bool>,
    line_numbers: Option<bool>,
    padding: u32,
) -> String {
    let ansi_opts = ansi_options(width, color, line_numbers, padding);
    render_ansi_terminal(&utf8(markdown), &build_options(flags), Some(&ansi_opts))
}

/// Result of `renderJsx`: the ESM import/export blocks and the JSX body.
#[cfg(feature = "mdx")]
#[wasm_bindgen]
pub struct JsxOutput {
    inner: ironmark::MdxOutput,
}

#[cfg(feature = "mdx")]
#[wasm_bindgen]
impl JsxOutput {
    /// Top-level `import`/`export` blocks, in document order.
    #[wasm_bindgen(getter)]
    pub fn esm(&self) -> String {
        self.inner.esm.clone()
    }

    /// The document as one JSX fragment expression (`<>…</>`).
    #[wasm_bindgen(getter)]
    pub fn jsx(&self) -> String {
        self.inner.jsx.clone()
    }

    /// A complete ES module: ESM followed by `export default function MDXContent`.
    #[wasm_bindgen(getter)]
    pub fn module(&self) -> String {
        self.inner.to_module()
    }
}

/// Compile MDX to JSX. `enable_mdx` is ignored: MDX parsing is on, or off when
/// `markdown` is true (plain Markdown, see `ironmark::render_markdown_jsx`).
/// `components` / `provider_import_source` select `@mdx-js/mdx`-style
/// component mapping (see `ironmark::JsxOptions`).
///
/// Throws an `Error` for unbalanced or unterminated JSX.
#[cfg(feature = "mdx")]
#[wasm_bindgen(js_name = "renderJsx")]
pub fn render_jsx(
    markdown: &[u8],
    flags: u32,
    components: Option<bool>,
    provider_import_source: Option<String>,
    markdown_only: Option<bool>,
) -> Result<JsxOutput, JsError> {
    let options = build_options(flags);
    let jsx = ironmark::JsxOptions {
        components: components.unwrap_or(false),
        provider_import_source,
    };
    let compile = if markdown_only.unwrap_or(false) {
        ironmark::render_markdown_jsx
    } else {
        ironmark::render_jsx_with
    };
    compile(&utf8(markdown), &options, &jsx)
        .map(|inner| JsxOutput { inner })
        .map_err(|err| JsError::new(err.message()))
}

#[cfg(feature = "html-parser")]
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
#[cfg(feature = "html-parser")]
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
#[cfg(feature = "html-parser")]
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
#[cfg(feature = "ast")]
#[wasm_bindgen(js_name = "renderMarkdown")]
pub fn render_markdown(ast_json: &str) -> Result<String, JsValue> {
    let ast: Block = serde_json::from_str(ast_json)
        .map_err(|err| JsValue::from_str(&format!("Invalid AST JSON: {err}")))?;
    Ok(ironmark::render_markdown(&ast))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// `DEFAULT_OPTIONS` packed the way `wasm/shared.js` packs it.
    const DEFAULT_FLAGS: u32 = 0b111_1111 | 1 << 10;

    #[test]
    fn default_flags_decode_to_default_options() {
        let decoded = build_options(DEFAULT_FLAGS);
        assert_eq!(decoded, limited_defaults());
    }

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
        let configured = build_options(DEFAULT_FLAGS);
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
