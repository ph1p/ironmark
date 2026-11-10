// `ironmark/html`: Markdown → HTML only, on the smallest WASM binary.
import type { HtmlDocument, IronmarkApi } from "./index.js";

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
  HtmlDocument,
  IronmarkApi,
  IronmarkBase,
  LinkInfo,
  MarkdownInput,
  ParseOptions,
  PresetName,
  TocEntry,
} from "./index.js";
export { CAPABILITIES, DEFAULT_OPTIONS, markdownToHtml, markdownToText, PRESETS } from "./index.js";

/** Load the WASM binary. Resolves at once under Node; must be awaited in the browser. */
export declare function init(input?: string | URL | WebAssembly.Module): Promise<void>;

/** Chainable API: `ironmark.parse(md).html()`. */
declare const ironmark: IronmarkApi<HtmlDocument>;
export default ironmark;
