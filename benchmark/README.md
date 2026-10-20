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
pnpm bench:js --check             # do the parsers even agree? (no timing)
pnpm bench:quick                  # Rust, ironmark only (~10 s)
```

## Benchmark your own Markdown

The built-in suite uses synthetic documents. To see how the parsers handle your content:

```bash
pnpm bench:js --file docs/guide.md
pnpm bench:bun --file docs/guide.md --parsers ironmark,bun
```

## What makes the comparison fair

- **Same feature set.** Every parser, Rust and JS, is configured for one shared profile, `gfm`: CommonMark plus GFM tables, strikethrough, task lists and autolink literals. ironmark's own extras (hard breaks, `==highlight==`, `++underline++`, heading ids) are off, because the others don't do that work. `--profile commonmark` turns all extensions off.
- **Same output, checked.** Every timed run first renders each case once per parser and compares the HTML, with markup flavour (`<b>` vs `<strong>`, attribute order, heading anchors) normalised away. A parser whose output differs from the majority is flagged `≠ output differs`: its time is for different work. `--check` prints the full report: CommonMark spec conformance per parser and per-case agreement with diff excerpts.
- **Noise shown.** Each result carries its interquartile range. When two parsers' ranges overlap, the runner calls it a tie (`≈ fastest … within noise`) instead of declaring a winner.
- **Machine recorded.** Output and snapshots include CPU, cores, memory, OS and runtime versions. Numbers are only comparable on the same machine.

Known gaps that normalisation can't hide:

| Parser              | Gap                                                                     |
| ------------------- | ----------------------------------------------------------------------- |
| pulldown-cmark      | no autolink literals                                                    |
| markdown-it (Rust)  | no task lists                                                           |
| markdown-wasm, md4w | always emit heading anchors (`<a class="anchor">`), no flag to turn off |
| markdown-wasm       | lowest spec conformance (~79%); differs on many edge cases              |

The concatenated spec document (`spec (all examples)`) is a stress test, not a correctness test. Its examples interact (for example, duplicate link reference definitions), so the parsers are expected to disagree there.

## `js-bench.mjs` options

Run it under `node` or `bun`. Whichever runtime runs the file is the one being measured.

| Option           | Effect                                                                    |
| ---------------- | ------------------------------------------------------------------------- |
| `--parsers a,b`  | Only these parsers (`ironmark`, `markdown-wasm`, `md4w`, `marked`, `bun`) |
| `--suite <text>` | Only sections or cases whose name contains `<text>` (case-insensitive)    |
| `--file <path>`  | Benchmark one Markdown file instead of the suite                          |
| `--profile <p>`  | `gfm` (default) or `commonmark` (all extensions off)                      |
| `--check`        | Spec conformance + output agreement report, no timing                     |
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
  mixed 9.8 KB (9.8 KB)
    ironmark          75.4 µs   ±3.2%    132.6 MB/s   fastest
    md4w              75.9 µs   ±3.6%    131.8 MB/s   ≈ fastest (1.01x, within noise)
    markdown-wasm     80.2 µs   ±2.1%    124.6 MB/s   1.06x slower
    marked           636.3 µs   ±6.4%     15.7 MB/s   8.44x slower
```

- **time**: median time per call. Compare parsers on this number.
- **±%**: half the interquartile range relative to the median. Large values (say ±20% or more) mean the machine was busy or the call is too short to time well: rerun.
- **MB/s**: input bytes per second (1 MB = 10⁶ bytes). Useful for comparing cases of different sizes.
- **within noise**: the two parsers' interquartile ranges overlap. Treat it as a tie.
- **≠ output differs**: this parser rendered different HTML from the others on this input. Run `--check` to see where.
- **Summary**: compares ironmark with the _fastest other parser_ on each case and counts faster / within noise / slower. The geomean is the typical speedup, and no single big input can dominate it.
- **single sample**: the parser took more than 1 s for one call. That almost always means quadratic behaviour (marked on `emphasis ×10k`, for example). It gets measured once, not repeatedly.

Kinds matter when comparing: WASM parsers pay to copy strings across the JS↔WASM boundary on every call, `marked` runs as plain JS, and `bun` is native code with no boundary at all.

## Getting trustworthy numbers

- Close other heavy apps and plug in your laptop. On macOS, keep it from sleeping.
- `--quick` and `BENCH_QUICK=1` give noisy numbers. Use them for direction, never record them.
- Rerun if a result surprises you. Trust "within noise" over small ratios.
- Each parser warms up separately, and the inputs are identical across parsers and runtimes.

## Recording a snapshot

```bash
brew install md4c   # the Rust side includes md4c
pnpm bench          # Rust + Node + Bun → history/YYYY-MM-DD_HH-MM-SS.json
```

Bun is optional: without it, the Bun section is skipped. Commit the new history file to extend the trend chart. The playground History tab tracks ten cases that every runtime runs (`TREND_CASES` in `generators.mjs`). Snapshots record the machine (`env`) and profile; compare trend points only from the same machine.

Snapshots before 2026-10-20 used different parser configurations (md4c with underline and hard breaks, markdown-it with syntax highlighting and typographer, ironmark with all its extensions on), so they are not directly comparable with later ones.

## Adding a parser

- **JS/WASM**: add an entry to `PARSERS` in `js-bench.mjs` (with `kind` and a `load(profile)` that configures the parser for that profile and returns `(markdown) => html`), add the package as a dev dependency, and give it a colour and label in `playground/src/components/BenchmarkPage.tsx`.
- **Rust**: add it to `benchmark/parse.rs` (configured for the `gfm` profile, see the comment above the parser wrappers) and the `[dev-dependencies]` in `Cargo.toml`.

Then run `--check` and add any feature it can't match to the gaps table above.

## Files

| File              | Purpose                                                                        |
| ----------------- | ------------------------------------------------------------------------------ |
| `parse.rs`        | criterion benchmarks (writes a temporary CSV to `history/`)                    |
| `js-bench.mjs`    | JS runner for Node and Bun                                                     |
| `report.mjs`      | `pnpm bench` orchestrator: WASM build → Node + Bun → merge Rust CSV → snapshot |
| `generators.mjs`  | Shared inputs, sampling, history and playground-data helpers                   |
| `all_features.md` | Input that exercises every extension                                           |
| `history/`        | Committed snapshots that make up the performance trend                         |
