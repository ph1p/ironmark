//! HTML-to-AST parser that converts HTML into the Block AST.

use std::borrow::Cow;

use compact_str::CompactString;

use crate::ast::{Block, ListKind, TableAlignment, TableData};

use super::inline::{
    escape_markdown_text, find_attr, inline_to_markdown, normalize_whitespace, parse_inline_html,
};
use super::tokenizer::{HtmlToken, HtmlTokenizer};

/// Options for HTML-to-AST parsing.
#[derive(Clone, Debug)]
pub struct HtmlParseOptions {
    /// Maximum nesting depth for block elements (default: 128).
    pub max_nesting_depth: usize,
    /// How to handle inline elements that don't map to markdown (default: StripTags).
    pub unknown_inline_handling: UnknownInlineHandling,
    /// Maximum input size in bytes; 0 means no limit (default: 0).
    pub max_input_size: usize,
}

impl Default for HtmlParseOptions {
    fn default() -> Self {
        Self {
            max_nesting_depth: 128,
            unknown_inline_handling: UnknownInlineHandling::StripTags,
            max_input_size: 0,
        }
    }
}

/// How to handle HTML elements without Markdown equivalents.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum UnknownInlineHandling {
    /// Remove unknown tags, keep text content (default).
    StripTags,
    /// Keep as raw HTML in markdown output.
    PreserveAsHtml,
}

/// Parse an HTML string and return the block-level AST.
///
/// This converts HTML back into the same [`Block`] AST structure used by
/// the markdown parser, enabling HTML-to-Markdown conversion.
///
/// # Examples
///
/// ```
/// use ironmark::{parse_html_to_ast, HtmlParseOptions, Block};
///
/// let ast = parse_html_to_ast("<h1>Hello</h1><p>World</p>", &HtmlParseOptions::default());
/// match ast {
///     Block::Document { children } => {
///         assert_eq!(children.len(), 2);
///     }
///     _ => panic!("expected Document"),
/// }
/// ```
pub fn parse_html_to_ast(html: &str, options: &HtmlParseOptions) -> Block {
    let html = if options.max_input_size > 0 && html.len() > options.max_input_size {
        &html[..options.max_input_size]
    } else {
        html
    };

    let parser = HtmlParser::new(html, options);
    parser.parse()
}

/// Stack entry for tracking open block elements.
#[derive(Debug)]
struct OpenBlock {
    tag: String,
    children: Vec<Block>,
    /// For lists: the list kind
    list_kind: Option<ListKind>,
    /// For ordered lists: the start number
    list_start: u32,
    /// For list items: whether it's a task list item and its state
    task_checked: Option<bool>,
    /// For tables: accumulated table state
    table_state: Option<TableState>,
    /// For code blocks: language info
    code_info: Option<String>,
    /// For list items: whether an explicit `<p>` child was seen. HTML has no
    /// blank-line signal for list tightness, so an author-written `<p>` inside
    /// an `<li>` is the only evidence that the list is loose.
    saw_explicit_paragraph: bool,
    /// Accumulated text/inline content
    text_content: String,
}

impl OpenBlock {
    fn new(tag: &str) -> Self {
        Self {
            tag: tag.to_string(),
            children: Vec::new(),
            list_kind: None,
            list_start: 1,
            task_checked: None,
            table_state: None,
            code_info: None,
            saw_explicit_paragraph: false,
            text_content: String::new(),
        }
    }
}

/// State for parsing tables.
#[derive(Debug, Default)]
struct TableState {
    alignments: Vec<TableAlignment>,
    header: Vec<CompactString>,
    rows: Vec<CompactString>,
    num_cols: usize,
    in_header: bool,
    current_row: Vec<String>,
}

/// HTML to AST parser.
struct HtmlParser<'a> {
    tokenizer: HtmlTokenizer<'a>,
    options: &'a HtmlParseOptions,
    /// Stack of open block elements.
    stack: Vec<OpenBlock>,
}

impl<'a> HtmlParser<'a> {
    fn new(html: &'a str, options: &'a HtmlParseOptions) -> Self {
        Self {
            tokenizer: HtmlTokenizer::new(html),
            options,
            stack: vec![OpenBlock::new("document")],
        }
    }

    fn parse(mut self) -> Block {
        while let Some(token) = self.tokenizer.next_token() {
            self.handle_token(token);
        }

        // Close any remaining open blocks
        while self.stack.len() > 1 {
            self.close_current_block();
        }

        // Finalize document
        let doc = self.stack.pop().unwrap();
        Block::Document {
            children: doc.children,
        }
    }

    fn handle_token(&mut self, token: HtmlToken<'_>) {
        match token {
            HtmlToken::StartTag {
                name,
                attrs,
                self_closing,
            } => {
                self.handle_start_tag(&name, &attrs, self_closing);
            }
            HtmlToken::EndTag { name } => {
                self.handle_end_tag(&name);
            }
            HtmlToken::Text(text) => {
                self.handle_text(&text);
            }
            // Raw-text elements (script/style/...) carry no document content.
            HtmlToken::RawText { .. } | HtmlToken::Comment(_) | HtmlToken::Doctype(_) => {
                // Ignore
            }
        }
    }

    fn handle_start_tag(
        &mut self,
        name: &str,
        attrs: &[(Cow<'_, str>, Cow<'_, str>)],
        self_closing: bool,
    ) {
        // First, check if we need to close any incompatible blocks
        self.auto_close_for_tag(name);

        match name {
            // Block elements
            "p" | "pre" | "blockquote" | "li" | "h1" | "h2" | "h3" | "h4" | "h5" | "h6" => {
                if name == "p"
                    && let Some(li) = self.find_li_mut()
                {
                    li.saw_explicit_paragraph = true;
                }
                self.open_block(name);
            }
            "ul" => {
                if let Some(block) = self.open_block("ul") {
                    block.list_kind = Some(ListKind::Bullet(b'-'));
                }
            }
            "ol" => {
                let start = find_attr(attrs, "start")
                    .and_then(|s| s.parse().ok())
                    .unwrap_or(1);
                if let Some(block) = self.open_block("ol") {
                    block.list_kind = Some(ListKind::Ordered(b'.'));
                    block.list_start = start;
                }
            }
            "table" => {
                if let Some(block) = self.open_block("table") {
                    block.table_state = Some(TableState::default());
                }
            }
            "thead" => {
                if let Some(table) = self.find_table_mut() {
                    table.in_header = true;
                }
            }
            "tbody" => {
                if let Some(table) = self.find_table_mut() {
                    table.in_header = false;
                }
            }
            "tr" => {
                if let Some(table) = self.find_table_mut() {
                    table.current_row.clear();
                }
            }
            "th" | "td" => {
                // Get alignment from style or align attribute
                let alignment = find_attr(attrs, "align")
                    .and_then(|s| parse_alignment(&s))
                    .or_else(|| find_attr(attrs, "style").and_then(|s| parse_style_alignment(&s)))
                    .unwrap_or(TableAlignment::None);

                if let Some(table) = self.find_table_mut()
                    && table.in_header
                    && table.alignments.len() < 100
                {
                    // Limit columns
                    table.alignments.push(alignment);
                }
                if self.can_open_block() {
                    self.stack.push(OpenBlock::new(name));
                }
            }
            "hr" => {
                self.flush_text();
                self.push_block(Block::ThematicBreak);
            }
            "br" => {
                // Keep it as markup so the inline pass emits a real hard break;
                // writing "  \n" here would be swallowed by whitespace
                // normalization on the plain-text path.
                let current = self.stack.last_mut().unwrap();
                current.text_content.push_str("<br>");
            }
            "code" => {
                // Check if inside <pre>
                if self.is_inside("pre") {
                    // Code block - extract language from class
                    let info = find_attr(attrs, "class")
                        .and_then(|c| extract_language_from_class(&c))
                        .unwrap_or_default();
                    if let Some(pre) = self.stack.last_mut() {
                        pre.code_info = Some(info);
                    }
                } else {
                    // Inline code - handled as inline element
                    let current = self.stack.last_mut().unwrap();
                    current.text_content.push_str("<code>");
                }
            }
            "div" | "section" | "article" | "main" | "header" | "footer" | "nav" | "aside"
            | "figure" | "figcaption" | "dl" | "dd" | "details" | "summary" | "fieldset"
            | "form" | "address" | "hgroup" => {
                // Treat as generic block container: contributes no markup of
                // its own, but its children/text must survive.
                self.open_block(name);
            }
            // A definition term reads as its own short block.
            "dt" => {
                self.open_block("p");
            }
            "input" => {
                // Check for task list checkbox
                let is_checkbox = find_attr(attrs, "type")
                    .map(|t| t == "checkbox")
                    .unwrap_or(false);
                if is_checkbox {
                    let checked = attrs.iter().any(|(k, _)| k == "checked");
                    // Find the enclosing list item
                    if let Some(li) = self.find_li_mut() {
                        li.task_checked = Some(checked);
                    }
                }
            }
            // Inline elements - append to text content as HTML
            "strong" | "b" | "em" | "i" | "del" | "s" | "strike" | "mark" | "u" | "ins" | "a"
            | "img" | "span" | "sub" | "sup" | "abbr" | "cite" | "q" | "small" | "time" | "kbd"
            | "var" | "samp" | "dfn" => {
                let current = self.stack.last_mut().unwrap();
                super::inline::write_open_tag(&mut current.text_content, name, attrs, self_closing);
            }
            _ => {
                // Unknown tag - ignore or handle based on options
            }
        }
    }

    fn handle_end_tag(&mut self, name: &str) {
        match name {
            "p" if self.is_current("p") => {
                self.close_paragraph();
            }
            "h1" | "h2" | "h3" | "h4" | "h5" | "h6" if self.is_current(name) => {
                self.close_heading();
            }
            "pre" if self.is_current("pre") => {
                self.close_code_block();
            }
            "blockquote" if self.is_current("blockquote") => {
                self.close_blockquote();
            }
            "ul" | "ol" if self.is_current(name) => {
                self.close_list();
            }
            "li" if self.is_current("li") => {
                self.close_list_item();
            }
            "table" if self.is_current("table") => {
                self.close_table();
            }
            "tr" => {
                self.close_table_row();
            }
            "th" | "td" if self.is_current(name) => {
                self.close_table_cell();
            }
            "thead" | "tbody" => {
                // Just a marker, no action needed
            }
            "div" | "section" | "article" | "main" | "header" | "footer" | "nav" | "aside"
            | "figure" | "figcaption" | "dl" | "dd" | "details" | "summary" | "fieldset"
            | "form" | "address" | "hgroup"
                if self.is_current(name) =>
            {
                self.close_generic_block();
            }
            "dt" if self.is_current("p") => {
                self.close_paragraph();
            }
            "code" if !self.is_inside("pre") => {
                // Inline code end
                let current = self.stack.last_mut().unwrap();
                current.text_content.push_str("</code>");
            }
            // Inline elements
            "strong" | "b" | "em" | "i" | "del" | "s" | "strike" | "mark" | "u" | "ins" | "a"
            | "span" | "sub" | "sup" | "abbr" | "cite" | "q" | "small" | "time" | "kbd" | "var"
            | "samp" | "dfn" => {
                let current = self.stack.last_mut().unwrap();
                current.text_content.push_str("</");
                current.text_content.push_str(name);
                current.text_content.push('>');
            }
            _ => {}
        }
    }

    fn handle_text(&mut self, text: &str) {
        let current = self.stack.last_mut().unwrap();
        current.text_content.push_str(text);
    }

    // Helper methods

    fn is_current(&self, tag: &str) -> bool {
        self.stack.last().map(|b| b.tag == tag).unwrap_or(false)
    }

    fn can_open_block(&self) -> bool {
        self.stack.len().saturating_sub(1) < self.options.max_nesting_depth
    }

    fn is_inside(&self, tag: &str) -> bool {
        self.stack.iter().any(|b| b.tag == tag)
    }

    fn find_table_mut(&mut self) -> Option<&mut TableState> {
        self.stack
            .iter_mut()
            .rev()
            .find_map(|b| b.table_state.as_mut())
    }

    fn find_li_mut(&mut self) -> Option<&mut OpenBlock> {
        self.stack.iter_mut().rev().find(|b| b.tag == "li")
    }

    fn auto_close_for_tag(&mut self, tag: &str) {
        // Auto-close certain tags when a new block starts
        match tag {
            "p" | "h1" | "h2" | "h3" | "h4" | "h5" | "h6" | "ul" | "ol" | "blockquote" | "pre"
            | "table" | "hr" | "div" | "section" | "article" | "figure" | "figcaption" | "dl"
            | "dt" | "dd" | "details" | "summary" | "address"
                if self.is_current("p") =>
            {
                // Close any open paragraph
                self.close_paragraph();
            }
            "li" if self.is_current("li") => {
                // Close previous li if any
                self.close_list_item();
            }
            _ => {}
        }
    }

    /// Flush pending text and push a new open block (depth-guarded).
    /// Returns the new block so callers can set extra fields.
    fn open_block(&mut self, tag: &str) -> Option<&mut OpenBlock> {
        self.flush_text();
        if self.can_open_block() {
            self.stack.push(OpenBlock::new(tag));
            self.stack.last_mut()
        } else {
            None
        }
    }

    /// Emit any text pending on the current container as a paragraph child.
    ///
    /// This runs whenever a nested block opens. Discarding the pending text
    /// instead would lose content: in `<li>a<ul>…</ul></li>` the `a` belongs to
    /// the item, before the nested list.
    fn flush_text(&mut self) {
        // `pre` accumulates verbatim code; never reinterpret it as a paragraph.
        if self.is_current("pre") {
            return;
        }
        let text = {
            let current = self.stack.last_mut().unwrap();
            std::mem::take(&mut current.text_content)
        };
        let trimmed = text.trim();

        if !trimmed.is_empty() {
            let raw = self.convert_inline_content(trimmed);
            self.stack
                .last_mut()
                .unwrap()
                .children
                .push(Block::Paragraph { raw });
        }
    }

    fn push_block(&mut self, block: Block) {
        if let Some(parent) = self.stack.last_mut() {
            parent.children.push(block);
        }
    }

    fn close_current_block(&mut self) {
        if self.stack.len() <= 1 {
            return;
        }

        let current = self.stack.last().unwrap().tag.clone();
        match current.as_str() {
            "p" => self.close_paragraph(),
            "h1" | "h2" | "h3" | "h4" | "h5" | "h6" => self.close_heading(),
            "pre" => self.close_code_block(),
            "blockquote" => self.close_blockquote(),
            "ul" | "ol" => self.close_list(),
            "li" => self.close_list_item(),
            "table" => self.close_table(),
            "th" | "td" => self.close_table_cell(),
            _ => self.close_generic_block(),
        }
    }

    fn close_paragraph(&mut self) {
        if let Some(block) = self.stack.pop() {
            let raw = self.convert_inline_content(&block.text_content);
            self.push_block(Block::Paragraph { raw });
        }
    }

    fn close_heading(&mut self) {
        if let Some(block) = self.stack.pop() {
            let level = block
                .tag
                .chars()
                .nth(1)
                .and_then(|c| c.to_digit(10))
                .unwrap_or(1) as u8;
            let raw = self.convert_inline_content(&block.text_content);
            self.push_block(Block::Heading { level, raw });
        }
    }

    fn close_code_block(&mut self) {
        if let Some(block) = self.stack.pop() {
            let info = block.code_info.unwrap_or_default();
            let literal = block.text_content;
            self.push_block(Block::CodeBlock {
                info: CompactString::new(&info),
                literal,
            });
        }
    }

    fn close_blockquote(&mut self) {
        if let Some(mut block) = self.stack.pop() {
            // Flush any remaining text
            let text = std::mem::take(&mut block.text_content);
            let trimmed = text.trim();
            if !trimmed.is_empty() {
                let raw = self.convert_inline_content(trimmed);
                block.children.push(Block::Paragraph { raw });
            }
            self.push_block(Block::BlockQuote {
                children: block.children,
            });
        }
    }

    fn close_list(&mut self) {
        if let Some(mut block) = self.stack.pop() {
            // Flush any remaining text
            let text = std::mem::take(&mut block.text_content);
            let trimmed = text.trim();
            if !trimmed.is_empty() {
                let raw = self.convert_inline_content(trimmed);
                block.children.push(Block::Paragraph { raw });
            }

            let kind = block.list_kind.unwrap_or(ListKind::Bullet(b'-'));
            let start = block.list_start;

            // Determine if tight. HTML gives no blank-line signal, so treat a
            // list as tight unless an item holds genuine multi-block content.
            // A leading paragraph followed only by nested lists is the normal
            // shape of `<li>text<ul>…</ul></li>` and stays tight; two
            // paragraphs (or a paragraph plus a code block) are loose.
            let tight = !block.saw_explicit_paragraph
                && block.children.iter().all(|child| {
                    let Block::ListItem { children, .. } = child else {
                        return true;
                    };
                    let non_list = children
                        .iter()
                        .filter(|c| !matches!(c, Block::List { .. }))
                        .count();
                    non_list <= 1
                        && children
                            .iter()
                            .all(|c| matches!(c, Block::Paragraph { .. } | Block::List { .. }))
                });

            self.push_block(Block::List {
                kind,
                start,
                tight,
                children: block.children,
            });
        }
    }

    fn close_list_item(&mut self) {
        if let Some(mut block) = self.stack.pop() {
            // Flush remaining text as paragraph
            let text = std::mem::take(&mut block.text_content);
            let trimmed = text.trim();
            if !trimmed.is_empty() {
                let raw = self.convert_inline_content(trimmed);
                block.children.push(Block::Paragraph { raw });
            }

            let explicit_para = block.saw_explicit_paragraph;
            self.push_block(Block::ListItem {
                children: block.children,
                checked: block.task_checked,
            });
            // Propagate the looseness signal to the enclosing list.
            if explicit_para && let Some(parent) = self.stack.last_mut() {
                parent.saw_explicit_paragraph = true;
            }
        }
    }

    fn close_table(&mut self) {
        if let Some(block) = self.stack.pop()
            && let Some(table) = block.table_state
            && table.num_cols > 0
        {
            self.push_block(Block::Table(Box::new(TableData {
                alignments: table.alignments,
                num_cols: table.num_cols,
                header: table.header,
                rows: table.rows,
            })));
        }
    }

    fn close_table_row(&mut self) {
        // Find table and add current row
        if let Some(table) = self.find_table_mut() {
            let row = std::mem::take(&mut table.current_row);
            if !row.is_empty() {
                if table.num_cols == 0 {
                    table.num_cols = row.len();
                }
                if table.in_header || table.header.is_empty() {
                    // This is the header row
                    table.header = row.into_iter().map(|s| CompactString::new(&s)).collect();
                    table.in_header = false;
                } else {
                    // Body row
                    for cell in row {
                        table.rows.push(CompactString::new(&cell));
                    }
                }
            }
        }
    }

    fn close_table_cell(&mut self) {
        if let Some(block) = self.stack.pop() {
            let content = self.convert_inline_content(&block.text_content);
            if let Some(table) = self.find_table_mut() {
                table.current_row.push(content);
            }
        }
    }

    fn close_generic_block(&mut self) {
        if let Some(mut block) = self.stack.pop() {
            // Flush any remaining text
            let text = std::mem::take(&mut block.text_content);
            let trimmed = text.trim();
            if !trimmed.is_empty() {
                let raw = self.convert_inline_content(trimmed);
                block.children.push(Block::Paragraph { raw });
            }
            // Push children to parent
            for child in block.children {
                self.push_block(child);
            }
        }
    }

    /// Convert HTML inline content to Markdown syntax.
    fn convert_inline_content(&self, html: &str) -> String {
        let trimmed = html.trim();
        if trimmed.is_empty() {
            return String::new();
        }

        // Fast path: no tags to interpret, but the text still needs Markdown
        // escaping and whitespace normalization or it will re-parse as markup
        // on the way back (e.g. a literal "# x" becoming a heading).
        if !trimmed.contains('<') {
            let normalized = normalize_whitespace(trimmed);
            let mut out = String::with_capacity(normalized.len());
            escape_markdown_text(&normalized, &mut out);
            return out;
        }

        // Parse inline HTML and convert to Markdown
        let elements = parse_inline_html(trimmed, self.options.unknown_inline_handling);
        inline_to_markdown(&elements)
    }
}

// Helper functions

fn extract_language_from_class(class: &str) -> Option<String> {
    for part in class.split_whitespace() {
        if let Some(lang) = part.strip_prefix("language-") {
            return Some(lang.to_string());
        }
        if let Some(lang) = part.strip_prefix("lang-") {
            return Some(lang.to_string());
        }
    }
    None
}

fn parse_alignment(align: &str) -> Option<TableAlignment> {
    match align {
        a if a.eq_ignore_ascii_case("left") => Some(TableAlignment::Left),
        a if a.eq_ignore_ascii_case("center") => Some(TableAlignment::Center),
        a if a.eq_ignore_ascii_case("right") => Some(TableAlignment::Right),
        _ => None,
    }
}

fn parse_style_alignment(style: &str) -> Option<TableAlignment> {
    let style_lower = style.to_ascii_lowercase();
    if style_lower.contains("text-align") {
        if style_lower.contains("left") {
            return Some(TableAlignment::Left);
        }
        if style_lower.contains("center") {
            return Some(TableAlignment::Center);
        }
        if style_lower.contains("right") {
            return Some(TableAlignment::Right);
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(html: &str) -> Block {
        parse_html_to_ast(html, &HtmlParseOptions::default())
    }

    fn get_children(block: &Block) -> &[Block] {
        match block {
            Block::Document { children } => children,
            _ => panic!("Expected Document"),
        }
    }

    #[test]
    fn test_paragraph() {
        let ast = parse("<p>Hello world</p>");
        let children = get_children(&ast);
        assert_eq!(children.len(), 1);
        assert!(matches!(&children[0], Block::Paragraph { raw } if raw == "Hello world"));
    }

    #[test]
    fn test_headings() {
        let ast = parse("<h1>Title</h1><h2>Subtitle</h2>");
        let children = get_children(&ast);
        assert_eq!(children.len(), 2);
        assert!(matches!(&children[0], Block::Heading { level: 1, raw } if raw == "Title"));
        assert!(matches!(&children[1], Block::Heading { level: 2, raw } if raw == "Subtitle"));
    }

    #[test]
    fn test_code_block() {
        let ast = parse(r#"<pre><code class="language-rust">fn main() {}</code></pre>"#);
        let children = get_children(&ast);
        assert_eq!(children.len(), 1);
        if let Block::CodeBlock { info, literal } = &children[0] {
            assert_eq!(info.as_str(), "rust");
            assert_eq!(literal, "fn main() {}");
        } else {
            panic!("Expected CodeBlock");
        }
    }

    #[test]
    fn test_unordered_list() {
        let ast = parse("<ul><li>Item 1</li><li>Item 2</li></ul>");
        let children = get_children(&ast);
        assert_eq!(children.len(), 1);
        if let Block::List { kind, children, .. } = &children[0] {
            assert!(matches!(kind, ListKind::Bullet(_)));
            assert_eq!(children.len(), 2);
        } else {
            panic!("Expected List");
        }
    }

    #[test]
    fn test_ordered_list() {
        let ast = parse(r#"<ol start="5"><li>Item A</li><li>Item B</li></ol>"#);
        let children = get_children(&ast);
        assert_eq!(children.len(), 1);
        if let Block::List {
            kind,
            start,
            children,
            ..
        } = &children[0]
        {
            assert!(matches!(kind, ListKind::Ordered(_)));
            assert_eq!(*start, 5);
            assert_eq!(children.len(), 2);
        } else {
            panic!("Expected List");
        }
    }

    #[test]
    fn test_blockquote() {
        let ast = parse("<blockquote><p>Quote text</p></blockquote>");
        let children = get_children(&ast);
        assert_eq!(children.len(), 1);
        if let Block::BlockQuote { children } = &children[0] {
            assert_eq!(children.len(), 1);
            assert!(matches!(&children[0], Block::Paragraph { .. }));
        } else {
            panic!("Expected BlockQuote");
        }
    }

    #[test]
    fn test_thematic_break() {
        let ast = parse("<p>Before</p><hr><p>After</p>");
        let children = get_children(&ast);
        assert_eq!(children.len(), 3);
        assert!(matches!(&children[1], Block::ThematicBreak));
    }

    #[test]
    fn test_inline_bold() {
        let ast = parse("<p><strong>Bold</strong> text</p>");
        let children = get_children(&ast);
        if let Block::Paragraph { raw } = &children[0] {
            assert!(raw.contains("**Bold**"));
        } else {
            panic!("Expected Paragraph");
        }
    }

    #[test]
    fn test_inline_link() {
        let ast = parse(r#"<p><a href="https://example.com">Link</a></p>"#);
        let children = get_children(&ast);
        if let Block::Paragraph { raw } = &children[0] {
            assert!(raw.contains("[Link](https://example.com)"));
        } else {
            panic!("Expected Paragraph");
        }
    }

    #[test]
    fn test_table() {
        let ast = parse(
            "<table><thead><tr><th>A</th><th>B</th></tr></thead><tbody><tr><td>1</td><td>2</td></tr></tbody></table>",
        );
        let children = get_children(&ast);
        assert_eq!(children.len(), 1);
        if let Block::Table(table) = &children[0] {
            assert_eq!(table.num_cols, 2);
            assert_eq!(table.header.len(), 2);
            assert_eq!(table.rows.len(), 2);
        } else {
            panic!("Expected Table");
        }
    }
}
