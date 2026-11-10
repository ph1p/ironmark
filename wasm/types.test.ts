import * as html from "./html.js";
import type { Capabilities, MarkdownInput, ParseOptions } from "./html.js";
import type { MarkdownInput as FullInput, ParseOptions as FullOptions } from "./index.js";

const options: ParseOptions = { preset: "llm", enableTables: true, safe: true };
const sharedOptions: FullOptions = options;
const htmlOptions: ParseOptions = sharedOptions;
const input: MarkdownInput = new Uint8Array([35, 32, 72, 105]);
const sharedInput: FullInput = input;
const result: string = html.markdownToHtml(sharedInput, htmlOptions);
const plain: string = html.markdownToText(sharedInput, htmlOptions);
void plain;
const initialization: Promise<void> = html.init(new URL("https://example.com/html.wasm"));
void result;
void initialization;

// @ts-expect-error HTML-only entry does not expose an AST parser.
html.markdownToAst("text");
// @ts-expect-error HTML-only entry does not expose ANSI rendering.
html.markdownToAnsi("text");
// @ts-expect-error HTML-only entry does not expose HTML parsing.
html.htmlToAst("<p>text</p>");
// @ts-expect-error HTML-only entry does not expose Markdown rendering.
html.astToMarkdown("");
const caps: Readonly<Capabilities> = html.CAPABILITIES;
void caps;
// @ts-expect-error Constants are frozen.
html.PRESETS.safe.disableRawHtml = false;
// @ts-expect-error Shared options remain typed, not an unconstrained object.
html.markdownToHtml("text", { unsupportedOption: true });

// ─── ironmark/mdx and ironmark/ast subpaths ──────────────────────────────────
import * as mdx from "./mdx.js";
import * as ast from "./ast.js";
import type { JsxOutput } from "./mdx.js";
import type { HeadingInfo, MarkdownAst } from "./ast.js";

const jsx: JsxOutput = mdx.mdxToJsx("# Hi", { components: true });
const mdxHtml: string = mdx.markdownToHtml("# Hi");
const tree: MarkdownAst = ast.markdownToAst("# Hi");
const headings: HeadingInfo[] = ast.extractHeadings(tree);
const roundTrip: string = ast.astToMarkdown(tree);
void [jsx, mdxHtml, headings, roundTrip, mdx.init(), ast.init()];

// @ts-expect-error MDX entry does not expose an AST parser.
mdx.markdownToAst("text");
// @ts-expect-error AST entry does not expose MDX compilation.
ast.mdxToJsx("text");
// @ts-expect-error AST entry does not expose ANSI rendering.
ast.markdownToAnsi("text");

// ─── AST shape ───────────────────────────────────────────────────────────────
for (const node of tree.Document.children) {
  if (typeof node === "string") {
    const brk: "ThematicBreak" = node;
    void brk;
  } else if ("Heading" in node) {
    const level: number = node.Heading.level;
    void level;
  }
}
const version: string = ast.AST_SCHEMA_VERSION;
void version;

// ─── Chainable default export ────────────────────────────────────────────────
import ironmark from "./index.js";
import htmlMark from "./html.js";
import astMark from "./ast.js";
import mdxMark from "./mdx.js";
import type { DeepReadonly, Document, ResolvedOptions, TocEntry } from "./index.js";
import type { HtmlDocument } from "./html.js";

const doc: Document = ironmark.with({ preset: "safe" }).parse("# Hi");
const docHtml: string = doc.with({ enableHeadingIds: true, safe: true }).html();
const docAnsi: string = doc.ansi({ width: 60, color: false });
const toc: DeepReadonly<TocEntry[]> = doc.toc();
const fromHtml: Document = ironmark.with({}).fromHtml("<p>x</p>", { preserveUnknownAsHtml: true });
const astHeadings: DeepReadonly<HeadingInfo[]> = astMark.parse("# x").with({}).headings();
const htmlHeadings: DeepReadonly<HeadingInfo[]> = htmlMark.parse("# x").headings();
const mdxModule: string = mdxMark.parse("# x").jsx({ components: true }).module;
const resolved: ResolvedOptions = doc.options;
const htmlDoc: HtmlDocument = htmlMark.parse("x").with({ hardBreaks: false });
const ready: Promise<void> = ironmark.init();
// Frozen outputs feed straight back into the AST helpers.
const docTree = doc.ast();
const normalized: string = ast.astToMarkdown(docTree);
const fromFrozen: HeadingInfo[] = ast.extractHeadings(docTree);
const editable: MarkdownAst = structuredClone(doc.ast()) as MarkdownAst;
editable.Document.children.push("ThematicBreak");
void [
  docHtml,
  docAnsi,
  toc,
  fromHtml,
  astHeadings,
  htmlHeadings,
  mdxModule,
  resolved,
  htmlDoc,
  ready,
  normalized,
  fromFrozen,
  ast.summarizeAst(docTree),
  doc.text(),
  doc.markdown(),
  htmlMark.parse("x").html(),
  htmlMark.parse("x").text(),
];

// @ts-expect-error HTML-only documents have no AST.
htmlMark.parse("x").ast();
// @ts-expect-error HTML-only documents have no JSX.
htmlMark.parse("x").jsx();
// @ts-expect-error MDX documents have no AST.
mdxMark.parse("x").ast();
// @ts-expect-error ironmark/ast documents cannot render ANSI.
astMark.parse("x").ansi();
// @ts-expect-error Only the full entry converts HTML.
astMark.fromHtml("<p>x</p>");
// @ts-expect-error Options are frozen.
doc.options.hardBreaks = true;
// @ts-expect-error Resolved options have no presets or shorthands.
void doc.options.preset;
// @ts-expect-error Cached results are frozen.
doc.headings().push({ level: 1, text: "x", id: "x" });
// @ts-expect-error The AST is frozen.
doc.ast().Document.children.push("ThematicBreak");
// @ts-expect-error Render options are not parse options.
doc.with({ width: 1 });
// @ts-expect-error Parse options are not render options.
doc.ansi({ hardBreaks: false });
// @ts-expect-error html() takes no options; use with().
doc.html({ safe: true });
const { html: detached } = doc;
// @ts-expect-error Methods must be called on the document.
detached();

// ─── Extras: shortcuts, ready, links, frontmatter, stats ────────────────────
import type { DocumentStats, ImageInfo, LinkInfo } from "./index.js";

const oneShot: string = ironmark.html("*x*");
const safeShot: string = ironmark.with({ preset: "safe" }).text("<b>x</b>");
const shotToc: DeepReadonly<TocEntry[]> = htmlMark.toc("# x");
const shotJsx: string = mdxMark.jsx("# x").jsx;
const shotMd: string = astMark.markdown("# x");
const readyPromise: Promise<void> = ironmark.ready;
const links: DeepReadonly<LinkInfo[]> = doc.links();
const images: DeepReadonly<ImageInfo[]> = htmlMark.parse("![a](b)").images();
const matter: string | null = doc.frontmatter();
const stats: DeepReadonly<DocumentStats> = doc.stats();
const words: number = stats.words;
void [oneShot, safeShot, shotToc, shotJsx, shotMd, readyPromise, links, images, matter, words];
// @ts-expect-error HTML-only entry has no AST shortcut.
htmlMark.ast("# x");
// @ts-expect-error AST entry has no JSX shortcut.
astMark.jsx("# x");
// @ts-expect-error Stats are read-only.
doc.stats().words = 1;
