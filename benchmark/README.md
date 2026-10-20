# Benchmarks

ironmark is measured in three places, because "how fast is it" depends on where you call it from:

| Runner                 | Measures                                                                        | Command           |
| ---------------------- | ------------------------------------------------------------------------------- | ----------------- |
| `parse.rs` (criterion) | Native Rust: ironmark vs pulldown-cmark, comrak, markdown-it, markdown-rs, md4c | `pnpm bench:rust` |
| `js-bench.mjs` on Node | ironmark WASM vs markdown-wasm, md4w (WASM) and marked (pure JS)                | `pnpm bench:js`   |
| `js-bench.mjs` on Bun  | same as Node, plus Bun's native `Bun.markdown.html`                             | `pnpm bench:bun`  |

`pnpm bench` runs all three and records one snapshot in `history/`. The [playground Benchmarks page](https://ph1p.js.org/ironmark/#benchmark) is built from those snapshots.

## Quick start

```bash
pnpm install
pnpm setup:wasm && pnpm build     # the JS runners need the WASM build

pnpm bench:js                     # full JS suite under Node (~40 s)
pnpm bench:js --quick             # short sampling, for a quick look
pnpm bench:quick                  # Rust, ironmark only (~10 s)
```

## Benchmark your own Markdown

The built-in suite uses synthetic documents. To see how the parsers handle your content:

```bash
pnpm bench:js --file docs/guide.md
pnpm bench:bun --file docs/guide.md --parsers ironmark,bun
```

## `js-bench.mjs` options

Run it under `node` or `bun`. Whichever runtime runs the file is the one being measured.

| Option           | Effect                                                                    |
| ---------------- | ------------------------------------------------------------------------- |
| `--parsers a,b`  | Only these parsers (`ironmark`, `markdown-wasm`, `md4w`, `marked`, `bun`) |
| `--suite <text>` | Only sections or cases whose name contains `<text>` (case-insensitive)    |
| `--file <path>`  | Benchmark one Markdown file instead of the suite                          |
| `--quick`        | Short sampling budget (also enabled by `BENCH_QUICK=1`)                   |
| `--save`         | Write a history snapshot (full suite only)                                |
| `--json`         | Print results as JSON on stdout. Progress goes to stderr                  |
| `--list`         | List parsers and suite cases                                              |

Examples:

```bash
node benchmark/js-bench.mjs --suite pathological
bun benchmark/js-bench.mjs --parsers ironmark,bun          # head-to-head vs Bun native
node benchmark/js-bench.mjs --suite table --json | jq .
```

## Reading the output

```
  table 1k rows (88.9 KB)
    ironmark       median   567.9 µs   p95   765.3 µs   fastest
    md4w           median    1.14 ms   p95    1.92 ms   2.01x slower
```

- **median**: time per call. Compare parsers on this number.
- **p95**: tail latency. A large gap between median and p95 usually means GC pauses or a noisy machine.
- **Summary**: compares ironmark with the _fastest other parser_ on each case. The geomean is the typical speedup, and no single big input can dominate it.
- **single sample**: the parser took more than 1 s for one call. That almost always means quadratic behaviour (marked on `emphasis ×10k`, for example). It gets measured once, not repeatedly.

Kinds matter when comparing: WASM parsers pay to copy strings across the JS↔WASM boundary on every call, `marked` runs as plain JS, and `bun` is native code with no boundary at all.

## Getting trustworthy numbers

- Close other heavy apps and plug in your laptop. On macOS, keep it from sleeping.
- `--quick` and `BENCH_QUICK=1` give noisy numbers. Use them for direction, never record them.
- Rerun if a result surprises you. Differences under about 5% are usually noise.
- Each parser warms up separately, and the inputs are identical across parsers and runtimes.

## Recording a snapshot

```bash
brew install md4c   # the Rust side includes md4c
pnpm bench          # Rust + Node + Bun → history/YYYY-MM-DD_HH-MM-SS.json
```

Bun is optional: without it, the Bun section is skipped. Commit the new history file to extend the trend chart.

## Adding a parser

- **JS/WASM**: add an entry to `PARSERS` in `js-bench.mjs` (with `kind` and a `load()` that returns `(markdown) => html`), add the package as a dev dependency, and give it a colour and label in `playground/src/components/BenchmarkPage.tsx`.
- **Rust**: add it to `benchmark/parse.rs` and the `[dev-dependencies]` in `Cargo.toml`.

## Files

| File              | Purpose                                                                        |
| ----------------- | ------------------------------------------------------------------------------ |
| `parse.rs`        | criterion benchmarks (writes a temporary CSV to `history/`)                    |
| `js-bench.mjs`    | JS runner for Node and Bun                                                     |
| `report.mjs`      | `pnpm bench` orchestrator: WASM build → Node + Bun → merge Rust CSV → snapshot |
| `generators.mjs`  | Shared inputs, sampling, history and playground-data helpers                   |
| `all_features.md` | Input that exercises every extension                                           |
| `history/`        | Committed snapshots that make up the performance trend                         |
