#![deny(clippy::undocumented_unsafe_blocks)]

//! # ironmark
//!
//! A fast, CommonMark 0.31.2 compliant Markdown-to-HTML parser with extensions.
//!
//! ## Usage
//!
//! ```
//! use ironmark::{parse, ParseOptions};
//!
//! // With defaults (all extensions enabled)
//! let html = parse("# Hello, **world**!", &ParseOptions::default());
//!
//! // Disable specific extensions
//! let opts = ParseOptions {
//!     enable_strikethrough: false,
//!     enable_tables: false,
//!     ..Default::default()
//! };
//! let html = parse("Plain CommonMark only.", &opts);
//! ```
//!
//! ## Extensions
//!
//! All extensions are enabled by default via [`ParseOptions`]:
//!
//! | Syntax | HTML | Option |
//! |---|---|---|
//! | `~~text~~` | `<del>` | `enable_strikethrough` |
//! | `==text==` | `<mark>` | `enable_highlight` |
//! | `++text++` | `<u>` | `enable_underline` |
//! | `\| table \|` | `<table>` | `enable_tables` |
//! | `- [x] task` | checkbox | `enable_task_lists` |
//! | bare URLs | `<a>` | `enable_autolink` |
//! | newlines | `<br />` | `hard_breaks` |

pub mod ast;
mod block;
mod entities;
mod html;
mod inline;
mod render;

pub use ast::{Block, ListKind, TableAlignment, TableData};
pub use block::{parse, parse_to_ast};

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

/// Options for customizing Markdown parsing behavior.
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
        }
    }
}
