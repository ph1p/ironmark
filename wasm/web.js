import * as wasmGlue from "./pkg/ironmark_bg.js";
import {
  createParseMarkdown,
  createRenderHtml,
  createRenderMarkdown,
  createRenderAnsiTerminal,
  createRenderJsx,
  createParseHtmlToAst,
  createHtmlToMarkdown,
  getCapabilities,
  getAstSchemaVersion,
  getDefaultOptions,
  getPresets,
  extractHeadings,
  summarizeAst,
} from "./shared.js";

let initialized = false;

export async function init(input) {
  if (initialized) return;
  const imports = { "./ironmark_bg.js": wasmGlue };

  const url =
    input instanceof URL || typeof input === "string"
      ? input
      : new URL("./pkg/ironmark_bg.wasm", import.meta.url);

  const instantiateResult =
    typeof input === "object" && input instanceof WebAssembly.Module
      ? await WebAssembly.instantiate(input, imports)
      : typeof WebAssembly.instantiateStreaming === "function"
        ? await WebAssembly.instantiateStreaming(fetch(url), imports)
        : await WebAssembly.instantiate(await fetch(url).then((r) => r.arrayBuffer()), imports);
  const instance =
    instantiateResult instanceof WebAssembly.Instance
      ? instantiateResult
      : instantiateResult.instance;

  wasmGlue.__wbg_set_wasm(instance.exports);
  initialized = true;
}

export const parseMarkdown = createParseMarkdown(wasmGlue.parseToAst);
export const renderHtml = createRenderHtml(wasmGlue.parse, wasmGlue.parseDefault);
export const renderMarkdown = createRenderMarkdown(wasmGlue.renderMarkdown);
export const renderAnsiTerminal = createRenderAnsiTerminal(wasmGlue.renderAnsi);
export const renderJsx = createRenderJsx(wasmGlue.renderJsx);
export const parseHtmlToAst = createParseHtmlToAst(wasmGlue.parseHtmlToAst);
export const htmlToMarkdown = createHtmlToMarkdown(wasmGlue.htmlToMarkdown);

export { getCapabilities, getAstSchemaVersion, getDefaultOptions, getPresets };
export { extractHeadings, summarizeAst };
