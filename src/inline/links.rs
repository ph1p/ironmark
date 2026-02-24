use super::*;

impl<'a> InlineScanner<'a> {
    pub(super) fn try_inline_link(&mut self) -> Option<(LinkDest, Option<String>)> {
        if self.pos >= self.bytes.len() || self.bytes[self.pos] != b'(' {
            return None;
        }
        let saved = self.pos;
        self.pos += 1;
        self.skip_ws();

        if self.pos < self.bytes.len() && self.bytes[self.pos] == b')' {
            self.pos += 1;
            return Some((LinkDest::Range(0, 0), None));
        }

        let dest = if self.pos < self.bytes.len() && self.bytes[self.pos] == b'<' {
            match self.scan_angle_dest() {
                Some(d) => LinkDest::Owned(d),
                None => {
                    self.pos = saved;
                    return None;
                }
            }
        } else {
            match self.scan_bare_dest() {
                Some(d) => d,
                None => {
                    self.pos = saved;
                    return None;
                }
            }
        };

        self.skip_ws();

        let mut title = None;
        if self.pos < self.bytes.len() && matches!(self.bytes[self.pos], b'"' | b'\'' | b'(') {
            title = self.scan_link_title();
            if title.is_none() {
                self.pos = saved;
                return None;
            }
            self.skip_ws();
        }

        if self.pos >= self.bytes.len() || self.bytes[self.pos] != b')' {
            self.pos = saved;
            return None;
        }
        self.pos += 1;
        Some((dest, title))
    }

    pub(super) fn scan_angle_dest(&mut self) -> Option<String> {
        self.pos += 1;
        let mut dest = String::new();
        while self.pos < self.bytes.len() {
            let b = self.bytes[self.pos];
            if b == b'>' {
                self.pos += 1;
                return Some(dest);
            }
            if b == b'<' || b == b'\n' {
                return None;
            }
            if b == b'\\'
                && self.pos + 1 < self.bytes.len()
                && is_ascii_punctuation(self.bytes[self.pos + 1])
            {
                dest.push(self.bytes[self.pos + 1] as char);
                self.pos += 2;
            } else if b == b'&' {
                if !self.resolve_entity_into(&mut dest) {
                    dest.push('&');
                    self.pos += 1;
                }
            } else {
                let cs = self.pos;
                self.pos += utf8_char_len(b);
                dest.push_str(&self.input[cs..self.pos]);
            }
        }
        None
    }

    pub(super) fn scan_bare_dest(&mut self) -> Option<LinkDest> {
        let start = self.pos;
        let mut has_special = false;
        let mut end = self.pos;
        while end < self.bytes.len() {
            let b = self.bytes[end];
            if b <= 0x20 {
                break;
            }
            if b == b')' || b == b'(' || b == b'\\' || b == b'&' {
                has_special = true;
                break;
            }
            end += 1;
        }
        if !has_special {
            self.pos = end;
            return Some(LinkDest::Range(start as u32, end as u32));
        }

        let mut dest = String::with_capacity(end - start + 8);
        if end > start {
            dest.push_str(&self.input[start..end]);
            self.pos = end;
        }
        let mut paren_depth = 0i32;
        while self.pos < self.bytes.len() {
            let b = self.bytes[self.pos];
            if b <= 0x20 {
                break;
            }
            if b == b'(' {
                paren_depth += 1;
                if paren_depth > 32 {
                    return None;
                }
                dest.push('(');
                self.pos += 1;
            } else if b == b')' {
                if paren_depth == 0 {
                    break;
                }
                paren_depth -= 1;
                dest.push(')');
                self.pos += 1;
            } else if b == b'\\'
                && self.pos + 1 < self.bytes.len()
                && is_ascii_punctuation(self.bytes[self.pos + 1])
            {
                dest.push(self.bytes[self.pos + 1] as char);
                self.pos += 2;
            } else if b == b'&' {
                if !self.resolve_entity_into(&mut dest) {
                    dest.push('&');
                    self.pos += 1;
                }
            } else {
                let cs = self.pos;
                self.pos += utf8_char_len(b);
                dest.push_str(&self.input[cs..self.pos]);
            }
        }
        if paren_depth != 0 {
            return None;
        }
        Some(LinkDest::Owned(dest))
    }

    pub(super) fn scan_link_title(&mut self) -> Option<String> {
        let q = self.bytes[self.pos];
        let cq = match q {
            b'"' => b'"',
            b'\'' => b'\'',
            b'(' => b')',
            _ => return None,
        };
        self.pos += 1;
        let mut title = String::new();
        while self.pos < self.bytes.len() {
            let b = self.bytes[self.pos];
            if b == cq && q != b'(' {
                self.pos += 1;
                return Some(title);
            }
            if b == b')' && q == b'(' {
                self.pos += 1;
                return Some(title);
            }
            if b == b'(' && q == b'(' {
                return None;
            }
            if b == b'\\'
                && self.pos + 1 < self.bytes.len()
                && is_ascii_punctuation(self.bytes[self.pos + 1])
            {
                title.push(self.bytes[self.pos + 1] as char);
                self.pos += 2;
            } else if b == b'&' {
                if !self.resolve_entity_into(&mut title) {
                    title.push('&');
                    self.pos += 1;
                }
            } else {
                let cs = self.pos;
                self.pos += utf8_char_len(b);
                title.push_str(&self.input[cs..self.pos]);
            }
        }
        None
    }

    pub(super) fn try_reference_link(
        &mut self,
        text_pos: usize,
        close_pos: usize,
    ) -> Option<(LinkDest, Option<String>)> {
        let saved = self.pos;
        let raw_label = &self.input[text_pos..close_pos];

        if self.pos < self.bytes.len() && self.bytes[self.pos] == b'[' {
            self.pos += 1;
            let label_start = self.pos;
            let mut depth = 0i32;
            while self.pos < self.bytes.len() {
                if self.bytes[self.pos] == b'\\' && self.pos + 1 < self.bytes.len() {
                    self.pos += 2;
                    continue;
                }
                if self.bytes[self.pos] == b'[' {
                    depth += 1;
                }
                if self.bytes[self.pos] == b']' {
                    if depth == 0 {
                        let label = &self.input[label_start..self.pos];
                        self.pos += 1;
                        let lookup = if label.trim().is_empty() {
                            raw_label
                        } else {
                            label
                        };
                        let key = normalize_reference_label(lookup);
                        if let Some(r) = self.refs.get(&*key) {
                            return Some((LinkDest::Owned(r.href.clone()), r.title.clone()));
                        }
                        self.pos = saved;
                        return None;
                    }
                    depth -= 1;
                }
                self.pos += 1;
            }
            self.pos = saved;
        }

        if self.refs.is_empty() {
            return None;
        }
        let key = normalize_reference_label(raw_label);
        if let Some(r) = self.refs.get(&*key) {
            if self.pos + 1 < self.bytes.len()
                && self.bytes[self.pos] == b'['
                && self.bytes[self.pos + 1] == b']'
            {
                self.pos += 2;
            }
            return Some((LinkDest::Owned(r.href.clone()), r.title.clone()));
        }

        None
    }

    pub(super) fn try_autolink(&mut self) -> bool {
        let start = self.pos;
        self.pos += 1;
        let content_start = self.pos;
        while self.pos < self.bytes.len() && self.bytes[self.pos] != b'>' {
            if self.bytes[self.pos] == b' '
                || self.bytes[self.pos] == b'\n'
                || self.bytes[self.pos] == b'<'
            {
                self.pos = start;
                return false;
            }
            self.pos += 1;
        }
        if self.pos >= self.bytes.len() {
            self.pos = start;
            return false;
        }
        let content = &self.input[content_start..self.pos];
        self.pos += 1;

        if let Some(colon) = content.find(':') {
            let scheme = &content[..colon];
            if scheme.len() >= 2
                && scheme.len() <= 32
                && scheme.as_bytes()[0].is_ascii_alphabetic()
                && scheme
                    .bytes()
                    .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'+' | b'.' | b'-'))
            {
                self.items
                    .push(InlineItem::RawHtmlOwned(build_autolink_html("", content)));
                return true;
            }
        }

        if is_email_autolink(content) {
            self.items
                .push(InlineItem::RawHtmlOwned(build_autolink_html(
                    "mailto:", content,
                )));
            return true;
        }

        self.pos = start;
        false
    }

    pub(super) fn try_html_inline(&mut self) -> bool {
        let rest = &self.input[self.pos..];
        let rest_start = self.pos;

        if rest.starts_with("<!--") {
            if rest.starts_with("<!-->") {
                self.items
                    .push(InlineItem::RawHtml(rest_start, rest_start + 5));
                self.pos += 5;
                return true;
            }
            if rest.starts_with("<!--->") {
                self.items
                    .push(InlineItem::RawHtml(rest_start, rest_start + 6));
                self.pos += 6;
                return true;
            }
            if let Some(end) = rest[4..].find("-->") {
                let tag_len = end + 7;
                self.items
                    .push(InlineItem::RawHtml(rest_start, rest_start + tag_len));
                self.pos += tag_len;
                return true;
            }
        }

        if rest.starts_with("<?") {
            if let Some(end) = rest[2..].find("?>") {
                let tag_len = end + 4;
                self.items
                    .push(InlineItem::RawHtml(rest_start, rest_start + tag_len));
                self.pos += tag_len;
                return true;
            }
        }

        if rest.starts_with("<![CDATA[") {
            if let Some(end) = rest[9..].find("]]>") {
                let tag_len = end + 12;
                self.items
                    .push(InlineItem::RawHtml(rest_start, rest_start + tag_len));
                self.pos += tag_len;
                return true;
            }
        }

        let bytes = rest.as_bytes();
        if bytes.len() > 2
            && bytes[1] == b'!'
            && bytes.get(2).map_or(false, |b| b.is_ascii_alphabetic())
        {
            if let Some(end) = rest.find('>') {
                let tag_len = end + 1;
                self.items
                    .push(InlineItem::RawHtml(rest_start, rest_start + tag_len));
                self.pos += tag_len;
                return true;
            }
        }

        if bytes.len() < 3 {
            return false;
        }
        let is_close = bytes[1] == b'/';
        let tstart = if is_close { 2 } else { 1 };
        if tstart >= bytes.len() || !bytes[tstart].is_ascii_alphabetic() {
            return false;
        }

        let mut i = tstart + 1;
        while i < bytes.len() && (bytes[i].is_ascii_alphanumeric() || bytes[i] == b'-') {
            i += 1;
        }

        if is_close {
            while i < bytes.len() && (bytes[i] == b' ' || bytes[i] == b'\t') {
                i += 1;
            }
            if i < bytes.len() && bytes[i] == b'>' {
                i += 1;
                self.items
                    .push(InlineItem::RawHtml(rest_start, rest_start + i));
                self.pos += i;
                return true;
            }
            return false;
        }

        loop {
            let had_space = {
                let before = i;
                while i < bytes.len() && matches!(bytes[i], b' ' | b'\t' | b'\n') {
                    i += 1;
                }
                i > before
            };
            if i >= bytes.len() {
                return false;
            }
            if bytes[i] == b'>' {
                i += 1;
                self.items
                    .push(InlineItem::RawHtml(rest_start, rest_start + i));
                self.pos += i;
                return true;
            }
            if bytes[i] == b'/' {
                i += 1;
                if i < bytes.len() && bytes[i] == b'>' {
                    i += 1;
                    self.items
                        .push(InlineItem::RawHtml(rest_start, rest_start + i));
                    self.pos += i;
                    return true;
                }
                return false;
            }
            if !had_space {
                return false;
            }
            if !(bytes[i].is_ascii_alphabetic() || bytes[i] == b'_' || bytes[i] == b':') {
                return false;
            }
            while i < bytes.len()
                && (bytes[i].is_ascii_alphanumeric()
                    || matches!(bytes[i], b'_' | b':' | b'.' | b'-'))
            {
                i += 1;
            }
            let si = i;
            while i < bytes.len() && matches!(bytes[i], b' ' | b'\t' | b'\n') {
                i += 1;
            }
            if i < bytes.len() && bytes[i] == b'=' {
                i += 1;
                while i < bytes.len() && matches!(bytes[i], b' ' | b'\t' | b'\n') {
                    i += 1;
                }
                if i >= bytes.len() {
                    return false;
                }
                if bytes[i] == b'\'' || bytes[i] == b'"' {
                    let q = bytes[i];
                    i += 1;
                    while i < bytes.len() && bytes[i] != q {
                        i += 1;
                    }
                    if i >= bytes.len() {
                        return false;
                    }
                    i += 1;
                } else {
                    if matches!(
                        bytes[i],
                        b' ' | b'\t' | b'"' | b'\'' | b'=' | b'<' | b'>' | b'`'
                    ) {
                        return false;
                    }
                    while i < bytes.len()
                        && !matches!(
                            bytes[i],
                            b' ' | b'\t' | b'\n' | b'"' | b'\'' | b'=' | b'<' | b'>' | b'`'
                        )
                    {
                        i += 1;
                    }
                }
            } else {
                i = si;
            }
        }
    }

    pub(super) fn try_entity(&mut self) -> bool {
        let start = self.pos;
        self.pos += 1;
        if self.pos >= self.bytes.len() {
            self.pos = start;
            return false;
        }

        let mut char_buf: [u8; 8] = [0; 8];
        let mut char_len = 0usize;

        let ok = if self.bytes[self.pos] == b'#' {
            self.pos += 1;
            let hex = self.pos < self.bytes.len() && matches!(self.bytes[self.pos], b'x' | b'X');
            if hex {
                self.pos += 1;
            }
            let ns = self.pos;
            if hex {
                while self.pos < self.bytes.len() && self.bytes[self.pos].is_ascii_hexdigit() {
                    self.pos += 1;
                }
            } else {
                while self.pos < self.bytes.len() && self.bytes[self.pos].is_ascii_digit() {
                    self.pos += 1;
                }
            }
            if self.pos == ns
                || self.pos - ns > 7
                || self.pos >= self.bytes.len()
                || self.bytes[self.pos] != b';'
            {
                false
            } else {
                self.pos += 1;
                let value = &self.input[ns..self.pos - 1];
                let cp = if hex {
                    u32::from_str_radix(value, 16).ok()
                } else {
                    value.parse::<u32>().ok()
                };
                if let Some(mut cp) = cp {
                    if cp == 0 {
                        cp = 0xFFFD;
                    }
                    let c = char::from_u32(cp).unwrap_or('\u{FFFD}');
                    char_len = c.encode_utf8(&mut char_buf).len();
                    true
                } else {
                    false
                }
            }
        } else {
            let ns = self.pos;
            while self.pos < self.bytes.len() && self.bytes[self.pos].is_ascii_alphanumeric() {
                self.pos += 1;
            }
            if self.pos == ns || self.pos >= self.bytes.len() || self.bytes[self.pos] != b';' {
                false
            } else {
                let name = &self.input[ns..self.pos];
                self.pos += 1;
                let found = entities::lookup_entity_codepoints(name);
                if let Some(codepoints) = found {
                    let mut off = 0usize;
                    for &cp in codepoints {
                        if let Some(c) = char::from_u32(cp) {
                            let n = c.encode_utf8(&mut char_buf[off..]).len();
                            off += n;
                        }
                    }
                    char_len = off;
                    true
                } else {
                    false
                }
            }
        };

        if ok {
            if char_len == 1 {
                match char_buf[0] {
                    b'&' => {
                        self.items.push(InlineItem::TextStatic("&amp;"));
                        return true;
                    }
                    b'<' => {
                        self.items.push(InlineItem::TextStatic("&lt;"));
                        return true;
                    }
                    b'>' => {
                        self.items.push(InlineItem::TextStatic("&gt;"));
                        return true;
                    }
                    b'"' => {
                        self.items.push(InlineItem::TextStatic("&quot;"));
                        return true;
                    }
                    _ => {
                        self.items.push(InlineItem::TextStatic(
                            ASCII_CHAR_STRS[char_buf[0] as usize],
                        ));
                        return true;
                    }
                }
            }
            let needs_escape = char_buf[..char_len]
                .iter()
                .any(|&b| matches!(b, b'&' | b'<' | b'>' | b'"'));
            if needs_escape {
                let resolved = unsafe { std::str::from_utf8_unchecked(&char_buf[..char_len]) };
                let mut s = String::with_capacity(char_len + 8);
                escape_html_into(&mut s, resolved);
                self.items.push(InlineItem::TextOwned(s));
            } else {
                self.items.push(InlineItem::TextInline {
                    buf: char_buf,
                    len: char_len as u8,
                });
            }
            true
        } else {
            self.pos = start;
            false
        }
    }

    pub(super) fn resolve_entity_into(&mut self, dest: &mut String) -> bool {
        let start = self.pos;
        self.pos += 1;
        if self.pos >= self.bytes.len() {
            self.pos = start;
            return false;
        }

        if self.bytes[self.pos] == b'#' {
            self.pos += 1;
            let hex = self.pos < self.bytes.len() && matches!(self.bytes[self.pos], b'x' | b'X');
            if hex {
                self.pos += 1;
            }
            let ns = self.pos;
            if hex {
                while self.pos < self.bytes.len() && self.bytes[self.pos].is_ascii_hexdigit() {
                    self.pos += 1;
                }
            } else {
                while self.pos < self.bytes.len() && self.bytes[self.pos].is_ascii_digit() {
                    self.pos += 1;
                }
            }
            if self.pos == ns
                || self.pos - ns > 7
                || self.pos >= self.bytes.len()
                || self.bytes[self.pos] != b';'
            {
                self.pos = start;
                return false;
            }
            self.pos += 1;
            let value = &self.input[ns..self.pos - 1];
            let cp = if hex {
                u32::from_str_radix(value, 16).ok()
            } else {
                value.parse::<u32>().ok()
            };
            if let Some(mut cp) = cp {
                if cp == 0 {
                    cp = 0xFFFD;
                }
                let c = char::from_u32(cp).unwrap_or('\u{FFFD}');
                let mut buf = [0u8; 4];
                dest.push_str(c.encode_utf8(&mut buf));
                true
            } else {
                self.pos = start;
                false
            }
        } else {
            let ns = self.pos;
            while self.pos < self.bytes.len() && self.bytes[self.pos].is_ascii_alphanumeric() {
                self.pos += 1;
            }
            if self.pos == ns || self.pos >= self.bytes.len() || self.bytes[self.pos] != b';' {
                self.pos = start;
                return false;
            }
            let name = &self.input[ns..self.pos];
            self.pos += 1;
            if entities::lookup_entity_into(name, dest) {
                true
            } else {
                self.pos = start;
                false
            }
        }
    }
}
