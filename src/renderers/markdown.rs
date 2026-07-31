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
            // The fence must be longer than any backtick run inside the body,
            // otherwise the content terminates its own block.
            let fence_len = (longest_backtick_run(literal) + 1).max(3);
            for _ in 0..fence_len {
                out.push('`');
            }
            if !info.is_empty() {
                out.push_str(info.as_str());
            }
            out.push('\n');
            out.push_str(literal);
            if !literal.ends_with('\n') {
                out.push('\n');
            }
            for _ in 0..fence_len {
                out.push('`');
            }
            out.push('\n');
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

                // Build the marker first: its width sets the continuation
                // indent, so nested content lines up under the item text.
                let mut marker = String::new();
                match kind {
                    ListKind::Bullet(m) => {
                        marker.push(*m as char);
                        marker.push(' ');
                    }
                    ListKind::Ordered(delimiter) => {
                        use std::fmt::Write;
                        let _ = write!(marker, "{num}");
                        marker.push(*delimiter as char);
                        marker.push(' ');
                        num += 1;
                    }
                }

                render_list_item(child, out, &marker, depth + 1, *tight);
            }
        }
        Block::ListItem { children, .. } => {
            // Reached only when a ListItem appears outside a List; render its
            // children plainly. Marker/indent handling lives in render_list_item.
            for (i, child) in children.iter().enumerate() {
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

/// Render one list item: `marker` on the first line, then every subsequent
/// line indented by the marker's width so nested blocks stay inside the item.
fn render_list_item(block: &Block, out: &mut String, marker: &str, depth: usize, tight: bool) {
    let (children, checked): (&[Block], Option<bool>) = match block {
        Block::ListItem { children, checked } => (children, *checked),
        // Not a ListItem (malformed AST): render as the item's sole content.
        other => {
            let mut body = String::new();
            render_block(other, &mut body, depth, true);
            write_indented(out, &body, marker);
            return;
        }
    };

    let mut body = String::new();
    if let Some(c) = checked {
        body.push_str(if c { "[x] " } else { "[ ] " });
    }
    for (i, child) in children.iter().enumerate() {
        render_block(child, &mut body, depth, i == 0);
        // Blank line between blocks in a loose item.
        if !tight && i + 1 < children.len() {
            body.push('\n');
        }
    }

    write_indented(out, &body, marker);
}

/// Write `body` with `marker` prefixing its first line and an equal-width run
/// of spaces prefixing the rest. Blank lines stay blank (no trailing spaces).
fn write_indented(out: &mut String, body: &str, marker: &str) {
    let mut first = true;
    for line in body.lines() {
        if first {
            out.push_str(marker);
            first = false;
        } else if !line.is_empty() {
            for _ in 0..marker.len() {
                out.push(' ');
            }
        }
        out.push_str(line);
        out.push('\n');
    }
    if first {
        // Empty item: still emit the marker so the list structure survives.
        out.push_str(marker.trim_end());
        out.push('\n');
    }
}

/// Longest run of consecutive backticks in `s`.
pub(crate) fn longest_backtick_run(s: &str) -> usize {
    let mut max = 0;
    let mut cur = 0;
    for &b in s.as_bytes() {
        if b == b'`' {
            cur += 1;
            if cur > max {
                max = cur;
            }
        } else {
            cur = 0;
        }
    }
    max
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
        write_table_cell(cell.as_str(), out);
        out.push_str(" |");
        if i >= num_cols - 1 {
            break;
        }
    }
    // Pad a short header row: the delimiter row defines num_cols, and a
    // mismatch makes the whole table degrade back to a paragraph.
    for _ in table.header.len()..num_cols {
        out.push_str("  |");
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
                write_table_cell(cell.as_str(), out);
                out.push_str(" |");
            } else {
                out.push_str(" |");
            }
        }
        out.push('\n');
    }
}

/// Write a table cell, escaping the characters that would break the row: `|`
/// ends a cell, and a newline ends the row.
fn write_table_cell(cell: &str, out: &mut String) {
    for ch in cell.chars() {
        match ch {
            '|' => out.push_str("\\|"),
            '\n' | '\r' => out.push(' '),
            _ => out.push(ch),
        }
    }
}
