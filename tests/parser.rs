use ironmark::{ParseOptions, render_html};

fn assert_html(md: &str, expected: &str) {
    let opts = ParseOptions {
        hard_breaks: false,
        ..Default::default()
    };
    assert_eq!(render_html(md, &opts), expected);
}

#[test]
fn parses_empty_and_whitespace_input() {
    assert_html("", "");
    assert_html("   \n\n\t\n", "");
}

#[test]
fn parses_headings_h1_to_h6() {
    assert_html(
        "# h1\n## h2\n### h3\n#### h4\n##### h5\n###### h6",
        "<h1>h1</h1>\n<h2>h2</h2>\n<h3>h3</h3>\n<h4>h4</h4>\n<h5>h5</h5>\n<h6>h6</h6>\n",
    );
}

#[test]
fn parses_setext_headings() {
    assert_html(
        "Heading one\n===========\n\nHeading two\n-----------",
        "<h1>Heading one</h1>\n<h2>Heading two</h2>\n",
    );
}

#[test]
fn parses_indented_heading() {
    assert_html("   ## heading", "<h2>heading</h2>\n");
}

#[test]
fn non_heading_without_space_after_hash() {
    assert_html("##heading", "<p>##heading</p>\n");
}

#[test]
fn paragraph_collapses_lines_until_block_boundary() {
    assert_html(
        "line one\nline two\n\n# h\nline three",
        "<p>line one\nline two</p>\n<h1>h</h1>\n<p>line three</p>\n",
    );
}

#[test]
fn parses_inline_styles() {
    assert_html(
        "this is **strong** and *em* and `code`",
        "<p>this is <strong>strong</strong> and <em>em</em> and <code>code</code></p>\n",
    );
}

#[test]
fn parses_simple_inline_mix() {
    assert_html(
        "This has **bold**, *italic*, `code`, ~~strike~~, and [link](http://x.com/1).",
        "<p>This has <strong>bold</strong>, <em>italic</em>, <code>code</code>, <del>strike</del>, and <a href=\"http://x.com/1\">link</a>.</p>\n",
    );
}

#[test]
fn parses_underscore_variants() {
    assert_html(
        "__strong__ and _em_",
        "<p><strong>strong</strong> and <em>em</em></p>\n",
    );
}

#[test]
fn parses_nested_inline_markup() {
    assert_html(
        "**outer *inner***",
        "<p><strong>outer <em>inner</em></strong></p>\n",
    );
}

#[test]
fn parses_links_and_inline_label_markup() {
    assert_html(
        "visit [**site**](https://example.com)",
        "<p>visit <a href=\"https://example.com\"><strong>site</strong></a></p>\n",
    );
}

#[test]
fn complex_inline_markup_still_falls_back() {
    assert_html(
        "visit [**site**](https://example.com) and **outer *inner***",
        "<p>visit <a href=\"https://example.com\"><strong>site</strong></a> and <strong>outer <em>inner</em></strong></p>\n",
    );
}

#[test]
fn parses_reference_style_links_and_shortcuts() {
    assert_html(
        "[A ref][id]\n\n[Shortcut]\n\n[id]: https://example.com \"Ref\"\n[shortcut]: https://shortcut.test",
        "<p><a href=\"https://example.com\" title=\"Ref\">A ref</a></p>\n<p><a href=\"https://shortcut.test\">Shortcut</a></p>\n",
    );
}

#[test]
fn parses_reference_style_images() {
    assert_html(
        "![Logo][brand]\n\n[brand]: https://img.test/logo.png \"Logo title\"",
        "<p><img src=\"https://img.test/logo.png\" alt=\"Logo\" title=\"Logo title\" /></p>\n",
    );
}

#[test]
fn link_url_is_html_escaped() {
    assert_html(
        "[x](https://example.com?a=1&b=2)",
        "<p><a href=\"https://example.com?a=1&amp;b=2\">x</a></p>\n",
    );
}

#[test]
fn unparsable_link_is_left_as_text() {
    assert_html("look [here](missing", "<p>look [here](missing</p>\n");
}

#[test]
fn parses_lists() {
    assert_html(
        "- one\n- two\n\n1. first\n2. second",
        "<ul>\n<li>one</li>\n<li>two</li>\n</ul>\n<ol>\n<li>first</li>\n<li>second</li>\n</ol>\n",
    );
}

#[test]
fn parses_nested_lists_flexible_default() {
    assert_html(
        "- one\n  - two\n    - three",
        "<ul>\n<li>one\n<ul>\n<li>two\n<ul>\n<li>three</li>\n</ul>\n</li>\n</ul>\n</li>\n</ul>\n",
    );
}

#[test]
fn parses_mixed_nested_lists() {
    assert_html(
        "1. one\n  - two\n    1. three",
        "<ol>\n<li>one</li>\n</ol>\n<ul>\n<li>two\n<ol>\n<li>three</li>\n</ol>\n</li>\n</ul>\n",
    );
}

#[test]
fn parses_all_unordered_markers() {
    assert_html(
        "- one\n* two\n+ three",
        "<ul>\n<li>one</li>\n</ul>\n<ul>\n<li>two</li>\n</ul>\n<ul>\n<li>three</li>\n</ul>\n",
    );
}

#[test]
fn ordered_list_requires_digit_dot_space() {
    assert_html("1.one\n1. two", "<p>1.one</p>\n<ol>\n<li>two</li>\n</ol>\n");
}

#[test]
fn parses_blockquotes_and_fences() {
    assert_html(
        "> hello\n> **world**\n\n```rs\nfn main() {}\n```",
        "<blockquote>\n<p>hello\n<strong>world</strong></p>\n</blockquote>\n<pre><code class=\"language-rs\">fn main() {}\n</code></pre>\n",
    );
}

#[test]
fn blockquote_marker_with_optional_space() {
    assert_html(">a\n> b", "<blockquote>\n<p>a\nb</p>\n</blockquote>\n");
}

#[test]
fn fenced_code_without_language() {
    assert_html("```\n<raw>\n```", "<pre><code>&lt;raw&gt;\n</code></pre>\n");
}

#[test]
fn fenced_code_without_closing_fence_consumes_rest() {
    assert_html(
        "```txt\nline 1\nline 2",
        "<pre><code class=\"language-txt\">line 1\nline 2\n</code></pre>\n",
    );
}

#[test]
fn parses_indented_code_block() {
    assert_html(
        "    let x = 1;\n\tlet y = 2;\n\nend",
        "<pre><code>let x = 1;\nlet y = 2;\n</code></pre>\n<p>end</p>\n",
    );
}

#[test]
fn parses_inline_images_with_title() {
    assert_html(
        "![alt text](https://img.test/logo.png \"Logo\")",
        "<p><img src=\"https://img.test/logo.png\" alt=\"alt text\" title=\"Logo\" /></p>\n",
    );
}

#[test]
fn parses_link_title_attribute() {
    assert_html(
        "[Example](https://example.com \"Homepage\")",
        "<p><a href=\"https://example.com\" title=\"Homepage\">Example</a></p>\n",
    );
}

#[test]
fn parses_basic_autolinks() {
    assert_html(
        "<https://example.com> <hello@example.com>",
        "<p><a href=\"https://example.com\">https://example.com</a> <a href=\"mailto:hello@example.com\">hello@example.com</a></p>\n",
    );
}

#[test]
fn backslash_escapes_inline_markers() {
    assert_html("\\*no em\\* and \\[x\\](y)", "<p>*no em* and [x](y)</p>\n");
}

#[test]
fn escaped_html_in_text_and_quotes() {
    assert_html(
        "<script>alert('x')</script> \"quote\"",
        "<script>alert('x')</script> \"quote\"\n",
    );
}

#[test]
fn raw_html_is_not_escaped() {
    assert_html(
        "Use <kbd>Ctrl</kbd> and <em>HTML</em>",
        "<p>Use <kbd>Ctrl</kbd> and <em>HTML</em></p>\n",
    );
}

#[test]
fn raw_html_block_is_passed_through() {
    assert_html(
        "<dl>\n<dt>Term</dt>\n<dd>Definition</dd>\n</dl>\n\ntext",
        "<dl>\n<dt>Term</dt>\n<dd>Definition</dd>\n</dl>\n<p>text</p>\n",
    );
}

#[test]
fn parses_windows_line_endings() {
    assert_html(
        "# h\r\n\r\n- x\r\n- y\r\n",
        "<h1>h</h1>\n<ul>\n<li>x</li>\n<li>y</li>\n</ul>\n",
    );
}

#[test]
fn parses_table_with_alignment() {
    assert_html(
        "| Name | Score | Ratio |\n| :--- | ---: | :---: |\n| Alice | 10 | 1.2 |\n| Bob | 20 | 2.4 |",
        "<table>\n<thead>\n<tr>\n<th style=\"text-align: left\">Name</th>\n<th style=\"text-align: right\">Score</th>\n<th style=\"text-align: center\">Ratio</th>\n</tr>\n</thead>\n<tbody>\n<tr>\n<td style=\"text-align: left\">Alice</td>\n<td style=\"text-align: right\">10</td>\n<td style=\"text-align: center\">1.2</td>\n</tr>\n<tr>\n<td style=\"text-align: left\">Bob</td>\n<td style=\"text-align: right\">20</td>\n<td style=\"text-align: center\">2.4</td>\n</tr>\n</tbody>\n</table>\n",
    );
}

#[test]
fn table_requires_separator_line() {
    assert_html("A | B\nx | y", "<p>A | B\nx | y</p>\n");
}

#[test]
fn table_cells_support_inline_markup() {
    assert_html(
        "| Col |\n| --- |\n| **bold** and [link](https://example.com) |",
        "<table>\n<thead>\n<tr>\n<th>Col</th>\n</tr>\n</thead>\n<tbody>\n<tr>\n<td><strong>bold</strong> and <a href=\"https://example.com\">link</a></td>\n</tr>\n</tbody>\n</table>\n",
    );
}

#[test]
fn parses_horizontal_rules() {
    assert_html("***\n---\n___", "<hr />\n<hr />\n<hr />\n");
}

#[test]
fn hard_breaks_option_converts_soft_breaks() {
    let opts = ParseOptions {
        hard_breaks: true,
        ..Default::default()
    };

    assert_eq!(render_html("1\n2", &opts), "<p>1<br />\n2</p>\n");

    assert_eq!(
        render_html("> 1\n> 2", &opts),
        "<blockquote>\n<p>1<br />\n2</p>\n</blockquote>\n"
    );

    let opts_soft = ParseOptions {
        hard_breaks: false,
        ..Default::default()
    };
    assert_eq!(render_html("1\n2", &opts_soft), "<p>1\n2</p>\n");
}

// ── Strikethrough ──────────────────────────────────────────────────

#[test]
fn parses_strikethrough() {
    assert_html("~~deleted~~", "<p><del>deleted</del></p>\n");
}

#[test]
fn strikethrough_with_inline() {
    assert_html(
        "~~**bold deleted**~~",
        "<p><del><strong>bold deleted</strong></del></p>\n",
    );
}

#[test]
fn strikethrough_disabled() {
    let opts = ParseOptions {
        hard_breaks: false,
        enable_strikethrough: false,
        ..Default::default()
    };
    assert_eq!(render_html("~~deleted~~", &opts), "<p>~~deleted~~</p>\n");
}

#[test]
fn single_tilde_is_literal() {
    assert_html("~text~", "<p>~text~</p>\n");
}

#[test]
fn strikethrough_in_paragraph() {
    assert_html(
        "before ~~del~~ after",
        "<p>before <del>del</del> after</p>\n",
    );
}

// ── Highlight ──────────────────────────────────────────────────────

#[test]
fn parses_highlight() {
    assert_html("==marked==", "<p><mark>marked</mark></p>\n");
}

#[test]
fn highlight_with_inline() {
    assert_html(
        "==*em highlight*==",
        "<p><mark><em>em highlight</em></mark></p>\n",
    );
}

#[test]
fn highlight_disabled() {
    let opts = ParseOptions {
        hard_breaks: false,
        enable_highlight: false,
        ..Default::default()
    };
    assert_eq!(render_html("==marked==", &opts), "<p>==marked==</p>\n");
}

#[test]
fn single_equals_is_literal() {
    assert_html("=text=", "<p>=text=</p>\n");
}

// ── Underline ──────────────────────────────────────────────────────

#[test]
fn parses_underline() {
    assert_html("++underlined++", "<p><u>underlined</u></p>\n");
}

#[test]
fn underline_with_inline() {
    assert_html(
        "++**bold underline**++",
        "<p><u><strong>bold underline</strong></u></p>\n",
    );
}

#[test]
fn underline_disabled() {
    let opts = ParseOptions {
        hard_breaks: false,
        enable_underline: false,
        ..Default::default()
    };
    assert_eq!(
        render_html("++underlined++", &opts),
        "<p>++underlined++</p>\n"
    );
}

// ── Tables toggle ──────────────────────────────────────────────────

#[test]
fn tables_disabled() {
    let opts = ParseOptions {
        hard_breaks: false,
        enable_tables: false,
        ..Default::default()
    };
    let md = "| A |\n| --- |\n| B |";
    let html = render_html(md, &opts);
    assert_eq!(html, "<p>| A |\n| --- |\n| B |</p>\n");
}

// ── Nesting ────────────────────────────────────────────────────────

#[test]
fn highlight_inside_emphasis() {
    assert_html(
        "**==bold highlight==**",
        "<p><strong><mark>bold highlight</mark></strong></p>\n",
    );
}

#[test]
fn strikethrough_inside_highlight() {
    assert_html("==~~both~~==", "<p><mark><del>both</del></mark></p>\n");
}

// ── Backslash escapes for extension delimiters ─────────────────────

#[test]
fn backslash_escapes_extension_delimiters() {
    assert_html("\\~\\~no strike\\~\\~", "<p>~~no strike~~</p>\n");
}

// ── Bare URL and email autolink ─────────────────────────────────────

#[test]
fn bare_url_https() {
    assert_html(
        "https://example.com",
        "<p><a href=\"https://example.com\">https://example.com</a></p>\n",
    );
}

#[test]
fn bare_url_http() {
    assert_html(
        "http://example.com",
        "<p><a href=\"http://example.com\">http://example.com</a></p>\n",
    );
}

#[test]
fn bare_url_in_text() {
    assert_html(
        "visit https://example.com today",
        "<p>visit <a href=\"https://example.com\">https://example.com</a> today</p>\n",
    );
}

#[test]
fn bare_url_with_path() {
    assert_html(
        "https://example.com/path?q=1&b=2",
        "<p><a href=\"https://example.com/path?q=1&amp;b=2\">https://example.com/path?q=1&amp;b=2</a></p>\n",
    );
}

#[test]
fn bare_url_trailing_punctuation_stripped() {
    assert_html(
        "https://example.com.",
        "<p><a href=\"https://example.com\">https://example.com</a>.</p>\n",
    );
}

#[test]
fn bare_url_with_balanced_parens() {
    assert_html(
        "https://en.wikipedia.org/wiki/Rust_(programming_language)",
        "<p><a href=\"https://en.wikipedia.org/wiki/Rust_(programming_language)\">https://en.wikipedia.org/wiki/Rust_(programming_language)</a></p>\n",
    );
}

#[test]
fn bare_email() {
    assert_html(
        "user@example.com",
        "<p><a href=\"mailto:user@example.com\">user@example.com</a></p>\n",
    );
}

#[test]
fn bare_email_in_text() {
    assert_html(
        "contact user@example.com for info",
        "<p>contact <a href=\"mailto:user@example.com\">user@example.com</a> for info</p>\n",
    );
}

#[test]
fn autolink_disabled() {
    let opts = ParseOptions {
        hard_breaks: false,
        enable_autolink: false,
        ..Default::default()
    };
    assert_eq!(
        render_html("https://example.com", &opts),
        "<p>https://example.com</p>\n"
    );
    assert_eq!(
        render_html("user@example.com", &opts),
        "<p>user@example.com</p>\n"
    );
}

#[test]
fn bare_url_not_in_code_span() {
    assert_html(
        "`https://example.com`",
        "<p><code>https://example.com</code></p>\n",
    );
}

#[test]
fn angle_bracket_autolinks_still_work() {
    assert_html(
        "<https://example.com>",
        "<p><a href=\"https://example.com\">https://example.com</a></p>\n",
    );
}

#[test]
fn bare_url_no_scheme_no_match() {
    assert_html("example.com", "<p>example.com</p>\n");
}

#[test]
fn bare_email_no_dot_no_match() {
    assert_html("user@localhost", "<p>user@localhost</p>\n");
}

#[test]
fn bare_url_trailing_comma() {
    assert_html(
        "see https://example.com, ok",
        "<p>see <a href=\"https://example.com\">https://example.com</a>, ok</p>\n",
    );
}

#[test]
fn bare_url_case_insensitive_scheme() {
    assert_html(
        "HTTPS://EXAMPLE.COM",
        "<p><a href=\"HTTPS://EXAMPLE.COM\">HTTPS://EXAMPLE.COM</a></p>\n",
    );
}

// ── Task lists ──────────────────────────────────────────────────────

#[test]
fn task_list_unchecked() {
    assert_html(
        "- [ ] unchecked",
        "<ul>\n<li><input type=\"checkbox\" disabled=\"\" /> unchecked</li>\n</ul>\n",
    );
}

#[test]
fn task_list_checked_lowercase() {
    assert_html(
        "- [x] checked",
        "<ul>\n<li><input type=\"checkbox\" checked=\"\" disabled=\"\" /> checked</li>\n</ul>\n",
    );
}

#[test]
fn task_list_checked_uppercase() {
    assert_html(
        "- [X] checked",
        "<ul>\n<li><input type=\"checkbox\" checked=\"\" disabled=\"\" /> checked</li>\n</ul>\n",
    );
}

#[test]
fn task_list_mixed_items() {
    assert_html(
        "- [ ] todo\n- [x] done\n- normal",
        "<ul>\n<li><input type=\"checkbox\" disabled=\"\" /> todo</li>\n<li><input type=\"checkbox\" checked=\"\" disabled=\"\" /> done</li>\n<li>normal</li>\n</ul>\n",
    );
}

#[test]
fn task_list_ordered() {
    assert_html(
        "1. [ ] first\n2. [x] second",
        "<ol>\n<li><input type=\"checkbox\" disabled=\"\" /> first</li>\n<li><input type=\"checkbox\" checked=\"\" disabled=\"\" /> second</li>\n</ol>\n",
    );
}

#[test]
fn task_list_nested() {
    assert_html(
        "- [ ] parent\n  - [x] child",
        "<ul>\n<li><input type=\"checkbox\" disabled=\"\" /> parent\n<ul>\n<li><input type=\"checkbox\" checked=\"\" disabled=\"\" /> child</li>\n</ul>\n</li>\n</ul>\n",
    );
}

#[test]
fn task_list_disabled_option() {
    let opts = ParseOptions {
        hard_breaks: false,
        enable_task_lists: false,
        ..Default::default()
    };
    assert_eq!(
        render_html("- [ ] unchecked\n- [x] checked", &opts),
        "<ul>\n<li>[ ] unchecked</li>\n<li>[x] checked</li>\n</ul>\n",
    );
}

#[test]
fn task_list_loose() {
    assert_html(
        "- [ ] item one\n\n- [x] item two",
        "<ul>\n<li><input type=\"checkbox\" disabled=\"\" /> \n<p>item one</p>\n</li>\n<li><input type=\"checkbox\" checked=\"\" disabled=\"\" /> \n<p>item two</p>\n</li>\n</ul>\n",
    );
}

// ── permissiveAtxHeaders ────────────────────────────────────────────

#[test]
fn permissive_atx_headers_enabled() {
    let opts = ParseOptions {
        hard_breaks: false,
        permissive_atx_headers: true,
        ..Default::default()
    };
    assert_eq!(render_html("#Hello", &opts), "<h1>Hello</h1>\n");
    assert_eq!(render_html("##World", &opts), "<h2>World</h2>\n");
    assert_eq!(render_html("######Level6", &opts), "<h6>Level6</h6>\n");
}

#[test]
fn permissive_atx_headers_disabled_by_default() {
    // Without permissive mode, `#heading` (no space) stays as paragraph text
    let opts = ParseOptions {
        hard_breaks: false,
        ..Default::default()
    };
    assert_eq!(render_html("#nospace", &opts), "<p>#nospace</p>\n");
}

#[test]
fn permissive_atx_with_normal_headings_still_works() {
    let opts = ParseOptions {
        hard_breaks: false,
        permissive_atx_headers: true,
        ..Default::default()
    };
    // Normal headings with space still work
    assert_eq!(render_html("# Normal", &opts), "<h1>Normal</h1>\n");
    // No-space also works
    assert_eq!(render_html("#NoSpace", &opts), "<h1>NoSpace</h1>\n");
}

// ── noIndentedCodeBlocks ────────────────────────────────────────────

#[test]
fn indented_code_block_enabled_by_default() {
    let opts = ParseOptions {
        hard_breaks: false,
        ..Default::default()
    };
    assert_eq!(
        render_html("    code here", &opts),
        "<pre><code>code here\n</code></pre>\n"
    );
}

#[test]
fn no_indented_code_blocks_treats_as_paragraph() {
    let opts = ParseOptions {
        hard_breaks: false,
        enable_indented_code_blocks: false,
        ..Default::default()
    };
    assert_eq!(render_html("    not code", &opts), "<p>not code</p>\n");
}

#[test]
fn no_indented_code_blocks_fenced_still_works() {
    let opts = ParseOptions {
        hard_breaks: false,
        enable_indented_code_blocks: false,
        ..Default::default()
    };
    assert_eq!(
        render_html("```\nfenced\n```", &opts),
        "<pre><code>fenced\n</code></pre>\n"
    );
}

// ── noHtmlBlocks / noHtmlSpans ──────────────────────────────────────

#[test]
fn no_html_blocks_prevents_html_block_constructs() {
    // With no_html_blocks, a lone HTML tag on its own line is treated as a paragraph,
    // not an HTML block. The inline HTML scanner in the paragraph still passes it through
    // unless no_html_spans is also set.
    let opts = ParseOptions {
        hard_breaks: false,
        no_html_blocks: true,
        ..Default::default()
    };
    // A real HTML block (<script> on its own line) would normally be an HTML block;
    // with no_html_blocks it's rendered as a paragraph with inline HTML still passing through.
    // To fully suppress, use no_html_spans or disable_raw_html.
    let html = render_html("<em>text</em>", &opts);
    // Not an HTML block anymore — it's a paragraph, but inline HTML still passes through
    assert!(
        html.starts_with("<p>"),
        "should be a paragraph, not bare HTML block"
    );
}

#[test]
fn no_html_blocks_and_no_html_spans_together() {
    // Both together fully suppress HTML at block and inline level
    let opts = ParseOptions {
        hard_breaks: false,
        no_html_blocks: true,
        no_html_spans: true,
        ..Default::default()
    };
    let html = render_html("<div>hello</div>", &opts);
    assert!(!html.contains("<div>"), "HTML should be fully escaped");
    assert!(html.contains("&lt;div&gt;"), "should be HTML-escaped");
}

#[test]
fn no_html_spans_escapes_inline_html() {
    let opts = ParseOptions {
        hard_breaks: false,
        no_html_spans: true,
        ..Default::default()
    };
    let html = render_html("text <strong>bold</strong> end", &opts);
    assert!(!html.contains("<strong>"), "inline HTML should be escaped");
    assert!(html.contains("&lt;strong&gt;"), "should be HTML-escaped");
}

#[test]
fn no_html_blocks_does_not_affect_inline_html() {
    // no_html_blocks alone: inline HTML within paragraphs still passes through
    let opts = ParseOptions {
        hard_breaks: false,
        no_html_blocks: true,
        no_html_spans: false,
        ..Default::default()
    };
    // Inline HTML within normal paragraph text should still render
    let html = render_html("text <em>word</em> end", &opts);
    assert!(
        html.contains("<em>word</em>"),
        "inline HTML should still pass through"
    );
}

// ── tagFilter ───────────────────────────────────────────────────────

#[test]
fn tag_filter_blocks_dangerous_tags() {
    let opts = ParseOptions {
        hard_breaks: false,
        tag_filter: true,
        ..Default::default()
    };
    for tag in &[
        "script",
        "iframe",
        "style",
        "textarea",
        "title",
        "xmp",
        "noembed",
        "noframes",
        "plaintext",
    ] {
        let md = format!("<{tag}>content</{tag}>");
        let html = render_html(&md, &opts);
        assert!(
            !html.contains(&format!("<{tag}>")),
            "tag <{tag}> should be filtered"
        );
        assert!(
            html.contains(&format!("&lt;{tag}&gt;")),
            "tag <{tag}> should be escaped"
        );
    }
}

#[test]
fn tag_filter_allows_safe_tags() {
    let opts = ParseOptions {
        hard_breaks: false,
        tag_filter: true,
        ..Default::default()
    };
    let html = render_html("text <em>word</em> end", &opts);
    assert!(
        html.contains("<em>word</em>"),
        "safe inline tags should pass through"
    );
}

#[test]
fn tag_filter_case_insensitive() {
    let opts = ParseOptions {
        hard_breaks: false,
        tag_filter: true,
        ..Default::default()
    };
    let html = render_html("<SCRIPT>x</SCRIPT>", &opts);
    assert!(
        !html.contains("<SCRIPT>"),
        "tag filter should be case-insensitive"
    );
}

// ── collapseWhitespace ──────────────────────────────────────────────

#[test]
fn collapse_whitespace_reduces_spaces() {
    let opts = ParseOptions {
        hard_breaks: false,
        collapse_whitespace: true,
        ..Default::default()
    };
    // Multiple spaces collapsed to one
    assert_eq!(render_html("a    b", &opts), "<p>a b</p>\n");
}

#[test]
fn collapse_whitespace_tabs_too() {
    let opts = ParseOptions {
        hard_breaks: false,
        collapse_whitespace: true,
        ..Default::default()
    };
    assert_eq!(render_html("a\t\tb", &opts), "<p>a b</p>\n");
}

#[test]
fn collapse_whitespace_does_not_affect_code_spans() {
    let opts = ParseOptions {
        hard_breaks: false,
        collapse_whitespace: true,
        ..Default::default()
    };
    // Content inside `` `...` `` must not be collapsed
    assert_eq!(
        render_html("`a    b`", &opts),
        "<p><code>a    b</code></p>\n"
    );
}

// ── Heading IDs ─────────────────────────────────────────────────────

#[test]
fn heading_id_simple() {
    let opts = ParseOptions {
        hard_breaks: false,
        enable_heading_ids: true,
        ..Default::default()
    };
    assert_eq!(
        render_html("# Hello World", &opts),
        "<h1 id=\"hello-world\">Hello World</h1>\n"
    );
}

#[test]
fn heading_id_strips_markdown() {
    let opts = ParseOptions {
        hard_breaks: false,
        enable_heading_ids: true,
        ..Default::default()
    };
    // Bold inside heading → slug is plain text
    assert_eq!(
        render_html("## **Bold** Text", &opts),
        "<h2 id=\"bold-text\"><strong>Bold</strong> Text</h2>\n"
    );
}

#[test]
fn heading_id_special_chars() {
    let opts = ParseOptions {
        hard_breaks: false,
        enable_heading_ids: true,
        ..Default::default()
    };
    assert_eq!(
        render_html("# Hello, World!", &opts),
        "<h1 id=\"hello-world\">Hello, World!</h1>\n"
    );
}

#[test]
fn heading_anchor_link() {
    let opts = ParseOptions {
        hard_breaks: false,
        enable_heading_ids: true,
        enable_heading_anchors: true,
        ..Default::default()
    };
    let html = render_html("# Title", &opts);
    assert!(html.contains("id=\"title\""));
    assert!(html.contains("<a class=\"anchor\" href=\"#title\">"));
}

#[test]
fn heading_ids_disabled_by_default() {
    let opts = ParseOptions {
        hard_breaks: false,
        ..Default::default()
    };
    let html = render_html("# Hello", &opts);
    assert!(
        !html.contains("id="),
        "heading IDs should be off by default"
    );
}

// ── wikiLinks ───────────────────────────────────────────────────────

#[test]
fn wiki_link_basic() {
    let opts = ParseOptions {
        hard_breaks: false,
        enable_wiki_links: true,
        ..Default::default()
    };
    assert_eq!(
        render_html("[[Hello World]]", &opts),
        "<p><a href=\"hello_world\">Hello World</a></p>\n"
    );
}

#[test]
fn wiki_link_simple_slug() {
    let opts = ParseOptions {
        hard_breaks: false,
        enable_wiki_links: true,
        ..Default::default()
    };
    assert_eq!(
        render_html("[[page]]", &opts),
        "<p><a href=\"page\">page</a></p>\n"
    );
}

#[test]
fn wiki_link_disabled_by_default() {
    let opts = ParseOptions {
        hard_breaks: false,
        ..Default::default()
    };
    let html = render_html("[[wiki link]]", &opts);
    // Falls through as normal bracket text
    assert!(
        !html.contains("<a href="),
        "wiki links should be off by default"
    );
}

#[test]
fn wiki_link_no_newline_inside() {
    // Multi-line wiki links are not supported (security / no multi-line)
    let opts = ParseOptions {
        hard_breaks: false,
        enable_wiki_links: true,
        ..Default::default()
    };
    // The [[ spans a newline → should NOT become a wiki link
    let html = render_html("[[line\none]]", &opts);
    assert!(!html.contains("<a href="), "no multi-line wiki links");
}

#[test]
fn wiki_link_html_escaped_text() {
    let opts = ParseOptions {
        hard_breaks: false,
        enable_wiki_links: true,
        ..Default::default()
    };
    let html = render_html("[[<xss>]]", &opts);
    assert!(
        !html.contains("<xss>"),
        "wiki link text must be HTML-escaped"
    );
}

// ── latexMath ────────────────────────────────────────────────────────

#[test]
fn math_inline_basic() {
    let opts = ParseOptions {
        hard_breaks: false,
        enable_latex_math: true,
        ..Default::default()
    };
    assert_eq!(
        render_html("$x + y$", &opts),
        "<p><span class=\"math-inline\">x + y</span></p>\n"
    );
}

#[test]
fn math_display_basic() {
    let opts = ParseOptions {
        hard_breaks: false,
        enable_latex_math: true,
        ..Default::default()
    };
    assert_eq!(
        render_html("$$E = mc^2$$", &opts),
        "<p><span class=\"math-display\">E = mc^2</span></p>\n"
    );
}

#[test]
fn math_content_html_escaped() {
    let opts = ParseOptions {
        hard_breaks: false,
        enable_latex_math: true,
        ..Default::default()
    };
    // < and > inside math must be escaped (security)
    let html = render_html("$a < b$", &opts);
    assert!(html.contains("&lt;"), "math content must be HTML-escaped");
    assert!(
        !html.contains("<b>"),
        "math content must not contain raw HTML"
    );
}

#[test]
fn math_disabled_by_default() {
    let opts = ParseOptions {
        hard_breaks: false,
        ..Default::default()
    };
    let html = render_html("$x + y$", &opts);
    // Dollar signs are literal
    assert!(html.contains("$x + y$"), "math should be off by default");
    assert!(!html.contains("math-inline"), "no math spans when disabled");
}

#[test]
fn math_inline_no_newline() {
    // Inline math must not span lines
    let opts = ParseOptions {
        hard_breaks: false,
        enable_latex_math: true,
        ..Default::default()
    };
    let html = render_html("$a\nb$", &opts);
    // No math-inline span — should fall back to literal $
    assert!(
        !html.contains("math-inline"),
        "inline math should not span newlines"
    );
}

// --- Render-mode source-range bookkeeping (zero-copy paragraphs/headings/code) ---

#[test]
fn empty_code_block_does_not_steal_next_range() {
    let opts = ParseOptions::default();
    // Top level
    let html = render_html("```\n```\n\n```\nxyz\n```\n", &opts);
    assert_eq!(
        html,
        "<pre><code></code></pre>\n<pre><code>xyz\n</code></pre>\n"
    );
    // Inside blockquote
    let html = render_html("> ```\n> ```\n\n```\nabc\n```\n", &opts);
    assert!(html.starts_with("<blockquote>\n<pre><code></code></pre>\n</blockquote>"));
    assert!(html.contains("<pre><code>abc\n</code></pre>"));
    // Inside list item
    let html = render_html("- ```\n  ```\n\n```\nqqq\n```\n", &opts);
    assert!(html.contains("<li>\n<pre><code></code></pre>\n</li>"));
    assert!(html.ends_with("<pre><code>qqq\n</code></pre>\n"));
}

#[test]
fn zero_copy_paragraph_range_sync() {
    let opts = ParseOptions {
        hard_breaks: false,
        ..Default::default()
    };
    // Mixed paragraphs, headings, and code blocks must each get their own text.
    let html = render_html(
        "first para\n\n# Head\n\n```\ncode\n```\n\nsecond para\nwith continuation\n\n## Tail\n",
        &opts,
    );
    assert!(html.contains("<p>first para</p>"));
    assert!(html.contains(">Head<"));
    assert!(html.contains("<pre><code>code\n</code></pre>"));
    assert!(html.contains("<p>second para\nwith continuation</p>"));
    assert!(html.contains(">Tail<"));
}

#[test]
fn empty_atx_heading_keeps_ranges_in_sync() {
    let opts = ParseOptions::default();
    let html = render_html("#\n\npara\n\n## x\n", &opts);
    assert!(html.contains("<h1></h1>"));
    assert!(html.contains("<p>para</p>"));
    assert!(html.contains(">x<"));
}

#[test]
fn paragraph_trailing_whitespace_trimmed_hard_break_kept() {
    let opts = ParseOptions {
        hard_breaks: false,
        ..Default::default()
    };
    // Inner trailing two spaces = hard break; paragraph-final trailing ws trimmed.
    let html = render_html("foo  \nbar   \n\nnext\n", &opts);
    assert!(
        html.contains("<br />"),
        "inner hard break must survive: {html}"
    );
    assert!(
        html.contains("bar</p>"),
        "final trailing ws must be trimmed: {html}"
    );
    assert!(html.contains("<p>next</p>"));
}

#[test]
fn setext_heading_from_zero_copy_paragraph() {
    let opts = ParseOptions::default();
    let html = render_html("Title\n=====\n\npara\n\nSub\n---\n", &opts);
    assert!(html.contains(">Title<"));
    assert!(html.contains("<p>para</p>"));
    assert!(html.contains(">Sub<"));
}

#[test]
fn setext_heading_after_ref_def_paragraph() {
    let opts = ParseOptions::default();
    let html = render_html("[r]: /url\nTitle\n=====\n\n[r]\n", &opts);
    assert!(html.contains(">Title<"), "{html}");
    assert!(html.contains("<a href=\"/url\">r</a>"), "{html}");
}

#[test]
fn table_from_zero_copy_paragraph_header() {
    let opts = ParseOptions::default();
    let html = render_html("a | b\n--- | ---\n1 | 2\n\npara\n", &opts);
    assert!(html.contains("<th>a</th>"), "{html}");
    assert!(html.contains("<td>2</td>"), "{html}");
    assert!(html.contains("<p>para</p>"), "{html}");
}

#[test]
fn ref_def_only_paragraph_emits_nothing_but_resolves() {
    let opts = ParseOptions::default();
    let html = render_html("[foo]: /bar\n\n[foo]\n\n# H\n", &opts);
    assert!(html.contains("<a href=\"/bar\">foo</a>"), "{html}");
    assert!(!html.contains("[foo]:"), "{html}");
    assert!(html.contains(">H<"), "{html}");
}

#[test]
fn bracket_paragraph_not_a_ref_def() {
    let opts = ParseOptions::default();
    let html = render_html("[notadef] text\n\nnext\n", &opts);
    assert!(html.contains("<p>[notadef] text</p>"), "{html}");
    assert!(html.contains("<p>next</p>"), "{html}");
}

#[test]
fn crlf_paragraph_continuation() {
    let opts = ParseOptions {
        hard_breaks: false,
        ..Default::default()
    };
    let html = render_html("alpha\r\nbeta\r\n\r\ngamma\r\n", &opts);
    assert!(html.contains("<p>alpha\nbeta</p>"), "{html}");
    assert!(html.contains("<p>gamma</p>"), "{html}");
}

#[test]
fn lazy_continuation_into_blockquote_paragraph() {
    let opts = ParseOptions {
        hard_breaks: false,
        ..Default::default()
    };
    let html = render_html("> quoted\nlazy\n\nafter\n", &opts);
    assert!(html.contains("<p>quoted\nlazy</p>"), "{html}");
    assert!(html.contains("<p>after</p>"), "{html}");
}

#[test]
fn heading_ids_from_zero_copy_raw() {
    let opts = ParseOptions {
        enable_heading_ids: true,
        ..Default::default()
    };
    let html = render_html("# My Heading\n\npara\n\nSetext Head\n===\n", &opts);
    assert!(html.contains("id=\"my-heading\""), "{html}");
    assert!(html.contains("id=\"setext-head\""), "{html}");
}

#[test]
fn indented_paragraph_with_leading_ws_and_brackets() {
    let opts = ParseOptions::default();
    // Leading-whitespace slice path in finalize (starts with space after marker handling)
    let html = render_html("   spaced para\n\n   [x] not a def\n", &opts);
    assert!(html.contains("<p>spaced para</p>"), "{html}");
    assert!(html.contains("<p>[x] not a def</p>"), "{html}");
}
