export type MarkdownInput = string | Uint8Array | ArrayBuffer | ArrayBufferView;

export interface ParseOptions {
  /** When true, every newline in a paragraph becomes a hard line break (`<br />`). Default: true. */
  hardBreaks?: boolean;
  /** Enable ==highlight== syntax for `<mark>`. Default: true. */
  enableHighlight?: boolean;
  /** Enable ~~strikethrough~~ syntax for `<del>`. Default: true. */
  enableStrikethrough?: boolean;
  /** Enable ++underline++ syntax for `<u>`. Default: true. */
  enableUnderline?: boolean;
  /** Enable pipe table syntax. Default: true. */
  enableTables?: boolean;
  /** Automatically detect bare URLs and emails and wrap them in links. Default: true. */
  enableAutolink?: boolean;
  /** Enable GitHub-style task lists (`- [ ] unchecked`, `- [x] checked`). Default: true. */
  enableTaskLists?: boolean;
}

/**
 * Parse Markdown to HTML.
 *
 * @param markdown - Markdown source (string or binary).
 * @param options - Optional parsing options.
 * @returns HTML string.
 */
/**
 * Initialize the WASM module.
 *
 * - **Node.js**: This is a no-op — WASM is embedded and loaded synchronously at import time.
 * - **Browser/Bundler**: Must be called (and awaited) before using `parse()`.
 *   Optionally pass a URL or `WebAssembly.Module` to override the default WASM location.
 *   Calling `init()` multiple times is safe (subsequent calls are no-ops).
 */
export declare function init(input?: string | URL | WebAssembly.Module): Promise<void>;

/**
 * Parse Markdown to HTML.
 *
 * @param markdown - Markdown source (string or binary).
 * @param options - Optional parsing options.
 * @returns HTML string.
 */
export declare function parse(markdown: MarkdownInput, options?: ParseOptions): string;

/**
 * Parse Markdown and return the block-level AST as a JSON string.
 *
 * @param markdown - Markdown source (string or binary).
 * @param options - Optional parsing options.
 * @returns JSON string representing the AST.
 */
export declare function parseToAst(markdown: MarkdownInput, options?: ParseOptions): string;
