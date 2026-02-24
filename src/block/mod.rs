mod html_block;
mod leaf_blocks;
mod link_ref_def;
mod parser;

use html_block::*;
use leaf_blocks::*;
use link_ref_def::*;

use crate::ParseOptions;
use crate::ast::{Block, ListKind, TableAlignment};
use crate::entities;
use crate::html::trim_cr;
use crate::inline::{InlineBuffers, LinkRefMap};
use crate::render::render_block;
use std::borrow::Cow;

pub fn parse(markdown: &str, options: &ParseOptions) -> String {
    let mut parser = BlockParser::new(markdown, options.enable_tables, options.enable_task_lists);
    let doc = parser.parse();
    let refs = parser.ref_defs;
    let mut out = String::with_capacity(markdown.len() + markdown.len() / 4);
    let mut bufs = InlineBuffers::new();
    render_block(&doc, &refs, &mut out, options, &mut bufs);
    out
}

// ── Line representation ──────────────────────────────────────────────

#[derive(Clone, Debug)]
struct Line<'a> {
    raw: &'a str,
    col_offset: usize,
    byte_offset: usize,
    partial_spaces: usize,
}

impl<'a> Line<'a> {
    fn new(raw: &'a str) -> Self {
        Self {
            raw,
            col_offset: 0,
            byte_offset: 0,
            partial_spaces: 0,
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
    fn is_blank(&self) -> bool {
        if self.partial_spaces > 0 {
            return false;
        }
        let bytes = self.raw.as_bytes();
        let mut off = self.byte_offset;
        while off < bytes.len() {
            match bytes[off] {
                b' ' | b'\t' => off += 1,
                _ => return false,
            }
        }
        true
    }

    /// Advance past spaces/tabs, returning the number of columns consumed (up to `max`).
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

    /// Advance past exactly one partial tab or spaces, consuming `n` columns.
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
    fn peek_nonspace_col(&self) -> (usize, usize, u8) {
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
                b => return (col, off, b),
            }
        }
        (col, off, 0)
    }

    fn indent(&self) -> usize {
        let (col, _, _) = self.peek_nonspace_col();
        col - self.col_offset
    }

    fn first_nonspace_byte(&self) -> u8 {
        let (_, _, b) = self.peek_nonspace_col();
        b
    }

    fn advance_to_nonspace(&mut self) {
        self.partial_spaces = 0; // partial spaces are whitespace, consume them
        let (col, off, _) = self.peek_nonspace_col();
        self.col_offset = col;
        self.byte_offset = off;
    }

    #[inline]
    fn rest_of_line(&self) -> &'a str {
        let (_, off, _) = self.peek_nonspace_col();
        if off >= self.raw.len() {
            ""
        } else {
            &self.raw[off..]
        }
    }

    /// Get the remaining content as a string, with partial tab spaces expanded.
    fn remainder_with_partial(&self) -> Cow<'a, str> {
        if self.partial_spaces > 0 {
            let rem = self.remainder();
            let mut s = String::with_capacity(self.partial_spaces + rem.len());
            for _ in 0..self.partial_spaces {
                s.push(' ');
            }
            s.push_str(rem);
            Cow::Owned(s)
        } else {
            Cow::Borrowed(self.remainder())
        }
    }
}

// ── Open block types ─────────────────────────────────────────────────

#[derive(Clone, Debug)]
enum OpenBlockType {
    Document,
    BlockQuote,
    ListItem {
        /// Column position where content starts (after marker + spaces)
        content_col: usize,
        /// True if the item started with a blank line after the marker
        started_blank: bool,
    },
    FencedCode {
        fence_char: u8,
        fence_len: usize,
        fence_indent: usize,
        info: String,
    },
    IndentedCode,
    HtmlBlock {
        end_condition: HtmlBlockEnd,
    },
    Paragraph,
    Table {
        alignments: Vec<TableAlignment>,
        header: Vec<String>,
        rows: Vec<Vec<String>>,
    },
}

#[derive(Copy, Clone, Debug, PartialEq)]
enum HtmlBlockEnd {
    /// Type 1: ends at </pre>, </script>, </style>, </textarea>
    EndTag(&'static str),
    /// Type 2: ends at -->
    Comment,
    /// Type 3: ends at ?>
    ProcessingInstruction,
    /// Type 4: ends at >
    Declaration,
    /// Type 5: ends at ]]>
    Cdata,
    /// Types 6,7: ends at blank line
    BlankLine,
}

#[derive(Clone, Debug)]
struct OpenBlock {
    block_type: OpenBlockType,
    content: String,
    children: Vec<Block>,
    had_blank_in_item: bool,
    list_has_blank_between: bool,
    checked: Option<bool>,
    list_start: u32,
    list_kind: Option<ListKind>,
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
            checked: None,
            list_start: 0,
            list_kind: None,
        }
    }
}

// ── Block parser ─────────────────────────────────────────────────────

pub(crate) struct BlockParser<'a> {
    input: &'a str,
    pub(crate) ref_defs: LinkRefMap,
    /// Stack of open blocks. Index 0 is the Document.
    open: Vec<OpenBlock>,
    enable_tables: bool,
    enable_task_lists: bool,
}

impl<'a> BlockParser<'a> {
    pub fn new(input: &'a str, enable_tables: bool, enable_task_lists: bool) -> Self {
        let doc = OpenBlock::new(OpenBlockType::Document);
        let mut open = Vec::with_capacity(8);
        open.push(doc);
        Self {
            input,
            ref_defs: LinkRefMap::default(),
            open,
            enable_tables,
            enable_task_lists,
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

    fn has_open_leaf_after(&self, idx: usize) -> bool {
        for i in (idx + 1)..self.open.len() {
            match &self.open[i].block_type {
                OpenBlockType::Paragraph
                | OpenBlockType::FencedCode { .. }
                | OpenBlockType::IndentedCode
                | OpenBlockType::HtmlBlock { .. } => return true,
                _ => {}
            }
        }
        false
    }

    fn mark_blank_on_list_items(&mut self) {
        // Find the innermost list item, but only mark it if there's no
        // container block (blockquote) between it and the blank line.
        // A blank inside a nested blockquote should not make the list item loose.
        let len = self.open.len();
        for i in (1..len).rev() {
            match &self.open[i].block_type {
                OpenBlockType::ListItem { .. } => {
                    self.open[i].had_blank_in_item = true;
                    break;
                }
                OpenBlockType::BlockQuote => {
                    // The blank is inside a blockquote, not at the list item level.
                    // Don't mark the enclosing list item.
                    break;
                }
                _ => {}
            }
        }
    }

    #[inline]
    fn close_top_block(&mut self) {
        let block = self.open.pop().unwrap();
        let finalized = self.finalize_block(block);
        if let Some(block) = finalized {
            let parent = self.open.last_mut().unwrap();
            parent.children.push(block);
        }
    }
}
