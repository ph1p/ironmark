//! A parsed document that several outputs share.

use crate::ParseOptions;
use crate::ast::Block;
use crate::block::{BlockParser, split_input};
use crate::inline::LinkRefMap;

/// Markdown parsed once, so several outputs can be rendered without parsing
/// it again.
///
/// HTML is not one of them: [`render_html`](crate::render_html) uses its own,
/// faster parse that defers leaf text instead of building this tree.
///
/// # Examples
///
/// ```
/// use ironmark::{ParseOptions, ParsedDocument};
///
/// let doc = ParsedDocument::parse("# Hi *there*\n", &ParseOptions::default());
/// assert_eq!(doc.markdown(), "# Hi *there*\n");
/// assert_eq!(doc.analyze().text, "Hi there");
/// ```
#[derive(Clone, Debug)]
pub struct ParsedDocument {
    pub(crate) block: Block,
    #[cfg_attr(not(any(feature = "html", feature = "ansi")), allow(dead_code))]
    pub(crate) refs: LinkRefMap,
    #[cfg_attr(not(any(feature = "html", feature = "ansi")), allow(dead_code))]
    pub(crate) options: ParseOptions,
    #[cfg_attr(not(feature = "html"), allow(dead_code))]
    pub(crate) frontmatter: Option<String>,
    /// Length of the parsed body, to presize output buffers.
    #[cfg_attr(not(any(feature = "html", feature = "ansi")), allow(dead_code))]
    pub(crate) body_len: usize,
}

impl ParsedDocument {
    /// Parse `markdown` into the same tree [`parse_markdown`](crate::parse_markdown) returns.
    pub fn parse(markdown: &str, options: &ParseOptions) -> Self {
        let (frontmatter, body) = split_input(markdown, options, usize::MAX);
        let mut parser = BlockParser::new(body, options);
        let block = parser.parse();
        Self {
            block,
            refs: parser.ref_defs,
            options: options.clone(),
            frontmatter: frontmatter.map(str::to_owned),
            body_len: body.len(),
        }
    }

    /// The root [`Block::Document`].
    pub fn block(&self) -> &Block {
        &self.block
    }

    /// The root [`Block::Document`], without copying it.
    pub fn into_block(self) -> Block {
        self.block
    }

    /// Normalized Markdown (see [`render_markdown`](crate::render_markdown)).
    pub fn markdown(&self) -> String {
        crate::render_markdown(&self.block)
    }

    /// Text, headings, links and front matter (see [`analyze`](crate::analyze)).
    #[cfg(feature = "html")]
    pub fn analyze(&self) -> crate::Analysis {
        crate::renderers::text::analyze_parsed(self, self.frontmatter.clone())
    }

    /// ANSI terminal output (see [`render_ansi_terminal`](crate::render_ansi_terminal)).
    #[cfg(feature = "ansi")]
    pub fn ansi(&self, aopts: Option<&crate::AnsiOptions>) -> String {
        crate::ansi::render_parsed(self, aopts)
    }
}
