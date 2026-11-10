//! MDX → JSX compilation.
//!
//! [`render_jsx`] renders the document to HTML with every MDX piece wrapped
//! in NUL sentinels (see [`super::push_sentinel`]), then rewrites that HTML
//! into JSX in one linear pass:
//!
//! - ironmark's own HTML tags get JSX attribute names (`class` → `className`,
//!   `style="text-align: left"` → `style={{textAlign: "left"}}`, boolean
//!   `checked=""` → `checked`);
//! - text braces become `&#123;`/`&#125;` so they stay literal;
//! - code block content becomes a JS string literal so whitespace survives;
//! - newlines inside text become `{"\n"}` (JSX would otherwise fold them away
//!   together with the neighbouring space);
//! - MDX pieces are emitted verbatim, and every open tag — HTML or JSX — is
//!   checked against its closing tag.

use std::borrow::Cow;
use std::fmt;

use super::SENTINEL;
use crate::ParseOptions;
use crate::entities::decode_entities;

/// How [`render_jsx_with`] emits JSX.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct JsxOptions {
    /// Route elements through a `_components` object, like `@mdx-js/mdx`.
    ///
    /// Markdown-generated tags become `<_components.h1>` (overridable through
    /// `props.components`), and capitalised components the ESM does not
    /// declare are looked up in `_components` instead of being free variables.
    /// [`MdxOutput::to_module`] then emits the matching component prelude, a
    /// `wrapper` layout and a "component is not defined" check.
    ///
    /// Default: `false` (plain `<h1>`, components must be imported).
    pub components: bool,
    /// Module to import `useMDXComponents` from (e.g. `"@mdx-js/react"`), so an
    /// `MDXProvider` can supply components. Implies [`components`](Self::components).
    ///
    /// Default: `None`.
    pub provider_import_source: Option<String>,
}

/// The result of compiling MDX with [`render_jsx`].
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct MdxOutput {
    /// Top-level `import`/`export` statements, in document order, separated by newlines.
    pub esm: String,
    /// The document body as a single JSX fragment (`<>…</>`).
    ///
    /// In components mode it refers to `_components`, which only
    /// [`to_module`](Self::to_module) defines.
    pub jsx: String,
    /// Whether the body was compiled in components mode ([`JsxOptions::components`]).
    pub components: bool,
    /// Markdown-generated tag names routed through `_components`, in first-use order.
    pub elements: Vec<String>,
    /// Components the body uses but the ESM does not declare, in first-use order;
    /// in components mode they are taken from `_components`.
    pub references: Vec<String>,
    /// See [`JsxOptions::provider_import_source`].
    pub provider_import_source: Option<String>,
}

impl MdxOutput {
    /// Assemble a JavaScript module: the ESM statements followed by
    /// `export default function MDXContent(props) { return <>…</>; }`.
    ///
    /// The module still contains JSX, so it needs a JSX transform (esbuild,
    /// Babel, SWC, Oxc, TypeScript) before it runs. Without components mode,
    /// components must be imported by the document itself; with it, the module
    /// follows `@mdx-js/mdx`: components come from the provider (if any), then
    /// `props.components`, and `components.wrapper` wraps the content.
    pub fn to_module(&self) -> String {
        let mut out = String::with_capacity(self.esm.len() + self.jsx.len() + 512);
        if let Some(source) = &self.provider_import_source {
            out.push_str("import { useMDXComponents as _provideComponents } from ");
            push_js_string(&mut out, source);
            out.push_str(";\n");
        }
        if !self.esm.is_empty() {
            out.push_str(&self.esm);
            out.push('\n');
        }
        if !out.is_empty() {
            out.push('\n');
        }
        if !self.components {
            out.push_str("export default function MDXContent(props) {\n  return ");
            out.push_str(&self.jsx);
            out.push_str(";\n}\n");
            return out;
        }
        let provide = if self.provider_import_source.is_some() {
            "..._provideComponents(), "
        } else {
            ""
        };
        out.push_str("function _createMdxContent(props) {\n  const _components = {");
        for el in &self.elements {
            out.push_str(el);
            out.push_str(": \"");
            out.push_str(el);
            out.push_str("\", ");
        }
        out.push_str(provide);
        out.push_str("...props.components};\n");
        if !self.references.is_empty() {
            out.push_str("  const {");
            out.push_str(&self.references.join(", "));
            out.push_str("} = _components;\n");
            for r in &self.references {
                out.push_str("  if (!");
                out.push_str(r);
                out.push_str(") _missingMdxReference(\"");
                out.push_str(r);
                out.push_str("\");\n");
            }
        }
        out.push_str("  return ");
        out.push_str(&self.jsx);
        out.push_str(";\n}\n\nexport default function MDXContent(props = {}) {\n  const { wrapper: MDXLayout } = {");
        out.push_str(provide);
        out.push_str("...props.components};\n  return MDXLayout ? <MDXLayout {...props}><_createMdxContent {...props} /></MDXLayout> : _createMdxContent(props);\n}\n");
        if !self.references.is_empty() {
            out.push_str("\nfunction _missingMdxReference(id) {\n  throw new Error(\"Expected component `\" + id + \"` to be defined: you likely forgot to import, pass, or provide it.\");\n}\n");
        }
        out
    }
}

/// An MDX document that cannot be compiled to valid JSX.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MdxError {
    message: String,
}

impl MdxError {
    fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }

    /// Human-readable description of the problem.
    pub fn message(&self) -> &str {
        &self.message
    }
}

impl fmt::Display for MdxError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "MDX error: {}", self.message)
    }
}

impl std::error::Error for MdxError {}

/// Compile an MDX document to JSX.
///
/// `options` are used as given except that [`ParseOptions::enable_mdx`] is
/// forced on. The returned [`MdxOutput`] holds the ESM statements and the body
/// as a JSX fragment; [`MdxOutput::to_module`] joins them into a module.
///
/// Fails when the JSX does not nest (`<A><B></A>`), a tag is never closed, or
/// a tag or expression is left unterminated at the end of the document.
///
/// **Security:** MDX is code. Expressions and components run with the full
/// privileges of the page that renders them; `disable_raw_html` and URL
/// sanitising do not make compiled MDX safe for untrusted input.
///
/// # Examples
///
/// ```
/// use ironmark::{render_jsx, ParseOptions};
///
/// let out = render_jsx("import X from './x'\n\n# Hi {props.name}\n\n<X />", &ParseOptions::default()).unwrap();
/// assert_eq!(out.esm, "import X from './x'");
/// assert!(out.jsx.contains("<h1>Hi {props.name}</h1>"));
/// assert!(out.jsx.contains("<X />"));
/// ```
pub fn render_jsx(markdown: &str, options: &ParseOptions) -> Result<MdxOutput, MdxError> {
    render_jsx_with(markdown, options, &JsxOptions::default())
}

/// Compile an MDX document to JSX with [`JsxOptions`].
///
/// Same as [`render_jsx`], plus `@mdx-js/mdx`-style component mapping when
/// [`JsxOptions::components`] or [`JsxOptions::provider_import_source`] is set.
///
/// # Examples
///
/// ```
/// use ironmark::{render_jsx_with, JsxOptions, ParseOptions};
///
/// let jsx = JsxOptions { provider_import_source: Some("@mdx-js/react".into()), ..Default::default() };
/// let out = render_jsx_with("# Hi\n\n<Note />", &ParseOptions::default(), &jsx).unwrap();
/// assert!(out.jsx.contains("<_components.h1>Hi</_components.h1>"));
/// assert_eq!(out.references, ["Note"]);
/// assert!(out.to_module().contains("const {Note} = _components;"));
/// ```
pub fn render_jsx_with(
    markdown: &str,
    options: &ParseOptions,
    jsx: &JsxOptions,
) -> Result<MdxOutput, MdxError> {
    compile(markdown, options, jsx, true)
}

/// Compile plain Markdown (no MDX syntax) to JSX with [`JsxOptions`].
///
/// Like [`render_jsx_with`], but [`ParseOptions::enable_mdx`] is forced off:
/// braces stay literal text, `import`/`export` lines are paragraphs, and there
/// is no ESM. Raw HTML is rewritten as JSX (`class` → `className`, void tags
/// self-closed) and HTML comments are dropped, so the HTML must still nest.
///
/// # Examples
///
/// ```
/// use ironmark::{render_markdown_jsx, JsxOptions, ParseOptions};
///
/// let out = render_markdown_jsx("# Hi {x}\n\n<div class=\"a\">b</div>\n", &ParseOptions::default(), &JsxOptions::default()).unwrap();
/// assert!(out.esm.is_empty());
/// assert!(out.jsx.contains("<h1>Hi &#123;x&#125;</h1>"));
/// assert!(out.jsx.contains("<div className=\"a\">b</div>"));
/// ```
pub fn render_markdown_jsx(
    markdown: &str,
    options: &ParseOptions,
    jsx: &JsxOptions,
) -> Result<MdxOutput, MdxError> {
    compile(markdown, options, jsx, false)
}

fn compile(
    markdown: &str,
    options: &ParseOptions,
    jsx: &JsxOptions,
    mdx: bool,
) -> Result<MdxOutput, MdxError> {
    // NUL is the sentinel byte; keep real NULs out of the rendered HTML.
    let markdown: Cow<'_, str> = if markdown.contains(SENTINEL) {
        Cow::Owned(markdown.replace(SENTINEL, "\u{FFFD}"))
    } else {
        Cow::Borrowed(markdown)
    };
    let opts = ParseOptions {
        enable_mdx: mdx,
        ..*options
    };
    let html = crate::block::render_html_impl(&markdown, &opts, true);
    convert(&html, jsx)
}

/// One lexical unit of sentinel-annotated HTML.
#[derive(Clone, Copy, Debug)]
enum Tok<'a> {
    /// Plain (already HTML-escaped) text.
    Text(&'a str),
    /// An HTML tag written by ironmark's renderer.
    Html(&'a str),
    /// A sentinel-wrapped MDX piece: marker and literal.
    Mdx(char, &'a str),
}

fn tokenize(html: &str) -> Result<Vec<Tok<'_>>, MdxError> {
    let b = html.as_bytes();
    let mut toks = Vec::with_capacity(html.len() / 16 + 4);
    let mut i = 0;
    while i < b.len() {
        match b[i] {
            0 => {
                // `\0`, marker byte (ASCII), literal, `\0`.
                let marker = *b
                    .get(i + 1)
                    .ok_or_else(|| MdxError::new("internal: truncated sentinel"))?
                    as char;
                let body = i + 2;
                let end = memchr::memchr(0, &b[body..])
                    .map(|o| body + o)
                    .ok_or_else(|| MdxError::new("internal: unterminated sentinel"))?;
                toks.push(Tok::Mdx(marker, &html[body..end]));
                i = end + 1;
            }
            b'<' if b[i + 1..].starts_with(b"!--") => {
                // Raw HTML comment (plain Markdown only): JSX has no comments, drop it.
                i = html[i + 4..].find("-->").map_or(b.len(), |e| i + 4 + e + 3);
            }
            b'<' if matches!(b.get(i + 1), Some(b'!' | b'?')) => {
                // `<!DOCTYPE …>`, `<![CDATA[…]]>`, `<?…?>`: no JSX equivalent, drop it.
                i = memchr::memchr(b'>', &b[i..]).map_or(b.len(), |o| i + o + 1);
            }
            b'<' => {
                // Renderer output: attribute values never contain a raw `>`.
                let end = memchr::memchr(b'>', &b[i..])
                    .map(|o| i + o + 1)
                    .ok_or_else(|| MdxError::new("internal: unterminated HTML tag"))?;
                toks.push(Tok::Html(&html[i..end]));
                i = end;
            }
            _ => {
                let end = memchr::memchr2(0, b'<', &b[i..]).map_or(b.len(), |o| i + o);
                toks.push(Tok::Text(&html[i..end]));
                i = end;
            }
        }
    }
    Ok(toks)
}

/// HTML elements ironmark emits that sit on their own lines (whitespace
/// around them is layout, not content).
fn is_block_tag(name: &str) -> bool {
    matches!(
        name,
        "p" | "h1"
            | "h2"
            | "h3"
            | "h4"
            | "h5"
            | "h6"
            | "ul"
            | "ol"
            | "li"
            | "blockquote"
            | "pre"
            | "hr"
            | "table"
            | "thead"
            | "tbody"
            | "tr"
            | "th"
            | "td"
            | "div"
            | "section"
            | "details"
            | "summary"
    )
}

/// Name of an HTML or JSX tag literal (`<a …>`, `</a>`, `<a.b />`, `<>`).
fn tag_name(lit: &str) -> &str {
    let s = lit.strip_prefix('<').unwrap_or(lit);
    let s = s.strip_prefix('/').unwrap_or(s).trim_start();
    let end = s
        .find(|c: char| c.is_ascii_whitespace() || matches!(c, '/' | '>' | '{'))
        .unwrap_or(s.len());
    &s[..end]
}

/// Whether a token is layout-level: block HTML tags and flow/ESM pieces.
fn is_block_tok(t: Option<&Tok<'_>>) -> bool {
    match t {
        None => true,
        Some(Tok::Html(h)) => is_block_tag(tag_name(h)),
        Some(Tok::Mdx(m, _)) => matches!(m, 'O' | 'C' | 'S' | 'E' | 'M'),
        Some(Tok::Text(_)) => false,
    }
}

struct Converter {
    out: String,
    esm: String,
    /// Open elements (HTML and JSX alike), innermost last, for nesting checks.
    stack: Vec<String>,
    /// Depth of open `<pre>` elements: text inside becomes JS string literals.
    pre_depth: usize,
    /// Components mode: markdown tags are written as `_components.<tag>`.
    components: bool,
    elements: Vec<String>,
    references: Vec<String>,
}

fn convert(html: &str, jsx: &JsxOptions) -> Result<MdxOutput, MdxError> {
    let components = jsx.components || jsx.provider_import_source.is_some();
    let toks = tokenize(html)?;
    let mut c = Converter {
        out: String::with_capacity(html.len() + html.len() / 4 + 8),
        esm: String::new(),
        stack: Vec::new(),
        pre_depth: 0,
        components,
        elements: Vec::new(),
        references: Vec::new(),
    };
    c.out.push_str("<>\n");
    for (idx, tok) in toks.iter().enumerate() {
        match *tok {
            Tok::Text(t) => {
                let prev = idx.checked_sub(1).and_then(|p| toks.get(p));
                c.text(t, prev, toks.get(idx + 1));
            }
            Tok::Html(h) => c.html_tag(h)?,
            Tok::Mdx(marker, lit) => c.mdx(marker, lit)?,
        }
    }
    if let Some(open) = c.stack.last() {
        return Err(MdxError::new(format!(
            "expected a closing tag for `<{open}>` before the end of the document"
        )));
    }
    while c.out.ends_with('\n') {
        c.out.pop();
    }
    c.out.push_str("\n</>");
    let declared = esm_identifiers(&c.esm);
    c.references.retain(|r| !declared.contains(&r.as_str()));
    Ok(MdxOutput {
        esm: c.esm,
        jsx: c.out,
        components,
        elements: c.elements,
        references: c.references,
        provider_import_source: jsx.provider_import_source.clone(),
    })
}

impl Converter {
    fn open(&mut self, name: &str) {
        self.stack.push(name.to_string());
    }

    fn close(&mut self, name: &str) -> Result<(), MdxError> {
        match self.stack.pop() {
            Some(open) if open == name => Ok(()),
            Some(open) => Err(MdxError::new(format!(
                "unexpected closing tag `</{name}>`, expected `</{open}>`"
            ))),
            None => Err(MdxError::new(format!(
                "unexpected closing tag `</{name}>` with no open element"
            ))),
        }
    }

    fn text(&mut self, t: &str, prev: Option<&Tok<'_>>, next: Option<&Tok<'_>>) {
        if self.pre_depth > 0 {
            if !t.is_empty() {
                self.out.push('{');
                push_js_string(&mut self.out, &decode_entities(t));
                self.out.push('}');
            }
            return;
        }
        let ws_only = t.bytes().all(|b| matches!(b, b' ' | b'\t' | b'\n'));
        if ws_only && t.contains('\n') && (is_block_tok(prev) || is_block_tok(next)) {
            // Layout whitespace between blocks; JSX drops it, as HTML ignores it.
            self.out.push_str(t);
            return;
        }
        let mut last = 0;
        for (i, b) in t.bytes().enumerate() {
            let rep = match b {
                b'{' => "&#123;",
                b'}' => "&#125;",
                b'\n' => "{\"\\n\"}",
                _ => continue,
            };
            self.out.push_str(&t[last..i]);
            self.out.push_str(rep);
            last = i + 1;
        }
        self.out.push_str(&t[last..]);
    }

    fn html_tag(&mut self, h: &str) -> Result<(), MdxError> {
        let name = tag_name(h);
        if h.starts_with("</") {
            self.close(name)?;
            if name == "pre" {
                self.pre_depth = self.pre_depth.saturating_sub(1);
            }
            if self.components {
                self.out.push_str("</_components.");
                self.out.push_str(name);
                self.out.push('>');
            } else {
                self.out.push_str(h);
            }
            return Ok(());
        }
        let self_closing = h.ends_with("/>");
        let is_void = matches!(name, "br" | "hr" | "img" | "input");
        let prefix = if self.components {
            if !self.elements.iter().any(|e| e == name) {
                self.elements.push(name.to_string());
            }
            "_components."
        } else {
            ""
        };
        push_jsx_attrs(&mut self.out, h, prefix, name, self_closing || is_void);
        if !(self_closing || is_void) {
            self.open(name);
            if name == "pre" {
                self.pre_depth += 1;
            }
        }
        Ok(())
    }

    fn mdx(&mut self, marker: char, lit: &str) -> Result<(), MdxError> {
        match marker {
            'M' => {
                if !self.esm.is_empty() {
                    self.esm.push('\n');
                }
                self.esm.push_str(lit);
                return Ok(());
            }
            'o' | 'O' => {
                let name = tag_name(lit);
                self.reference(name);
                self.open(name);
            }
            'c' | 'C' => self.close(tag_name(lit))?,
            's' | 'S' => self.reference(tag_name(lit)),
            'e' | 'E' => {}
            _ => {
                let head: String = lit.chars().take(40).collect();
                return Err(MdxError::new(format!(
                    "unterminated MDX construct starting at `{head}`"
                )));
            }
        }
        self.out.push_str(lit);
        Ok(())
    }

    /// Record the component a JSX tag refers to: a capitalised name or the
    /// root of a member name (`<Foo.Bar>` → `Foo`). Lowercase and custom
    /// element names (`<div>`, `<my-el>`) are literal tags, as in MDX.
    fn reference(&mut self, name: &str) {
        let root = name.split('.').next().unwrap_or("");
        let component = name.contains('.') || root.starts_with(|c: char| c.is_ascii_uppercase());
        if component
            && !root.is_empty()
            && !root.contains(['-', ':'])
            && !self.references.iter().any(|r| r == root)
        {
            self.references.push(root.to_string());
        }
    }
}

/// Identifiers in the ESM block, skipping strings and comments. Used as a
/// cheap over-approximation of the names it declares.
fn esm_identifiers(esm: &str) -> Vec<&str> {
    let b = esm.as_bytes();
    let mut ids = Vec::new();
    let mut i = 0;
    while i < b.len() {
        match b[i] {
            q @ (b'"' | b'\'' | b'`') => {
                i += 1;
                while i < b.len() && b[i] != q {
                    i += if b[i] == b'\\' { 2 } else { 1 };
                }
                i += 1;
            }
            b'/' if b.get(i + 1) == Some(&b'/') => {
                while i < b.len() && b[i] != b'\n' {
                    i += 1;
                }
            }
            b'/' if b.get(i + 1) == Some(&b'*') => {
                i = esm[i + 2..].find("*/").map_or(b.len(), |e| i + 2 + e + 2);
            }
            c if c == b'_' || c == b'$' || c.is_ascii_alphabetic() => {
                let start = i;
                while i < b.len() && (b[i] == b'_' || b[i] == b'$' || b[i].is_ascii_alphanumeric())
                {
                    i += 1;
                }
                ids.push(&esm[start..i]);
            }
            _ => i += 1,
        }
    }
    ids
}

/// Write an ironmark HTML start tag as JSX.
fn push_jsx_attrs(out: &mut String, h: &str, prefix: &str, name: &str, self_close: bool) {
    out.push('<');
    out.push_str(prefix);
    out.push_str(name);
    let inner = h
        .strip_prefix('<')
        .unwrap_or(h)
        .trim_end_matches('>')
        .trim_end_matches('/');
    let mut rest = inner[name.len()..].trim_start();
    while !rest.is_empty() {
        let name_end = rest
            .find(|c: char| c == '=' || c.is_ascii_whitespace())
            .unwrap_or(rest.len());
        let attr = &rest[..name_end];
        rest = &rest[name_end..];
        // Renderer output always double-quotes; raw HTML in plain Markdown may not.
        let mut value = None;
        if let Some(r) = rest.strip_prefix('=') {
            let r = r.trim_start();
            let (q, r) = match r.as_bytes().first() {
                Some(&q @ (b'"' | b'\'')) => (Some(q as char), &r[1..]),
                _ => (None, r),
            };
            let close = match q {
                Some(q) => r.find(q),
                None => r.find(|c: char| c.is_ascii_whitespace()),
            }
            .unwrap_or(r.len());
            value = Some(&r[..close]);
            rest = r.get(close + usize::from(q.is_some())..).unwrap_or("");
        }
        rest = rest.trim_start();
        let jsx_name = match attr {
            "class" => "className",
            "for" => "htmlFor",
            other => other,
        };
        out.push(' ');
        out.push_str(jsx_name);
        match (attr, value) {
            ("checked" | "disabled", Some("")) | (_, None) => {}
            ("style", Some(v)) => push_style_object(out, v),
            (_, Some(v)) => {
                out.push_str("=\"");
                if v.contains('"') {
                    out.push_str(&v.replace('"', "&quot;"));
                } else {
                    out.push_str(v);
                }
                out.push('"');
            }
        }
    }
    out.push_str(if self_close { " />" } else { ">" });
}

/// `a-b: c; d: e` → `={{aB: "c", d: "e"}}`.
fn push_style_object(out: &mut String, css: &str) {
    out.push_str("={{");
    let mut first = true;
    for decl in css.split(';') {
        let Some((prop, val)) = decl.split_once(':') else {
            continue;
        };
        if !first {
            out.push_str(", ");
        }
        first = false;
        let mut upper = false;
        for ch in prop.trim().chars() {
            if ch == '-' {
                upper = true;
            } else if upper {
                out.push(ch.to_ascii_uppercase());
                upper = false;
            } else {
                out.push(ch);
            }
        }
        out.push_str(": ");
        push_js_string(out, &decode_entities(val.trim()));
    }
    out.push_str("}}");
}

/// Append `s` as a double-quoted JavaScript string literal.
fn push_js_string(out: &mut String, s: &str) {
    use std::fmt::Write;
    out.push('"');
    for ch in s.chars() {
        match ch {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            '\u{2028}' => out.push_str("\\u2028"),
            '\u{2029}' => out.push_str("\\u2029"),
            c if (c as u32) < 0x20 => {
                let _ = write!(out, "\\u{:04x}", c as u32);
            }
            c => out.push(c),
        }
    }
    out.push('"');
}

#[cfg(test)]
mod tests {
    use super::*;

    fn jsx(md: &str) -> String {
        render_jsx(md, &ParseOptions::default()).unwrap().jsx
    }

    #[test]
    fn text_and_braces() {
        assert_eq!(jsx("a \\{ b"), "<>\n<p>a &#123; b</p>\n</>");
        assert_eq!(jsx("*a*\nb"), "<>\n<p><em>a</em><br />{\"\\n\"}b</p>\n</>");
    }

    #[test]
    fn code_block_is_string() {
        assert_eq!(
            jsx("```js\nif (a < b) {\n}\n```"),
            "<>\n<pre><code className=\"language-js\">{\"if (a < b) {\\n}\\n\"}</code></pre>\n</>"
        );
    }

    #[test]
    fn attrs() {
        let out = jsx("| a |\n|:-:|\n| b |\n\n- [x] done");
        assert!(
            out.contains("<th style={{textAlign: \"center\"}}>"),
            "{out}"
        );
        assert!(
            out.contains("<input type=\"checkbox\" checked disabled />"),
            "{out}"
        );
    }

    #[test]
    fn nesting_errors() {
        let opts = ParseOptions::default();
        assert!(render_jsx("<A>\n\n<B>\n\n</A>", &opts).is_err());
        assert!(render_jsx("<A>", &opts).is_err());
        assert!(render_jsx("a <b>c\n\nd</b>", &opts).is_err());
        assert!(render_jsx("<A>\n\n# hi\n\n</A>", &opts).is_ok());
    }

    #[test]
    fn components_mode() {
        let opts = ParseOptions::default();
        let jsx = JsxOptions {
            components: true,
            ..Default::default()
        };
        let out = render_jsx_with(
            "import A from './a'\n\n# Hi *x*\n\ntext <A /> <B.C /> <div /> <my-el />\n\n---",
            &opts,
            &jsx,
        )
        .unwrap();
        assert_eq!(
            out.jsx,
            "<>\n<_components.h1>Hi <_components.em>x</_components.em></_components.h1>\n\
             <_components.p>text <A /> <B.C /> <div /> <my-el /></_components.p>\n<_components.hr />\n</>"
        );
        assert_eq!(out.elements, ["h1", "em", "p", "hr"]);
        assert_eq!(out.references, ["B"]);
        let module = out.to_module();
        assert!(module.contains("const _components = {h1: \"h1\", em: \"em\", p: \"p\", hr: \"hr\", ...props.components};"), "{module}");
        assert!(
            module.contains("if (!B) _missingMdxReference(\"B\");"),
            "{module}"
        );
        assert!(!module.contains("_provideComponents"), "{module}");
    }

    #[test]
    fn esm_names() {
        assert_eq!(
            esm_identifiers("import X, {Y as Z} from \"./Q\" // W\nexport const V = `U`"),
            [
                "import", "X", "Y", "as", "Z", "from", "export", "const", "V"
            ]
        );
    }

    #[test]
    fn plain_markdown() {
        let md = |src: &str| {
            render_markdown_jsx(src, &ParseOptions::default(), &JsxOptions::default()).unwrap()
        };
        let out = md("import X from 'y'\n\nHi {x} <X />");
        assert!(out.esm.is_empty());
        assert!(out.references.is_empty());
        assert_eq!(
            out.jsx,
            "<>\n<p>import X from 'y'</p>\n<p>Hi &#123;x&#125; <X /></p>\n</>"
        );
        assert_eq!(
            md("<!-- note -->\n\n<div class='a' id=b title=\"c\">\n\nx<br>\n\n</div>").jsx,
            "<>\n\n<div className=\"a\" id=\"b\" title=\"c\">\n<p>x<br /></p>\n</div>\n</>"
        );
        assert!(
            render_markdown_jsx("<p>open", &ParseOptions::default(), &JsxOptions::default())
                .is_err()
        );
    }

    #[test]
    fn decode() {
        assert_eq!(
            decode_entities("&lt;a&gt; &amp;&#123;&#x41;&bogus"),
            "<a> &{A&bogus"
        );
    }
}
