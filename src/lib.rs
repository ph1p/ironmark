#![deny(clippy::undocumented_unsafe_blocks)]

//! # ironmark
//!
//! A fast, CommonMark 0.31.2 compliant Markdown-to-HTML parser with extensions.
//!
//! ## Usage
//!
//! ```
//! use ironmark::{render_html, ParseOptions};
//!
//! // With defaults (all extensions enabled)
//! let html = render_html("# Hello, **world**!", &ParseOptions::default());
//!
//! // Disable specific extensions
//! let opts = ParseOptions {
//!     enable_strikethrough: false,
//!     enable_tables: false,
//!     ..Default::default()
//! };
//! let html = render_html("Plain CommonMark only.", &opts);
//! ```
//!
//! ## Security
//!
//! When rendering **untrusted** input, enable these options:
//!
//! ```
//! use ironmark::{render_html, ParseOptions};
//!
//! let opts = ParseOptions {
//!     disable_raw_html: true,  // escape HTML blocks & inline HTML
//!     max_input_size: 1_000_000, // limit input to 1 MB
//!     ..Default::default()
//! };
//! let html = render_html("<script>alert(1)</script>", &opts);
//! assert!(!html.contains("<script>"));
//! ```
//!
//! Additionally, `javascript:`, `vbscript:`, and `data:` URIs (except `data:image/…`)
//! are **always** stripped from link and image destinations regardless of options.
//!
//! ## Extensions
//!
//! Extensions enabled by default via [`ParseOptions`]:
//!
//! | Syntax | HTML output | Option |
//! |---|---|---|
//! | `~~text~~` | `<del>` | [`enable_strikethrough`](ParseOptions::enable_strikethrough) |
//! | `==text==` | `<mark>` | [`enable_highlight`](ParseOptions::enable_highlight) |
//! | `++text++` | `<u>` | [`enable_underline`](ParseOptions::enable_underline) |
//! | `\| table \|` | `<table>` | [`enable_tables`](ParseOptions::enable_tables) |
//! | `- [x] task` | `<input type="checkbox">` | [`enable_task_lists`](ParseOptions::enable_task_lists) |
//! | bare URLs / emails | `<a>` | [`enable_autolink`](ParseOptions::enable_autolink) |
//! | newlines | `<br />` | [`hard_breaks`](ParseOptions::hard_breaks) |
//!
//! Extensions disabled by default (opt-in):
//!
//! | Syntax | HTML output | Option |
//! |---|---|---|
//! | `[[wiki]]` | `<a href="wiki">` | [`enable_wiki_links`](ParseOptions::enable_wiki_links) |
//! | `$math$` | `<span class="math-inline">` | [`enable_latex_math`](ParseOptions::enable_latex_math) |
//! | `$$math$$` | `<span class="math-display">` | [`enable_latex_math`](ParseOptions::enable_latex_math) |
//! | `# heading` with `id=` | `<h1 id="heading">` | [`enable_heading_ids`](ParseOptions::enable_heading_ids) |
//! | `# heading` with anchor | `<h1>… <a class="anchor">` | [`enable_heading_anchors`](ParseOptions::enable_heading_anchors) |
//! | `#Heading` (no space) | `<h1>` | [`permissive_atx_headers`](ParseOptions::permissive_atx_headers) |

// Core modules (always compiled)
pub mod ast;
mod block;
#[cfg_attr(not(feature = "html"), allow(dead_code))]
mod entities;
#[cfg_attr(not(feature = "html"), allow(dead_code))]
mod html;
#[cfg_attr(not(feature = "html"), allow(dead_code))]
mod inline;
mod renderers;

// Feature-gated modules
#[cfg(feature = "ansi")]
mod ansi;
#[cfg(feature = "html-parser")]
mod html_parser;
#[cfg(feature = "html")]
mod render;

// FFI module (always available for C bindings)
pub mod ffi;

// Core exports (always available)
pub use ast::{Block, ListKind, TableAlignment, TableData};
pub use block::parse_markdown;
pub use renderers::markdown::render_markdown;

// Feature-gated exports
#[cfg(feature = "ansi")]
pub use ansi::{AnsiOptions, render_ansi_terminal};
#[cfg(feature = "html")]
#[doc(hidden)]
pub use block::benchmark_parse_table_row as __benchmark_parse_table_row;
#[cfg(feature = "html")]
#[doc(hidden)]
pub use block::benchmark_render_html_parse_phase as __benchmark_render_html_parse_phase;
#[cfg(feature = "html")]
pub use block::render_html;
#[cfg(feature = "html-parser")]
pub use html_parser::{
    HtmlParseOptions, UnknownInlineHandling, html_to_markdown, parse_html_to_ast,
};
#[cfg(feature = "html")]
#[doc(hidden)]
pub use inline::benchmark_parse_inline as __benchmark_parse_inline;
#[cfg(feature = "html")]
#[doc(hidden)]
pub use render::benchmark_heading_slug as __benchmark_heading_slug;

#[inline(always)]
pub(crate) fn is_ascii_punctuation(b: u8) -> bool {
    matches!(b, b'!'..=b'/' | b':'..=b'@' | b'['..=b'`' | b'{'..=b'~')
}

#[inline(always)]
pub(crate) fn utf8_char_len(first: u8) -> usize {
    if first < 0x80 {
        1
    } else if first < 0xE0 {
        2
    } else if first < 0xF0 {
        3
    } else {
        4
    }
}

/// Options for customizing Markdown parsing and rendering behavior.
///
/// Construct with [`Default::default()`] and override only the fields you need:
///
/// ```
/// use ironmark::{render_html, ParseOptions};
///
/// let html = render_html("~~strike~~ ==highlight==", &ParseOptions {
///     enable_strikethrough: true,
///     enable_highlight: true,
///     ..Default::default()
/// });
/// ```
pub struct ParseOptions {
    /// When `true`, every newline inside a paragraph becomes a hard line break (`<br />`),
    /// similar to GitHub Flavored Markdown. Default: `true`.
    pub hard_breaks: bool,
    /// Enable `==highlight==` syntax → `<mark>`. Default: `true`.
    pub enable_highlight: bool,
    /// Enable `~~strikethrough~~` syntax → `<del>`. Default: `true`.
    pub enable_strikethrough: bool,
    /// Enable `++underline++` syntax → `<u>`. Default: `true`.
    pub enable_underline: bool,
    /// Enable pipe table syntax. Default: `true`.
    pub enable_tables: bool,
    /// Automatically detect bare URLs (`https://...`) and emails (`user@example.com`)
    /// and wrap them in `<a>` tags. Default: `true`.
    pub enable_autolink: bool,
    /// Enable GitHub-style task lists (`- [ ] unchecked`, `- [x] checked`)
    /// in list items. Default: `true`.
    pub enable_task_lists: bool,
    /// When `true`, raw HTML blocks and inline HTML are escaped instead of passed
    /// through verbatim. This prevents XSS when rendering untrusted markdown.
    /// Default: `false`.
    pub disable_raw_html: bool,
    /// Maximum nesting depth for block-level containers (blockquotes, list items).
    /// Once exceeded, further nesting is treated as paragraph text.
    /// Default: `128`.
    pub max_nesting_depth: usize,
    /// Maximum input size in bytes. Inputs exceeding this limit are truncated.
    /// `0` means no limit. Default: `0`.
    pub max_input_size: usize,

    // ── Extension options (all default to `false`) ──────────────────────────
    /// Auto-generate `id=` attributes on headings from their text content (slugified).
    /// Default: `false`.
    pub enable_heading_ids: bool,
    /// Render an `<a class="anchor">` link inside each heading (implies slug generation).
    /// Default: `false`.
    pub enable_heading_anchors: bool,
    /// When `true` (the default), a block indented by 4 or more spaces is a code block.
    /// Set to `false` to disable indented code blocks and treat them as paragraphs instead.
    /// Default: `true`.
    pub enable_indented_code_blocks: bool,
    /// When `true`, HTML block constructs are disabled: their source is escaped as text.
    /// More granular than `disable_raw_html` (which also affects inline HTML).
    /// Default: `false`.
    pub no_html_blocks: bool,
    /// When `true`, inline HTML spans are disabled: their source is escaped as text.
    /// More granular than `disable_raw_html` (which also affects HTML blocks).
    /// Default: `false`.
    pub no_html_spans: bool,
    /// Enable the GFM tag filter: a fixed set of dangerous HTML tags
    /// (`title`, `textarea`, `style`, `xmp`, `iframe`, `noembed`, `noframes`,
    /// `script`, `plaintext`) is escaped even when HTML is otherwise allowed.
    /// Default: `false`.
    pub tag_filter: bool,
    /// Collapse runs of spaces/tabs in text nodes to a single space.
    /// Does not affect code spans or hard/soft line breaks.
    /// Default: `false`.
    pub collapse_whitespace: bool,
    /// Allow ATX headings without a space after `#` (e.g. `#Heading`).
    /// Default: `false`.
    pub permissive_atx_headers: bool,
    /// Enable `[[wiki link]]` syntax → `<a href="wiki-link">wiki link</a>`.
    /// Default: `false`.
    pub enable_wiki_links: bool,
    /// Enable `$inline$` and `$$display$$` math syntax.
    /// Content is HTML-escaped and wrapped in `<span class="math-inline">` /
    /// `<span class="math-display">` for client-side rendering.
    /// Default: `false`.
    pub enable_latex_math: bool,
}

impl Default for ParseOptions {
    fn default() -> Self {
        Self {
            hard_breaks: true,
            enable_highlight: true,
            enable_strikethrough: true,
            enable_underline: true,
            enable_tables: true,
            enable_autolink: true,
            enable_task_lists: true,
            disable_raw_html: false,
            max_nesting_depth: 128,
            max_input_size: 0,
            enable_heading_ids: false,
            enable_heading_anchors: false,
            enable_indented_code_blocks: true,
            no_html_blocks: false,
            no_html_spans: false,
            tag_filter: false,
            collapse_whitespace: false,
            permissive_atx_headers: false,
            enable_wiki_links: false,
            enable_latex_math: false,
        }
    }
}
