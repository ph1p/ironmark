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

// ─── Browser initialization ──────────────────────────────────────────────────

// Instantiate a wasm-bindgen glue module from a URL, path, or compiled Module.
// `defaultUrl` is built by each entry with `new URL(..., import.meta.url)` so
// bundlers can see and emit the asset.
export async function instantiateWasm(glue, importName, input, defaultUrl) {
  const imports = { [importName]: glue };
  const url = input instanceof URL || typeof input === "string" ? input : defaultUrl;
  const result =
    typeof input === "object" && input instanceof WebAssembly.Module
      ? await WebAssembly.instantiate(input, imports)
      : typeof WebAssembly.instantiateStreaming === "function"
        ? await WebAssembly.instantiateStreaming(fetch(url), imports)
        : await WebAssembly.instantiate(await fetch(url).then((r) => r.arrayBuffer()), imports);
  const instance = result instanceof WebAssembly.Instance ? result : result.instance;
  glue.__wbg_set_wasm(instance.exports);
}

// ─── Preset resolution ───────────────────────────────────────────────────────

/** Recursively freeze exported data so callers cannot corrupt shared defaults. */
function deepFreeze(value) {
  for (const inner of Object.values(value)) {
    if (inner && typeof inner === "object") deepFreeze(inner);
  }
  return Object.freeze(value);
}

export const PRESETS = deepFreeze({
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
});

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
    r.enableMdx,
  ];
}

// ─── Public API factories ─────────────────────────────────────────────────────

export function createMarkdownToAst(wasmParseToAst) {
  return function markdownToAst(input, options) {
    return JSON.parse(wasmParseToAst(...optionArgs(input, options)));
  };
}

export function createMarkdownToHtml(wasmParse, wasmParseDefault = null) {
  return function markdownToHtml(input, options) {
    if (options == null && wasmParseDefault) return wasmParseDefault(toBytes(input));
    return wasmParse(...optionArgs(input, options));
  };
}

export function createAstToMarkdown(wasmRenderMarkdown) {
  return function astToMarkdown(ast) {
    const json =
      Array.isArray(ast) || (typeof ast === "object" && ast !== null) ? JSON.stringify(ast) : ast;
    return wasmRenderMarkdown(json);
  };
}

export function createMarkdownToAnsi(wasmRenderAnsi) {
  return function markdownToAnsi(input, options) {
    return wasmRenderAnsi(
      ...optionArgs(input, options),
      options?.width ?? 0,
      options?.color,
      options?.lineNumbers,
      options?.padding ?? 0,
    );
  };
}

export function createMdxToJsx(wasmRenderJsx) {
  return function mdxToJsx(input, options) {
    const out = wasmRenderJsx(
      ...optionArgs(input, options),
      options?.components,
      options?.providerImportSource,
    );
    try {
      return { esm: out.esm, jsx: out.jsx, module: out.module };
    } finally {
      out.free();
    }
  };
}

export function createHtmlToAst(wasmParseHtmlToAst) {
  return function htmlToAst(html, options) {
    return JSON.parse(wasmParseHtmlToAst(html, options?.preserveUnknownAsHtml));
  };
}

export function createHtmlToMarkdown(wasmHtmlToMarkdown) {
  return function htmlToMarkdown(html, options) {
    return wasmHtmlToMarkdown(html, options?.preserveUnknownAsHtml);
  };
}

// ─── Introspection helpers ────────────────────────────────────────────────────

export const AST_SCHEMA_VERSION = "2";

/** Every output format, in the order `CAPABILITIES.formats` lists them. */
export const ALL_FORMATS = ["html", "ast", "markdown", "ansi", "jsx"];

/** Machine-readable metadata; `formats` lists only what the importing entry's binary can do. */
export function createCapabilities(formats) {
  return deepFreeze({
    astSchemaVersion: AST_SCHEMA_VERSION,
    formats: [...formats],
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
      "enableMdx",
    ],
    security: ["disableRawHtml", "noHtmlBlocks", "noHtmlSpans", "tagFilter"],
  });
}

export const DEFAULT_OPTIONS = deepFreeze({
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
  enableMdx: false,
});

// ─── Utility functions ────────────────────────────────────────────────────────

// The AST is serde's externally tagged form: `{ Heading: { level, raw } }`, a bare
// string for unit variants (`"ThematicBreak"`), and `{ Document: { children } }` at the root.
function nodeType(node) {
  if (typeof node === "string") return node;
  const keys = node && typeof node === "object" ? Object.keys(node) : [];
  return keys.length === 1 ? keys[0] : "Unknown";
}

function topLevelBlocks(ast) {
  if (Array.isArray(ast)) return ast;
  return ast?.Document?.children ?? [];
}

function walkAst(nodes, visit) {
  for (const node of nodes) {
    const type = nodeType(node);
    visit(type, typeof node === "string" ? undefined : node[type]);
    const children = typeof node === "string" ? undefined : node[type]?.children;
    if (Array.isArray(children)) walkAst(children, visit);
  }
}

export function extractHeadings(ast) {
  const headings = [];
  walkAst(topLevelBlocks(ast), (type, data) => {
    if (type !== "Heading") return;
    const raw = data?.raw ?? "";
    headings.push({ level: data?.level ?? 1, text: inlineText(raw), id: headingSlug(raw) });
  });
  return headings;
}

export function summarizeAst(ast) {
  const blocks = topLevelBlocks(ast);
  const nodeCounts = {};
  walkAst(blocks, (type) => {
    nodeCounts[type] = (nodeCounts[type] ?? 0) + 1;
  });
  return { blockCount: blocks.length, nodeCounts };
}

const ENTITIES = { amp: "&", lt: "<", gt: ">", quot: '"', apos: "'", nbsp: " " };

/** Heading `raw` is unparsed inline Markdown; reduce it to the text a reader sees. */
function inlineText(raw) {
  let out = "";
  let i = 0;
  while (i < raw.length) {
    const ch = raw[i];
    if (ch === "\\" && /[!-/:-@[-`{-~]/.test(raw[i + 1] ?? "")) {
      out += raw[i + 1];
      i += 2;
    } else if (ch === "`") {
      const fence = /^`+/.exec(raw.slice(i))[0];
      const close = raw.indexOf(fence, i + fence.length);
      if (close === -1) {
        out += fence;
        i += fence.length;
      } else {
        out += raw.slice(i + fence.length, close).trim();
        i = close + fence.length;
      }
    } else if (ch === "<") {
      const close = raw.indexOf(">", i);
      const inner = close === -1 ? "" : raw.slice(i + 1, close);
      if (close === -1) {
        out += ch;
        i += 1;
      } else {
        // Keep autolink targets, drop HTML/JSX tags.
        if (/^[a-z][a-z0-9+.-]*:\S*$|^[^\s@]+@[^\s@]+$/i.test(inner)) out += inner;
        i = close + 1;
      }
    } else if (ch === "!" && raw[i + 1] === "[") {
      i += 1;
    } else if (ch === "[" || ch === "]") {
      // `](url)` and `][ref]` destinations belong to the link, not its text.
      if (ch === "]") {
        const dest = /^\]\([^)]*\)|^\]\[[^\]]*\]/.exec(raw.slice(i));
        i += dest ? dest[0].length : 1;
      } else {
        i += 1;
      }
    } else if (ch === "&") {
      const entity = /^&(?:#(\d+)|#[xX]([0-9a-fA-F]+)|([a-zA-Z]+));/.exec(raw.slice(i));
      const code = entity && (entity[1] ? +entity[1] : entity[2] ? parseInt(entity[2], 16) : null);
      const named = entity?.[3] && ENTITIES[entity[3]];
      if (code != null && code <= 0x10ffff) out += String.fromCodePoint(code || 0xfffd);
      else if (named) out += named;
      else out += entity ? entity[0] : ch;
      i += entity && (code != null || named) ? entity[0].length : 1;
    } else if ("*_~=+".includes(ch)) {
      // Delimiter runs only when they touch a word; `a * b` and `snake_case` keep theirs.
      const run = /^[*_~=+]+/.exec(raw.slice(i))[0];
      const before = raw[i - 1] ?? " ";
      const after = raw[i + run.length] ?? " ";
      const intraword = /\w/.test(before) && /\w/.test(after);
      const spaced = /\s/.test(before) && /\s/.test(after);
      if (intraword || spaced) out += run;
      i += run.length;
    } else {
      out += ch;
      i += 1;
    }
  }
  return out.replace(/\s+/g, " ").trim();
}

/** Port of `src/render/slug.rs`, so `id` matches what `enableHeadingIds` renders. */
function headingSlug(raw) {
  let slug = "";
  let prevDash = true;
  const dash = () => {
    if (!prevDash && slug) {
      slug += "-";
      prevDash = true;
    }
  };
  for (let i = 0; i < raw.length;) {
    const ch = raw[i];
    if (ch === "<") {
      const close = raw.indexOf(">", i);
      i = close === -1 ? i + 1 : close + 1;
    } else if (" \t-.".includes(ch)) {
      dash();
      i += 1;
    } else if (/[A-Za-z0-9]/.test(ch)) {
      slug += ch.toLowerCase();
      prevDash = false;
      i += 1;
    } else if (ch.charCodeAt(0) >= 0x80) {
      const c = String.fromCodePoint(raw.codePointAt(i));
      if (/[\p{Alphabetic}\p{N}]/u.test(c)) {
        slug += c.toLowerCase();
        prevDash = false;
      } else {
        dash();
      }
      i += c.length;
    } else {
      i += 1; // Markdown syntax and other ASCII punctuation are dropped.
    }
  }
  return slug.replace(/-+$/, "");
}
