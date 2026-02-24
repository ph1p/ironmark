# ironmark

Fast Markdown-to-HTML parser written in Rust. Fully compliant with [CommonMark 0.31.2](https://spec.commonmark.org/0.31.2/) (652/652 spec tests pass). Available as a Rust crate and as an npm package via WebAssembly.

## Features

- Zero third-party parsing dependencies
- Headings (`#` through `######`) and setext headings
- Paragraphs, emphasis, strong emphasis, inline code
- ~~strikethrough~~, ==highlight==, ++underline++
- Links, reference links, autolinks (angle-bracket and bare URL/email), images
- Ordered and unordered lists with nesting, task lists (checkboxes)
- Blockquotes, horizontal rules
- Fenced and indented code blocks
- Tables with alignment
- Raw HTML passthrough
- Backslash escapes and HTML entities

## JavaScript / TypeScript

### Install

```bash
npm install ironmark
```

### Usage (Node.js)

WASM is embedded and loaded synchronously — no `init()` needed:

```ts
import { parse } from "ironmark";

const html = parse("# Hello\n\nThis is **fast**.");

const bytes = new TextEncoder().encode("# Hello from bytes");
const html2 = parse(bytes);
```

### Usage (Browser / Bundler)

Call `init()` once before using `parse()`:

```ts
import { init, parse } from "ironmark";

await init();

const html = parse("# Hello\n\nThis is **fast**.");
```

`init()` is idempotent (safe to call multiple times) and can optionally take a custom URL to the `.wasm` file.

### Options

```ts
import { parse } from "ironmark";

const html = parse("line one\nline two", {
  hardBreaks: false, // every newline becomes <br /> (default: true)
  enableHighlight: true, // ==highlight== → <mark> (default: true)
  enableStrikethrough: true, // ~~strike~~ → <del> (default: true)
  enableUnderline: true, // ++underline++ → <u> (default: true)
  enableTables: true, // pipe tables (default: true)
  enableAutolink: true, // bare URLs & emails → <a> (default: true)
  enableTaskLists: true, // - [ ] / - [x] checkboxes (default: true)
});
```

### Build from source

```bash
npm run setup:wasm
npm run build
```

| Command              | Description            |
| -------------------- | ---------------------- |
| `npm run setup:wasm` | Install prerequisites  |
| `npm run build`      | Release WASM build     |
| `npm run build:dev`  | Debug WASM build       |
| `npm run test`       | Run Rust tests         |
| `npm run check`      | Format check + tests   |
| `npm run clean`      | Remove build artifacts |

## Rust

### Add to your project

```bash
cargo add ironmark
```

### Usage

```rust
use ironmark::{parse, ParseOptions};

fn main() {
    let html = parse("# Hello\n\nThis is **fast**.", &ParseOptions::default());
    println!("{html}");
}
```

### With options

```rust
use ironmark::{parse, ParseOptions};

fn main() {
    let options = ParseOptions {
        hard_breaks: true,
        enable_strikethrough: false, // disable ~~strikethrough~~
        enable_autolink: true,      // bare URLs & emails → <a>
        enable_task_lists: true,    // - [ ] / - [x] checkboxes
        ..Default::default()
    };

    let html = parse("line one\nline two", &options);
    println!("{html}");
}
```

## Troubleshooting

### `wasm32-unknown-unknown target not found` with Homebrew Rust

The build scripts prepend `$HOME/.cargo/bin` to `PATH` so that rustup-managed binaries take priority. If the error persists:

```bash
npm run setup:wasm
```

### `wasm-bindgen` not found

```bash
npm run setup:wasm
```
