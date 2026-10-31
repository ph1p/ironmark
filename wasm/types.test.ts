import * as html from "./html.js";
import type { MarkdownInput, ParseOptions, RenderHtmlOptions } from "./html.js";
import type { MarkdownInput as FullInput, ParseOptions as FullOptions } from "./index.js";

const options: ParseOptions = { preset: "llm", enableTables: true, safe: true };
const sharedOptions: FullOptions = options;
const htmlOptions: RenderHtmlOptions = sharedOptions;
const input: MarkdownInput = new Uint8Array([35, 32, 72, 105]);
const sharedInput: FullInput = input;
const result: string = html.renderHtml(sharedInput, htmlOptions);
const initialization: void | Promise<void> = html.init(new URL("https://example.com/html.wasm"));
void result;
void initialization;

// @ts-expect-error HTML-only entry does not expose an AST parser.
html.parseMarkdown("text");
// @ts-expect-error HTML-only entry does not expose ANSI rendering.
html.renderAnsiTerminal("text");
// @ts-expect-error HTML-only entry does not expose HTML parsing.
html.parseHtmlToAst("<p>text</p>");
// @ts-expect-error HTML-only entry does not expose Markdown rendering.
html.renderMarkdown([]);
// @ts-expect-error Full capabilities would misleadingly advertise absent formats.
html.getCapabilities();
// @ts-expect-error Shared options remain typed, not an unconstrained object.
html.renderHtml("text", { unsupportedOption: true });
