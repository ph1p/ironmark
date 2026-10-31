import assert from "node:assert/strict";
import { spawnSync } from "node:child_process";
import { existsSync, mkdirSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { basename, dirname, join } from "node:path";
import test from "node:test";
import { fileURLToPath, pathToFileURL } from "node:url";

const ROOT = dirname(dirname(fileURLToPath(import.meta.url)));
const TARGET = join(ROOT, "target");
const NODE = process.env.IRONMARK_TEST_NODE || process.execPath;

function run(command, args, options = {}) {
  return spawnSync(command, args, {
    encoding: "utf8",
    timeout: 120_000,
    maxBuffer: 16 * 1024 * 1024,
    ...options,
  });
}

function successful(result) {
  assert.ifError(result.error);
  assert.equal(result.status, 0, result.stderr);
  return result.stdout;
}

function temporary(t) {
  mkdirSync(TARGET, { recursive: true });
  const dir = mkdtempSync(join(TARGET, "package-test-"));
  t.after(() => rmSync(dir, { recursive: true, force: true }));
  return dir;
}

const PROBE = `
import assert from "node:assert/strict";
import { createRequire } from "node:module";
import { readFileSync } from "node:fs";
import { fileURLToPath } from "node:url";
const api = await import(process.argv[2]);
const entry = process.argv[3];
assert.strictEqual(api, await import(entry));
assert.deepEqual(Object.keys(api).sort(), [
  "init", "parseMarkdown", "renderHtml", "renderMarkdown", "renderAnsiTerminal",
  "parseHtmlToAst", "htmlToMarkdown", "getCapabilities", "getAstSchemaVersion",
  "getDefaultOptions", "getPresets", "extractHeadings", "summarizeAst"
].sort());
assert.equal(api.init(), undefined);
assert.equal(api.init(), undefined);
const html = api.renderHtml("Hello **world**.");
assert.equal(typeof html, "string"); // Not a Promise; no async initialization.
assert.equal(html, "<p>Hello <strong>world</strong>.</p>\\n");
assert.equal(api.renderHtml(new TextEncoder().encode("Hello"), {}), "<p>Hello</p>\\n");
const ast = api.parseMarkdown("# Hello\\n\\nText");
assert.deepEqual(ast, {Document:{children:[
  {Heading:{level:1,raw:"Hello"}},{Paragraph:{raw:"Text"}}
]}});
assert.equal(api.renderMarkdown(ast), "# Hello\\n\\nText\\n");
const ansi = api.renderAnsiTerminal("Hello **world**.", undefined, {color:false});
assert.equal(typeof ansi, "string");
assert.ok(ansi.includes("Hello world."));
assert.ok(!ansi.includes("\\x1b"));
assert.deepEqual(api.parseHtmlToAst("<h2>Hello</h2><p>Text</p>"), {Document:{children:[
  {Heading:{level:2,raw:"Hello"}},{Paragraph:{raw:"Text"}}
]}});
assert.equal(api.htmlToMarkdown("<h2>Hello</h2><p>Text</p>"), "## Hello\\n\\nText\\n");
assert.equal(api.getAstSchemaVersion(), "2");
assert.equal(api.getCapabilities().astSchemaVersion, "2");
assert.deepEqual(api.getCapabilities().formats, ["html","ast","markdown","ansi"]);
const defaults = api.getDefaultOptions();
assert.equal(defaults.enableTables, true);
defaults.hardBreaks = false;
assert.equal(api.getDefaultOptions().hardBreaks, true);
const presets = api.getPresets();
assert.deepEqual(Object.keys(presets).sort(), ["default","safe","strict","llm"].sort());
assert.equal(presets.safe.disableRawHtml, true);
presets.safe.disableRawHtml = false;
assert.equal(api.getPresets().safe.disableRawHtml, true);
const headingNodes = [{type:"Heading",level:2,children:[{type:"Text",text:"Hello World"}]}];
assert.deepEqual(api.extractHeadings(headingNodes), [{level:2,text:"Hello World",id:"hello-world"}]);
assert.deepEqual(api.summarizeAst(headingNodes), {blockCount:1,nodeCounts:{Heading:1,Text:1}});
const require = createRequire(import.meta.url);
assert.equal(require.resolve("ironmark"), fileURLToPath(entry));
assert.deepEqual(
  readFileSync(require.resolve("ironmark/ironmark.wasm")),
  readFileSync(new URL("./pkg/ironmark_bg.wasm", entry))
);
console.log(JSON.stringify({node:process.version,exports:Object.keys(api).length,synchronous:true}));
`;

test("packed package resolves its file-backed assets independently of cwd", async (t) => {
  assert.ok(existsSync(join(ROOT, "wasm/node.js")), "run pnpm build before package tests");
  const dir = temporary(t);
  const consumer = join(dir, "consumer space café");
  const packaged = join(consumer, "node_modules/ironmark");
  const elsewhere = join(dir, "different cwd Ω");
  mkdirSync(packaged, { recursive: true });
  mkdirSync(elsewhere, { recursive: true });
  // Establish a consumer package boundary: otherwise the enclosing repository's
  // own "ironmark" name enables self-resolution instead of node_modules lookup.
  writeFileSync(
    join(consumer, "package.json"),
    JSON.stringify({ name: "ironmark-package-consumer", private: true, type: "module" }),
  );
  const output = successful(
    run("npm", ["pack", "--ignore-scripts", "--json", "--pack-destination", dir], {
      cwd: ROOT,
      env: { ...process.env, npm_config_cache: join(TARGET, "npm-cache") },
    }),
  );
  const packs = JSON.parse(output);
  assert.equal(packs.length, 1);
  const pack = packs[0];
  assert.equal(typeof pack.filename, "string");
  assert.equal(basename(pack.filename), pack.filename);
  assert.ok(!pack.filename.includes("\\") && pack.filename.endsWith(".tgz"));
  for (const asset of [
    "wasm/node.js",
    "wasm/pkg/ironmark_bg.wasm",
    "wasm/pkg/ironmark_bg.js",
    "wasm/web.js",
    "wasm/shared.js",
    "wasm/index.d.ts",
  ]) {
    assert.ok(
      pack.files.some((file) => file.path === asset),
      `missing packed ${asset}`,
    );
  }
  successful(
    run("tar", ["-xzf", join(dir, pack.filename), "--strip-components=1", "-C", packaged]),
  );
  const packageJson = JSON.parse(readFileSync(join(packaged, "package.json"), "utf8"));
  assert.deepEqual(packageJson.exports, {
    ".": {
      node: { types: "./wasm/index.d.ts", default: "./wasm/node.js" },
      default: { types: "./wasm/index.d.ts", default: "./wasm/web.js" },
    },
    "./ironmark.wasm": "./wasm/pkg/ironmark_bg.wasm",
  });
  assert.equal(packageJson.module, "wasm/web.js");
  for (const asset of ["wasm/pkg/ironmark_bg.wasm", "wasm/pkg/ironmark_bg.js", "wasm/web.js"]) {
    assert.deepEqual(readFileSync(join(packaged, asset)), readFileSync(join(ROOT, asset)));
  }
  const generated = readFileSync(join(packaged, "wasm/node.js"), "utf8");
  assert.ok(!/base64|atob|Uint8Array\.from/.test(generated));
  const entry = pathToFileURL(join(packaged, "wasm/node.js")).href;
  const probe = join(consumer, "probe.mjs");
  writeFileSync(probe, PROBE);

  await t.test("direct file URL keeps all existing exports usable and synchronous", () => {
    const report = JSON.parse(successful(run(NODE, [probe, entry, entry], { cwd: elsewhere })));
    assert.equal(report.exports, 13);
    assert.equal(report.synchronous, true);
    t.diagnostic(`Packed consumer runtime: ${report.node}`);
  });

  await t.test("bare package import selects the Node conditional export", () => {
    const report = JSON.parse(
      successful(run(NODE, [probe, "ironmark", entry], { cwd: elsewhere })),
    );
    assert.equal(report.exports, 13);
    assert.equal(report.synchronous, true);
  });

  await t.test("missing raw WASM fails in a fresh disposable package", () => {
    rmSync(join(packaged, "wasm/pkg/ironmark_bg.wasm")); // Never the repository artifact.
    const result = run(NODE, [probe, "ironmark", entry], { cwd: elsewhere });
    assert.ifError(result.error);
    assert.notEqual(result.status, 0);
    assert.match(result.stderr, /ENOENT/);
    assert.match(result.stderr, /ironmark_bg\.wasm/);
    assert.ok(existsSync(join(ROOT, "wasm/pkg/ironmark_bg.wasm")));
  });
});

test("Node generator fails clearly before writing when its WASM artifact is missing", (t) => {
  const dir = temporary(t);
  const builder = join(dir, "build.mjs");
  writeFileSync(builder, readFileSync(join(ROOT, "wasm/build.mjs")));
  const result = run(NODE, [builder], { cwd: ROOT });
  assert.ifError(result.error);
  assert.notEqual(result.status, 0);
  assert.match(result.stderr, /Cannot read WASM artifact/);
  assert.match(result.stderr, /build it before generating the Node entry/);
  assert.ok(!existsSync(join(dir, "node.js")));
});
