# WASM size-profile experiment

This is optional maintainer tooling, not package setup. The speed-first
`release-wasm` profile, normal builds, and published API remain unchanged.

## Build and check the candidates

```bash
pnpm build
node wasm/build-html.mjs
node wasm/build-size.mjs --check
```

The builder checks wasm-bindgen 0.2.129 and uses the same wasm-opt
`--enable-bulk-memory -O3` flags for every candidate. It builds full and
HTML-only artifacts with opt-level `s` (`release-wasm-size`) and experimental
`z` (`release-wasm-size-z`). `--check` exercises each actual candidate's exact
CommonMark/options/byte/limit fixtures and compiled exports; it does not merely
run the default artifact's tests.

| Candidate     | WASM path                                      | Synchronous Node entry |
| ------------- | ---------------------------------------------- | ---------------------- |
| Full `s`      | `wasm/pkg-size/full-s/ironmark_full_s_bg.wasm` | `wasm/size-full-s.js`  |
| HTML-only `s` | `wasm/pkg-size/html-s/ironmark_html_s_bg.wasm` | `wasm/size-html-s.js`  |
| Full `z`      | `wasm/pkg-size/full-z/ironmark_full_z_bg.wasm` | `wasm/size-full-z.js`  |
| HTML-only `z` | `wasm/pkg-size/html-z/ironmark_html_z_bg.wasm` | `wasm/size-html-z.js`  |

Outputs are ignored and isolated from both speed references.

## Measure size and runtime separately

```bash
node benchmark/measure.mjs --artifact wasm/pkg-size/full-s/ironmark_full_s_bg.wasm
node benchmark/measure.mjs --entry wasm/size-full-s.js
node benchmark/api.mjs --entry wasm/size-full-s.js
```

With `--entry`, startup changes but the other artifact/package fields still
describe the default full build. Compare full candidates with full-speed,
HTML-only candidates with HTML-speed, and keep inputs/options/samplers identical.
No benchmark history is written by these commands.

## Observed tradeoffs

Retain speed-first defaults; consider `s` only for an explicitly size-prioritized
use case, subject to review. `z` has a substantial warm-render cost and remains
diagnostic-only, not a shipping recommendation.

On Apple M1 Max with Node 26.3.1, Rust 1.99.1, wasm-bindgen 0.2.129, and
wasm-opt 130 (2026-10-31), full WASM gzip bytes were 181,943 for speed,
149,613 for `s`, and 121,429 for `z`; HTML-only sizes were 106,638, 96,432,
and 79,437 respectively. Across three balanced sessions, median paired HTML
time ratios aggregated geometrically were 1.046× / 1.039× for full / HTML-only
`s`, and 1.469× / 1.434× for `z` (above 1 means more time).

An initial unstable run was excluded from runtime conclusions but retained
locally. The single controlled retry required reference import, process-wall,
and README-render medians to remain within a predeclared 10% spread; the
maximum observed spread was 6.11%. These are workload- and machine-specific
measurements, not general speed guarantees or browser benchmarks.

No new npm exports or publish allowlist entries are added. Public artifacts
would still require package/type/browser validation and maintainer approval;
adding another artifact does not automatically shrink the npm tarball.
