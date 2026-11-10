// `ironmark/mdx`: MDX → JSX (plus HTML rendering), without the AST, ANSI or HTML-parser code.
export type {
  Capabilities,
  JsxOptions,
  JsxOutput,
  MarkdownInput,
  ParseOptions,
  PresetName,
} from "./index.js";
export { CAPABILITIES, DEFAULT_OPTIONS, markdownToHtml, mdxToJsx, PRESETS } from "./index.js";

/** Load the WASM binary. A no-op under Node; must be awaited in the browser. */
export declare function init(input?: string | URL | WebAssembly.Module): void | Promise<void>;
