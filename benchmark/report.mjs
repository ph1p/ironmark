import { readFileSync, writeFileSync, existsSync, readdirSync } from "node:fs";
import { createRequire } from "node:module";
import { join, resolve } from "node:path";
import { execSync } from "node:child_process";

const require = createRequire(import.meta.url);
const ROOT = resolve(import.meta.dirname, "..");
const CRITERION_DIR = join(ROOT, "target", "criterion");

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

// ─── Input generators ───────────────────────────────────────────────

function genInlineHeavy(n = 200) {
  return Array.from(
    { length: n },
    (_, i) =>
      `This has **bold**, *italic*, \`code\`, ~~strike~~, [link](http://x.com/${i}), and more.\n\n`,
  ).join("");
}

function genHeadings(n = 200) {
  return Array.from(
    { length: n },
    (_, i) => `# Heading ${i}\n\nSome paragraph text under heading ${i}.\n`,
  ).join("");
}

function genNestedList(depth = 50) {
  return Array.from({ length: depth }, (_, i) => `${"  ".repeat(i)}- item ${i}\n`).join("");
}

function genTable(rows = 100, cols = 10) {
  const header = `|${Array.from({ length: cols }, (_, c) => ` col${c} `).join("|")}|`;
  const sep = `|${Array.from({ length: cols }, () => " --- ").join("|")}|`;
  const body = Array.from(
    { length: rows },
    (_, r) => `|${Array.from({ length: cols }, (_, c) => ` r${r}c${c} `).join("|")}|`,
  ).join("\n");
  return `${header}\n${sep}\n${body}\n`;
}

function genCodeBlocks(n = 100) {
  return Array.from(
    { length: n },
    (_, i) => `\`\`\`rust\nfn func_${i}() {\n    println!("hello");\n}\n\`\`\`\n\n`,
  ).join("");
}

// ─── Step 3: Run WASM benchmarks ────────────────────────────────────

const { parse: ironmarkParse } = await import("../wasm/node.js");
const markdownWasm = require("markdown-wasm/dist/markdown.node.js");
const { init: md4wInit, mdToHtml } = await import("md4w");
await md4wInit();

const wasmParsers = {
  ironmark: (input) => ironmarkParse(input),
  "markdown-wasm": (input) => markdownWasm.parse(input),
  md4w: (input) => mdToHtml(input),
};

function runWasmBench(name, input, iterations = 500) {
  const results = {};
  for (const [lib, fn] of Object.entries(wasmParsers)) {
    for (let i = 0; i < 50; i++) fn(input);
    const times = [];
    for (let i = 0; i < iterations; i++) {
      const start = performance.now();
      fn(input);
      times.push(performance.now() - start);
    }
    times.sort((a, b) => a - b);
    results[lib] = {
      median_ns: times[Math.floor(times.length / 2)] * 1e6,
      mean_ns: (times.reduce((a, b) => a + b, 0) / times.length) * 1e6,
    };
  }
  return { name, bytes: input.length, results };
}

console.log("\nRunning WASM benchmarks...\n");

const specJson = readFileSync(join(ROOT, "tests/spec/spec-0.31.2.json"), "utf8");
const specMarkdown = JSON.parse(specJson)
  .map((t) => t.markdown)
  .join("\n");

const wasmSections = [
  {
    title: "CommonMark Spec",
    benches: [runWasmBench("spec (all examples)", specMarkdown)],
  },
  {
    title: "Document Sizes",
    benches: [1_000, 10_000, 100_000].map((size) => {
      const base = genInlineHeavy();
      const input = base.repeat(Math.ceil(size / base.length)).slice(0, size);
      return runWasmBench(`mixed ${fmtBytes(size)}`, input);
    }),
  },
  {
    title: "Block Types",
    benches: [
      runWasmBench("headings", genHeadings()),
      runWasmBench("nested lists", genNestedList()),
      runWasmBench("table (100x10)", genTable()),
      runWasmBench("code blocks", genCodeBlocks()),
    ],
  },
  {
    title: "Inline-heavy",
    benches: [runWasmBench("inline heavy", genInlineHeavy())],
  },
];

console.log("WASM benchmarks done.\n");

// ─── Step 4: Read Rust criterion results ────────────────────────────

// Criterion layout: target/criterion/{group}/{lib}/{label}/new/estimates.json

const KNOWN_RUST_LIBS = ["ironmark", "pulldown_cmark", "comrak", "markdown_rs"];

function readCriterionResults() {
  if (!existsSync(CRITERION_DIR)) return [];

  const groups = new Map();

  function walkSync(dir) {
    let entries;
    try {
      entries = readdirSync(dir, { withFileTypes: true });
    } catch {
      return;
    }
    for (const e of entries) {
      if (!e.isDirectory() || e.name === "report") continue;
      const full = join(dir, e.name);
      const estPath = join(full, "new", "estimates.json");
      if (existsSync(estPath)) {
        const rel = full.slice(CRITERION_DIR.length + 1);
        const parts = rel.split("/");
        const libIdx = parts.findIndex((p) => KNOWN_RUST_LIBS.includes(p));
        if (libIdx === -1) continue;
        const groupName = parts.slice(0, libIdx).join("/");
        const libName = parts[libIdx];
        const label = parts.slice(libIdx + 1).join("/");
        const bytes = parseInt(label) || 0;

        const est = JSON.parse(readFileSync(estPath, "utf8"));
        if (!groups.has(groupName)) groups.set(groupName, { bytes, results: {} });
        groups.get(groupName).results[libName] = {
          median_ns: est.median.point_estimate,
          mean_ns: est.mean.point_estimate,
        };
      } else {
        walkSync(full);
      }
    }
  }

  walkSync(CRITERION_DIR);

  return [...groups.entries()].map(([name, data]) => ({
    name,
    bytes: data.bytes,
    results: data.results,
  }));
}

const rustResults = readCriterionResults();
const hasRust = rustResults.length > 0;

if (!hasRust) {
  console.log("No Rust criterion results found in target/criterion/.");
}

// Group rust results into sections by prefix matching
const rustSections = [];
if (hasRust) {
  const sectionOrder = [
    ["commonmark_spec", "CommonMark Spec"],
    ["document_size", "Document Sizes"],
    ["block_types", "Block Types"],
    ["inline_heavy", "Inline-heavy"],
  ];

  for (const [prefix, title] of sectionOrder) {
    const matching = rustResults.filter(
      (r) =>
        r.name === prefix || r.name.startsWith(prefix + "_") || r.name.startsWith(prefix + "/"),
    );
    if (matching.length > 0) {
      rustSections.push({
        title,
        benches: matching.map((b) => {
          let name = b.name;
          if (name.startsWith(prefix + "_")) name = name.slice(prefix.length + 1);
          else if (name.startsWith(prefix + "/")) name = name.slice(prefix.length + 1);
          else if (name === prefix) name = prefix;
          return { ...b, name };
        }),
      });
    }
  }
}

// ─── Step 5: Generate HTML report ───────────────────────────────────

function fmtNs(ns) {
  if (ns < 1000) return `${ns.toFixed(0)} ns`;
  if (ns < 1e6) return `${(ns / 1000).toFixed(1)} \u00b5s`;
  return `${(ns / 1e6).toFixed(2)} ms`;
}

function fmtBytes(b) {
  if (b < 1024) return `${b} B`;
  if (b < 1024 * 1024) return `${(b / 1024).toFixed(1)} KB`;
  return `${(b / 1024 / 1024).toFixed(1)} MB`;
}

function throughput(bytes, ns) {
  if (!bytes || !ns) return "-";
  return `${(bytes / (1024 * 1024) / (ns / 1e9)).toFixed(1)} MB/s`;
}

function buildSectionHtml(title, benches, libs, colorMap) {
  let html = `<h2>${title}</h2>`;

  for (const b of benches) {
    const entries = libs
      .map((lib) => (b.results[lib] ? { lib, ...b.results[lib] } : null))
      .filter(Boolean);
    if (entries.length === 0) continue;

    entries.sort((a, b) => a.median_ns - b.median_ns);

    const winner = entries[0];
    const maxThroughput = Math.max(
      ...entries.map((e) => (b.bytes ? b.bytes / e.median_ns : 1 / e.median_ns)),
    );

    const label = b.bytes ? `${b.name} (${fmtBytes(b.bytes)})` : b.name;

    let speedupHtml = "";
    if (entries.length >= 2) {
      const ratio = entries[1].median_ns / winner.median_ns;
      if (ratio > 1.01) {
        speedupHtml = `<span class="speedup">${ratio.toFixed(1)}x faster</span>`;
      } else {
        speedupHtml = `<span class="speedup tied">~tied</span>`;
      }
    }

    html += `<div class="bench-card">`;
    html += `<div class="card-header"><h3>${label}</h3><div class="winner-badge" style="border-color:${colorMap[winner.lib] || "#888"}">${winner.lib} ${speedupHtml}</div></div>`;
    html += `<div class="bars">`;

    for (const e of entries) {
      const tp = b.bytes ? b.bytes / e.median_ns : 1 / e.median_ns;
      const pct = (tp / maxThroughput) * 100;
      const color = colorMap[e.lib] || "#888";
      const isWinner = e === winner;
      html += `
        <div class="bar-row${isWinner ? " bar-winner" : ""}">
          <span class="bar-label">${e.lib}</span>
          <div class="bar-track">
            <div class="bar-fill" style="width:${pct.toFixed(1)}%;background:${color}"></div>
          </div>
          <span class="bar-value">${fmtNs(e.median_ns)}</span>
          <span class="bar-throughput">${throughput(b.bytes, e.median_ns)}</span>
        </div>`;
    }

    html += `</div></div>`;
  }

  return html;
}

const RUST_LIBS = ["ironmark", "pulldown_cmark", "comrak", "markdown_rs"];
const WASM_LIBS = ["ironmark", "markdown-wasm", "md4w"];

const RUST_COLORS = {
  ironmark: "#e8590c",
  pulldown_cmark: "#1971c2",
  comrak: "#2f9e44",
  markdown_rs: "#c2185b",
};

const WASM_COLORS = {
  ironmark: "#e8590c",
  "markdown-wasm": "#7048e8",
  md4w: "#0ea5e9",
};

let rustHtml = "";
if (hasRust) {
  rustHtml = `<h1>Native Rust</h1>
    <p class="subtitle">ironmark vs pulldown-cmark vs comrak vs markdown-rs &mdash; <code>cargo bench</code> (criterion)</p>`;
  for (const section of rustSections) {
    rustHtml += buildSectionHtml(section.title, section.benches, RUST_LIBS, RUST_COLORS);
  }
}

let wasmHtml = `<h1>WASM (Node.js)</h1>
  <p class="subtitle">ironmark vs markdown-wasm vs md4w &mdash; median of 500 iterations</p>`;
for (const section of wasmSections) {
  wasmHtml += buildSectionHtml(section.title, section.benches, WASM_LIBS, WASM_COLORS);
}

const html = `<!DOCTYPE html>
<html lang="en">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<title>ironmark benchmark results</title>
<style>
  *, *::before, *::after { box-sizing: border-box; margin: 0; padding: 0; }
  body {
    font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, Helvetica, Arial, sans-serif;
    background: #0f0f0f; color: #e0e0e0;
    padding: 2rem; max-width: 960px; margin: 0 auto;
    line-height: 1.5;
  }
  header { text-align: center; margin-bottom: 3rem; }
  header h1 { font-size: 2rem; font-weight: 700; color: #fff; letter-spacing: -0.02em; }
  header p { color: #888; margin-top: 0.25rem; }
  h1 {
    font-size: 1.4rem; font-weight: 600; color: #fff;
    border-bottom: 1px solid #2a2a2a; padding-bottom: 0.5rem;
    margin: 2.5rem 0 0.5rem;
  }
  .subtitle { color: #888; font-size: 0.85rem; margin-bottom: 1.5rem; }
  .subtitle code { background: #1a1a1a; padding: 0.1em 0.4em; border-radius: 3px; font-size: 0.8rem; }
  h2 { font-size: 1.1rem; font-weight: 600; color: #ccc; margin: 2rem 0 1rem; }
  .bench-card {
    background: #1a1a1a; border: 1px solid #2a2a2a; border-radius: 8px;
    padding: 1.25rem; margin-bottom: 1rem;
  }
  .card-header { display: flex; align-items: center; justify-content: space-between; margin-bottom: 0.75rem; gap: 0.75rem; }
  .card-header h3 { font-size: 0.9rem; font-weight: 500; color: #aaa; margin: 0; }
  .winner-badge {
    font-size: 0.75rem; font-weight: 600; color: #fff;
    border: 1px solid; border-radius: 4px;
    padding: 0.15em 0.6em; white-space: nowrap;
    display: flex; align-items: center; gap: 0.5em;
  }
  .speedup { font-size: 0.7rem; font-weight: 400; color: #8f8; }
  .speedup.tied { color: #888; }
  .bars { display: flex; flex-direction: column; gap: 0.5rem; }
  .bar-row { display: grid; grid-template-columns: 120px 1fr 90px 80px; align-items: center; gap: 0.75rem; }
  .bar-winner .bar-label { color: #fff; font-weight: 600; }
  .bar-label { font-size: 0.8rem; font-weight: 500; color: #777; text-align: right; }
  .bar-track { background: #252525; border-radius: 4px; height: 22px; overflow: hidden; }
  .bar-fill { height: 100%; border-radius: 4px; transition: width 0.3s ease; min-width: 2px; }
  .bar-value { font-size: 0.8rem; color: #e0e0e0; font-variant-numeric: tabular-nums; text-align: right; }
  .bar-throughput { font-size: 0.75rem; color: #666; font-variant-numeric: tabular-nums; text-align: right; }

  .legend { display: flex; gap: 1.5rem; margin: 1rem 0 0; flex-wrap: wrap; }
  .legend-item { display: flex; align-items: center; gap: 0.4rem; font-size: 0.8rem; color: #aaa; }
  .legend-dot { width: 10px; height: 10px; border-radius: 2px; }

  .note { color: #666; font-size: 0.75rem; text-align: center; margin-top: 3rem; }

  @media (max-width: 640px) {
    body { padding: 1rem; }
    .bar-row { grid-template-columns: 80px 1fr 70px 60px; gap: 0.4rem; }
    .bar-label, .bar-value { font-size: 0.7rem; }
    .bar-throughput { font-size: 0.65rem; }
  }
</style>
</head>
<body>
  <header>
    <h1>ironmark</h1>
    <p>benchmark results</p>
  </header>

  ${
    hasRust
      ? `
  <div class="legend">
    <span class="legend-item"><span class="legend-dot" style="background:#e8590c"></span> ironmark</span>
    <span class="legend-item"><span class="legend-dot" style="background:#1971c2"></span> pulldown-cmark</span>
    <span class="legend-item"><span class="legend-dot" style="background:#2f9e44"></span> comrak</span>
    <span class="legend-item"><span class="legend-dot" style="background:#c2185b"></span> markdown-rs</span>
  </div>
  ${rustHtml}`
      : ""
  }

  <div class="legend"${hasRust ? ' style="margin-top:2.5rem"' : ""}>
    <span class="legend-item"><span class="legend-dot" style="background:#e8590c"></span> ironmark</span>
    <span class="legend-item"><span class="legend-dot" style="background:#7048e8"></span> markdown-wasm</span>
    <span class="legend-item"><span class="legend-dot" style="background:#0ea5e9"></span> md4w</span>
  </div>
  ${wasmHtml}

  <p class="note">
    Bars show throughput (longer is faster). Generated on ${new Date().toISOString().slice(0, 10)}.
  </p>
</body>
</html>`;

const outPath = join(ROOT, "benchmark", "results.html");
writeFileSync(outPath, html);
console.log(`\nReport written to ${outPath}`);
