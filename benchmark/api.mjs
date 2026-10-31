/** Warm public-API measurements, intentionally separate from competitor history. */
import { readFileSync } from "node:fs";
import { join, resolve } from "node:path";
import { pathToFileURL } from "node:url";
import {
  ROOT,
  captureEnv,
  collectSamples,
  genCodeBlocks,
  genInlineHeavy,
  genManyRefLinks,
  genMixedDoc,
  genTable,
  loadAllFeatures,
} from "./generators.mjs";
import { entryUrl, parseArgs, summarize } from "./measure.mjs";

// Matches the explicit GFM profile in js-bench.mjs, without changing competitors.
export const GFM = Object.freeze({
  hardBreaks: false,
  enableHighlight: false,
  enableUnderline: false,
  enableStrikethrough: true,
  enableTables: true,
  enableTaskLists: true,
  enableAutolink: true,
});

export function apiCases() {
  return [
    ["repo/README.md", readFileSync(join(ROOT, "README.md"), "utf8"), "repository"],
    ["repo/benchmark/all_features.md", loadAllFeatures(), "repository"],
    ["synthetic/empty", "", "synthetic"],
    ["synthetic/short-plain", "Hello, world.\n", "synthetic"],
    ["synthetic/short-inline", "**bold** and *em* and `code`\n", "synthetic"],
    ["synthetic/inline-heavy", genInlineHeavy(50), "synthetic"],
    ["synthetic/references", genManyRefLinks(50), "synthetic"],
    ["synthetic/code-blocks", genCodeBlocks(20), "synthetic"],
    ["synthetic/mixed", genMixedDoc(100), "synthetic"],
    ...[1, 8, 9, 10].map((cols) => [
      `synthetic/table-${cols}-cols`,
      genTable(8, cols),
      "synthetic",
    ]),
    [
      "synthetic/table-discarded-escaped-pipe",
      "| h |\n| --- |\n| a\\|b | discarded | extra |\n",
      "synthetic",
    ],
  ];
}

export function measureApis(api, cases = apiCases(), sample = collectSamples) {
  const empty = {};
  const variants = [
    ["html-default", "renderHtml", (input) => api.renderHtml(input), "omitted"],
    ["html-empty-options", "renderHtml", (input) => api.renderHtml(input, empty), empty],
    ["html-gfm", "renderHtml", (input) => api.renderHtml(input, GFM), GFM],
    ["ast-default", "parseMarkdown", (input) => api.parseMarkdown(input), "omitted"],
    ["ansi-default", "renderAnsiTerminal", (input) => api.renderAnsiTerminal(input), "omitted"],
  ];
  return variants.map(([name, exported, run, options]) => {
    if (typeof api[exported] !== "function") {
      return { api: name, status: "unsupported", missing_export: exported };
    }
    return {
      api: name,
      status: "supported",
      options,
      cases: cases.map(([label, input, kind]) => ({
        case: label,
        kind,
        input_bytes: Buffer.byteLength(input),
        ...summarize(sample(run, input)),
      })),
    };
  });
}

export async function measureApi(args = process.argv.slice(2)) {
  const options = parseArgs(args, ["--entry"]);
  const entry = entryUrl(options["--entry"] ?? join(ROOT, "wasm/node.js"));
  const api = await import(entry);
  if (typeof api.renderHtml !== "function") throw new Error("Entry must export renderHtml");
  return { schema: 1, env: captureEnv(), entry, mode: "warm-api", results: measureApis(api) };
}

if (process.argv[1] && import.meta.url === pathToFileURL(resolve(process.argv[1])).href) {
  try {
    console.log(JSON.stringify(await measureApi(), null, 2));
  } catch (error) {
    console.error(error.message);
    process.exitCode = 1;
  }
}
