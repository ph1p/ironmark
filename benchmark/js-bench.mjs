/**
 * JS-side benchmark: ironmark WASM vs other JS-callable Markdown parsers.
 * Runs under Node or Bun — whichever runtime executes this file is the one
 * being measured. Under Bun, Bun's native `Bun.markdown.html` joins in.
 *
 *   node benchmark/js-bench.mjs [options]
 *   bun  benchmark/js-bench.mjs [options]
 *
 * Options:
 *   --parsers a,b    only run these parsers (default: all available)
 *   --suite <text>   only run sections/cases whose name contains <text>
 *   --file <path>    benchmark a single Markdown file instead of the suite
 *   --quick          short sampling budget (noisy — for dev loops only)
 *   --save           write a timestamped history JSON to benchmark/history/
 *   --json           print results as JSON on stdout (logs go to stderr)
 *   --list           list parsers and suite cases, then exit
 *
 * Requires the WASM build (`pnpm build`) for ironmark.
 */

import { existsSync, readFileSync } from "node:fs";
import { createRequire } from "node:module";
import { join, relative, resolve } from "node:path";
import { parseArgs } from "node:util";
import {
  ROOT,
  fmtNs,
  fmtBytes,
  loadSpecMarkdown,
  loadAllFeatures,
  genInlineHeavy,
  genHeadings,
  genNestedList,
  genTable,
  genCodeBlocks,
  genMixedDoc,
  genPathologicalBackticks,
  genPathologicalEmphasis,
  genManyRefLinks,
  collectSamples,
  median,
  writeHistoryJson,
} from "./generators.mjs";

const require = createRequire(import.meta.url);
const isBun = typeof Bun !== "undefined";
const runtime = isBun ? "bun" : "wasm"; // history JSON key; "wasm" = Node runtime
const runtimeLabel = isBun ? `Bun ${Bun.version}` : `Node ${process.versions.node}`;

// ─── CLI ──────────────────────────────────────────────────────────────────────

const { values: args } = parseArgs({
  options: {
    parsers: { type: "string" },
    suite: { type: "string" },
    file: { type: "string" },
    quick: { type: "boolean", default: process.env.BENCH_QUICK === "1" },
    save: { type: "boolean", default: false },
    json: { type: "boolean", default: false },
    list: { type: "boolean", default: false },
    help: { type: "boolean", short: "h", default: false },
  },
});

if (args.help) {
  const src = readFileSync(new URL(import.meta.url), "utf8");
  console.log(
    src
      .slice(src.indexOf("/**") + 3, src.indexOf("*/"))
      .replace(/^ \* ?/gm, "")
      .trim(),
  );
  process.exit(0);
}

if (args.save && (args.file || args.suite || args.parsers || args.quick)) {
  console.error("--save records the full suite only; drop --file/--suite/--parsers/--quick.");
  process.exit(1);
}

const log = args.json ? (...a) => process.stderr.write(a.join(" ") + "\n") : console.log;

// ─── Parsers ──────────────────────────────────────────────────────────────────

// kind: wasm | js | native — shown in output so results aren't misread.
// load() returns the render function, or null when unavailable.
const PARSERS = {
  ironmark: {
    kind: "wasm",
    async load() {
      if (!existsSync(join(ROOT, "wasm", "node.js"))) {
        console.error("ironmark WASM build missing — run `pnpm build` first.");
        process.exit(1);
      }
      const { renderHtml } = await import("../wasm/node.js");
      return (input) => renderHtml(input);
    },
  },
  "markdown-wasm": {
    kind: "wasm",
    async load() {
      const markdownWasm = require("markdown-wasm/dist/markdown.node.js");
      return (input) => markdownWasm.parse(input);
    },
  },
  md4w: {
    kind: "wasm",
    async load() {
      const { init, mdToHtml } = await import("md4w");
      await init();
      return (input) => mdToHtml(input);
    },
  },
  marked: {
    kind: "js",
    async load() {
      const { marked } = await import("marked");
      return (input) => marked.parse(input);
    },
  },
  bun: {
    kind: "native",
    async load() {
      return isBun && Bun.markdown?.html ? (input) => Bun.markdown.html(input) : null;
    },
  },
};

// ─── Suite ────────────────────────────────────────────────────────────────────

// Section titles and case names are part of the history JSON schema — the
// playground trend reads "CommonMark Spec" / "spec (all examples)".
function buildSuite() {
  if (args.file) {
    const path = resolve(args.file);
    const name = relative(process.cwd(), path);
    return [{ title: "Custom file", cases: [{ name, input: readFileSync(path, "utf8") }] }];
  }
  return [
    {
      title: "CommonMark Spec",
      cases: [{ name: "spec (all examples)", input: loadSpecMarkdown() }],
    },
    { title: "All Features", cases: [{ name: "all features", input: loadAllFeatures() }] },
    {
      title: "Document Sizes",
      cases: [1_000, 10_000, 100_000].map((size) => ({
        name: `mixed ${fmtBytes(size)}`,
        input: genMixedDoc(Math.ceil(size / 80)).slice(0, size),
      })),
    },
    {
      title: "Block Types",
      cases: [
        { name: "headings", input: genHeadings() },
        { name: "nested lists", input: genNestedList() },
        { name: "table (100×10)", input: genTable() },
        { name: "code blocks", input: genCodeBlocks() },
      ],
    },
    { title: "Inline-heavy", cases: [{ name: "inline heavy", input: genInlineHeavy() }] },
    {
      title: "Pathological",
      cases: [
        { name: "backticks ×500", input: genPathologicalBackticks() },
        { name: "emphasis ×10k", input: genPathologicalEmphasis() },
        { name: "table 1k rows", input: genTable(1_000, 10) },
        { name: "ref links ×1k", input: genManyRefLinks() },
      ],
    },
  ];
}

function filterSuite(sections) {
  if (!args.suite) return sections;
  const needle = args.suite.toLowerCase();
  return sections
    .map((s) =>
      s.title.toLowerCase().includes(needle)
        ? s
        : { ...s, cases: s.cases.filter((c) => c.name.toLowerCase().includes(needle)) },
    )
    .filter((s) => s.cases.length > 0);
}

if (args.list) {
  console.log("Parsers:");
  for (const [name, p] of Object.entries(PARSERS)) {
    console.log(`  ${name.padEnd(14)} ${p.kind}${name === "bun" ? " (Bun runtime only)" : ""}`);
  }
  console.log("\nSuite:");
  for (const s of buildSuite()) {
    console.log(`  ${s.title}`);
    for (const c of s.cases) console.log(`    ${c.name} (${fmtBytes(Buffer.byteLength(c.input))})`);
  }
  process.exit(0);
}

async function loadParsers() {
  const wanted = args.parsers ? args.parsers.split(",").map((s) => s.trim()) : Object.keys(PARSERS);
  const unknown = wanted.filter((n) => !PARSERS[n]);
  if (unknown.length) {
    console.error(
      `Unknown parser(s): ${unknown.join(", ")}. Known: ${Object.keys(PARSERS).join(", ")}`,
    );
    process.exit(1);
  }
  const loaded = {};
  for (const name of wanted) {
    const fn = await PARSERS[name].load();
    if (fn) loaded[name] = fn;
    else if (args.parsers) log(`  skipping ${name}: not available under ${runtimeLabel}`);
  }
  return loaded;
}

// ─── Run ──────────────────────────────────────────────────────────────────────

const parsers = await loadParsers();
const sampling = args.quick
  ? { maxIterations: 100, minIterations: 10, budgetMs: 100, warmupMs: 30 }
  : {};

log(
  `${runtimeLabel} — ${Object.keys(parsers)
    .map((n) => `${n} (${PARSERS[n].kind})`)
    .join(", ")}`,
);
if (args.quick) log("quick mode: numbers are noisy, don't record them");
log("");

const sections = [];
for (const section of filterSuite(buildSuite())) {
  log(`── ${section.title} ${"─".repeat(Math.max(0, 50 - section.title.length))}`);
  const benches = [];
  for (const { name, input } of section.cases) {
    const bytes = Buffer.byteLength(input, "utf8");
    const results = {};
    // Each parser is measured in isolation so JIT state doesn't bleed across.
    for (const [lib, fn] of Object.entries(parsers)) {
      const samples = collectSamples(fn, input, sampling);
      results[lib] = {
        median_ns: median(samples) * 1e6,
        p95_ns: samples[Math.floor(samples.length * 0.95)] * 1e6,
        samples: samples.length,
      };
    }
    printBench(name, bytes, results);
    benches.push({ name, bytes, results });
  }
  sections.push({ title: section.title, benches });
}

printSummary(sections);

if (args.json) {
  process.stdout.write(JSON.stringify(sections));
} else if (args.save) {
  const { filename } = writeHistoryJson(join(ROOT, "benchmark", "history"), {
    [runtime]: sections,
  });
  log(`\nResults written to benchmark/history/${filename}`);
}

// ─── Output ───────────────────────────────────────────────────────────────────

function printBench(name, bytes, results) {
  const entries = Object.entries(results).sort(([, a], [, b]) => a.median_ns - b.median_ns);
  const fastest = entries[0][1].median_ns;
  log(`  ${name} (${fmtBytes(bytes)})`);
  for (const [lib, r] of entries) {
    const ratio = r.median_ns / fastest;
    let rel = ratio < 1.01 ? "fastest" : `${ratio.toFixed(2)}x slower`;
    if (r.samples === 1) rel += " (single sample: >1 s per call)";
    log(
      `    ${lib.padEnd(14)} median ${fmtNs(r.median_ns).padStart(10)}   p95 ${fmtNs(r.p95_ns).padStart(10)}   ${rel}`,
    );
  }
  log("");
}

// ironmark vs the fastest other parser on each case. Geomean of the per-case
// speedups gives one number that isn't dominated by the largest inputs.
function printSummary(sections) {
  const benches = sections.flatMap((s) => s.benches);
  const ratios = benches
    .map(({ results }) => {
      const im = results.ironmark?.median_ns;
      const rivals = Object.entries(results)
        .filter(([lib]) => lib !== "ironmark")
        .map(([, r]) => r.median_ns);
      return im && rivals.length ? Math.min(...rivals) / im : null;
    })
    .filter((r) => r !== null);
  if (ratios.length === 0) return;
  const wins = ratios.filter((r) => r >= 1).length;
  const geomean = Math.exp(ratios.reduce((s, r) => s + Math.log(r), 0) / ratios.length);
  log("── Summary: ironmark vs fastest other parser ──────");
  log(`  ironmark fastest in ${wins}/${ratios.length} cases`);
  log(
    geomean >= 1
      ? `  geomean: ironmark ${geomean.toFixed(2)}x faster`
      : `  geomean: ironmark ${(1 / geomean).toFixed(2)}x slower`,
  );
}
