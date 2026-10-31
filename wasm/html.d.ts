import type { MarkdownInput, RenderHtmlOptions } from "./index.js";

export type { MarkdownInput, ParseOptions, PresetName, RenderHtmlOptions } from "./index.js";

/** Prototype: Node is synchronous; browser initialization must be awaited. */
export declare function init(input?: string | URL | WebAssembly.Module): void | Promise<void>;

/** Render Markdown as HTML using the same options and limits as the full entry. */
export declare function renderHtml(input: MarkdownInput, options?: RenderHtmlOptions): string;
