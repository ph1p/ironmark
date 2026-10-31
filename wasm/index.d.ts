// ─── Input types ─────────────────────────────────────────────────────────────

export type MarkdownInput = string | Uint8Array | ArrayBuffer | ArrayBufferView;

// ─── Preset names ─────────────────────────────────────────────────────────────

/**
 * Named option presets for common use cases.
 *
 * - `"default"` — Default ironmark behavior; all extensions enabled.
 * - `"safe"` — Disables raw HTML and enables the GFM tag filter. Use for untrusted input.
 * - `"strict"` — CommonMark-only; disables extensions and restricts permissive behaviors.
 * - `"llm"` — Deterministic, structure-first output optimized for AI pipelines and agents.
 *   Disables autolink, wiki links, math, hard breaks, and raw HTML; enables heading IDs
 *   and whitespace normalization.
 */
export type PresetName = "default" | "safe" | "strict" | "llm";

// ─── Parse options ────────────────────────────────────────────────────────────

export interface ParseOptions {
  /**
   * Apply a named preset before applying any explicit options.
   * Explicit options always override the preset.
   */
  preset?: PresetName;

  /**
   * Enable safe rendering. Equivalent to `{ disableRawHtml: true, tagFilter: true }`.
   * Explicit `disableRawHtml` / `tagFilter` values override this.
   */
  safe?: boolean;

  /**
   * When true, output is normalized for deterministic comparison:
   * whitespace is collapsed and output is stable across runs.
   */
  deterministic?: boolean;

  /**
   * Reserved for future use. When true, the AST will include source position
   * metadata. Has no effect in the current version.
   */
  stableAst?: boolean;

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
  /** When true, raw HTML blocks and inline HTML are both escaped (XSS prevention). Default: false. */
  disableRawHtml?: boolean;
  /** Auto-generate `id=` attributes on headings from their slugified text. Default: false. */
  enableHeadingIds?: boolean;
  /** Render an `<a class="anchor">` inside each heading (implies heading IDs). Default: false. */
  enableHeadingAnchors?: boolean;
  /** When false, 4-space-indented code blocks are disabled (treated as paragraphs). Default: true. */
  enableIndentedCodeBlocks?: boolean;
  /** Disable HTML block constructs (escape them as text). Default: false. */
  noHtmlBlocks?: boolean;
  /** Disable inline HTML spans (escape them as text). Default: false. */
  noHtmlSpans?: boolean;
  /** Enable GFM tag filter: escape dangerous tags like `<script>`, `<iframe>`, etc. Default: false. */
  tagFilter?: boolean;
  /** Collapse runs of spaces/tabs in text nodes to a single space. Default: false. */
  collapseWhitespace?: boolean;
  /** Allow ATX headings without a space after `#` (e.g. `#Heading`). Default: false. */
  permissiveAtxHeaders?: boolean;
  /** Enable `[[wiki link]]` syntax → `<a href="...">`. Default: false. */
  enableWikiLinks?: boolean;
  /** Enable `$...$` and `$$...$$` math syntax with HTML-escaped content. Default: false. */
  enableLatexMath?: boolean;
}

// ─── Render-specific option aliases ──────────────────────────────────────────

/** Options for `renderHtml()`. Same as `ParseOptions`. */
export type RenderHtmlOptions = ParseOptions;

// ─── ANSI options ─────────────────────────────────────────────────────────────

export interface AnsiOptions {
  /**
   * Terminal column width for word-wrap, heading underlines, and thematic breaks.
   * Set to `0` to disable all width-dependent formatting. Default: `80`.
   */
  width?: number;
  /**
   * Emit ANSI 256-colour escape codes. Set to `false` for plain-text output
   * (e.g. when piping to a file or a non-colour terminal). Default: `true`.
   */
  color?: boolean;
  /**
   * Show line numbers in fenced code blocks, right-aligned to the total line
   * count and separated from the code by a `│` border. Default: `false`.
   */
  lineNumbers?: boolean;
  /**
   * Horizontal padding to add on both sides of every output line.
   * Also adds `ceil(padding / 2)` blank lines at the top. Default: `0`.
   */
  padding?: number;
}

// ─── HTML parse options ───────────────────────────────────────────────────────

export interface HtmlParseOptions {
  /**
   * If true, unknown HTML tags (like `<sup>`, `<sub>`, `<abbr>`) are preserved
   * as raw HTML in the Markdown output. If false (default), unknown tags are
   * stripped but their text content is kept.
   */
  preserveUnknownAsHtml?: boolean;
}

// ─── AST node types ───────────────────────────────────────────────────────────

/**
 * A single AST node. The `t` field is the type discriminant.
 * Child nodes are in `c`; leaf text content is in `text`.
 */
export interface AstNode {
  t: string;
  c?: AstNode[];
  text?: string;
  [key: string]: unknown;
}

/** Parsed AST: an array of top-level block nodes returned by `parseMarkdown()`. */
export type MarkdownAst = AstNode[];

// ─── Introspection return types ───────────────────────────────────────────────

export interface Capabilities {
  astSchemaVersion: string;
  formats: string[];
  presets: PresetName[];
  extensions: string[];
  security: string[];
}

export interface HeadingInfo {
  level: number;
  text: string;
  id: string;
}

export interface AstSummary {
  blockCount: number;
  nodeCounts: Record<string, number>;
}

// ─── Initialization ───────────────────────────────────────────────────────────

/**
 * Initialize the WASM module.
 *
 * - **Node.js**: no-op — packaged WASM is read from disk synchronously at import time.
 * - **Browser/Bundler**: must be awaited before calling any other function.
 *   Accepts a URL or `WebAssembly.Module` to override the default WASM location.
 *   Safe to call multiple times.
 *
 * @example
 * ```ts
 * import { init, renderHtml } from "ironmark";
 * await init();
 * const html = renderHtml("# Hello");
 * ```
 */
export declare function init(input?: string | URL | WebAssembly.Module): Promise<void>;

// ─── Core API ─────────────────────────────────────────────────────────────────

/**
 * Parse Markdown and return the structured AST as a JavaScript object.
 *
 * The recommended entry point for AI pipelines, agents, and any workflow
 * that needs to inspect or transform document structure.
 *
 * @param input - Markdown source.
 * @param options - Optional parse options or preset.
 * @returns Parsed AST — a JavaScript array, no `JSON.parse()` needed.
 *
 * @example
 * ```ts
 * import { parseMarkdown } from "ironmark";
 *
 * const ast = parseMarkdown("# Hello\n\n**World**");
 * ```
 *
 * @example With a preset
 * ```ts
 * const ast = parseMarkdown(userInput, { preset: "llm" });
 * ```
 */
export declare function parseMarkdown(input: MarkdownInput, options?: ParseOptions): MarkdownAst;

/**
 * Render Markdown to an HTML string.
 *
 * Use `safe: true` or `preset: "safe"` for any untrusted input.
 *
 * @param input - Markdown source.
 * @param options - Optional render options.
 * @returns HTML string.
 *
 * @example
 * ```ts
 * import { renderHtml } from "ironmark";
 *
 * const html = renderHtml("# Hello");
 * const safeHtml = renderHtml(userInput, { safe: true });
 * ```
 */
export declare function renderHtml(input: MarkdownInput, options?: RenderHtmlOptions): string;

/**
 * Render an AST back to a Markdown string.
 *
 * Pass the result of `parseMarkdown()` directly — accepts an AST object or
 * a JSON string. Useful for normalizing Markdown or round-trip conversion.
 *
 * @param ast - AST from `parseMarkdown()`, or a JSON string.
 * @returns Markdown string.
 *
 * @example
 * ```ts
 * import { parseMarkdown, renderMarkdown } from "ironmark";
 *
 * const ast = parseMarkdown("**Hello**");
 * const md = renderMarkdown(ast);
 * ```
 */
export declare function renderMarkdown(ast: MarkdownAst | string): string;

/**
 * Render Markdown as ANSI-coloured terminal output.
 *
 * Produces a string with ANSI 256-colour escape codes suitable for TTY display.
 *
 * @param input - Markdown source.
 * @param options - Optional parse options.
 * @param ansiOptions - Optional ANSI rendering options.
 * @returns String with ANSI escape codes.
 *
 * @example
 * ```ts
 * import { renderAnsiTerminal } from "ironmark";
 *
 * process.stdout.write(renderAnsiTerminal("# Hello\n\n**bold**"));
 * ```
 */
export declare function renderAnsiTerminal(
  input: MarkdownInput,
  options?: ParseOptions,
  ansiOptions?: AnsiOptions,
): string;

/**
 * Parse an HTML string and return the AST as a JavaScript object.
 *
 * Converts HTML into the same AST structure used by the Markdown parser,
 * enabling HTML → Markdown conversion via `renderMarkdown()`.
 *
 * @param html - HTML source string.
 * @param preserveUnknownAsHtml - If true, unknown HTML tags are preserved as raw HTML.
 * @returns Parsed AST object.
 *
 * @example
 * ```ts
 * import { parseHtmlToAst, renderMarkdown } from "ironmark";
 *
 * const ast = parseHtmlToAst("<h1>Hello</h1><p>World</p>");
 * const md = renderMarkdown(ast);
 * ```
 */
export declare function parseHtmlToAst(html: string, preserveUnknownAsHtml?: boolean): MarkdownAst;

/**
 * Convert HTML to Markdown.
 *
 * @param html - HTML source string.
 * @param preserveUnknownAsHtml - If true, unknown HTML tags are preserved as raw HTML.
 * @returns Markdown string.
 *
 * @example
 * ```ts
 * import { htmlToMarkdown } from "ironmark";
 *
 * const md = htmlToMarkdown("<p><strong>Bold</strong> text</p>");
 * // Returns: "**Bold** text"
 * ```
 */
export declare function htmlToMarkdown(html: string, preserveUnknownAsHtml?: boolean): string;

// ─── Introspection helpers ────────────────────────────────────────────────────

/**
 * Return machine-readable metadata about this ironmark build.
 *
 * @example
 * ```ts
 * import { getCapabilities } from "ironmark";
 * const caps = getCapabilities();
 * // { astSchemaVersion: "2", formats: [...], presets: [...], ... }
 * ```
 */
export declare function getCapabilities(): Capabilities;

/**
 * Return the current AST schema version string.
 * An increment signals a breaking change to the AST node shape.
 */
export declare function getAstSchemaVersion(): string;

/**
 * Return the resolved default `ParseOptions` — the effective value of every
 * option when none are specified.
 */
export declare function getDefaultOptions(): Required<
  Omit<ParseOptions, "preset" | "safe" | "deterministic" | "stableAst">
>;

/**
 * Return all named presets and their resolved option objects.
 *
 * @example
 * ```ts
 * import { getPresets } from "ironmark";
 * const { llm } = getPresets();
 * ```
 */
export declare function getPresets(): Record<PresetName, Partial<ParseOptions>>;

// ─── Utility functions ────────────────────────────────────────────────────────

/**
 * Extract all headings from a parsed AST.
 *
 * Returns level, plain-text content, and a slugified `id` for each heading.
 *
 * @example
 * ```ts
 * import { parseMarkdown, extractHeadings } from "ironmark";
 *
 * const headings = extractHeadings(parseMarkdown("# Hello\n\n## World"));
 * // [{ level: 1, text: "Hello", id: "hello" }, { level: 2, text: "World", id: "world" }]
 * ```
 */
export declare function extractHeadings(ast: MarkdownAst): HeadingInfo[];

/**
 * Summarize an AST: count top-level blocks and all node types.
 *
 * @example
 * ```ts
 * import { parseMarkdown, summarizeAst } from "ironmark";
 *
 * const summary = summarizeAst(parseMarkdown("# Hello\n\nA paragraph.\n\n- item"));
 * // { blockCount: 3, nodeCounts: { Heading: 1, Paragraph: 1, List: 1, ... } }
 * ```
 */
export declare function summarizeAst(ast: MarkdownAst): AstSummary;
