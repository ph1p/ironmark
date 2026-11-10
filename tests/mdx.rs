//! MDX extension: block/inline recognition, HTML passthrough and JSX output.
#![cfg(feature = "html")]

use ironmark::{Block, ParseOptions, parse_markdown, render_html, render_jsx};

fn mdx_opts() -> ParseOptions {
    ParseOptions {
        enable_mdx: true,
        ..Default::default()
    }
}

fn jsx(md: &str) -> String {
    render_jsx(md, &ParseOptions::default())
        .unwrap_or_else(|e| panic!("{md:?}: {e}"))
        .jsx
}

fn body(md: &str) -> String {
    let out = jsx(md);
    out.strip_prefix("<>\n")
        .and_then(|s| s.strip_suffix("\n</>"))
        .unwrap_or(&out)
        .to_string()
}

#[test]
fn off_by_default() {
    let html = render_html("<Foo />\n\n{x}", &ParseOptions::default());
    assert_eq!(html, "<Foo />\n<p>{x}</p>\n");
    let doc = parse_markdown("import a from 'b'", &ParseOptions::default());
    assert!(
        matches!(doc, Block::Document { ref children } if matches!(children[0], Block::Paragraph { .. }))
    );
}

#[test]
fn ast_blocks() {
    let doc = parse_markdown(
        "import a from 'b'\nexport const c = 1\n\n<X y={1} />\n\n<Y\n/>\n\n{a}",
        &mdx_opts(),
    );
    let Block::Document { children } = doc else {
        panic!()
    };
    assert_eq!(
        children,
        vec![
            Block::MdxEsm {
                literal: "import a from 'b'\nexport const c = 1".into()
            },
            Block::MdxFlow {
                literal: "<X y={1} />".into()
            },
            Block::MdxFlow {
                literal: "<Y\n/>".into()
            },
            Block::MdxFlow {
                literal: "{a}".into()
            },
        ]
    );
}

#[test]
fn esm_only_at_top_level_column_zero() {
    let out = render_jsx(
        "  import a from 'b'\n\n> import c from 'd'\n\npara\nimport e from 'f'",
        &ParseOptions::default(),
    )
    .unwrap();
    assert_eq!(out.esm, "");
    let out = render_jsx(
        "import a from 'b'\n\nexport function f() {\n  return 1\n}\n\nhi",
        &ParseOptions::default(),
    )
    .unwrap();
    assert_eq!(
        out.esm,
        "import a from 'b'\nexport function f() {\n  return 1\n}"
    );
    assert_eq!(out.jsx, "<>\n<p>hi</p>\n</>");
}

#[test]
fn flow_interrupts_paragraph() {
    assert_eq!(
        body("text\n<Foo />\nmore"),
        "<p>text</p>\n<Foo />\n<p>more</p>"
    );
}

#[test]
fn markdown_inside_components() {
    assert_eq!(
        body("<Callout type=\"warn\">\n\n**bold**\n\n- a\n\n</Callout>"),
        "<Callout type=\"warn\">\n<p><strong>bold</strong></p>\n<ul>\n<li>a</li>\n</ul>\n</Callout>"
    );
    assert_eq!(
        body("> <Note>\n> hi\n> </Note>"),
        "<blockquote>\n<Note>\n<p>hi</p>\n</Note>\n</blockquote>"
    );
}

#[test]
fn multiline_flow() {
    assert_eq!(
        body("<Chart\n  data={[1, 2]}\n  opts={{ a: \"}\" }}\n/>\n\nafter"),
        "<Chart\n  data={[1, 2]}\n  opts={{ a: \"}\" }}\n/>\n<p>after</p>"
    );
    assert_eq!(
        body("{items.map(i => (\n  <li key={i}>{i}</li>\n))}"),
        "{items.map(i => (\n  <li key={i}>{i}</li>\n))}"
    );
}

#[test]
fn unterminated_constructs_fall_back_to_text() {
    assert_eq!(body("<Foo bar={\n"), "<p>&lt;Foo bar=&#123;</p>");
    assert_eq!(body("a < b and {open"), "<p>a &lt; b and &#123;open</p>");
}

#[test]
fn inline_pieces() {
    assert_eq!(
        body("Hi {props.name}, see <Badge kind=\"new\">*fresh*</Badge>."),
        "<p>Hi {props.name}, see <Badge kind=\"new\"><em>fresh</em></Badge>.</p>"
    );
    assert_eq!(
        body("`<A>{x}`"),
        "<p><code>&lt;A&gt;&#123;x&#125;</code></p>"
    );
    assert_eq!(
        body("\\{not an expression\\}"),
        "<p>&#123;not an expression&#125;</p>"
    );
}

#[test]
fn preset_disables_indented_code_and_html() {
    assert_eq!(body("    not code"), "<p>not code</p>");
    assert_eq!(body("<!-- c -->"), "<p>&lt;!-- c --&gt;</p>");
}

#[test]
fn code_blocks_keep_whitespace() {
    assert_eq!(
        body("```\n{a}\n\n  <b>\n```"),
        "<pre><code>{\"{a}\\n\\n  <b>\\n\"}</code></pre>"
    );
}

#[test]
fn newlines_in_text_survive() {
    let opts = ParseOptions {
        hard_breaks: false,
        ..Default::default()
    };
    let out = render_jsx("*a*\nb", &opts).unwrap();
    assert_eq!(out.jsx, "<>\n<p><em>a</em>{\"\\n\"}b</p>\n</>");
}

#[test]
fn html_attributes_become_jsx() {
    let out = body("| a |\n|:-:|\n| b |\n\n- [x] done\n\n```rust\nx\n```");
    assert!(
        out.contains("<th style={{textAlign: \"center\"}}>a</th>"),
        "{out}"
    );
    assert!(
        out.contains("<input type=\"checkbox\" checked disabled />"),
        "{out}"
    );
    assert!(out.contains("<code className=\"language-rust\">"), "{out}");
}

#[test]
fn nesting_is_checked() {
    let o = ParseOptions::default();
    for bad in [
        "<A>",
        "<A>\n\n<B>\n\n</A>",
        "x\n\n</A>",
        "a <b>c\n\nd</b>",
        "<a><b></a></b>",
    ] {
        assert!(render_jsx(bad, &o).is_err(), "{bad:?} should fail");
    }
    let err = render_jsx("<A>\n\n<B>\n\n</A>", &o).unwrap_err();
    assert_eq!(
        err.message(),
        "unexpected closing tag `</A>`, expected `</B>`"
    );
}

#[test]
fn nul_is_replaced() {
    assert_eq!(body("a\0b"), "<p>a\u{FFFD}b</p>");
}

#[test]
fn html_mode_passes_through() {
    let html = render_html("import a from 'b'\n\n<X>\n\n*hi* {y}\n\n</X>", &mdx_opts());
    assert_eq!(html, "<X>\n<p><em>hi</em> {y}</p>\n</X>\n");
    let safe = ParseOptions {
        disable_raw_html: true,
        ..mdx_opts()
    };
    assert_eq!(
        render_html("<X />\n\na <b>c</b>", &safe),
        "&lt;X /&gt;\n<p>a &lt;b&gt;c&lt;/b&gt;</p>\n"
    );
}

#[test]
fn to_module() {
    let out = render_jsx("import X from './x'\n\n<X />", &ParseOptions::default()).unwrap();
    assert_eq!(
        out.to_module(),
        "import X from './x'\n\nexport default function MDXContent(props) {\n  return <>\n<X />\n</>;\n}\n"
    );
}

#[test]
fn provider_module() {
    let jsx = ironmark::JsxOptions {
        provider_import_source: Some("@mdx-js/react".into()),
        ..Default::default()
    };
    let out = ironmark::render_jsx_with(
        "import { Meta } from \"@storybook/addon-docs/blocks\"\n\n<Meta title=\"A\" />\n\n# Hi `x`\n\n<Note />",
        &ParseOptions::default(),
        &jsx,
    )
    .unwrap();
    assert_eq!(out.references, ["Note"]);
    assert_eq!(
        out.to_module(),
        r#"import { useMDXComponents as _provideComponents } from "@mdx-js/react";
import { Meta } from "@storybook/addon-docs/blocks"

function _createMdxContent(props) {
  const _components = {h1: "h1", code: "code", ..._provideComponents(), ...props.components};
  const {Note} = _components;
  if (!Note) _missingMdxReference("Note");
  return <>
<Meta title="A" />
<_components.h1>Hi <_components.code>x</_components.code></_components.h1>
<Note />
</>;
}

export default function MDXContent(props = {}) {
  const { wrapper: MDXLayout } = {..._provideComponents(), ...props.components};
  return MDXLayout ? <MDXLayout {...props}><_createMdxContent {...props} /></MDXLayout> : _createMdxContent(props);
}

function _missingMdxReference(id) {
  throw new Error("Expected component `" + id + "` to be defined: you likely forgot to import, pass, or provide it.");
}
"#
    );
}
