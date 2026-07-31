mod html_block;
mod leaf_blocks;
mod link_ref_def;
mod parser;

use html_block::*;
use leaf_blocks::*;
use link_ref_def::*;

use crate::ParseOptions;
use crate::ast::{Block, ListKind, TableAlignment};
use crate::html::trim_cr;
use compact_str::CompactString;
use smallvec::SmallVec;
use std::borrow::Cow;

#[cfg(feature = "html")]
use crate::inline::InlineBuffers;
use crate::inline::LinkRefMap;
#[cfg(feature = "html")]
use crate::render::render_block;

/// Parse a Markdown string and return the rendered HTML.
///
/// # Examples
///
/// ```
/// use ironmark::{render_html, ParseOptions};
///
/// let html = render_html("**bold** and *italic*", &ParseOptions::default());
/// assert!(html.contains("<strong>bold</strong>"));
/// ```
#[cfg(feature = "html")]
/// Truncate `s` to at most `limit` bytes, backing off to the nearest UTF-8 char
/// boundary. A `limit` of 0 means unlimited and returns `s` unchanged.
fn truncate_to_limit(s: &str, limit: usize) -> &str {
    if limit == 0 || s.len() <= limit {
        return s;
    }
    let mut end = limit;
    while end > 0 && !s.is_char_boundary(end) {
        end -= 1;
    }
    &s[..end]
}

pub fn render_html(markdown: &str, options: &ParseOptions) -> String {
    // Source byte offsets are tracked as u32 in render mode (`src_ranges`), so the
    // input is hard-capped at u32::MAX bytes to keep those casts from wrapping —
    // a wrapped range would slice the source out of bounds and panic. This is below
    // any user `max_input_size` and applies even when that limit is 0 (unlimited).
    let u32_cap = u32::MAX as usize;
    let effective_limit = match options.max_input_size {
        0 => u32_cap,
        n => n.min(u32_cap),
    };
    let markdown = truncate_to_limit(markdown, effective_limit);
    let mut parser = BlockParser::new(markdown, options);
    parser.render_mode = true;
    parser.src_ranges = Vec::with_capacity(estimate_block_count(markdown.len()));
    let doc = parser.parse();
    let mut out = if markdown.len() <= 256 {
        String::with_capacity(markdown.len() + 32)
    } else {
        String::with_capacity(markdown.len() * 2)
    };
    let refs = parser.ref_defs;
    let src_ranges = parser.src_ranges;
    let mut bufs = InlineBuffers::new();
    bufs.prepare(options);
    render_block(
        &doc,
        &refs,
        &mut out,
        options,
        &mut bufs,
        markdown,
        &src_ranges,
    );
    out
}

/// Parse a Markdown string and return the block-level AST.
///
/// This returns the raw AST without rendering to HTML, useful for
/// programmatic inspection or transformation of the document structure.
///
/// # Examples
///
/// ```
/// use ironmark::{parse_markdown, ParseOptions, Block};
///
/// let ast = parse_markdown("# Hello", &ParseOptions::default());
/// match &ast {
///     Block::Document { children } => {
///         assert_eq!(children.len(), 1);
///     }
///     _ => panic!("expected Document"),
/// }
/// ```
pub fn parse_markdown(markdown: &str, options: &ParseOptions) -> Block {
    let markdown = truncate_to_limit(markdown, options.max_input_size);
    let mut parser = BlockParser::new(markdown, options);
    parser.parse()
}

pub fn benchmark_parse_table_row(line: &str, num_cols: usize) -> Vec<CompactString> {
    parse_table_row(line, num_cols).into_vec()
}

#[cfg(feature = "html")]
#[doc(hidden)]
pub fn benchmark_render_html_parse_phase(
    markdown: &str,
    options: &ParseOptions,
) -> (Block, Vec<(u32, u32)>) {
    let mut parser = BlockParser::new(markdown, options);
    parser.render_mode = true;
    let doc = parser.parse();
    let ranges = parser.src_ranges;
    (doc, ranges)
}

#[derive(Clone, Debug)]
struct Line<'a> {
    raw: &'a str,
    col_offset: usize,
    byte_offset: usize,
    partial_spaces: usize,
    cached_ns_col: usize,
    cached_ns_off: usize,
    cached_ns_byte: u8,
}

impl<'a> Line<'a> {
    fn new(raw: &'a str) -> Self {
        Self {
            raw,
            col_offset: 0,
            byte_offset: 0,
            partial_spaces: 0,
            cached_ns_col: 0,
            cached_ns_off: 0,
            cached_ns_byte: 0,
        }
    }

    fn remainder(&self) -> &'a str {
        if self.byte_offset >= self.raw.len() {
            ""
        } else {
            &self.raw[self.byte_offset..]
        }
    }

    #[inline(always)]
    fn is_blank(&mut self) -> bool {
        if self.partial_spaces > 0 {
            return false;
        }
        let (_, ns_off, ns_byte) = self.peek_nonspace_col();
        ns_byte == 0 && ns_off >= self.raw.len()
    }

    #[inline]
    fn skip_indent(&mut self, max: usize) -> usize {
        let bytes = self.raw.as_bytes();
        let mut cols = 0;
        if self.partial_spaces > 0 {
            let consume = self.partial_spaces.min(max);
            cols += consume;
            self.col_offset += consume;
            self.partial_spaces -= consume;
            if cols >= max {
                return cols;
            }
        }
        let remaining = max - cols;
        let end = (self.byte_offset + remaining).min(bytes.len());
        if end > self.byte_offset {
            let mut fast_end = self.byte_offset;
            while fast_end < end && bytes[fast_end] == b' ' {
                fast_end += 1;
            }
            let fast_count = fast_end - self.byte_offset;
            if fast_count >= remaining {
                self.byte_offset += remaining;
                self.col_offset += remaining;
                return max;
            }
            if fast_count > 0 {
                cols += fast_count;
                self.byte_offset += fast_count;
                self.col_offset += fast_count;
            }
        }
        while self.byte_offset < bytes.len() && cols < max {
            match bytes[self.byte_offset] {
                b' ' => {
                    cols += 1;
                    self.byte_offset += 1;
                    self.col_offset += 1;
                }
                b'\t' => {
                    let tab_width = 4 - (self.col_offset % 4);
                    if cols + tab_width > max {
                        let consume = max - cols;
                        self.partial_spaces = tab_width - consume;
                        self.col_offset += consume;
                        self.byte_offset += 1;
                        cols += consume;
                        break;
                    }
                    cols += tab_width;
                    self.byte_offset += 1;
                    self.col_offset += tab_width;
                }
                _ => break,
            }
        }
        cols
    }

    fn advance_columns(&mut self, n: usize) {
        let bytes = self.raw.as_bytes();
        let mut cols = 0;
        while self.byte_offset < bytes.len() && cols < n {
            match bytes[self.byte_offset] {
                b' ' => {
                    cols += 1;
                    self.byte_offset += 1;
                    self.col_offset += 1;
                }
                b'\t' => {
                    let tab_width = 4 - (self.col_offset % 4);
                    cols += tab_width;
                    self.byte_offset += 1;
                    self.col_offset += tab_width;
                }
                _ => {
                    cols += 1;
                    self.byte_offset += 1;
                    self.col_offset += 1;
                }
            }
        }
    }

    #[inline(always)]
    fn peek_nonspace_col(&mut self) -> (usize, usize, u8) {
        if self.cached_ns_off >= self.byte_offset
            && (self.cached_ns_byte != 0 || self.cached_ns_off >= self.raw.len())
        {
            return (self.cached_ns_col, self.cached_ns_off, self.cached_ns_byte);
        }
        let bytes = self.raw.as_bytes();
        let mut col = self.col_offset;
        let mut off = self.byte_offset;
        if self.partial_spaces > 0 {
            col += self.partial_spaces;
        }
        while off < bytes.len() {
            match bytes[off] {
                b' ' => {
                    col += 1;
                    off += 1;
                }
                b'\t' => {
                    col += 4 - (col % 4);
                    off += 1;
                }
                b => {
                    self.cached_ns_col = col;
                    self.cached_ns_off = off;
                    self.cached_ns_byte = b;
                    return (col, off, b);
                }
            }
        }
        self.cached_ns_col = col;
        self.cached_ns_off = off;
        self.cached_ns_byte = 0;
        (col, off, 0)
    }

    fn advance_to_nonspace(&mut self) {
        self.partial_spaces = 0;
        let (col, off, _) = self.peek_nonspace_col();
        self.col_offset = col;
        self.byte_offset = off;
    }

    fn remainder_with_partial(&self) -> Cow<'a, str> {
        if self.partial_spaces > 0 {
            static SPACES: &str = "    ";
            let rem = self.remainder();
            let mut s = String::with_capacity(self.partial_spaces + rem.len());
            s.push_str(&SPACES[..self.partial_spaces]);
            s.push_str(rem);
            Cow::Owned(s)
        } else {
            Cow::Borrowed(self.remainder())
        }
    }
}

#[derive(Clone, Debug)]
struct FencedCodeData {
    fence_char: u8,
    fence_len: usize,
    fence_indent: usize,
    info: CompactString,
}

type TableRow = SmallVec<[CompactString; 8]>;

#[derive(Clone, Debug)]
struct TableData {
    alignments: SmallVec<[TableAlignment; 8]>,
    header: TableRow,
    rows: Vec<TableRow>,
}

#[derive(Clone, Debug)]
enum OpenBlockType {
    Document,
    BlockQuote,
    ListItem {
        content_col: usize,
        started_blank: bool,
    },
    // Not boxed: the heap allocation for Box<FencedCodeData> is measurably expensive
    // when parsing many top-level fenced code blocks (100+ boxes/frees ≈ 3 µs).
    // OpenBlockType lives in a Vec<OpenBlock> with max depth ~16, so the extra inline
    // size is fine.
    FencedCode(FencedCodeData),
    IndentedCode,
    HtmlBlock {
        end_condition: HtmlBlockEnd,
    },
    Paragraph,
    Table(Box<TableData>),
}

#[derive(Copy, Clone, Debug, PartialEq)]
enum HtmlBlockEnd {
    EndTag(&'static str),
    Comment,
    ProcessingInstruction,
    Declaration,
    Cdata,
    BlankLine,
}

#[derive(Clone, Debug)]
struct OpenBlock {
    block_type: OpenBlockType,
    content: String,
    children: Vec<Block>,
    had_blank_in_item: bool,
    list_has_blank_between: bool,
    content_has_newline: bool,
    checked: Option<bool>,
    list_start: u32,
    list_kind: Option<ListKind>,
    /// Render-mode deferred text: the byte range in the source string holding this
    /// block's literal content (fenced code with no indent stripping/CR, or a
    /// paragraph's pending text). When set, `content` is left empty and the range
    /// is used directly at render time to avoid copying.
    src_range: Option<(u32, u32)>,
}

impl OpenBlock {
    #[inline]
    fn new(block_type: OpenBlockType) -> Self {
        Self {
            block_type,
            content: String::new(),
            children: Vec::new(),
            had_blank_in_item: false,
            list_has_blank_between: false,
            content_has_newline: false,
            checked: None,
            list_start: 0,
            list_kind: None,
            src_range: None,
        }
    }

    #[inline]
    fn with_content_capacity(block_type: OpenBlockType, cap: usize) -> Self {
        Self {
            content: String::with_capacity(cap),
            ..Self::new(block_type)
        }
    }

    #[inline]
    fn new_list_item(content_col: usize, started_blank: bool) -> Self {
        Self {
            block_type: OpenBlockType::ListItem {
                content_col,
                started_blank,
            },
            content: String::new(),
            children: Vec::new(),
            had_blank_in_item: false,
            list_has_blank_between: false,
            content_has_newline: false,
            checked: None,
            list_start: 0,
            list_kind: None,
            src_range: None,
        }
    }
}

#[inline(always)]
fn estimate_block_count(input_len: usize) -> usize {
    (input_len / 50).clamp(8, 256)
}

pub(crate) struct BlockParser<'a> {
    input: &'a str,
    pub(crate) ref_defs: LinkRefMap,
    open: Vec<OpenBlock>,
    enable_tables: bool,
    enable_task_lists: bool,
    open_blockquotes: usize,
    list_indent_sum: usize,
    last_list_item_idx: Option<usize>,
    max_nesting_depth: usize,
    enable_indented_code_blocks: bool,
    permissive_atx_headers: bool,
    no_html_blocks: bool,
    /// Render-mode deferred-text stream, shared by code blocks, paragraphs, and
    /// headings. Each entry is `(start, end)` into `self.input`, pushed in document
    /// order by `defer_raw`/`finalize_block`. The corresponding `literal`/`raw` is
    /// the empty sentinel String; the renderer consumes exactly one entry per empty
    /// sentinel, so every render-mode empty sentinel MUST have pushed a range.
    pub(crate) src_ranges: Vec<(u32, u32)>,
    /// When `true`, fenced code blocks with no CR/indent can store a source range
    /// instead of copying the literal content. Used only by `render_html`; the public
    /// `parse_markdown` API always copies so callers get a fully-populated AST.
    render_mode: bool,
}

impl<'a> BlockParser<'a> {
    pub fn new(input: &'a str, options: &ParseOptions) -> Self {
        let mut doc = OpenBlock::new(OpenBlockType::Document);
        doc.children = Vec::with_capacity(estimate_block_count(input.len()));
        let mut open = Vec::with_capacity(16);
        open.push(doc);
        Self {
            input,
            ref_defs: LinkRefMap::default(),
            open,
            enable_tables: options.enable_tables,
            enable_task_lists: options.enable_task_lists,
            open_blockquotes: 0,
            list_indent_sum: 0,
            last_list_item_idx: None,
            max_nesting_depth: options.max_nesting_depth,
            enable_indented_code_blocks: options.enable_indented_code_blocks,
            permissive_atx_headers: options.permissive_atx_headers,
            no_html_blocks: options.no_html_blocks || options.disable_raw_html,
            src_ranges: Vec::new(),
            render_mode: false,
        }
    }

    pub fn parse(&mut self) -> Block {
        let input = self.input;
        let bytes = input.as_bytes();
        let len = bytes.len();
        let mut start = 0;
        while start < len {
            let end = memchr_newline(bytes, start);
            let raw_line = &input[start..end];
            let raw_line = trim_cr(raw_line);
            let line = Line::new(raw_line);
            self.process_line(line);

            let tip = self.open.len() - 1;
            if tip > 0
                && let OpenBlockType::FencedCode(ref fc_data) = self.open[tip].block_type
            {
                let fc = fc_data.fence_char;
                let fl = fc_data.fence_len;
                let fi = fc_data.fence_indent;
                if tip == 1 && fi == 0 {
                    start = end + 1;
                    start = self.bulk_scan_fenced_code(input, bytes, start, len, fc, fl);
                    continue;
                }
                // A fence inside containers can still be bulk-scanned, but the
                // shape test and the two scanners live behind one outlined call
                // so this loop keeps the register/stack shape it has for the
                // common top-level case above.
                if let Some(next) =
                    self.bulk_scan_container_fenced_code(bytes, end + 1, tip, fc, fl, fi)
                {
                    start = next;
                    continue;
                }
            }

            start = end + 1;
        }
        while self.open.len() > 1 {
            self.close_top_block();
        }
        let doc = self.open.pop().unwrap();
        Block::Document {
            children: doc.children,
        }
    }

    #[inline(never)]
    fn bulk_scan_fenced_code(
        &mut self,
        input: &str,
        bytes: &[u8],
        start: usize,
        len: usize,
        fence_char: u8,
        fence_len: usize,
    ) -> usize {
        let content_start = start;

        // Scan for the fence char directly instead of walking line by line:
        // content lines rarely contain it, so one SIMD pass usually jumps
        // straight to the closing fence. A closing fence is a whole line
        // (≤3 leading spaces, then only fence chars + trailing ws); tabs in
        // the indent always exceed 3 columns and can never start one.
        let mut search = start;
        while search < len {
            let Some(off) = memchr::memchr(fence_char, &bytes[search..len]) else {
                break;
            };
            let i = search + off;
            let mut ls = i;
            while ls > content_start && i - ls < 3 && bytes[ls - 1] == b' ' {
                ls -= 1;
            }
            if ls == content_start || bytes[ls - 1] == b'\n' {
                let line_end = memchr_newline(bytes, i);
                let check_end = if line_end > ls && bytes[line_end - 1] == b'\r' {
                    line_end - 1
                } else {
                    line_end
                };
                if is_closing_fence(&bytes[ls..check_end], fence_char, fence_len) {
                    if ls > content_start {
                        // A line ending in `\r\n` requires CR stripping; detect it
                        // once over the content region (single-byte scan first —
                        // `\r` is absent from almost all inputs).
                        let region = &bytes[content_start..ls];
                        let has_cr = memchr::memchr(b'\r', region).is_some()
                            && memchr::memmem::find(region, b"\r\n").is_some();
                        if !has_cr {
                            // Fast path: content is a direct slice of source — record
                            // range, skip copy.
                            self.open[1].src_range = Some((content_start as u32, ls as u32));
                        } else {
                            self.push_bulk_content(input, content_start, ls, has_cr);
                        }
                    }
                    self.close_top_block();
                    return line_end + 1;
                }
                // Not a closing fence — nothing else on this line can be one.
                search = line_end + 1;
            } else {
                // Mid-line fence char — skip the whole run.
                let mut j = i + 1;
                while j < len && bytes[j] == fence_char {
                    j += 1;
                }
                search = j;
            }
        }

        if len > content_start {
            let region = &bytes[content_start..len];
            let has_cr = memchr::memchr(b'\r', region).is_some()
                && (memchr::memmem::find(region, b"\r\n").is_some() || bytes[len - 1] == b'\r');
            if !has_cr && bytes[len - 1] == b'\n' {
                // Fast path: source ends with '\n' — content is a direct slice of source,
                // no copy needed.
                self.open[1].src_range = Some((content_start as u32, len as u32));
            } else {
                // Either has CR escaping or the source doesn't end with '\n', so we must
                // copy and ensure a trailing newline.
                self.push_bulk_content(input, content_start, len, has_cr);
                let content = &mut self.open[1].content;
                if !content.ends_with('\n') {
                    content.push('\n');
                }
            }
        }
        len
    }

    /// Bulk-scan a fenced code block that sits inside containers, if the
    /// container stack has a shape whose per-line prefix can be reproduced
    /// without re-walking the stack.
    ///
    /// Only two shapes qualify: all list items (a fixed column indent) and all
    /// blockquotes (a fixed number of `>` markers). A mixed stack, or no
    /// container at all, returns `None` so the caller keeps its per-line path.
    ///
    /// Returns the offset to resume scanning from.
    #[inline(never)]
    fn bulk_scan_container_fenced_code(
        &mut self,
        bytes: &[u8],
        start: usize,
        tip: usize,
        fence_char: u8,
        fence_len: usize,
        fence_indent: usize,
    ) -> Option<usize> {
        // `open[0]` is the Document and `open[tip]` is the fence, so everything
        // between them is a container. All-list-items therefore implies zero
        // open blockquotes, and all-blockquotes implies `open_blockquotes ==
        // tip - 1` — no separate cross-check is needed. Testing the first frame
        // picks the candidate shape so only one full scan ever runs.
        let containers = &self.open[1..tip];
        match containers.first().map(|b| &b.block_type) {
            Some(OpenBlockType::ListItem { .. })
                if containers
                    .iter()
                    .all(|b| matches!(b.block_type, OpenBlockType::ListItem { .. })) =>
            {
                let strip = self.list_indent_sum + fence_indent;
                Some(self.bulk_scan_nested_fenced_code(bytes, start, fence_char, fence_len, strip))
            }
            Some(OpenBlockType::BlockQuote)
                if containers
                    .iter()
                    .all(|b| matches!(b.block_type, OpenBlockType::BlockQuote)) =>
            {
                Some(self.bulk_scan_quoted_fenced_code(
                    bytes,
                    start,
                    fence_char,
                    fence_len,
                    containers.len(),
                    fence_indent,
                ))
            }
            _ => None,
        }
    }

    /// Bulk-scan a fenced code block nested inside list items.
    ///
    /// `strip` columns of leading space are removed from every content line (the
    /// containers' content indent plus the fence's own indent), so the body is
    /// not a contiguous source slice and must be copied. The win over the
    /// per-line path is skipping the container-stack walk and the leaf-block
    /// probes for each line.
    ///
    /// Bails out (returning the line start it stopped at) as soon as a line is
    /// not a valid continuation, letting the normal per-line path handle the
    /// rest — so list-item termination and lazy continuation stay unchanged.
    fn bulk_scan_nested_fenced_code(
        &mut self,
        bytes: &[u8],
        mut start: usize,
        fence_char: u8,
        fence_len: usize,
        strip: usize,
    ) -> usize {
        let input = self.input;
        let len = bytes.len();
        let tip = self.open.len() - 1;
        while start < len {
            let end = memchr_newline(bytes, start);
            let mut line_end = end;
            if line_end > start && bytes[line_end - 1] == b'\r' {
                line_end -= 1;
            }

            // Count the leading space run. A tab anywhere in it makes column
            // arithmetic ambiguous, so hand the line back to the slow path.
            let mut ws = start;
            while ws < line_end && bytes[ws] == b' ' {
                ws += 1;
            }
            if ws < line_end && bytes[ws] == b'\t' {
                return start;
            }
            let indent = ws - start;
            let blank = ws == line_end;

            // A blank line stays inside the fence but carries no indent to strip.
            if !blank && indent < strip {
                return start;
            }

            let content_start = if blank { line_end } else { start + strip };

            // A closing fence is measured after the container indent is stripped.
            if !blank && is_closing_fence(&bytes[content_start..line_end], fence_char, fence_len) {
                self.close_top_block();
                return end + 1;
            }

            let content = &mut self.open[tip].content;
            content.push_str(&input[content_start..line_end]);
            content.push('\n');
            start = end + 1;
        }
        len
    }

    /// Bulk-scan a fenced code block nested inside blockquotes.
    ///
    /// Each line must carry `depth` blockquote markers (`>` with ≤3 leading
    /// spaces each and one optional space after); `fence_indent` further columns
    /// are then stripped from the content. Bails out to the per-line path on any
    /// line that does not match, so lazy continuation and quote termination
    /// behave exactly as before.
    fn bulk_scan_quoted_fenced_code(
        &mut self,
        bytes: &[u8],
        mut start: usize,
        fence_char: u8,
        fence_len: usize,
        depth: usize,
        fence_indent: usize,
    ) -> usize {
        let input = self.input;
        let len = bytes.len();
        let tip = self.open.len() - 1;
        while start < len {
            let end = memchr_newline(bytes, start);
            let mut line_end = end;
            if line_end > start && bytes[line_end - 1] == b'\r' {
                line_end -= 1;
            }

            // Walk the `>` markers. Any tab makes column arithmetic ambiguous.
            let mut i = start;
            let mut ok = true;
            for _ in 0..depth {
                let ws = i;
                while i < line_end && bytes[i] == b' ' {
                    i += 1;
                }
                if i - ws > 3 || i >= line_end || bytes[i] != b'>' {
                    ok = false;
                    break;
                }
                i += 1;
                if i < line_end && bytes[i] == b' ' {
                    i += 1;
                } else if i < line_end && bytes[i] == b'\t' {
                    ok = false;
                    break;
                }
            }
            if !ok {
                return start;
            }

            // Strip the fence's own indent, but never past the end of the line.
            let mut content_start = i;
            let indent_limit = (content_start + fence_indent).min(line_end);
            while content_start < indent_limit && bytes[content_start] == b' ' {
                content_start += 1;
            }
            if content_start < line_end && bytes[content_start] == b'\t' {
                return start;
            }

            if is_closing_fence(&bytes[content_start..line_end], fence_char, fence_len) {
                self.close_top_block();
                return end + 1;
            }

            let content = &mut self.open[tip].content;
            content.push_str(&input[content_start..line_end]);
            content.push('\n');
            start = end + 1;
        }
        len
    }

    #[inline]
    fn push_bulk_content(&mut self, input: &str, start: usize, end: usize, has_cr: bool) {
        let content = &mut self.open[1].content;
        if !has_cr {
            content.push_str(&input[start..end]);
        } else {
            let s = &input[start..end];
            content.reserve(s.len());
            for chunk in s.split('\r') {
                content.push_str(chunk);
            }
        }
    }

    fn mark_blank_on_list_items(&mut self) {
        if let Some(idx) = self.last_list_item_idx {
            if self.open_blockquotes == 0 {
                self.open[idx].had_blank_in_item = true;
                return;
            }
            // A blockquote exists somewhere; check if one sits between the list
            // item and the tip (if so, the blank belongs to the blockquote, not
            // the list item).
            let len = self.open.len();
            for i in (idx + 1)..len {
                if matches!(self.open[i].block_type, OpenBlockType::BlockQuote) {
                    return;
                }
            }
            self.open[idx].had_blank_in_item = true;
        }
    }

    #[inline]
    fn close_top_block(&mut self) {
        let block = self.open.pop().unwrap();
        match &block.block_type {
            OpenBlockType::BlockQuote => {
                self.open_blockquotes -= 1;
            }
            OpenBlockType::ListItem { content_col, .. } => {
                self.list_indent_sum -= content_col;
                // The item we just popped was last_list_item_idx; the next one
                // must be strictly below that index, so search only that prefix.
                self.last_list_item_idx = match self.last_list_item_idx {
                    Some(idx) if idx > 0 => self.open[..idx]
                        .iter()
                        .rposition(|b| matches!(b.block_type, OpenBlockType::ListItem { .. })),
                    _ => None,
                };
            }
            _ => {}
        }
        let finalized = self.finalize_block(block);
        if let Some(block) = finalized {
            let parent = self.open.last_mut().unwrap();
            // `children` starts empty, so a bare push would allocate capacity 1
            // and then double. Seed it once at a size that covers most blocks.
            if parent.children.capacity() == 0 {
                parent.children.reserve(4);
            }
            parent.children.push(block);
        }
    }
}
