# ironmark

[![CI](https://github.com/ph1p/ironmark/actions/workflows/ci.yml/badge.svg)](https://github.com/ph1p/ironmark/actions/workflows/ci.yml) [![npm](https://img.shields.io/npm/v/ironmark)](https://www.npmjs.com/package/ironmark) [![crates.io](https://img.shields.io/crates/v/ironmark)](https://crates.io/crates/ironmark)

Fast Markdown parser written in Rust with **zero third-party** parsing dependencies. Outputs HTML, AST, ANSI terminal, or Markdown. Fully compliant with [CommonMark 0.31.2](https://spec.commonmark.org/0.31.2/) (652/652 spec tests pass). Available as a Rust crate and as an npm package via WebAssembly.

## Table of Contents

- [Quick start](#quick-start)
  - [Documents: wrap once, ask for anything](#documents-wrap-once-ask-for-anything)
- [Which function should I use?](#which-function-should-i-use)
- [JavaScript / TypeScript](#javascript--typescript)
  - [Node.js](#nodejs)
  - [Markdown → HTML](#markdown--html)
  - [Markdown → AST](#markdown--ast)
  - [Markdown → AST → Markdown](#markdown--ast--markdown)
  - [Safe HTML rendering](#safe-html-rendering)
  - [ANSI Terminal Output](#ansi-terminal-output)
  - [HTML to Markdown](#html-to-markdown)
  - [Browser / Bundler](#browser--bundler)
  - [Smaller bundles: subpath imports](#smaller-bundles-subpath-imports)
  - [Using ironmark in AI pipelines](#using-ironmark-in-ai-pipelines)
  - [Presets](#presets)
  - [Introspection constants](#introspection-constants)
  - [Utility functions](#utility-functions)
  - [Migrating from 2.x](#migrating-from-2x)
- [Configuration](#configuration)
  - [Extensions](#extensions)
  - [Security](#security)
  - [Other Options](#other-options)
- [MDX](#mdx)
- [CLI](#cli)
- [Rust](#rust)
- [C / C++](#c--c++)
- [Benchmarks](#benchmarks)
- [Development](#development)

## Quick start

```bash
npm install ironmark
# or
pnpm add ironmark
```

```ts
import ironmark from "ironmark";

const doc = ironmark.parse("# Hello\n\n**World**");

doc.html(); // "<h1>Hello</h1>\n<p><strong>World</strong></p>\n"
doc.ast(); // { Document: { children: [...] } }
doc.headings(); // [{ level: 1, text: "Hello", id: "hello" }]
```

Prefer plain functions? Every output also has a named export:

```ts
import { markdownToHtml, markdownToAst } from "ironmark";

const html = markdownToHtml("# Hello\n\n**World**");
const ast = markdownToAst("# Hello\n\n**World**");
```

### Documents: wrap once, ask for anything

`ironmark.parse(input, options?)` wraps the input in a document. Nothing renders until you ask for an output. Each output is its own WASM call, made once and cached, so calling `doc.html()` twice costs one render.

| Method                      | Returns                                                                                    |
| --------------------------- | ------------------------------------------------------------------------------------------ |
| `doc.html()`                | HTML string                                                                                |
| `doc.text()`                | Plain text for search indexes, excerpts or embeddings                                      |
| `doc.headings()`            | `[{ level, text, id }]`; `id` matches `enableHeadingIds`                                   |
| `doc.toc()`                 | Headings nested by level: `[{ level, text, id, children }]`                                |
| `doc.ast()`                 | The AST (deeply frozen; `structuredClone(doc.ast())` to edit)                              |
| `doc.markdown()`            | Normalized Markdown                                                                        |
| `doc.links()`               | `[{ url, title, text }]` for every link, in document order (dangerous URLs come back `""`) |
| `doc.images()`              | `[{ url, title, alt }]` for every image                                                    |
| `doc.frontmatter()`         | The raw front matter string, or `null` (needs `enableFrontmatter`)                         |
| `doc.stats()`               | `{ words, characters, readingTime }`; `readingTime` is minutes at 200 words per minute     |
| `doc.summary()`             | `{ blockCount, nodeCounts }`                                                               |
| `doc.ansi(render?)`         | Terminal output; `render` = `{ width, color, lineNumbers, padding }`                       |
| `doc.jsx(render?)`          | MDX compiled to `{ esm, jsx, module }`; `render` = `{ components, providerImportSource }`  |
| `doc.with(options)`         | A new document over the same source with `options` layered on top                          |
| `doc.source`, `doc.options` | The original input and the resolved, frozen options                                        |

Parse options (`preset`, `safe`, `enableTables`, …) are set only through `parse()` and `with()`, so every output of one document agrees on them. `ansi()` and `jsx()` take render options only; each distinct set is cached separately.

`text()`, `headings()`, `toc()`, `links()`, `images()`, `frontmatter()` and `stats()` exist in every entry. `ast()`, `summary()` and `markdown()` need `ironmark/ast` or the main entry, `jsx()` needs `ironmark/mdx` or the main entry, and `ansi()` the main entry. Methods a binary cannot run are absent from its documents, and the types say so.

How `with()` layers options:

- A layer that sets `preset` starts again from that preset; earlier options are dropped.
- Otherwise the layer's keys override the base and the rest is kept.
- `safe: false` turns `disableRawHtml` and `tagFilter` off; `deterministic: false` turns `collapseWhitespace` off.

```ts
import ironmark from "ironmark";

// Configure once, reuse everywhere.
const md = ironmark.with({ preset: "safe", enableHeadingIds: true });

const post = md.parse(userInput);
const article = `<article>${post.html()}</article>`;
const sidebar = post.toc(); // nested headings, ids match the rendered ones
const excerpt = post.text().slice(0, 160);

process.stdout.write(post.ansi({ width: 100 }));
post.with({ enableTables: false }).html(); // same source, other options

// HTML in, clean Markdown out.
ironmark.fromHtml("<h1>Hi</h1><p><b>bold</b></p>").markdown(); // "# Hi\n\n**bold**\n"
```

For a single output, skip the document: every document method is also a shortcut on the instance, taking the input first. `ironmark.html(md)` is `ironmark.parse(md).html()`, and `md.text(input)` on a configured instance uses its options. An entry has the shortcuts its documents have.

Front matter is off by default. With `enableFrontmatter: true`, a leading `---` YAML (closed by `---` or `...`) or `+++` TOML block is removed before parsing, and `doc.frontmatter()` returns its raw text for your own YAML/TOML parser:

```ts
const post = ironmark.parse(file, { enableFrontmatter: true });
const meta = yaml.parse(post.frontmatter() ?? "");
```

In the browser, `await ironmark.ready` (or `await ironmark.init()`) once first; both return the same cached promise. `init()` returns a Promise in every entry; under Node it resolves immediately.

## Which function should I use?

| Goal                            | Function                                                                         |
| ------------------------------- | -------------------------------------------------------------------------------- |
| Several outputs from one input  | `ironmark.parse(input)` — see [Documents](#documents-wrap-once-ask-for-anything) |
| Render Markdown to HTML         | `markdownToHtml(input, options?)`                                                |
| Plain text (search, excerpts)   | `markdownToText(input, options?)`                                                |
| Parse Markdown to AST object    | `markdownToAst(input, options?)`                                                 |
| Normalize / round-trip Markdown | `markdownToAst()` + `astToMarkdown()`                                            |
| Terminal / CLI output           | `markdownToAnsi(input, options?)`                                                |
| Convert HTML back to Markdown   | `htmlToMarkdown(html, options?)` / `htmlToAst(html, options?)`                   |
| Compile MDX to JSX              | `mdxToJsx(input, options?)` — see [MDX](#mdx)                                    |

## JavaScript / TypeScript

### Node.js

Node loads the packaged `wasm/pkg/ironmark_bg.wasm` synchronously relative to the
module — no `init()` needed:

```ts
import { markdownToHtml, markdownToAst } from "ironmark";

const html = markdownToHtml("# Hello\n\nThis is **fast**.");
const ast = markdownToAst("# Hello");
```

Keep the WASM asset and glue alongside the Node entry when copying or bundling
the package; it is not a self-contained single-file JavaScript bundle.
Node bundlers may need asset-copy configuration to preserve this layout.
The browser entry still uses asynchronous `init()`.

### Markdown → HTML

```ts
import { markdownToHtml } from "ironmark";

const html = markdownToHtml("# Hello\n\nThis is **fast**.");
```

### Markdown → plain text

`markdownToText` runs the real inline parser, so emphasis, links, code spans, entities and escapes come out as a reader of the rendered HTML sees them. Blocks are separated by a blank line, tight list items by a newline and table cells by tabs; raw HTML blocks, MDX blocks and thematic breaks are dropped.

```ts
import { markdownToText } from "ironmark";

markdownToText("# Hi *there*\n\na &lt; b &copy;"); // "Hi there\n\na < b ©"
```

### Markdown → AST

`markdownToAst()` returns a parsed JavaScript object — no `JSON.parse()` needed.
Each node is an object with one key naming its type; `ThematicBreak` is the bare
string `"ThematicBreak"`. Leaf text (`raw`, table cells) is unparsed inline Markdown.

```ts
import { markdownToAst } from "ironmark";

const ast = markdownToAst("# Hello\n\n- [x] done");
// {
//   Document: {
//     children: [
//       { Heading: { level: 1, raw: "Hello" } },
//       { List: { kind: { Bullet: 45 }, start: 0, tight: true, children: [
//         { ListItem: { checked: true, children: [{ Paragraph: { raw: "done" } }] } },
//       ] } },
//     ],
//   },
// }
for (const node of ast.Document.children) {
  if (typeof node !== "string" && "Heading" in node) console.log(node.Heading.raw);
}
```

### Markdown → AST → Markdown

```ts
import { markdownToAst, astToMarkdown } from "ironmark";

const ast = markdownToAst("**Hello**");
const md = astToMarkdown(ast);
// Returns: "**Hello**"
```

### Safe HTML rendering

Use `safe: true` for **any untrusted input** — it disables raw HTML passthrough and enables the GFM tag filter.

```ts
import { markdownToHtml } from "ironmark";

// Simple safe flag
const html = markdownToHtml(userInput, { safe: true });

// Or use the built-in preset
const html = markdownToHtml(userInput, { preset: "safe" });
```

What `safe: true` does:

- Sets `disableRawHtml: true` — all inline and block HTML is escaped
- Sets `tagFilter: true` — dangerous tags (`<script>`, `<iframe>`, etc.) are escaped

Dangerous URI schemes (`javascript:`, `vbscript:`, `data:` except `data:image/…`) are **always** stripped from link destinations regardless of options.

### ANSI Terminal Output

Use `markdownToAnsi()` to render Markdown as coloured terminal output (ANSI 256-colour escape codes).

````ts
import { markdownToAnsi } from "ironmark";

// Defaults: width 80, color enabled
const ansi = markdownToAnsi("# Hello\n\n**bold** and `code`");
process.stdout.write(ansi);

// Terminal options sit next to the parse options in one object
const ansi = markdownToAnsi("# Hello\n\n```rust\nfn main() {}\n```", {
  width: 120,
  lineNumbers: true,
  enableTables: false,
});
process.stdout.write(ansi);

// Plain text — no ANSI codes (useful for piping to files)
const plain = markdownToAnsi("# Hello", { color: false });
````

#### ANSI options

`AnsiOptions` extends `ParseOptions` with:

| Option        | Type      | Default | Description                                                                                   |
| ------------- | --------- | ------- | --------------------------------------------------------------------------------------------- |
| `width`       | `number`  | `80`    | Column width for word-wrap, heading underlines, rule length. `0` = use default.               |
| `color`       | `boolean` | `true`  | Emit ANSI colour codes. `false` = plain text output.                                          |
| `lineNumbers` | `boolean` | `false` | Show line numbers in fenced code blocks.                                                      |
| `padding`     | `number`  | `0`     | Horizontal padding added to both sides of each line, plus ⌈padding/2⌉ blank lines at the top. |

### HTML to Markdown

Convert HTML back to Markdown syntax using `htmlToMarkdown()`.

```ts
import { htmlToMarkdown } from "ironmark";

const md = htmlToMarkdown("<h1>Hello</h1><p><strong>Bold</strong> text</p>");
// Returns: "# Hello\n\n**Bold** text"

// Preserve unknown HTML tags (e.g. <sup>, <sub>) as raw HTML in output
const md = htmlToMarkdown("<p>H<sub>2</sub>O</p>", { preserveUnknownAsHtml: true });
// Returns: "H<sub>2</sub>O"
```

For AST access, use `htmlToAst()`:

```ts
import { htmlToAst, astToMarkdown } from "ironmark";

const ast = htmlToAst("<h1>Hello</h1><p>World</p>");
const md = astToMarkdown(ast);
```

### Browser / Bundler

Call `init()` once before using any function. It's idempotent and optionally accepts a custom `.wasm` URL.

```ts
import { init, markdownToHtml, markdownToAst } from "ironmark";

await init();

const html = markdownToHtml("# Hello\n\nThis is **fast**.");
const ast = markdownToAst("# Hello");
```

#### Vite

```ts
import { init, markdownToHtml } from "ironmark";
import wasmUrl from "ironmark/ironmark.wasm?url";

await init(wasmUrl);

const html = markdownToHtml("# Hello\n\nThis is **fast**.");
```

### Smaller bundles: subpath imports

WASM can't be tree-shaken, so `import { markdownToHtml } from "ironmark"` still downloads the whole binary (AST serialization, ANSI, MDX and the HTML parser included). Each subpath ships its own binary, compiled with only the code its functions need:

| Import          | Exports                                                                                                                       | WASM (brotli) |
| --------------- | ----------------------------------------------------------------------------------------------------------------------------- | ------------- |
| `ironmark`      | everything                                                                                                                    | ~155 KB       |
| `ironmark/ast`  | `markdownToAst`, `astToMarkdown`, `markdownToHtml`, `markdownToText`, `extractHeadings`, `summarizeAst`, `AST_SCHEMA_VERSION` | ~123 KB       |
| `ironmark/mdx`  | `mdxToJsx`, `markdownToHtml`, `markdownToText`                                                                                | ~97 KB        |
| `ironmark/html` | `markdownToHtml`, `markdownToText`                                                                                            | ~89 KB        |

```ts
import { init, mdxToJsx } from "ironmark/mdx";

await init(); // browser only; resolves immediately under Node, where the subpaths load synchronously

const { module } = mdxToJsx("# Hello {props.name}");
```

Every subpath has the same default export, with only the document methods its binary supports: `ironmark/html` documents have `html()`, `ironmark/mdx` adds `jsx()`, and `ironmark/ast` adds the AST methods (`ast()`, `markdown()`, `text()`, `headings()`, `toc()`, `summary()`). `fromHtml()` and `ansi()` need the full entry.

```ts
import ironmark from "ironmark/html";

ironmark.parse("# Hello").html();
```

Every subpath also exports `DEFAULT_OPTIONS`, `PRESETS` and `CAPABILITIES`; its `CAPABILITIES.formats` lists only what that binary can do (`["html", "text"]` for `ironmark/html`). Options and output are identical to the main entry; only the set of functions differs. Types come from the same declarations, so `ParseOptions`, `JsxOutput` and friends are shared.

Pick **one** entry per app. Each subpath has its own WASM instance, so importing `ironmark/mdx` and `ironmark/ast` together loads the shared parser core twice. That is larger than `ironmark` alone. If you need functions from two subpaths, use `ironmark`.

### Using ironmark in AI pipelines

ironmark is designed to be a reliable, deterministic parsing layer for AI agents and code generation tools.

**Recommended pattern for agents:**

```ts
import { markdownToAst, markdownToHtml, astToMarkdown, CAPABILITIES } from "ironmark";

// 1. Inspect capabilities at startup
CAPABILITIES.formats; // ["html", "text", "ast", "markdown", "ansi", "jsx"]

// 2. Parse with deterministic options
const ast = markdownToAst(content, { preset: "llm" });

// 3. Render or transform
const html = markdownToHtml(content, { preset: "llm" });
const normalized = astToMarkdown(ast);
```

**The `llm` preset** produces deterministic, structure-first output:

- Disables autolinks, wiki links, math, hard breaks (ambiguous in AI-generated text)
- Enables heading IDs and whitespace normalization
- Disables raw HTML passthrough (safe by default)

```ts
import { markdownToAst, extractHeadings, summarizeAst } from "ironmark";

const ast = markdownToAst(content, { preset: "llm" });

// Extract document structure
const headings = extractHeadings(ast);
// [{ level: 1, text: "Introduction", id: "introduction" }, ...]

// Summarize node types
const summary = summarizeAst(ast);
// { blockCount: 5, nodeCounts: { Heading: 2, Paragraph: 3, ... } }
```

### Presets

Named presets configure multiple options at once. Explicit options always override the preset.

| Preset      | Description                                                                                        |
| ----------- | -------------------------------------------------------------------------------------------------- |
| `"default"` | All extensions enabled. Current default behavior.                                                  |
| `"safe"`    | Disables raw HTML, enables GFM tag filter. Use for untrusted input.                                |
| `"strict"`  | CommonMark-only: disables extensions and permissive behaviors.                                     |
| `"llm"`     | Deterministic, stable output for AI pipelines. Disables ambiguous extensions, enables heading IDs. |

```ts
import { markdownToHtml, markdownToAst } from "ironmark";

// Use a preset
const html = markdownToHtml(content, { preset: "safe" });

// Presets compose with explicit options (explicit wins)
const html = markdownToHtml(content, { preset: "llm", enableTables: false });
```

### Introspection constants

Stable, machine-readable metadata for agents and tooling. All four are frozen.

```ts
import { CAPABILITIES, AST_SCHEMA_VERSION, DEFAULT_OPTIONS, PRESETS } from "ironmark";

CAPABILITIES;
// {
//   astSchemaVersion: "2",
//   formats: ["html", "text", "ast", "markdown", "ansi", "jsx"], // only what this entry supports
//   presets: ["default", "safe", "strict", "llm"],
//   extensions: [...],
//   security: [...]
// }

AST_SCHEMA_VERSION; // "2" — bumps on any breaking change to the AST shape
DEFAULT_OPTIONS.enableTables; // true — the effective value of every option
PRESETS.llm; // the options the "llm" preset applies

// Copy before changing: the constants are frozen
const mine = { ...DEFAULT_OPTIONS, hardBreaks: false };
```

### Utility functions

```ts
import { markdownToAst, extractHeadings, summarizeAst } from "ironmark";

const ast = markdownToAst("# Hello\n\n## World\n\nA paragraph.");

// Every heading, nested ones included. `text` has inline Markdown stripped;
// `id` is exactly what `enableHeadingIds` renders.
const headings = extractHeadings(ast);
// [
//   { level: 1, text: "Hello", id: "hello" },
//   { level: 2, text: "World", id: "world" },
// ]

// Count node types
const summary = summarizeAst(ast);
// { blockCount: 3, nodeCounts: { Heading: 2, Paragraph: 1 } }
```

### Migrating from 2.x

The JavaScript API was renamed so every function reads `sourceToTarget`, and the
getters became frozen constants. The Rust API is unchanged.

| 2.x                                      | Now                                                     |
| ---------------------------------------- | ------------------------------------------------------- |
| `renderHtml(md, opts)`                   | `markdownToHtml(md, opts)`                              |
| `parseMarkdown(md, opts)`                | `markdownToAst(md, opts)`                               |
| `renderMarkdown(ast)`                    | `astToMarkdown(ast)`                                    |
| `renderAnsiTerminal(md, opts, ansiOpts)` | `markdownToAnsi(md, { ...opts, ...ansiOpts })`          |
| `renderJsx(md, opts)`                    | `mdxToJsx(md, opts)`                                    |
| `parseHtmlToAst(html, true)`             | `htmlToAst(html, { preserveUnknownAsHtml: true })`      |
| `htmlToMarkdown(html, true)`             | `htmlToMarkdown(html, { preserveUnknownAsHtml: true })` |
| `getCapabilities()`                      | `CAPABILITIES`                                          |
| `getAstSchemaVersion()`                  | `AST_SCHEMA_VERSION`                                    |
| `getDefaultOptions()`                    | `DEFAULT_OPTIONS` (frozen; spread to copy)              |
| `getPresets()`                           | `PRESETS` (frozen; spread to copy)                      |
| type `RenderHtmlOptions`                 | `ParseOptions`                                          |
| type `AstNode` (`{ t, c, text }`)        | `AstNode`: the real tagged union (`{ Heading: { … } }`) |

`init()` now returns a Promise in every entry, including Node, so the same
`await init()` works everywhere.

`extractHeadings()` and `summarizeAst()` now read the real AST shape. Before, they
expected the documented-but-wrong `{ t, c }` nodes and found nothing.

## Configuration

### Extensions (default `true`)

| Option              | JS (`camelCase`)           | Rust (`snake_case`)           | Description                                            |
| ------------------- | -------------------------- | ----------------------------- | ------------------------------------------------------ |
| Hard breaks         | `hardBreaks`               | `hard_breaks`                 | Every newline becomes `<br />`                         |
| Highlight           | `enableHighlight`          | `enable_highlight`            | `==text==` → `<mark>`                                  |
| Strikethrough       | `enableStrikethrough`      | `enable_strikethrough`        | `~~text~~` → `<del>`                                   |
| Underline           | `enableUnderline`          | `enable_underline`            | `++text++` → `<u>`                                     |
| Tables              | `enableTables`             | `enable_tables`               | Pipe table syntax                                      |
| Autolink            | `enableAutolink`           | `enable_autolink`             | Bare URLs & emails → `<a>`                             |
| Task lists          | `enableTaskLists`          | `enable_task_lists`           | `- [ ]` / `- [x]` checkboxes                           |
| Indented code       | `enableIndentedCodeBlocks` | `enable_indented_code_blocks` | 4-space indent → `<pre><code>`                         |
| Wiki links          | `enableWikiLinks`          | `enable_wiki_links`           | `[[page]]` → `<a href="page">`                         |
| LaTeX math          | `enableLatexMath`          | `enable_latex_math`           | `$inline$` and `$$display$$` → `<span class="math-…">` |
| Front matter        | `enableFrontmatter`        | `enable_frontmatter`          | Skip a leading `---`/`+++` block (off by default)      |
| Heading IDs         | `enableHeadingIds`         | `enable_heading_ids`          | Auto `id=` on headings from slugified text             |
| Heading anchors     | `enableHeadingAnchors`     | `enable_heading_anchors`      | `<a class="anchor">` inside each heading (implies IDs) |
| Permissive headings | `permissiveAtxHeaders`     | `permissive_atx_headers`      | Allow `#Heading` without space after `#`               |

Bare-URL autolinks follow GFM trailing-punctuation rules, with one deliberate difference: a trailing `>` is not part of the link (`https://x.com>` links `https://x.com`), so URLs written inside `>`-terminated text do not swallow the bracket.

### MDX (opt-in)

`enable_mdx` / `enableMdx` (default `false`) parses JSX tags, `{expressions}` and `import`/`export`. See [MDX](#mdx).

### Security

| Option           | JS (`camelCase`) | Rust (`snake_case`) | Default        | Description                                                        |
| ---------------- | ---------------- | ------------------- | -------------- | ------------------------------------------------------------------ |
| Safe mode        | `safe`           | —                   | `false`        | Shorthand: sets `disableRawHtml: true` and `tagFilter: true`       |
| Disable raw HTML | `disableRawHtml` | `disable_raw_html`  | `false`        | Escape **all** HTML blocks and inline HTML                         |
| No HTML blocks   | `noHtmlBlocks`   | `no_html_blocks`    | `false`        | Escape block-level HTML only (more granular than `disableRawHtml`) |
| No HTML spans    | `noHtmlSpans`    | `no_html_spans`     | `false`        | Escape inline HTML only                                            |
| Tag filter       | `tagFilter`      | `tag_filter`        | `false`        | GFM tag filter: escape `<script>`, `<iframe>`, etc.                |
| Max nesting      | —                | `max_nesting_depth` | `128`          | Limit blockquote/list nesting depth (DoS prevention)               |
| Max input size   | —                | `max_input_size`    | `0` (no limit) | Truncate input beyond this byte count                              |

> In the WASM build, `max_nesting_depth` is fixed at `128` and `max_input_size` at `10 MB`.

Dangerous URI schemes (`javascript:`, `vbscript:`, `data:` except `data:image/…`) are **always** stripped from link and image destinations, regardless of options.

### Other Options

| Option              | JS (`camelCase`)     | Rust (`snake_case`)   | Default | Description                                       |
| ------------------- | -------------------- | --------------------- | ------- | ------------------------------------------------- |
| Collapse whitespace | `collapseWhitespace` | `collapse_whitespace` | `false` | Collapse runs of spaces/tabs in text to one space |
| Preset              | `preset`             | —                     | —       | Apply a named option preset (JS only)             |
| Deterministic       | `deterministic`      | —                     | `false` | Normalize whitespace for stable output (JS only)  |

## MDX

ironmark compiles [MDX](https://mdxjs.com/) — Markdown with JSX tags, `{expressions}` and `import`/`export` — to JSX. It is off by default (`enable_mdx` / `enableMdx`) and needs no extra dependencies. The output is a JavaScript module that still contains JSX, so run it through your bundler's JSX transform (esbuild, Babel, SWC, Oxc, TypeScript).

> **MDX is code.** Expressions and components run with the page's privileges. Only compile MDX from trusted authors: `disable_raw_html` does not make compiled MDX safe.

### Quick start

```ts
import { mdxToJsx } from "ironmark";

const { esm, jsx, module } = mdxToJsx(`import { Chart } from './chart.js'
export const year = 2026

# Report {year}

<Chart data={[1, 2]} />`);
```

```rust
use ironmark::{render_jsx, ParseOptions};

let out = render_jsx("import X from './x'\n\n# Hi {props.name}\n\n<X />", &ParseOptions::default()).unwrap();
// out.esm == "import X from './x'"
// out.jsx == "<>\n<h1>Hi {props.name}</h1>\n<X />\n</>"
let module = out.to_module(); // ESM + `export default function MDXContent(props) { … }`
```

`render_jsx` / `mdxToJsx` turn on `enable_mdx` themselves. Every other option (tables, task lists, highlight, …) applies as usual.

### Output

| Field (JS) | Rust                     | Contents                                                                         |
| ---------- | ------------------------ | -------------------------------------------------------------------------------- |
| `esm`      | `esm`                    | Top-level `import`/`export` statements, in document order                        |
| `jsx`      | `jsx`                    | The document body as one JSX fragment (`<>…</>`)                                 |
| `module`   | `to_module()`            | `esm` plus `export default function MDXContent(props)` — import this             |
| —          | `elements`, `references` | Markdown tags routed through `_components`, and components used but not declared |

Markdown becomes plain JSX elements (`<h1>`, `<p>`, `<pre><code className="language-js">`). Code block content becomes a JS string, so braces and whitespace survive. Text braces are escaped to `&#123;` / `&#125;`.

### Syntax

| Write                                        | Meaning                                                                |
| -------------------------------------------- | ---------------------------------------------------------------------- |
| `import X from './x'` / `export const a = 1` | ESM. Only at the top level, unindented; the block ends at a blank line |
| `<Note kind="tip" />` on its own line        | Block-level JSX element                                                |
| `Text <Badge>new</Badge> text`               | Inline JSX inside a paragraph                                          |
| `{year}`, `{props.items.map(…)}`             | Expression, inline or as its own block                                 |
| `<A x={1} {...rest} b={<C />} />`            | Expression, spread and element attribute values                        |
| `<a.b>`, `<svg:rect>`, `<>…</>`              | Member names, namespaces, fragments                                    |

Markdown works inside components when it is separated from the tags by blank lines:

```mdx
<Note kind="tip">

Some **markdown** here.

</Note>
```

MDX changes a few Markdown rules, as in `@mdx-js/mdx`:

- **HTML is JSX.** Write `className` and `style={{ color: "red" }}`, not `class` and `style="…"`. Tags must be closed (`<br />`).
- **No HTML comments.** `<!-- … -->` stays literal text. Use `{/* … */}`.
- **No indented code blocks.** Indentation is ignored; use fenced code.
- **`<https://…>` autolinks still work.** A `<` that does not start a valid JSX tag falls back to a CommonMark autolink (`@mdx-js/mdx` rejects these).
- **Literal braces and `<`** need escaping: `\{`, `\}`. A `<` followed by a space or digit (`a < b`, `1<2`) stays text.

Mismatched or unclosed tags are errors: `MdxError` in Rust, a thrown `Error` in JS. For example: ``unexpected closing tag `</B>`, expected `</A>` `` or ``expected a closing tag for `<Note>` before the end of the document``.

Known limits: expressions are matched by brace depth (strings, template literals, comments and nested JSX are understood, but a regular-expression literal containing an unbalanced brace is not), and nesting deeper than 64 elements/expressions is rejected.

### Components

By default the output is plain JSX: every component must be imported by the document, and `<h1>` stays `<h1>`. The `components` option (Rust: `JsxOptions`) switches to the `@mdx-js/mdx` component model:

- Markdown tags become `<_components.h1>`, so `props.components` can replace them (`{ h1: MyHeading, a: Link }`).
- Capitalised components the document does not import are read from `props.components`. A missing one throws ``Expected component `Note` to be defined``.
- `components.wrapper` wraps the whole document (a layout).
- `providerImportSource: "@mdx-js/react"` (implies `components`) also reads components from an `MDXProvider`, so the module works anywhere `@mdx-js/react` does, including Storybook docs.

```ts
mdxToJsx("# Hi\n\n<Note />", { components: true }).module;
```

```js
function _createMdxContent(props) {
  const _components = { h1: "h1", ...props.components };
  const { Note } = _components;
  if (!Note) _missingMdxReference("Note");
  return (
    <>
      <_components.h1>Hi</_components.h1>
      <Note />
    </>
  );
}

export default function MDXContent(props = {}) {
  const { wrapper: MDXLayout } = { ...props.components };
  return MDXLayout ? (
    <MDXLayout {...props}>
      <_createMdxContent {...props} />
    </MDXLayout>
  ) : (
    _createMdxContent(props)
  );
}
// … plus the _missingMdxReference helper
```

```rust
use ironmark::{render_jsx_with, JsxOptions, ParseOptions};

let jsx = JsxOptions { provider_import_source: Some("@mdx-js/react".into()), ..Default::default() };
let out = render_jsx_with("# Hi\n\n<Note />", &ParseOptions::default(), &jsx).unwrap();
```

Using the compiled module with React:

```jsx
import Guide from "./Guide.mdx";

<Guide components={{ Note, h1: (p) => <h1 className="title" {...p} />, wrapper: Layout }} />;
```

### Bundlers

**Vite** — a ready-made plugin is in [`examples/mdx/vite-plugin-ironmark-mdx.mjs`](examples/mdx/vite-plugin-ironmark-mdx.mjs). Its core:

```js
import { mdxToJsx } from "ironmark";
import { transformWithOxc } from "vite";

export default function ironmarkMdx(options = { components: true }) {
  return {
    name: "ironmark-mdx",
    enforce: "pre",
    transform(code, id) {
      if (!id.split("?")[0].endsWith(".mdx")) return null;
      const { module } = mdxToJsx(code, options);
      return transformWithOxc(module, `${id}.jsx`, { lang: "jsx", jsx: { runtime: "automatic" } });
    },
  };
}
```

**esbuild** — load `.mdx` files as JSX:

```js
import { readFile } from "node:fs/promises";
import { mdxToJsx } from "ironmark";

const ironmarkMdx = {
  name: "ironmark-mdx",
  setup(build) {
    build.onLoad({ filter: /\.mdx$/ }, async ({ path }) => ({
      contents: mdxToJsx(await readFile(path, "utf8"), { components: true }).module,
      loader: "jsx",
    }));
  },
};
```

Set `jsx: "automatic"` in the esbuild options. Other bundlers follow the same pattern: compile with `mdxToJsx`, then hand the module to the JSX transform.

### Other renderers

With `enable_mdx` set, the other entry points parse MDX too, but only `render_jsx` compiles it:

| Function                            | MDX handling                                                                                 |
| ----------------------------------- | -------------------------------------------------------------------------------------------- |
| `render_html` / `markdownToHtml`    | JSX and expressions pass through verbatim (escaped under `disable_raw_html`); ESM is dropped |
| `parse_to_ast` / `markdownToAst`    | `Block::MdxFlow { literal }` and `Block::MdxEsm { literal }` nodes                           |
| `render_markdown` / `astToMarkdown` | MDX blocks are written back verbatim                                                         |
| `render_ansi` / `markdownToAnsi`    | MDX blocks are skipped                                                                       |

### Examples

- [`examples/mdx`](examples/mdx): `cargo run --example mdx` prints the compiled module (`--components`, `--provider` for the other modes), and a Vite + React app renders the same `Guide.mdx`.
- The [playground](playground) has an MDX option with a multi-file workspace (`.mdx`, `.jsx`, `.js`), a live React preview and a JSX output tab.

## CLI

Render Markdown as HTML, ANSI terminal output, or an AST. Default format is `html`.

### npm

```bash
npx ironmark README.md
```

Or install globally:

```bash
npm install -g ironmark
ironmark README.md
```

### Rust

Native binary — faster startup, auto-detects terminal width via `$COLUMNS` / `tput cols`.

```bash
cargo install ironmark --features cli
ironmark README.md
```

### Options

Both CLIs support the same flags:

```text
OPTIONS (all formats):
    --format <html|ansi|ast>    Output format; ast also accepts 'json' (default: html)
    --preset <name>             Apply a named option preset: default, safe, strict, llm
    --safe                      Alias for --preset safe
    --no-hard-breaks            Don't turn soft newlines into hard line breaks
    --no-tables                 Disable pipe table syntax
    --no-highlight              Disable ==highlight== syntax
    --no-strikethrough          Disable ~~strikethrough~~ syntax
    --no-underline              Disable ++underline++ syntax
    --no-autolink               Disable bare URL auto-linking
    --no-task-lists             Disable - [x] task list syntax
    --math                      Enable $inline$ and $$display$$ math
    --wiki-links                Enable [[wiki link]] syntax
    --frontmatter               Skip a leading ---/+++ front matter block
    --capabilities              Print machine-readable capabilities JSON and exit
    --default-options           Print resolved default options JSON and exit
    --list-presets              Print all presets and exit
    --max-size N                Truncate input to N bytes (Rust only)
    -h, --help                  Print this help and exit
    -V, --version               Print version and exit

OPTIONS (ansi format only):
    --width N            Terminal column width (default: auto-detect, fallback 80)
    --padding N          Horizontal padding added to both sides of each line (default: 0)
    --no-color           Disable ANSI escape codes (plain text)
    -n, --line-numbers   Show line numbers in fenced code blocks
```

### Examples

```bash
# Render as HTML (default)
echo '# Hello' | npx ironmark
npx ironmark README.md

# Safe mode for untrusted content
npx ironmark --safe README.md
npx ironmark --preset safe README.md

# LLM-optimized output
npx ironmark --preset llm README.md

# Render as ANSI terminal output
npx ironmark --format ansi README.md
npx ironmark --format ansi --width 120 README.md
npx ironmark --format ansi --no-color README.md | less

# Render as AST (JSON)
npx ironmark --format ast README.md

# Inspect capabilities
npx ironmark --capabilities
npx ironmark --list-presets

# Rust (after cargo install ironmark --features cli)
echo '# Hello' | ironmark
ironmark --format ansi README.md
ironmark --format ansi --width 120 README.md
cat doc.md | ironmark --format ansi --math --wiki-links
```

## Rust

```bash
cargo add ironmark
```

```rust
use ironmark::{render_html, ParseOptions};

fn main() {
    // with defaults
    let html = render_html("# Hello\n\nThis is **fast**.", &ParseOptions::default());

    // with custom options
    let html = render_html("line one\nline two", &ParseOptions {
        hard_breaks: false,
        enable_strikethrough: false,
        ..Default::default()
    });

    // safe mode for untrusted input
    let html = render_html("<script>alert(1)</script>", &ParseOptions {
        disable_raw_html: true,
        max_input_size: 1_000_000, // 1 MB
        ..Default::default()
    });
}
```

### Cargo features

| Feature       | Default | Enables                                           |
| ------------- | ------- | ------------------------------------------------- |
| `html`        | yes     | Markdown → HTML (`render_html`)                   |
| `ansi`        | yes     | Markdown → ANSI terminal (`render_ansi_terminal`) |
| `html-parser` | yes     | HTML → AST → Markdown (`html_to_markdown`)        |
| `serde`       | no      | `Serialize`/`Deserialize` for the AST types       |
| `cli`         | no      | The `ironmark` binary                             |

Parsing to the AST (`parse_markdown`) and AST → Markdown (`render_markdown`) are always available. To build only what you use:

```toml
[dependencies]
ironmark = { version = "2", default-features = false, features = ["html"] }
```

### AST Output

`parse_to_ast()` returns the typed Rust AST (`Block`) directly:

```rust
use ironmark::{Block, ParseOptions, parse_markdown};

fn main() {
    let ast = parse_markdown("# Hello", &ParseOptions::default());

    match ast {
        Block::Document { children } => {
            println!("top-level blocks: {}", children.len());
        }
        _ => unreachable!("root nodes always Document"),
    }
}
```

Exported AST types:

- `Block`
- `ListKind`
- `TableData`
- `TableAlignment`

### HTML to Markdown

Convert HTML back to Markdown syntax:

```rust
use ironmark::{html_to_markdown, HtmlParseOptions};

fn main() {
    let md = html_to_markdown(
        "<h1>Hello</h1><p><strong>Bold</strong> text</p>",
        &HtmlParseOptions::default(),
    );
    // Returns: "# Hello\n\n**Bold** text"
}
```

For AST access, use `parse_html_to_ast()`:

```rust
use ironmark::{parse_html_to_ast, HtmlParseOptions, UnknownInlineHandling};

fn main() {
    // Default: strip unknown tags, keep text content
    let ast = parse_html_to_ast("<p>H<sub>2</sub>O</p>", &HtmlParseOptions::default());

    // Preserve unknown tags as raw HTML
    let ast = parse_html_to_ast(
        "<p>H<sub>2</sub>O</p>",
        &HtmlParseOptions {
            unknown_inline_handling: UnknownInlineHandling::PreserveAsHtml,
            ..Default::default()
        },
    );
}
```

`HtmlParseOptions` fields:

| Field                     | Type                    | Default     | Description                           |
| ------------------------- | ----------------------- | ----------- | ------------------------------------- |
| `max_nesting_depth`       | `usize`                 | `128`       | Limit nesting depth (DoS prevention)  |
| `unknown_inline_handling` | `UnknownInlineHandling` | `StripTags` | How to handle unknown HTML tags       |
| `max_input_size`          | `usize`                 | `0`         | Truncate input beyond this byte count |

`UnknownInlineHandling` variants:

- `StripTags` — Remove unknown tags, keep text content (default)
- `PreserveAsHtml` — Keep unknown tags as raw HTML in output

### AST to Markdown

Render an AST back to Markdown syntax:

```rust
use ironmark::{parse_markdown, render_markdown, ParseOptions};

fn main() {
    let ast = parse_markdown("# Hello\n\n**World**", &ParseOptions::default());
    let md = render_markdown(&ast);
    // Returns: "# Hello\n\n**World**"
}
```

### Plain text, headings and links

`render_text()` returns the document as plain text; `extract_headings()` returns every heading with the `id` the HTML renderer would give it; `extract_links()` returns every link and image with its resolved URL. All three need the `html` feature. `split_frontmatter()` splits a leading front matter block from the body; with `enable_frontmatter: true` every renderer skips that block. Need several of these? `analyze()` returns text, headings, links and front matter from one parse.

```rust
use ironmark::{ParseOptions, analyze, extract_headings, extract_links, render_text, split_frontmatter};

fn main() {
    let opts = ParseOptions::default();
    assert_eq!(render_text("# Hi *there*", &opts), "Hi there");

    let headings = extract_headings("# Hello *world*\n\n## Next", &opts);
    assert_eq!(headings[0].id, "hello-world");
    assert_eq!(headings[1].level, 2);

    let links = extract_links("[docs](/docs) ![logo](l.png)", &opts);
    assert_eq!((links[0].url.as_str(), links[1].image), ("/docs", true));

    let (matter, body) = split_frontmatter("---\ntitle: x\n---\n# Hi").unwrap();
    assert_eq!((matter, body), ("title: x\n", "# Hi"));

    let doc = analyze("# Hi\n\n[docs](/docs)", &opts);
    assert_eq!((doc.text.as_str(), doc.headings.len(), doc.links.len()), ("Hi\n\ndocs", 1, 1));
}
```

### ANSI Terminal Output

`render_ansi_terminal()` renders Markdown as ANSI-coloured terminal output. Pass `Some(&AnsiOptions { .. })` to control width, colour, and line numbers, or `None` for defaults.

````rust
use ironmark::{AnsiOptions, ParseOptions, render_ansi_terminal};

fn main() {
    // Defaults — width 80, colour enabled
    let out = render_ansi_terminal("# Hello\n\n**bold** and `code`", &ParseOptions::default(), None);
    print!("{out}");

    // Custom options — 120 columns, line numbers in code blocks
    let out = render_ansi_terminal(
        "# Hello\n\n```rust\nfn main() {}\n```",
        &ParseOptions::default(),
        Some(&AnsiOptions { width: 120, line_numbers: true, ..AnsiOptions::default() }),
    );
    print!("{out}");

    // With padding — adds horizontal spacing on both sides
    let out = render_ansi_terminal(
        "# Hello\n\n> A quote",
        &ParseOptions::default(),
        Some(&AnsiOptions { padding: 2, ..AnsiOptions::default() }),
    );
    print!("{out}");

    // Plain text — no ANSI codes (e.g. for writing to a file)
    let plain = render_ansi_terminal(
        "# Hello",
        &ParseOptions::default(),
        Some(&AnsiOptions { color: false, ..AnsiOptions::default() }),
    );
}
````

`AnsiOptions` fields:

| Field          | Type    | Default | Description                                                                                   |
| -------------- | ------- | ------- | --------------------------------------------------------------------------------------------- |
| `width`        | `usize` | `80`    | Column width for word-wrap, heading underlines, rule length. `0` = disable.                   |
| `color`        | `bool`  | `true`  | Emit ANSI 256-colour escape codes.                                                            |
| `line_numbers` | `bool`  | `false` | Show line numbers in fenced code blocks.                                                      |
| `padding`      | `usize` | `0`     | Horizontal padding added to both sides of each line, plus ⌈padding/2⌉ blank lines at the top. |

## C / C++

The crate compiles to a static library (`libironmark.a`) that exposes two C functions. A header is provided at `include/ironmark.h`.

### Build the library

```bash
cargo build --release
# output: target/release/libironmark.a
```

### Link

```sh
# Linux
cc -o example example.c -L target/release -l ironmark -lpthread -ldl

# macOS
cc -o example example.c -L target/release -l ironmark \
   -framework CoreFoundation -framework Security
```

### Usage

```c
#include "include/ironmark.h"
#include <stdio.h>

int main(void) {
    char *html = ironmark_render_html("# Hello\n\nThis is **fast**.");
    if (html) {
        printf("%s\n", html);
        ironmark_free(html);
    }
    return 0;
}
```

**Memory contract**: `ironmark_render_html` returns a heap-allocated string. You **must** free it with `ironmark_free`. Passing any other pointer to `ironmark_free` is undefined behaviour. Both functions are null-safe: `ironmark_render_html(NULL)` returns `NULL`; `ironmark_free(NULL)` is a no-op.

Parsing always uses the default `ParseOptions` (all extensions enabled, `disable_raw_html` off). Options are not yet configurable through the C API.

## Benchmarks

Compares ironmark against pulldown-cmark, comrak, markdown-it, markdown-rs and md4c (native Rust), and against markdown-wasm, md4w, marked and Bun's built-in parser (JS/WASM). All parsers are configured for the same feature set (CommonMark + GFM tables, strikethrough, task lists, autolink literals). Results are available at [ph1p.js.org/ironmark/#benchmark](https://ph1p.js.org/ironmark/#benchmark).

```bash
pnpm bench:rust                     # native Rust (criterion)
pnpm bench:js                       # WASM under Node
pnpm bench:bun                      # WASM under Bun, incl. Bun.markdown.html
pnpm bench:js --file my-doc.md      # benchmark your own Markdown
pnpm bench:js --check               # verify the parsers produce the same output first
pnpm bench                          # everything → benchmark/history snapshot
```

See [benchmark/README.md](benchmark/README.md) for all options, how to read the output, and how to get stable numbers.

## Development

This project uses [pnpm](https://pnpm.io/) for package management.

### Build from source

```bash
pnpm setup:wasm
pnpm build
```

| Command           | Description            |
| ----------------- | ---------------------- |
| `pnpm setup:wasm` | Install prerequisites  |
| `pnpm build`      | Release WASM build     |
| `pnpm build:dev`  | Debug WASM build       |
| `pnpm test`       | Run Rust tests         |
| `pnpm check`      | Format check           |
| `pnpm clean`      | Remove build artifacts |

### Troubleshooting

**`wasm32-unknown-unknown target not found`** or **`wasm-bindgen not found`** — run `pnpm setup:wasm` to install all prerequisites
