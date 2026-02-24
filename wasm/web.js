import { __wbg_set_wasm, parse as wasmParse } from "./pkg/ironmark_bg.js";
import { createParse } from "./shared.js";

let initialized = false;

export async function init(input) {
  if (initialized) return;

  const url =
    input instanceof URL || typeof input === "string"
      ? input
      : new URL("./pkg/ironmark_bg.wasm", import.meta.url);

  const { instance } =
    typeof input === "object" && input instanceof WebAssembly.Module
      ? await WebAssembly.instantiate(input, {})
      : typeof WebAssembly.instantiateStreaming === "function"
        ? await WebAssembly.instantiateStreaming(fetch(url), {})
        : await WebAssembly.instantiate(await fetch(url).then((r) => r.arrayBuffer()), {});

  __wbg_set_wasm(instance.exports);
  initialized = true;
}

export const parse = createParse(wasmParse);
