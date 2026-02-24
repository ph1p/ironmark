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
export declare function parse(markdown: MarkdownInput, options?: ParseOptions): string;
