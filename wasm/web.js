import * as wasmGlue from "./pkg/ironmark_bg.js";
import {
  instantiateWasm,
  createMarkdownToAst,
  createMarkdownToHtml,
  createAstToMarkdown,
  createMarkdownToAnsi,
  createMdxToJsx,
  createHtmlToAst,
  createHtmlToMarkdown,
  ALL_FORMATS,
  createCapabilities,
  AST_SCHEMA_VERSION,
  DEFAULT_OPTIONS,
  PRESETS,
  extractHeadings,
  summarizeAst,
} from "./shared.js";

let initialized = false;

export async function init(input) {
  if (initialized) return;
  const url = new URL("./pkg/ironmark_bg.wasm", import.meta.url);
  await instantiateWasm(wasmGlue, "./ironmark_bg.js", input, url);
  initialized = true;
}

export const markdownToAst = createMarkdownToAst(wasmGlue.parseToAst);
export const markdownToHtml = createMarkdownToHtml(wasmGlue.parse, wasmGlue.parseDefault);
export const astToMarkdown = createAstToMarkdown(wasmGlue.renderMarkdown);
export const markdownToAnsi = createMarkdownToAnsi(wasmGlue.renderAnsi);
export const mdxToJsx = createMdxToJsx(wasmGlue.renderJsx);
export const htmlToAst = createHtmlToAst(wasmGlue.parseHtmlToAst);
export const htmlToMarkdown = createHtmlToMarkdown(wasmGlue.htmlToMarkdown);

export const CAPABILITIES = createCapabilities(ALL_FORMATS);
export { AST_SCHEMA_VERSION, DEFAULT_OPTIONS, PRESETS };
export { extractHeadings, summarizeAst };
