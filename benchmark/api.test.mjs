import assert from "node:assert/strict";
import { mkdirSync, mkdtempSync, rmSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import test from "node:test";
import { ROOT } from "./generators.mjs";
import { GFM, apiCases, measureApi, measureApis } from "./api.mjs";

test("API variants distinguish omitted, empty, and explicit GFM options", () => {
  const calls = [];
  const api = {
    renderHtml(...args) {
      calls.push(["html", ...args]);
      return "<p>x</p>";
    },
    parseMarkdown(...args) {
      calls.push(["ast", ...args]);
      return {};
    },
    renderAnsiTerminal(...args) {
      calls.push(["ansi", ...args]);
      return "x";
    },
  };
  const results = measureApis(api, [["test", "é", "synthetic"]], (fn, input) => {
    fn(input);
    return [3, 1, 2];
  });
  assert.equal(results.length, 5);
  assert.ok(results.every((r) => r.status === "supported"));
  assert.deepEqual(calls[0], ["html", "é"]);
  assert.deepEqual(calls[1], ["html", "é", {}]);
  assert.equal(calls[2][2], GFM);
  assert.deepEqual(calls[3], ["ast", "é"]);
  assert.deepEqual(calls[4], ["ansi", "é"]);
  for (const r of results) {
    assert.equal(r.cases[0].input_bytes, 2);
    assert.equal(r.cases[0].median_ms, 2);
  }
});

test("missing AST/ANSI are unsupported with no fabricated timings", () => {
  const result = measureApis({ renderHtml: () => "x" }, [["test", "", "synthetic"]], () => [1]);
  for (const r of result.slice(3)) {
    assert.equal(r.status, "unsupported");
    assert.equal(r.cases, undefined);
    assert.ok(r.missing_export);
  }
  assert.throws(
    () =>
      measureApis(
        {
          renderHtml: () => {
            throw Error("render failed");
          },
        },
        [["x", "", "synthetic"]],
        (fn, input) => {
          fn(input);
          return [1];
        },
      ),
    /render failed/,
  );
});

test("corpus labels repository files distinctly from generated cases", () => {
  const cases = apiCases();
  assert.equal(new Set(cases.map(([name]) => name)).size, cases.length);
  assert.deepEqual(
    cases.filter(([, , kind]) => kind === "repository").map(([name]) => name),
    ["repo/README.md", "repo/benchmark/all_features.md"],
  );
  for (const [, input, kind] of cases) {
    assert.equal(typeof input, "string");
    assert.ok(["repository", "synthetic"].includes(kind));
  }
});

test("--entry selects an alternate module; missing entry/export fails", async (t) => {
  mkdirSync(join(ROOT, "target"), { recursive: true });
  const dir = mkdtempSync(join(ROOT, "target/api-test-"));
  t.after(() => rmSync(dir, { recursive: true, force: true }));
  const entry = join(dir, "alternate.mjs");
  writeFileSync(entry, "export function renderHtml() { return 'alternate'; }\n");
  const report = await measureApi(["--entry", entry]);
  assert.ok(report.entry.endsWith("/alternate.mjs"));
  assert.equal(report.results[3].status, "unsupported");
  await assert.rejects(measureApi(["--entry", join(dir, "missing.mjs")]), /Missing entry/);
  const invalid = join(dir, "invalid.mjs");
  writeFileSync(invalid, "export const unrelated = 1;\n");
  await assert.rejects(measureApi(["--entry", invalid]), /must export renderHtml/);
});
