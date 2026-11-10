// `ironmark/html`: Markdown → HTML only, on the smallest WASM binary.
export type { Capabilities, MarkdownInput, ParseOptions, PresetName } from "./index.js";
export { CAPABILITIES, DEFAULT_OPTIONS, markdownToHtml, PRESETS } from "./index.js";

/** Load the WASM binary. A no-op under Node; must be awaited in the browser. */
export declare function init(input?: string | URL | WebAssembly.Module): void | Promise<void>;
