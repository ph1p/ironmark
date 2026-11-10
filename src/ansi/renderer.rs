use crate::ParseOptions;
use crate::ast::{Block, ListKind};
use crate::inline::{InlineBuffers, LinkRefMap};

use super::AnsiOptions;
use super::constants::*;
use super::inline::{parse_inline_ansi, parse_inline_ansi_heading};
use super::wrap::{expand_tabs, visible_len, wrap_ansi};

pub(super) struct AnsiRenderer<'a> {
    pub(super) refs: &'a LinkRefMap,
    pub(super) opts: &'a ParseOptions,
    pub(super) aopts: &'a AnsiOptions,
    pub(super) bufs: &'a mut InlineBuffers,
    pub(super) out: &'a mut String,
    /// Current list nesting depth (0 = top level).
    pub(super) list_depth: usize,
    /// Ordered list counters, one per nesting level.
    pub(super) list_counters: Vec<u32>,
    /// Whether the previous block was a heading (used to suppress inter-heading blank lines).
    pub(super) prev_was_heading: bool,
}

impl<'a> AnsiRenderer<'a> {
    #[inline]
    pub(super) fn color(&self) -> bool {
        self.aopts.color
    }

    #[inline]
    pub(super) fn width(&self) -> usize {
        self.aopts.width
    }

    /// Push an ANSI code only when colour is enabled.
    #[inline]
    pub(super) fn push_ansi(&mut self, code: &str) {
        if self.color() {
            self.out.push_str(code);
        }
    }

    pub(super) fn render_block(&mut self, block: &Block) {
        match block {
            Block::Document { children } => {
                for child in children {
                    self.render_block(child);
                }
            }

            Block::Heading { level, raw } => {
                // Blank line before heading — but not at the very start of the document
                // and not when immediately following another heading (no double-gap).
                if !self.out.is_empty() && !self.prev_was_heading {
                    self.out.push('\n');
                }

                let fg: &str = match level {
                    1 => FG_H1,
                    2 => FG_H2,
                    3 => FG_H3,
                    4 => FG_H4,
                    5 => FG_H5,
                    _ => FG_H6,
                };

                // In heading context closing inline tags restore BOLD + heading colour
                // so inline formatting never permanently overrides it.
                let mut inline = String::new();
                if self.color() {
                    parse_inline_ansi_heading(
                        &mut inline,
                        raw,
                        self.refs,
                        self.opts,
                        self.aopts,
                        self.bufs,
                        fg,
                    );
                } else {
                    parse_inline_ansi(
                        &mut inline,
                        raw,
                        self.refs,
                        self.opts,
                        self.aopts,
                        self.bufs,
                    );
                }
                let vis = visible_len(&inline);

                if self.color() {
                    self.out.push_str(BOLD);
                    self.out.push_str(fg);
                    self.out.push_str(&inline);
                    self.out.push_str(RESET);
                } else {
                    self.out.push_str(&inline);
                }
                self.out.push('\n');

                // h1/h2: underline exactly the width of the heading text
                match level {
                    1 => {
                        let text_len = vis.max(4);
                        self.push_ansi(FG_H1);
                        for _ in 0..text_len {
                            self.out.push('═');
                        }
                        self.push_ansi(RESET);
                        self.out.push('\n');
                    }
                    2 => {
                        let text_len = vis.max(4);
                        self.push_ansi(FG_H2);
                        for _ in 0..text_len {
                            self.out.push('─');
                        }
                        self.push_ansi(RESET);
                        self.out.push('\n');
                    }
                    _ => {
                        // h3–h6: blank line separates from content
                        self.out.push('\n');
                    }
                }

                self.prev_was_heading = true;
                return; // skip the prev_was_heading = false at end
            }

            Block::Paragraph { raw } => {
                let indent = "  ".repeat(self.list_depth);
                let mut inline = String::new();
                parse_inline_ansi(
                    &mut inline,
                    raw,
                    self.refs,
                    self.opts,
                    self.aopts,
                    self.bufs,
                );
                // Wrap each hard-break segment independently
                let wrap_width = self.width().saturating_sub(indent.len());
                let mut first_line = true;
                for part in inline.split('\n') {
                    if !first_line {
                        self.out.push('\n');
                    }
                    first_line = false;
                    self.out.push_str(&indent);
                    self.out.push_str(&wrap_ansi(part, wrap_width, &indent));
                }
                self.out.push('\n');
                if self.list_depth == 0 {
                    self.out.push('\n');
                }
            }

            Block::CodeBlock { info, literal } => {
                let lang = match memchr::memchr3(b' ', b'\t', b'\n', info.as_bytes()) {
                    Some(0) | None => info.as_str(),
                    Some(pos) => &info[..pos],
                };

                let lines: Vec<&str> = literal.lines().collect();
                let total_lines = lines.len();
                let num_width = if self.aopts.line_numbers && total_lines > 0 {
                    format!("{}", total_lines).len()
                } else {
                    0
                };

                // Bracket style: short top stub with optional lang label, left bar,
                // short bottom stub. No right border, no full-width background fill.
                //
                // ┌ rust ──
                // │ fn main() {}
                // └────────
                const STUB: usize = 4;

                if self.color() {
                    self.out.push_str(FG_BORDER);
                    self.out.push('┌');
                    if !lang.is_empty() {
                        self.out.push_str(FG_LANG);
                        self.out.push(' ');
                        self.out.push_str(lang);
                        self.out.push(' ');
                        self.out.push_str(FG_BORDER);
                        for _ in 0..STUB {
                            self.out.push('─');
                        }
                    } else {
                        for _ in 0..STUB + 1 {
                            self.out.push('─');
                        }
                    }
                    self.out.push_str(RESET);
                    self.out.push('\n');
                } else if !lang.is_empty() {
                    self.out.push_str(lang);
                    self.out.push('\n');
                }

                for (idx, line) in lines.iter().enumerate() {
                    let expanded = expand_tabs(line, 4);
                    if self.color() {
                        self.out.push_str(FG_BORDER);
                        self.out.push('│');
                        self.out.push_str(RESET);
                        if num_width > 0 {
                            self.out.push_str(FG_BORDER);
                            self.out.push_str(&format!(" {:>num_width$} ", idx + 1));
                            self.out.push_str(RESET);
                        }
                        self.out.push_str(FG_CODE);
                        self.out.push(' ');
                        self.out.push_str(&expanded);
                        self.out.push_str(RESET);
                    } else {
                        self.out.push('|');
                        if num_width > 0 {
                            self.out.push_str(&format!(" {:>num_width$} ", idx + 1));
                        }
                        self.out.push(' ');
                        self.out.push_str(&expanded);
                    }
                    self.out.push('\n');
                }

                if self.color() {
                    self.out.push_str(FG_BORDER);
                    self.out.push('└');
                    let stub_len = if !lang.is_empty() {
                        // Match top stub width: 1 (space) + lang + 1 (space) + STUB dashes
                        1 + lang.len() + 1 + STUB
                    } else {
                        STUB + 1
                    };
                    for _ in 0..stub_len {
                        self.out.push('─');
                    }
                    self.out.push_str(RESET);
                    self.out.push('\n');
                }
                self.out.push('\n');
            }

            Block::BlockQuote { children } => {
                let mut inner = String::new();
                let mut inner_renderer = AnsiRenderer {
                    refs: self.refs,
                    opts: self.opts,
                    aopts: self.aopts,
                    bufs: self.bufs,
                    out: &mut inner,
                    list_depth: self.list_depth,
                    list_counters: self.list_counters.clone(),
                    prev_was_heading: false,
                };
                for child in children {
                    inner_renderer.render_block(child);
                }
                // Prefix every line with the bar; collapse consecutive blank lines.
                // Trim trailing newlines from inner so .lines() doesn't produce a
                // spurious blank bar line at the end.
                let inner = inner.trim_end_matches('\n');
                let mut prev_blank = false;
                for line in inner.lines() {
                    let is_blank = visible_len(line) == 0;
                    if is_blank {
                        if !prev_blank {
                            if self.color() {
                                self.out.push_str("  ");
                                self.out.push_str(FG_QUOTE_BAR);
                                self.out.push('▌');
                                self.out.push_str(RESET);
                            } else {
                                self.out.push_str("  |");
                            }
                            self.out.push('\n');
                        }
                        prev_blank = true;
                        continue;
                    }
                    prev_blank = false;
                    if self.color() {
                        // Re-inject FG_DIM_TEXT after every RESET so inline code
                        // spans don't permanently steal the blockquote text colour.
                        let recoloured = if line.contains(RESET) {
                            std::borrow::Cow::Owned(line.replace(RESET, RESET_DIM))
                        } else {
                            std::borrow::Cow::Borrowed(line)
                        };
                        self.out.push_str("  ");
                        self.out.push_str(FG_QUOTE_BAR);
                        self.out.push_str("▌ ");
                        self.out.push_str(RESET);
                        self.out.push_str(FG_DIM_TEXT);
                        self.out.push_str(&recoloured);
                        self.out.push_str(RESET);
                    } else {
                        self.out.push_str("  | ");
                        self.out.push_str(line);
                    }
                    self.out.push('\n');
                }
                self.out.push('\n');
            }

            Block::ThematicBreak => {
                let len = if self.width() > 0 { self.width() } else { 40 };
                if self.color() {
                    self.out.push_str(FG_RULE);
                }
                // Ornamental break with centred star
                if len >= 5 {
                    let side = (len - 3) / 2;
                    let rest = len - 3 - side;
                    for _ in 0..side {
                        self.out.push('─');
                    }
                    self.out.push_str(" ✦ ");
                    for _ in 0..rest {
                        self.out.push('─');
                    }
                } else {
                    for _ in 0..len {
                        self.out.push('─');
                    }
                }
                self.push_ansi(RESET);
                self.out.push('\n');
                self.out.push('\n');
            }

            Block::List {
                kind,
                start,
                children,
                ..
            } => {
                let prev_counters = self.list_counters.clone();
                if matches!(kind, ListKind::Ordered(_)) {
                    self.list_counters.push(*start);
                }
                self.list_depth += 1;
                for child in children {
                    self.render_list_item(child, kind);
                }
                self.list_depth -= 1;
                self.list_counters = prev_counters;
                if self.list_depth == 0 {
                    self.out.push('\n');
                }
            }

            Block::ListItem { .. } => {
                self.render_list_item(block, &ListKind::Bullet(b'-'));
            }

            Block::HtmlBlock { .. } | Block::MdxFlow { .. } | Block::MdxEsm { .. } => {}

            Block::Table(table) => {
                self.render_table(table);
            }
        }
        self.prev_was_heading = false;
    }

    pub(super) fn render_list_item(&mut self, block: &Block, kind: &ListKind) {
        let indent = "  ".repeat(self.list_depth - 1);
        // Bullet symbols: • / ○ / ◦ — filled → unfilled → small-unfilled hierarchy
        let bullet: String = match kind {
            ListKind::Bullet(_) => {
                let sym = match (self.list_depth - 1) % 3 {
                    0 => "•",
                    1 => "○",
                    _ => "◦",
                };
                format!("{indent}{sym} ")
            }
            ListKind::Ordered(_) => {
                let n = self
                    .list_counters
                    .last_mut()
                    .map(|c| {
                        let v = *c;
                        *c += 1;
                        v
                    })
                    .unwrap_or(1);
                format!("{indent}{n}. ")
            }
        };

        if let Block::ListItem { children, checked } = block {
            if self.color() {
                self.out.push_str(BOLD);
                self.out.push_str(FG_BULLET);
            }
            self.out.push_str(&bullet);
            if self.color() {
                self.out.push_str(RESET);
            }

            // Task list checkbox — real Unicode glyphs, 1 column each
            if let Some(is_checked) = checked {
                if *is_checked {
                    if self.color() {
                        self.out.push_str(FG_CHECKED);
                    }
                    self.out.push_str("✓ ");
                } else {
                    if self.color() {
                        self.out.push_str(FG_UNCHECKED);
                    }
                    self.out.push_str("☐ ");
                }
                if self.color() {
                    self.out.push_str(RESET);
                }
            }

            let bullet_vis = visible_len(&bullet);
            let check_extra = if checked.is_some() { 2 } else { 0 }; // "✓ " or "☐ "
            let cont_indent = " ".repeat(indent.len() + bullet_vis + check_extra);
            let mut first = true;
            for child in children {
                if first {
                    first = false;
                    if let Block::Paragraph { raw } = child {
                        let mut inline = String::new();
                        parse_inline_ansi(
                            &mut inline,
                            raw,
                            self.refs,
                            self.opts,
                            self.aopts,
                            self.bufs,
                        );
                        let used = indent.len() + bullet_vis + check_extra;
                        let wrap_width = self.width().saturating_sub(used);
                        let wrapped = wrap_ansi(&inline, wrap_width, &cont_indent);
                        self.out.push_str(&wrapped);
                        self.out.push('\n');
                        continue;
                    }
                }
                let mut item_out = String::new();
                let mut sub = AnsiRenderer {
                    refs: self.refs,
                    opts: self.opts,
                    aopts: self.aopts,
                    bufs: self.bufs,
                    out: &mut item_out,
                    list_depth: self.list_depth,
                    list_counters: self.list_counters.clone(),
                    prev_was_heading: false,
                };
                sub.render_block(child);
                for line in item_out.lines() {
                    self.out.push_str(&cont_indent);
                    self.out.push_str(line);
                    self.out.push('\n');
                }
            }
        }
    }
}
