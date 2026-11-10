# ironmark

[![CI](https://github.com/ph1p/ironmark/actions/workflows/ci.yml/badge.svg)](https://github.com/ph1p/ironmark/actions/workflows/ci.yml) [![npm](https://img.shields.io/npm/v/ironmark)](https://www.npmjs.com/package/ironmark) [![crates.io](https://img.shields.io/crates/v/ironmark)](https://crates.io/crates/ironmark)

Fast Markdown parser written in Rust with **zero third-party** parsing dependencies. Outputs HTML, plain text, AST, ANSI terminal, Markdown, or MDX compiled to JSX, with plugins for Vite, esbuild, Rollup, webpack and Astro. Fully compliant with [CommonMark 0.31.2](https://spec.commonmark.org/0.31.2/) (652/652 spec tests pass). Available as a Rust crate and as an npm package via WebAssembly.

**[Documentation](https://ph1p.github.io/ironmark/)** · **[Playground](https://ph1p.github.io/ironmark/playground/)**

## Install

```bash
npm install ironmark   # Node, Bun and browsers (WebAssembly)
cargo add ironmark     # Rust
```

## Quick start

### JavaScript

```ts
import ironmark from "ironmark";

const doc = ironmark.parse("# Hello\n\n**World**");

doc.html(); // "<h1>Hello</h1>\n<p><strong>World</strong></p>\n"
doc.text(); // "Hello\n\nWorld"
doc.headings(); // [{ level: 1, text: "Hello", id: "hello" }]
doc.ast(); // { Document: { children: [...] } }
```

A document renders nothing until you ask for an output, and computes each output once. Every output also has a plain function (`markdownToHtml`, `markdownToAst` and so on). In the browser, `await ironmark.ready` once before the first call.

### Rust

```rust
use ironmark::{render_html, ParseOptions};

let html = render_html("# Hello\n\n**World**\n", &ParseOptions::default());
assert_eq!(html, "<h1>Hello</h1>\n<p><strong>World</strong></p>\n");

// Untrusted input: escape raw HTML and cap the input size.
let safe = render_html("<script>alert(1)</script>", &ParseOptions {
    disable_raw_html: true,
    max_input_size: 1_000_000,
    ..Default::default()
});
```

## Features

- **[Documents](https://ph1p.github.io/ironmark/js/documents)**: parse once, then ask for HTML, text, AST, headings, links, front matter or ANSI.
- **[HTML](https://ph1p.github.io/ironmark/js/html)** with heading ids, and **[plain text](https://ph1p.github.io/ironmark/js/text)** for search indexes and LLM prompts.
- **[AST](https://ph1p.github.io/ironmark/js/ast)** that renders back to Markdown.
- **[Terminal output](https://ph1p.github.io/ironmark/js/ansi)** with colours, wrapping and line numbers.
- **[HTML to Markdown](https://ph1p.github.io/ironmark/js/html-to-markdown)** conversion.
- **[MDX](https://ph1p.github.io/ironmark/mdx)** and plain Markdown compiled to JSX, with `@mdx-js/mdx`-style [components](https://ph1p.github.io/ironmark/mdx/components).
- **Extensions**, on by default: tables, task lists, strikethrough, highlight, underline, autolinks and more. See [options](https://ph1p.github.io/ironmark/config/options) and [presets](https://ph1p.github.io/ironmark/config/presets).
- **[Security](https://ph1p.github.io/ironmark/config/security)**: dangerous URLs are always stripped. Raw HTML escaping, nesting limits and input size limits handle untrusted input.
- **[Small bundles](https://ph1p.github.io/ironmark/js/subpaths)**: subpath imports such as `ironmark/html` load only the binary they need.
- **[Rust](https://ph1p.github.io/ironmark/rust)** API with Cargo features, plus a [C API](https://ph1p.github.io/ironmark/rust/c).

Not sure where to start? [Which function should I use?](https://ph1p.github.io/ironmark/which-function) maps common goals to functions. Upgrading from 2.x? See the [migration guide](https://ph1p.github.io/ironmark/js/migration).

## Build tool plugins

Import `.md` and `.mdx` files as components. `.mdx` files compile as MDX. `.md` files compile as plain Markdown and also export `html`, `headings` and `frontmatter`.

```ts
// vite.config.ts
import ironmark from "ironmark/vite";

export default { plugins: [ironmark()] };
```

```tsx
import Guide from "./guide.mdx";
import Readme, { headings } from "./README.md";

<Guide components={{ h1: Title }} />;
```

| Import             | Use it in                                    | Guide                                                      |
| ------------------ | -------------------------------------------- | ---------------------------------------------------------- |
| `ironmark/vite`    | Vite, SvelteKit, Nuxt, Vitest, Storybook     | [Vite](https://ph1p.github.io/ironmark/plugins/vite)       |
| `ironmark/esbuild` | esbuild, `Bun.build()`, `Bun.plugin()`, tsup | [esbuild](https://ph1p.github.io/ironmark/plugins/esbuild) |
| `ironmark/rollup`  | Rollup, Rolldown                             | [Rollup](https://ph1p.github.io/ironmark/plugins/rollup)   |
| `ironmark/webpack` | webpack 5, Rspack                            | [webpack](https://ph1p.github.io/ironmark/plugins/webpack) |
| `ironmark/astro`   | Astro with `@astrojs/react` or `/preact`     | [Astro](https://ph1p.github.io/ironmark/plugins/astro)     |

All plugins share the [same options](https://ph1p.github.io/ironmark/plugins#options).

## CLI

```bash
npx ironmark --format ansi README.md    # via npm
cargo install ironmark --features cli   # native binary
```

Formats are `html` (default), `ansi` and `ast`. See the [CLI docs](https://ph1p.github.io/ironmark/cli) for every flag.

## Benchmarks

ironmark is compared with pulldown-cmark, comrak, markdown-it, markdown-rs, md4c, markdown-wasm, md4w, marked and Bun's built-in parser, all on the same feature set. See the [latest results](https://ph1p.github.io/ironmark/playground/#benchmark) and [how to run them](https://ph1p.github.io/ironmark/benchmarks).

## Development

```bash
pnpm setup:wasm && pnpm build   # build the WASM package
cargo test --offline            # Rust tests, including the CommonMark spec
pnpm test:wasm                  # JS tests
```

More in the [development guide](https://ph1p.github.io/ironmark/development).

## License

[MIT](LICENSE)
