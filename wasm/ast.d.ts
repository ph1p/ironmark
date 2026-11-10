// `ironmark/ast`: Markdown → AST and AST → Markdown (plus HTML rendering), without ANSI, MDX or HTML-parser code.
import type { AstDocument, IronmarkApi } from "./index.js";

export type {
  AstDocument,
  AstDocumentMethods,
  AstNode,
  AstSummary,
  BaseDocument,
  DeepReadonly,
  DocumentStats,
  ResolvedOptions,
  ShortcutName,
  Shortcuts,
  Capabilities,
  HeadingInfo,
  ImageInfo,
  IronmarkApi,
  IronmarkBase,
  LinkInfo,
  ListKind,
  MarkdownAst,
  MarkdownInput,
  ParseOptions,
  PresetName,
  TableAlignment,
  TableData,
  TocEntry,
} from "./index.js";
export {
  AST_SCHEMA_VERSION,
  astToMarkdown,
  CAPABILITIES,
  DEFAULT_OPTIONS,
  extractHeadings,
  markdownToAst,
  markdownToHtml,
  markdownToText,
  PRESETS,
  summarizeAst,
} from "./index.js";

/** Load the WASM binary. Resolves at once under Node; must be awaited in the browser. */
export declare function init(input?: string | URL | WebAssembly.Module): Promise<void>;

/** Chainable API: `ironmark.parse(md).html()`. */
declare const ironmark: IronmarkApi<AstDocument>;
export default ironmark;
