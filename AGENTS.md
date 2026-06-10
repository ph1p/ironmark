# AGENTS.md

This file provides guidance to AI coding agents when working with code in this repository.

## Project

**ironmark** — a fast Markdown-to-HTML parser in Rust, fully CommonMark 0.31.2 compliant (652/652 spec tests). Also includes an ANSI terminal renderer for coloured, formatted terminal output. Published as both a Rust crate and an npm package (via WASM). Zero third-party parsing dependencies (only `memchr` and `rustc-hash`).

## Commands

```bash
# Rust
cargo test --offline          # run all tests (spec + integration + doctests)
cargo test --offline -- commonmark  # run only CommonMark spec tests
cargo test --offline -- <name>      # run a specific test by name
cargo bench                          # criterion benchmarks (benchmark/parse.rs)
cargo bench --features bench-md4c   # include md4c (requires: brew install md4c)
cargo fmt                     # format Rust code
cargo clippy                  # lint (deny undocumented_unsafe_blocks)

# WASM build
pnpm setup:wasm               # install wasm32 target + wasm-bindgen-cli
pnpm build                    # release WASM build (+ wasm-opt)
pnpm build:dev                # debug WASM build

# JS/TS
pnpm check                    # cargo fmt --check && oxlint && oxfmt --check
pnpm fmt                      # cargo fmt && oxfmt --write
pnpm lint                     # oxlint
pnpm test                     # alias for cargo test --offline
pnpm bench                    # full benchmark: Rust (md4c) + Bun + WASM → single history JSON (requires: brew install md4c)
```

## Architecture

Two-phase pipeline: **block parsing → inline parsing → HTML rendering**.

### Phase 1: Block parsing (`src/block/`)

- `mod.rs` — public `parse()` and `parse_to_ast()` entry points; block-level structures (open-block stack)
- `parser.rs` — line-by-line block parser; container tracking (blockquotes, lists)
- `leaf_blocks.rs` — ATX/setext headings, code blocks, thematic breaks
- `html_block.rs` — HTML block detection (7 conditions)
- `link_ref_def.rs` — link reference definition extraction

### Phase 2: Inline parsing (`src/inline/`)

- `mod.rs` — inline parser entry, delimiter run algorithm (emphasis, extensions)
- `scanner.rs` — character-level scanning, autolinks, entity resolution
- `links.rs` — link/image bracket matching, reference resolution
- `render.rs` — inline content → HTML string output

### HTML rendering (`src/render/`)

- `mod.rs` — stack-based block renderer that calls inline parsing for leaf block content; receives `&ParseOptions` to control extension behavior
- `slug.rs` — heading slug/anchor id generation
- Render mode defers leaf text (paragraphs, ATX/setext headings, fenced code) as source byte ranges (`code_src_ranges`) instead of copying into the AST; the renderer resolves them in document order. Every render-mode empty-literal code block and empty-raw heading must push a range (even an empty one) to keep that index in sync.

### ANSI terminal rendering (`src/ansi/`)

- Coloured, formatted terminal output using ANSI escape codes
- Suitable for `cat`-like tools or CLI help text
- `mod.rs` — public `render_ansi()` entry point with `AnsiOptions`
- `renderer.rs` — `AnsiRenderer` block-level renderer
- `inline.rs` — HTML-to-ANSI tag translator, inline renderer
- `wrap.rs` — text wrapping, visible length calculation, tab expansion helpers
- `constants.rs` — ANSI 256-colour escape code constants
- Features: configurable width, colour toggle, line numbers in code blocks, horizontal and vertical padding

### Supporting modules

- `ast.rs` — AST node types (`Block` enum, `ListKind`, `TableData`, `TableAlignment`)
- `html.rs` — HTML escaping, URI sanitization, dangerous-protocol stripping
- `entities/mod.rs` — HTML5 entity resolution
- `entities/data.rs` — entity lookup tables
- `lib.rs` — `ParseOptions` struct, public API re-exports, utility functions

### WASM layer (`wasm/`)

- `wasm/src/lib.rs` — wasm-bindgen bindings exposing `parse()` and `parseToAst()`
- `wasm/node.js` / `wasm/web.js` — JS entry points (node: sync embedded, web: async init)
- `wasm/shared.js` — shared JS logic between node/web
- `wasm/index.d.ts` — TypeScript type definitions
- `wasm/build.mjs` — post-build script for WASM packaging

### Tests (`tests/`)

- `tests/commonmark_spec.rs` — runs all 652 CommonMark 0.31.2 spec examples from JSON
- `tests/parser.rs` — 76 integration tests for extensions and edge cases
- `tests/spec/spec-0.31.2.json` — CommonMark spec test data

### Benchmark (`benchmark/`)

- `benchmark/parse.rs` — criterion benchmarks comparing ironmark vs comrak, pulldown-cmark, markdown-it, markdown-rs; md4c included when `--features bench-md4c` is passed (requires system `md4c`: `brew install md4c`)
- `benchmark/report.mjs` — runs Bun + WASM benchmarks, reads the Rust CSV, merges all results into a single dated history JSON; deletes the CSV after reading
- `benchmark/history/YYYY-MM-DD.json` — dated snapshot (Rust + WASM + Bun results); commit these to build a performance trend record; CSV files are temporary intermediates deleted by `report.mjs`

### Playground (`playground/`)

- Vite + TypeScript web app for interactive testing

## Key conventions

- Three public entry points: `parse(markdown, &ParseOptions)` → HTML string, `parse_to_ast(markdown, &ParseOptions)` → `Vec<Block>`, `render_ansi(markdown, &ParseOptions, &AnsiOptions)` → ANSI terminal string
- Optional `serde` feature flag for AST serialization
- Spec tests run with `hard_breaks: false` and `enable_autolink: false` to match CommonMark baseline
- All extensions default to `true` (`hard_breaks`, `enable_highlight`, `enable_strikethrough`, `enable_underline`, `enable_tables`, `enable_autolink`, `enable_task_lists`)
- ANSI options default to `width: 80`, `color: true`, `line_numbers: false`, `padding: 0`
- Security options: `disable_raw_html` (escapes HTML), `max_nesting_depth` (default 128), `max_input_size` (default 0 = unlimited)
- Dangerous URIs (`javascript:`, `vbscript:`, `data:` except `data:image/…`) always stripped regardless of options
- Extension delimiters (`~~`, `==`, `++`) use `tag_size` encoding: 1=em, 2=strong, 3=del, 4=mark, 5=u
- `#![deny(clippy::undocumented_unsafe_blocks)]` enforced in lib.rs
- Rust edition 2024; release profile uses LTO + codegen-units=1 + panic=abort + strip=true
- WASM uses a separate `release-wasm` profile inheriting from release with `wasm-opt -O3`
- Commits follow conventional commits (semantic-release via `.releaserc.cjs`)
- Workspace members: root crate (`ironmark`) + `wasm/` crate (`ironmark-wasm`)
- Dependencies: only `memchr` and `rustc-hash` at runtime; `serde` optional for AST serialization
