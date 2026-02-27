use super::*;

impl<'a> InlineScanner<'a> {
    pub(super) fn try_inline_link(&mut self) -> Option<(LinkDest, Option<Rc<str>>)> {
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
                Some(d) => LinkDest::Owned(d.into()),
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

        let mut title: Option<Rc<str>> = None;
        if self.pos < self.bytes.len() && matches!(self.bytes[self.pos], b'"' | b'\'' | b'(') {
            match self.scan_link_title() {
                Some(t) => title = Some(t.into()),
                None => {
                    self.pos = saved;
                    return None;
                }
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
            // ')' ends an inline destination but does not require the slow path.
            if b == b')' {
                break;
            }
            if b == b'(' || b == b'\\' || b == b'&' {
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
        Some(LinkDest::Owned(dest.into()))
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
            if b == cq {
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
    ) -> Option<(LinkDest, Option<Rc<str>>)> {
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
                self.items.push(InlineItem::Autolink(
                    content_start as u32,
                    self.pos as u32 - 1,
                    false,
                ));
                return true;
            }
        }

        {
            let eb = content.as_bytes();
            if let Some(at) = eb.iter().position(|&b| b == b'@')
                && at > 0
                && at + 1 < eb.len()
                && eb[..at].iter().all(|&b| is_email_local_char(b))
                && eb[at + 1..]
                    .iter()
                    .all(|&b| b.is_ascii_alphanumeric() || b == b'-' || b == b'.')
            {
                self.items.push(InlineItem::Autolink(
                    content_start as u32,
                    self.pos as u32 - 1,
                    true,
                ));
                return true;
            }
        }

        self.pos = start;
        false
    }

    fn emit_raw_html(&mut self, len: usize) -> bool {
        let s = self.pos;
        self.items.push(InlineItem::RawHtml(s, s + len));
        self.pos += len;
        true
    }

    pub(super) fn try_html_inline(&mut self) -> bool {
        let rest = &self.input[self.pos..];
        let bytes = rest.as_bytes();

        if let Some(rest2) = rest.strip_prefix("<!--") {
            if rest.starts_with("<!-->") {
                return self.emit_raw_html(5);
            }
            if rest.starts_with("<!--->") {
                return self.emit_raw_html(6);
            }
            if let Some(end) = rest2.find("-->") {
                return self.emit_raw_html(end + 7);
            }
        }

        if let Some(rest2) = rest.strip_prefix("<?")
            && let Some(end) = rest2.find("?>")
        {
            return self.emit_raw_html(end + 4);
        }

        if let Some(rest2) = rest.strip_prefix("<![CDATA[")
            && let Some(end) = rest2.find("]]>")
        {
            return self.emit_raw_html(end + 12);
        }

        if bytes.len() > 2
            && bytes[1] == b'!'
            && bytes[2].is_ascii_alphabetic()
            && let Some(end) = rest.find('>')
        {
            return self.emit_raw_html(end + 1);
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
            while i < bytes.len() && matches!(bytes[i], b' ' | b'\t') {
                i += 1;
            }
            return if i < bytes.len() && bytes[i] == b'>' {
                self.emit_raw_html(i + 1)
            } else {
                false
            };
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
                return self.emit_raw_html(i + 1);
            }
            if bytes[i] == b'/' {
                return if i + 1 < bytes.len() && bytes[i + 1] == b'>' {
                    self.emit_raw_html(i + 2)
                } else {
                    false
                };
            }
            if !had_space
                || !(bytes[i].is_ascii_alphabetic() || bytes[i] == b'_' || bytes[i] == b':')
            {
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

    #[inline]
    fn parse_entity_ref(&mut self, buf: &mut [u8; 8]) -> Option<u8> {
        let start = self.pos;
        let bytes = self.bytes;
        let len = bytes.len();
        self.pos += 1;
        if self.pos >= len {
            self.pos = start;
            return None;
        }

        if bytes[self.pos] == b'#' {
            self.pos += 1;
            if self.pos >= len {
                self.pos = start;
                return None;
            }
            let hex = matches!(bytes[self.pos], b'x' | b'X');
            if hex {
                self.pos += 1;
            }
            let ns = self.pos;
            let mut cp: u32 = 0;
            if hex {
                while self.pos < len {
                    let b = bytes[self.pos];
                    let digit = match b {
                        b'0'..=b'9' => (b - b'0') as u32,
                        b'a'..=b'f' => (b - b'a' + 10) as u32,
                        b'A'..=b'F' => (b - b'A' + 10) as u32,
                        _ => break,
                    };
                    cp = cp.wrapping_mul(16).wrapping_add(digit);
                    self.pos += 1;
                }
            } else {
                while self.pos < len {
                    let b = bytes[self.pos];
                    if !b.is_ascii_digit() {
                        break;
                    }
                    cp = cp.wrapping_mul(10).wrapping_add((b - b'0') as u32);
                    self.pos += 1;
                }
            }
            let ndigits = self.pos - ns;
            if ndigits == 0 || ndigits > 7 || self.pos >= len || bytes[self.pos] != b';' {
                self.pos = start;
                return None;
            }
            self.pos += 1;
            if cp == 0 {
                cp = 0xFFFD;
            }
            let c = char::from_u32(cp).unwrap_or('\u{FFFD}');
            let n = c.encode_utf8(&mut buf[..]).len();
            Some(n as u8)
        } else {
            let ns = self.pos;
            let first = bytes[ns];
            if !first.is_ascii_alphabetic() {
                self.pos = start;
                return None;
            }
            let max_len = entities::MAX_ENTITY_LEN[first as usize] as usize;
            if max_len == 0 {
                self.pos = start;
                return None;
            }
            let limit = if len - ns > max_len + 1 {
                ns + max_len + 1
            } else {
                len
            };
            self.pos += 1; // skip first (already validated as alpha)
            while self.pos < limit && bytes[self.pos].is_ascii_alphanumeric() {
                self.pos += 1;
            }
            if self.pos >= len || bytes[self.pos] != b';' {
                self.pos = start;
                return None;
            }
            // SAFETY: `ns..self.pos` is validated to be within `self.input` by cursor bounds checks above.
            let name = unsafe { self.input.get_unchecked(ns..self.pos) };
            self.pos += 1;
            if let Some((cp1, cp2)) = entities::lookup_entity_codepoints(name) {
                let mut off = 0usize;
                if let Some(c) = char::from_u32(cp1) {
                    off += c.encode_utf8(&mut buf[off..]).len();
                }
                if cp2 != 0
                    && let Some(c) = char::from_u32(cp2)
                {
                    off += c.encode_utf8(&mut buf[off..]).len();
                }
                Some(off as u8)
            } else {
                self.pos = start;
                None
            }
        }
    }

    #[inline]
    pub(super) fn try_entity(&mut self) -> bool {
        let bytes = self.bytes;
        let len = bytes.len();
        let start = self.pos;
        if start + 2 < len && bytes[start + 1] != b'#' {
            let result = match bytes[start + 1] {
                b'a' if start + 4 < len
                    && bytes[start + 2] == b'm'
                    && bytes[start + 3] == b'p'
                    && bytes[start + 4] == b';' =>
                {
                    Some(("&amp;", 5))
                }
                b'l' if start + 3 < len && bytes[start + 2] == b't' && bytes[start + 3] == b';' => {
                    Some(("&lt;", 4))
                }
                b'g' if start + 3 < len && bytes[start + 2] == b't' && bytes[start + 3] == b';' => {
                    Some(("&gt;", 4))
                }
                b'n' if start + 5 < len
                    && bytes[start + 2] == b'b'
                    && bytes[start + 3] == b's'
                    && bytes[start + 4] == b'p'
                    && bytes[start + 5] == b';' =>
                {
                    Some(("\u{a0}", 6))
                }
                b'q' if start + 5 < len
                    && bytes[start + 2] == b'u'
                    && bytes[start + 3] == b'o'
                    && bytes[start + 4] == b't'
                    && bytes[start + 5] == b';' =>
                {
                    Some(("&quot;", 6))
                }
                _ => None,
            };
            if let Some((text, advance)) = result {
                self.pos += advance;
                self.items.push(InlineItem::TextStatic(text));
                return true;
            }
        }

        let mut char_buf = [0u8; 8];
        let Some(char_len) = self.parse_entity_ref(&mut char_buf) else {
            return false;
        };
        let char_len = char_len as usize;

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
                    self.items.push(InlineItem::TextInline {
                        buf: char_buf,
                        len: 1,
                    });
                    return true;
                }
            }
        }
        let needs_escape = char_buf[..char_len]
            .iter()
            .any(|&b| matches!(b, b'&' | b'<' | b'>' | b'"'));
        if needs_escape {
            // SAFETY: `char_buf[..char_len]` is produced by UTF-8 encoding from scalar values.
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
    }

    pub(super) fn resolve_entity_into(&mut self, dest: &mut String) -> bool {
        let mut buf = [0u8; 8];
        let Some(len) = self.parse_entity_ref(&mut buf) else {
            return false;
        };
        // SAFETY: `parse_entity_ref` writes valid UTF-8 bytes into `buf[..len]`.
        dest.push_str(unsafe { std::str::from_utf8_unchecked(&buf[..len as usize]) });
        true
    }
}
