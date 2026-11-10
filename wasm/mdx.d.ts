// `ironmark/mdx`: MDX → JSX (plus HTML rendering), without the AST, ANSI or HTML-parser code.
import type { MdxDocument, IronmarkApi } from "./index.js";

export type {
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
  JsxDocumentMethods,
  JsxOptions,
  JsxOutput,
  JsxRenderOptions,
  MarkdownInput,
  MdxDocument,
  ParseOptions,
  PresetName,
  TocEntry,
} from "./index.js";
export {
  CAPABILITIES,
  DEFAULT_OPTIONS,
  markdownToHtml,
  markdownToText,
  mdxToJsx,
  PRESETS,
} from "./index.js";

/** Load the WASM binary. Resolves at once under Node; must be awaited in the browser. */
export declare function init(input?: string | URL | WebAssembly.Module): Promise<void>;

/** Chainable API: `ironmark.parse(md).html()`. */
declare const ironmark: IronmarkApi<MdxDocument>;
export default ironmark;
