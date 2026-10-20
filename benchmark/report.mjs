/**
 * Full benchmark report — what `pnpm bench` runs after `pnpm bench:rust`.
 *
 * 1. builds the WASM package
 * 2. runs js-bench.mjs under Node and (if installed) Bun
 * 3. folds in the Rust criterion results from the temporary history CSV
 * 4. writes one timestamped history JSON to benchmark/history/, together with
 *    the machine and toolchain versions the numbers were measured on
 *
 * The playground Benchmarks page is generated from those history files.
 */

import { existsSync, readdirSync, readFileSync, rmSync } from "node:fs";
import { join } from "node:path";
import { execFileSync, execSync } from "node:child_process";
import { ROOT, captureEnv, fmtEnv, rustcVersion, writeHistoryJson } from "./generators.mjs";

const HISTORY_DIR = join(ROOT, "benchmark", "history");

// ─── Build WASM ─────────────────────────────────────────────────────

console.log("Building WASM...\n");
try {
  execSync("pnpm build", { cwd: ROOT, stdio: "inherit" });
} catch {
  console.error("\nWASM build failed.\n");
  process.exit(1);
}

// ─── Run JS benchmarks (Node + Bun) ─────────────────────────────────

function runJsBench(runtime) {
  console.log(`\nRunning JS benchmarks under ${runtime}...\n`);
  try {
    const out = execFileSync(runtime, ["benchmark/js-bench.mjs", "--json"], {
      cwd: ROOT,
      stdio: ["ignore", "pipe", "inherit"],
      maxBuffer: 64 * 1024 * 1024,
    });
    return JSON.parse(out.toString());
  } catch (err) {
    const reason = err.code === "ENOENT" ? `${runtime} not installed` : "run failed";
    console.error(`${runtime} benchmarks skipped (${reason}).\n`);
    return null;
  }
}

const wasmRun = runJsBench("node");
const bunRun = runJsBench("bun");

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
    const [, group, parser, input_bytes, median_ns, p25_ns, p75_ns] = line.split(",");
    if (!group || !parser || !median_ns) continue;
    const bytes = parseInt(input_bytes) || 0;
    if (!groups.has(group)) groups.set(group, { bytes, results: {} });
    const result = { median_ns: parseFloat(median_ns) };
    if (p25_ns && p75_ns) Object.assign(result, { p25_ns: +p25_ns, p75_ns: +p75_ns });
    groups.get(group).results[parser] = result;
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

const env = { ...captureEnv(), bun: bunRun?.env.bun, rustc: rustcVersion() };
const { filename } = writeHistoryJson(HISTORY_DIR, {
  env,
  profile: "gfm",
  rust: rustSections,
  wasm: wasmRun?.sections ?? [],
  bun: bunRun?.sections ?? [],
});
console.log(`Measured on ${fmtEnv(env)}`);
console.log(`History written to benchmark/history/${filename}`);
