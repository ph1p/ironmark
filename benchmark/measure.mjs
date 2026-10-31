/** Artifact/package sizes and truly fresh-process startup. No implicit build or history writes. */
import { spawnSync } from "node:child_process";
import { mkdirSync, mkdtempSync, readFileSync, rmSync, statSync } from "node:fs";
import { basename, join, resolve } from "node:path";
import { pathToFileURL } from "node:url";
import { brotliCompressSync, constants, gzipSync } from "node:zlib";
import { ROOT, captureEnv, median, quantile } from "./generators.mjs";

export function summarize(samples) {
  if (
    !Array.isArray(samples) ||
    samples.length === 0 ||
    samples.some((n) => !Number.isFinite(n) || n < 0)
  ) {
    throw new Error("Expected nonempty finite nonnegative samples");
  }
  const sorted = [...samples].sort((a, b) => a - b);
  return {
    samples: samples.length,
    median_ms: median(sorted),
    p25_ms: quantile(sorted, 0.25),
    p75_ms: quantile(sorted, 0.75),
  };
}

export function artifactSizes(path) {
  let bytes;
  try {
    bytes = readFileSync(path);
  } catch (cause) {
    throw new Error(`Missing or unreadable artifact: ${path}; run pnpm build first`, { cause });
  }
  return {
    path: resolve(path),
    raw_bytes: bytes.length,
    gzip_bytes: gzipSync(bytes, { level: 9 }).length,
    brotli_bytes: brotliCompressSync(bytes, {
      params: { [constants.BROTLI_PARAM_QUALITY]: 11 },
    }).length,
    compression: { gzip_level: 9, brotli_quality: 11 },
  };
}

export function parseJson(text, label) {
  try {
    return JSON.parse(text);
  } catch (cause) {
    throw new Error(`Malformed ${label} JSON`, { cause });
  }
}

function nonnegative(n) {
  return Number.isFinite(n) && n >= 0;
}

export function parsePack(text) {
  const data = parseJson(text, "npm pack");
  const pack = Array.isArray(data) && data.length === 1 ? data[0] : null;
  if (
    !pack ||
    typeof pack.filename !== "string" ||
    pack.filename === "" ||
    basename(pack.filename) !== pack.filename ||
    /[/\\]/.test(pack.filename) ||
    !pack.filename.endsWith(".tgz") ||
    !Array.isArray(pack.files) ||
    pack.files.length === 0 ||
    pack.files.some((f) => !f || !Number.isSafeInteger(f.size) || f.size < 0)
  ) {
    throw new Error("Malformed npm pack metadata");
  }
  return pack;
}

export function runCommand(command, args, options = {}) {
  const result = spawnSync(command, args, {
    encoding: "utf8",
    timeout: 120_000,
    maxBuffer: 16 * 1024 * 1024,
    ...options,
  });
  if (result.error || result.status !== 0) {
    throw new Error(`${command} failed: ${result.error?.message ?? result.stderr}`);
  }
  return result.stdout;
}

export function packageSizes(root = ROOT, run = runCommand) {
  const target = join(root, "target");
  mkdirSync(target, { recursive: true });
  const temporary = mkdtempSync(join(target, "measure-pack-"));
  const options = {
    cwd: root,
    env: { ...process.env, npm_config_cache: join(target, "npm-cache") },
  };
  try {
    const dry = parsePack(run("npm", ["pack", "--dry-run", "--ignore-scripts", "--json"], options));
    const packed = parsePack(
      run("npm", ["pack", "--ignore-scripts", "--json", "--pack-destination", temporary], options),
    );
    if (
      dry.filename !== packed.filename ||
      JSON.stringify(dry.files) !== JSON.stringify(packed.files)
    ) {
      throw new Error("npm package contents changed between dry-run and pack");
    }
    return {
      filename: packed.filename,
      tarball_bytes: statSync(join(temporary, packed.filename)).size,
      unpacked_bytes: dry.files.reduce((sum, file) => sum + file.size, 0),
      files: dry.files.length,
    };
  } finally {
    rmSync(temporary, { recursive: true, force: true }); // Only our own mkdtemp directory.
  }
}

const CHILD = `
const started = performance.now();
const api = await import(process.argv[1]);
const imported = performance.now();
if (typeof api.renderHtml !== "function") throw new Error("Entry must export renderHtml");
const html = api.renderHtml("Hello, **world**.\\n");
const rendered = performance.now();
if (typeof html !== "string" || html.length === 0) throw new Error("Empty/invalid render");
console.log(JSON.stringify({import_ms: imported - started, first_render_ms: rendered - imported}));
`;

export function parseChild(text) {
  const result = parseJson(text, "startup child");
  if (!result || !nonnegative(result.import_ms) || !nonnegative(result.first_render_ms)) {
    throw new Error("Malformed startup child metrics");
  }
  return { import_ms: result.import_ms, first_render_ms: result.first_render_ms };
}

export function entryUrl(entry) {
  const path = resolve(entry);
  if (!statSync(path, { throwIfNoEntry: false })?.isFile()) {
    throw new Error(`Missing entry: ${path}; run pnpm build first`);
  }
  return pathToFileURL(path).href;
}

export function startup(entry, count = 20, run = runCommand) {
  if (!Number.isInteger(count) || count < 20)
    throw new Error("At least 20 fresh processes required");
  const url = entryUrl(entry);
  const samples = [];
  for (let i = 0; i < count; i++) {
    const start = performance.now();
    const result = parseChild(
      run(process.execPath, ["--input-type=module", "-e", CHILD, url], { timeout: 30_000 }),
    );
    samples.push({ ...result, process_wall_ms: performance.now() - start });
  }
  return {
    entry: url,
    input: "Hello, **world**.\n",
    options: "omitted",
    import: summarize(samples.map((s) => s.import_ms)),
    first_render: summarize(samples.map((s) => s.first_render_ms)),
    process_wall: summarize(samples.map((s) => s.process_wall_ms)),
    samples,
  };
}

export function parseArgs(args, allowed) {
  const result = {};
  for (let i = 0; i < args.length; i++) {
    const key = args[i];
    if (!allowed.includes(key) || result[key] || !args[i + 1] || args[i + 1].startsWith("--")) {
      throw new Error(`Expected ${allowed.join(" or ")} followed by a path; got ${key}`);
    }
    result[key] = args[++i];
  }
  return result;
}

export function measure(args = process.argv.slice(2)) {
  const options = parseArgs(args, ["--artifact", "--entry"]);
  if (options["--artifact"] && options["--entry"]) {
    throw new Error("--artifact is size-only; use --entry separately for startup");
  }
  const wasm = artifactSizes(options["--artifact"] ?? join(ROOT, "wasm/pkg/ironmark_bg.wasm"));
  return {
    schema: 1,
    env: captureEnv(),
    wasm,
    ...(options["--artifact"]
      ? { mode: "artifact-only" }
      : {
          mode: "package-and-startup",
          package: packageSizes(),
          startup: startup(options["--entry"] ?? join(ROOT, "wasm/node.js")),
        }),
  };
}

if (process.argv[1] && import.meta.url === pathToFileURL(resolve(process.argv[1])).href) {
  try {
    console.log(JSON.stringify(measure(), null, 2));
  } catch (error) {
    console.error(error.message);
    process.exitCode = 1;
  }
}
