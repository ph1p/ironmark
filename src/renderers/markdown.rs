use crate::ast::{Block, ListKind, TableAlignment, TableData};

/// Renders an AST back into a Markdown string.
///
/// This function converts a parsed AST (from `parse_markdown` or `parse_html_to_ast`)
/// back into Markdown syntax.
///
/// # Examples
///
/// ```
/// use ironmark::{parse_markdown, ParseOptions};
/// use ironmark::render_markdown;
///
/// let ast = parse_markdown("# Hello\n\n**world**", &ParseOptions::default());
/// let md = render_markdown(&ast);
/// assert!(md.contains("# Hello"));
/// ```
pub fn render_markdown(root: &Block) -> String {
    let mut out = String::new();
    render_block(root, &mut out, 0, false);
    // Trim trailing whitespace but keep one final newline
    let trimmed_len = out.trim_end().len();
    out.truncate(trimmed_len);
    if !out.is_empty() {
        out.push('\n');
    }
    out
}

fn render_block(block: &Block, out: &mut String, depth: usize, in_list_item: bool) {
    match block {
        Block::Document { children } => {
            let mut first = true;
            for child in children {
                if !first {
                    // Add blank line between top-level blocks
                    if !out.ends_with("\n\n") && !out.ends_with("\n>\n") {
                        out.push('\n');
                    }
                }
                first = false;
                render_block(child, out, depth, false);
            }
        }
        Block::Paragraph { raw } => {
            if in_list_item && !out.ends_with('\n') && !out.is_empty() {
                // Don't add extra line break for first paragraph in list item
            }
            out.push_str(raw);
            out.push('\n');
        }
        Block::Heading { level, raw } => {
            for _ in 0..*level {
                out.push('#');
            }
            out.push(' ');
            out.push_str(raw);
            out.push('\n');
        }
        Block::ThematicBreak => {
            out.push_str("---\n");
        }
        Block::CodeBlock { info, literal } => {
            out.push_str("```");
            if !info.is_empty() {
                out.push_str(info.as_str());
            }
            out.push('\n');
            out.push_str(literal);
            if !literal.ends_with('\n') {
                out.push('\n');
            }
            out.push_str("```\n");
        }
        Block::BlockQuote { children } => {
            for child in children {
                let mut child_out = String::new();
                render_block(child, &mut child_out, depth + 1, false);
                for line in child_out.lines() {
                    out.push_str("> ");
                    out.push_str(line);
                    out.push('\n');
                }
            }
        }
        Block::List {
            kind,
            start,
            tight,
            children,
        } => {
            let mut num = *start;
            for (i, child) in children.iter().enumerate() {
                // Add blank line between items in loose lists (except first)
                if !*tight && i > 0 {
                    out.push('\n');
                }

                // Render list marker
                match kind {
                    ListKind::Bullet(marker) => {
                        out.push(*marker as char);
                        out.push(' ');
                    }
                    ListKind::Ordered(delimiter) => {
                        use std::fmt::Write;
                        let _ = write!(out, "{num}");
                        out.push(*delimiter as char);
                        out.push(' ');
                        num += 1;
                    }
                }

                // Render list item content
                render_list_item_content(child, out, depth + 1, *tight);
            }
        }
        Block::ListItem { children, checked } => {
            // Task list checkbox
            if let Some(c) = *checked {
                out.push_str(if c { "[x] " } else { "[ ] " });
            }
            for (i, child) in children.iter().enumerate() {
                if i > 0 {
                    // Indent continuation
                    out.push_str("    ");
                }
                render_block(child, out, depth, i == 0);
            }
        }
        Block::HtmlBlock { literal } => {
            out.push_str(literal);
            if !literal.ends_with('\n') {
                out.push('\n');
            }
        }
        Block::Table(table_data) => {
            render_table(table_data, out);
        }
    }
}

fn render_list_item_content(block: &Block, out: &mut String, depth: usize, tight: bool) {
    if let Block::ListItem { children, checked } = block {
        // Task list checkbox
        if let Some(c) = *checked {
            out.push_str(if c { "[x] " } else { "[ ] " });
        }

        for (i, child) in children.iter().enumerate() {
            if i > 0 {
                // Add indent for continuation blocks
                for _ in 0..depth {
                    out.push_str("    ");
                }
            }
            render_block(child, out, depth, i == 0);
            if !tight && i < children.len() - 1 {
                out.push('\n');
            }
        }
    } else {
        render_block(block, out, depth, true);
    }
}

fn render_table(table: &TableData, out: &mut String) {
    let num_cols = table.num_cols;
    if num_cols == 0 {
        return;
    }

    // Render header row
    out.push('|');
    for (i, cell) in table.header.iter().enumerate() {
        out.push(' ');
        out.push_str(cell.as_str());
        out.push_str(" |");
        if i >= num_cols - 1 {
            break;
        }
    }
    out.push('\n');

    // Render separator row with alignments
    out.push('|');
    for i in 0..num_cols {
        let alignment = table
            .alignments
            .get(i)
            .copied()
            .unwrap_or(TableAlignment::None);
        match alignment {
            TableAlignment::None => out.push_str(" --- |"),
            TableAlignment::Left => out.push_str(" :-- |"),
            TableAlignment::Center => out.push_str(" :-: |"),
            TableAlignment::Right => out.push_str(" --: |"),
        }
    }
    out.push('\n');

    // Render body rows
    let num_rows = table.rows.len() / num_cols;
    for row_idx in 0..num_rows {
        out.push('|');
        for col_idx in 0..num_cols {
            let cell_idx = row_idx * num_cols + col_idx;
            if let Some(cell) = table.rows.get(cell_idx) {
                out.push(' ');
                out.push_str(cell.as_str());
                out.push_str(" |");
            } else {
                out.push_str(" |");
            }
        }
        out.push('\n');
    }
}
