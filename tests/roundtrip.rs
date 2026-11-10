//! Round-trip conversion tests.
//!
//! Tests bidirectional conversion:
//! - Markdown -> HTML -> Markdown
//! - HTML -> Markdown -> HTML
//!
//! These tests verify semantic preservation, not exact string equality.

use ironmark::{
    HtmlParseOptions, ParseOptions, html_to_markdown, parse_html_to_ast, parse_markdown,
    render_html, render_markdown,
};

fn md_opts() -> ParseOptions {
    ParseOptions::default()
}

fn html_opts() -> HtmlParseOptions {
    HtmlParseOptions::default()
}

const README_FIXTURE: &str = include_str!("../README.md");

// ============================================================================
// Markdown -> HTML -> Markdown Round-trip Tests
// ============================================================================

mod md_html_md {
    use super::*;

    fn roundtrip(markdown: &str) -> String {
        let html = render_html(markdown, &md_opts());
        html_to_markdown(&html, &html_opts())
    }

    #[test]
    fn paragraph() {
        let md = roundtrip("Hello world");
        assert_eq!(md.trim(), "Hello world");
    }

    #[test]
    fn multiple_paragraphs() {
        let md = roundtrip("First paragraph.\n\nSecond paragraph.");
        assert!(md.contains("First paragraph."));
        assert!(md.contains("Second paragraph."));
    }

    #[test]
    fn headings() {
        for level in 1..=6 {
            let original = format!("{} Heading {}", "#".repeat(level), level);
            let md = roundtrip(&original);
            assert!(md.contains(&format!("{} Heading", "#".repeat(level))));
        }
    }

    #[test]
    fn bold() {
        let md = roundtrip("**Bold** text");
        assert!(md.contains("**Bold**") || md.contains("__Bold__"));
    }

    #[test]
    fn italic() {
        let md = roundtrip("*Italic* text");
        assert!(md.contains("*Italic*") || md.contains("_Italic_"));
    }

    #[test]
    fn bold_italic() {
        let md = roundtrip("***Bold italic*** text");
        assert!(md.contains("Bold italic"));
    }

    #[test]
    fn inline_code() {
        let md = roundtrip("Use `code` here");
        assert!(md.contains("`code`"));
    }

    #[test]
    fn strikethrough() {
        let md = roundtrip("~~Deleted~~ text");
        assert!(md.contains("~~Deleted~~"));
    }

    #[test]
    fn highlight() {
        let md = roundtrip("==Highlighted== text");
        assert!(md.contains("==Highlighted=="));
    }

    #[test]
    fn underline() {
        let md = roundtrip("++Underlined++ text");
        assert!(md.contains("++Underlined++"));
    }

    #[test]
    fn link() {
        let md = roundtrip("[Link](https://example.com)");
        assert!(md.contains("[Link](https://example.com)"));
    }

    #[test]
    fn link_with_title() {
        let md = roundtrip(r#"[Link](https://example.com "Title")"#);
        assert!(md.contains("[Link](https://example.com"));
        assert!(md.contains("Title"));
    }

    #[test]
    fn image() {
        let md = roundtrip("![Alt](image.png)");
        assert!(md.contains("![Alt](image.png)"));
    }

    #[test]
    fn code_block() {
        let md = roundtrip("```rust\nfn main() {}\n```");
        assert!(md.contains("```rust"));
        assert!(md.contains("fn main() {}"));
    }

    #[test]
    fn code_block_no_language() {
        let md = roundtrip("```\nplain code\n```");
        assert!(md.contains("```"));
        assert!(md.contains("plain code"));
    }

    #[test]
    fn blockquote() {
        let md = roundtrip("> Quoted text");
        assert!(md.contains("> ") || md.contains(">"));
        assert!(md.contains("Quoted text"));
    }

    #[test]
    fn nested_blockquote() {
        let md = roundtrip("> Level 1\n>> Level 2");
        assert!(md.contains("Level 1"));
        assert!(md.contains("Level 2"));
    }

    #[test]
    fn unordered_list() {
        let md = roundtrip("- Item 1\n- Item 2\n- Item 3");
        assert!(md.contains("Item 1"));
        assert!(md.contains("Item 2"));
        assert!(md.contains("Item 3"));
    }

    #[test]
    fn ordered_list() {
        let md = roundtrip("1. First\n2. Second\n3. Third");
        assert!(md.contains("First"));
        assert!(md.contains("Second"));
        assert!(md.contains("Third"));
    }

    #[test]
    fn task_list() {
        let md = roundtrip("- [ ] Todo\n- [x] Done");
        assert!(md.contains("[ ]") || md.contains("Todo"));
        assert!(md.contains("[x]") || md.contains("Done"));
    }

    #[test]
    fn thematic_break() {
        let md = roundtrip("Above\n\n---\n\nBelow");
        assert!(md.contains("---") || md.contains("***") || md.contains("___"));
    }

    #[test]
    fn table() {
        let original = "| A | B |\n|---|---|\n| 1 | 2 |";
        let md = roundtrip(original);
        assert!(md.contains("A"));
        assert!(md.contains("B"));
        assert!(md.contains("1"));
        assert!(md.contains("2"));
    }

    #[test]
    fn table_with_alignment() {
        let original = "| Left | Center | Right |\n|:---|:---:|---:|\n| L | C | R |";
        let md = roundtrip(original);
        assert!(md.contains("Left"));
        assert!(md.contains("Center"));
        assert!(md.contains("Right"));
    }

    #[test]
    fn complex_document() {
        let original = r#"# Title

This is a **bold** and *italic* paragraph with `code`.

## Code Example

```rust
fn main() {
    println!("Hello");
}
```

> A quote

- List item 1
- List item 2

| Col A | Col B |
|-------|-------|
| 1     | 2     |

---

End."#;
        let md = roundtrip(original);
        assert!(md.contains("# Title"));
        assert!(md.contains("**bold**") || md.contains("__bold__"));
        assert!(md.contains("```rust"));
        assert!(md.contains("println!"));
        assert!(md.contains("List item"));
        assert!(md.contains("Col A"));
    }
}

// ============================================================================
// HTML -> Markdown -> HTML Round-trip Tests
// ============================================================================

mod html_md_html {
    use super::*;

    fn roundtrip(html: &str) -> String {
        let md = html_to_markdown(html, &html_opts());
        render_html(&md, &md_opts())
    }

    fn contains_tag(html: &str, tag: &str) -> bool {
        html.contains(&format!("<{}", tag))
    }

    #[test]
    fn paragraph() {
        let html = roundtrip("<p>Hello world</p>");
        assert!(contains_tag(&html, "p"));
        assert!(html.contains("Hello world"));
    }

    #[test]
    fn headings() {
        for level in 1..=6 {
            let original = format!("<h{0}>Heading {0}</h{0}>", level);
            let html = roundtrip(&original);
            assert!(contains_tag(&html, &format!("h{}", level)));
            assert!(html.contains(&format!("Heading {}", level)));
        }
    }

    #[test]
    fn bold() {
        let html = roundtrip("<p><strong>Bold</strong> text</p>");
        assert!(contains_tag(&html, "strong"));
        assert!(html.contains("Bold"));
    }

    #[test]
    fn italic() {
        let html = roundtrip("<p><em>Italic</em> text</p>");
        assert!(contains_tag(&html, "em"));
        assert!(html.contains("Italic"));
    }

    #[test]
    fn inline_code() {
        let html = roundtrip("<p>Use <code>code</code> here</p>");
        assert!(contains_tag(&html, "code"));
        assert!(html.contains("code"));
    }

    #[test]
    fn link() {
        let html = roundtrip(r#"<p><a href="https://example.com">Link</a></p>"#);
        assert!(html.contains(r#"href="https://example.com""#));
        assert!(html.contains("Link"));
    }

    #[test]
    fn image() {
        let html = roundtrip(r#"<p><img src="test.png" alt="Alt" /></p>"#);
        assert!(html.contains("src=\"test.png\""));
        assert!(html.contains("alt=\"Alt\""));
    }

    #[test]
    fn code_block() {
        let html = roundtrip(r#"<pre><code class="language-rust">fn main() {}</code></pre>"#);
        assert!(contains_tag(&html, "pre"));
        assert!(contains_tag(&html, "code"));
        assert!(html.contains("fn main()"));
    }

    #[test]
    fn blockquote() {
        let html = roundtrip("<blockquote><p>Quoted</p></blockquote>");
        assert!(contains_tag(&html, "blockquote"));
        assert!(html.contains("Quoted"));
    }

    #[test]
    fn unordered_list() {
        let html = roundtrip("<ul><li>One</li><li>Two</li></ul>");
        assert!(contains_tag(&html, "ul"));
        assert!(contains_tag(&html, "li"));
        assert!(html.contains("One"));
        assert!(html.contains("Two"));
    }

    #[test]
    fn ordered_list() {
        let html = roundtrip("<ol><li>First</li><li>Second</li></ol>");
        assert!(contains_tag(&html, "ol"));
        assert!(contains_tag(&html, "li"));
    }

    #[test]
    fn thematic_break() {
        let html = roundtrip("<p>Above</p><hr><p>Below</p>");
        assert!(contains_tag(&html, "hr"));
    }

    #[test]
    fn table() {
        let html = roundtrip(
            "<table><thead><tr><th>A</th><th>B</th></tr></thead><tbody><tr><td>1</td><td>2</td></tr></tbody></table>",
        );
        assert!(contains_tag(&html, "table"));
        assert!(contains_tag(&html, "th"));
        assert!(contains_tag(&html, "td"));
    }

    #[test]
    fn strikethrough() {
        let html = roundtrip("<p><del>Deleted</del></p>");
        assert!(contains_tag(&html, "del"));
    }

    #[test]
    fn highlight() {
        let html = roundtrip("<p><mark>Highlighted</mark></p>");
        assert!(contains_tag(&html, "mark"));
    }

    #[test]
    fn underline() {
        let html = roundtrip("<p><u>Underlined</u></p>");
        assert!(contains_tag(&html, "u"));
    }
}

// ============================================================================
// AST Round-trip Tests (Markdown -> AST -> Markdown)
// ============================================================================

mod ast_roundtrip {
    use super::*;

    fn roundtrip(markdown: &str) -> String {
        let ast = parse_markdown(markdown, &md_opts());
        render_markdown(&ast)
    }

    #[test]
    fn paragraph() {
        let md = roundtrip("Hello world");
        assert_eq!(md.trim(), "Hello world");
    }

    #[test]
    fn heading() {
        let md = roundtrip("# Heading");
        assert!(md.contains("# Heading"));
    }

    #[test]
    fn bold() {
        let md = roundtrip("**Bold** text");
        assert!(md.contains("**Bold**"));
    }

    #[test]
    fn code_block() {
        let md = roundtrip("```rust\ncode\n```");
        assert!(md.contains("```rust"));
        assert!(md.contains("code"));
    }

    #[test]
    fn list() {
        let md = roundtrip("- One\n- Two");
        assert!(md.contains("One"));
        assert!(md.contains("Two"));
    }

    #[test]
    fn blockquote() {
        let md = roundtrip("> Quote");
        assert!(md.contains(">"));
        assert!(md.contains("Quote"));
    }

    #[test]
    fn table() {
        let md = roundtrip("| A | B |\n|---|---|\n| 1 | 2 |");
        assert!(md.contains("|"));
        assert!(md.contains("A"));
        assert!(md.contains("B"));
    }

    #[test]
    fn readme_md_html_md_ast_md() {
        let html = render_html(README_FIXTURE, &md_opts());
        let normalized_md = html_to_markdown(&html, &html_opts());
        let ast = parse_markdown(&normalized_md, &md_opts());
        let final_md = render_markdown(&ast);
        let reparsed_ast = parse_markdown(&final_md, &md_opts());

        assert_eq!(reparsed_ast, ast);
        assert!(final_md.contains("# ironmark"));
        assert!(final_md.contains("## Build tool plugins"));
        assert!(final_md.contains("### JavaScript"));
    }
}

// ============================================================================
// HTML AST Round-trip Tests (HTML -> AST -> Markdown -> HTML)
// ============================================================================

mod html_ast_roundtrip {
    use super::*;

    fn roundtrip(html: &str) -> String {
        let ast = parse_html_to_ast(html, &html_opts());
        let md = render_markdown(&ast);
        render_html(&md, &md_opts())
    }

    #[test]
    fn paragraph() {
        let html = roundtrip("<p>Hello</p>");
        assert!(html.contains("<p>"));
        assert!(html.contains("Hello"));
    }

    #[test]
    fn heading() {
        let html = roundtrip("<h1>Title</h1>");
        assert!(html.contains("<h1>"));
        assert!(html.contains("Title"));
    }

    #[test]
    fn bold() {
        let html = roundtrip("<p><strong>Bold</strong></p>");
        assert!(html.contains("<strong>"));
    }

    #[test]
    fn list() {
        let html = roundtrip("<ul><li>Item</li></ul>");
        assert!(html.contains("<ul>"));
        assert!(html.contains("<li>"));
    }
}
