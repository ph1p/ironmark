import assert from "node:assert/strict";
import { spawnSync } from "node:child_process";
import { readFileSync } from "node:fs";
import test from "node:test";
import * as full from "./node.js";
import * as html from "./html-node.js";
import * as fullGlue from "./pkg/ironmark_bg.js";
import * as htmlGlue from "./pkg-html/ironmark_html_bg.js";
import { htmlChecks } from "./html-fixtures.mjs";

const wasmModule = new WebAssembly.Module(
  readFileSync(new URL("./pkg-html/ironmark_html_bg.wasm", import.meta.url)),
);
for (const [index, [name, check]] of htmlChecks.entries()) {
  test(name, () => {
    if (index === 0) assert.notStrictEqual(htmlGlue, fullGlue);
    check(full, html, { wasmModule, htmlOnly: true });
  });
}

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
