use super::*;

impl<'a> BlockParser<'a> {
    #[inline(never)]
    pub(super) fn process_line(&mut self, mut line: Line<'a>) {
        // Fast path: if tip is a fenced code block at document level (no containers),
        // skip full container matching and handle directly
        let num_open = self.open.len();
        if num_open == 2 {
            if let OpenBlockType::FencedCode {
                fence_char,
                fence_len,
                fence_indent,
                ..
            } = &self.open[1].block_type
            {
                let fc = *fence_char;
                let fl = *fence_len;
                let fi = *fence_indent;
                let rem = line.remainder();
                if is_closing_fence(rem, fc, fl) {
                    self.close_top_block();
                    return;
                }
                if fi == 0 {
                    self.open[1].content.push_str(rem);
                } else {
                    let mut content_line = line.clone();
                    let _ = content_line.skip_indent(fi);
                    if content_line.partial_spaces == 0 {
                        self.open[1].content.push_str(content_line.remainder());
                    } else {
                        let content = content_line.remainder_with_partial();
                        self.open[1].content.push_str(&content);
                    }
                }
                self.open[1].content.push('\n');
                return;
            }
        }

        // Phase 1: Match continuation of open container blocks
        let mut matched = 1; // document always matches

        let mut all_matched = true;
        let mut i = 1;
        while i < num_open {
            match &self.open[i].block_type {
                OpenBlockType::BlockQuote => {
                    let (ns_col, _, ns_byte) = line.peek_nonspace_col();
                    let indent = ns_col - line.col_offset;
                    if indent <= 3 && ns_byte == b'>' {
                        line.advance_to_nonspace();
                        line.byte_offset += 1;
                        line.col_offset += 1;
                        if line.partial_spaces > 0 {
                            // There are remaining partial tab spaces
                            let consume = 1.min(line.partial_spaces);
                            line.partial_spaces -= consume;
                            line.col_offset += consume;
                        } else if line.byte_offset < line.raw.len() {
                            let b = line.raw.as_bytes()[line.byte_offset];
                            if b == b' ' {
                                line.byte_offset += 1;
                                line.col_offset += 1;
                            } else if b == b'\t' {
                                // Consume 1 column of the tab for the optional space
                                let tab_width = 4 - (line.col_offset % 4);
                                line.byte_offset += 1;
                                line.col_offset += 1;
                                if tab_width > 1 {
                                    line.partial_spaces = tab_width - 1;
                                }
                            }
                        }
                        matched = i + 1;
                    } else {
                        all_matched = false;
                        break;
                    }
                }
                OpenBlockType::ListItem {
                    content_col,
                    started_blank,
                    ..
                } => {
                    let content_col = *content_col;
                    let started_blank = *started_blank;
                    // Single peek gives us indent and whether line is blank
                    let (ns_col, ns_off, ns_byte) = line.peek_nonspace_col();
                    let indent = ns_col - line.col_offset;
                    let is_blank = ns_byte == 0 && ns_off >= line.raw.len();
                    if is_blank {
                        if started_blank
                            && self.open[i].children.is_empty()
                            && self.open[i].content.is_empty()
                            && !self.has_open_leaf_after(i)
                        {
                            all_matched = false;
                            break;
                        }
                        let _ = line.skip_indent(content_col);
                        matched = i + 1;
                    } else if indent >= content_col {
                        line.skip_indent(content_col);
                        matched = i + 1;
                    } else {
                        all_matched = false;
                        break;
                    }
                }
                OpenBlockType::FencedCode { .. }
                | OpenBlockType::IndentedCode
                | OpenBlockType::HtmlBlock { .. }
                | OpenBlockType::Paragraph
                | OpenBlockType::Table { .. } => {
                    // Leaf blocks: don't match continuation here, handled later
                    matched = i;
                    all_matched = false;
                    break;
                }
                OpenBlockType::Document => {
                    matched = i + 1;
                }
            }
            i += 1;
        }

        if all_matched {
            matched = num_open;
        }

        // Check if the tip is a leaf block that accepts lines
        let tip_idx = num_open - 1;
        let tip_is_leaf = matches!(
            self.open[tip_idx].block_type,
            OpenBlockType::FencedCode { .. }
                | OpenBlockType::IndentedCode
                | OpenBlockType::HtmlBlock { .. }
                | OpenBlockType::Paragraph
                | OpenBlockType::Table { .. }
        );

        // Handle leaf block continuation
        if matched == num_open || (matched == num_open - 1 && tip_is_leaf) {
            if tip_is_leaf && matched >= num_open - 1 {
                match &self.open[tip_idx].block_type {
                    OpenBlockType::FencedCode {
                        fence_char,
                        fence_len,
                        fence_indent,
                        ..
                    } => {
                        let fc = *fence_char;
                        let fl = *fence_len;
                        let fi = *fence_indent;
                        let rem = line.remainder();
                        if is_closing_fence(rem, fc, fl) {
                            self.close_top_block();
                            return;
                        }
                        // Strip up to fence_indent spaces from content
                        if fi == 0 {
                            // No indent to strip - fast path
                            self.open[tip_idx].content.push_str(rem);
                        } else {
                            let mut content_line = line.clone();
                            let _ = content_line.skip_indent(fi);
                            if content_line.partial_spaces == 0 {
                                self.open[tip_idx]
                                    .content
                                    .push_str(content_line.remainder());
                            } else {
                                let content = content_line.remainder_with_partial();
                                self.open[tip_idx].content.push_str(&content);
                            }
                        }
                        self.open[tip_idx].content.push('\n');
                        return;
                    }
                    OpenBlockType::IndentedCode => {
                        if line.is_blank() {
                            // Preserve indent beyond 4 columns for blank lines too
                            let mut bl = line.clone();
                            let _ = bl.skip_indent(4);
                            let rest = bl.remainder_with_partial();
                            if !self.open[tip_idx].content.is_empty() {
                                self.open[tip_idx].content.push('\n');
                            }
                            self.open[tip_idx].content.push_str(&rest);
                            self.mark_blank_on_list_items();
                            return;
                        }
                        let indent = line.indent();
                        if indent >= 4 {
                            // Remove exactly 4 columns of indent (preserving excess)
                            let _ = line.skip_indent(4);
                            let rest = line.remainder_with_partial();
                            if !self.open[tip_idx].content.is_empty() {
                                self.open[tip_idx].content.push('\n');
                            }
                            self.open[tip_idx].content.push_str(&rest);
                            return;
                        }
                        // Not enough indent, close the code block
                        self.close_top_block();
                        // Open new blocks with the already-consumed line
                        self.open_new_blocks(line);
                        return;
                    }
                    OpenBlockType::HtmlBlock { end_condition } => {
                        let end_condition = *end_condition;
                        // Types 6/7 end at blank line
                        if end_condition == HtmlBlockEnd::BlankLine && line.is_blank() {
                            self.close_top_block();
                            return;
                        }
                        if !self.open[tip_idx].content.is_empty() {
                            self.open[tip_idx].content.push('\n');
                        }
                        self.open[tip_idx].content.push_str(line.remainder());
                        if html_block_ends(&end_condition, line.remainder()) {
                            self.close_top_block();
                        }
                        return;
                    }
                    OpenBlockType::Table { .. } => {
                        if line.is_blank() {
                            self.close_top_block();
                            self.mark_blank_on_list_items();
                            return;
                        }
                        let rest = line.rest_of_line();
                        if let OpenBlockType::Table {
                            ref mut rows,
                            ref alignments,
                            ..
                        } = self.open[tip_idx].block_type
                        {
                            let num_cols = alignments.len();
                            let row = parse_table_row(rest, num_cols);
                            rows.push(row);
                        }
                        return;
                    }
                    OpenBlockType::Paragraph => {
                        // Compute indent and first-nonspace once for the whole paragraph branch
                        let (ns_col, ns_off, ns_byte) = line.peek_nonspace_col();
                        let indent = ns_col - line.col_offset;
                        let is_blank = ns_byte == 0 && ns_off >= line.raw.len();

                        if is_blank {
                            self.close_top_block();
                            self.mark_blank_on_list_items();
                            return;
                        }

                        let rest = if ns_off >= line.raw.len() {
                            ""
                        } else {
                            &line.raw[ns_off..]
                        };

                        // Check for table separator (paragraph has single line = header)
                        if self.enable_tables && !self.open[tip_idx].content.contains('\n') {
                            if let Some(alignments) = parse_table_separator(rest) {
                                let num_cols = alignments.len();
                                let header = parse_table_row(&self.open[tip_idx].content, num_cols);
                                if header.len() == num_cols {
                                    self.open.pop();
                                    self.open.push(OpenBlock::new(OpenBlockType::Table {
                                        alignments,
                                        header,
                                        rows: Vec::new(),
                                    }));
                                    return;
                                }
                            }
                        }
                        // Fast path: if first non-space byte cannot start any
                        // paragraph-interrupting construct, skip all checks.
                        if indent > 3
                            || !matches!(
                                ns_byte,
                                b'=' | b'-'
                                    | b'*'
                                    | b'_'
                                    | b'#'
                                    | b'`'
                                    | b'~'
                                    | b'<'
                                    | b'>'
                                    | b'+'
                                    | b'0'..=b'9' | b'|' | b':'
                            )
                        {
                            self.open[tip_idx].content.push('\n');
                            if indent == 0 {
                                self.open[tip_idx].content.push_str(line.remainder());
                            } else {
                                line.advance_to_nonspace();
                                self.open[tip_idx].content.push_str(line.remainder());
                            }
                            return;
                        }
                        // Check for setext heading underline (up to 3 spaces indent)
                        if indent <= 3 {
                            if let Some(level) = parse_setext_underline(rest) {
                                let content = std::mem::take(&mut self.open[tip_idx].content);
                                let remaining = self.extract_ref_defs(&content);
                                if remaining.is_empty() {
                                    self.open.pop();
                                    let mut para = OpenBlock::new(OpenBlockType::Paragraph);
                                    para.content.push_str(rest);
                                    self.open.push(para);
                                    return;
                                }
                                let raw = if remaining.len() != remaining.trim_end().len() {
                                    remaining.trim_end().to_string()
                                } else {
                                    remaining
                                };
                                self.open.pop();
                                let heading = Block::Heading { level, raw };
                                let parent = self.open.last_mut().unwrap();
                                parent.children.push(heading);
                                return;
                            }
                        }
                        // Check for thematic break (which can interrupt a paragraph)
                        if indent <= 3 && is_thematic_break(rest) {
                            self.close_top_block();
                            let parent = self.open.last_mut().unwrap();
                            parent.children.push(Block::ThematicBreak);
                            return;
                        }
                        // Check for ATX heading
                        if indent <= 3 {
                            if let Some((level, content)) = parse_atx_heading(rest) {
                                self.close_top_block();
                                let parent = self.open.last_mut().unwrap();
                                parent.children.push(Block::Heading {
                                    level,
                                    raw: content.to_string(),
                                });
                                return;
                            }
                        }
                        // Check for fenced code start
                        if indent <= 3 {
                            if let Some((fence_char, fence_len, info)) = parse_fence_start(rest) {
                                self.close_top_block();
                                self.open.push(OpenBlock::new(OpenBlockType::FencedCode {
                                    fence_char,
                                    fence_len,
                                    fence_indent: indent,
                                    info: resolve_entities_and_escapes(info),
                                }));
                                return;
                            }
                        }
                        // Check for HTML block start (types 1-6 can interrupt paragraph)
                        if indent <= 3 {
                            if let Some(end_condition) = parse_html_block_start(rest, true) {
                                self.close_top_block();
                                let mut block = OpenBlock::new(OpenBlockType::HtmlBlock {
                                    end_condition: end_condition,
                                });
                                block.content.push_str(line.remainder());
                                if html_block_ends(&end_condition, line.remainder()) {
                                    let parent = self.open.last_mut().unwrap();
                                    parent.children.push(Block::HtmlBlock {
                                        literal: block.content,
                                    });
                                } else {
                                    self.open.push(block);
                                }
                                return;
                            }
                        }
                        // Check for blockquote start (can interrupt paragraph)
                        if indent <= 3 && ns_byte == b'>' {
                            self.close_top_block();
                            self.open_new_blocks(line);
                            return;
                        }
                        // Check for list marker that can interrupt paragraph
                        if indent <= 3 {
                            if let Some(marker) = parse_list_marker(rest) {
                                if can_interrupt_paragraph(&marker) {
                                    self.close_top_block();
                                    self.open_new_blocks(line);
                                    return;
                                }
                            }
                        }
                        // Paragraph continuation (including lazy continuation)
                        self.open[tip_idx].content.push('\n');
                        // Fast path: if no leading whitespace, skip advance_to_nonspace
                        if indent == 0 {
                            self.open[tip_idx].content.push_str(line.remainder());
                        } else {
                            line.advance_to_nonspace();
                            self.open[tip_idx].content.push_str(line.remainder());
                        }
                        return;
                    }
                    _ => {}
                }
            }
        }

        // Check for lazy continuation of a paragraph
        if !all_matched && !line.is_blank() {
            let tip_idx = self.open.len() - 1;
            if matches!(self.open[tip_idx].block_type, OpenBlockType::Paragraph) {
                let rest = line.rest_of_line();
                let indent = line.indent();

                // Check if the line would start a new block at the matched level
                let can_start_new = (indent <= 3 && line.first_nonspace_byte() == b'>')
                    || (indent <= 3 && is_thematic_break(rest))
                    || (indent <= 3 && parse_atx_heading(rest).is_some())
                    || (indent <= 3 && parse_fence_start(rest).is_some())
                    || (indent <= 3 && parse_html_block_start(rest, false).is_some());

                if !can_start_new {
                    // Also check: if there's an unmatched list item, and the line
                    // would be a list marker at the parent level, don't allow lazy
                    let has_unmatched_list = (matched..num_open).any(|idx| {
                        matches!(self.open[idx].block_type, OpenBlockType::ListItem { .. })
                    });

                    let is_new_list_marker = indent <= 3 && parse_list_marker(rest).is_some();

                    if has_unmatched_list && is_new_list_marker {
                        // Don't allow lazy continuation - this starts a new item
                    } else if !is_new_list_marker || !has_unmatched_list {
                        // Allow lazy continuation (also if it's a list marker that
                        // can interrupt paragraph and there's no unmatched list)
                        if !(indent <= 3
                            && parse_list_marker(rest)
                                .map_or(false, |m| can_interrupt_paragraph(&m)))
                        {
                            self.open[tip_idx].content.push('\n');
                            line.advance_to_nonspace();
                            self.open[tip_idx].content.push_str(line.remainder());
                            return;
                        }
                    }
                }
            }
        }

        // Phase 2: Close unmatched blocks
        while self.open.len() > matched {
            self.close_top_block();
        }

        // Phase 3: Try to open new container/leaf blocks
        self.open_new_blocks(line);
    }

    #[inline(never)]
    pub(super) fn open_new_blocks(&mut self, mut line: Line<'a>) {
        // Keep trying to open new blocks
        loop {
            // Single call to peek_nonspace_col gives us indent AND first byte
            let (ns_col, ns_off, first_byte) = line.peek_nonspace_col();
            let indent = ns_col - line.col_offset;

            if first_byte == 0 && ns_off >= line.raw.len() {
                // Blank line
                // Mark blank line on innermost enclosing list item (for loose detection)
                let len = self.open.len();
                let mut found_list_item = false;
                for i in (1..len).rev() {
                    if matches!(self.open[i].block_type, OpenBlockType::ListItem { .. }) {
                        self.open[i].had_blank_in_item = true;
                        found_list_item = true;
                        break;
                    }
                }
                // If no list item is open but the parent has a list as its last child,
                // mark that a blank occurred between list items
                if !found_list_item {
                    let parent = self.open.last_mut().unwrap();
                    if parent
                        .children
                        .last()
                        .map_or(false, |c| matches!(c, Block::List { .. }))
                    {
                        parent.list_has_blank_between = true;
                    }
                }
                return;
            }

            // Blockquote
            if indent <= 3 && first_byte == b'>' {
                line.advance_to_nonspace();
                line.byte_offset += 1;
                line.col_offset += 1;
                if line.partial_spaces > 0 {
                    let consume = 1.min(line.partial_spaces);
                    line.partial_spaces -= consume;
                    line.col_offset += consume;
                } else if line.byte_offset < line.raw.len() {
                    let b = line.raw.as_bytes()[line.byte_offset];
                    if b == b' ' {
                        line.byte_offset += 1;
                        line.col_offset += 1;
                    } else if b == b'\t' {
                        let tab_width = 4 - (line.col_offset % 4);
                        line.byte_offset += 1;
                        line.col_offset += 1;
                        if tab_width > 1 {
                            line.partial_spaces = tab_width - 1;
                        }
                    }
                }
                self.open.push(OpenBlock::new(OpenBlockType::BlockQuote));
                continue;
            }

            if indent <= 3 {
                // Compute rest once for all checks that need it
                let rest = if ns_off >= line.raw.len() {
                    ""
                } else {
                    &line.raw[ns_off..]
                };

                // ATX heading
                if let Some((level, content)) = parse_atx_heading(rest) {
                    line.advance_to_nonspace();
                    let parent = self.open.last_mut().unwrap();
                    parent.children.push(Block::Heading {
                        level,
                        raw: content.to_string(),
                    });
                    return;
                }

                // Fenced code block
                if let Some((fence_char, fence_len, info)) = parse_fence_start(rest) {
                    self.open.push(OpenBlock::new(OpenBlockType::FencedCode {
                        fence_char,
                        fence_len,
                        fence_indent: indent,
                        info: resolve_entities_and_escapes(info),
                    }));
                    return;
                }

                // HTML block
                if let Some(end_condition) = parse_html_block_start(rest, false) {
                    let mut block = OpenBlock::new(OpenBlockType::HtmlBlock {
                        end_condition: end_condition,
                    });
                    block.content.push_str(line.remainder());
                    if html_block_ends(&end_condition, line.remainder()) {
                        let parent = self.open.last_mut().unwrap();
                        parent.children.push(Block::HtmlBlock {
                            literal: block.content,
                        });
                    } else {
                        self.open.push(block);
                    }
                    return;
                }

                // Thematic break
                if is_thematic_break(rest) {
                    let parent = self.open.last_mut().unwrap();
                    parent.children.push(Block::ThematicBreak);
                    return;
                }

                // List item
                if let Some(marker) = parse_list_marker(rest) {
                    let marker_indent = indent;
                    line.advance_to_nonspace();
                    let rest_is_blank = self.start_list_item(&mut line, marker, marker_indent);
                    if rest_is_blank {
                        return;
                    }
                    continue;
                }
            } else {
                // indent >= 4: Indented code block
                // Cannot start indented code in a list item if we're looking at the first content
                // Check that tip is not a paragraph
                let tip = self.open.last().unwrap();
                if !matches!(tip.block_type, OpenBlockType::Paragraph) {
                    // Remove exactly 4 columns of indent, preserving excess
                    let _ = line.skip_indent(4);
                    let content = line.remainder_with_partial();
                    let mut block = OpenBlock::new(OpenBlockType::IndentedCode);
                    block.content.push_str(&content);
                    self.open.push(block);
                    return;
                }
            }

            line.advance_to_nonspace();
            let mut block = OpenBlock::new(OpenBlockType::Paragraph);
            block.content.push_str(line.remainder());
            self.open.push(block);
            return;
        }
    }

    /// Returns true if the rest of the line after the marker is blank (empty item).
    #[inline]
    pub(super) fn start_list_item(
        &mut self,
        line: &mut Line<'a>,
        marker: ListMarkerInfo,
        marker_indent: usize,
    ) -> bool {
        line.advance_columns(marker.marker_len);
        let (ns_col, ns_off, ns_byte) = line.peek_nonspace_col();
        let rest_blank = ns_byte == 0 && ns_off >= line.raw.len();
        let spaces_after;
        if rest_blank {
            spaces_after = 1;
        } else {
            let total_sp = ns_col - line.col_offset;
            if total_sp >= 5 {
                spaces_after = 1;
            } else if total_sp == 0 {
                spaces_after = 1;
            } else {
                spaces_after = total_sp;
            }
        }

        let content_col = marker_indent + marker.marker_len + spaces_after;

        if !rest_blank {
            let _ = line.skip_indent(spaces_after);
        }

        // Detect task list checkbox: [ ] or [x] or [X] followed by a space
        let mut checked = None;
        if !rest_blank && self.enable_task_lists {
            let rem = line.remainder().as_bytes();
            if rem.len() >= 4 && rem[0] == b'[' && rem[2] == b']' && rem[3] == b' ' {
                match rem[1] {
                    b' ' => {
                        checked = Some(false);
                        line.byte_offset += 4;
                        line.col_offset += 4;
                    }
                    b'x' | b'X' => {
                        checked = Some(true);
                        line.byte_offset += 4;
                        line.col_offset += 4;
                    }
                    _ => {}
                }
            }
        }

        let list_kind = marker.kind;

        let mut item = OpenBlock::new(OpenBlockType::ListItem {
            content_col,
            started_blank: rest_blank,
        });
        item.list_kind = Some(list_kind);
        item.list_start = marker.start_num;
        item.checked = checked;
        self.open.push(item);
        rest_blank
    }

    pub(super) fn finalize_block(&mut self, block: OpenBlock) -> Option<Block> {
        match block.block_type {
            OpenBlockType::Document => Some(Block::Document {
                children: block.children,
            }),
            OpenBlockType::BlockQuote => Some(Block::BlockQuote {
                children: block.children,
            }),
            OpenBlockType::ListItem { .. } => {
                let item = Block::ListItem {
                    children: block.children,
                    checked: block.checked,
                };
                let parent = self.open.last_mut().unwrap();
                let kind = block.list_kind.unwrap_or(ListKind::Bullet(b'-'));
                let had_blank = block.had_blank_in_item;

                // A list item with a blank line that has 2+ children is "loose-inducing"
                // (the blank is between block children, not just trailing)
                let item_children_count = match &item {
                    Block::ListItem { children, .. } => children.len(),
                    _ => 0,
                };
                let blank_between_children = had_blank && item_children_count >= 2;

                // If this item had a trailing blank (blank but not between its own
                // children), propagate the blank to the enclosing list item.
                // This handles the case where a blank line falls between a sublist
                // and subsequent content in the parent item.
                if had_blank && !blank_between_children {
                    // Propagate to enclosing list item (the parent might be a ListItem)
                    if matches!(parent.block_type, OpenBlockType::ListItem { .. }) {
                        parent.had_blank_in_item = true;
                    }
                }

                if let Some(Block::List {
                    kind: lk,
                    children: items,
                    tight,
                    ..
                }) = parent.children.last_mut()
                {
                    if *lk == kind {
                        // A blank line between items (had_blank on previous item,
                        // and now we're adding another) → loose
                        if parent.list_has_blank_between {
                            *tight = false;
                        }
                        // Blank between children of this item → loose
                        if blank_between_children {
                            *tight = false;
                        }
                        items.push(item);
                        if had_blank {
                            parent.list_has_blank_between = true;
                        }
                        return None;
                    }
                }

                // New list
                parent.list_has_blank_between = false;
                if had_blank {
                    parent.list_has_blank_between = true;
                }

                let list = Block::List {
                    kind,
                    start: block.list_start,
                    tight: !blank_between_children,
                    children: vec![item],
                };
                return Some(list);
            }
            OpenBlockType::FencedCode { info, .. } => Some(Block::CodeBlock {
                info,
                literal: block.content,
            }),
            OpenBlockType::IndentedCode => {
                let mut literal = block.content;
                literal.push('\n');
                while literal.ends_with("\n\n") {
                    literal.pop();
                }
                if !literal.ends_with('\n') {
                    literal.push('\n');
                }
                Some(Block::CodeBlock {
                    info: String::new(),
                    literal,
                })
            }
            OpenBlockType::HtmlBlock { .. } => {
                let literal = block.content;
                Some(Block::HtmlBlock { literal })
            }
            OpenBlockType::Table {
                alignments,
                header,
                rows,
            } => Some(Block::Table {
                alignments,
                header,
                rows,
            }),
            OpenBlockType::Paragraph => {
                let trimmed = block.content.trim();
                if trimmed.is_empty() {
                    return None;
                }
                // Try to extract link reference definitions
                // Optimize: avoid copy when no ref defs are extracted and content is already trimmed
                let remaining = self.extract_ref_defs_owned(block.content);
                if remaining.is_empty() {
                    return None;
                }
                Some(Block::Paragraph { raw: remaining })
            }
        }
    }

    pub(super) fn extract_ref_defs(&mut self, content: &str) -> String {
        let mut pos = 0;
        loop {
            let trimmed = content[pos..].trim_start();
            if !trimmed.starts_with('[') {
                break;
            }
            if let Some((label, href, title, consumed)) = parse_link_ref_def(trimmed) {
                let key = crate::inline::normalize_reference_label(&label);
                if !self.ref_defs.contains_key(&key) {
                    let resolved_href = resolve_entities_and_escapes(&href);
                    let resolved_title = title.map(|t| resolve_entities_and_escapes(&t));
                    self.ref_defs.insert(
                        key,
                        crate::inline::LinkReference {
                            href: resolved_href,
                            title: resolved_title,
                        },
                    );
                }
                let trim_offset = content.len() - pos - trimmed.len();
                pos += trim_offset + consumed;
            } else {
                break;
            }
        }
        content[pos..].trim().to_string()
    }

    /// Like extract_ref_defs but takes an owned String and avoids copying when no ref defs found.
    pub(super) fn extract_ref_defs_owned(&mut self, mut content: String) -> String {
        // Compute trim boundaries
        let bytes = content.as_bytes();
        let len = bytes.len();
        let mut start = 0;
        while start < len && matches!(bytes[start], b' ' | b'\t' | b'\n' | b'\r') {
            start += 1;
        }
        if start == len {
            return String::new();
        }
        let mut end = len;
        while end > start && matches!(bytes[end - 1], b' ' | b'\t' | b'\n' | b'\r') {
            end -= 1;
        }

        // Quick check: if doesn't start with '[', no ref defs possible
        if bytes[start] != b'[' {
            // Trim in-place if needed
            if start == 0 && end == len {
                return content; // Already trimmed, return as-is — zero copy!
            }
            content.truncate(end);
            if start > 0 {
                content.drain(..start);
            }
            return content;
        }
        // Has potential ref defs — use the full extraction
        self.extract_ref_defs(&content[start..end])
    }
}
