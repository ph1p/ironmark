use super::*;

#[inline(always)]
fn advance_past_blockquote_marker(line: &mut Line) {
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
}

impl<'a> BlockParser<'a> {
    #[inline(never)]
    pub(super) fn process_line(&mut self, mut line: Line<'a>) {
        let num_open = self.open.len();

        let mut matched = 1;

        let mut all_matched = true;
        let mut i = 1;

        if num_open > 2 && line.partial_spaces == 0 && self.open_blockquotes == 0 {
            let tip_is_leaf = matches!(
                self.open[num_open - 1].block_type,
                OpenBlockType::Paragraph
                    | OpenBlockType::FencedCode(..)
                    | OpenBlockType::IndentedCode
                    | OpenBlockType::HtmlBlock { .. }
                    | OpenBlockType::Table(..)
            );
            let container_end = if tip_is_leaf { num_open - 1 } else { num_open };

            if container_end > 1 {
                let total_indent = self.list_indent_sum;

                let (ns_col, ns_off, ns_byte) = line.peek_nonspace_col();
                let is_blank = ns_byte == 0 && ns_off >= line.raw.len();

                if !is_blank {
                    let indent = ns_col - line.col_offset;
                    let off = line.byte_offset;
                    let no_tabs = (ns_off - off) == indent;

                    if indent >= total_indent && no_tabs {
                        line.byte_offset += total_indent;
                        line.col_offset += total_indent;
                        matched = container_end;
                        i = container_end;
                        all_matched = container_end == num_open;
                        if all_matched {
                            matched = num_open;
                        }
                    }
                }
            }
        }

        while i < num_open {
            match &self.open[i].block_type {
                OpenBlockType::BlockQuote => {
                    let (ns_col, _, ns_byte) = line.peek_nonspace_col();
                    let indent = ns_col - line.col_offset;
                    if indent <= 3 && ns_byte == b'>' {
                        line.advance_to_nonspace();
                        advance_past_blockquote_marker(&mut line);
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
                OpenBlockType::FencedCode(..)
                | OpenBlockType::IndentedCode
                | OpenBlockType::HtmlBlock { .. }
                | OpenBlockType::Paragraph
                | OpenBlockType::Table(..) => {
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

        let tip_idx = num_open - 1;
        let tip_is_leaf = matches!(
            self.open[tip_idx].block_type,
            OpenBlockType::FencedCode(..)
                | OpenBlockType::IndentedCode
                | OpenBlockType::HtmlBlock { .. }
                | OpenBlockType::Paragraph
                | OpenBlockType::Table(..)
        );

        if matched == num_open || (matched == num_open - 1 && tip_is_leaf) {
            if tip_is_leaf && matched >= num_open - 1 {
                match &self.open[tip_idx].block_type {
                    OpenBlockType::FencedCode(fc_data) => {
                        let fc = fc_data.fence_char;
                        let fl = fc_data.fence_len;
                        let fi = fc_data.fence_indent;
                        if is_closing_fence(line.remainder().as_bytes(), fc, fl) {
                            self.close_top_block();
                            return;
                        }
                        if fi > 0 {
                            let _ = line.skip_indent(fi);
                        }
                        if line.partial_spaces > 0 {
                            let content = line.remainder_with_partial();
                            self.open[tip_idx].content.push_str(&content);
                        } else {
                            self.open[tip_idx].content.push_str(line.remainder());
                        }
                        self.open[tip_idx].content.push('\n');
                        return;
                    }
                    OpenBlockType::IndentedCode => {
                        if line.is_blank() {
                            let _ = line.skip_indent(4);
                            let rest = line.remainder_with_partial();
                            if !self.open[tip_idx].content.is_empty() {
                                self.open[tip_idx].content.push('\n');
                            }
                            self.open[tip_idx].content.push_str(&rest);
                            self.mark_blank_on_list_items();
                            return;
                        }
                        let indent = line.indent();
                        if indent >= 4 {
                            let _ = line.skip_indent(4);
                            let rest = line.remainder_with_partial();
                            if !self.open[tip_idx].content.is_empty() {
                                self.open[tip_idx].content.push('\n');
                            }
                            self.open[tip_idx].content.push_str(&rest);
                            return;
                        }
                        self.close_top_block();
                        self.open_new_blocks(line);
                        return;
                    }
                    OpenBlockType::HtmlBlock { end_condition } => {
                        let end_condition = *end_condition;
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
                    OpenBlockType::Table(..) => {
                        if line.is_blank() {
                            self.close_top_block();
                            self.mark_blank_on_list_items();
                            return;
                        }
                        let rest = line.rest_of_line();
                        if let OpenBlockType::Table(td) = &mut self.open[tip_idx].block_type {
                            let num_cols = td.alignments.len();
                            let row = parse_table_row(rest, num_cols);
                            td.rows.push(row);
                        }
                        return;
                    }
                    OpenBlockType::Paragraph => {
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

                        if self.enable_tables && !self.open[tip_idx].content_has_newline {
                            if let Some(alignments) = parse_table_separator(rest) {
                                let num_cols = alignments.len();
                                let header = parse_table_row(&self.open[tip_idx].content, num_cols);
                                if header.len() == num_cols {
                                    self.open.pop();
                                    self.open.push(OpenBlock::new(OpenBlockType::Table(Box::new(
                                        TableData {
                                            alignments,
                                            header,
                                            rows: Vec::with_capacity(16),
                                        },
                                    ))));
                                    return;
                                }
                            }
                        }
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
                            self.open[tip_idx].content_has_newline = true;
                            if indent == 0 {
                                self.open[tip_idx].content.push_str(line.remainder());
                            } else {
                                line.advance_to_nonspace();
                                self.open[tip_idx].content.push_str(line.remainder());
                            }
                            return;
                        }
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
                            if is_thematic_break(rest) {
                                self.close_top_block();
                                let parent = self.open.last_mut().unwrap();
                                parent.children.push(Block::ThematicBreak);
                                return;
                            }
                            if let Some((level, content)) = parse_atx_heading(rest) {
                                self.close_top_block();
                                let parent = self.open.last_mut().unwrap();
                                parent.children.push(Block::Heading {
                                    level,
                                    raw: content.to_string(),
                                });
                                return;
                            }
                            if let Some((fence_char, fence_len, info)) = parse_fence_start(rest) {
                                self.close_top_block();
                                self.open.push(OpenBlock::new(OpenBlockType::FencedCode(
                                    Box::new(FencedCodeData {
                                        fence_char,
                                        fence_len,
                                        fence_indent: indent,
                                        info: resolve_entities_and_escapes(info),
                                    }),
                                )));
                                return;
                            }
                            if let Some(end_condition) = parse_html_block_start(rest, true) {
                                self.close_top_block();
                                let mut block =
                                    OpenBlock::new(OpenBlockType::HtmlBlock { end_condition });
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
                            if ns_byte == b'>' {
                                self.close_top_block();
                                self.open_new_blocks(line);
                                return;
                            }
                            if let Some(marker) = parse_list_marker(rest) {
                                if can_interrupt_paragraph(&marker) {
                                    self.close_top_block();
                                    self.open_new_blocks(line);
                                    return;
                                }
                            }
                        }
                        self.open[tip_idx].content.push('\n');
                        self.open[tip_idx].content_has_newline = true;
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

        if !all_matched && !line.is_blank() {
            let tip_idx = self.open.len() - 1;
            if matches!(self.open[tip_idx].block_type, OpenBlockType::Paragraph) {
                let rest = line.rest_of_line();
                let indent = line.indent();

                let can_start_new = (indent <= 3 && line.first_nonspace_byte() == b'>')
                    || (indent <= 3 && is_thematic_break(rest))
                    || (indent <= 3 && parse_atx_heading(rest).is_some())
                    || (indent <= 3 && parse_fence_start(rest).is_some())
                    || (indent <= 3 && parse_html_block_start(rest, false).is_some());

                if !can_start_new {
                    let has_unmatched_list = (matched..num_open).any(|idx| {
                        matches!(self.open[idx].block_type, OpenBlockType::ListItem { .. })
                    });

                    let cached_marker = if indent <= 3 {
                        parse_list_marker(rest)
                    } else {
                        None
                    };
                    let is_new_list_marker = cached_marker.is_some();

                    if has_unmatched_list && is_new_list_marker {
                    } else if !is_new_list_marker || !has_unmatched_list {
                        if !(cached_marker.map_or(false, |m| can_interrupt_paragraph(&m))) {
                            self.open[tip_idx].content.push('\n');
                            self.open[tip_idx].content_has_newline = true;
                            line.advance_to_nonspace();
                            self.open[tip_idx].content.push_str(line.remainder());
                            return;
                        }
                    }
                }
            }
        }

        while self.open.len() > matched {
            self.close_top_block();
        }

        self.open_new_blocks(line);
    }

    #[inline(never)]
    pub(super) fn open_new_blocks(&mut self, mut line: Line<'a>) {
        loop {
            let (ns_col, ns_off, first_byte) = line.peek_nonspace_col();
            let indent = ns_col - line.col_offset;

            if first_byte == 0 && ns_off >= line.raw.len() {
                let len = self.open.len();
                let mut found_list_item = false;
                for i in (1..len).rev() {
                    if matches!(self.open[i].block_type, OpenBlockType::ListItem { .. }) {
                        self.open[i].had_blank_in_item = true;
                        found_list_item = true;
                        break;
                    }
                }
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

            if indent <= 3 && first_byte == b'>' {
                line.advance_to_nonspace();
                advance_past_blockquote_marker(&mut line);
                self.open.push(OpenBlock::new(OpenBlockType::BlockQuote));
                self.open_blockquotes += 1;
                continue;
            }

            if indent <= 3 {
                let rest = if ns_off >= line.raw.len() {
                    ""
                } else {
                    &line.raw[ns_off..]
                };

                match first_byte {
                    b'0'..=b'9' | b'+' => {
                        if let Some(marker) = parse_list_marker(rest) {
                            let marker_indent = indent;
                            line.advance_to_nonspace();
                            let rest_is_blank =
                                self.start_list_item(&mut line, marker, marker_indent);
                            if rest_is_blank {
                                return;
                            }
                            continue;
                        }
                    }
                    b'-' | b'*' => {
                        if is_thematic_break(rest) {
                            let parent = self.open.last_mut().unwrap();
                            parent.children.push(Block::ThematicBreak);
                            return;
                        }
                        if let Some(marker) = parse_list_marker(rest) {
                            let marker_indent = indent;
                            line.advance_to_nonspace();
                            let rest_is_blank =
                                self.start_list_item(&mut line, marker, marker_indent);
                            if rest_is_blank {
                                return;
                            }
                            continue;
                        }
                    }
                    _ => {
                        if let Some((level, content)) = parse_atx_heading(rest) {
                            line.advance_to_nonspace();
                            let parent = self.open.last_mut().unwrap();
                            parent.children.push(Block::Heading {
                                level,
                                raw: content.to_string(),
                            });
                            return;
                        }
                        if let Some((fence_char, fence_len, info)) = parse_fence_start(rest) {
                            self.open
                                .push(OpenBlock::new(OpenBlockType::FencedCode(Box::new(
                                    FencedCodeData {
                                        fence_char,
                                        fence_len,
                                        fence_indent: indent,
                                        info: resolve_entities_and_escapes(info),
                                    },
                                ))));
                            return;
                        }
                        if let Some(end_condition) = parse_html_block_start(rest, false) {
                            let mut block =
                                OpenBlock::new(OpenBlockType::HtmlBlock { end_condition });
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
                        if is_thematic_break(rest) {
                            let parent = self.open.last_mut().unwrap();
                            parent.children.push(Block::ThematicBreak);
                            return;
                        }
                        if let Some(marker) = parse_list_marker(rest) {
                            let marker_indent = indent;
                            line.advance_to_nonspace();
                            let rest_is_blank =
                                self.start_list_item(&mut line, marker, marker_indent);
                            if rest_is_blank {
                                return;
                            }
                            continue;
                        }
                    }
                }
            } else {
                let tip = self.open.last().unwrap();
                if !matches!(tip.block_type, OpenBlockType::Paragraph) {
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
        let spaces_after = if rest_blank {
            1
        } else {
            let total_sp = ns_col - line.col_offset;
            if total_sp == 0 || total_sp >= 5 {
                1
            } else {
                total_sp
            }
        };

        let content_col = marker_indent + marker.marker_len + spaces_after;

        if !rest_blank {
            let _ = line.skip_indent(spaces_after);
        }

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
        self.list_indent_sum += content_col;
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
                let had_blank = block.had_blank_in_item;
                let kind = block.list_kind.unwrap_or(ListKind::Bullet(b'-'));
                let blank_between_children = had_blank && block.children.len() >= 2;

                let item = Block::ListItem {
                    children: block.children,
                    checked: block.checked,
                };
                let parent = self.open.last_mut().unwrap();

                if had_blank && !blank_between_children {
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
                        if parent.list_has_blank_between {
                            *tight = false;
                        }
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
            OpenBlockType::FencedCode(fc_data) => Some(Block::CodeBlock {
                info: fc_data.info,
                literal: block.content,
            }),
            OpenBlockType::IndentedCode => {
                let mut literal = block.content;
                literal.push('\n');
                let trimmed_len = literal.trim_end_matches('\n').len();
                literal.truncate(trimmed_len + 1); // keep exactly one trailing newline
                Some(Block::CodeBlock {
                    info: String::new(),
                    literal,
                })
            }
            OpenBlockType::HtmlBlock { .. } => {
                let literal = block.content;
                Some(Block::HtmlBlock { literal })
            }
            OpenBlockType::Table(td) => Some(Block::Table {
                alignments: td.alignments,
                header: td.header,
                rows: td.rows,
            }),
            OpenBlockType::Paragraph => {
                if block.content.is_empty() {
                    return None;
                }
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
                if !self.ref_defs.contains_key(&*key) {
                    let resolved_href = resolve_entities_and_escapes(&href);
                    let resolved_title = title.map(|t| resolve_entities_and_escapes(&t));
                    self.ref_defs.insert(
                        key.into_owned(),
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
    #[inline]
    pub(super) fn extract_ref_defs_owned(&mut self, mut content: String) -> String {
        let bytes = content.as_bytes();
        let len = bytes.len();

        if len > 0 && !matches!(bytes[0], b' ' | b'\t' | b'\n' | b'\r' | b'[') {
            if !matches!(bytes[len - 1], b' ' | b'\t' | b'\n' | b'\r') {
                return content;
            }
            let mut end = len;
            while end > 0 && matches!(bytes[end - 1], b' ' | b'\t' | b'\n' | b'\r') {
                end -= 1;
            }
            content.truncate(end);
            return content;
        }

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

        if bytes[start] != b'[' {
            if start == 0 && end == len {
                return content;
            }
            content.truncate(end);
            if start > 0 {
                content.drain(..start);
            }
            return content;
        }
        self.extract_ref_defs(&content[start..end])
    }
}
