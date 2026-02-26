use crate::ParseOptions;
use crate::ast::{Block, ListKind, TableAlignment};
use crate::html::escape_html_into;
use crate::inline::{InlineBuffers, LinkRefMap, parse_inline_pass};

#[inline(always)]
fn is_plain_text(s: &str) -> bool {
    static NEEDS_PROCESSING: [bool; 256] = {
        let mut t = [false; 256];
        t[b'*' as usize] = true;
        t[b'_' as usize] = true;
        t[b'\\' as usize] = true;
        t[b'`' as usize] = true;
        t[b'!' as usize] = true;
        t[b'[' as usize] = true;
        t[b']' as usize] = true;
        t[b'<' as usize] = true;
        t[b'&' as usize] = true;
        t[b'\n' as usize] = true;
        t[b'~' as usize] = true;
        t[b'=' as usize] = true;
        t[b'+' as usize] = true;
        t[b':' as usize] = true;
        t[b'@' as usize] = true;
        t[b'>' as usize] = true;
        t[b'"' as usize] = true;
        t
    };
    for &b in s.as_bytes() {
        if NEEDS_PROCESSING[b as usize] {
            return false;
        }
    }
    true
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

pub(crate) fn render_block(
    block: &Block,
    refs: &LinkRefMap,
    out: &mut String,
    opts: &ParseOptions,
    bufs: &mut InlineBuffers,
) {
    let mut stack: Vec<Work<'_>> = vec![Work::Block(block)];

    while let Some(work) = stack.pop() {
        match work {
            Work::CloseTag(tag) => out.push_str(tag),
            Work::TightListItem(block) => {
                render_tight_list_item(block, refs, out, opts, bufs, &mut stack);
            }
            Work::TightBlock(block) => {
                if let Block::Paragraph { raw } = block {
                    parse_inline_pass(out, raw, refs, opts, bufs);
                } else {
                    render_one(block, refs, out, opts, bufs, &mut stack);
                }
            }
            Work::Block(block) => {
                render_one(block, refs, out, opts, bufs, &mut stack);
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
    refs: &LinkRefMap,
    out: &mut String,
    opts: &ParseOptions,
    bufs: &mut InlineBuffers,
    stack: &mut Vec<Work<'a>>,
) {
    match block {
        Block::Document { children } => {
            for child in children.iter().rev() {
                stack.push(Work::Block(child));
            }
        }
        Block::ThematicBreak => out.push_str("<hr />\n"),
        Block::Heading { level, raw } => {
            out.push_str("<h");
            out.push((b'0' + level) as char);
            out.push('>');
            parse_inline_pass(out, raw, refs, opts, bufs);
            out.push_str("</h");
            out.push((b'0' + level) as char);
            out.push_str(">\n");
        }
        Block::Paragraph { raw } => {
            out.push_str("<p>");
            parse_inline_pass(out, raw, refs, opts, bufs);
            out.push_str("</p>\n");
        }
        Block::CodeBlock { info, literal } => {
            out.push_str("<pre><code");
            if !info.is_empty() {
                let lang = info.split_whitespace().next().unwrap_or("");
                if !lang.is_empty() {
                    out.push_str(" class=\"language-");
                    escape_html_into(out, lang);
                    out.push('"');
                }
            }
            out.push('>');
            escape_html_into(out, literal);
            out.push_str("</code></pre>\n");
        }
        Block::HtmlBlock { literal } => {
            out.push_str(literal);
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
                render_nested_tight_list(kind, *start, children, refs, out, opts, bufs, stack);
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
        Block::Table {
            alignments,
            header,
            rows,
        } => {
            let all_none = alignments.iter().all(|a| *a == TableAlignment::None);
            out.push_str("<table>\n<thead>\n<tr>\n");
            for (i, cell) in header.iter().enumerate() {
                let align = if all_none {
                    TableAlignment::None
                } else {
                    alignments.get(i).copied().unwrap_or(TableAlignment::None)
                };
                render_table_cell(out, cell, "th", align, refs, opts, bufs);
            }
            out.push_str("</tr>\n</thead>\n");
            if !rows.is_empty() {
                out.push_str("<tbody>\n");
                if all_none {
                    for row in rows {
                        out.push_str("<tr>\n");
                        for cell in row.iter() {
                            out.push_str("<td>");
                            parse_inline_pass(out, cell, refs, opts, bufs);
                            out.push_str("</td>\n");
                        }
                        out.push_str("</tr>\n");
                    }
                } else {
                    for row in rows {
                        out.push_str("<tr>\n");
                        for (i, cell) in row.iter().enumerate() {
                            let align = alignments.get(i).copied().unwrap_or(TableAlignment::None);
                            render_table_cell(out, cell, "td", align, refs, opts, bufs);
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

#[inline(never)]
fn render_nested_tight_list<'a>(
    kind: &ListKind,
    start: u32,
    children: &'a [Block],
    refs: &LinkRefMap,
    out: &mut String,
    opts: &ParseOptions,
    bufs: &mut InlineBuffers,
    stack: &mut Vec<Work<'a>>,
) {
    const MAX_DEPTH: usize = 64;
    let mut close_tags: [&'static str; MAX_DEPTH] = [""; MAX_DEPTH];
    let mut depth: usize = 0;

    let mut cur_kind = kind;
    let mut cur_start = start;
    let mut cur_children: &'a [Block] = children;

    loop {
        emit_list_open(out, cur_kind, cur_start);

        let Block::ListItem {
            children: item_children,
            checked,
        } = &cur_children[0]
        else {
            stack.push(Work::CloseTag(list_close_tag(cur_kind)));
            stack.push(Work::Block(&cur_children[0]));
            break;
        };

        out.push_str("<li>");
        emit_checkbox(out, *checked);

        if item_children.len() == 2 && depth < MAX_DEPTH {
            if let (
                Block::Paragraph { raw },
                Block::List {
                    kind: inner_kind,
                    start: inner_start,
                    tight: true,
                    children: inner_children,
                },
            ) = (&item_children[0], &item_children[1])
            {
                if inner_children.len() == 1 {
                    if is_plain_text(raw) {
                        out.push_str(raw);
                    } else {
                        parse_inline_pass(out, raw, refs, opts, bufs);
                    }
                    out.push('\n');
                    close_tags[depth] = list_close_tag(cur_kind);
                    depth += 1;
                    cur_kind = inner_kind;
                    cur_start = *inner_start;
                    cur_children = inner_children;
                    continue;
                }
            }
        }

        if item_children.len() == 1 {
            if let Block::Paragraph { raw } = &item_children[0] {
                if is_plain_text(raw) {
                    out.push_str(raw);
                } else {
                    parse_inline_pass(out, raw, refs, opts, bufs);
                }
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
                    parse_inline_pass(out, raw, refs, opts, bufs);
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
    refs: &LinkRefMap,
    out: &mut String,
    opts: &ParseOptions,
    bufs: &mut InlineBuffers,
    stack: &mut Vec<Work<'a>>,
) {
    let Block::ListItem { children, checked } = block else {
        render_one(block, refs, out, opts, bufs, stack);
        return;
    };

    out.push_str("<li>");
    emit_checkbox(out, *checked);

    if children.len() == 1 {
        if let Block::Paragraph { raw } = &children[0] {
            parse_inline_pass(out, raw, refs, opts, bufs);
            out.push_str("</li>\n");
            return;
        }
    }

    stack.push(Work::CloseTag("</li>\n"));
    let mut prev_was_para = false;
    for (idx, child) in children.iter().enumerate() {
        match child {
            Block::Paragraph { raw } => {
                parse_inline_pass(out, raw, refs, opts, bufs);
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

fn render_table_cell(
    out: &mut String,
    content: &str,
    tag: &str,
    align: TableAlignment,
    refs: &LinkRefMap,
    opts: &ParseOptions,
    bufs: &mut InlineBuffers,
) {
    out.push('<');
    out.push_str(tag);
    match align {
        TableAlignment::Left => out.push_str(" style=\"text-align: left\""),
        TableAlignment::Right => out.push_str(" style=\"text-align: right\""),
        TableAlignment::Center => out.push_str(" style=\"text-align: center\""),
        TableAlignment::None => {}
    }
    out.push('>');
    parse_inline_pass(out, content, refs, opts, bufs);
    out.push_str("</");
    out.push_str(tag);
    out.push_str(">\n");
}
