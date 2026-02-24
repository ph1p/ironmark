use crate::ParseOptions;
use crate::ast::{Block, ListKind, TableAlignment};
use crate::html::escape_html_into;
use crate::inline::{InlineBuffers, LinkRefMap, parse_inline_pass};

pub(crate) fn render_block(
    block: &Block,
    refs: &LinkRefMap,
    out: &mut String,
    opts: &ParseOptions,
    bufs: &mut InlineBuffers,
) {
    match block {
        Block::Document { children } => {
            for child in children {
                render_block(child, refs, out, opts, bufs);
            }
        }
        Block::ThematicBreak => {
            out.push_str("<hr />\n");
        }
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
            for child in children {
                render_block(child, refs, out, opts, bufs);
            }
            out.push_str("</blockquote>\n");
        }
        Block::List {
            kind,
            start,
            tight,
            children,
        } => {
            match kind {
                ListKind::Bullet(_) => out.push_str("<ul>\n"),
                ListKind::Ordered(_delim) => {
                    if *start == 1 {
                        out.push_str("<ol>\n");
                    } else {
                        use std::fmt::Write;
                        out.push_str("<ol start=\"");
                        let _ = write!(out, "{}", start);
                        out.push_str("\">\n");
                    }
                }
            }
            for item in children {
                render_list_item(item, refs, out, *tight, opts, bufs);
            }
            match kind {
                ListKind::Bullet(_) => out.push_str("</ul>\n"),
                ListKind::Ordered(_) => out.push_str("</ol>\n"),
            }
        }
        Block::ListItem { children, .. } => {
            out.push_str("<li>");
            for child in children {
                render_block(child, refs, out, opts, bufs);
            }
            out.push_str("</li>\n");
        }
        Block::Table {
            alignments,
            header,
            rows,
        } => {
            out.push_str("<table>\n<thead>\n<tr>\n");
            for (i, cell) in header.iter().enumerate() {
                let align = alignments.get(i).copied().unwrap_or(TableAlignment::None);
                render_table_cell(out, cell, "th", align, refs, opts, bufs);
            }
            out.push_str("</tr>\n</thead>\n");
            if !rows.is_empty() {
                out.push_str("<tbody>\n");
                for row in rows {
                    out.push_str("<tr>\n");
                    for (i, cell) in row.iter().enumerate() {
                        let align = alignments.get(i).copied().unwrap_or(TableAlignment::None);
                        render_table_cell(out, cell, "td", align, refs, opts, bufs);
                    }
                    out.push_str("</tr>\n");
                }
                out.push_str("</tbody>\n");
            }
            out.push_str("</table>\n");
        }
    }
}

#[inline]
fn render_list_item(
    block: &Block,
    refs: &LinkRefMap,
    out: &mut String,
    tight: bool,
    opts: &ParseOptions,
    bufs: &mut InlineBuffers,
) {
    let Block::ListItem { children, checked } = block else {
        render_block(block, refs, out, opts, bufs);
        return;
    };

    out.push_str("<li>");
    match checked {
        Some(true) => out.push_str("<input type=\"checkbox\" checked=\"\" disabled=\"\" /> "),
        Some(false) => out.push_str("<input type=\"checkbox\" disabled=\"\" /> "),
        None => {}
    }
    if tight {
        if children.len() == 1 {
            if let Block::Paragraph { raw } = &children[0] {
                parse_inline_pass(out, raw, refs, opts, bufs);
                out.push_str("</li>\n");
                return;
            }
        }
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
                    render_block(child, refs, out, opts, bufs);
                    prev_was_para = false;
                }
            }
        }
    } else if !children.is_empty() {
        out.push('\n');
        for child in children {
            render_block(child, refs, out, opts, bufs);
        }
    }
    out.push_str("</li>\n");
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
