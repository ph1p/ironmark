import assert from "node:assert/strict";
import { existsSync, mkdtempSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { dirname, join } from "node:path";
import test from "node:test";
import { fileURLToPath } from "node:url";
import astro from "./plugins/astro.js";
import { createMdxCompiler } from "./plugins/core.js";
import esbuild from "./plugins/esbuild.js";
import rollup from "./plugins/rollup.js";
import vite from "./plugins/vite.js";
import webpack from "./plugins/webpack.js";

const ROOT = dirname(dirname(fileURLToPath(import.meta.url)));
const SOURCE = "---\ntitle: Hi\n---\n\nimport X from './x.js'\n\n# Hello {props.name}\n\n<X />\n";
const BROKEN = "<A>\n\nunclosed\n";
const parseFrontmatter = (raw) => Object.fromEntries([raw.trim().split(": ")]);

/** Collects `this.error()` calls the way Rollup and Vite do: by throwing. */
const pluginContext = (meta = {}) => ({
  meta,
  error(message) {
    throw new Error(message);
  },
});

test("core: compiles MDX with pragmas, components and stripped front matter", () => {
  const code = createMdxCompiler().compile(SOURCE, "doc.mdx");
  assert.ok(code.startsWith("/** @jsxRuntime automatic */\n/** @jsxImportSource react */\n"));
  assert.match(code, /import X from '\.\/x\.js'/);
  assert.match(code, /props\.components/);
  assert.doesNotMatch(code, /title: Hi|<hr/);
  assert.doesNotMatch(code, /export const frontmatter/);
});

test("core: frontmatter option exports the parsed value", () => {
  const seen = [];
  const mdx = createMdxCompiler({
    frontmatter: (raw, file) => (seen.push(file), parseFrontmatter(raw)),
  });
  assert.match(mdx.compile(SOURCE, "doc.mdx"), /export const frontmatter = \{"title":"Hi"\};/);
  assert.deepEqual(seen, ["doc.mdx"]);
  assert.match(mdx.compile("# No front matter\n", "b.mdx"), /export const frontmatter = \{\};/);
});

test("core: options pass through to the JSX renderer", () => {
  const preact = createMdxCompiler({ jsxImportSource: "preact", components: false });
  const code = preact.compile(SOURCE, "doc.mdx");
  assert.match(code, /@jsxImportSource preact/);
  assert.doesNotMatch(code, /_components/);
  const provider = createMdxCompiler({ providerImportSource: "@mdx-js/react" });
  assert.match(provider.compile(SOURCE, "doc.mdx"), /@mdx-js\/react/);
});

test("core: test() matches include, ignores queries and virtual ids", () => {
  const mdx = createMdxCompiler();
  assert.ok(mdx.test("/a/doc.mdx"));
  assert.ok(mdx.test("/a/doc.mdx?import"));
  assert.ok(!mdx.test("/a/doc.md"));
  assert.ok(!mdx.test("\0virtual.mdx"));
  const sticky = createMdxCompiler({ include: /\.mdx?$/g });
  assert.ok(sticky.test("a.md") && sticky.test("a.md"), "global regex lastIndex is reset");
});

test("core: errors name the file", () => {
  assert.throws(() => createMdxCompiler().compile(BROKEN, "bad.mdx"), /^Error: bad\.mdx: .*<A>/);
});

test("esbuild: onLoad returns JSX contents, or an error with its location", async () => {
  const dir = mkdtempSync(join(tmpdir(), "ironmark-plugins-"));
  try {
    const good = join(dir, "good.mdx");
    const bad = join(dir, "bad.mdx");
    writeFileSync(good, SOURCE);
    writeFileSync(bad, BROKEN);
    let filter;
    let onLoad;
    esbuild().setup({
      onLoad(options, callback) {
        ({ filter } = options);
        onLoad = callback;
      },
    });
    assert.ok(filter.test(good) && !filter.test(join(dir, "x.md")));
    const ok = await onLoad({ path: good });
    assert.equal(ok.loader, "jsx");
    assert.equal(ok.resolveDir, dir);
    assert.match(ok.contents, /@jsxRuntime automatic/);
    const failed = await onLoad({ path: bad });
    assert.equal(failed.errors.length, 1);
    assert.deepEqual(failed.errors[0].location, { file: bad });
    assert.match(failed.errors[0].text, /<A>/);
    assert.doesNotMatch(failed.errors[0].text, /bad\.mdx/, "esbuild adds the file itself");
  } finally {
    rmSync(dir, { recursive: true, force: true });
  }
});

test("rollup: enables Rollup's JSX transform unless set or running in Rolldown", () => {
  const plugin = rollup({ jsxImportSource: "preact" });
  assert.deepEqual(plugin.options.call(pluginContext(), { input: "a.mdx" }), {
    input: "a.mdx",
    jsx: {
      mode: "automatic",
      factory: "React.createElement",
      importSource: "preact",
      jsxImportSource: "preact/jsx-runtime",
    },
  });
  assert.equal(plugin.options.call(pluginContext(), { jsx: "preserve" }), null);
  assert.equal(plugin.options.call(pluginContext({ rolldownVersion: "1.0.0" }), {}), null);
});

test("rollup: transform returns a JSX module and reports errors", () => {
  const plugin = rollup();
  const context = pluginContext();
  assert.equal(plugin.transform.call(context, "x", "a.js"), null);
  const result = plugin.transform.call(context, SOURCE, "/a/doc.mdx?v=1");
  assert.equal(result.moduleType, "jsx");
  assert.match(result.code, /@jsxImportSource react/);
  assert.throws(() => plugin.transform.call(context, BROKEN, "/a/bad.mdx"), /\/a\/bad\.mdx: /);
});

test("webpack: loader compiles with rule options and caches one compiler per rule", () => {
  const calls = [];
  const options = { jsxImportSource: "preact" };
  const context = (query, resourcePath) => ({
    query,
    resourcePath,
    getOptions() {
      calls.push(query);
      return query === options ? options : {};
    },
  });
  assert.match(webpack.call(context(options, "/a.mdx"), SOURCE), /@jsxImportSource preact/);
  assert.match(webpack.call(context(options, "/b.mdx"), SOURCE), /@jsxImportSource preact/);
  assert.match(webpack.call(context("", "/c.mdx"), SOURCE), /@jsxImportSource react/);
  webpack.call(context("", "/d.mdx"), SOURCE);
  assert.deepEqual(calls, [options, ""]);
  assert.throws(() => webpack.call(context("", "/bad.mdx"), BROKEN), /\/bad\.mdx: /);
});

const playgroundVite = existsSync(join(ROOT, "playground/node_modules/vite/package.json"));

test(
  "vite: transforms JSX with the project's Vite",
  { skip: !playgroundVite && "playground dependencies are not installed" },
  async () => {
    const plugin = vite();
    plugin.configResolved({ root: join(ROOT, "playground") });
    const context = pluginContext();
    assert.equal(await plugin.transform.call(context, "x", "/a.js"), null);
    const { code } = await plugin.transform.call(context, SOURCE, "/a/doc.mdx?import");
    assert.match(code, /from "react\/jsx-runtime"/);
    assert.doesNotMatch(code, /<X|<h1/);
    await assert.rejects(plugin.transform.call(context, BROKEN, "/a/bad.mdx"), /\/a\/bad\.mdx: /);
  },
);

test("astro: picks the JSX import source from the UI integration", () => {
  const setup = (integrations, options) => {
    const warnings = [];
    const updates = [];
    astro(options).hooks["astro:config:setup"]({
      config: { integrations: integrations.map((name) => ({ name })) },
      updateConfig: (config) => updates.push(config),
      logger: { warn: (message) => warnings.push(message) },
    });
    return { warnings, plugins: updates.flatMap((update) => update.vite.plugins) };
  };
  const react = setup(["@astrojs/react"]);
  assert.deepEqual(react.warnings, []);
  assert.deepEqual(
    react.plugins.map((plugin) => plugin.name),
    ["ironmark:mdx"],
  );
  assert.match(setup(["@astrojs/preact", "@astrojs/mdx"]).warnings[0], /@astrojs\/mdx/);
  assert.equal(setup([], { jsxImportSource: "react" }).plugins.length, 1);
  assert.throws(() => setup(["@astrojs/vue"]), /add @astrojs\/react or @astrojs\/preact/);
});
