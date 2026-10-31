import assert from "node:assert/strict";
import { spawnSync } from "node:child_process";
import { readFileSync } from "node:fs";
import test from "node:test";
import * as full from "./node.js";
import * as html from "./html-node.js";
import * as fullGlue from "./pkg/ironmark_bg.js";
import * as htmlGlue from "./pkg-html/ironmark_html_bg.js";

const BASELINE = {
  hardBreaks: false,
  enableHighlight: false,
  enableStrikethrough: false,
  enableUnderline: false,
  enableTables: false,
  enableAutolink: false,
  enableTaskLists: false,
};

test("HTML-only JS and compiled WASM exports omit unrelated formats", () => {
  assert.deepEqual(Object.keys(html).sort(), ["init", "renderHtml"]);
  assert.equal(html.init(), undefined);
  assert.notStrictEqual(htmlGlue, fullGlue);
  const module = new WebAssembly.Module(
    readFileSync(new URL("./pkg-html/ironmark_html_bg.wasm", import.meta.url)),
  );
  const names = WebAssembly.Module.exports(module).map((entry) => entry.name);
  assert.ok(names.includes("parse"));
  assert.ok(names.includes("parseDefault"));
  for (const removed of [
    "parseToAst",
    "renderAnsi",
    "parseHtmlToAst",
    "htmlToMarkdown",
    "renderMarkdown",
  ]) {
    assert.ok(!names.includes(removed), `unexpected slim WASM export: ${removed}`);
  }
  // Interleaving full and slim calls must not overwrite each other's instance.
  for (let i = 0; i < 3; i++) {
    assert.equal(html.renderHtml("**slim**"), "<p><strong>slim</strong></p>\n");
    assert.deepEqual(full.parseMarkdown("full"), {
      Document: { children: [{ Paragraph: { raw: "full" } }] },
    });
    assert.equal(full.htmlToMarkdown("<p>full</p>"), "full\n");
    assert.equal(html.renderHtml("plain"), "<p>plain</p>\n");
  }
});

test("all 652 CommonMark examples match exact expected HTML independently", () => {
  const examples = JSON.parse(
    readFileSync(new URL("../tests/spec/spec-0.31.2.json", import.meta.url), "utf8"),
  );
  assert.equal(examples.length, 652);
  for (const example of examples) {
    assert.equal(
      full.renderHtml(example.markdown, BASELINE),
      example.html,
      `full example ${example.example}: ${example.section}`,
    );
    assert.equal(
      html.renderHtml(example.markdown, BASELINE),
      example.html,
      `HTML-only example ${example.example}: ${example.section}`,
    );
  }
});

test("every public parse option is exercised with exact full/slim parity", () => {
  const exercised = new Set();
  const fixtures = [
    ["hardBreaks", "a\nb"],
    ["enableHighlight", "==mark=="],
    ["enableStrikethrough", "~~gone~~"],
    ["enableUnderline", "++under++"],
    ["enableTables", "|H|\n|---|\n|x|"],
    ["enableAutolink", "https://example.com"],
    ["enableTaskLists", "- [x] done"],
    ["disableRawHtml", "<b>raw</b>"],
    ["enableHeadingIds", "# Heading"],
    ["enableHeadingAnchors", "# Heading", { enableHeadingIds: true }],
    ["enableIndentedCodeBlocks", "    code"],
    ["noHtmlBlocks", "<div>\nraw\n</div>"],
    ["noHtmlSpans", "some <b>raw</b> text"],
    ["tagFilter", "<script>alert(1)</script>"],
    ["collapseWhitespace", "a   b"],
    ["permissiveAtxHeaders", "#Heading"],
    ["enableWikiLinks", "[[Page]]"],
    ["enableLatexMath", "$x$"],
    ["safe", "<b>raw</b>"],
    ["deterministic", "a   b"],
  ];
  for (const [key, input, base = {}] of fixtures) {
    const enabled = { ...base, [key]: true };
    const disabled = { ...base, [key]: false };
    const on = full.renderHtml(input, enabled);
    const off = full.renderHtml(input, disabled);
    assert.notEqual(on, off, `${key} fixture must actually exercise its behavior`);
    assert.equal(html.renderHtml(input, enabled), on, `${key}=true`);
    assert.equal(html.renderHtml(input, disabled), off, `${key}=false`);
    exercised.add(key);
  }
  const presetInput =
    "# Heading\n\n**bold** ==mark== ~~gone~~ ++u++ https://example.com\n\n<b>raw</b>\n\nSome   text\n";
  for (const preset of ["default", "safe", "strict", "llm"]) {
    const expected = full.renderHtml(presetInput, { preset });
    assert.equal(html.renderHtml(presetInput, { preset }), expected, preset);
    if (preset !== "default") assert.notEqual(expected, full.renderHtml(presetInput));
  }
  assert.equal(
    html.renderHtml("a   b", { preset: "llm", collapseWhitespace: false }),
    full.renderHtml("a   b", { preset: "llm", collapseWhitespace: false }),
  );
  exercised.add("preset");
  // Reserved AST metadata flag intentionally has no HTML effect.
  assert.equal(
    html.renderHtml("text", { stableAst: true }),
    full.renderHtml("text", { stableAst: true }),
  );
  assert.equal(
    html.renderHtml("text", { stableAst: false }),
    full.renderHtml("text", { stableAst: false }),
  );
  exercised.add("stableAst");
  const declarations = readFileSync(new URL("./index.d.ts", import.meta.url), "utf8");
  const options = declarations.match(/export interface ParseOptions \{([\s\S]*?)^\}/m)[1];
  const declared = [...options.matchAll(/^\s+(\w+)\?:/gm)].map((match) => match[1]);
  assert.deepEqual(
    [...exercised].sort(),
    declared.sort(),
    "new public options need exercised fixtures",
  );
});

test("string, byte views, BOM, and malformed UTF-8 keep exact default HTML", () => {
  for (const [input, expected] of [
    ["Hello **world**.", "<p>Hello <strong>world</strong>.</p>\n"],
    ["", ""],
    [new Uint8Array([0x61, 0xff, 0x62, 0xc3]), "<p>a\uFFFDb\uFFFD</p>\n"],
    [new Uint8Array([0xef, 0xbb, 0xbf, 0x61, 0x26, 0x62]), "<p>a&amp;b</p>\n"],
  ]) {
    for (const options of [undefined, {}, { preset: "default" }]) {
      assert.equal(full.renderHtml(input, options), expected);
      assert.equal(html.renderHtml(input, options), expected);
    }
  }
  const bytes = new TextEncoder().encode("!Unicode α & text!");
  for (const input of [
    bytes.subarray(1, bytes.length - 1),
    Buffer.from(bytes).subarray(1, bytes.length - 1),
    new DataView(bytes.buffer, 1, bytes.length - 2),
    bytes.slice(1, -1).buffer,
  ]) {
    assert.equal(html.renderHtml(input), "<p>Unicode α &amp; text</p>\n");
    assert.equal(html.renderHtml(input), full.renderHtml(input));
  }
});

test("default/configured HTML-only rendering preserves the 10 MiB UTF-8 limit", () => {
  const limit = 10 * 1024 * 1024;
  const fence = "ordinary paragraph\n\n```\n";
  const inputOfSize = (size) => fence + "x".repeat(size - fence.length);
  const cases = [
    [limit - 1, "", limit - 1],
    [limit, "", limit],
    [limit + 1, "", limit],
    [limit, "SUFFIX", limit],
    [limit - 1, "éSUFFIX", limit - 1],
  ];
  for (const [size, suffix, retainedSize] of cases) {
    const input = inputOfSize(size) + suffix;
    const prefix = inputOfSize(retainedSize);
    const expected = `<p>ordinary paragraph</p>\n<pre><code>${prefix.slice(fence.length)}\n</code></pre>\n`;
    for (const value of [input, new TextEncoder().encode(input)]) {
      for (const options of [undefined, {}, { preset: "default" }]) {
        for (const [name, api] of [
          ["full", full],
          ["HTML-only", html],
        ]) {
          const actual = api.renderHtml(value, options);
          assert.ok(
            actual === expected,
            `${name} input limit mismatch (${actual.length} vs ${expected.length})`,
          );
        }
      }
    }
  }
  const prefix = inputOfSize(limit - 2);
  const malformed = new Uint8Array(limit);
  malformed.set(new TextEncoder().encode(prefix));
  malformed.set([0xff, 0x61], limit - 2);
  // Lossy decoding expands 0xff to three UTF-8 bytes, crossing the limit.
  const expected = `<p>ordinary paragraph</p>\n<pre><code>${prefix.slice(fence.length)}\n</code></pre>\n`;
  for (const options of [undefined, {}, { preset: "default" }]) {
    for (const [name, api] of [
      ["full", full],
      ["HTML-only", html],
    ]) {
      const actual = api.renderHtml(malformed, options);
      assert.ok(actual === expected, `${name} decoded input limit mismatch`);
    }
  }
});

test("browser prototype initializes a compiled Module without sharing the full glue", () => {
  const script = `
import assert from "node:assert/strict";
import {readFileSync} from "node:fs";
const [fullURL, htmlURL, assetURL] = process.argv.slice(1);
const full = await import(fullURL);
const html = await import(htmlURL);
assert.deepEqual(Object.keys(html).sort(), ["init","renderHtml"]);
const before = full.renderHtml("**full**");
const module = new WebAssembly.Module(readFileSync(new URL(assetURL)));
const initialized = html.init(module);
assert.ok(initialized instanceof Promise);
await initialized;
await html.init(module);
assert.equal(html.renderHtml("**slim**"), "<p><strong>slim</strong></p>\\n");
assert.equal(full.renderHtml("**full**"), before);
assert.deepEqual(full.parseMarkdown("full"), {Document:{children:[{Paragraph:{raw:"full"}}]}});
`;
  const result = spawnSync(
    process.execPath,
    [
      "--input-type=module",
      "-e",
      script,
      new URL("./node.js", import.meta.url).href,
      new URL("./html-web.js", import.meta.url).href,
      new URL("./pkg-html/ironmark_html_bg.wasm", import.meta.url).href,
    ],
    { encoding: "utf8", timeout: 30_000 },
  );
  assert.ifError(result.error);
  assert.equal(result.status, 0, result.stderr);
});
