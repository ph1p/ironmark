mod slug;

pub use slug::benchmark_heading_slug;
pub(crate) use slug::heading_slug_into;

use crate::ParseOptions;
use crate::ast::{Block, ListKind, TableAlignment};
use crate::html::{
    collapse_and_escape_into, encode_url_escaped_into, escape_html_into, gfm_tag_is_filtered,
};
use crate::inline::{InlineBuffers, LinkRefMap, parse_inline_pass};

#[inline(always)]
fn render_code_block<'a>(
    out: &mut String,
    info: &str,
    literal: &'a str,
    ctx: RenderCtx<'a>,
    src_range_idx: &mut usize,
) {
    if info.is_empty() {
        out.push_str("<pre><code>");
    } else {
        // Scalar scan beats memchr3 for short info strings (typical: "rust", "js").
        let info_bytes = info.as_bytes();
        let lang_end = info_bytes
            .iter()
            .position(|&b| b == b' ' || b == b'\t' || b == b'\n')
            .unwrap_or(info_bytes.len());
        if lang_end == 0 {
            out.push_str("<pre><code>");
        } else {
            let lang = &info[..lang_end];
            out.push_str("<pre><code class=\"language-");
            escape_html_into(out, lang);
            out.push_str("\">");
        }
    }
    let literal = ctx.resolve_raw(literal, src_range_idx);
    escape_html_into(out, literal);
    out.push_str("</code></pre>\n");
}

#[inline(always)]
fn emit_checkbox(out: &mut String, checked: Option<bool>) {
    match checked {
        Some(true) => out.push_str("<input type=\"checkbox\" checked=\"\" disabled=\"\" /> "),
        Some(false) => out.push_str("<input type=\"checkbox\" disabled=\"\" /> "),
        None => {}
    }
}

enum Work<'a> {
    Block(&'a Block),
    TightListItem(&'a Block),
    TightBlock(&'a Block),
    CloseTag(&'static str),
}

#[derive(Copy, Clone)]
struct RenderCtx<'a> {
    refs: &'a LinkRefMap,
    opts: &'a ParseOptions,
    source: &'a str,
    src_ranges: &'a [(u32, u32)],
}

impl<'a> RenderCtx<'a> {
    /// Resolve a deferred zero-copy `raw` field (paragraphs/headings parsed in
    /// render mode store their text as a source range instead of a String).
    /// An empty `raw` means "consume the next range"; non-empty is used as-is.
    #[inline(always)]
    fn resolve_raw(&self, raw: &'a str, idx: &mut usize) -> &'a str {
        if raw.is_empty()
            && let Some(&(s, e)) = self.src_ranges.get(*idx)
        {
            *idx += 1;
            return &self.source[s as usize..e as usize];
        }
        raw
    }
}

/// `<p>…</p>` for a (possibly deferred) paragraph — the ONE consumer chokepoint
/// for loose paragraphs.
#[inline(always)]
fn render_paragraph<'a>(
    out: &mut String,
    raw: &'a str,
    ctx: RenderCtx<'a>,
    bufs: &mut InlineBuffers,
    src_range_idx: &mut usize,
) {
    let raw = ctx.resolve_raw(raw, src_range_idx);
    out.push_str("<p>");
    parse_inline_pass(out, raw, ctx.refs, ctx.opts, bufs);
    out.push_str("</p>\n");
}

/// Tight-list paragraph (no `<p>` wrapper) — the ONE consumer chokepoint for
/// tight paragraphs.
#[inline(always)]
fn render_tight_paragraph<'a>(
    out: &mut String,
    raw: &'a str,
    ctx: RenderCtx<'a>,
    bufs: &mut InlineBuffers,
    src_range_idx: &mut usize,
) {
    let raw = ctx.resolve_raw(raw, src_range_idx);
    push_inline_or_plain(out, raw, ctx.refs, ctx.opts, bufs);
}

/// `<hN>…</hN>` with optional slug id/anchor — the ONE consumer chokepoint for
/// headings.
#[inline]
fn render_heading<'a>(
    out: &mut String,
    level: u8,
    raw: &'a str,
    ctx: RenderCtx<'a>,
    bufs: &mut InlineBuffers,
    src_range_idx: &mut usize,
) {
    let raw = ctx.resolve_raw(raw, src_range_idx);
    static TAGS: [&str; 7] = ["", "h1", "h2", "h3", "h4", "h5", "h6"];
    let tag = TAGS[level as usize];
    out.push('<');
    out.push_str(tag);
    let mut slug = std::mem::take(&mut bufs.scratch);
    let use_slug = ctx.opts.enable_heading_ids || ctx.opts.enable_heading_anchors;
    if use_slug {
        heading_slug_into(&mut slug, raw);
        if !slug.is_empty() {
            out.push_str(" id=\"");
            escape_html_into(out, &slug);
            out.push('"');
        }
    }
    out.push('>');
    parse_inline_pass(out, raw, ctx.refs, ctx.opts, bufs);
    if ctx.opts.enable_heading_anchors && !slug.is_empty() {
        out.push_str(" <a class=\"anchor\" href=\"#");
        encode_url_escaped_into(out, &slug);
        out.push_str("\">¶</a>");
    }
    out.push_str("</");
    out.push_str(tag);
    out.push_str(">\n");
    bufs.scratch = slug;
}

pub(crate) fn render_block(
    block: &Block,
    refs: &LinkRefMap,
    out: &mut String,
    opts: &ParseOptions,
    bufs: &mut InlineBuffers,
    source: &str,
    src_ranges: &[(u32, u32)],
) {
    let ctx = RenderCtx {
        refs,
        opts,
        source,
        src_ranges,
    };
    let mut src_range_idx: usize = 0;

    if let Block::Document { children } = block {
        match children.as_slice() {
            [child] => {
                render_single_child_doc(child, ctx, out, bufs, &mut src_range_idx);
                return;
            }
            children => {
                // Fast path: iterate document children directly, using a local stack only
                // for container blocks (blockquote, list, etc.). Leaf-only documents (common
                // case) avoid allocating the Work stack entirely.
                render_document_children(children, ctx, out, bufs, &mut src_range_idx);
                return;
            }
        }
    }

    let mut stack: Vec<Work<'_>> = Vec::with_capacity(32);
    stack.push(Work::Block(block));

    while let Some(work) = stack.pop() {
        match work {
            Work::CloseTag(tag) => out.push_str(tag),
            Work::TightListItem(block) => {
                render_tight_list_item(block, ctx, out, bufs, &mut stack, &mut src_range_idx);
            }
            Work::TightBlock(block) => {
                if let Block::Paragraph { raw } = block {
                    render_tight_paragraph(out, raw, ctx, bufs, &mut src_range_idx);
                } else {
                    render_one(block, ctx, out, bufs, &mut stack, &mut src_range_idx);
                }
            }
            Work::Block(block) => {
                render_one(block, ctx, out, bufs, &mut stack, &mut src_range_idx);
            }
        }
    }
}

/// Render all direct children of a Document block without allocating a Work stack.
/// Handles leaf blocks inline and container blocks via a local stack. In practice,
/// most documents contain only leaf blocks at the top level (paragraphs, headings,
/// code blocks, HR), so the common case avoids the `Vec<Work>` allocation entirely.
#[inline(never)]
fn render_document_children<'a>(
    children: &'a [Block],
    ctx: RenderCtx<'a>,
    out: &mut String,
    bufs: &mut InlineBuffers,
    src_range_idx: &mut usize,
) {
    for (idx, child) in children.iter().enumerate() {
        match child {
            Block::ThematicBreak => out.push_str("<hr />\n"),
            Block::Paragraph { raw } => {
                render_paragraph(out, raw, ctx, bufs, src_range_idx);
            }
            Block::Heading { level, raw } => {
                render_heading(out, *level, raw, ctx, bufs, src_range_idx);
            }
            Block::CodeBlock { info, literal } => {
                render_code_block(out, info, literal, ctx, src_range_idx);
            }
            Block::HtmlBlock { literal } => {
                let escape_it = ctx.opts.disable_raw_html
                    || ctx.opts.no_html_blocks
                    || (ctx.opts.tag_filter && gfm_tag_is_filtered(literal));
                if escape_it {
                    escape_html_into(out, literal);
                } else {
                    out.push_str(literal);
                }
                if !literal.ends_with('\n') {
                    out.push('\n');
                }
            }
            Block::Table(td) => {
                // Tables at document level: call render_one with a local stack (tables
                // never push extra Work items, so the stack stays empty after the call).
                let mut dummy_stack: Vec<Work<'_>> = Vec::new();
                render_one(child, ctx, out, bufs, &mut dummy_stack, src_range_idx);
                let _ = td;
            }
            // Container blocks (BlockQuote, List, ListItem): use a local stack for this
            // child and any remaining children. This is the uncommon case for top-level docs.
            _ => {
                let mut stack: Vec<Work<'a>> = Vec::with_capacity(8);
                // Push remaining children (including this one) in reverse so we pop in order.
                for remaining_child in children[idx..].iter().rev() {
                    stack.push(Work::Block(remaining_child));
                }
                while let Some(work) = stack.pop() {
                    match work {
                        Work::CloseTag(tag) => out.push_str(tag),
                        Work::TightListItem(b) => {
                            render_tight_list_item(b, ctx, out, bufs, &mut stack, src_range_idx);
                        }
                        Work::TightBlock(b) => {
                            if let Block::Paragraph { raw } = b {
                                render_tight_paragraph(out, raw, ctx, bufs, src_range_idx);
                            } else {
                                render_one(b, ctx, out, bufs, &mut stack, src_range_idx);
                            }
                        }
                        Work::Block(b) => render_one(b, ctx, out, bufs, &mut stack, src_range_idx),
                    }
                }
                return;
            }
        }
    }
}

#[inline]
fn render_single_child_doc(
    child: &Block,
    ctx: RenderCtx<'_>,
    out: &mut String,
    bufs: &mut InlineBuffers,
    src_range_idx: &mut usize,
) {
    match child {
        Block::Paragraph { raw } => {
            render_paragraph(out, raw, ctx, bufs, src_range_idx);
        }
        _ => {
            let mut stack = Vec::with_capacity(8);
            stack.push(Work::Block(child));
            while let Some(work) = stack.pop() {
                match work {
                    Work::CloseTag(tag) => out.push_str(tag),
                    Work::TightListItem(block) => {
                        render_tight_list_item(block, ctx, out, bufs, &mut stack, src_range_idx);
                    }
                    Work::TightBlock(block) => {
                        if let Block::Paragraph { raw } = block {
                            render_tight_paragraph(out, raw, ctx, bufs, src_range_idx);
                        } else {
                            render_one(block, ctx, out, bufs, &mut stack, src_range_idx);
                        }
                    }
                    Work::Block(block) => {
                        render_one(block, ctx, out, bufs, &mut stack, src_range_idx)
                    }
                }
            }
        }
    }
}

fn list_close_tag(kind: &ListKind) -> &'static str {
    match kind {
        ListKind::Bullet(_) => "</ul>\n",
        ListKind::Ordered(_) => "</ol>\n",
    }
}

#[inline(always)]
fn emit_list_open(out: &mut String, kind: &ListKind, start: u32) {
    match kind {
        ListKind::Bullet(_) => out.push_str("<ul>\n"),
        ListKind::Ordered(_) => {
            if start == 1 {
                out.push_str("<ol>\n");
            } else {
                use std::fmt::Write;
                out.push_str("<ol start=\"");
                let _ = write!(out, "{}", start);
                out.push_str("\">\n");
            }
        }
    }
}

#[inline]
fn render_one<'a>(
    block: &'a Block,
    ctx: RenderCtx<'a>,
    out: &mut String,
    bufs: &mut InlineBuffers,
    stack: &mut Vec<Work<'a>>,
    src_range_idx: &mut usize,
) {
    match block {
        Block::Document { children } => {
            for child in children.iter().rev() {
                stack.push(Work::Block(child));
            }
        }
        Block::ThematicBreak => out.push_str("<hr />\n"),
        Block::Heading { level, raw } => {
            render_heading(out, *level, raw, ctx, bufs, src_range_idx);
        }
        Block::Paragraph { raw } => {
            render_paragraph(out, raw, ctx, bufs, src_range_idx);
        }
        Block::CodeBlock { info, literal } => {
            render_code_block(out, info, literal, ctx, src_range_idx);
        }
        Block::HtmlBlock { literal } => {
            let escape_it = ctx.opts.disable_raw_html
                || ctx.opts.no_html_blocks
                || (ctx.opts.tag_filter && gfm_tag_is_filtered(literal));
            if escape_it {
                escape_html_into(out, literal);
            } else {
                out.push_str(literal);
            }
            if !literal.ends_with('\n') {
                out.push('\n');
            }
        }
        Block::BlockQuote { children } => {
            out.push_str("<blockquote>\n");
            stack.push(Work::CloseTag("</blockquote>\n"));
            for child in children.iter().rev() {
                stack.push(Work::Block(child));
            }
        }
        Block::List {
            kind,
            start,
            tight,
            children,
        } => {
            if *tight && children.len() == 1 {
                render_nested_tight_list(
                    kind,
                    *start,
                    children,
                    ctx,
                    out,
                    bufs,
                    stack,
                    src_range_idx,
                );
                return;
            }
            emit_list_open(out, kind, *start);
            stack.push(Work::CloseTag(list_close_tag(kind)));
            if *tight {
                for item in children.iter().rev() {
                    stack.push(Work::TightListItem(item));
                }
            } else {
                for item in children.iter().rev() {
                    stack.push(Work::Block(item));
                }
            }
        }
        Block::ListItem { children, checked } => {
            out.push_str("<li>");
            emit_checkbox(out, *checked);
            if !children.is_empty() {
                out.push('\n');
                stack.push(Work::CloseTag("</li>\n"));
                for child in children.iter().rev() {
                    stack.push(Work::Block(child));
                }
            } else {
                out.push_str("</li>\n");
            }
        }
        Block::Table(td) => {
            let alignments = &td.alignments;
            let header = &td.header;
            let num_cols = td.num_cols;
            let all_none = alignments.iter().all(|a| *a == TableAlignment::None);
            out.push_str("<table>\n<thead>\n<tr>\n");
            for (i, cell) in header.iter().enumerate() {
                let align = if all_none {
                    TableAlignment::None
                } else {
                    alignments.get(i).copied().unwrap_or(TableAlignment::None)
                };
                render_table_cell(out, cell.as_str(), "th", align, ctx.refs, ctx.opts, bufs);
            }
            out.push_str("</tr>\n</thead>\n");
            let num_rows = td.rows.len().checked_div(num_cols).unwrap_or(0);
            if num_rows > 0 {
                out.push_str("<tbody>\n");
                if all_none {
                    for row in td.rows.chunks_exact(num_cols) {
                        out.push_str("<tr>\n");
                        for cell in row {
                            out.push_str("<td>");
                            push_inline_or_plain(out, cell.as_str(), ctx.refs, ctx.opts, bufs);
                            out.push_str("</td>\n");
                        }
                        out.push_str("</tr>\n");
                    }
                } else {
                    for row in td.rows.chunks_exact(num_cols) {
                        out.push_str("<tr>\n");
                        for (c, cell) in row.iter().enumerate() {
                            let align = alignments.get(c).copied().unwrap_or(TableAlignment::None);
                            render_table_cell(
                                out,
                                cell.as_str(),
                                "td",
                                align,
                                ctx.refs,
                                ctx.opts,
                                bufs,
                            );
                        }
                        out.push_str("</tr>\n");
                    }
                }
                out.push_str("</tbody>\n");
            }
            out.push_str("</table>\n");
        }
    }
}

/// Returns `true` if `s` needs no inline parsing or HTML escaping.
/// Uses the pre-built scan_table (same table as `parse_inline_pass`) so we
/// do a single O(n) pass instead of multiple memchr SIMD scans.
#[inline(always)]
fn is_trivially_plain(s: &str, scan_table: &[u8; 256]) -> bool {
    s.as_bytes()
        .iter()
        .all(|&b| scan_table[b as usize] == 0 && b >= b' ')
}

/// For trivially plain text, push directly; otherwise run the full inline pass.
#[inline(always)]
fn push_inline_or_plain(
    out: &mut String,
    raw: &str,
    refs: &LinkRefMap,
    opts: &ParseOptions,
    bufs: &mut InlineBuffers,
) {
    if is_trivially_plain(raw, &bufs.scan_table) {
        if opts.collapse_whitespace {
            collapse_and_escape_into(out, raw);
        } else {
            out.push_str(raw);
        }
    } else {
        parse_inline_pass(out, raw, refs, opts, bufs);
    }
}

#[inline(never)]
#[allow(clippy::too_many_arguments)]
fn render_nested_tight_list<'a>(
    kind: &ListKind,
    start: u32,
    children: &'a [Block],
    ctx: RenderCtx<'a>,
    out: &mut String,
    bufs: &mut InlineBuffers,
    stack: &mut Vec<Work<'a>>,
    src_range_idx: &mut usize,
) {
    const MAX_DEPTH: usize = 64;
    let mut close_tags: [&'static str; MAX_DEPTH] = [""; MAX_DEPTH];
    let mut depth: usize = 0;

    let mut cur_kind = kind;
    let mut cur_start = start;
    let mut cur_children: &'a [Block] = children;

    loop {
        let Block::ListItem {
            children: item_children,
            checked,
        } = &cur_children[0]
        else {
            emit_list_open(out, cur_kind, cur_start);
            stack.push(Work::CloseTag(list_close_tag(cur_kind)));
            stack.push(Work::Block(&cur_children[0]));
            break;
        };

        match cur_kind {
            ListKind::Bullet(_) => out.push_str("<ul>\n<li>"),
            ListKind::Ordered(_) => {
                emit_list_open(out, cur_kind, cur_start);
                out.push_str("<li>");
            }
        }
        emit_checkbox(out, *checked);

        if item_children.len() == 2
            && depth < MAX_DEPTH
            && let (
                Block::Paragraph { raw },
                Block::List {
                    kind: inner_kind,
                    start: inner_start,
                    tight: true,
                    children: inner_children,
                },
            ) = (&item_children[0], &item_children[1])
            && inner_children.len() == 1
        {
            render_tight_paragraph(out, raw, ctx, bufs, src_range_idx);
            out.push('\n');
            close_tags[depth] = list_close_tag(cur_kind);
            depth += 1;
            cur_kind = inner_kind;
            cur_start = *inner_start;
            cur_children = inner_children;
            continue;
        }

        if item_children.len() == 1
            && let Block::Paragraph { raw } = &item_children[0]
        {
            render_tight_paragraph(out, raw, ctx, bufs, src_range_idx);
            out.push_str("</li>\n");
            out.push_str(list_close_tag(cur_kind));
            let mut i = depth;
            while i > 0 {
                i -= 1;
                out.push_str("</li>\n");
                out.push_str(close_tags[i]);
            }
            return;
        }

        {
            let mut i = 0;
            while i < depth {
                stack.push(Work::CloseTag(close_tags[i]));
                stack.push(Work::CloseTag("</li>\n"));
                i += 1;
            }
        }
        stack.push(Work::CloseTag(list_close_tag(cur_kind)));
        stack.push(Work::CloseTag("</li>\n"));

        let mut prev_was_para = false;
        for (idx, child) in item_children.iter().enumerate() {
            match child {
                Block::Paragraph { raw } => {
                    render_tight_paragraph(out, raw, ctx, bufs, src_range_idx);
                    prev_was_para = true;
                }
                _ => {
                    if prev_was_para || idx == 0 {
                        out.push('\n');
                    }
                    for remaining in item_children[idx..].iter().rev() {
                        stack.push(Work::TightBlock(remaining));
                    }
                    return;
                }
            }
        }
        return;
    }

    out.reserve(depth * 12);
    let mut i = depth;
    while i > 0 {
        i -= 1;
        out.push_str("</li>\n");
        out.push_str(close_tags[i]);
    }
}

#[inline]
fn render_tight_list_item<'a>(
    block: &'a Block,
    ctx: RenderCtx<'a>,
    out: &mut String,
    bufs: &mut InlineBuffers,
    stack: &mut Vec<Work<'a>>,
    src_range_idx: &mut usize,
) {
    let Block::ListItem { children, checked } = block else {
        render_one(block, ctx, out, bufs, stack, src_range_idx);
        return;
    };

    out.push_str("<li>");
    emit_checkbox(out, *checked);

    if children.len() == 1
        && let Block::Paragraph { raw } = &children[0]
    {
        render_tight_paragraph(out, raw, ctx, bufs, src_range_idx);
        out.push_str("</li>\n");
        return;
    }

    stack.push(Work::CloseTag("</li>\n"));
    let mut prev_was_para = false;
    for (idx, child) in children.iter().enumerate() {
        match child {
            Block::Paragraph { raw } => {
                render_tight_paragraph(out, raw, ctx, bufs, src_range_idx);
                prev_was_para = true;
            }
            _ => {
                if prev_was_para || idx == 0 {
                    out.push('\n');
                }
                for remaining in children[idx..].iter().rev() {
                    stack.push(Work::TightBlock(remaining));
                }
                return;
            }
        }
    }
}

#[inline]
fn render_table_cell(
    out: &mut String,
    content: &str,
    tag: &str,
    align: TableAlignment,
    refs: &LinkRefMap,
    opts: &ParseOptions,
    bufs: &mut InlineBuffers,
) {
    let (open, close) = match (tag, align) {
        ("th", TableAlignment::None) => ("<th>", "</th>\n"),
        ("td", TableAlignment::None) => ("<td>", "</td>\n"),
        ("th", TableAlignment::Left) => ("<th style=\"text-align: left\">", "</th>\n"),
        ("th", TableAlignment::Right) => ("<th style=\"text-align: right\">", "</th>\n"),
        ("th", TableAlignment::Center) => ("<th style=\"text-align: center\">", "</th>\n"),
        ("td", TableAlignment::Left) => ("<td style=\"text-align: left\">", "</td>\n"),
        ("td", TableAlignment::Right) => ("<td style=\"text-align: right\">", "</td>\n"),
        ("td", TableAlignment::Center) => ("<td style=\"text-align: center\">", "</td>\n"),
        _ => {
            out.push('<');
            out.push_str(tag);
            match align {
                TableAlignment::Left => out.push_str(" style=\"text-align: left\""),
                TableAlignment::Right => out.push_str(" style=\"text-align: right\""),
                TableAlignment::Center => out.push_str(" style=\"text-align: center\""),
                TableAlignment::None => {}
            }
            out.push('>');
            push_inline_or_plain(out, content, refs, opts, bufs);
            out.push_str("</");
            out.push_str(tag);
            out.push_str(">\n");
            return;
        }
    };
    out.push_str(open);
    push_inline_or_plain(out, content, refs, opts, bufs);
    out.push_str(close);
}
