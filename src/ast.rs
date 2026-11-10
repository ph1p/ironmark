use compact_str::CompactString;

/// A block-level node in the parsed Markdown document.
///
/// Returned by [`parse_markdown`](crate::parse_markdown). The root is always
/// [`Block::Document`]; all other variants appear as children.
///
/// `raw` fields on leaf nodes (`Paragraph`, `Heading`) contain the raw inline
/// Markdown source — they have **not** been parsed into spans yet. Pass them
/// to the inline renderer or inspect them for custom processing.
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum Block {
    /// Root node — always the outermost block returned by [`parse_markdown`](crate::parse_markdown).
    Document { children: Vec<Block> },
    /// A `>` blockquote. May contain any block-level children.
    BlockQuote { children: Vec<Block> },
    /// An ordered or unordered list. Children are [`Block::ListItem`] nodes.
    List {
        /// Whether the list uses bullets (`-`, `*`, `+`) or numbers.
        kind: ListKind,
        /// Starting number for ordered lists; `1` for unordered lists.
        start: u32,
        /// `true` when all items are separated by blank lines (loose list).
        /// Loose lists wrap item content in `<p>` tags.
        tight: bool,
        children: Vec<Block>,
    },
    /// A single item inside a [`Block::List`].
    ListItem {
        children: Vec<Block>,
        /// `Some(false)` = `- [ ]` unchecked, `Some(true)` = `- [x]` checked,
        /// `None` = not a task-list item.
        checked: Option<bool>,
    },
    /// A paragraph. `raw` is the unparsed inline Markdown content.
    Paragraph { raw: String },
    /// An ATX (`# …`) or setext (`===` / `---`) heading.
    Heading {
        /// Heading level 1–6.
        level: u8,
        /// Unparsed inline Markdown content of the heading.
        raw: String,
    },
    /// A fenced (`` ``` ``) or indented code block.
    CodeBlock {
        /// Language hint from the opening fence (e.g. `"rust"`). Empty if none.
        info: CompactString,
        /// Literal code content, including the trailing newline.
        literal: String,
    },
    /// A raw HTML block (type 1–7 per the CommonMark spec).
    /// `literal` contains the verbatim HTML source.
    HtmlBlock { literal: String },
    /// An MDX flow construct: one or more JSX tags and/or `{expressions}` that
    /// make up whole lines. Only produced when
    /// [`enable_mdx`](crate::ParseOptions::enable_mdx) is set. `literal` is the
    /// verbatim source.
    MdxFlow { literal: String },
    /// A top-level MDX `import`/`export` block. Only produced when
    /// [`enable_mdx`](crate::ParseOptions::enable_mdx) is set. `literal` is the
    /// verbatim JavaScript source.
    MdxEsm { literal: String },
    /// A thematic break (`---`, `***`, or `___`). Renders as `<hr />`.
    ThematicBreak,
    /// A pipe table. See [`TableData`] for the cell layout.
    Table(Box<TableData>),
}

/// Data for a [`Block::Table`] node.
///
/// Cells are stored in a flat row-major `Vec`. To access row `r`, column `c`
/// (0-indexed), use `rows[r * num_cols + c]`. Header cells are stored
/// separately in `header`.
///
/// ```
/// # use ironmark::{parse_markdown, Block, ParseOptions};
/// let ast = parse_markdown("| A | B |\n|---|---|\n| 1 | 2 |", &ParseOptions::default());
/// if let Block::Document { children } = ast {
///     if let Block::Table(t) = &children[0] {
///         assert_eq!(t.header[0], "A");
///         assert_eq!(t.rows[0 * t.num_cols + 1], "2"); // row 0, col 1
///     }
/// }
/// ```
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct TableData {
    /// Per-column alignment, length == `num_cols`.
    pub alignments: Vec<TableAlignment>,
    /// Number of columns.
    pub num_cols: usize,
    /// Header row cells, length == `num_cols`. Contains raw inline Markdown.
    pub header: Vec<CompactString>,
    /// Body cells in row-major order, length == `num_rows * num_cols`.
    /// Contains raw inline Markdown.
    pub rows: Vec<CompactString>,
}

/// Marker type for a [`Block::List`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum ListKind {
    /// Unordered list. The inner byte is the marker character: `b'-'`, `b'*'`, or `b'+'`.
    Bullet(u8),
    /// Ordered list. The inner byte is the delimiter: `b'.'` or `b')'`.
    Ordered(u8),
}

/// Column alignment in a [`TableData`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum TableAlignment {
    /// No alignment specified (`---`).
    None,
    /// Left-aligned (`:---`).
    Left,
    /// Center-aligned (`:---:`).
    Center,
    /// Right-aligned (`---:`).
    Right,
}
