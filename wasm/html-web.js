// Prototype only: this entry has no public npm subpath yet.
import * as wasmGlue from "./pkg-html/ironmark_html_bg.js";
import { createRenderHtml } from "./shared.js";

let initialized = false;

export async function init(input) {
  if (initialized) return;
  const imports = { "./ironmark_html_bg.js": wasmGlue };
  const url =
    input instanceof URL || typeof input === "string"
      ? input
      : new URL("./pkg-html/ironmark_html_bg.wasm", import.meta.url);
  const result =
    typeof input === "object" && input instanceof WebAssembly.Module
      ? await WebAssembly.instantiate(input, imports)
      : typeof WebAssembly.instantiateStreaming === "function"
        ? await WebAssembly.instantiateStreaming(fetch(url), imports)
        : await WebAssembly.instantiate(await fetch(url).then((r) => r.arrayBuffer()), imports);
  const instance = result instanceof WebAssembly.Instance ? result : result.instance;
  wasmGlue.__wbg_set_wasm(instance.exports);
  initialized = true;
}

export const renderHtml = createRenderHtml(wasmGlue.parse, wasmGlue.parseDefault);
