const encoder = new TextEncoder();

// ─── Internal helpers ────────────────────────────────────────────────────────

// The WASM side takes UTF-8 bytes (one memcpy across the boundary). Byte input
// goes through as is; a leading BOM is dropped, as TextDecoder did before.
function bytesOf(view) {
  return view[0] === 0xef && view[1] === 0xbb && view[2] === 0xbf ? view.subarray(3) : view;
}

function toBytes(markdown) {
  if (typeof markdown === "string") return encoder.encode(markdown);
  if (markdown instanceof Uint8Array) return bytesOf(markdown);
  if (markdown instanceof ArrayBuffer) return bytesOf(new Uint8Array(markdown));
  if (ArrayBuffer.isView(markdown))
    return bytesOf(new Uint8Array(markdown.buffer, markdown.byteOffset, markdown.byteLength));
  throw new TypeError("markdown must be a string, Uint8Array, ArrayBuffer, or Buffer");
}

// ─── Preset resolution ───────────────────────────────────────────────────────

const PRESETS = {
  default: {},
  safe: {
    disableRawHtml: true,
    tagFilter: true,
  },
  strict: {
    hardBreaks: false,
    enableHighlight: false,
    enableStrikethrough: false,
    enableUnderline: false,
    enableAutolink: false,
    enableWikiLinks: false,
    enableLatexMath: false,
    permissiveAtxHeaders: false,
    collapseWhitespace: false,
    disableRawHtml: true,
  },
  llm: {
    hardBreaks: false,
    enableHighlight: true,
    enableStrikethrough: true,
    enableUnderline: true,
    enableTables: true,
    enableAutolink: false,
    enableTaskLists: true,
    disableRawHtml: true,
    enableHeadingIds: true,
    enableIndentedCodeBlocks: true,
    noHtmlBlocks: true,
    noHtmlSpans: true,
    tagFilter: true,
    collapseWhitespace: true,
    permissiveAtxHeaders: false,
    enableWikiLinks: false,
    enableLatexMath: false,
  },
};

export function resolveOptions(options) {
  if (options == null) return {};
  const preset = options.preset != null ? (PRESETS[options.preset] ?? {}) : {};
  const safe = options.safe === true ? PRESETS.safe : {};
  const deterministic = options.deterministic === true ? { collapseWhitespace: true } : {};
  const { preset: _p, safe: _s, deterministic: _d, stableAst: _a, ...rest } = options;
  return { ...preset, ...safe, ...deterministic, ...rest };
}

function optionArgs(markdown, options) {
  const r = resolveOptions(options);
  return [
    toBytes(markdown),
    r.hardBreaks,
    r.enableHighlight,
    r.enableStrikethrough,
    r.enableUnderline,
    r.enableTables,
    r.enableAutolink,
    r.enableTaskLists,
    r.disableRawHtml,
    r.enableHeadingIds,
    r.enableHeadingAnchors,
    r.enableIndentedCodeBlocks,
    r.noHtmlBlocks,
    r.noHtmlSpans,
    r.tagFilter,
    r.collapseWhitespace,
    r.permissiveAtxHeaders,
    r.enableWikiLinks,
    r.enableLatexMath,
  ];
}

// ─── Public API factories ─────────────────────────────────────────────────────

export function createParseMarkdown(wasmParseToAst) {
  return function parseMarkdown(input, options) {
    return JSON.parse(wasmParseToAst(...optionArgs(input, options)));
  };
}

export function createRenderHtml(wasmParse, wasmParseDefault = null) {
  return function renderHtml(input, options) {
    if (options == null && wasmParseDefault) return wasmParseDefault(toBytes(input));
    return wasmParse(...optionArgs(input, options));
  };
}

export function createRenderMarkdown(wasmRenderMarkdown) {
  return function renderMarkdown(ast) {
    const json =
      Array.isArray(ast) || (typeof ast === "object" && ast !== null) ? JSON.stringify(ast) : ast;
    return wasmRenderMarkdown(json);
  };
}

export function createRenderAnsiTerminal(wasmRenderAnsi) {
  return function renderAnsiTerminal(input, options, ansiOptions) {
    return wasmRenderAnsi(
      ...optionArgs(input, options),
      ansiOptions?.width ?? 0,
      ansiOptions?.color,
      ansiOptions?.lineNumbers,
      ansiOptions?.padding ?? 0,
    );
  };
}

export function createParseHtmlToAst(wasmParseHtmlToAst) {
  return function parseHtmlToAst(html, preserveUnknownAsHtml) {
    return JSON.parse(wasmParseHtmlToAst(html, preserveUnknownAsHtml));
  };
}

export function createHtmlToMarkdown(wasmHtmlToMarkdown) {
  return function htmlToMarkdown(html, preserveUnknownAsHtml) {
    return wasmHtmlToMarkdown(html, preserveUnknownAsHtml);
  };
}

// ─── Introspection helpers ────────────────────────────────────────────────────

export const AST_SCHEMA_VERSION = "2";

export function getCapabilities() {
  return {
    astSchemaVersion: AST_SCHEMA_VERSION,
    formats: ["html", "ast", "markdown", "ansi"],
    presets: Object.keys(PRESETS),
    extensions: [
      "hardBreaks",
      "enableHighlight",
      "enableStrikethrough",
      "enableUnderline",
      "enableTables",
      "enableAutolink",
      "enableTaskLists",
      "enableHeadingIds",
      "enableHeadingAnchors",
      "enableIndentedCodeBlocks",
      "enableWikiLinks",
      "enableLatexMath",
    ],
    security: ["disableRawHtml", "noHtmlBlocks", "noHtmlSpans", "tagFilter"],
  };
}

export function getAstSchemaVersion() {
  return AST_SCHEMA_VERSION;
}

export function getDefaultOptions() {
  return {
    hardBreaks: true,
    enableHighlight: true,
    enableStrikethrough: true,
    enableUnderline: true,
    enableTables: true,
    enableAutolink: true,
    enableTaskLists: true,
    disableRawHtml: false,
    enableHeadingIds: false,
    enableHeadingAnchors: false,
    enableIndentedCodeBlocks: true,
    noHtmlBlocks: false,
    noHtmlSpans: false,
    tagFilter: false,
    collapseWhitespace: false,
    permissiveAtxHeaders: false,
    enableWikiLinks: false,
    enableLatexMath: false,
  };
}

export function getPresets() {
  return structuredClone(PRESETS);
}

// ─── Utility functions ────────────────────────────────────────────────────────

export function extractHeadings(ast) {
  const headings = [];
  const blocks = Array.isArray(ast) ? ast : (ast?.children ?? []);
  function walk(nodes) {
    for (const node of nodes) {
      if (node?.t === "Heading" || node?.type === "Heading") {
        const level = node.level ?? node.l ?? 1;
        const text = extractText(node.children ?? node.c ?? []);
        headings.push({ level, text, id: slugify(text) });
      }
      const children = node?.children ?? node?.c ?? node?.items ?? [];
      if (Array.isArray(children)) walk(children);
    }
  }
  walk(blocks);
  return headings;
}

function extractText(nodes) {
  if (!Array.isArray(nodes)) return String(nodes ?? "");
  return nodes
    .map((n) => {
      if (typeof n === "string") return n;
      const t = n?.t ?? n?.type ?? "";
      if (t === "Text" || t === "Code") return n.text ?? n.value ?? "";
      return extractText(n?.children ?? n?.c ?? []);
    })
    .join("");
}

function slugify(text) {
  return text
    .toLowerCase()
    .replace(/[^\w\s-]/g, "")
    .trim()
    .replace(/[\s_]+/g, "-");
}

export function summarizeAst(ast) {
  const counts = {};
  const blocks = Array.isArray(ast) ? ast : (ast?.children ?? []);
  function walk(nodes) {
    for (const node of nodes) {
      const t = node?.t ?? node?.type ?? "Unknown";
      counts[t] = (counts[t] ?? 0) + 1;
      const children = node?.children ?? node?.c ?? node?.items ?? [];
      if (Array.isArray(children)) walk(children);
    }
  }
  walk(blocks);
  return { blockCount: blocks.length, nodeCounts: counts };
}
