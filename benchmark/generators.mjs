/**
 * Shared Markdown input generators for JS/Bun benchmarks.
 * Used by both report.mjs (Node.js WASM) and bun-bench.mjs (Bun native).
 */

import { existsSync, mkdirSync, readdirSync, readFileSync, writeFileSync } from "node:fs";
import { dirname, join, resolve } from "node:path";

export const ROOT = resolve(import.meta.dirname, "..");

export function genInlineHeavy(n = 200) {
  return Array.from(
    { length: n },
    (_, i) =>
      `This has **bold**, *italic*, \`code\`, ~~strike~~, [link](http://x.com/${i}), and more.\n\n`,
  ).join("");
}

export function genHeadings(n = 200) {
  return Array.from(
    { length: n },
    (_, i) => `# Heading ${i}\n\nSome paragraph text under heading ${i}.\n`,
  ).join("");
}

export function genNestedList(depth = 50) {
  return Array.from({ length: depth }, (_, i) => `${"  ".repeat(i)}- item ${i}\n`).join("");
}

export function genTable(rows = 100, cols = 10) {
  const header = `|${Array.from({ length: cols }, (_, c) => ` col${c} `).join("|")}|`;
  const sep = `|${Array.from({ length: cols }, () => " --- ").join("|")}|`;
  const body = Array.from(
    { length: rows },
    (_, r) => `|${Array.from({ length: cols }, (_, c) => ` r${r}c${c} `).join("|")}|`,
  ).join("\n");
  return `${header}\n${sep}\n${body}\n`;
}

export function genCodeBlocks(n = 100) {
  return Array.from(
    { length: n },
    (_, i) => `\`\`\`rust\nfn func_${i}() {\n    println!("hello");\n}\n\`\`\`\n\n`,
  ).join("");
}

/** Mixed document cycling through headings, lists, code, blockquotes, plain text, ordered lists. */
export function genMixedDoc(lines = 2000) {
  const parts = [];
  let line = 0;
  while (line < lines) {
    switch (Math.floor(line / 20) % 6) {
      case 0:
        parts.push(`# Section ${Math.floor(line / 20)}\n\n`);
        line += 2;
        for (let i = 0; i < Math.min(4, lines - line); i++) {
          parts.push(
            `This is paragraph text with **bold**, *italic*, \`code\`, and [a link](http://example.com/${i}).\n`,
          );
          line++;
        }
        parts.push("\n");
        line++;
        break;
      case 1:
        for (let i = 0; i < Math.min(6, lines - line); i++) {
          parts.push(`- List item ${i} with some text\n`);
          line++;
        }
        parts.push("\n");
        line++;
        break;
      case 2:
        parts.push("```rust\n");
        line++;
        for (let i = 0; i < Math.min(5, lines - line); i++) {
          parts.push(`    let x_${i} = compute(${i});\n`);
          line++;
        }
        parts.push("```\n\n");
        line += 2;
        break;
      case 3:
        for (let i = 0; i < Math.min(3, lines - line); i++) {
          parts.push("> Quoted text with *emphasis* and **strong**.\n");
          line++;
        }
        parts.push("\n");
        line++;
        break;
      case 4:
        for (let i = 0; i < Math.min(5, lines - line); i++) {
          parts.push("Plain text without any special formatting or markup characters at all.\n");
          line++;
        }
        parts.push("\n");
        line++;
        break;
      case 5:
        for (let i = 0; i < Math.min(4, lines - line); i++) {
          parts.push(`${i + 1}. Item with \`code\` and **bold**\n`);
          line++;
        }
        parts.push("\n");
        line++;
        break;
    }
  }
  return parts.join("");
}

export function genPathologicalBackticks(n = 500) {
  return Array.from({ length: n }, (_, i) => "`".repeat(i + 1) + " ").join("");
}

export function genPathologicalEmphasis(n = 10_000) {
  return "*a ".repeat(n);
}

export function genManyRefLinks(n = 1_000) {
  const defs = '[refdef]: http://example.com/very/long/url "Title"\n\n';
  return (
    defs + Array.from({ length: n }, () => "See [refdef] and [refdef] and [refdef].\n\n").join("")
  );
}

export function loadSpecMarkdown() {
  const json = readFileSync(join(ROOT, "tests/spec/spec-0.31.2.json"), "utf8");
  return JSON.parse(json)
    .map((t) => t.markdown)
    .join("\n");
}

export function loadAllFeatures() {
  return readFileSync(join(ROOT, "benchmark", "all_features.md"), "utf8");
}

/**
 * Time-budgeted sampling shared by the Bun and Node (WASM) benchmark runners.
 *
 * Batch timing: time `batch` calls in one `performance.now()` pair to avoid
 * per-call measurement overhead dominating sub-100µs inputs. Warmup runs until
 * `warmupMs` elapses (capped at 100 calls) so the JIT stabilises. Sampling is
 * budgeted: small inputs get up to `maxIterations` samples, big/slow inputs
 * stop after `budgetMs` (but never below `minIterations`).
 *
 * Returns per-call millisecond samples, sorted ascending.
 */
export function collectSamples(
  fn,
  input,
  { maxIterations = 500, minIterations = 30, budgetMs = 500, warmupMs = 150, batch = 10 } = {},
) {
  const warmupEnd = performance.now() + warmupMs;
  for (let i = 0; i < 100 && performance.now() < warmupEnd; i++) fn(input);

  const samples = [];
  const budgetEnd = performance.now() + budgetMs;
  while (
    samples.length < maxIterations &&
    (samples.length < minIterations || performance.now() < budgetEnd)
  ) {
    const start = performance.now();
    for (let b = 0; b < batch; b++) fn(input);
    samples.push((performance.now() - start) / batch);
  }
  samples.sort((a, b) => a - b);
  return samples;
}

/** Median of an ascending-sorted sample array. */
export function median(sorted) {
  const mid = Math.floor(sorted.length / 2);
  return sorted.length % 2 === 0 ? (sorted[mid - 1] + sorted[mid]) / 2 : sorted[mid];
}

export function fmtNs(ns) {
  if (ns < 1_000) return `${ns.toFixed(0)} ns`;
  if (ns < 1_000_000) return `${(ns / 1_000).toFixed(1)} µs`;
  return `${(ns / 1_000_000).toFixed(2)} ms`;
}

export function fmtBytes(b) {
  if (b < 1024) return `${b} B`;
  if (b < 1024 * 1024) return `${(b / 1024).toFixed(1)} KB`;
  return `${(b / (1024 * 1024)).toFixed(1)} MB`;
}

export function todayDate() {
  return new Date().toISOString().slice(0, 10);
}

/** Returns a timestamp string like `2026-04-08_14-32-05` for use in filenames. */
export function nowTimestamp() {
  return new Date().toISOString().slice(0, 19).replace("T", "_").replace(/:/g, "-");
}

/**
 * Write a new timestamped history JSON file for this run.
 * Each bench run gets its own file: `YYYY-MM-DD_HH-MM-SS.json`.
 * Returns the record written.
 */
export function writeHistoryJson(historyDir, data) {
  mkdirSync(historyDir, { recursive: true });
  const ts = nowTimestamp();
  const path = join(historyDir, `${ts}.json`);
  const record = { timestamp: ts, date: todayDate(), ...data };
  writeFileSync(path, JSON.stringify(record, null, 2));
  return { record, filename: `${ts}.json` };
}

/**
 * Read all dated history JSONs from benchmark/history/ and write
 * playground/public/benchmark-data.json for the playground Benchmarks page.
 * Pass `latest` to override the latest sections (used by report.mjs after a fresh bench run).
 * When called without `latest`, falls back to the most recent history file's data.
 */
export function writePlaygroundData(latest = null) {
  const historyDir = join(ROOT, "benchmark", "history");
  const allFiles = existsSync(historyDir)
    ? readdirSync(historyDir)
        .filter((f) => /^\d{4}-\d{2}-\d{2}_\d{2}-\d{2}-\d{2}\.json$/.test(f))
        .sort()
    : [];

  // Track the most recent non-null sections for each runtime independently,
  // since bun-bench.mjs and report.mjs write separate files.
  const latestByRuntime = { wasm: null, rust: null, bun: null };

  const trend = allFiles
    .map((f) => {
      try {
        const rec = JSON.parse(readFileSync(join(historyDir, f), "utf8"));
        if (!latest) {
          if (rec.wasm) latestByRuntime.wasm = { sections: rec.wasm };
          if (rec.rust) latestByRuntime.rust = { sections: rec.rust };
          if (rec.bun) latestByRuntime.bun = { sections: rec.bun };
        }
        const wasmBench = rec.wasm
          ?.find((s) => s.title === "CommonMark Spec")
          ?.benches?.find((b) => b.name === "spec (all examples)");
        const bunBench = rec.bun
          ?.find((s) => s.title === "CommonMark Spec")
          ?.benches?.find((b) => b.name === "spec (all examples)");
        const rustBench = rec.rust
          ?.find((s) => s.title === "CommonMark Spec")
          ?.benches?.find((b) => b.name === "commonmark_spec");
        const wasmNs = wasmBench?.results?.ironmark?.median_ns;
        const bunNs = bunBench?.results?.ironmark?.median_ns;
        const rustNs = rustBench?.results?.ironmark?.median_ns;
        if (!wasmNs && !bunNs && !rustNs) return null;
        const point = { timestamp: rec.timestamp ?? f.slice(0, 19) };
        if (wasmNs) point.ironmark_wasm_ns = wasmNs;
        if (bunNs) point.ironmark_bun_ns = bunNs;
        if (rustNs) point.ironmark_rust_ns = rustNs;
        const wasmFastest = fastestCompetitorMedian(wasmBench);
        const bunFastest = fastestCompetitorMedian(bunBench);
        const rustFastest = fastestCompetitorMedian(rustBench);
        if (wasmNs && wasmFastest) point.ironmark_wasm_ratio = wasmNs / wasmFastest;
        if (bunNs && bunFastest) point.ironmark_bun_ratio = bunNs / bunFastest;
        if (rustNs && rustFastest) point.ironmark_rust_ratio = rustNs / rustFastest;
        return point;
      } catch {
        return null;
      }
    })
    .filter(Boolean);

  const data = {
    generatedAt: new Date().toISOString(),
    latest: latest ?? latestByRuntime,
    trend,
  };

  const outPath = join(ROOT, "playground", "public", "benchmark-data.json");
  mkdirSync(dirname(outPath), { recursive: true });
  writeFileSync(outPath, JSON.stringify(data, null, 2));
  console.log(`Playground data written to playground/public/benchmark-data.json`);
}

// Fastest median among the OTHER parsers. Including ironmark itself would
// clamp the ratio to 1.00 whenever ironmark wins, flattening the trend into
// a straight line — the ratio should track the margin vs the best rival
// (<1.00 = ironmark faster).
function fastestCompetitorMedian(bench) {
  if (!bench?.results) return undefined;
  const values = Object.entries(bench.results)
    .filter(([lib]) => lib !== "ironmark")
    .map(([, r]) => r?.median_ns)
    .filter((v) => typeof v === "number" && v > 0);
  return values.length > 0 ? Math.min(...values) : undefined;
}
