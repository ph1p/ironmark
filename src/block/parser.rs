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
    /// Byte range of `s` within `self.input`. `s` MUST be a subslice of the
    /// input (all `Line.raw` slices are) and non-empty — `Line::remainder()`
    /// returns a static `""` past the end, which has no input-relative address.
    #[inline]
    fn src_range_of(&self, s: &str) -> (u32, u32) {
        debug_assert!(!s.is_empty());
        let base = self.input.as_ptr() as usize;
        let start = s.as_ptr() as usize - base;
        debug_assert!(start + s.len() <= self.input.len());
        (start as u32, (start + s.len()) as u32)
    }

    /// Defer leaf text for the renderer. `s` MUST be a direct source slice.
    ///
    /// This is the ONE producer of the deferred-range protocol: in render mode
    /// it pushes the source range of `s` (an empty range when `s` is empty —
    /// the renderer consumes exactly one range per empty `raw`/`literal`, so
    /// skipping the push would desync every later block) and returns the empty
    /// sentinel String. In AST mode it returns an owned copy.
    #[inline]
    fn defer_raw(&mut self, s: &str) -> String {
        if self.render_mode {
            let rng = if s.is_empty() {
                (0, 0)
            } else {
                self.src_range_of(s)
            };
            self.src_ranges.push(rng);
            String::new()
        } else {
            s.to_string()
        }
    }

    /// If the paragraph at `idx` holds a deferred source range (render mode),
    /// copy the bytes into `content` so callers can read/extend it as a String.
    #[inline]
    fn materialize_paragraph(&mut self, idx: usize) {
        debug_assert!(matches!(
            self.open[idx].block_type,
            OpenBlockType::Paragraph
        ));
        if let Some((s, e)) = self.open[idx].src_range.take() {
            let input = self.input;
            let block = &mut self.open[idx];
            debug_assert!(block.content.is_empty());
            block.content.push_str(&input[s as usize..e as usize]);
        }
    }

    /// Append a continuation line to the paragraph at `tip_idx`. In render mode
    /// a paragraph may be a deferred source range; if the new line directly
    /// follows it in the source (separated by exactly one `\n`, nothing
    /// stripped), extend the range instead of copying.
    #[inline]
    fn append_paragraph_line(&mut self, tip_idx: usize, rem: &str) {
        if let Some((s, e)) = self.open[tip_idx].src_range {
            if !rem.is_empty() {
                let (rs, re) = self.src_range_of(rem);
                if rs == e + 1 {
                    let tip = &mut self.open[tip_idx];
                    tip.src_range = Some((s, re));
                    tip.content_has_newline = true;
                    return;
                }
            }
            self.materialize_paragraph(tip_idx);
        }
        let tip = &mut self.open[tip_idx];
        tip.content.reserve(1 + rem.len());
        tip.content.push('\n');
        tip.content_has_newline = true;
        tip.content.push_str(rem);
    }

    /// Build a heading block; text is deferred via [`Self::defer_raw`].
    #[inline]
    fn make_heading(&mut self, level: u8, content: &str) -> Block {
        Block::Heading {
            level,
            raw: self.defer_raw(content),
        }
    }

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
                        let has_leaf_above = num_open > i + 1
                            && matches!(
                                self.open[num_open - 1].block_type,
                                OpenBlockType::Paragraph
                                    | OpenBlockType::FencedCode(..)
                                    | OpenBlockType::IndentedCode
                                    | OpenBlockType::HtmlBlock { .. }
                            );
                        if started_blank
                            && self.open[i].children.is_empty()
                            && self.open[i].content.is_empty()
                            && !has_leaf_above
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

        if (matched == num_open - 1 || matched == num_open) && tip_is_leaf {
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
                    let (ic, _, _) = line.peek_nonspace_col();
                    if ic - line.col_offset >= 4 {
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
                    let (_, ro, _) = line.peek_nonspace_col();
                    let rest = if ro >= line.raw.len() {
                        ""
                    } else {
                        &line.raw[ro..]
                    };
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

                    if self.enable_tables
                        && !self.open[tip_idx].content_has_newline
                        && let Some(alignments) = parse_table_separator(rest)
                    {
                        self.materialize_paragraph(tip_idx);
                        let num_cols = alignments.len();
                        let paragraph_len = self.open[tip_idx].content.len();
                        let header = parse_table_row(&self.open[tip_idx].content, num_cols);
                        if header.len() == num_cols {
                            self.open.pop();
                            self.open.push(OpenBlock::new(OpenBlockType::Table(Box::new(
                                TableData {
                                    alignments,
                                    header,
                                    rows: Vec::with_capacity((paragraph_len / 16).max(8)),
                                },
                            ))));
                            return;
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
                        line.advance_to_nonspace();
                        let rem = line.remainder();
                        self.append_paragraph_line(tip_idx, rem);
                        return;
                    }
                    if indent <= 3 {
                        if let Some(level) = parse_setext_underline(rest) {
                            let tip_range = self.open[tip_idx].src_range.take();
                            let input = self.input;
                            let content = std::mem::take(&mut self.open[tip_idx].content);
                            let content_str: &str = match tip_range {
                                Some((s, e)) => &input[s as usize..e as usize],
                                None => &content,
                            };
                            let remaining = self.extract_ref_defs(content_str);
                            if remaining.is_empty() {
                                self.open.pop();
                                let mut para =
                                    OpenBlock::with_content_capacity(OpenBlockType::Paragraph, 128);
                                para.content.push_str(rest);
                                self.open.push(para);
                                return;
                            }
                            let heading = match remaining {
                                Cow::Borrowed(s) if tip_range.is_some() => {
                                    // Borrowed from the source input (tip_range is only set
                                    // in render mode) — defer instead of copying.
                                    self.make_heading(level, s.trim_end())
                                }
                                Cow::Borrowed(s) => Block::Heading {
                                    level,
                                    raw: s.trim_end().to_string(),
                                },
                                Cow::Owned(mut s) => {
                                    let trimmed_len = s.trim_end().len();
                                    s.truncate(trimmed_len);
                                    Block::Heading { level, raw: s }
                                }
                            };
                            self.open.pop();
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
                        if let Some((level, content)) =
                            parse_atx_heading(rest, self.permissive_atx_headers)
                        {
                            self.close_top_block();
                            let heading = self.make_heading(level, content);
                            let parent = self.open.last_mut().unwrap();
                            parent.children.push(heading);
                            return;
                        }
                        if let Some((fence_char, fence_len, info)) = parse_fence_start(rest) {
                            self.close_top_block();
                            let fc = OpenBlockType::FencedCode(FencedCodeData {
                                fence_char,
                                fence_len,
                                fence_indent: indent,
                                info: CompactString::from(
                                    resolve_entities_and_escapes(info).as_ref(),
                                ),
                            });
                            // In render mode at depth 1 with no indent, bulk_scan_fenced_code
                            // will fire immediately and never write to `content`. Skip the
                            // 128-byte pre-allocation to avoid a malloc that is immediately freed.
                            let block = if self.render_mode && indent == 0 {
                                OpenBlock::new(fc)
                            } else {
                                OpenBlock::with_content_capacity(fc, 128)
                            };
                            self.open.push(block);
                            return;
                        }
                        if let Some(end_condition) = parse_html_block_start(rest, true) {
                            self.close_top_block();
                            let mut block = OpenBlock::with_content_capacity(
                                OpenBlockType::HtmlBlock { end_condition },
                                128,
                            );
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
                        if let Some(marker) = parse_list_marker(rest)
                            && can_interrupt_paragraph(&marker)
                        {
                            self.close_top_block();
                            self.open_new_blocks(line);
                            return;
                        }
                    }
                    line.advance_to_nonspace();
                    let rem = line.remainder();
                    self.append_paragraph_line(tip_idx, rem);
                    return;
                }
                _ => {}
            }
        }

        if !all_matched && !line.is_blank() {
            let tip_idx = self.open.len() - 1;
            if matches!(self.open[tip_idx].block_type, OpenBlockType::Paragraph) {
                let (rc, ro, rb) = line.peek_nonspace_col();
                let rest = if ro >= line.raw.len() {
                    ""
                } else {
                    &line.raw[ro..]
                };
                let indent = rc - line.col_offset;

                // `LEAF_START_BYTE` gates the probe chain: only those bytes can
                // begin a construct that interrupts a lazy continuation, so an
                // ordinary prose line costs one table lookup instead of five
                // scans.
                let can_start_new = indent <= 3
                    && (rb == b'>'
                        || (LEAF_START_BYTE[rb as usize]
                            && (is_thematic_break(rest)
                                || parse_atx_heading(rest, self.permissive_atx_headers)
                                    .is_some()
                                || parse_fence_start(rest).is_some()
                                || (!self.no_html_blocks
                                    && parse_html_block_start(rest, false).is_some()))));

                if !can_start_new {
                    let marker = if indent <= 3 && LEAF_START_BYTE[rb as usize] {
                        parse_list_marker(rest)
                    } else {
                        None
                    };
                    let has_unmatched_list =
                        self.last_list_item_idx.is_some_and(|idx| idx >= matched);
                    let should_break = (has_unmatched_list && marker.is_some())
                        || marker.as_ref().is_some_and(can_interrupt_paragraph);
                    if !should_break {
                        line.advance_to_nonspace();
                        let rem = line.remainder();
                        self.append_paragraph_line(tip_idx, rem);
                        return;
                    }
                }
            }
        }

        while self.open.len() > matched {
            self.close_top_block();
        }

        self.open_new_blocks(line);
    }

    /// Open a new paragraph holding the line's remainder. `line` must already be
    /// advanced past leading whitespace.
    #[inline]
    fn push_paragraph_line(&mut self, line: Line<'a>) {
        let rem = line.remainder();
        let block = if self.render_mode && !rem.is_empty() {
            // Render mode: defer the copy — record the source range and leave
            // `content` empty. Materialised on demand (continuation lines,
            // setext/table conversion) or resolved at render time.
            let mut block = OpenBlock::new(OpenBlockType::Paragraph);
            block.src_range = Some(self.src_range_of(rem));
            block
        } else {
            let mut block = OpenBlock::with_content_capacity(OpenBlockType::Paragraph, 128);
            block.content.push_str(rem);
            block
        };
        self.open.push(block);
    }

    pub(super) fn open_new_blocks(&mut self, mut line: Line<'a>) {
        loop {
            let (ns_col, ns_off, first_byte) = line.peek_nonspace_col();
            let indent = ns_col - line.col_offset;

            if first_byte == 0 && ns_off >= line.raw.len() {
                if let Some(idx) = self.last_list_item_idx {
                    self.open[idx].had_blank_in_item = true;
                } else {
                    let parent = self.open.last_mut().unwrap();
                    if parent
                        .children
                        .last()
                        .is_some_and(|c| matches!(c, Block::List { .. }))
                    {
                        parent.list_has_blank_between = true;
                    }
                }
                return;
            }

            if indent <= 3 && first_byte == b'>' {
                if self.open.len() >= self.max_nesting_depth {
                    // Nesting depth exceeded — treat as paragraph text
                    line.advance_to_nonspace();
                    let mut block = OpenBlock::with_content_capacity(OpenBlockType::Paragraph, 128);
                    block.content.push_str(line.remainder());
                    self.open.push(block);
                    return;
                }
                line.advance_to_nonspace();
                advance_past_blockquote_marker(&mut line);
                self.open.push(OpenBlock::new(OpenBlockType::BlockQuote));
                self.open_blockquotes += 1;
                continue;
            }

            if indent <= 3 {
                // Every leaf-block construct is identified by its first byte.
                // One table lookup skips the whole probe chain (thematic break,
                // list marker, ATX heading, fence, HTML block) for ordinary
                // prose lines, which are the overwhelming majority.
                if !LEAF_START_BYTE[first_byte as usize] {
                    line.advance_to_nonspace();
                    self.push_paragraph_line(line);
                    return;
                }
                let rest = if ns_off >= line.raw.len() {
                    ""
                } else {
                    &line.raw[ns_off..]
                };

                if matches!(first_byte, b'-' | b'*' | b'+' | b'0'..=b'9') {
                    if matches!(first_byte, b'-' | b'*') && is_thematic_break(rest) {
                        let parent = self.open.last_mut().unwrap();
                        parent.children.push(Block::ThematicBreak);
                        return;
                    }
                    if let Some(marker) = parse_list_marker(rest) {
                        if self.open.len() >= self.max_nesting_depth {
                            line.advance_to_nonspace();
                            let mut block =
                                OpenBlock::with_content_capacity(OpenBlockType::Paragraph, 128);
                            block.content.push_str(line.remainder());
                            self.open.push(block);
                            return;
                        }
                        let marker_indent = indent;
                        line.advance_to_nonspace();
                        let rest_is_blank = self.start_list_item(&mut line, marker, marker_indent);
                        if rest_is_blank {
                            return;
                        }
                        continue;
                    }
                }
                if matches!(first_byte, b'_') && is_thematic_break(rest) {
                    let parent = self.open.last_mut().unwrap();
                    parent.children.push(Block::ThematicBreak);
                    return;
                }
                if let Some((level, content)) = parse_atx_heading(rest, self.permissive_atx_headers)
                {
                    line.advance_to_nonspace();
                    let heading = self.make_heading(level, content);
                    let parent = self.open.last_mut().unwrap();
                    parent.children.push(heading);
                    return;
                }
                if let Some((fence_char, fence_len, info)) = parse_fence_start(rest) {
                    self.open.push(OpenBlock::with_content_capacity(
                        OpenBlockType::FencedCode(FencedCodeData {
                            fence_char,
                            fence_len,
                            fence_indent: indent,
                            info: CompactString::from(resolve_entities_and_escapes(info).as_ref()),
                        }),
                        64,
                    ));
                    return;
                }
                if !self.no_html_blocks
                    && let Some(end_condition) = parse_html_block_start(rest, false)
                {
                    let mut block = OpenBlock::with_content_capacity(
                        OpenBlockType::HtmlBlock { end_condition },
                        128,
                    );
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
                if let Some(marker) = parse_list_marker(rest) {
                    if self.open.len() >= self.max_nesting_depth {
                        line.advance_to_nonspace();
                        let mut block =
                            OpenBlock::with_content_capacity(OpenBlockType::Paragraph, 128);
                        block.content.push_str(line.remainder());
                        self.open.push(block);
                        return;
                    }
                    let marker_indent = indent;
                    line.advance_to_nonspace();
                    let rest_is_blank = self.start_list_item(&mut line, marker, marker_indent);
                    if rest_is_blank {
                        return;
                    }
                    continue;
                }
            } else if self.enable_indented_code_blocks {
                let tip = self.open.last().unwrap();
                if !matches!(tip.block_type, OpenBlockType::Paragraph) {
                    let _ = line.skip_indent(4);
                    let content = line.remainder_with_partial();
                    let mut block =
                        OpenBlock::with_content_capacity(OpenBlockType::IndentedCode, 128);
                    block.content.push_str(&content);
                    self.open.push(block);
                    return;
                }
            }

            self.push_paragraph_line(line);
            return;
        }
    }

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

        let mut item = OpenBlock::new_list_item(content_col, rest_blank);
        item.list_kind = Some(list_kind);
        item.list_start = marker.start_num;
        item.checked = checked;
        self.list_indent_sum += content_col;
        self.open.push(item);
        self.last_list_item_idx = Some(self.open.len() - 1);
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

                if had_blank
                    && !blank_between_children
                    && matches!(parent.block_type, OpenBlockType::ListItem { .. })
                {
                    parent.had_blank_in_item = true;
                }

                if let Some(Block::List {
                    kind: lk,
                    children: items,
                    tight,
                    ..
                }) = parent.children.last_mut()
                    && *lk == kind
                {
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

                parent.list_has_blank_between = had_blank;

                // Lists almost always gain siblings, and `vec![item]` would start
                // at capacity 1 and then double on every push. One small
                // allocation up front removes that regrowth chain.
                let mut items = Vec::with_capacity(4);
                items.push(item);
                let list = Block::List {
                    kind,
                    start: block.list_start,
                    tight: !blank_between_children,
                    children: items,
                };
                Some(list)
            }
            OpenBlockType::FencedCode(fc_data) => {
                if self.render_mode {
                    // Render-mode fast path: content is a direct source slice (or empty) —
                    // record the range and emit the empty-literal sentinel. The empty case
                    // MUST also push (an empty range) or the renderer's index desyncs and
                    // steals the next block's range.
                    if block.src_range.is_some() || block.content.is_empty() {
                        self.src_ranges.push(block.src_range.unwrap_or((0, 0)));
                        return Some(Block::CodeBlock {
                            info: fc_data.info,
                            literal: String::new(),
                        });
                    }
                } else if let Some((start, end)) = block.src_range {
                    // AST mode (parse_markdown): always materialise the literal so callers
                    // get a fully-populated Block with no empty strings.
                    let content = &self.input[start as usize..end as usize];
                    return Some(Block::CodeBlock {
                        info: fc_data.info,
                        literal: content.to_owned(),
                    });
                }
                Some(Block::CodeBlock {
                    info: fc_data.info,
                    literal: block.content,
                })
            }
            OpenBlockType::IndentedCode => {
                let mut literal = block.content;
                literal.push('\n');
                let trimmed_len = literal.trim_end_matches('\n').len();
                literal.truncate(trimmed_len + 1); // keep exactly one trailing newline
                Some(Block::CodeBlock {
                    info: CompactString::default(),
                    literal,
                })
            }
            OpenBlockType::HtmlBlock { .. } => Some(Block::HtmlBlock {
                literal: block.content,
            }),
            OpenBlockType::Table(td) => {
                let num_cols = td.alignments.len();
                let total_cells = td.rows.iter().map(SmallVec::len).sum();
                let mut rows_flat = Vec::with_capacity(total_cells);
                for row in td.rows {
                    rows_flat.extend(row);
                }
                Some(Block::Table(Box::new(crate::ast::TableData {
                    alignments: td.alignments.into_vec(),
                    num_cols,
                    header: td.header.into_vec(),
                    rows: rows_flat,
                })))
            }
            OpenBlockType::Paragraph => {
                if let Some((s, e)) = block.src_range {
                    // Render-mode zero-copy paragraph (ranges are only set on
                    // paragraphs in render mode).
                    debug_assert!(self.render_mode);
                    let input = self.input;
                    let slice = &input[s as usize..e as usize];
                    if !matches!(slice.as_bytes()[0], b' ' | b'\t' | b'\n' | b'\r' | b'[') {
                        // No leading ws / no possible ref def: trim trailing ws by
                        // shrinking the slice — defer_raw keeps it zero-copy.
                        let trimmed = slice.trim_end_matches([' ', '\t', '\n', '\r']);
                        return Some(Block::Paragraph {
                            raw: self.defer_raw(trimmed),
                        });
                    }
                    // Leading ws or '[': run ref-def extraction on the source slice.
                    return match self.extract_ref_defs(slice) {
                        // Still a direct source slice — keep zero-copy.
                        Cow::Borrowed(t) if !t.is_empty() => Some(Block::Paragraph {
                            raw: self.defer_raw(t),
                        }),
                        Cow::Owned(o) if !o.is_empty() => Some(Block::Paragraph { raw: o }),
                        _ => None,
                    };
                }
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

    pub(super) fn extract_ref_defs<'c>(&mut self, content: &'c str) -> Cow<'c, str> {
        // Large def blocks (many `[label]: url` lines) would otherwise rehash the
        // map ~log(n) times; count line-leading '[' once and reserve up front.
        // Small blocks (a handful of defs) skip the extra scan — the threshold just
        // needs to be past the size where a rehash outweighs one memchr pass.
        if content.len() > 512 {
            let bytes = content.as_bytes();
            let defs = memchr::memchr_iter(b'\n', bytes)
                .filter(|&p| bytes.get(p + 1) == Some(&b'['))
                .count()
                + 1;
            self.ref_defs.reserve(defs);
        }
        let mut pos = 0;
        loop {
            let trimmed = content[pos..].trim_start();
            if !trimmed.starts_with('[') {
                break;
            }
            if let Some((label, href, title, consumed)) = parse_link_ref_def(trimmed) {
                let key = crate::inline::normalize_reference_label(label);
                if !self.ref_defs.contains_key(&*key) {
                    let resolved_href: std::rc::Rc<str> =
                        resolve_entities_and_escapes(&href).into();
                    let resolved_title = title
                        .map(|t| -> std::rc::Rc<str> { resolve_entities_and_escapes(&t).into() });
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
        let remaining = content[pos..].trim();
        if pos == 0 && remaining.len() == content.len() {
            // No ref defs extracted and no trimming needed — return borrowed
            Cow::Borrowed(content)
        } else {
            Cow::Owned(remaining.to_string())
        }
    }

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
        self.extract_ref_defs(&content[start..end]).into_owned()
    }
}

/// Bytes that can begin a leaf-block construct at an indent of ≤3: thematic
/// break (`-*_`), list marker (`-*+` and digits), ATX heading (`#`), code fence
/// (`` ` `` / `~`) and HTML block (`<`). Any other first byte can only start a
/// paragraph, so the whole probe chain in [`BlockParser::open_new_blocks`] is
/// skipped for it.
static LEAF_START_BYTE: [bool; 256] = {
    let mut t = [false; 256];
    let starters = b"-*_+#`~<";
    let mut i = 0;
    while i < starters.len() {
        t[starters[i] as usize] = true;
        i += 1;
    }
    let mut d = b'0';
    while d <= b'9' {
        t[d as usize] = true;
        d += 1;
    }
    t
};
