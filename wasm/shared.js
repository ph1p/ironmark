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
  for (const key in value) {
    const inner = value[key];
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

// `safe: false` and `deterministic: false` switch their flags back off, so a layer added
// with `with()` can undo what an earlier layer turned on.
const UNSAFE = { disableRawHtml: false, tagFilter: false };

export function resolveOptions(options) {
  if (options == null) return {};
  const preset = options.preset != null ? (PRESETS[options.preset] ?? {}) : {};
  const safe = options.safe === true ? PRESETS.safe : options.safe === false ? UNSAFE : {};
  const deterministic =
    options.deterministic != null ? { collapseWhitespace: options.deterministic === true } : {};
  const { preset: _p, safe: _s, deterministic: _d, stableAst: _a, ...rest } = options;
  return { ...preset, ...safe, ...deterministic, ...rest };
}

// Parse options cross the boundary as one integer: bit `i` is the `i`-th key of
// `DEFAULT_OPTIONS` (`build_options` in wasm/src/lib.rs decodes it).
function optionArgs(markdown, options) {
  let flags = DEFAULT_FLAGS;
  if (options != null) {
    const r = resolveOptions(options);
    for (const key in r) {
      const bit = FLAG_BITS[key];
      if (bit === undefined || r[key] == null) continue;
      flags = r[key] ? flags | bit : flags & ~bit;
    }
  }
  return [toBytes(markdown), flags];
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

export function createMarkdownToText(wasmRenderText) {
  return function markdownToText(input, options) {
    return wasmRenderText(...optionArgs(input, options));
  };
}

/**
 * Parse once into a Rust-side tree (a wasm-bindgen `Parsed` handle) that renders the
 * analysis, AST, normalized Markdown and ANSI output without parsing again. The caller
 * owns the handle and must `free()` it. `parseDocument.outputs` names the methods this
 * entry's binary compiled into the handle (`ast` and `markdown` need `ast`, `ansi` needs `ansi`).
 */
export function createParseDocument(wasmParseDocument, Parsed) {
  function parseDocument(input, options) {
    return wasmParseDocument(...optionArgs(input, options));
  }
  parseDocument.outputs = new Set(Object.getOwnPropertyNames(Parsed.prototype));
  return parseDocument;
}

/**
 * Decode `Parsed.analyze()`: `[text, frontmatter | null, headingCount,
 * (level, id, text)…, (image, url, title | null, text)…]`.
 */
function decodeAnalysis(flat) {
  const headings = [];
  const links = [];
  const images = [];
  const linksAt = 3 + flat[2] * 3;
  for (let i = 3; i < linksAt; i += 3) {
    headings.push({ level: flat[i], text: flat[i + 2], id: flat[i + 1] });
  }
  for (let i = linksAt; i < flat.length; i += 4) {
    const url = flat[i + 1];
    const title = flat[i + 2];
    const text = flat[i + 3];
    if (flat[i]) images.push({ url, title, alt: text });
    else links.push({ url, title, text });
  }
  return { text: flat[0], frontmatter: flat[1] ?? null, headings, links, images };
}

export function createAstToMarkdown(wasmRenderMarkdown) {
  return function astToMarkdown(ast) {
    const json =
      Array.isArray(ast) || (typeof ast === "object" && ast !== null) ? JSON.stringify(ast) : ast;
    return wasmRenderMarkdown(json);
  };
}

/** ANSI render options as the bindings take them (0 = default width / no padding). */
function ansiArgs(options) {
  return [options?.width ?? 0, options?.color, options?.lineNumbers, options?.padding ?? 0];
}

export function createMarkdownToAnsi(wasmRenderAnsi) {
  return function markdownToAnsi(input, options) {
    return wasmRenderAnsi(...optionArgs(input, options), ...ansiArgs(options));
  };
}

export function createMdxToJsx(wasmRenderJsx) {
  return function mdxToJsx(input, options) {
    const out = wasmRenderJsx(
      ...optionArgs(input, options),
      options?.components,
      options?.providerImportSource,
      options?.markdown,
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
export const ALL_FORMATS = ["html", "text", "ast", "markdown", "ansi", "jsx"];

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
      "enableFrontmatter",
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
  enableFrontmatter: false,
});

const FLAG_BITS = Object.fromEntries(Object.keys(DEFAULT_OPTIONS).map((key, i) => [key, 1 << i]));
const DEFAULT_FLAGS = Object.keys(DEFAULT_OPTIONS).reduce(
  (flags, key) => (DEFAULT_OPTIONS[key] ? flags | FLAG_BITS[key] : flags),
  0,
);

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

/** Nest a flat heading list by level: each heading owns the deeper ones that follow it. */
function buildToc(headings) {
  const root = [];
  const stack = [];
  for (const heading of headings) {
    const entry = { ...heading, children: [] };
    while (stack.length && stack.at(-1).level >= heading.level) stack.pop();
    (stack.at(-1)?.children ?? root).push(entry);
    stack.push(entry);
  }
  return root;
}

// ─── Fluent API ──────────────────────────────────────────────────────────────

const decoder = new TextDecoder();

/** `Document` methods also offered on `ironmark` itself as one-shot shortcuts. */
const SHORTCUTS = [
  "html",
  "text",
  "headings",
  "toc",
  "links",
  "images",
  "frontmatter",
  "stats",
  "ast",
  "markdown",
  "ansi",
  "jsx",
];

const WORD = /[\p{L}\p{N}]+(?:['’][\p{L}\p{N}]+)*/gu;

/** Word and character counts plus reading time (minutes at 200 words per minute). */
function textStats(text) {
  const words = text.match(WORD)?.length ?? 0;
  const characters = text.match(/\S/gu)?.length ?? 0;
  return { words, characters, readingTime: Math.ceil(words / 200) };
}

/**
 * `Document` methods per output they render; dropped when the entry's `Parsed` handle
 * lacks that output (`jsx`: when the entry has no `mdxToJsx`).
 */
const REQUIRES = {
  analyze: ["text", "stats", "headings", "toc", "links", "images", "frontmatter"],
  ast: ["ast", "summary"],
  markdown: ["markdown"],
  ansi: ["ansi"],
  jsx: ["jsx"],
};

/**
 * Layer parse options over already-resolved `base`. A layer with a `preset` starts over
 * from that preset; otherwise its keys (after `safe`/`deterministic` expansion) win.
 */
function mergeOptions(base, options) {
  if (options == null) return base;
  return Object.freeze({ ...(options.preset != null ? {} : base), ...resolveOptions(options) });
}

/** Copy byte input so later writes to the caller's buffer cannot change the document. */
function ownedBytes(input) {
  return typeof input === "string" ? encoder.encode(input) : toBytes(input).slice();
}

/**
 * Build an entry's default export: `ironmark.parse(md)` returns a `Document` that renders
 * lazily and caches each output. `api` holds the entry's functions; a `Document` method
 * whose function is missing is left off the prototype, so `"ansi" in doc` tells the truth.
 */
export function createIronmark(api) {
  // The one live parse tree: WASM memory pressure never triggers a JS GC, so trees are
  // freed eagerly instead of waiting for the FinalizationRegistry. The slot holds the
  // tree of the document rendered last and is released once the current task's
  // synchronous work is done, so a loop over many documents keeps at most one tree.
  // Outputs of one document share a tree while they are rendered back to back;
  // interleaving documents or awaiting in between parses again.
  let live = null;
  let releaseQueued = false;
  const release = () => {
    releaseQueued = false;
    live?.tree.free();
    live = null;
  };

  class Document {
    #bytes;
    #options;
    #source;
    #cache = new Map();

    constructor(bytes, options, source) {
      this.#bytes = bytes;
      this.#options = options;
      this.#source = source;
    }

    /** The Markdown source; byte input is decoded once on first access. */
    get source() {
      return (this.#source ??= decoder.decode(this.#bytes));
    }

    get options() {
      return this.#options;
    }

    #output(key, render) {
      if (!this.#cache.has(key)) this.#cache.set(key, render());
      return this.#cache.get(key);
    }

    /** The shared parse tree for the outputs below; valid until the next microtask. */
    #tree() {
      if (live?.owner !== this) {
        live?.tree.free();
        live = { owner: this, tree: api.parseDocument(this.#bytes, this.#options) };
        if (!releaseQueued) {
          releaseQueued = true;
          queueMicrotask(release);
        }
      }
      return live.tree;
    }

    html() {
      // No options at all keeps the `parseDefault` fast path.
      const options = Object.keys(this.#options).length ? this.#options : undefined;
      return this.#output("html", () => api.markdownToHtml(this.#bytes, options));
    }

    /** Text, headings, links and front matter come from one render, cached together. */
    #analysis() {
      return this.#output("analysis", () => deepFreeze(decodeAnalysis(this.#tree().analyze())));
    }

    text() {
      return this.#analysis().text;
    }

    headings() {
      return this.#analysis().headings;
    }

    toc() {
      return this.#output("toc", () => deepFreeze(buildToc(this.headings())));
    }

    links() {
      return this.#analysis().links;
    }

    images() {
      return this.#analysis().images;
    }

    frontmatter() {
      return this.#analysis().frontmatter;
    }

    stats() {
      return this.#output("stats", () => Object.freeze(textStats(this.text())));
    }

    ast() {
      return this.#output("ast", () => deepFreeze(JSON.parse(this.#tree().ast())));
    }

    summary() {
      return this.#output("summary", () => deepFreeze(summarizeAst(this.ast())));
    }

    markdown() {
      return this.#output("markdown", () => this.#tree().markdown());
    }

    ansi(options) {
      const args = ansiArgs(options);
      return this.#output(`ansi:${args}`, () => this.#tree().ansi(...args));
    }

    jsx({ components, providerImportSource, markdown } = {}) {
      return this.#output(`jsx:${components}:${providerImportSource}:${markdown}`, () =>
        Object.freeze(
          api.mdxToJsx(this.#bytes, {
            ...this.#options,
            components,
            providerImportSource,
            markdown,
          }),
        ),
      );
    }

    /** A new document over the same source with these parse options layered on top. */
    with(options) {
      return new Document(this.#bytes, mergeOptions(this.#options, options), this.#source);
    }
  }

  const outputs = new Set(api.parseDocument.outputs);
  if (api.mdxToJsx) outputs.add("jsx");
  for (const [output, methods] of Object.entries(REQUIRES)) {
    if (!outputs.has(output)) for (const method of methods) delete Document.prototype[method];
  }
  const shortcuts = SHORTCUTS.filter((method) => method in Document.prototype);

  function create(defaults) {
    const ironmark = {
      options: defaults,
      init: api.init,
      parse: (input, options) =>
        new Document(
          ownedBytes(input),
          mergeOptions(defaults, options),
          typeof input === "string" ? input : undefined,
        ),
      with: (options) => create(mergeOptions(defaults, options)),
      get ready() {
        return api.init();
      },
    };
    // `ironmark.html(md)` is `ironmark.parse(md).html()` without the defensive copy:
    // the input is read once, right away.
    for (const method of shortcuts) {
      ironmark[method] = (input, ...args) =>
        new Document(toBytes(input), defaults, typeof input === "string" ? input : undefined)[
          method
        ](...args);
    }
    if (api.htmlToMarkdown) {
      ironmark.fromHtml = (html, options) => {
        const { preserveUnknownAsHtml, ...rest } = options ?? {};
        const markdown = api.htmlToMarkdown(html, { preserveUnknownAsHtml });
        return new Document(encoder.encode(markdown), mergeOptions(defaults, rest), markdown);
      };
    }
    return Object.freeze(ironmark);
  }

  return create(Object.freeze({}));
}
