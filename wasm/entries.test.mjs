// Parity: every slim subpath entry must produce exactly what the full entry produces.
import assert from "node:assert/strict";
import test from "node:test";
import * as full from "./node.js";
import * as html from "./html-node.js";
import * as mdx from "./mdx-node.js";
import * as ast from "./ast-node.js";

const SAMPLES = [
  "# Title\n\nHello **world** and https://x.com> ok.\n",
  "| a | b |\n|---|---|\n| 1 | 2 |\n\n- [x] done\n- [ ] todo\n",
  "```js\nconst a = 1;\n```\n\n> quote ~~gone~~ ==mark==\n",
];
const MDX =
  'import { Note } from "./note.js";\n\n# Hi {props.name}\n\n<Note kind="info">**body**</Note>\n';

test("slim entries expose only their own API", () => {
  assert.deepEqual(Object.keys(html).sort(), [
    "CAPABILITIES",
    "DEFAULT_OPTIONS",
    "PRESETS",
    "init",
    "markdownToHtml",
  ]);
  assert.deepEqual(Object.keys(mdx).sort(), [
    "CAPABILITIES",
    "DEFAULT_OPTIONS",
    "PRESETS",
    "init",
    "markdownToHtml",
    "mdxToJsx",
  ]);
  assert.deepEqual(Object.keys(ast).sort(), [
    "AST_SCHEMA_VERSION",
    "CAPABILITIES",
    "DEFAULT_OPTIONS",
    "PRESETS",
    "astToMarkdown",
    "extractHeadings",
    "init",
    "markdownToAst",
    "markdownToHtml",
    "summarizeAst",
  ]);
});

test("each entry advertises only the formats its binary supports", () => {
  assert.deepEqual(html.CAPABILITIES.formats, ["html"]);
  assert.deepEqual(mdx.CAPABILITIES.formats, ["html", "jsx"]);
  assert.deepEqual(ast.CAPABILITIES.formats, ["html", "ast", "markdown"]);
  for (const entry of [html, mdx, ast]) {
    assert.deepEqual({ ...entry.CAPABILITIES, formats: [] }, { ...full.CAPABILITIES, formats: [] });
    assert.strictEqual(entry.DEFAULT_OPTIONS, full.DEFAULT_OPTIONS);
    assert.strictEqual(entry.PRESETS, full.PRESETS);
  }
});

test("markdownToHtml matches the full entry in every subpath", () => {
  for (const entry of [html, mdx, ast]) {
    for (const md of SAMPLES) {
      assert.equal(entry.markdownToHtml(md), full.markdownToHtml(md));
      assert.equal(
        entry.markdownToHtml(md, { preset: "safe" }),
        full.markdownToHtml(md, { preset: "safe" }),
      );
    }
  }
});

test("ironmark/mdx mdxToJsx matches the full entry", () => {
  assert.deepEqual(mdx.mdxToJsx(MDX), full.mdxToJsx(MDX));
  assert.deepEqual(
    mdx.mdxToJsx(MDX, { providerImportSource: "@mdx-js/react" }),
    full.mdxToJsx(MDX, { providerImportSource: "@mdx-js/react" }),
  );
});

test("ironmark/ast round-trips like the full entry", () => {
  for (const md of SAMPLES) {
    const tree = ast.markdownToAst(md);
    assert.deepEqual(tree, full.markdownToAst(md));
    assert.equal(ast.astToMarkdown(tree), full.astToMarkdown(tree));
    assert.deepEqual(ast.extractHeadings(tree), full.extractHeadings(tree));
    assert.deepEqual(ast.summarizeAst(tree), full.summarizeAst(tree));
  }
  assert.equal(ast.AST_SCHEMA_VERSION, full.AST_SCHEMA_VERSION);
});
