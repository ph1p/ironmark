//! Direct contract tests for HTML-to-AST parsing and HTML-to-Markdown conversion.

use ironmark::{
    Block, HtmlParseOptions, ListKind, TableAlignment, UnknownInlineHandling, html_to_markdown,
    parse_html_to_ast,
};

fn default_opts() -> HtmlParseOptions {
    HtmlParseOptions::default()
}

fn document_children(block: &Block) -> &[Block] {
    match block {
        Block::Document { children } => children,
        other => panic!("expected document, got {other:?}"),
    }
}

fn paragraph_raw(block: &Block) -> &str {
    match block {
        Block::Paragraph { raw } => raw,
        other => panic!("expected paragraph, got {other:?}"),
    }
}

#[test]
fn parses_basic_block_elements_to_exact_ast_shapes() {
    let ast = parse_html_to_ast("<p>Hello world</p><hr><h2>Title</h2>", &default_opts());
    let children = document_children(&ast);

    assert_eq!(children.len(), 3);
    assert_eq!(paragraph_raw(&children[0]), "Hello world");
    assert!(matches!(children[1], Block::ThematicBreak));
    assert!(matches!(
        &children[2],
        Block::Heading { level: 2, raw } if raw == "Title"
    ));
}

#[test]
fn parses_heading_levels_exactly() {
    for level in 1..=6 {
        let html = format!("<h{level}>Heading {level}</h{level}>");
        let ast = parse_html_to_ast(&html, &default_opts());
        let children = document_children(&ast);

        assert_eq!(children.len(), 1);
        assert!(matches!(
            &children[0],
            Block::Heading { level: actual, raw } if *actual == level && raw == &format!("Heading {level}")
        ));
    }
}

#[test]
fn parses_code_blocks_with_and_without_language() {
    let rust_ast = parse_html_to_ast(
        r#"<pre><code class="language-rust">fn main() {}</code></pre>"#,
        &default_opts(),
    );
    let plain_ast = parse_html_to_ast("<pre><code>plain code</code></pre>", &default_opts());

    let rust_children = document_children(&rust_ast);
    let plain_children = document_children(&plain_ast);

    assert!(matches!(
        &rust_children[0],
        Block::CodeBlock { info, literal } if info.as_str() == "rust" && literal == "fn main() {}"
    ));
    assert!(matches!(
        &plain_children[0],
        Block::CodeBlock { info, literal } if info.is_empty() && literal == "plain code"
    ));
}

#[test]
fn parses_blockquotes_and_nested_blockquotes() {
    let ast = parse_html_to_ast(
        "<blockquote><p>Outer</p><blockquote><p>Inner</p></blockquote></blockquote>",
        &default_opts(),
    );
    let children = document_children(&ast);

    assert_eq!(children.len(), 1);
    match &children[0] {
        Block::BlockQuote { children } => {
            assert_eq!(children.len(), 2);
            assert_eq!(paragraph_raw(&children[0]), "Outer");
            match &children[1] {
                Block::BlockQuote { children } => {
                    assert_eq!(children.len(), 1);
                    assert_eq!(paragraph_raw(&children[0]), "Inner");
                }
                other => panic!("expected nested blockquote, got {other:?}"),
            }
        }
        other => panic!("expected blockquote, got {other:?}"),
    }
}

#[test]
fn parses_lists_task_lists_and_nested_lists() {
    let unordered = parse_html_to_ast("<ul><li>One</li><li>Two</li></ul>", &default_opts());
    let ordered = parse_html_to_ast(
        r#"<ol start="5"><li>A</li><li>B</li></ol>"#,
        &default_opts(),
    );
    let nested = parse_html_to_ast(
        "<ul><li><p>Parent</p><ul><li>Nested</li></ul></li><li>Sibling</li></ul>",
        &default_opts(),
    );
    let tasks = parse_html_to_ast(
        r#"<ul><li><input type="checkbox" checked>Done</li><li><input type="checkbox">Todo</li></ul>"#,
        &default_opts(),
    );

    assert!(matches!(
        &document_children(&unordered)[0],
        Block::List { kind: ListKind::Bullet(b'-'), start: 1, tight: true, children } if children.len() == 2
    ));
    assert!(matches!(
        &document_children(&ordered)[0],
        Block::List { kind: ListKind::Ordered(b'.'), start: 5, children, .. } if children.len() == 2
    ));

    match &document_children(&nested)[0] {
        Block::List {
            tight: false,
            children,
            ..
        } => match &children[0] {
            Block::ListItem { children, .. } => {
                assert_eq!(children.len(), 2);
                assert_eq!(paragraph_raw(&children[0]), "Parent");
                assert!(matches!(children[1], Block::List { .. }));
            }
            other => panic!("expected list item, got {other:?}"),
        },
        other => panic!("expected nested list, got {other:?}"),
    }

    match &document_children(&tasks)[0] {
        Block::List { children, .. } => {
            assert!(matches!(
                &children[0],
                Block::ListItem { checked: Some(true), children } if paragraph_raw(&children[0]) == "Done"
            ));
            assert!(matches!(
                &children[1],
                Block::ListItem { checked: Some(false), children } if paragraph_raw(&children[0]) == "Todo"
            ));
        }
        other => panic!("expected task list, got {other:?}"),
    }
}

#[test]
fn parses_tables_and_alignment_attributes() {
    let align_attr = parse_html_to_ast(
        r#"<table><thead><tr><th align="left">A</th><th align="center">B</th><th align="right">C</th></tr></thead><tbody><tr><td>1</td><td>2</td><td>3</td></tr></tbody></table>"#,
        &default_opts(),
    );
    let align_style = parse_html_to_ast(
        r#"<table><thead><tr><th style="text-align:left">Left</th><th style="text-align:center">Center</th><th style="text-align:right">Right</th></tr></thead></table>"#,
        &default_opts(),
    );

    match &document_children(&align_attr)[0] {
        Block::Table(table) => {
            assert_eq!(table.num_cols, 3);
            assert_eq!(
                table.alignments,
                vec![
                    TableAlignment::Left,
                    TableAlignment::Center,
                    TableAlignment::Right,
                ]
            );
            assert_eq!(table.header, vec!["A", "B", "C"]);
            assert_eq!(table.rows, vec!["1", "2", "3"]);
        }
        other => panic!("expected table, got {other:?}"),
    }

    match &document_children(&align_style)[0] {
        Block::Table(table) => {
            assert_eq!(
                table.alignments,
                vec![
                    TableAlignment::Left,
                    TableAlignment::Center,
                    TableAlignment::Right,
                ]
            );
            assert_eq!(table.header, vec!["Left", "Center", "Right"]);
            assert!(table.rows.is_empty());
        }
        other => panic!("expected styled table, got {other:?}"),
    }
}

#[test]
fn generic_block_containers_flatten_into_document_children() {
    let ast = parse_html_to_ast(
        "<div><p>Alpha</p><section><p>Beta</p></section></div>",
        &default_opts(),
    );
    let children = document_children(&ast);

    assert_eq!(children.len(), 2);
    assert_eq!(paragraph_raw(&children[0]), "Alpha");
    assert_eq!(paragraph_raw(&children[1]), "Beta");
}

#[test]
fn unknown_block_wrappers_are_not_preserved() {
    let ast = parse_html_to_ast("<widget><p>Hello</p></widget>", &default_opts());
    let children = document_children(&ast);

    assert_eq!(children.len(), 1);
    assert_eq!(paragraph_raw(&children[0]), "Hello");
}

#[test]
fn empty_and_whitespace_only_input_produce_empty_documents() {
    let empty = parse_html_to_ast("", &default_opts());
    let whitespace = parse_html_to_ast("   \n\t\n   ", &default_opts());
    let empty_generic = parse_html_to_ast("<div></div><section></section>", &default_opts());

    assert!(document_children(&empty).is_empty());
    assert!(document_children(&whitespace).is_empty());
    assert!(document_children(&empty_generic).is_empty());
}

#[test]
fn malformed_html_and_orphan_end_tags_are_handled_gracefully() {
    let unclosed = parse_html_to_ast("<p>Unclosed paragraph", &default_opts());
    let orphan = parse_html_to_ast("<p>Hello</p></span>", &default_opts());

    assert_eq!(
        paragraph_raw(&document_children(&unclosed)[0]),
        "Unclosed paragraph"
    );
    assert_eq!(paragraph_raw(&document_children(&orphan)[0]), "Hello");
}

#[test]
fn max_input_size_truncates_input_before_parsing() {
    let opts = HtmlParseOptions {
        max_input_size: 12,
        ..default_opts()
    };
    let ast = parse_html_to_ast("<p>hello</p><p>world</p>", &opts);
    let children = document_children(&ast);

    assert_eq!(children.len(), 1);
    assert_eq!(paragraph_raw(&children[0]), "hello");
}

#[test]
fn max_nesting_depth_limits_block_nesting() {
    let unlimited = parse_html_to_ast(
        "<blockquote><blockquote><p>Deep</p></blockquote></blockquote>",
        &default_opts(),
    );
    let limited = parse_html_to_ast(
        "<blockquote><blockquote><p>Deep</p></blockquote></blockquote>",
        &HtmlParseOptions {
            max_nesting_depth: 1,
            ..default_opts()
        },
    );

    match &document_children(&unlimited)[0] {
        Block::BlockQuote { children } => {
            assert_eq!(children.len(), 1);
            assert!(matches!(children[0], Block::BlockQuote { .. }));
        }
        other => panic!("expected nested blockquote tree, got {other:?}"),
    }

    match &document_children(&limited)[0] {
        Block::BlockQuote { children } => {
            assert_eq!(children.len(), 1);
            assert_eq!(paragraph_raw(&children[0]), "Deep");
        }
        other => panic!("expected flattened blockquote, got {other:?}"),
    }
}

#[test]
fn converts_inline_markdown_equivalents_exactly() {
    assert_eq!(
        html_to_markdown("<p><strong>Bold</strong> text</p>", &default_opts()),
        "**Bold** text\n"
    );
    assert_eq!(
        html_to_markdown("<p><em>Italic</em> text</p>", &default_opts()),
        "*Italic* text\n"
    );
    assert_eq!(
        html_to_markdown("<p>Use <code>let x = 1;</code> here</p>", &default_opts()),
        "Use `let x = 1;` here\n"
    );
    assert_eq!(
        html_to_markdown(
            r#"<p><a href="https://example.com" title="Title">Link</a></p>"#,
            &default_opts()
        ),
        "[Link](https://example.com \"Title\")\n"
    );
    assert_eq!(
        html_to_markdown(
            r#"<p><img src="test.png" alt="Alt text" /></p>"#,
            &default_opts()
        ),
        "![Alt text](test.png)\n"
    );
    assert_eq!(
        html_to_markdown("<p><del>Deleted</del> text</p>", &default_opts()),
        "~~Deleted~~ text\n"
    );
    assert_eq!(
        html_to_markdown("<p><mark>Highlighted</mark> text</p>", &default_opts()),
        "==Highlighted== text\n"
    );
    assert_eq!(
        html_to_markdown("<p><u>Underlined</u> text</p>", &default_opts()),
        "++Underlined++ text\n"
    );
    assert_eq!(
        html_to_markdown(
            "<p><strong><em>Bold italic</em></strong></p>",
            &default_opts()
        ),
        "***Bold italic***\n"
    );
}

#[test]
fn converts_entities_breaks_and_whitespace_normalization_exactly() {
    assert_eq!(
        html_to_markdown("<p>&amp; &#65; &#x42;</p>", &default_opts()),
        "& A B\n"
    );
    assert_eq!(
        html_to_markdown("<p>Line 1<br />Line 2</p>", &default_opts()),
        "Line 1  \nLine 2\n"
    );
    assert_eq!(
        html_to_markdown("<p><strong>Hello   \n world</strong></p>", &default_opts()),
        "**Hello world**\n"
    );
}

#[test]
fn unknown_inline_handling_controls_markdown_output() {
    let strip = HtmlParseOptions {
        unknown_inline_handling: UnknownInlineHandling::StripTags,
        ..default_opts()
    };
    let preserve = HtmlParseOptions {
        unknown_inline_handling: UnknownInlineHandling::PreserveAsHtml,
        ..default_opts()
    };

    assert_eq!(
        html_to_markdown("<p><sup>Superscript</sup> text</p>", &strip),
        "Superscript text\n"
    );
    assert_eq!(
        html_to_markdown("<p><sup>Superscript</sup> text</p>", &preserve),
        "<sup>Superscript</sup> text\n"
    );
}

#[test]
fn attribute_value_quotes_survive_reserialization() {
    // A `"` inside a preserved tag's attribute must be escaped on the way out,
    // or it closes the attribute early and the rest of the value is lost.
    let preserve = HtmlParseOptions {
        unknown_inline_handling: UnknownInlineHandling::PreserveAsHtml,
        ..default_opts()
    };
    assert_eq!(
        html_to_markdown(r#"<p><sup data-x='he said "hi"'>s</sup></p>"#, &preserve),
        "<sup data-x=\"he said &quot;hi&quot;\">s</sup>\n"
    );
}

#[test]
fn link_title_keeps_interior_quotes() {
    // Reaches the title via a single-quoted attribute and via `&quot;`; both
    // must round-trip through the Markdown title escape.
    assert_eq!(
        html_to_markdown(
            r#"<p><a href="/x" title='he said "hi"'>y</a></p>"#,
            &default_opts()
        ),
        "[y](/x \"he said \\\"hi\\\"\")\n"
    );
    assert_eq!(
        html_to_markdown(
            r#"<p><a href="/x" title="t &quot;q&quot;">y</a></p>"#,
            &default_opts()
        ),
        "[y](/x \"t \\\"q\\\"\")\n"
    );
}

#[test]
fn link_destination_with_space_is_bracketed() {
    // A bare space would end the destination, degrading the link to plain text.
    assert_eq!(
        html_to_markdown(r#"<p><a href="/a b">y</a></p>"#, &default_opts()),
        "[y](</a b>)\n"
    );
}

#[test]
fn leading_ordered_marker_is_escaped() {
    // `1. text` at the start of a paragraph would re-parse as an ordered list.
    assert_eq!(
        html_to_markdown("<p>1. not a list</p>", &default_opts()),
        "1\\. not a list\n"
    );
    // A digit run mid-text is not a marker and must stay unescaped.
    assert_eq!(
        html_to_markdown("<p>version 1.2 here</p>", &default_opts()),
        "version 1.2 here\n"
    );
}
