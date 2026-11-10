//! Plain-text rendering, heading and link extraction.
//!
//! Inline content goes through the real inline parser (rendered to HTML, then
//! reduced to text), so emphasis, links, code spans, entities and escapes come
//! out exactly as a reader of the rendered HTML would see them.

use crate::ParseOptions;
use crate::ast::Block;
use crate::block::{BlockParser, prepare_input};
use crate::entities::decode_entities;
use crate::inline::{InlineBuffers, LinkRefMap, parse_inline_pass};
use crate::render::heading_slug_into;

/// A heading found by [`extract_headings`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HeadingInfo {
    /// Heading level 1–6.
    pub level: u8,
    /// The heading as plain text.
    pub text: String,
    /// The slug [`render_html`](crate::render_html) uses for `id` when
    /// [`enable_heading_ids`](ParseOptions::enable_heading_ids) is set.
    /// Duplicate headings get the same id, as in the HTML renderer.
    pub id: String,
}

/// A link or image found by [`extract_links`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LinkInfo {
    /// `true` for an image, `false` for a link.
    pub image: bool,
    /// The destination as rendered: references resolved, entities decoded,
    /// percent-encoding applied. Empty when the URL was stripped as dangerous.
    pub url: String,
    /// The title, if one was given.
    pub title: Option<String>,
    /// A link's text or an image's `alt`, as plain text.
    pub text: String,
}

/// Render Markdown as plain text.
///
/// Inline markup is removed, entities and escapes are resolved, and hard line
/// breaks become `\n`. Blocks are separated by a blank line, items of a tight
/// list by a single newline, and table cells by tabs. Raw HTML blocks, MDX
/// blocks and thematic breaks are dropped.
///
/// # Examples
///
/// ```
/// use ironmark::{render_text, ParseOptions};
///
/// let text = render_text("# Hi *there*\n\na &lt; b &copy;", &ParseOptions::default());
/// assert_eq!(text, "Hi there\n\na < b ©");
/// ```
pub fn render_text(markdown: &str, options: &ParseOptions) -> String {
    with_renderer(markdown, options, |doc, r| {
        let mut out = String::with_capacity(markdown.len());
        r.block(doc, &mut out);
        out
    })
}

/// Collect every heading in document order, including headings nested in
/// blockquotes and lists.
///
/// # Examples
///
/// ```
/// use ironmark::{extract_headings, ParseOptions};
///
/// let headings = extract_headings("# Hello *world*\n\n## Next", &ParseOptions::default());
/// assert_eq!(headings[0].text, "Hello world");
/// assert_eq!(headings[0].id, "hello-world");
/// assert_eq!(headings[1].level, 2);
/// ```
pub fn extract_headings(markdown: &str, options: &ParseOptions) -> Vec<HeadingInfo> {
    with_renderer(markdown, options, |doc, r| {
        let mut headings = Vec::new();
        for_each_block(doc, &mut |block| {
            if let Block::Heading { level, raw } = block {
                let mut text = String::new();
                r.render_inline(raw);
                html_to_text(&r.html, &mut text);
                headings.push(heading_info(*level, raw, text));
            }
        });
        headings
    })
}

/// Collect every link and image in document order: inline, reference, wiki
/// and auto links, plus `<a href>` and `<img src>` written as raw inline HTML.
///
/// # Examples
///
/// ```
/// use ironmark::{extract_links, ParseOptions};
///
/// let md = "[Docs](https://x.y \"Read\") ![logo](a.png)\n\n[ref]\n\n[ref]: /r";
/// let links = extract_links(md, &ParseOptions::default());
/// assert_eq!(links[0].url, "https://x.y");
/// assert_eq!(links[0].title.as_deref(), Some("Read"));
/// assert_eq!(links[0].text, "Docs");
/// assert!(links[1].image);
/// assert_eq!(links[2].url, "/r");
/// ```
pub fn extract_links(markdown: &str, options: &ParseOptions) -> Vec<LinkInfo> {
    with_renderer(markdown, options, |doc, r| {
        let mut links = Vec::new();
        for_each_block(doc, &mut |block| match block {
            Block::Paragraph { raw } | Block::Heading { raw, .. } => {
                r.render_inline(raw);
                scan_links(&r.html, &mut links);
            }
            Block::Table(table) => {
                for cell in table.header.iter().chain(&table.rows) {
                    r.render_inline(cell);
                    scan_links(&r.html, &mut links);
                }
            }
            _ => {}
        });
        links
    })
}

/// Plain text, headings, links and front matter of one document, as returned
/// by [`analyze`].
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Analysis {
    /// Same as [`render_text`].
    pub text: String,
    /// Same as [`extract_headings`].
    pub headings: Vec<HeadingInfo>,
    /// Same as [`extract_links`].
    pub links: Vec<LinkInfo>,
    /// The raw front matter (see [`split_frontmatter`](crate::split_frontmatter)),
    /// only when [`enable_frontmatter`](ParseOptions::enable_frontmatter) is set.
    pub frontmatter: Option<String>,
}

/// [`render_text`], [`extract_headings`] and [`extract_links`] in one pass:
/// the document is parsed once and each block's inline content is rendered
/// once, instead of once per call.
///
/// # Examples
///
/// ```
/// use ironmark::{analyze, ParseOptions};
///
/// let doc = analyze("# Hi\n\nSee [docs](/d).", &ParseOptions::default());
/// assert_eq!(doc.text, "Hi\n\nSee docs.");
/// assert_eq!(doc.headings[0].id, "hi");
/// assert_eq!(doc.links[0].url, "/d");
/// assert_eq!(doc.frontmatter, None);
/// ```
pub fn analyze(markdown: &str, options: &ParseOptions) -> Analysis {
    let frontmatter = options
        .enable_frontmatter
        .then(|| crate::split_frontmatter(markdown))
        .flatten()
        .map(|(raw, _)| raw.to_owned());
    with_renderer(markdown, options, |doc, r| {
        r.headings = Some(Vec::new());
        r.links = Some(Vec::new());
        let mut text = String::with_capacity(markdown.len());
        r.block(doc, &mut text);
        Analysis {
            text,
            headings: r.headings.take().unwrap_or_default(),
            links: r.links.take().unwrap_or_default(),
            frontmatter,
        }
    })
}

/// Parse `markdown` and hand the document plus a renderer over its link
/// references to `f`.
fn with_renderer<T>(
    markdown: &str,
    options: &ParseOptions,
    f: impl FnOnce(&Block, &mut TextRenderer) -> T,
) -> T {
    let mut parser = BlockParser::new(prepare_input(markdown, options, usize::MAX), options);
    let doc = parser.parse();
    f(&doc, &mut TextRenderer::new(&parser.ref_defs, options))
}

/// Visit `block` and every block nested in it, in document order.
fn for_each_block(block: &Block, f: &mut impl FnMut(&Block)) {
    f(block);
    if let Block::Document { children }
    | Block::BlockQuote { children }
    | Block::List { children, .. }
    | Block::ListItem { children, .. } = block
    {
        for child in children {
            for_each_block(child, f);
        }
    }
}

fn heading_info(level: u8, raw: &str, text: String) -> HeadingInfo {
    let mut id = String::new();
    heading_slug_into(&mut id, raw);
    HeadingInfo { level, text, id }
}

struct TextRenderer<'a> {
    refs: &'a LinkRefMap,
    opts: &'a ParseOptions,
    bufs: InlineBuffers,
    html: String,
    /// Collected while rendering text when set (see [`analyze`]).
    headings: Option<Vec<HeadingInfo>>,
    links: Option<Vec<LinkInfo>>,
}

impl<'a> TextRenderer<'a> {
    fn new(refs: &'a LinkRefMap, opts: &'a ParseOptions) -> Self {
        let mut bufs = InlineBuffers::new();
        bufs.prepare(opts);
        Self {
            refs,
            opts,
            bufs,
            html: String::new(),
            headings: None,
            links: None,
        }
    }

    /// Render one block's inline content to HTML in the reused `html` buffer.
    fn render_inline(&mut self, raw: &str) {
        self.html.clear();
        parse_inline_pass(&mut self.html, raw, self.refs, self.opts, &mut self.bufs);
    }

    /// Append the inline content as text, collecting its links if asked to.
    fn inline(&mut self, raw: &str, out: &mut String) {
        self.render_inline(raw);
        if let Some(links) = &mut self.links {
            scan_links(&self.html, links);
        }
        html_to_text(&self.html, out);
    }

    fn blocks(&mut self, blocks: &[Block], separator: &str, out: &mut String) {
        let mut wrote = false;
        for block in blocks {
            let start = out.len();
            if wrote {
                out.push_str(separator);
            }
            let body = out.len();
            self.block(block, out);
            if out.len() == body {
                out.truncate(start); // Dropped block: no separator either.
            } else {
                wrote = true;
            }
        }
    }

    fn block(&mut self, block: &Block, out: &mut String) {
        match block {
            Block::Paragraph { raw } => self.inline(raw, out),
            Block::Heading { level, raw } => {
                let start = out.len();
                self.inline(raw, out);
                if let Some(headings) = &mut self.headings {
                    headings.push(heading_info(*level, raw, out[start..].to_owned()));
                }
            }
            Block::CodeBlock { literal, .. } => {
                out.push_str(literal.strip_suffix('\n').unwrap_or(literal));
            }
            Block::Document { children }
            | Block::BlockQuote { children }
            | Block::ListItem { children, .. } => self.blocks(children, "\n\n", out),
            Block::List {
                tight, children, ..
            } => self.blocks(children, if *tight { "\n" } else { "\n\n" }, out),
            Block::Table(table) => {
                let cols = table.num_cols.max(1);
                let rows = std::iter::once(&table.header[..]).chain(table.rows.chunks(cols));
                for (i, row) in rows.enumerate() {
                    if i > 0 {
                        out.push('\n');
                    }
                    for (j, cell) in row.iter().enumerate() {
                        if j > 0 {
                            out.push('\t');
                        }
                        self.inline(cell, out);
                    }
                }
            }
            Block::HtmlBlock { .. }
            | Block::MdxFlow { .. }
            | Block::MdxEsm { .. }
            | Block::ThematicBreak => {}
        }
    }
}

/// Collect the `<a href>` and `<img src>` tags of inline-renderer HTML.
fn scan_links(html: &str, found: &mut Vec<LinkInfo>) {
    let mut at = 0;
    while let Some(open) = memchr::memchr(b'<', &html.as_bytes()[at..]) {
        let start = at + open + 1;
        let Some(len) = memchr::memchr(b'>', &html.as_bytes()[start..]) else {
            break;
        };
        let tag = &html[start..start + len];
        at = start + len + 1;
        let decoded = |v: &str| decode_entities(v).into_owned();
        let title = || attribute(tag, "title").map(decoded);
        if tag.starts_with("a ")
            && let Some(href) = attribute(tag, "href")
        {
            let end = html[at..].find("</a>").map_or(html.len(), |n| at + n);
            let mut text = String::new();
            html_to_text(&html[at..end], &mut text);
            found.push(LinkInfo {
                image: false,
                url: decoded(href),
                title: title(),
                text,
            });
        } else if tag.starts_with("img ")
            && let Some(src) = attribute(tag, "src")
        {
            found.push(LinkInfo {
                image: true,
                url: decoded(src),
                title: title(),
                text: attribute(tag, "alt").map(decoded).unwrap_or_default(),
            });
        }
    }
}

/// Reduce inline-renderer HTML to the text it displays: tags dropped (an image
/// keeps its `alt`), `<br />` becomes `\n`, entities decoded, other whitespace
/// collapsed to single spaces.
fn html_to_text(html: &str, out: &mut String) {
    let mut raw = String::with_capacity(html.len());
    let bytes = html.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        match bytes[i] {
            b'<' => {
                let Some(len) = memchr::memchr(b'>', &bytes[i..]) else {
                    raw.push_str(&html[i..]);
                    break;
                };
                let tag = &html[i + 1..i + len];
                if tag.starts_with("br") {
                    raw.push('\n');
                    // The renderer follows `<br />` with a newline; it is the same break.
                    if bytes.get(i + len + 1) == Some(&b'\n') {
                        i += 1;
                    }
                } else if tag.starts_with("img")
                    && let Some(alt) = attribute(tag, "alt")
                {
                    raw.push_str(alt);
                }
                i += len + 1;
            }
            b'\n' => {
                raw.push(' ');
                i += 1;
            }
            _ => {
                let end = memchr::memchr2(b'<', b'\n', &bytes[i..]).map_or(bytes.len(), |n| i + n);
                raw.push_str(&html[i..end]);
                i = end;
            }
        }
    }

    let start = out.len();
    let mut pending_space = false;
    for ch in decode_entities(&raw).chars() {
        if ch == '\n' {
            pending_space = false;
            out.push('\n');
        } else if ch.is_whitespace() {
            pending_space = true;
        } else {
            if pending_space && out.len() > start && !out.ends_with('\n') {
                out.push(' ');
            }
            pending_space = false;
            out.push(ch);
        }
    }
    while out.len() > start && out.ends_with('\n') {
        out.pop();
    }
}

/// The raw (still entity-encoded) value of a double-quoted attribute.
fn attribute<'h>(tag: &'h str, name: &str) -> Option<&'h str> {
    let mut rest = tag;
    while let Some(at) = rest.find(name) {
        let after = &rest[at + name.len()..];
        let boundary = rest[..at].ends_with(' ');
        if boundary && let Some(value) = after.strip_prefix("=\"") {
            return value.find('"').map(|end| &value[..end]);
        }
        rest = after;
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    fn text(md: &str) -> String {
        render_text(md, &ParseOptions::default())
    }

    #[test]
    fn strips_inline_markup() {
        assert_eq!(text("**bold** *em* ~~del~~ `a < b`"), "bold em del a < b");
        assert_eq!(
            text("[link](https://x.y) and ![logo](a.png)"),
            "link and logo"
        );
        assert_eq!(text("[ref]\n\n[ref]: https://x.y"), "ref");
    }

    #[test]
    fn keeps_literal_angle_brackets_and_entities() {
        assert_eq!(text("a < b > c"), "a < b > c");
        assert_eq!(text("&copy; &amp; &#x41; \\*"), "© & A *");
        assert_eq!(text("x <span>y</span> z"), "x y z");
    }

    #[test]
    fn line_breaks() {
        assert_eq!(text("one\\\ntwo"), "one\ntwo");
        let soft = ParseOptions {
            hard_breaks: false,
            ..ParseOptions::default()
        };
        assert_eq!(render_text("one\ntwo", &soft), "one two");
        assert_eq!(render_text("one  \ntwo", &soft), "one\ntwo");
    }

    #[test]
    fn block_layout() {
        let md = "# T\n\n- a\n- b\n\n> q\n\n```\ncode\n```\n\n---\n\n<div>x</div>\n\n| h1 | h2 |\n|---|---|\n| *1* | 2 |";
        assert_eq!(text(md), "T\n\na\nb\n\nq\n\ncode\n\nh1\th2\n1\t2");
        assert_eq!(text("- a\n\n- b"), "a\n\nb");
        assert_eq!(text(""), "");
    }

    #[test]
    fn headings_match_renderer_ids() {
        let md = "# Hello *World*\n\n> ## Nested `code`\n\n- ### In list\n\n# Hello *World*";
        let found = extract_headings(md, &ParseOptions::default());
        let ids: Vec<_> = found
            .iter()
            .map(|h| (h.level, h.text.as_str(), h.id.as_str()))
            .collect();
        assert_eq!(
            ids,
            [
                (1, "Hello World", "hello-world"),
                (2, "Nested code", "nested-code"),
                (3, "In list", "in-list"),
                (1, "Hello World", "hello-world"),
            ]
        );
        let html = crate::render_html(
            "# Hello *World*",
            &ParseOptions {
                enable_heading_ids: true,
                ..ParseOptions::default()
            },
        );
        assert!(html.contains("id=\"hello-world\""));
    }

    #[test]
    fn links_and_images() {
        let md = "# [*Hi*](/h)\n\n[![a &amp; b](i.png \"T\")](/x) <https://auto.dev> [bad](javascript:x)\n\n| <a href=\"/raw\">r</a> |\n|---|\n\n`[no](/code)`";
        let found: Vec<_> = extract_links(md, &ParseOptions::default())
            .into_iter()
            .map(|l| (l.image, l.url, l.title, l.text))
            .collect();
        let s = |v: &str| v.to_string();
        assert_eq!(
            found,
            [
                (false, s("/h"), None, s("Hi")),
                (false, s("/x"), None, s("a & b")),
                (true, s("i.png"), Some(s("T")), s("a & b")),
                (false, s("https://auto.dev"), None, s("https://auto.dev")),
                (false, s(""), None, s("bad")),
                (false, s("/raw"), None, s("r")),
            ]
        );
    }

    #[test]
    fn analyze_matches_the_single_extractors() {
        let md = "+++\na = 1\n+++\n# [*Hi*](/h)\n\n> ## Q ![i](i.png)\n\n- [x](/x)\n\n| [c](/c) |\n|---|\n\n<div>[no](/no)</div>\n\n---\n\n# Hi";
        for enable_frontmatter in [false, true] {
            let opts = ParseOptions {
                enable_frontmatter,
                ..ParseOptions::default()
            };
            let all = analyze(md, &opts);
            assert_eq!(all.text, render_text(md, &opts));
            assert_eq!(all.headings, extract_headings(md, &opts));
            assert_eq!(all.links, extract_links(md, &opts));
            assert_eq!(
                all.frontmatter.as_deref(),
                enable_frontmatter.then_some("a = 1\n")
            );
        }
    }

    #[test]
    fn frontmatter_is_skipped_when_enabled() {
        let md = "---\ntitle: x\n---\n# Hi";
        let on = ParseOptions {
            enable_frontmatter: true,
            ..ParseOptions::default()
        };
        assert_eq!(render_text(md, &on), "Hi");
        assert_eq!(crate::render_html(md, &on), "<h1>Hi</h1>\n");
        assert!(crate::render_html(md, &ParseOptions::default()).contains("<hr />"));
    }
}
