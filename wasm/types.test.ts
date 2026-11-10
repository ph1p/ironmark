import * as html from "./html.js";
import type { Capabilities, MarkdownInput, ParseOptions } from "./html.js";
import type { MarkdownInput as FullInput, ParseOptions as FullOptions } from "./index.js";

const options: ParseOptions = { preset: "llm", enableTables: true, safe: true };
const sharedOptions: FullOptions = options;
const htmlOptions: ParseOptions = sharedOptions;
const input: MarkdownInput = new Uint8Array([35, 32, 72, 105]);
const sharedInput: FullInput = input;
const result: string = html.markdownToHtml(sharedInput, htmlOptions);
const initialization: void | Promise<void> = html.init(new URL("https://example.com/html.wasm"));
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
