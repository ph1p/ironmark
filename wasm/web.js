import * as wasmGlue from "./pkg/ironmark_bg.js";
import { createParse, createParseToAst } from "./shared.js";

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

export const parse = createParse(wasmGlue.parse);
export const parseToAst = createParseToAst(wasmGlue.parseToAst);
