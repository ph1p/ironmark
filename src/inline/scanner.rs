use super::*;

impl<'a> InlineScanner<'a> {
    pub(super) fn scan_all(&mut self) {
        let mut text_start = self.pos;

        while self.pos < self.bytes.len() {
            let b = self.bytes[self.pos];
            if SPECIAL[b as usize] == 0 {
                self.pos += 1;
                while self.pos < self.bytes.len() && SPECIAL[self.bytes[self.pos] as usize] == 0 {
                    self.pos += 1;
                }
                continue;
            }

            match b {
                b'\\' => {
                    if self.pos + 1 < self.bytes.len() {
                        let next = self.bytes[self.pos + 1];
                        if next == b'\n' {
                            self.flush_text_range(text_start, self.pos);
                            self.items.push(InlineItem::HardBreak);
                            self.pos += 2;
                            text_start = self.pos;
                            continue;
                        }
                        if is_ascii_punctuation(next) {
                            self.flush_text_range(text_start, self.pos);
                            let escaped: &'static str = match next {
                                b'&' => "&amp;",
                                b'<' => "&lt;",
                                b'>' => "&gt;",
                                b'"' => "&quot;",
                                _ => {
                                    self.items
                                        .push(InlineItem::TextRange(self.pos + 1, self.pos + 2));
                                    self.pos += 2;
                                    text_start = self.pos;
                                    continue;
                                }
                            };
                            self.items.push(InlineItem::TextStatic(escaped));
                            self.pos += 2;
                            text_start = self.pos;
                            continue;
                        }
                    }
                    self.pos += 1;
                }
                b'`' => {
                    self.flush_text_range(text_start, self.pos);
                    self.scan_code_span();
                    text_start = self.pos;
                }
                b'*' | b'_' => {
                    self.flush_text_range(text_start, self.pos);
                    self.scan_delim_run(b);
                    text_start = self.pos;
                }
                b'~' | b'=' | b'+' => {
                    let enabled = match b {
                        b'~' => self.opts.enable_strikethrough,
                        b'=' => self.opts.enable_highlight,
                        b'+' => self.opts.enable_underline,
                        _ => false,
                    };
                    if enabled && self.pos + 1 < self.bytes.len() && self.bytes[self.pos + 1] == b {
                        self.flush_text_range(text_start, self.pos);
                        self.scan_delim_run(b);
                        text_start = self.pos;
                    } else {
                        self.pos += 1;
                    }
                }
                b'!' => {
                    if self.pos + 1 < self.bytes.len() && self.bytes[self.pos + 1] == b'[' {
                        self.flush_text_range(text_start, self.pos);
                        let idx = self.items.len();
                        self.items.push(InlineItem::BracketOpen { is_image: true });
                        self.pos += 2;
                        self.brackets.push(BracketInfo {
                            item_idx: idx,
                            is_image: true,
                            delim_bottom: self.delims.len(),
                            active: true,
                            text_pos: self.pos,
                        });
                        text_start = self.pos;
                    } else {
                        self.pos += 1;
                    }
                }
                b'[' => {
                    self.flush_text_range(text_start, self.pos);
                    let idx = self.items.len();
                    self.items.push(InlineItem::BracketOpen { is_image: false });
                    self.pos += 1;
                    self.brackets.push(BracketInfo {
                        item_idx: idx,
                        is_image: false,
                        delim_bottom: self.delims.len(),
                        active: true,
                        text_pos: self.pos,
                    });
                    text_start = self.pos;
                }
                b']' => {
                    self.flush_text_range(text_start, self.pos);
                    self.pos += 1;
                    self.handle_close_bracket();
                    text_start = self.pos;
                }
                b'<' => {
                    self.flush_text_range(text_start, self.pos);
                    if self.try_autolink() || self.try_html_inline() {
                        text_start = self.pos;
                    } else {
                        self.items.push(InlineItem::TextStatic("&lt;"));
                        self.pos += 1;
                        text_start = self.pos;
                    }
                }
                b'&' => {
                    self.flush_text_range(text_start, self.pos);
                    if self.try_entity() {
                        text_start = self.pos;
                    } else {
                        self.items.push(InlineItem::TextStatic("&amp;"));
                        self.pos += 1;
                        text_start = self.pos;
                    }
                }
                b'\n' => {
                    let is_hard = self.pos >= text_start + 2
                        && self.bytes[self.pos - 1] == b' '
                        && self.bytes[self.pos - 2] == b' ';
                    let mut text_end = self.pos;
                    while text_end > text_start && self.bytes[text_end - 1] == b' ' {
                        text_end -= 1;
                    }
                    self.flush_text_range(text_start, text_end);
                    if is_hard {
                        self.items.push(InlineItem::HardBreak);
                    } else {
                        self.items.push(InlineItem::SoftBreak);
                    }
                    self.pos += 1;
                    text_start = self.pos;
                }
                b':' => {
                    if self.opts.enable_autolink && self.try_bare_url(text_start) {
                        text_start = self.pos;
                    } else {
                        self.pos += 1;
                    }
                }
                b'@' => {
                    if self.opts.enable_autolink && self.try_bare_email(text_start) {
                        text_start = self.pos;
                    } else {
                        self.pos += 1;
                    }
                }
                _ => unreachable!(),
            }
        }
        self.flush_text_range(text_start, self.pos);
    }

    #[inline]
    pub(super) fn flush_text_range(&mut self, start: usize, end: usize) {
        if start < end {
            self.items.push(InlineItem::TextRange(start, end));
        }
    }

    pub(super) fn scan_code_span(&mut self) {
        let start = self.pos;
        let mut open_count: usize = 0;
        while self.pos < self.bytes.len() && self.bytes[self.pos] == b'`' {
            open_count += 1;
            self.pos += 1;
        }
        let after_open = self.pos;

        // Check backtick cache: if we already know no closer of this length exists, bail fast.
        if open_count <= 63 && (self.backtick_no_match >> open_count) & 1 == 1 {
            self.items.push(InlineItem::TextRange(start, after_open));
            self.pos = after_open;
            return;
        }

        loop {
            if let Some(idx) = memchr::memchr(b'`', &self.bytes[self.pos..]) {
                self.pos += idx;
            } else {
                self.pos = self.bytes.len();
            }
            if self.pos >= self.bytes.len() {
                // No matching closer found — cache this length so future runs skip the scan.
                if open_count <= 63 {
                    self.backtick_no_match |= 1u64 << open_count;
                }
                self.items.push(InlineItem::TextRange(start, after_open));
                self.pos = after_open;
                return;
            }
            let close_start = self.pos;
            let mut close_count = 0;
            while self.pos < self.bytes.len() && self.bytes[self.pos] == b'`' {
                close_count += 1;
                self.pos += 1;
            }
            if close_count == open_count {
                let raw = &self.input[after_open..close_start];
                let has_newline = raw.as_bytes().contains(&b'\n');
                if !has_newline {
                    // Fast path: no newline replacement needed. Check stripping.
                    let (s, e) = if raw.len() >= 2
                        && raw.as_bytes()[0] == b' '
                        && raw.as_bytes()[raw.len() - 1] == b' '
                        && !raw.bytes().all(|b| b == b' ')
                    {
                        (after_open + 1, close_start - 1)
                    } else {
                        (after_open, close_start)
                    };
                    self.items.push(InlineItem::CodeRange(s as u32, e as u32));
                } else {
                    // Slow path: replace newlines, then strip.
                    let content = raw.replace('\n', " ");
                    let stripped = if content.len() >= 2
                        && content.as_bytes()[0] == b' '
                        && content.as_bytes()[content.len() - 1] == b' '
                        && !content.bytes().all(|b| b == b' ')
                    {
                        content[1..content.len() - 1].to_string()
                    } else {
                        content
                    };
                    self.items.push(InlineItem::Code(stripped));
                }
                return;
            }
        }
    }

    pub(super) fn scan_delim_run(&mut self, marker: u8) {
        let run_start = self.pos;
        let mut count = 0;
        while self.pos < self.bytes.len() && self.bytes[self.pos] == marker {
            count += 1;
            self.pos += 1;
        }

        let before = if run_start > 0 {
            char_before(self.input, run_start)
        } else {
            ' '
        };
        let after = if self.pos < self.bytes.len() {
            char_at(self.input, self.pos)
        } else {
            ' '
        };
        let (can_open, can_close) = flanking(marker, before, after);

        let idx = self.items.len();
        self.items.push(InlineItem::DelimRun {
            kind: marker,
            count: count as u16,
            can_open,
            can_close,
            open_em: SmallEmVec::new(),
            close_em: SmallEmVec::new(),
        });
        self.delims.push(idx);
    }

    pub(super) fn handle_close_bracket(&mut self) {
        if self.brackets.is_empty() {
            self.items.push(InlineItem::TextStatic("]"));
            return;
        }
        let bi = self.brackets.len() - 1;
        if !self.brackets[bi].active {
            self.brackets.pop();
            self.items.push(InlineItem::TextStatic("]"));
            return;
        }

        let opener_item = self.brackets[bi].item_idx;
        let is_image = self.brackets[bi].is_image;
        let delim_bottom = self.brackets[bi].delim_bottom;
        let text_pos = self.brackets[bi].text_pos;
        let close_pos = self.pos - 1;

        if let Some((dest, title)) = self.try_inline_link() {
            self.resolve_link(bi, is_image, delim_bottom, opener_item, dest, title);
            return;
        }

        if let Some((dest, title)) = self.try_reference_link(text_pos, close_pos) {
            self.resolve_link(bi, is_image, delim_bottom, opener_item, dest, title);
            return;
        }

        self.brackets.pop();
        self.items.push(InlineItem::TextStatic("]"));
    }

    fn resolve_link(
        &mut self,
        bi: usize,
        is_image: bool,
        delim_bottom: usize,
        opener_item: usize,
        dest: LinkDest,
        title: Option<Rc<str>>,
    ) {
        if !is_image {
            for j in 0..bi {
                if !self.brackets[j].is_image {
                    self.brackets[j].active = false;
                }
            }
        }
        self.brackets.truncate(bi);
        self.process_emphasis(delim_bottom);
        let link_idx = self.links.len() as u16;
        self.links.push(LinkInfo {
            dest,
            title,
            is_image,
        });
        self.items[opener_item] = InlineItem::LinkStart(link_idx);
        self.items.push(InlineItem::LinkEnd);
    }

    pub(super) fn skip_ws(&mut self) {
        while self.pos < self.bytes.len() && matches!(self.bytes[self.pos], b' ' | b'\t' | b'\n') {
            self.pos += 1;
        }
    }

    pub(super) fn process_emphasis(&mut self, stack_bottom: usize) {
        let mut closer_di = stack_bottom;
        while closer_di < self.delims.len() {
            let ci = self.delims[closer_di];
            if ci == usize::MAX {
                closer_di += 1;
                continue;
            }
            let (ckind, ccount, ccan_close, ccan_open) = match &self.items[ci] {
                InlineItem::DelimRun {
                    kind,
                    count,
                    can_close,
                    can_open,
                    ..
                } => (*kind, *count, *can_close, *can_open),
                _ => {
                    closer_di += 1;
                    continue;
                }
            };
            if !ccan_close || ccount == 0 {
                closer_di += 1;
                continue;
            }

            let mut found = None;
            let mut odi = closer_di;
            while odi > stack_bottom {
                odi -= 1;
                let oi = self.delims[odi];
                if oi == usize::MAX {
                    continue;
                }
                let (okind, ocount, ocan_open, ocan_close) = match &self.items[oi] {
                    InlineItem::DelimRun {
                        kind,
                        count,
                        can_open,
                        can_close,
                        ..
                    } => (*kind, *count, *can_open, *can_close),
                    _ => continue,
                };
                if okind != ckind || !ocan_open || ocount == 0 {
                    continue;
                }
                if matches!(ckind, b'*' | b'_') {
                    if (ocan_close || ccan_open) && (ocount + ccount) % 3 == 0 {
                        if ocount % 3 != 0 || ccount % 3 != 0 {
                            continue;
                        }
                    }
                }
                if matches!(ckind, b'~' | b'=' | b'+') && (ocount < 2 || ccount < 2) {
                    continue;
                }
                found = Some(odi);
                break;
            }

            let Some(opener_di) = found else {
                closer_di += 1;
                continue;
            };

            let oi = self.delims[opener_di];
            let ci = self.delims[closer_di];

            let ocount = match &self.items[oi] {
                InlineItem::DelimRun { count, .. } => *count,
                _ => 0,
            };
            let ccount = match &self.items[ci] {
                InlineItem::DelimRun { count, .. } => *count,
                _ => 0,
            };

            let is_ext = matches!(ckind, b'~' | b'=' | b'+');
            let use_count: u16 = if ocount >= 2 && ccount >= 2 { 2 } else { 1 };
            let tag_size: u8 = if is_ext {
                match ckind {
                    b'~' => 3, // <del>
                    b'=' => 4, // <mark>
                    b'+' => 5, // <u>
                    _ => use_count as u8,
                }
            } else {
                use_count as u8
            };

            if let InlineItem::DelimRun { count, open_em, .. } = &mut self.items[oi] {
                *count -= use_count;
                open_em.push(tag_size);
            }
            if let InlineItem::DelimRun {
                count, close_em, ..
            } = &mut self.items[ci]
            {
                *count -= use_count;
                close_em.push(tag_size);
            }

            // Mark delimiters between opener and closer for removal (avoid O(n) drain).
            for di in (opener_di + 1)..closer_di {
                self.delims[di] = usize::MAX;
            }

            let new_ocount = match &self.items[self.delims[opener_di]] {
                InlineItem::DelimRun { count, .. } => *count,
                _ => 0,
            };
            if new_ocount == 0 {
                self.delims[opener_di] = usize::MAX;
            }

            let new_ccount = match &self.items[self.delims[closer_di]] {
                InlineItem::DelimRun { count, .. } => *count,
                _ => 0,
            };
            if new_ccount == 0 {
                self.delims[closer_di] = usize::MAX;
            }
            // Don't advance closer_di — the loop will re-check or skip as needed.
        }
        // Compact: remove all marked entries in one pass, then truncate.
        self.delims.retain(|&v| v != usize::MAX);
        self.delims.truncate(stack_bottom);
    }

    fn try_bare_url(&mut self, text_start: usize) -> bool {
        let bytes = self.bytes;
        let len = bytes.len();

        if self.pos + 3 >= len || bytes[self.pos + 1] != b'/' || bytes[self.pos + 2] != b'/' {
            return false;
        }

        let colon_pos = self.pos;
        let scheme_start = if colon_pos >= 5
            && self.input[colon_pos - 5..colon_pos].eq_ignore_ascii_case("https")
        {
            colon_pos - 5
        } else if colon_pos >= 4
            && self.input[colon_pos - 4..colon_pos].eq_ignore_ascii_case("http")
        {
            colon_pos - 4
        } else {
            return false;
        };

        if scheme_start > 0 {
            let prev = bytes[scheme_start - 1];
            if prev.is_ascii_alphanumeric() || prev == b'_' {
                return false;
            }
        }

        let url_body_start = colon_pos + 3;
        if url_body_start >= len {
            return false;
        }

        let first_body = bytes[url_body_start];
        if first_body <= b' ' || first_body == b'<' {
            return false;
        }

        let mut end = url_body_start;
        let mut paren_depth: i32 = 0;
        while end < len {
            let b = bytes[end];
            if b <= b' ' || b == b'<' {
                break;
            }
            if b == b'(' {
                paren_depth += 1;
            } else if b == b')' {
                if paren_depth <= 0 {
                    break;
                }
                paren_depth -= 1;
            }
            end += 1;
        }

        while end > url_body_start {
            let last = bytes[end - 1];
            if matches!(
                last,
                b'.' | b',' | b':' | b';' | b'!' | b'?' | b'"' | b'\'' | b')' | b']'
            ) {
                if last == b')' {
                    let url_slice = &bytes[scheme_start..end];
                    let opens = url_slice.iter().filter(|&&b| b == b'(').count();
                    let closes = url_slice.iter().filter(|&&b| b == b')').count();
                    if closes <= opens {
                        break;
                    }
                }
                end -= 1;
            } else {
                break;
            }
        }

        if end <= url_body_start {
            return false;
        }

        self.flush_text_range(text_start, scheme_start);

        self.items
            .push(InlineItem::Autolink(scheme_start as u32, end as u32, false));

        self.pos = end;
        true
    }

    fn try_bare_email(&mut self, text_start: usize) -> bool {
        let bytes = self.bytes;
        let len = bytes.len();
        let at_pos = self.pos;

        if at_pos == 0 || at_pos + 1 >= len {
            return false;
        }

        let mut local_start = at_pos;
        while local_start > 0 && is_email_local_char(bytes[local_start - 1]) {
            local_start -= 1;
        }

        if local_start == at_pos {
            return false;
        }

        if local_start > 0 {
            let prev = bytes[local_start - 1];
            if prev.is_ascii_alphanumeric() || prev == b'_' {
                return false;
            }
        }

        let domain_start = at_pos + 1;
        let mut end = domain_start;
        while end < len {
            let b = bytes[end];
            if b.is_ascii_alphanumeric() || b == b'-' || b == b'.' {
                end += 1;
            } else {
                break;
            }
        }

        if end == domain_start {
            return false;
        }

        if matches!(bytes[end - 1], b'-' | b'.') {
            return false;
        }

        let domain = &self.input[domain_start..end];
        let last_dot = domain.rfind('.');
        match last_dot {
            None => return false,
            Some(dot_pos) => {
                if dot_pos + 1 >= domain.len() {
                    return false;
                }
            }
        }

        self.flush_text_range(text_start, local_start);

        self.items
            .push(InlineItem::Autolink(local_start as u32, end as u32, true));

        self.pos = end;
        true
    }
}
