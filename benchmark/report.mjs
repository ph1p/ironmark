import { existsSync, readdirSync, readFileSync, rmSync } from "node:fs";
import { createRequire } from "node:module";
import { join } from "node:path";
import { execSync } from "node:child_process";
import {
  ROOT,
  fmtBytes,
  loadSpecMarkdown,
  loadAllFeatures,
  genInlineHeavy,
  genHeadings,
  genNestedList,
  genTable,
  genCodeBlocks,
  genMixedDoc,
  collectSamples,
  median,
  writeHistoryJson,
} from "./generators.mjs";

const require = createRequire(import.meta.url);
const HISTORY_DIR = join(ROOT, "benchmark", "history");

// ─── Build WASM ─────────────────────────────────────────────────────

console.log("Building WASM...\n");
try {
  execSync("pnpm build", {
    cwd: ROOT,
    stdio: "inherit",
  });
} catch {
  console.error("\nWASM build failed — skipping WASM results.\n");
  process.exit(1);
}

// ─── Run Bun benchmarks ─────────────────────────────────────────────────────

console.log("\nRunning Bun benchmarks...\n");
let bunSections = [];
try {
  const bunOutput = execSync("bun benchmark/bun-bench.mjs --json", {
    cwd: ROOT,
    stdio: ["ignore", "pipe", "inherit"],
  });
  bunSections = JSON.parse(bunOutput.toString());
  console.log("Bun benchmarks done.\n");
} catch {
  console.error("Bun benchmarks failed — skipping Bun results.\n");
}

const allFeaturesInput = loadAllFeatures();

// ─── Run WASM benchmarks ────────────────────────────────────────────

const { renderHtml: ironmarkParse } = await import("../wasm/node.js");
const markdownWasm = require("markdown-wasm/dist/markdown.node.js");
const { init: md4wInit, mdToHtml } = await import("md4w");
await md4wInit();

const wasmParsers = {
  ironmark: (input) => ironmarkParse(input),
  "markdown-wasm": (input) => markdownWasm.parse(input),
  md4w: (input) => mdToHtml(input),
};

function runWasmBench(name, input) {
  const results = {};
  const bytes = Buffer.byteLength(input, "utf8");
  for (const [lib, fn] of Object.entries(wasmParsers)) {
    // Measure each parser independently so JIT state doesn't bleed across.
    const samples = collectSamples(fn, input);
    results[lib] = {
      median_ns: median(samples) * 1e6,
      mean_ns: (samples.reduce((a, b) => a + b, 0) / samples.length) * 1e6,
    };
  }
  return { name, bytes, results };
}

console.log("\nRunning WASM benchmarks...\n");

const specMarkdown = loadSpecMarkdown();

const wasmSections = [
  {
    title: "CommonMark Spec",
    benches: [runWasmBench("spec (all examples)", specMarkdown)],
  },
  {
    title: "All Features",
    benches: [runWasmBench("all features", allFeaturesInput)],
  },
  {
    title: "Document Sizes",
    benches: [1_000, 10_000, 100_000].map((size) => {
      const input = genMixedDoc(Math.ceil(size / 80)).slice(0, size);
      return runWasmBench(`mixed ${fmtBytes(size)}`, input);
    }),
  },
  {
    title: "Block Types",
    benches: [
      runWasmBench("headings", genHeadings()),
      runWasmBench("nested lists", genNestedList()),
      runWasmBench("table (100×10)", genTable()),
      runWasmBench("code blocks", genCodeBlocks()),
    ],
  },
  {
    title: "Inline-heavy",
    benches: [runWasmBench("inline heavy", genInlineHeavy())],
  },
];

console.log("WASM benchmarks done.\n");

// ─── Read Rust results from history CSV ─────────────────────────────

function readLatestHistoryCsv() {
  if (!existsSync(HISTORY_DIR)) return [];
  const files = readdirSync(HISTORY_DIR)
    .filter((f) => /^\d{4}-\d{2}-\d{2}\.csv$/.test(f))
    .sort();
  if (files.length === 0) return [];
  const latest = files[files.length - 1];
  const latestPath = join(HISTORY_DIR, latest);
  console.log(`Reading Rust results from benchmark/history/${latest}`);
  const lines = readFileSync(latestPath, "utf8")
    .trim()
    .split("\n")
    .filter((l) => l && !l.startsWith("date,"));
  const groups = new Map();
  for (const line of lines) {
    const [, group, parser, input_bytes, median_ns] = line.split(",");
    if (!group || !parser || !median_ns) continue;
    const bytes = parseInt(input_bytes) || 0;
    if (!groups.has(group)) groups.set(group, { bytes, results: {} });
    groups.get(group).results[parser] = { median_ns: parseFloat(median_ns) };
  }
  rmSync(latestPath);
  console.log(`Deleted temporary ${latest}`);
  return [...groups.entries()].map(([name, data]) => ({
    name,
    bytes: data.bytes,
    results: data.results,
  }));
}

const rustResultsFlat = readLatestHistoryCsv();

if (rustResultsFlat.length === 0) {
  console.log("No Rust history CSV found in benchmark/history/ — run cargo bench first.");
}

// Group flat Rust results into titled sections for the SVG and JSON.
const rustSections = [
  ["commonmark_spec", "CommonMark Spec"],
  ["all_features", "All Features"],
  ["document_size", "Document Sizes"],
  ["block_types", "Block Types"],
  ["inline_heavy", "Inline-heavy"],
].flatMap(([prefix, title]) => {
  const benches = rustResultsFlat
    .filter(
      (r) =>
        r.name === prefix || r.name.startsWith(prefix + "_") || r.name.startsWith(prefix + "/"),
    )
    .map((b) => {
      let name = b.name;
      if (name.startsWith(prefix + "_")) name = name.slice(prefix.length + 1);
      else if (name.startsWith(prefix + "/")) name = name.slice(prefix.length + 1);
      return { ...b, name };
    });
  return benches.length > 0 ? [{ title, benches }] : [];
});

// ─── Persist history JSON ────────────────────────────────────────────

const { filename } = writeHistoryJson(HISTORY_DIR, {
  rust: rustSections,
  wasm: wasmSections,
  bun: bunSections,
});
console.log(`History written to benchmark/history/${filename}`);
