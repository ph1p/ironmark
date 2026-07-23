//! HTML tokenizer for parsing HTML into tokens.
//!
//! This tokenizer produces a stream of HTML tokens (start tags, end tags, text, etc.)
//! that the parser can consume to build the AST.

use std::borrow::Cow;

/// An HTML token produced by the tokenizer.
#[derive(Clone, Debug, PartialEq)]
pub enum HtmlToken<'a> {
    /// A start tag like `<p>` or `<a href="...">`.
    StartTag {
        /// Tag name (lowercase).
        name: Cow<'a, str>,
        /// Attribute name-value pairs.
        attrs: Vec<(Cow<'a, str>, Cow<'a, str>)>,
        /// Whether this is a self-closing tag like `<br />`.
        self_closing: bool,
    },
    /// An end tag like `</p>`.
    EndTag {
        /// Tag name (lowercase).
        name: Cow<'a, str>,
    },
    /// Text content between tags.
    Text(Cow<'a, str>),
    /// An HTML comment `<!-- ... -->`.
    Comment(Cow<'a, str>),
    /// A DOCTYPE declaration.
    Doctype(Cow<'a, str>),
}

/// HTML tokenizer that produces tokens from an HTML string.
pub struct HtmlTokenizer<'a> {
    input: &'a str,
    bytes: &'a [u8],
    pos: usize,
}

impl<'a> HtmlTokenizer<'a> {
    /// Create a new tokenizer for the given HTML input.
    pub fn new(input: &'a str) -> Self {
        Self {
            input,
            bytes: input.as_bytes(),
            pos: 0,
        }
    }

    /// Get the next token, or None if at end of input.
    pub fn next_token(&mut self) -> Option<HtmlToken<'a>> {
        if self.pos >= self.bytes.len() {
            return None;
        }

        if self.bytes[self.pos] == b'<' {
            self.parse_tag_or_comment()
        } else {
            self.parse_text()
        }
    }

    /// Parse text content until we hit a `<` or end of input.
    fn parse_text(&mut self) -> Option<HtmlToken<'a>> {
        let start = self.pos;

        while self.pos < self.bytes.len() && self.bytes[self.pos] != b'<' {
            self.pos += 1;
        }

        if self.pos > start {
            let text = &self.input[start..self.pos];
            // Decode HTML entities
            let decoded = decode_entities(text);
            Some(HtmlToken::Text(decoded))
        } else {
            None
        }
    }

    /// Parse a tag, comment, or DOCTYPE starting with `<`.
    fn parse_tag_or_comment(&mut self) -> Option<HtmlToken<'a>> {
        debug_assert_eq!(self.bytes[self.pos], b'<');
        self.pos += 1; // Skip '<'

        if self.pos >= self.bytes.len() {
            return Some(HtmlToken::Text(Cow::Borrowed("<")));
        }

        // Check for comment: <!--
        if self.bytes[self.pos..].starts_with(b"!--") {
            return self.parse_comment();
        }

        // Check for DOCTYPE: <!DOCTYPE
        if self.bytes[self.pos..].starts_with(b"!DOCTYPE")
            || self.bytes[self.pos..].starts_with(b"!doctype")
        {
            return self.parse_doctype();
        }

        // Check for CDATA (treat as text)
        if self.bytes[self.pos..].starts_with(b"![CDATA[") {
            return self.parse_cdata();
        }

        // Check for end tag: </
        if self.bytes[self.pos] == b'/' {
            return self.parse_end_tag();
        }

        // Start tag
        self.parse_start_tag()
    }

    /// Parse a comment `<!-- ... -->`.
    fn parse_comment(&mut self) -> Option<HtmlToken<'a>> {
        self.pos += 3; // Skip '!--'
        let start = self.pos;

        // Find closing -->
        while self.pos + 2 < self.bytes.len() {
            if &self.bytes[self.pos..self.pos + 3] == b"-->" {
                let comment = &self.input[start..self.pos];
                self.pos += 3; // Skip '-->'
                return Some(HtmlToken::Comment(Cow::Borrowed(comment)));
            }
            self.pos += 1;
        }

        // Unclosed comment - consume rest as comment
        self.pos = self.bytes.len();
        Some(HtmlToken::Comment(Cow::Borrowed(&self.input[start..])))
    }

    /// Parse a DOCTYPE declaration.
    fn parse_doctype(&mut self) -> Option<HtmlToken<'a>> {
        let start = self.pos - 1; // Include the '<'

        // Find closing >
        while self.pos < self.bytes.len() && self.bytes[self.pos] != b'>' {
            self.pos += 1;
        }

        if self.pos < self.bytes.len() {
            self.pos += 1; // Skip '>'
        }

        Some(HtmlToken::Doctype(Cow::Borrowed(
            &self.input[start..self.pos],
        )))
    }

    /// Parse a CDATA section as text.
    fn parse_cdata(&mut self) -> Option<HtmlToken<'a>> {
        self.pos += 8; // Skip '![CDATA['
        let start = self.pos;

        // Find closing ]]>
        while self.pos + 2 < self.bytes.len() {
            if &self.bytes[self.pos..self.pos + 3] == b"]]>" {
                let text = &self.input[start..self.pos];
                self.pos += 3; // Skip ']]>'
                return Some(HtmlToken::Text(Cow::Borrowed(text)));
            }
            self.pos += 1;
        }

        // Unclosed CDATA
        self.pos = self.bytes.len();
        Some(HtmlToken::Text(Cow::Borrowed(&self.input[start..])))
    }

    /// Parse an end tag `</name>`.
    fn parse_end_tag(&mut self) -> Option<HtmlToken<'a>> {
        self.pos += 1; // Skip '/'

        self.skip_whitespace();
        let name = self.parse_tag_name();

        if name.is_empty() {
            // Invalid end tag, treat as text
            return Some(HtmlToken::Text(Cow::Borrowed("</")));
        }

        self.skip_whitespace();

        // Skip to closing >
        while self.pos < self.bytes.len() && self.bytes[self.pos] != b'>' {
            self.pos += 1;
        }

        if self.pos < self.bytes.len() {
            self.pos += 1; // Skip '>'
        }

        Some(HtmlToken::EndTag {
            name: lowercase_cow(name),
        })
    }

    /// Parse a start tag `<name ...>` or `<name ... />`.
    fn parse_start_tag(&mut self) -> Option<HtmlToken<'a>> {
        self.skip_whitespace();
        let name = self.parse_tag_name();

        if name.is_empty() {
            // Invalid tag, treat as text
            return Some(HtmlToken::Text(Cow::Borrowed("<")));
        }

        let mut attrs = Vec::new();
        let mut self_closing = false;

        // Parse attributes
        loop {
            self.skip_whitespace();

            if self.pos >= self.bytes.len() {
                break;
            }

            let b = self.bytes[self.pos];

            if b == b'>' {
                self.pos += 1;
                break;
            }

            if b == b'/' {
                self.pos += 1;
                self.skip_whitespace();
                if self.pos < self.bytes.len() && self.bytes[self.pos] == b'>' {
                    self.pos += 1;
                    self_closing = true;
                }
                break;
            }

            // Parse attribute
            if let Some((attr_name, attr_value)) = self.parse_attribute() {
                attrs.push((attr_name, attr_value));
            } else {
                // Skip invalid character
                self.pos += 1;
            }
        }

        let name = lowercase_cow(name);

        // Void elements are always self-closing
        if is_void_element(&name) {
            self_closing = true;
        }

        Some(HtmlToken::StartTag {
            name,
            attrs,
            self_closing,
        })
    }

    /// Parse a tag name as a borrowed slice of the input.
    fn parse_tag_name(&mut self) -> &'a str {
        let start = self.pos;

        while self.pos < self.bytes.len() {
            let b = self.bytes[self.pos];
            if b.is_ascii_alphanumeric() || b == b'-' || b == b'_' || b == b':' {
                self.pos += 1;
            } else {
                break;
            }
        }

        &self.input[start..self.pos]
    }

    /// Parse an attribute `name="value"` or `name='value'` or `name=value` or `name`.
    fn parse_attribute(&mut self) -> Option<(Cow<'a, str>, Cow<'a, str>)> {
        let name_start = self.pos;

        // Parse attribute name
        while self.pos < self.bytes.len() {
            let b = self.bytes[self.pos];
            if b.is_ascii_alphanumeric()
                || b == b'-'
                || b == b'_'
                || b == b':'
                || b == b'.'
                || b == b'@'
            {
                self.pos += 1;
            } else {
                break;
            }
        }

        if self.pos == name_start {
            return None;
        }

        let name = &self.input[name_start..self.pos];

        self.skip_whitespace();

        // Check for =
        if self.pos >= self.bytes.len() || self.bytes[self.pos] != b'=' {
            // Boolean attribute (no value)
            return Some((lowercase_cow(name), Cow::Borrowed("")));
        }

        self.pos += 1; // Skip '='
        self.skip_whitespace();

        // Parse value
        let value = if self.pos < self.bytes.len() {
            let quote = self.bytes[self.pos];
            if quote == b'"' || quote == b'\'' {
                self.pos += 1;
                let value_start = self.pos;

                while self.pos < self.bytes.len() && self.bytes[self.pos] != quote {
                    self.pos += 1;
                }

                let value = &self.input[value_start..self.pos];

                if self.pos < self.bytes.len() {
                    self.pos += 1; // Skip closing quote
                }

                decode_entities(value)
            } else {
                // Unquoted value
                let value_start = self.pos;
                while self.pos < self.bytes.len() {
                    let b = self.bytes[self.pos];
                    if b.is_ascii_whitespace() || b == b'>' || b == b'/' {
                        break;
                    }
                    self.pos += 1;
                }
                let value = &self.input[value_start..self.pos];
                decode_entities(value)
            }
        } else {
            Cow::Borrowed("")
        };

        Some((lowercase_cow(name), value))
    }

    /// Skip whitespace characters.
    fn skip_whitespace(&mut self) {
        while self.pos < self.bytes.len() && self.bytes[self.pos].is_ascii_whitespace() {
            self.pos += 1;
        }
    }
}

/// Lowercase a tag/attribute name, borrowing when it already is lowercase (the common case).
fn lowercase_cow(name: &str) -> Cow<'_, str> {
    if name.bytes().any(|b| b.is_ascii_uppercase()) {
        Cow::Owned(name.to_ascii_lowercase())
    } else {
        Cow::Borrowed(name)
    }
}

/// Check if a (lowercase) tag name is a void element (self-closing).
fn is_void_element(name: &str) -> bool {
    matches!(
        name,
        "area"
            | "base"
            | "br"
            | "col"
            | "embed"
            | "hr"
            | "img"
            | "input"
            | "link"
            | "meta"
            | "param"
            | "source"
            | "track"
            | "wbr"
    )
}

/// Decode HTML entities in a string, using the full HTML5 entity table.
/// Clean runs between entities are bulk-copied; entity-free text stays borrowed.
fn decode_entities(s: &str) -> Cow<'_, str> {
    let bytes = s.as_bytes();
    if memchr::memchr(b'&', bytes).is_none() {
        return Cow::Borrowed(s);
    }

    let mut result = String::with_capacity(s.len());
    let mut i = 0;
    let mut seg_start = 0;
    while let Some(off) = memchr::memchr(b'&', &bytes[i..]) {
        let amp = i + off;
        result.push_str(&s[seg_start..amp]);
        if let Some(end) = crate::entities::resolve_entity_in_bytes(bytes, amp, &mut result) {
            i = end;
        } else {
            result.push('&');
            i = amp + 1;
        }
        seg_start = i;
    }
    result.push_str(&s[seg_start..]);
    Cow::Owned(result)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tokenize(html: &str) -> Vec<HtmlToken<'_>> {
        let mut tokenizer = HtmlTokenizer::new(html);
        let mut tokens = Vec::new();
        while let Some(token) = tokenizer.next_token() {
            tokens.push(token);
        }
        tokens
    }

    #[test]
    fn test_simple_tag() {
        let tokens = tokenize("<p>Hello</p>");
        assert_eq!(tokens.len(), 3);
        assert!(matches!(&tokens[0], HtmlToken::StartTag { name, .. } if name == "p"));
        assert!(matches!(&tokens[1], HtmlToken::Text(t) if t == "Hello"));
        assert!(matches!(&tokens[2], HtmlToken::EndTag { name } if name == "p"));
    }

    #[test]
    fn test_attributes() {
        let tokens = tokenize(r#"<a href="https://example.com" title='Test'>Link</a>"#);
        assert_eq!(tokens.len(), 3);
        if let HtmlToken::StartTag { name, attrs, .. } = &tokens[0] {
            assert_eq!(name, "a");
            assert_eq!(attrs.len(), 2);
            assert_eq!(attrs[0].0, "href");
            assert_eq!(attrs[0].1, "https://example.com");
            assert_eq!(attrs[1].0, "title");
            assert_eq!(attrs[1].1, "Test");
        } else {
            panic!("Expected StartTag");
        }
    }

    #[test]
    fn test_self_closing() {
        let tokens = tokenize("<br /><hr><img src='test.png' />");
        assert_eq!(tokens.len(), 3);
        assert!(
            matches!(&tokens[0], HtmlToken::StartTag { name, self_closing, .. } if name == "br" && *self_closing)
        );
        assert!(
            matches!(&tokens[1], HtmlToken::StartTag { name, self_closing, .. } if name == "hr" && *self_closing)
        );
        assert!(
            matches!(&tokens[2], HtmlToken::StartTag { name, self_closing, .. } if name == "img" && *self_closing)
        );
    }

    #[test]
    fn test_comment() {
        let tokens = tokenize("<!-- This is a comment --><p>Text</p>");
        assert_eq!(tokens.len(), 4);
        assert!(matches!(&tokens[0], HtmlToken::Comment(c) if c == " This is a comment "));
    }

    #[test]
    fn test_entities() {
        let tokens = tokenize("<p>&amp; &lt; &gt; &quot;</p>");
        if let HtmlToken::Text(t) = &tokens[1] {
            assert_eq!(t, "& < > \"");
        } else {
            panic!("Expected Text");
        }
    }

    #[test]
    fn test_nested_tags() {
        // <div>, <p>, "Hello ", <strong>, "world", </strong>, </p>, </div> = 8 tokens
        let tokens = tokenize("<div><p>Hello <strong>world</strong></p></div>");
        assert_eq!(tokens.len(), 8);
    }
}
