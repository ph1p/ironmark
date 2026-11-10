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
  /**
   * Parse MDX syntax: JSX tags (`<Card title="x">`), `{expressions}` and top-level
   * `import`/`export` blocks. Also disables indented code and HTML blocks, like MDX.
   * In `markdownToHtml()` the MDX pieces pass through verbatim; use `mdxToJsx()` to
   * compile to JSX. Only enable for trusted input: expressions are executable code.
   * Default: false.
   */
  enableMdx?: boolean;
}

// ─── ANSI options ─────────────────────────────────────────────────────────────

/** Options for `markdownToAnsi()`: the parse options plus terminal layout. */
export interface AnsiOptions extends ParseOptions {
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

// ─── JSX output ───────────────────────────────────────────────────────────────

/** Result of `mdxToJsx()`. */
export interface JsxOutput {
  /** Top-level `import`/`export` blocks, in document order. */
  esm: string;
  /** The document body as one JSX fragment expression (`<>…</>`). */
  jsx: string;
  /** A complete ES module: `esm` plus `export default function MDXContent(props)`. */
  module: string;
}

/** Options for `mdxToJsx()` on top of the parse options. */
export interface JsxOptions extends ParseOptions {
  /**
   * Route elements through a `_components` object, like `@mdx-js/mdx`: markdown tags
   * become `<_components.h1>` (overridable via `props.components`), components the
   * ESM does not declare are looked up there, and `components.wrapper` wraps the
   * content. Default: false.
   */
  components?: boolean;
  /**
   * Module to import `useMDXComponents` from (e.g. `"@mdx-js/react"`) so an
   * `MDXProvider` can supply components. Implies `components`. Default: none.
   */
  providerImportSource?: string;
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
//
// The AST is serde's externally tagged JSON: each node is an object with a single
// key naming its type (`{ Heading: { level: 1, raw: "Hi" } }`); `ThematicBreak` is
// the bare string `"ThematicBreak"`. `raw` and table cells hold unparsed inline
// Markdown.

export type ListKind = { Bullet: number } | { Ordered: number };
export type TableAlignment = "None" | "Left" | "Center" | "Right";

export interface TableData {
  alignments: TableAlignment[];
  num_cols: number;
  header: string[];
  /** Body cells in row-major order: row `r`, column `c` is `rows[r * num_cols + c]`. */
  rows: string[];
}

/** A block-level node below the document root. */
export type AstNode =
  | { BlockQuote: { children: AstNode[] } }
  | { List: { kind: ListKind; start: number; tight: boolean; children: AstNode[] } }
  | { ListItem: { children: AstNode[]; checked: boolean | null } }
  | { Paragraph: { raw: string } }
  | { Heading: { level: 1 | 2 | 3 | 4 | 5 | 6; raw: string } }
  | { CodeBlock: { info: string; literal: string } }
  | { HtmlBlock: { literal: string } }
  | { MdxFlow: { literal: string } }
  | { MdxEsm: { literal: string } }
  | { Table: TableData }
  | "ThematicBreak";

/** The root returned by `markdownToAst()` and `htmlToAst()`. */
export interface MarkdownAst {
  Document: { children: AstNode[] };
}

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
 * import { init, markdownToHtml } from "ironmark";
 * await init();
 * const html = markdownToHtml("# Hello");
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
 * @returns The `Document` root as a JavaScript object, no `JSON.parse()` needed.
 *
 * @example
 * ```ts
 * import { markdownToAst } from "ironmark";
 *
 * const ast = markdownToAst("# Hello\n\n**World**");
 * // { Document: { children: [{ Heading: { level: 1, raw: "Hello" } }, { Paragraph: { raw: "**World**" } }] } }
 * ```
 *
 * @example With a preset
 * ```ts
 * const ast = markdownToAst(userInput, { preset: "llm" });
 * ```
 */
export declare function markdownToAst(input: MarkdownInput, options?: ParseOptions): MarkdownAst;

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
 * import { markdownToHtml } from "ironmark";
 *
 * const html = markdownToHtml("# Hello");
 * const safeHtml = markdownToHtml(userInput, { safe: true });
 * ```
 */
export declare function markdownToHtml(input: MarkdownInput, options?: ParseOptions): string;

/**
 * Render an AST back to a Markdown string.
 *
 * Pass the result of `markdownToAst()` directly — accepts an AST object or
 * a JSON string. Useful for normalizing Markdown or round-trip conversion.
 *
 * @param ast - AST from `markdownToAst()`, or a JSON string.
 * @returns Markdown string.
 *
 * @example
 * ```ts
 * import { markdownToAst, astToMarkdown } from "ironmark";
 *
 * const ast = markdownToAst("**Hello**");
 * const md = astToMarkdown(ast);
 * ```
 */
export declare function astToMarkdown(ast: MarkdownAst | string): string;

/**
 * Render Markdown as ANSI-coloured terminal output.
 *
 * Produces a string with ANSI 256-colour escape codes suitable for TTY display.
 *
 * @param input - Markdown source.
 * @param options - Optional parse options plus `width`, `color`, `lineNumbers`, `padding`.
 * @returns String with ANSI escape codes.
 *
 * @example
 * ```ts
 * import { markdownToAnsi } from "ironmark";
 *
 * process.stdout.write(markdownToAnsi("# Hello\n\n**bold**", { width: 100 }));
 * ```
 */
export declare function markdownToAnsi(input: MarkdownInput, options?: AnsiOptions): string;

/**
 * Compile MDX to JSX.
 *
 * MDX parsing is always on (`enableMdx` is ignored). The result still needs a
 * JSX transform (esbuild, Babel, SWC, …) to run. Only use with trusted input:
 * `{expressions}` and `import`/`export` blocks are executable code.
 *
 * @param input - MDX source.
 * @param options - Optional parse options, plus `components` / `providerImportSource`.
 * @returns ESM, JSX body, and a ready-to-bundle module.
 * @throws Error when JSX tags are unbalanced or unterminated.
 *
 * @example
 * ```ts
 * import { mdxToJsx } from "ironmark";
 *
 * const { module } = mdxToJsx("import Card from './Card'\n\n# Hi\n\n<Card>{props.name}</Card>");
 * ```
 */
export declare function mdxToJsx(input: MarkdownInput, options?: JsxOptions): JsxOutput;

/**
 * Parse an HTML string and return the AST as a JavaScript object.
 *
 * Converts HTML into the same AST structure used by the Markdown parser,
 * enabling HTML → Markdown conversion via `astToMarkdown()`.
 *
 * @param html - HTML source string.
 * @param options - `preserveUnknownAsHtml` keeps unknown tags as raw HTML.
 * @returns The `Document` root, same shape as `markdownToAst()`.
 *
 * @example
 * ```ts
 * import { htmlToAst, astToMarkdown } from "ironmark";
 *
 * const ast = htmlToAst("<h1>Hello</h1><p>World</p>");
 * const md = astToMarkdown(ast);
 * ```
 */
export declare function htmlToAst(html: string, options?: HtmlParseOptions): MarkdownAst;

/**
 * Convert HTML to Markdown.
 *
 * @param html - HTML source string.
 * @param options - `preserveUnknownAsHtml` keeps unknown tags as raw HTML.
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
export declare function htmlToMarkdown(html: string, options?: HtmlParseOptions): string;

// ─── Constants ────────────────────────────────────────────────────────────────

/**
 * Machine-readable metadata about the imported entry. `formats` lists only the
 * outputs its WASM binary supports (`ironmark/html` reports `["html"]`). Frozen.
 *
 * @example
 * ```ts
 * import { CAPABILITIES } from "ironmark";
 * // { astSchemaVersion: "2", formats: ["html", "ast", "markdown", "ansi", "jsx"], ... }
 * ```
 */
export declare const CAPABILITIES: Readonly<Capabilities>;

/** The AST schema version. An increment signals a breaking change to the node shape. */
export declare const AST_SCHEMA_VERSION: string;

/** The effective value of every option when none are given. Frozen. */
export declare const DEFAULT_OPTIONS: Readonly<
  Required<Omit<ParseOptions, "preset" | "safe" | "deterministic" | "stableAst">>
>;

/**
 * Every named preset and the options it applies. Frozen.
 *
 * @example
 * ```ts
 * import { PRESETS } from "ironmark";
 * const { llm } = PRESETS;
 * ```
 */
export declare const PRESETS: Readonly<Record<PresetName, Readonly<Partial<ParseOptions>>>>;

// ─── Utility functions ────────────────────────────────────────────────────────

/**
 * Extract all headings from a parsed AST, in document order (including nested ones).
 *
 * Returns level, plain-text content (inline Markdown stripped), and the `id` that
 * `enableHeadingIds` renders for that heading.
 *
 * @example
 * ```ts
 * import { markdownToAst, extractHeadings } from "ironmark";
 *
 * const headings = extractHeadings(markdownToAst("# Hello\n\n## World"));
 * // [{ level: 1, text: "Hello", id: "hello" }, { level: 2, text: "World", id: "world" }]
 * ```
 */
export declare function extractHeadings(ast: MarkdownAst | AstNode[]): HeadingInfo[];

/**
 * Summarize an AST: count top-level blocks and every node type (the root excluded).
 *
 * @example
 * ```ts
 * import { markdownToAst, summarizeAst } from "ironmark";
 *
 * const summary = summarizeAst(markdownToAst("# Hello\n\nA paragraph.\n\n- item"));
 * // { blockCount: 3, nodeCounts: { Heading: 1, Paragraph: 1, List: 1, ... } }
 * ```
 */
export declare function summarizeAst(ast: MarkdownAst | AstNode[]): AstSummary;
