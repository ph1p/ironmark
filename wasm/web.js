import * as wasmGlue from "./pkg/ironmark_bg.js";
import {
  instantiateWasm,
  createMarkdownToAst,
  createMarkdownToHtml,
  createAstToMarkdown,
  createMarkdownToText,
  createMarkdownHeadings,
  createMarkdownLinks,
  createMarkdownFrontmatter,
  createNormalizeMarkdown,
  createMarkdownToAnsi,
  createMdxToJsx,
  createHtmlToAst,
  createHtmlToMarkdown,
  ALL_FORMATS,
  createCapabilities,
  createIronmark,
  AST_SCHEMA_VERSION,
  DEFAULT_OPTIONS,
  PRESETS,
  extractHeadings,
  summarizeAst,
} from "./shared.js";

let initializing;

/** Load the WASM binary once; later calls (and `ironmark.ready`) share the same promise. */
export function init(input) {
  const url = new URL("./pkg/ironmark_bg.wasm", import.meta.url);
  initializing ??= instantiateWasm(wasmGlue, "./ironmark_bg.js", input, url).catch((error) => {
    initializing = undefined;
    throw error;
  });
  return initializing;
}

export const markdownToAst = /* @__PURE__ */ createMarkdownToAst(wasmGlue.parseToAst);
export const markdownToHtml = /* @__PURE__ */ createMarkdownToHtml(
  wasmGlue.parse,
  wasmGlue.parseDefault,
);
export const astToMarkdown = /* @__PURE__ */ createAstToMarkdown(wasmGlue.renderMarkdown);
export const markdownToAnsi = /* @__PURE__ */ createMarkdownToAnsi(wasmGlue.renderAnsi);
export const mdxToJsx = /* @__PURE__ */ createMdxToJsx(wasmGlue.renderJsx);
export const htmlToAst = /* @__PURE__ */ createHtmlToAst(wasmGlue.parseHtmlToAst);
export const htmlToMarkdown = /* @__PURE__ */ createHtmlToMarkdown(wasmGlue.htmlToMarkdown);
export const markdownToText = /* @__PURE__ */ createMarkdownToText(wasmGlue.renderText);
const markdownHeadings = /* @__PURE__ */ createMarkdownHeadings(wasmGlue.extractHeadings);
const normalizeMarkdown = /* @__PURE__ */ createNormalizeMarkdown(wasmGlue.normalizeMarkdown);
const markdownLinks = /* @__PURE__ */ createMarkdownLinks(wasmGlue.extractLinks);
const markdownFrontmatter = /* @__PURE__ */ createMarkdownFrontmatter(wasmGlue.frontmatter);

export const CAPABILITIES = /* @__PURE__ */ createCapabilities(ALL_FORMATS);
export { AST_SCHEMA_VERSION, DEFAULT_OPTIONS, PRESETS };
export { extractHeadings, summarizeAst };

export default /* @__PURE__ */ createIronmark({
  init,
  markdownToHtml,
  markdownToText,
  markdownHeadings,
  markdownLinks,
  markdownFrontmatter,
  normalizeMarkdown,
  markdownToAst,
  astToMarkdown,
  markdownToAnsi,
  mdxToJsx,
  htmlToMarkdown,
});
