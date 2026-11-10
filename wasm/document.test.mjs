// The chainable default export: `ironmark.parse(md).html()` and friends.
import assert from "node:assert/strict";
import test from "node:test";
import ironmark, * as full from "./node.js";
import htmlEntry from "./html-node.js";
import mdxEntry from "./mdx-node.js";
import astEntry from "./ast-node.js";

const MD = "# Title *x*\n\n## Sub\n\nHello **world** [l](http://a.b).\n\n### Deep\n\n# Two\n";

test("document outputs match the named functions", () => {
  const doc = ironmark.parse(MD);
  assert.equal(doc.source, MD);
  assert.equal(doc.html(), full.markdownToHtml(MD));
  assert.equal(doc.text(), full.markdownToText(MD));
  assert.deepEqual(doc.ast(), full.markdownToAst(MD));
  assert.equal(doc.markdown(), full.astToMarkdown(full.markdownToAst(MD)));
  assert.equal(doc.ansi({ color: false }), full.markdownToAnsi(MD, { color: false }));
  assert.deepEqual(doc.headings(), full.extractHeadings(full.markdownToAst(MD)));
  assert.deepEqual(doc.summary(), full.summarizeAst(full.markdownToAst(MD)));
  assert.equal("toString" in Object.getPrototypeOf(doc), true); // Object.prototype only
  assert.equal(Object.hasOwn(Object.getPrototypeOf(doc), "toString"), false);
  assert.equal(Object.hasOwn(Object.getPrototypeOf(doc), "toJSON"), false);
  const mdx = ironmark.parse("# Hi {props.name}\n");
  assert.deepEqual({ ...mdx.jsx() }, full.mdxToJsx("# Hi {props.name}\n"));
});

test("outputs are cached per render-option values and frozen", () => {
  const doc = ironmark.parse(MD);
  assert.strictEqual(doc.ast(), doc.ast());
  assert.strictEqual(doc.headings(), doc.headings());
  assert.strictEqual(doc.jsx(), doc.jsx({}));
  assert.ok(Object.isFrozen(doc.ast().Document.children[0].Heading));
  assert.ok(Object.isFrozen(doc.toc()[0].children));
  assert.ok(Object.isFrozen(doc.headings()));
  assert.equal(doc.ansi(), doc.ansi({}));
  assert.notEqual(doc.ansi({ color: false }), doc.ansi());
  assert.equal(doc.ansi({ color: false }), doc.ansi({ color: false, width: undefined }));
  // Parse options go through parse()/with(), never through render methods.
  assert.equal(
    doc.with({ enableHeadingIds: true }).html(),
    full.markdownToHtml(MD, { enableHeadingIds: true }),
  );
});

test("text() and headings() use the real inline parser", () => {
  const doc = ironmark.parse("# A [ref] &amp; `b`\n\none\ntwo\n\n[ref]: http://x.y\n");
  assert.equal(doc.text(), "A ref & b\n\none\ntwo");
  assert.equal(doc.with({ hardBreaks: false }).text(), "A ref & b\n\none two");
  assert.deepEqual(doc.headings(), [{ level: 1, text: "A ref & b", id: "a-ref-amp-b" }]);
});

test("text() and toc()", () => {
  const doc = ironmark.parse(
    `${MD}\n| a | b |\n|---|---|\n| 1 | 2 |\n\n- x\n- y\n\n<div>raw</div>\n`,
  );
  assert.equal(doc.text(), "Title x\n\nSub\n\nHello world l.\n\nDeep\n\nTwo\n\na\tb\n1\t2\n\nx\ny");
  assert.deepEqual(doc.toc(), [
    {
      level: 1,
      text: "Title x",
      id: "title-x",
      children: [
        {
          level: 2,
          text: "Sub",
          id: "sub",
          children: [{ level: 3, text: "Deep", id: "deep", children: [] }],
        },
      ],
    },
    { level: 1, text: "Two", id: "two", children: [] },
  ]);
});

test("options layer: with(), presets and switches", () => {
  const raw = "<b>x</b>";
  const safe = ironmark.with({ preset: "safe" });
  assert.deepEqual(safe.options, { disableRawHtml: true, tagFilter: true });
  assert.ok(Object.isFrozen(safe.options));
  assert.deepEqual(ironmark.options, {});
  assert.equal(safe.parse(raw).html(), full.markdownToHtml(raw, { preset: "safe" }));
  assert.equal(
    safe.parse(raw).with({ disableRawHtml: false }).html(),
    full.markdownToHtml(raw, { tagFilter: true }),
  );
  assert.equal(ironmark.parse(raw).html(), full.markdownToHtml(raw));
  assert.equal(
    ironmark.parse(raw, { safe: true }).html(),
    full.markdownToHtml(raw, { safe: true }),
  );
  // A preset in a layer starts over from that preset.
  const reset = ironmark.with({ enableHeadingIds: true }).with({ preset: "strict" });
  assert.equal(reset.options.enableHeadingIds, undefined);
  assert.equal(reset.options.hardBreaks, false);
  // `safe: false` / `deterministic: false` switch their flags back off.
  assert.deepEqual(safe.with({ safe: false }).options, { disableRawHtml: false, tagFilter: false });
  assert.equal(safe.parse(raw).with({ safe: false }).html(), full.markdownToHtml(raw));
  const det = ironmark.with({ deterministic: true });
  assert.deepEqual(det.options, { collapseWhitespace: true });
  assert.deepEqual(det.with({ deterministic: false }).options, { collapseWhitespace: false });
  assert.equal("stableAst" in ironmark.with({ stableAst: true }).options, false);
});

test("string input keeps the original string as source", () => {
  const md = "caf\u00e9 \ud83d\ude00";
  assert.strictEqual(ironmark.parse(md).source, md);
  assert.strictEqual(ironmark.parse(md).with({ hardBreaks: false }).source, md);
});

test("byte input is copied and decoded once", () => {
  const bytes = new TextEncoder().encode("# Bytes");
  const doc = ironmark.parse(bytes);
  bytes.fill(0x20);
  assert.equal(doc.source, "# Bytes");
  assert.equal(doc.html(), "<h1>Bytes</h1>\n");
  assert.throws(() => ironmark.parse(42), TypeError);
});

test("fromHtml() converts to Markdown first", () => {
  const doc = ironmark.fromHtml("<h1>Hi</h1><p><b>x</b> <sup>2</sup></p>", {
    preserveUnknownAsHtml: true,
  });
  assert.equal(
    doc.source,
    full.htmlToMarkdown("<h1>Hi</h1><p><b>x</b> <sup>2</sup></p>", { preserveUnknownAsHtml: true }),
  );
  assert.deepEqual(doc.options, {});
  assert.equal(doc.headings()[0].text, "Hi");
});

test("slim entries only offer the methods their binary supports", () => {
  const methods = (entry) =>
    ["html", "ansi", "jsx", "ast", "markdown", "summary"].filter((m) => m in entry.parse("# x"));
  assert.deepEqual(methods(htmlEntry), ["html"]);
  assert.deepEqual(methods(mdxEntry), ["html", "jsx"]);
  assert.deepEqual(methods(astEntry), ["html", "ast", "markdown", "summary"]);
  assert.equal(methods(ironmark).length, 6);
  const common = ["text", "headings", "toc", "links", "images", "frontmatter", "stats"];
  for (const entry of [ironmark, htmlEntry, mdxEntry, astEntry]) {
    const doc = entry.parse("# x");
    for (const method of common) assert.equal(method in doc, true, method);
    // Shortcuts exist exactly for the outputs the entry's documents have.
    for (const method of ["html", "ansi", "jsx", "ast", "markdown", ...common]) {
      assert.equal(method in entry, method in doc, `shortcut ${method}`);
    }
  }
  for (const entry of [htmlEntry, mdxEntry, astEntry]) {
    const doc = entry.parse(MD);
    assert.equal(doc.html(), ironmark.parse(MD).html());
    assert.equal(doc.text(), ironmark.parse(MD).text());
    assert.deepEqual(doc.toc(), ironmark.parse(MD).toc());
    assert.equal("fromHtml" in entry, false);
  }
  assert.equal(astEntry.parse(MD).markdown(), ironmark.parse(MD).markdown());
});

test("links() and images()", () => {
  const md =
    '# [Home](/ "Start")\n\nSee [docs][d], <https://a.dev> and [![logo &amp; mark](l.png "L")](/x).\n\n| <a href="/raw">raw</a> |\n|---|\n\n[d]: /docs\n';
  const doc = ironmark.parse(md);
  assert.deepEqual(doc.links(), [
    { url: "/", title: "Start", text: "Home" },
    { url: "/docs", title: null, text: "docs" },
    { url: "https://a.dev", title: null, text: "https://a.dev" },
    { url: "/x", title: null, text: "logo & mark" },
    { url: "/raw", title: null, text: "raw" },
  ]);
  assert.deepEqual(doc.images(), [{ url: "l.png", title: "L", alt: "logo & mark" }]);
  assert.strictEqual(doc.links(), doc.links());
  assert.ok(Object.isFrozen(doc.images()[0]));
  assert.deepEqual(ironmark.parse("[x](javascript:alert(1))").links(), [
    { url: "", title: null, text: "x" },
  ]);
  assert.deepEqual(ironmark.parse("[[Wiki Page]]", { enableWikiLinks: true }).links(), [
    { url: "wiki_page", title: null, text: "Wiki Page" },
  ]);
  assert.deepEqual(htmlEntry.parse(md).links(), doc.links());
});

test("frontmatter() is opt-in and stripped from every output", () => {
  const md = "---\ntitle: Hi\ntags: [a]\n---\n# Body\n";
  const doc = ironmark.parse(md, { enableFrontmatter: true });
  assert.equal(doc.frontmatter(), "title: Hi\ntags: [a]\n");
  assert.equal(doc.html(), "<h1>Body</h1>\n");
  assert.equal(doc.text(), "Body");
  assert.deepEqual(doc.headings(), [{ level: 1, text: "Body", id: "body" }]);
  assert.equal(doc.markdown(), "# Body\n");
  assert.equal(ironmark.parse(md).frontmatter(), null);
  assert.notEqual(ironmark.parse(md).html(), doc.html());
  assert.equal(ironmark.parse("# none", { enableFrontmatter: true }).frontmatter(), null);
  assert.equal(
    ironmark.parse("+++\na = 1\n+++\nx", { enableFrontmatter: true }).frontmatter(),
    "a = 1\n",
  );
  assert.equal(mdxEntry.parse(md, { enableFrontmatter: true }).frontmatter(), doc.frontmatter());
});

test("stats() counts the plain text", () => {
  assert.deepEqual(ironmark.parse("# Hello *world*\n\nDon't stop — `x`!").stats(), {
    words: 5,
    characters: 22,
    readingTime: 1,
  });
  assert.deepEqual(ironmark.parse("").stats(), { words: 0, characters: 0, readingTime: 0 });
  assert.equal(ironmark.parse("word ".repeat(401)).stats().readingTime, 3);
});

test("shortcuts render once without a document", () => {
  assert.equal(ironmark.html(MD), full.markdownToHtml(MD));
  assert.equal(ironmark.text(MD), full.markdownToText(MD));
  assert.equal(ironmark.ansi(MD, { color: false }), full.markdownToAnsi(MD, { color: false }));
  assert.deepEqual(ironmark.toc(MD), ironmark.parse(MD).toc());
  assert.equal(ironmark.with({ preset: "safe" }).html("<b>x</b>"), "<p>&lt;b&gt;x&lt;/b&gt;</p>\n");
  assert.equal(htmlEntry.html(new TextEncoder().encode("*b*")), "<p><em>b</em></p>\n");
  assert.equal(mdxEntry.jsx("# Hi").jsx, mdxEntry.parse("# Hi").jsx().jsx);
  assert.equal(astEntry.markdown("*a*"), "*a*\n");
});

test("ready resolves like init()", async () => {
  for (const entry of [ironmark, htmlEntry, mdxEntry, astEntry]) {
    assert.ok(entry.ready instanceof Promise);
    await entry.ready;
  }
});
