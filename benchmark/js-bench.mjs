/**
 * JS-side benchmark: ironmark WASM vs other JS-callable Markdown parsers.
 * Runs under Node or Bun — whichever runtime executes this file is the one
 * being measured. Under Bun, Bun's native `Bun.markdown.html` joins in.
 *
 *   node benchmark/js-bench.mjs [options]
 *   bun  benchmark/js-bench.mjs [options]
 *
 * Options:
 *   --parsers a,b       only run these parsers (default: all available)
 *   --suite <text>      only run sections/cases whose name contains <text>
 *   --file <path>       benchmark a single Markdown file instead of the suite
 *   --profile <name>    feature set every parser is configured with:
 *                       gfm (default) or commonmark
 *   --check             don't time anything: report CommonMark spec
 *                       conformance and where parser outputs differ
 *   --quick             short sampling budget (noisy — for dev loops only)
 *   --save              write a timestamped history JSON to benchmark/history/
 *   --json              print results as JSON on stdout (logs go to stderr)
 *   --list              list parsers and suite cases, then exit
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
  fmtEnv,
  loadSpecExamples,
  loadSpecMarkdown,
  loadAllFeatures,
  genInlineHeavy,
  genHeadings,
  genNestedList,
  genTable,
  genCodeBlocks,
  genMixedDocOfSize,
  genPathologicalBackticks,
  genPathologicalEmphasis,
  genManyRefLinks,
  collectSamples,
  captureEnv,
  median,
  quantile,
  throughputMBs,
  relSpread,
  withinNoise,
  normalizeSpecHtml,
  stripHeadingAnchors,
  normalizeForCompare,
  writeHistoryJson,
} from "./generators.mjs";

const require = createRequire(import.meta.url);
const isBun = typeof Bun !== "undefined";
const runtime = isBun ? "bun" : "wasm"; // history JSON key; "wasm" = Node runtime
const runtimeLabel = isBun ? `Bun ${Bun.version}` : `Node ${process.versions.node}`;

// ─── CLI ──────────────────────────────────────────────────────────────────────

const PROFILES = {
  gfm: "CommonMark + GFM tables, strikethrough, task lists, autolink literals",
  commonmark: "CommonMark only, all extensions off",
};

const { values: args } = parseArgs({
  options: {
    parsers: { type: "string" },
    suite: { type: "string" },
    file: { type: "string" },
    profile: { type: "string", default: "gfm" },
    check: { type: "boolean", default: false },
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

if (!PROFILES[args.profile]) {
  console.error(`Unknown profile "${args.profile}". Known: ${Object.keys(PROFILES).join(", ")}`);
  process.exit(1);
}

if (
  args.save &&
  (args.file || args.suite || args.parsers || args.quick || args.check || args.profile !== "gfm")
) {
  console.error(
    "--save records the full default run only; drop --file/--suite/--parsers/--quick/--check/--profile.",
  );
  process.exit(1);
}

const log = args.json ? (...a) => process.stderr.write(a.join(" ") + "\n") : console.log;

// ─── Parsers ──────────────────────────────────────────────────────────────────

// Every parser gets the same feature set (see PROFILES) so none of them wins by
// doing less work. Hard breaks, highlight (==), underline (++) and heading ids
// stay off: not every parser supports them.
//
// kind: wasm | js | native — shown in output so results aren't misread.
// load(profile) returns the render function, or null when unavailable.
const MD4C_GFM = [
  "TABLES",
  "STRIKETHROUGH",
  "TASKLISTS",
  "PERMISSIVE_URL_AUTO_LINKS",
  "PERMISSIVE_EMAIL_AUTO_LINKS",
  "PERMISSIVE_WWW_AUTO_LINKS",
];
// markdown-wasm takes the same md4c flags, as numbers.
const MD4C_FLAG_BITS = {
  TABLES: 0x0100,
  STRIKETHROUGH: 0x0200,
  TASKLISTS: 0x0800,
  PERMISSIVE_URL_AUTO_LINKS: 0x0004,
  PERMISSIVE_EMAIL_AUTO_LINKS: 0x0008,
  PERMISSIVE_WWW_AUTO_LINKS: 0x0400,
};

const PARSERS = {
  ironmark: {
    kind: "wasm",
    async load(profile) {
      if (!existsSync(join(ROOT, "wasm", "node.js"))) {
        console.error("ironmark WASM build missing — run `pnpm build` first.");
        process.exit(1);
      }
      const { renderHtml } = await import("../wasm/node.js");
      const gfm = profile === "gfm";
      const options = {
        hardBreaks: false,
        enableHighlight: false,
        enableUnderline: false,
        enableStrikethrough: gfm,
        enableTables: gfm,
        enableTaskLists: gfm,
        enableAutolink: gfm,
      };
      return (input) => renderHtml(input, options);
    },
  },
  "markdown-wasm": {
    kind: "wasm",
    async load(profile) {
      const markdownWasm = require("markdown-wasm/dist/markdown.node.js");
      const parseFlags =
        profile === "gfm" ? MD4C_GFM.reduce((f, name) => f | MD4C_FLAG_BITS[name], 0) : 0;
      return (input) => markdownWasm.parse(input, { parseFlags });
    },
  },
  md4w: {
    kind: "wasm",
    async load(profile) {
      const { init, mdToHtml } = await import("md4w");
      await init();
      const options = { parseFlags: profile === "gfm" ? MD4C_GFM : 0 };
      return (input) => mdToHtml(input, options);
    },
  },
  marked: {
    kind: "js",
    async load(profile) {
      const { Marked } = await import("marked");
      const marked = new Marked({ gfm: profile === "gfm", breaks: false });
      return (input) => marked.parse(input);
    },
  },
  bun: {
    kind: "native",
    async load(profile) {
      if (!isBun || !Bun.markdown?.html) return null;
      const gfm = profile === "gfm";
      const options = { tables: gfm, strikethrough: gfm, tasklists: gfm, autolinks: gfm };
      return (input) => Bun.markdown.html(input, options);
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
        input: genMixedDocOfSize(size),
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
  console.log("\nProfiles:");
  for (const [name, desc] of Object.entries(PROFILES)) console.log(`  ${name.padEnd(14)} ${desc}`);
  console.log("\nSuite:");
  for (const s of buildSuite()) {
    console.log(`  ${s.title}`);
    for (const c of s.cases) console.log(`    ${c.name} (${fmtBytes(Buffer.byteLength(c.input))})`);
  }
  process.exit(0);
}

async function loadParsers(profile) {
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
    const fn = await PARSERS[name].load(profile);
    if (fn) loaded[name] = fn;
    else if (args.parsers) log(`  skipping ${name}: not available under ${runtimeLabel}`);
  }
  return loaded;
}

// ─── Output agreement ─────────────────────────────────────────────────────────

/**
 * Groups parsers by normalised output. Returns { groups, majority } where
 * majority is the set of parsers in the single largest group (≥2 members),
 * or null when there is no clear majority.
 */
function compareOutputs(outputs) {
  const byHtml = new Map();
  for (const [lib, html] of Object.entries(outputs)) {
    const key = normalizeForCompare(html);
    if (!byHtml.has(key)) byHtml.set(key, []);
    byHtml.get(key).push(lib);
  }
  const groups = [...byHtml.entries()]
    .map(([html, libs]) => ({ html, libs }))
    .sort((a, b) => b.libs.length - a.libs.length);
  const clear = groups[0].libs.length >= 2 && groups[0].libs.length > (groups[1]?.libs.length ?? 0);
  return { groups, majority: clear ? new Set(groups[0].libs) : null };
}

/** Short excerpt of where `a` and `b` first differ. */
function firstDiff(a, b, context = 40) {
  let i = 0;
  while (i < a.length && i < b.length && a[i] === b[i]) i++;
  const from = Math.max(0, i - context / 2);
  const cut = (s) => JSON.stringify(s.slice(from, from + context));
  return { a: cut(a), b: cut(b) };
}

// ─── Check mode ───────────────────────────────────────────────────────────────

if (args.check) {
  await runCheck();
  process.exit(0);
}

async function runCheck() {
  // 1. Spec conformance — always with extensions off, as the spec intends.
  const cm = await loadParsers("commonmark");
  const examples = loadSpecExamples();
  log(`${runtimeLabel} — CommonMark 0.31.2 spec conformance (${examples.length} examples)`);
  for (const [lib, fn] of Object.entries(cm)) {
    let pass = 0;
    for (const ex of examples) {
      let html;
      try {
        html = fn(ex.markdown);
      } catch {
        continue;
      }
      if (normalizeSpecHtml(stripHeadingAnchors(html)) === normalizeSpecHtml(ex.html)) pass++;
    }
    const pct = ((pass / examples.length) * 100).toFixed(1);
    log(`  ${lib.padEnd(14)} ${String(pass).padStart(3)}/${examples.length}  ${pct.padStart(5)} %`);
  }

  // 2. Do parsers produce the same document for each suite case?
  const parsers = await loadParsers(args.profile);
  log(`\nOutput agreement, profile "${args.profile}" (markup flavour normalised away)`);
  for (const section of filterSuite(buildSuite())) {
    for (const { name, input } of section.cases) {
      const outputs = Object.fromEntries(
        Object.entries(parsers).map(([lib, fn]) => [lib, fn(input)]),
      );
      const { groups } = compareOutputs(outputs);
      if (groups.length === 1) {
        log(`  ✓ ${name}: all ${groups[0].libs.length} agree`);
        continue;
      }
      log(`  ≠ ${name}: ${groups.map((g) => g.libs.join(" = ")).join("  |  ")}`);
      for (const g of groups.slice(1)) {
        const d = firstDiff(groups[0].html, g.html);
        log(`      ${groups[0].libs[0].padEnd(14)} ${d.a}`);
        log(`      ${g.libs[0].padEnd(14)} ${d.b}`);
      }
    }
  }
}

// ─── Run ──────────────────────────────────────────────────────────────────────

const parsers = await loadParsers(args.profile);
const sampling = args.quick
  ? { maxIterations: 100, minIterations: 10, budgetMs: 100, warmupMs: 30 }
  : {};
const env = captureEnv();

log(
  `${runtimeLabel} — ${Object.keys(parsers)
    .map((n) => `${n} (${PARSERS[n].kind})`)
    .join(", ")}`,
);
log(`profile: ${args.profile} (${PROFILES[args.profile]})`);
log(`machine: ${fmtEnv(env)}`);
if (args.quick) log("quick mode: numbers are noisy, don't record them");
log("");

const sections = [];
for (const section of filterSuite(buildSuite())) {
  log(`── ${section.title} ${"─".repeat(Math.max(0, 50 - section.title.length))}`);
  const benches = [];
  for (const { name, input } of section.cases) {
    const bytes = Buffer.byteLength(input, "utf8");
    const outputs = {};
    const results = {};
    // Each parser is measured in isolation so JIT state doesn't bleed across.
    for (const [lib, fn] of Object.entries(parsers)) {
      outputs[lib] = fn(input);
      const samples = collectSamples(fn, input, sampling);
      results[lib] = {
        median_ns: median(samples) * 1e6,
        p25_ns: quantile(samples, 0.25) * 1e6,
        p75_ns: quantile(samples, 0.75) * 1e6,
        p95_ns: quantile(samples, 0.95) * 1e6,
        samples: samples.length,
      };
    }
    const { majority } = compareOutputs(outputs);
    if (majority) {
      for (const lib of Object.keys(results)) results[lib].matches_majority = majority.has(lib);
    }
    printBench(name, bytes, results);
    benches.push({ name, bytes, results });
  }
  sections.push({ title: section.title, benches });
}

printSummary(sections);

if (args.json) {
  process.stdout.write(JSON.stringify({ env, profile: args.profile, sections }));
} else if (args.save) {
  const { filename } = writeHistoryJson(join(ROOT, "benchmark", "history"), {
    env,
    profile: args.profile,
    [runtime]: sections,
  });
  log(`\nResults written to benchmark/history/${filename}`);
}

// ─── Output ───────────────────────────────────────────────────────────────────

function printBench(name, bytes, results) {
  const entries = Object.entries(results).sort(([, a], [, b]) => a.median_ns - b.median_ns);
  const [, fastest] = entries[0];
  log(`  ${name} (${fmtBytes(bytes)})`);
  for (const [lib, r] of entries) {
    const ratio = r.median_ns / fastest.median_ns;
    let rel;
    if (r === fastest) rel = "fastest";
    else if (withinNoise(r, fastest)) rel = `≈ fastest (${ratio.toFixed(2)}x, within noise)`;
    else rel = `${ratio.toFixed(2)}x slower`;
    if (r.samples === 1) rel += " (single sample: >1 s per call)";
    if (r.matches_majority === false) rel += "  ≠ output differs";
    const spread = relSpread(r);
    const pm = spread == null ? "" : `±${(spread * 100).toFixed(1)}%`;
    const mbs = `${throughputMBs(bytes, r.median_ns).toFixed(1)} MB/s`;
    log(
      `    ${lib.padEnd(14)} ${fmtNs(r.median_ns).padStart(10)} ${pm.padStart(7)}  ${mbs.padStart(12)}   ${rel}`,
    );
  }
  log("");
}

// ironmark vs the fastest other parser on each case. A case where the two
// interquartile ranges overlap counts as a tie, not a win or a loss. Geomean
// of the per-case speedups gives one number that isn't dominated by the
// largest inputs.
function printSummary(sections) {
  const cases = sections
    .flatMap((s) => s.benches)
    .map(({ results }) => {
      const im = results.ironmark;
      const rivals = Object.entries(results).filter(([lib]) => lib !== "ironmark");
      if (!im || rivals.length === 0) return null;
      const [, best] = rivals.reduce((a, b) => (b[1].median_ns < a[1].median_ns ? b : a));
      return { ratio: best.median_ns / im.median_ns, tie: withinNoise(im, best) };
    })
    .filter(Boolean);
  if (cases.length === 0) return;
  const wins = cases.filter((c) => !c.tie && c.ratio > 1).length;
  const ties = cases.filter((c) => c.tie).length;
  const losses = cases.length - wins - ties;
  const geomean = Math.exp(cases.reduce((s, c) => s + Math.log(c.ratio), 0) / cases.length);
  log("── Summary: ironmark vs fastest other parser ──────");
  log(`  ${cases.length} cases: ${wins} faster, ${ties} within noise, ${losses} slower`);
  log(
    geomean >= 1
      ? `  geomean: ironmark ${geomean.toFixed(2)}x faster`
      : `  geomean: ironmark ${(1 / geomean).toFixed(2)}x slower`,
  );
}
