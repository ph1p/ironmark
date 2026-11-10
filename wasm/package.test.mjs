import assert from "node:assert/strict";
import { spawnSync } from "node:child_process";
import { existsSync, mkdirSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { basename, dirname, join } from "node:path";
import test from "node:test";
import { fileURLToPath, pathToFileURL } from "node:url";

const ROOT = dirname(dirname(fileURLToPath(import.meta.url)));
const TARGET = join(ROOT, "target");
const NODE = process.env.IRONMARK_TEST_NODE || process.execPath;
const PLUGINS = ["vite", "esbuild", "rollup", "webpack", "astro"];

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
  "default", "init", "markdownToAst", "markdownToHtml", "markdownToText", "astToMarkdown", "markdownToAnsi", "mdxToJsx",
  "htmlToAst", "htmlToMarkdown", "CAPABILITIES", "AST_SCHEMA_VERSION",
  "DEFAULT_OPTIONS", "PRESETS", "extractHeadings", "summarizeAst"
].sort());
assert.ok(api.init() instanceof Promise);
assert.equal(await api.init(), undefined);
assert.equal(api.default.parse("# Hi").html(), "<h1>Hi</h1>\\n");
assert.equal(api.default.init, api.init);
const html = api.markdownToHtml("Hello **world**.");
assert.equal(typeof html, "string"); // Not a Promise; no async initialization.
assert.equal(html, "<p>Hello <strong>world</strong>.</p>\\n");
assert.equal(api.markdownToHtml(new TextEncoder().encode("Hello"), {}), "<p>Hello</p>\\n");
const ast = api.markdownToAst("# Hello\\n\\nText");
assert.deepEqual(ast, {Document:{children:[
  {Heading:{level:1,raw:"Hello"}},{Paragraph:{raw:"Text"}}
]}});
assert.equal(api.astToMarkdown(ast), "# Hello\\n\\nText\\n");
const ansi = api.markdownToAnsi("Hello **world**.", {color:false});
assert.equal(typeof ansi, "string");
assert.ok(ansi.includes("Hello world."));
assert.ok(!ansi.includes("\\x1b"));
assert.deepEqual(api.htmlToAst("<h2>Hello</h2><p>Text</p>"), {Document:{children:[
  {Heading:{level:2,raw:"Hello"}},{Paragraph:{raw:"Text"}}
]}});
assert.equal(api.htmlToMarkdown("<h2>Hello</h2><p>Text</p>"), "## Hello\\n\\nText\\n");
assert.equal(api.AST_SCHEMA_VERSION, "2");
assert.equal(api.CAPABILITIES.astSchemaVersion, "2");
assert.deepEqual(api.CAPABILITIES.formats, ["html","text","ast","markdown","ansi","jsx"]);
const jsx = api.mdxToJsx("import X from './x'\\n\\n# Hi {props.name}\\n\\n<X />\\n");
assert.equal(jsx.esm, "import X from './x'");
assert.equal(jsx.jsx, "<>\\n<h1>Hi {props.name}</h1>\\n<X />\\n</>");
assert.ok(jsx.module.includes("export default function MDXContent(props)"));
assert.throws(() => api.mdxToJsx("<A>\\n\\n</B>\\n"), /unexpected closing tag/);
const mapped = api.mdxToJsx("# Hi\\n\\n<Note />\\n", {providerImportSource:"@mdx-js/react"});
assert.ok(mapped.jsx.includes("<_components.h1>Hi</_components.h1>"));
assert.ok(mapped.module.startsWith('import { useMDXComponents as _provideComponents } from "@mdx-js/react";'));
assert.ok(mapped.module.includes("const {Note} = _components;"));
assert.equal(api.markdownToHtml("Hi {x} <Y />", {enableMdx:true}), "<p>Hi {x} <Y /></p>\\n");
assert.equal(api.DEFAULT_OPTIONS.enableTables, true);
assert.ok(Object.isFrozen(api.DEFAULT_OPTIONS));
assert.deepEqual(Object.keys(api.PRESETS).sort(), ["default","safe","strict","llm"].sort());
assert.equal(api.PRESETS.safe.disableRawHtml, true);
assert.ok(Object.isFrozen(api.PRESETS) && Object.isFrozen(api.PRESETS.safe));
assert.ok(Object.isFrozen(api.CAPABILITIES) && Object.isFrozen(api.CAPABILITIES.formats));
assert.equal(api.htmlToMarkdown("<p>H<sub>2</sub>O</p>", {preserveUnknownAsHtml:true}), "H<sub>2</sub>O\\n");
const doc = api.markdownToAst("# Hello *World*\\n\\n> ## **Nested** code\\n\\n- item\\n");
assert.deepEqual(api.extractHeadings(doc), [
  {level:1,text:"Hello World",id:"hello-world"},{level:2,text:"Nested code",id:"nested-code"}
]);
assert.deepEqual(api.summarizeAst(doc), {blockCount:3,nodeCounts:{
  Heading:2,BlockQuote:1,List:1,ListItem:1,Paragraph:1
}});
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
    ...["html", "mdx", "ast"].flatMap((name) => [
      `wasm/${name}-node.js`,
      `wasm/${name}-web.js`,
      `wasm/${name}.d.ts`,
      `wasm/pkg-${name}/ironmark_${name}_bg.js`,
      `wasm/pkg-${name}/ironmark_${name}_bg.wasm`,
    ]),
    ...PLUGINS.flatMap((name) => [`wasm/plugins/${name}.js`, `wasm/plugins/${name}.d.ts`]),
    "wasm/plugins/core.js",
    "wasm/plugins/index.d.ts",
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
    "./html": {
      node: { types: "./wasm/html.d.ts", default: "./wasm/html-node.js" },
      default: { types: "./wasm/html.d.ts", default: "./wasm/html-web.js" },
    },
    "./mdx": {
      node: { types: "./wasm/mdx.d.ts", default: "./wasm/mdx-node.js" },
      default: { types: "./wasm/mdx.d.ts", default: "./wasm/mdx-web.js" },
    },
    "./ast": {
      node: { types: "./wasm/ast.d.ts", default: "./wasm/ast-node.js" },
      default: { types: "./wasm/ast.d.ts", default: "./wasm/ast-web.js" },
    },
    ...Object.fromEntries(
      PLUGINS.map((name) => [
        `./${name}`,
        { types: `./wasm/plugins/${name}.d.ts`, default: `./wasm/plugins/${name}.js` },
      ]),
    ),
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
    assert.equal(report.exports, 16);
    assert.equal(report.synchronous, true);
    t.diagnostic(`Packed consumer runtime: ${report.node}`);
  });

  await t.test("bare package import selects the Node conditional export", () => {
    const report = JSON.parse(
      successful(run(NODE, [probe, "ironmark", entry], { cwd: elsewhere })),
    );
    assert.equal(report.exports, 16);
    assert.equal(report.synchronous, true);
  });

  await t.test("bare subpath imports load their own slim binaries", () => {
    const script = `
const [html, mdx, ast] = await Promise.all(["ironmark/html", "ironmark/mdx", "ironmark/ast"].map((s) => import(s)));
const out = {
  html: Object.keys(html).sort(),
  mdx: Object.keys(mdx).sort(),
  ast: Object.keys(ast).sort(),
  rendered: html.markdownToHtml("**x**"),
  chained: html.default.parse("**x**").html(),
  jsx: typeof mdx.mdxToJsx("# Hi").module,
  heading: ast.markdownToAst("# Hi").Document.children[0].Heading.raw,
};
console.log(JSON.stringify(out));`;
    const probeSub = join(consumer, "probe-subpaths.mjs");
    writeFileSync(probeSub, script);
    const report = JSON.parse(successful(run(NODE, [probeSub], { cwd: elsewhere })));
    assert.deepEqual(report.html, [
      "CAPABILITIES",
      "DEFAULT_OPTIONS",
      "PRESETS",
      "default",
      "init",
      "markdownToHtml",
      "markdownToText",
    ]);
    assert.deepEqual(report.mdx, [
      "CAPABILITIES",
      "DEFAULT_OPTIONS",
      "PRESETS",
      "default",
      "init",
      "markdownToHtml",
      "markdownToText",
      "mdxToJsx",
    ]);
    assert.deepEqual(report.ast, [
      "AST_SCHEMA_VERSION",
      "CAPABILITIES",
      "DEFAULT_OPTIONS",
      "PRESETS",
      "astToMarkdown",
      "default",
      "extractHeadings",
      "init",
      "markdownToAst",
      "markdownToHtml",
      "markdownToText",
      "summarizeAst",
    ]);
    assert.equal(report.rendered, "<p><strong>x</strong></p>\n");
    assert.equal(report.chained, report.rendered);
    assert.equal(report.jsx, "string");
    assert.equal(report.heading, "Hi");
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
