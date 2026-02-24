use super::*;
use crate::ParseOptions;

impl<'a> InlineScanner<'a> {
    pub(super) fn render_to_html(&self, out: &mut String, opts: &ParseOptions) {
        let mut tag_buf: [u8; 16] = [0; 16];
        let mut tag_len: usize = 0;
        let mut i = 0;

        while i < self.items.len() {
            match &self.items[i] {
                InlineItem::TextRange(start, end) => {
                    escape_html_into(out, &self.input[*start..*end]);
                }
                InlineItem::TextOwned(t) => {
                    out.push_str(t);
                }
                InlineItem::TextStatic(t) => {
                    out.push_str(t);
                }
                InlineItem::TextInline { buf, len } => {
                    out.push_str(unsafe { std::str::from_utf8_unchecked(&buf[..*len as usize]) });
                }
                InlineItem::RawHtml(start, end) => {
                    out.push_str(&self.input[*start..*end]);
                }
                InlineItem::Autolink(start, end, is_email) => {
                    let content = &self.input[*start as usize..*end as usize];
                    out.push_str("<a href=\"");
                    if *is_email {
                        out.push_str("mailto:");
                    }
                    crate::html::encode_url_escaped_into(out, content);
                    out.push_str("\">");
                    escape_html_into(out, content);
                    out.push_str("</a>");
                }
                InlineItem::Code(c) => {
                    out.push_str("<code>");
                    out.push_str(c);
                    out.push_str("</code>");
                }
                InlineItem::HardBreak => out.push_str("<br />\n"),
                InlineItem::SoftBreak => {
                    if opts.hard_breaks {
                        out.push_str("<br />\n");
                    } else {
                        out.push('\n');
                    }
                }
                InlineItem::DelimRun {
                    kind,
                    count,
                    open_em,
                    close_em,
                    ..
                } => {
                    for &size in close_em.as_slice() {
                        if tag_len > 0 && tag_buf[tag_len - 1] == size {
                            tag_len -= 1;
                            match size {
                                2 => out.push_str("</strong>"),
                                3 => out.push_str("</del>"),
                                4 => out.push_str("</mark>"),
                                5 => out.push_str("</u>"),
                                _ => out.push_str("</em>"),
                            }
                        }
                    }

                    if *count > 0 {
                        for _ in 0..*count {
                            out.push(*kind as char);
                        }
                    }

                    for &size in open_em.as_slice().iter().rev() {
                        if tag_len < 16 {
                            tag_buf[tag_len] = size;
                            tag_len += 1;
                        }
                        match size {
                            2 => out.push_str("<strong>"),
                            3 => out.push_str("<del>"),
                            4 => out.push_str("<mark>"),
                            5 => out.push_str("<u>"),
                            _ => out.push_str("<em>"),
                        }
                    }
                }
                InlineItem::BracketOpen { is_image, .. } => {
                    if *is_image {
                        out.push_str("![");
                    } else {
                        out.push('[');
                    }
                }
                InlineItem::LinkStart(link_idx) => {
                    let LinkInfo {
                        dest,
                        title,
                        is_image,
                    } = &self.links[*link_idx as usize];
                    if *is_image {
                        let alt_start = i + 1;
                        let mut alt_end = alt_start;
                        let mut depth = 1;
                        while alt_end < self.items.len() {
                            match &self.items[alt_end] {
                                InlineItem::LinkStart(..) => depth += 1,
                                InlineItem::LinkEnd => {
                                    depth -= 1;
                                    if depth == 0 {
                                        break;
                                    }
                                }
                                _ => {}
                            }
                            alt_end += 1;
                        }
                        let alt = self.collect_alt_text(alt_start, alt_end);
                        out.push_str("<img src=\"");
                        write_link_dest(out, dest, self.input);
                        out.push_str("\" alt=\"");
                        out.push_str(&alt);
                        out.push('"');
                        if let Some(t) = title {
                            out.push_str(" title=\"");
                            escape_html_into(out, t);
                            out.push('"');
                        }
                        out.push_str(" />");
                        i = alt_end;
                    } else {
                        out.push_str("<a href=\"");
                        write_link_dest(out, dest, self.input);
                        out.push('"');
                        if let Some(t) = title {
                            out.push_str(" title=\"");
                            escape_html_into(out, t);
                            out.push('"');
                        }
                        out.push('>');
                        if tag_len < 16 {
                            tag_buf[tag_len] = 0;
                            tag_len += 1;
                        }
                    }
                }
                InlineItem::LinkEnd => {
                    if tag_len > 0 && tag_buf[tag_len - 1] == 0 {
                        tag_len -= 1;
                        out.push_str("</a>");
                    }
                }
            }
            i += 1;
        }
    }

    pub(super) fn collect_alt_text(&self, start: usize, end: usize) -> String {
        let mut s = String::new();
        for idx in start..end {
            match &self.items[idx] {
                InlineItem::TextRange(a, b) => {
                    s.push_str(&self.input[*a..*b]);
                }
                InlineItem::TextOwned(t) => s.push_str(t),
                InlineItem::TextStatic(t) => s.push_str(t),
                InlineItem::TextInline { buf, len } => {
                    s.push_str(unsafe { std::str::from_utf8_unchecked(&buf[..*len as usize]) });
                }
                InlineItem::Code(c) => s.push_str(c),
                InlineItem::DelimRun { kind, count, .. } => {
                    for _ in 0..*count {
                        s.push(*kind as char);
                    }
                }
                InlineItem::RawHtml(_, _) | InlineItem::Autolink(..) => {}
                InlineItem::HardBreak | InlineItem::SoftBreak => {}
                InlineItem::LinkStart(_) => {}
                InlineItem::LinkEnd => {}
                InlineItem::BracketOpen { is_image, .. } => {
                    if *is_image {
                        s.push_str("![");
                    } else {
                        s.push('[');
                    }
                }
            }
        }
        s
    }
}

#[inline]
pub(super) fn write_link_dest(out: &mut String, dest: &LinkDest, input: &str) {
    match dest {
        LinkDest::Range(s, e) => {
            let s = *s as usize;
            let e = *e as usize;
            if s < e {
                crate::html::encode_url_escaped_into(out, &input[s..e]);
            }
        }
        LinkDest::Owned(d) => {
            crate::html::encode_url_escaped_into(out, d);
        }
    }
}
