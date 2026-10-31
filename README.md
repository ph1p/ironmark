# ironmark

[![CI](https://github.com/ph1p/ironmark/actions/workflows/ci.yml/badge.svg)](https://github.com/ph1p/ironmark/actions/workflows/ci.yml) [![npm](https://img.shields.io/npm/v/ironmark)](https://www.npmjs.com/package/ironmark) [![crates.io](https://img.shields.io/crates/v/ironmark)](https://crates.io/crates/ironmark)

Fast Markdown parser written in Rust with **zero third-party** parsing dependencies. Outputs HTML, AST, ANSI terminal, or Markdown. Fully compliant with [CommonMark 0.31.2](https://spec.commonmark.org/0.31.2/) (652/652 spec tests pass). Available as a Rust crate and as an npm package via WebAssembly.

## Table of Contents

- [Quick start](#quick-start)
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
  - [Using ironmark in AI pipelines](#using-ironmark-in-ai-pipelines)
  - [Presets](#presets)
  - [Introspection helpers](#introspection-helpers)
  - [Utility functions](#utility-functions)
- [Configuration](#configuration)
  - [Extensions](#extensions)
  - [Security](#security)
  - [Other Options](#other-options)
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
import { renderHtml, parseMarkdown } from "ironmark";

// Markdown → HTML
const html = renderHtml("# Hello\n\n**World**");

// Markdown → AST (parsed JavaScript object)
const ast = parseMarkdown("# Hello\n\n**World**");
```

## Which function should I use?

| Goal                            | Function                                            |
| ------------------------------- | --------------------------------------------------- |
| Render Markdown to HTML         | `renderHtml(input, options?)`                       |
| Parse Markdown to AST object    | `parseMarkdown(input, options?)`                    |
| Normalize / round-trip Markdown | `parseMarkdown()` + `renderMarkdown()`              |
| Terminal / CLI output           | `renderAnsiTerminal(input, options?, ansiOptions?)` |
| Convert HTML back to Markdown   | `htmlToMarkdown(html)` / `parseHtmlToAst(html)`     |

## JavaScript / TypeScript

### Node.js

Node loads the packaged `wasm/pkg/ironmark_bg.wasm` synchronously relative to the
module — no `init()` needed:

```ts
import { renderHtml, parseMarkdown } from "ironmark";

const html = renderHtml("# Hello\n\nThis is **fast**.");
const ast = parseMarkdown("# Hello");
```

Keep the WASM asset and glue alongside the Node entry when copying or bundling
the package; it is not a self-contained single-file JavaScript bundle.
Node bundlers may need asset-copy configuration to preserve this layout.
The browser entry still uses asynchronous `init()`.

### Markdown → HTML

```ts
import { renderHtml } from "ironmark";

const html = renderHtml("# Hello\n\nThis is **fast**.");
```

### Markdown → AST

`parseMarkdown()` returns a parsed JavaScript array — no `JSON.parse()` needed.

```ts
import { parseMarkdown } from "ironmark";

const ast = parseMarkdown("# Hello\n\n- [x] done");
// ast is a JavaScript array of block nodes
console.log(ast[0]); // { t: "Heading", level: 1, ... }
```

### Markdown → AST → Markdown

```ts
import { parseMarkdown, renderMarkdown } from "ironmark";

const ast = parseMarkdown("**Hello**");
const md = renderMarkdown(ast);
// Returns: "**Hello**"
```

### Safe HTML rendering

Use `safe: true` for **any untrusted input** — it disables raw HTML passthrough and enables the GFM tag filter.

```ts
import { renderHtml } from "ironmark";

// Simple safe flag
const html = renderHtml(userInput, { safe: true });

// Or use the built-in preset
const html = renderHtml(userInput, { preset: "safe" });
```

What `safe: true` does:

- Sets `disableRawHtml: true` — all inline and block HTML is escaped
- Sets `tagFilter: true` — dangerous tags (`<script>`, `<iframe>`, etc.) are escaped

Dangerous URI schemes (`javascript:`, `vbscript:`, `data:` except `data:image/…`) are **always** stripped from link destinations regardless of options.

### ANSI Terminal Output

Use `renderAnsiTerminal()` to render Markdown as coloured terminal output (ANSI 256-colour escape codes).

````ts
import { renderAnsiTerminal } from "ironmark";

// Defaults: width 80, color enabled
const ansi = renderAnsiTerminal("# Hello\n\n**bold** and `code`");
process.stdout.write(ansi);

// Custom options
const ansi = renderAnsiTerminal(
  "# Hello\n\n```rust\nfn main() {}\n```",
  {},
  { width: 120, lineNumbers: true },
);
process.stdout.write(ansi);

// Plain text — no ANSI codes (useful for piping to files)
const plain = renderAnsiTerminal("# Hello", {}, { color: false });
````

#### ANSI options

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
const md = htmlToMarkdown("<p>H<sub>2</sub>O</p>", true);
// Returns: "H<sub>2</sub>O"
```

For AST access, use `parseHtmlToAst()`:

```ts
import { parseHtmlToAst, renderMarkdown } from "ironmark";

const ast = parseHtmlToAst("<h1>Hello</h1><p>World</p>");
const md = renderMarkdown(ast);
```

### Browser / Bundler

Call `init()` once before using any function. It's idempotent and optionally accepts a custom `.wasm` URL.

```ts
import { init, renderHtml, parseMarkdown } from "ironmark";

await init();

const html = renderHtml("# Hello\n\nThis is **fast**.");
const ast = parseMarkdown("# Hello");
```

#### Vite

```ts
import { init, renderHtml } from "ironmark";
import wasmUrl from "ironmark/ironmark.wasm?url";

await init(wasmUrl);

const html = renderHtml("# Hello\n\nThis is **fast**.");
```

### Using ironmark in AI pipelines

ironmark is designed to be a reliable, deterministic parsing layer for AI agents and code generation tools.

**Recommended pattern for agents:**

```ts
import { parseMarkdown, renderHtml, renderMarkdown, getCapabilities } from "ironmark";

// 1. Inspect capabilities at startup
const caps = getCapabilities();
// { astSchemaVersion: "2", formats: [...], presets: [...] }

// 2. Parse with deterministic options
const ast = parseMarkdown(content, { preset: "llm" });

// 3. Render or transform
const html = renderHtml(content, { preset: "llm" });
const normalized = renderMarkdown(ast);
```

**The `llm` preset** produces deterministic, structure-first output:

- Disables autolinks, wiki links, math, hard breaks (ambiguous in AI-generated text)
- Enables heading IDs and whitespace normalization
- Disables raw HTML passthrough (safe by default)

```ts
import { parseMarkdown, extractHeadings, summarizeAst } from "ironmark";

const ast = parseMarkdown(content, { preset: "llm" });

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
import { renderHtml, parseMarkdown } from "ironmark";

// Use a preset
const html = renderHtml(content, { preset: "safe" });

// Presets compose with explicit options (explicit wins)
const html = renderHtml(content, { preset: "llm", enableTables: false });
```

### Introspection helpers

These functions return stable machine-readable metadata. Useful for agents and tooling to discover available features at runtime.

```ts
import { getCapabilities, getAstSchemaVersion, getDefaultOptions, getPresets } from "ironmark";

// What does this build support?
const caps = getCapabilities();
// {
//   astSchemaVersion: "2",
//   formats: ["html", "ast", "markdown", "ansi"],
//   presets: ["default", "safe", "strict", "llm"],
//   extensions: [...],
//   security: [...]
// }

// What is the current AST schema version?
const version = getAstSchemaVersion(); // "2"

// What are the resolved defaults for every option?
const defaults = getDefaultOptions();

// What options does each preset set?
const presets = getPresets();
console.log(presets.llm);
```

### Utility functions

```ts
import { parseMarkdown, extractHeadings, summarizeAst } from "ironmark";

const ast = parseMarkdown("# Hello\n\n## World\n\nA paragraph.");

// Extract heading structure
const headings = extractHeadings(ast);
// [
//   { level: 1, text: "Hello", id: "hello" },
//   { level: 2, text: "World", id: "world" },
// ]

// Count node types
const summary = summarizeAst(ast);
// { blockCount: 3, nodeCounts: { Heading: 2, Paragraph: 1 } }
```

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
| Heading IDs         | `enableHeadingIds`         | `enable_heading_ids`          | Auto `id=` on headings from slugified text             |
| Heading anchors     | `enableHeadingAnchors`     | `enable_heading_anchors`      | `<a class="anchor">` inside each heading (implies IDs) |
| Permissive headings | `permissiveAtxHeaders`     | `permissive_atx_headers`      | Allow `#Heading` without space after `#`               |

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
