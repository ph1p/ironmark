// `ironmark/ast`: Markdown → AST and AST → Markdown (plus HTML rendering), without ANSI, MDX or HTML-parser code.
export type {
  AstNode,
  AstSummary,
  Capabilities,
  HeadingInfo,
  ListKind,
  MarkdownAst,
  MarkdownInput,
  ParseOptions,
  PresetName,
  TableAlignment,
  TableData,
} from "./index.js";
export {
  AST_SCHEMA_VERSION,
  astToMarkdown,
  CAPABILITIES,
  DEFAULT_OPTIONS,
  extractHeadings,
  markdownToAst,
  markdownToHtml,
  PRESETS,
  summarizeAst,
} from "./index.js";

/** Load the WASM binary. A no-op under Node; must be awaited in the browser. */
export declare function init(input?: string | URL | WebAssembly.Module): void | Promise<void>;
