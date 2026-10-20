/**
 * Shared Markdown input generators, sampling and history helpers for the JS
 * benchmarks (js-bench.mjs, report.mjs) and the playground build.
 */

import { execFileSync } from "node:child_process";
import { existsSync, mkdirSync, readdirSync, readFileSync, writeFileSync } from "node:fs";
import { cpus, platform, release, totalmem } from "node:os";
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

/** Mixed document of exactly `size` bytes (ASCII, so chars = bytes). */
export function genMixedDocOfSize(size) {
  let lines = Math.ceil(size / 30);
  let doc = genMixedDoc(lines);
  while (doc.length < size) doc = genMixedDoc((lines *= 2));
  return doc.slice(0, size);
}

export function loadSpecExamples() {
  return JSON.parse(readFileSync(join(ROOT, "tests/spec/spec-0.31.2.json"), "utf8"));
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
 * Batch timing: time up to `batch` calls in one `performance.now()` pair to
 * avoid per-call measurement overhead dominating sub-100µs inputs; inputs that
 * already take ≥1 ms per call are timed one call at a time. Warmup runs until
 * `warmupMs` elapses (capped at 100 calls) so the JIT stabilises. Sampling is
 * budgeted: small inputs get up to `maxIterations` samples, big/slow inputs
 * stop after `budgetMs` (but never below `minIterations`, scaled down to 5
 * for calls slower than ~`budgetMs / 8`). A parser that needs
 * more than `slowMs` for one call (quadratic blow-ups) gets a single sample
 * instead of holding the run hostage.
 *
 * Returns per-call millisecond samples, sorted ascending.
 */
export function collectSamples(
  fn,
  input,
  {
    maxIterations = 500,
    minIterations = 30,
    budgetMs = 500,
    warmupMs = 150,
    batch = 10,
    slowMs = 1000,
  } = {},
) {
  // The last warmup call doubles as a probe for picking the batch size.
  let probeMs = 0;
  const warmupEnd = performance.now() + warmupMs;
  for (let i = 0; i < 100 && performance.now() < warmupEnd; i++) {
    const t = performance.now();
    fn(input);
    probeMs = performance.now() - t;
  }
  if (probeMs > slowMs) return [probeMs];
  const perSample = Math.max(1, Math.min(batch, Math.ceil(1 / Math.max(probeMs, 1e-3))));
  const minSamples = Math.min(minIterations, Math.max(5, Math.ceil((4 * budgetMs) / probeMs)));

  const samples = [];
  const budgetEnd = performance.now() + budgetMs;
  while (
    samples.length < maxIterations &&
    (samples.length < minSamples || performance.now() < budgetEnd)
  ) {
    const start = performance.now();
    for (let b = 0; b < perSample; b++) fn(input);
    samples.push((performance.now() - start) / perSample);
  }
  samples.sort((a, b) => a - b);
  return samples;
}

/** Median of an ascending-sorted sample array. */
export function median(sorted) {
  const mid = Math.floor(sorted.length / 2);
  return sorted.length % 2 === 0 ? (sorted[mid - 1] + sorted[mid]) / 2 : sorted[mid];
}

/** Linear-interpolated quantile (0..1) of an ascending-sorted sample array. */
export function quantile(sorted, q) {
  const pos = (sorted.length - 1) * q;
  const lo = Math.floor(pos);
  const hi = Math.min(lo + 1, sorted.length - 1);
  return sorted[lo] + (sorted[hi] - sorted[lo]) * (pos - lo);
}

/** MB/s for `bytes` parsed in `ns` nanoseconds. */
export function throughputMBs(bytes, ns) {
  return (bytes / ns) * 1e3;
}

/**
 * Half the interquartile range relative to the median, e.g. 0.03 = ±3 %.
 * Missing quartiles (older history, single samples) give null.
 */
export function relSpread(r) {
  if (r.p25_ns == null || r.p75_ns == null || !r.median_ns) return null;
  return (r.p75_ns - r.p25_ns) / 2 / r.median_ns;
}

/** True when two results' interquartile ranges overlap — too close to call. */
export function withinNoise(a, b) {
  if (a.p25_ns == null || b.p25_ns == null) return false;
  return a.p25_ns <= b.p75_ns && b.p25_ns <= a.p75_ns;
}

/**
 * Machine and toolchain the numbers were measured on. Benchmarks are only
 * comparable within one machine — store this with every snapshot.
 */
export function captureEnv() {
  const cpu = cpus();
  const env = {
    cpu: cpu[0]?.model.trim() ?? "unknown",
    cores: cpu.length,
    memory_gb: Math.round(totalmem() / 2 ** 30),
    os: `${platform()} ${release()}`,
    node: process.versions.node,
  };
  if (typeof Bun !== "undefined") env.bun = Bun.version;
  return env;
}

/** `rustc --version` without the "rustc " prefix, or undefined. */
export function rustcVersion() {
  try {
    return execFileSync("rustc", ["--version"], { encoding: "utf8" })
      .trim()
      .replace(/^rustc /, "");
  } catch {
    return undefined;
  }
}

export function fmtEnv(env) {
  if (!env) return "";
  return [
    env.cpu,
    `${env.cores} cores`,
    env.memory_gb && `${env.memory_gb} GB`,
    env.os,
    env.node && `Node ${env.node}`,
    env.bun && `Bun ${env.bun}`,
    env.rustc && `rustc ${env.rustc}`,
  ]
    .filter(Boolean)
    .join(" · ");
}

// ─── Output equivalence ─────────────────────────────────────────────

/**
 * Light normalisation, close to the CommonMark spec runner's: whitespace
 * between tags, self-closing void tags and attribute order don't count.
 */
export function normalizeSpecHtml(html) {
  return html
    .replace(/<([a-z][a-z0-9]*)((?:\s+[^\s=>]+(?:="[^"]*")?)*)\s*\/?>/gi, (_, tag, attrs) => {
      const sorted = (attrs.match(/[^\s=>]+(?:="[^"]*")?/g) ?? []).sort();
      return `<${tag.toLowerCase()}${sorted.map((a) => " " + a).join("")}>`;
    })
    .replace(/\s+/g, " ")
    .replace(/\s*(<\/?[a-z][^>]*>)\s*/gi, "$1")
    .trim();
}

/**
 * markdown-wasm and md4w always emit an empty `<a class="anchor">` in every
 * heading; there is no flag to turn it off. It isn't part of the parse result.
 */
export function stripHeadingAnchors(html) {
  return html.replace(/\s*<a\b[^>]*\bclass="anchor"[^>]*><\/a>/gi, "");
}

const KEEP_ATTRS = new Set(["href", "src", "alt", "title", "start"]);

/**
 * Aggressive normalisation for comparing parsers with each other: also
 * ignores markup flavour that differs without changing the document —
 * heading anchors, `<s>`/`<del>`, `<b>`/`<strong>`, task-list checkbox
 * markup, classes, table alignment style, and quote/`>` escaping in text.
 */
export function normalizeForCompare(html) {
  return normalizeSpecHtml(
    stripHeadingAnchors(html)
      .replace(/<input\b[^>]*>/gi, "")
      .replace(/<(\/?)(?:s|strike)>/gi, "<$1del>")
      .replace(/<(\/?)b>/gi, "<$1strong>")
      .replace(/<(\/?)i>/gi, "<$1em>")
      .replace(/<([a-z][a-z0-9]*)(\s[^>]*?)?(\/?)>/gi, (_, tag, attrs = "", slash) => {
        const kept = (attrs.match(/[^\s=]+="[^"]*"/g) ?? []).filter((a) =>
          KEEP_ATTRS.has(a.slice(0, a.indexOf("=")).toLowerCase()),
        );
        return `<${tag}${kept.map((a) => " " + a).join("")}${slash}>`;
      })
      .replace(/&quot;|&#0*34;|&#x0*22;/gi, '"')
      .replace(/&#0*39;|&#x0*27;|&apos;/gi, "'")
      .replace(/&gt;/g, ">"),
  );
}

export function fmtNs(ns) {
  if (ns < 1_000) return `${ns.toFixed(0)} ns`;
  if (ns < 1_000_000) return `${(ns / 1_000).toFixed(1)} µs`;
  if (ns < 1_000_000_000) return `${(ns / 1_000_000).toFixed(2)} ms`;
  return `${(ns / 1_000_000_000).toFixed(2)} s`;
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
  // since a standalone `js-bench.mjs --save` writes only its own runtime.
  const latestByRuntime = { wasm: null, rust: null, bun: null };
  let env = null;
  let profile = null;

  const trend = allFiles
    .map((f) => {
      try {
        const rec = JSON.parse(readFileSync(join(historyDir, f), "utf8"));
        if (rec.env) env = rec.env;
        if (rec.profile) profile = rec.profile;
        if (!latest) {
          if (rec.wasm) latestByRuntime.wasm = { sections: rec.wasm };
          if (rec.rust) latestByRuntime.rust = { sections: rec.rust };
          if (rec.bun) latestByRuntime.bun = { sections: rec.bun };
        }
        const cases = {};
        for (const runtime of ["wasm", "bun", "rust"]) {
          const byCase = trendCases(rec[runtime]);
          if (byCase) cases[runtime] = byCase;
        }
        if (Object.keys(cases).length === 0) return null;
        const point = { timestamp: rec.timestamp ?? f.slice(0, 19), cases };
        if (rec.profile) point.profile = rec.profile;
        if (rec.env?.cpu) point.machine = rec.env.cpu;
        return point;
      } catch {
        return null;
      }
    })
    .filter(Boolean);

  const data = {
    generatedAt: new Date().toISOString(),
    env,
    profile,
    latest: latest ?? latestByRuntime,
    trend,
  };

  const outPath = join(ROOT, "playground", "public", "benchmark-data.json");
  mkdirSync(dirname(outPath), { recursive: true });
  writeFileSync(outPath, JSON.stringify(data, null, 2));
  console.log(`Playground data written to playground/public/benchmark-data.json`);
}

/**
 * The cases the playground trend tracks, shared by every runtime. Keys are
 * stable across the Rust (criterion group) and JS names and across snapshots.
 */
export const TREND_CASES = [
  "spec",
  "all-features",
  "mixed-1k",
  "mixed-10k",
  "mixed-100k",
  "headings",
  "nested-lists",
  "table",
  "code-blocks",
  "inline-heavy",
];

function trendCaseKey(name) {
  const n = name.toLowerCase().replace(/_/g, " ");
  if (n === "commonmark spec" || n.startsWith("spec")) return "spec";
  const mixed = n.match(/^(?:mixed )?([\d.]+) ?(b|bytes|kb)$/);
  if (mixed) {
    const bytes = Number(mixed[1]) * (mixed[2] === "kb" ? 1024 : 1);
    // Nearest of 1k / 10k / 100k; the Rust-only 1 MB case is not tracked.
    const exp = Math.round(Math.log10(bytes));
    return exp >= 3 && exp <= 5 ? `mixed-${10 ** (exp - 3)}k` : null;
  }
  const key = n.replace(/ \(.*\)$/, "").replace(/ /g, "-");
  return TREND_CASES.includes(key) ? key : null;
}

/** `{ caseKey: { ns, ratio, rival } }` for ironmark in one runtime's sections. */
function trendCases(sections) {
  if (!sections) return null;
  const out = {};
  for (const bench of sections.flatMap((s) => s.benches ?? [])) {
    const key = trendCaseKey(bench.name);
    const ns = bench.results?.ironmark?.median_ns;
    if (!key || !(ns > 0)) continue;
    const entry = { ns };
    // Fastest OTHER parser: including ironmark would clamp the ratio to 1.00
    // whenever it wins and flatten the trend (<1.00 = ironmark faster).
    let rival = null;
    for (const [lib, r] of Object.entries(bench.results)) {
      if (lib !== "ironmark" && r?.median_ns > 0 && (!rival || r.median_ns < rival[1])) {
        rival = [lib, r.median_ns];
      }
    }
    if (rival) {
      entry.ratio = ns / rival[1];
      entry.rival = rival[0];
    }
    out[key] = entry;
  }
  return Object.keys(out).length ? out : null;
}
