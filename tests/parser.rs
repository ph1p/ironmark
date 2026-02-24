use ironmark::{ParseOptions, parse};

fn assert_html(md: &str, expected: &str) {
    let opts = ParseOptions {
        hard_breaks: false,
        ..Default::default()
    };
    assert_eq!(parse(md, &opts), expected);
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

    assert_eq!(parse("1\n2", &opts), "<p>1<br />\n2</p>\n");

    assert_eq!(
        parse("> 1\n> 2", &opts),
        "<blockquote>\n<p>1<br />\n2</p>\n</blockquote>\n"
    );

    assert_eq!(
        parse("1\n2", &ParseOptions::default()),
        "<p>1<br />\n2</p>\n"
    );
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
    assert_eq!(parse("~~deleted~~", &opts), "<p>~~deleted~~</p>\n");
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
    assert_eq!(parse("==marked==", &opts), "<p>==marked==</p>\n");
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
    assert_eq!(parse("++underlined++", &opts), "<p>++underlined++</p>\n");
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
    let html = parse(md, &opts);
    assert!(!html.contains("<table>"));
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
        parse("https://example.com", &opts),
        "<p>https://example.com</p>\n"
    );
    assert_eq!(
        parse("user@example.com", &opts),
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
        parse("- [ ] unchecked\n- [x] checked", &opts),
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
