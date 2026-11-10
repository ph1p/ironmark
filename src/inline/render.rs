use super::*;
use crate::ParseOptions;
use crate::html::{collapse_and_escape_into, gfm_tag_is_filtered, is_dangerous_url};

static EM_CLOSE: [&str; 6] = ["</em>", "</em>", "</strong>", "</del>", "</mark>", "</u>"];
static EM_OPEN: [&str; 6] = ["<em>", "<em>", "<strong>", "<del>", "<mark>", "<u>"];

impl<'a> InlineScanner<'a> {
    pub(super) fn render_to_html(&self, out: &mut String, opts: &ParseOptions) {
        let mut tag_buf: [u8; 16] = [0; 16];
        let mut tag_len: usize = 0;
        let mut i = 0;

        while i < self.items.len() {
            match &self.items[i] {
                InlineItem::TextRange(start, end) => {
                    let s = &self.input[*start..*end];
                    if opts.collapse_whitespace {
                        collapse_and_escape_into(out, s);
                    } else {
                        escape_html_into(out, s);
                    }
                }
                InlineItem::TextOwned(t) => {
                    if opts.collapse_whitespace {
                        collapse_and_escape_into(out, t);
                    } else {
                        out.push_str(t);
                    }
                }
                InlineItem::TextStatic(t) => out.push_str(t),
                InlineItem::TextInline { buf, len } => {
                    if let Ok(s) = std::str::from_utf8(&buf[..*len as usize]) {
                        out.push_str(s);
                    }
                }
                InlineItem::RawHtml(start, end) => {
                    let html = &self.input[*start..*end];
                    let escape_it = opts.disable_raw_html
                        || opts.no_html_spans
                        || (opts.tag_filter && gfm_tag_is_filtered(html));
                    if escape_it {
                        escape_html_into(out, html);
                    } else {
                        out.push_str(html);
                    }
                }
                InlineItem::MdxJsx(start, end, kind) => {
                    self.push_mdx_piece(out, opts, kind.marker(false), *start, *end);
                }
                InlineItem::MdxExpr(start, end) => {
                    self.push_mdx_piece(out, opts, 'e', *start, *end);
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
                    escape_html_into(out, c);
                    out.push_str("</code>");
                }
                InlineItem::CodeRange(s, e) => {
                    out.push_str("<code>");
                    escape_html_into(out, &self.input[*s as usize..*e as usize]);
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
                            out.push_str(EM_CLOSE[size as usize]);
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
                        out.push_str(EM_OPEN[size as usize]);
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
                    let safe = !is_dangerous_link_dest(dest, self.input);
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
                        if safe {
                            write_link_dest(out, dest, self.input);
                        }
                        out.push_str("\" alt=\"");
                        out.push_str(&alt);
                        out.push('"');
                        if let Some(t) = title {
                            out.push_str(" title=\"");
                            write_link_title(out, t, self.input);
                            out.push('"');
                        }
                        out.push_str(" />");
                        i = alt_end;
                    } else {
                        out.push_str("<a href=\"");
                        if safe {
                            write_link_dest(out, dest, self.input);
                        }
                        out.push('"');
                        if let Some(t) = title {
                            out.push_str(" title=\"");
                            write_link_title(out, t, self.input);
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
                InlineItem::WikiLink(start, end) => {
                    let text = &self.input[*start..*end];
                    out.push_str("<a href=\"");
                    // GitHub-wiki style slug: spaces → underscores, lowercase, percent-encode rest
                    let mut slug = String::with_capacity(text.len());
                    for c in text.chars() {
                        if c == ' ' {
                            slug.push('_');
                        } else {
                            for lc in c.to_lowercase() {
                                slug.push(lc);
                            }
                        }
                    }
                    crate::html::encode_url_escaped_into(out, &slug);
                    out.push_str("\">");
                    escape_html_into(out, text);
                    out.push_str("</a>");
                }
                InlineItem::MathInline(start, end) => {
                    out.push_str("<span class=\"math-inline\">");
                    escape_html_into(out, &self.input[*start..*end]);
                    out.push_str("</span>");
                }
                InlineItem::MathDisplay(start, end) => {
                    out.push_str("<span class=\"math-display\">");
                    escape_html_into(out, &self.input[*start..*end]);
                    out.push_str("</span>");
                }
            }
            i += 1;
        }
    }

    pub(super) fn collect_alt_text(&self, start: usize, end: usize) -> String {
        let mut s = String::with_capacity((end - start) * 4);
        for idx in start..end {
            match &self.items[idx] {
                InlineItem::TextRange(a, b) => s.push_str(&self.input[*a..*b]),
                InlineItem::CodeRange(a, b) => s.push_str(&self.input[*a as usize..*b as usize]),
                InlineItem::TextOwned(t) | InlineItem::Code(t) => s.push_str(t),
                InlineItem::TextStatic(t) => s.push_str(t),
                InlineItem::TextInline { buf, len } => {
                    if let Ok(st) = std::str::from_utf8(&buf[..*len as usize]) {
                        s.push_str(st);
                    }
                }
                InlineItem::DelimRun { kind, count, .. } => {
                    for _ in 0..*count {
                        s.push(*kind as char);
                    }
                }
                InlineItem::BracketOpen { is_image: true, .. } => s.push_str("!["),
                InlineItem::BracketOpen { .. } => s.push('['),
                _ => {}
            }
        }
        s
    }

    /// Emit one inline MDX piece: sentinel-wrapped for the JSX converter, or
    /// raw in HTML mode (escaped under `disable_raw_html`).
    #[cold]
    fn push_mdx_piece(
        &self,
        out: &mut String,
        opts: &ParseOptions,
        marker: char,
        start: usize,
        end: usize,
    ) {
        let lit = &self.input[start..end];
        if self.mdx_sentinels {
            crate::mdx::push_sentinel(out, marker, lit);
        } else {
            crate::mdx::push_raw(out, lit, opts.disable_raw_html);
        }
    }
}

/// Check if a LinkDest points to a dangerous URL without allocating.
#[inline]
fn is_dangerous_link_dest(dest: &LinkDest, input: &str) -> bool {
    match dest {
        LinkDest::Range(s, e) => {
            let s = *s as usize;
            let e = *e as usize;
            if s < e {
                is_dangerous_url(&input[s..e])
            } else {
                false
            }
        }
        LinkDest::Owned(d) => is_dangerous_url(d),
    }
}

#[inline]
fn write_link_title(out: &mut String, title: &LinkTitle, input: &str) {
    match title {
        LinkTitle::Range(s, e) => escape_html_into(out, &input[*s as usize..*e as usize]),
        LinkTitle::Owned(t) => escape_html_into(out, t),
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
