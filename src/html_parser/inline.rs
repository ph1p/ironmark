//! Inline HTML element handling and conversion to Markdown syntax.
//!
//! This module converts inline HTML elements (like `<strong>`, `<em>`, `<a>`)
//! back to their Markdown syntax equivalents.

use std::borrow::Cow;

use super::parser::UnknownInlineHandling;
use super::tokenizer::{HtmlToken, HtmlTokenizer};

/// Inline element types that map to Markdown syntax.
#[derive(Clone, Debug, PartialEq)]
pub enum InlineElement {
    /// Plain text content.
    Text(String),
    /// Bold text: `**text**`
    Bold(Vec<InlineElement>),
    /// Italic text: `*text*`
    Italic(Vec<InlineElement>),
    /// Strikethrough text: `~~text~~`
    Strike(Vec<InlineElement>),
    /// Highlighted text: `==text==`
    Highlight(Vec<InlineElement>),
    /// Underlined text: `++text++`
    Underline(Vec<InlineElement>),
    /// Inline code: `` `code` ``
    Code(String),
    /// Link: `[text](url "title")`
    Link {
        href: String,
        title: Option<String>,
        children: Vec<InlineElement>,
    },
    /// Image: `![alt](src "title")`
    Image {
        src: String,
        alt: String,
        title: Option<String>,
    },
    /// Hard line break (two spaces + newline).
    HardBreak,
    /// Raw HTML that couldn't be converted.
    RawHtml(String),
}

/// Parse inline HTML content and convert to InlineElement tree.
pub fn parse_inline_html(
    html: &str,
    unknown_handling: UnknownInlineHandling,
) -> Vec<InlineElement> {
    let mut parser = InlineParser::new(html, unknown_handling);
    parser.parse()
}

/// Convert inline elements to Markdown string.
pub fn inline_to_markdown(elements: &[InlineElement]) -> String {
    let mut out = String::new();
    for elem in elements {
        elem.write_markdown(&mut out);
    }
    out
}

impl InlineElement {
    /// Write this element as Markdown to the output string.
    fn write_markdown(&self, out: &mut String) {
        match self {
            InlineElement::Text(text) => {
                // Escape special Markdown characters in text
                escape_markdown_text(text, out);
            }
            InlineElement::Bold(children) => {
                out.push_str("**");
                for child in children {
                    child.write_markdown(out);
                }
                out.push_str("**");
            }
            InlineElement::Italic(children) => {
                out.push('*');
                for child in children {
                    child.write_markdown(out);
                }
                out.push('*');
            }
            InlineElement::Strike(children) => {
                out.push_str("~~");
                for child in children {
                    child.write_markdown(out);
                }
                out.push_str("~~");
            }
            InlineElement::Highlight(children) => {
                out.push_str("==");
                for child in children {
                    child.write_markdown(out);
                }
                out.push_str("==");
            }
            InlineElement::Underline(children) => {
                out.push_str("++");
                for child in children {
                    child.write_markdown(out);
                }
                out.push_str("++");
            }
            InlineElement::Code(code) => {
                write_code_span(code, out);
            }
            InlineElement::Link {
                href,
                title,
                children,
            } => {
                out.push('[');
                for child in children {
                    child.write_markdown(out);
                }
                out.push_str("](");
                out.push_str(href);
                if let Some(t) = title {
                    out.push_str(" \"");
                    escape_title(t, out);
                    out.push('"');
                }
                out.push(')');
            }
            InlineElement::Image { src, alt, title } => {
                out.push_str("![");
                out.push_str(alt);
                out.push_str("](");
                out.push_str(src);
                if let Some(t) = title {
                    out.push_str(" \"");
                    escape_title(t, out);
                    out.push('"');
                }
                out.push(')');
            }
            InlineElement::HardBreak => {
                out.push_str("  \n");
            }
            InlineElement::RawHtml(html) => {
                out.push_str(html);
            }
        }
    }
}

/// Escape special Markdown characters in text.
fn escape_markdown_text(text: &str, out: &mut String) {
    for ch in text.chars() {
        match ch {
            '\\' | '`' | '*' | '_' | '{' | '}' | '[' | ']' | '(' | ')' | '#' | '+' | '-' | '.'
            | '!' | '|' | '~' | '=' => {
                out.push('\\');
                out.push(ch);
            }
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            _ => out.push(ch),
        }
    }
}

/// Escape characters in a link/image title.
fn escape_title(title: &str, out: &mut String) {
    for ch in title.chars() {
        match ch {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            _ => out.push(ch),
        }
    }
}

/// Write a code span with appropriate backtick fencing.
fn write_code_span(code: &str, out: &mut String) {
    // Find the longest run of backticks in the code
    let max_run = find_max_backtick_run(code);
    let fence_len = max_run + 1;

    for _ in 0..fence_len {
        out.push('`');
    }

    // Add spacing if code starts/ends with backtick or space
    if code.starts_with('`') || code.ends_with('`') || code.starts_with(' ') || code.ends_with(' ')
    {
        out.push(' ');
        out.push_str(code);
        out.push(' ');
    } else {
        out.push_str(code);
    }

    for _ in 0..fence_len {
        out.push('`');
    }
}

/// Find the longest consecutive run of backticks in a string.
fn find_max_backtick_run(s: &str) -> usize {
    let mut max = 0;
    let mut current = 0;

    for ch in s.chars() {
        if ch == '`' {
            current += 1;
            max = max.max(current);
        } else {
            current = 0;
        }
    }

    max
}

/// Parser for inline HTML content.
struct InlineParser<'a> {
    tokenizer: HtmlTokenizer<'a>,
    unknown_handling: UnknownInlineHandling,
}

impl<'a> InlineParser<'a> {
    fn new(html: &'a str, unknown_handling: UnknownInlineHandling) -> Self {
        Self {
            tokenizer: HtmlTokenizer::new(html),
            unknown_handling,
        }
    }

    fn parse(&mut self) -> Vec<InlineElement> {
        self.parse_until(None)
    }

    fn parse_until(&mut self, end_tag: Option<&str>) -> Vec<InlineElement> {
        let mut elements = Vec::new();

        while let Some(token) = self.tokenizer.next_token() {
            match token {
                HtmlToken::Text(text) => {
                    if !text.is_empty() {
                        // Reuse the token's buffer when already normalized.
                        let normalized = match normalize_whitespace(&text) {
                            Cow::Owned(s) => s,
                            Cow::Borrowed(_) => text.into_owned(),
                        };
                        elements.push(InlineElement::Text(normalized));
                    }
                }
                HtmlToken::StartTag {
                    name,
                    attrs,
                    self_closing,
                } => {
                    if let Some(elem) = self.handle_start_tag(&name, &attrs, self_closing) {
                        elements.push(elem);
                    }
                }
                HtmlToken::EndTag { name } => {
                    if let Some(end) = end_tag
                        && name == end
                    {
                        break;
                    }
                    // Orphan end tag - ignore
                }
                HtmlToken::Comment(_) | HtmlToken::Doctype(_) => {
                    // Ignore comments and doctypes
                }
            }
        }

        elements
    }

    fn handle_start_tag(
        &mut self,
        name: &str,
        attrs: &[(Cow<'_, str>, Cow<'_, str>)],
        self_closing: bool,
    ) -> Option<InlineElement> {
        // Emphasis-style wrappers differ only in the variant constructor.
        let wrapper: Option<fn(Vec<InlineElement>) -> InlineElement> = match name {
            "strong" | "b" => Some(InlineElement::Bold),
            "em" | "i" => Some(InlineElement::Italic),
            "del" | "s" | "strike" => Some(InlineElement::Strike),
            "mark" => Some(InlineElement::Highlight),
            "u" | "ins" => Some(InlineElement::Underline),
            _ => None,
        };
        if let Some(ctor) = wrapper {
            return Some(ctor(self.parse_until(Some(name))));
        }

        match name {
            // Inline code
            "code" => {
                let content = self.collect_text_until(name);
                Some(InlineElement::Code(content))
            }
            // Links
            "a" => {
                let href = find_attr(attrs, "href").unwrap_or_default();
                let title = find_attr(attrs, "title");
                let children = self.parse_until(Some(name));
                Some(InlineElement::Link {
                    href,
                    title,
                    children,
                })
            }
            // Images
            "img" => {
                let src = find_attr(attrs, "src").unwrap_or_default();
                let alt = find_attr(attrs, "alt").unwrap_or_default();
                let title = find_attr(attrs, "title");
                Some(InlineElement::Image { src, alt, title })
            }
            // Line breaks
            "br" => Some(InlineElement::HardBreak),
            // Span - transparent, just parse children
            "span" => {
                if self_closing {
                    None
                } else {
                    self.flatten_children(name)
                }
            }
            // Unknown inline elements
            _ => {
                match self.unknown_handling {
                    UnknownInlineHandling::StripTags => {
                        if self_closing {
                            None
                        } else {
                            self.flatten_children(name)
                        }
                    }
                    UnknownInlineHandling::PreserveAsHtml => {
                        // Reconstruct the HTML tag
                        let mut html = String::new();
                        write_open_tag(&mut html, name, attrs, self_closing);
                        if !self_closing {
                            let inner = self.collect_html_until(name);
                            html.push_str(&inner);
                            html.push_str("</");
                            html.push_str(name);
                            html.push('>');
                        }
                        Some(InlineElement::RawHtml(html))
                    }
                }
            }
        }
    }

    /// Parse children of a structurally-transparent tag and collapse them into
    /// a single element (multiple children are re-serialized as text).
    fn flatten_children(&mut self, name: &str) -> Option<InlineElement> {
        let children = self.parse_until(Some(name));
        match children.len() {
            0 => None,
            1 => Some(children.into_iter().next().unwrap()),
            _ => Some(InlineElement::Text(
                children
                    .into_iter()
                    .map(|e| {
                        let mut s = String::new();
                        e.write_markdown(&mut s);
                        s
                    })
                    .collect(),
            )),
        }
    }

    /// Collect raw text content until the specified end tag.
    fn collect_text_until(&mut self, end_tag: &str) -> String {
        let mut text = String::new();

        while let Some(token) = self.tokenizer.next_token() {
            match token {
                HtmlToken::Text(t) => text.push_str(&t),
                HtmlToken::EndTag { name } if name == end_tag => break,
                _ => {}
            }
        }

        text
    }

    /// Collect HTML content (as string) until the specified end tag.
    fn collect_html_until(&mut self, end_tag: &str) -> String {
        let mut html = String::new();
        let mut depth = 1;

        while let Some(token) = self.tokenizer.next_token() {
            match &token {
                HtmlToken::Text(t) => html.push_str(t),
                HtmlToken::StartTag {
                    name,
                    attrs,
                    self_closing,
                } => {
                    write_open_tag(&mut html, name, attrs, *self_closing);
                    if !self_closing && name == end_tag {
                        depth += 1;
                    }
                }
                HtmlToken::EndTag { name } => {
                    if name == end_tag {
                        depth -= 1;
                        if depth == 0 {
                            break;
                        }
                    }
                    html.push_str("</");
                    html.push_str(name);
                    html.push('>');
                }
                HtmlToken::Comment(c) => {
                    html.push_str("<!--");
                    html.push_str(c);
                    html.push_str("-->");
                }
                HtmlToken::Doctype(d) => {
                    html.push_str(d);
                }
            }
        }

        html
    }
}

/// Find an attribute value by name.
pub(super) fn find_attr(attrs: &[(Cow<'_, str>, Cow<'_, str>)], name: &str) -> Option<String> {
    attrs
        .iter()
        .find(|(k, _)| k == name)
        .map(|(_, v)| v.to_string())
}

/// Serialize an opening tag `<name k="v" ...>` (or `<... />` when self-closing).
pub(super) fn write_open_tag(
    out: &mut String,
    name: &str,
    attrs: &[(Cow<'_, str>, Cow<'_, str>)],
    self_closing: bool,
) {
    out.push('<');
    out.push_str(name);
    for (k, v) in attrs {
        out.push(' ');
        out.push_str(k);
        out.push_str("=\"");
        out.push_str(v);
        out.push('"');
    }
    out.push_str(if self_closing { " />" } else { ">" });
}

/// Normalize whitespace in text (collapse runs of whitespace to single space).
/// Already-normalized text (the common case) is returned borrowed.
fn normalize_whitespace(text: &str) -> Cow<'_, str> {
    let mut prev_ws = false;
    for (i, ch) in text.char_indices() {
        if ch.is_whitespace() && (ch != ' ' || prev_ws) {
            // First char needing rewriting: copy the clean prefix, then
            // normalize the rest in the same pass.
            let mut result = String::with_capacity(text.len());
            result.push_str(&text[..i]);
            for ch in text[i..].chars() {
                if ch.is_whitespace() {
                    if !prev_ws {
                        result.push(' ');
                        prev_ws = true;
                    }
                } else {
                    result.push(ch);
                    prev_ws = false;
                }
            }
            return Cow::Owned(result);
        }
        prev_ws = ch.is_whitespace();
    }
    Cow::Borrowed(text)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bold() {
        let elements = parse_inline_html("<strong>bold</strong>", UnknownInlineHandling::StripTags);
        let md = inline_to_markdown(&elements);
        assert_eq!(md, "**bold**");
    }

    #[test]
    fn test_italic() {
        let elements = parse_inline_html("<em>italic</em>", UnknownInlineHandling::StripTags);
        let md = inline_to_markdown(&elements);
        assert_eq!(md, "*italic*");
    }

    #[test]
    fn test_nested() {
        let elements = parse_inline_html(
            "<strong><em>bold italic</em></strong>",
            UnknownInlineHandling::StripTags,
        );
        let md = inline_to_markdown(&elements);
        assert_eq!(md, "***bold italic***");
    }

    #[test]
    fn test_link() {
        let elements = parse_inline_html(
            r#"<a href="https://example.com" title="Example">link</a>"#,
            UnknownInlineHandling::StripTags,
        );
        let md = inline_to_markdown(&elements);
        assert_eq!(md, r#"[link](https://example.com "Example")"#);
    }

    #[test]
    fn test_image() {
        let elements = parse_inline_html(
            r#"<img src="test.png" alt="Test image" />"#,
            UnknownInlineHandling::StripTags,
        );
        let md = inline_to_markdown(&elements);
        assert_eq!(md, "![Test image](test.png)");
    }

    #[test]
    fn test_code() {
        let elements =
            parse_inline_html("<code>let x = 1;</code>", UnknownInlineHandling::StripTags);
        let md = inline_to_markdown(&elements);
        assert_eq!(md, "`let x = 1;`");
    }

    #[test]
    fn test_code_with_backticks() {
        let elements = parse_inline_html(
            "<code>use `backticks`</code>",
            UnknownInlineHandling::StripTags,
        );
        let md = inline_to_markdown(&elements);
        assert_eq!(md, "`` use `backticks` ``");
    }
}
