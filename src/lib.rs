mod ast;
mod block;
mod entities;
mod html;
mod inline;
mod render;

pub use block::parse;

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
