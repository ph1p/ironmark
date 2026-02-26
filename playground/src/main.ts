import "./style.css";
import { init, parse } from "ironmark";
import wasmUrl from "ironmark/ironmark.wasm?url";
import { version } from "../../package.json";

import { Compartment, EditorState } from "@codemirror/state";
import { EditorView, keymap, lineNumbers } from "@codemirror/view";
import { markdown } from "@codemirror/lang-markdown";
import { html as langHtml } from "@codemirror/lang-html";
import { javascript } from "@codemirror/lang-javascript";
import { css } from "@codemirror/lang-css";
import { json } from "@codemirror/lang-json";
import { python } from "@codemirror/lang-python";
import { rust } from "@codemirror/lang-rust";
import { yaml } from "@codemirror/lang-yaml";
import { oneDarkHighlightStyle } from "@codemirror/theme-one-dark";
import { defaultHighlightStyle, syntaxHighlighting } from "@codemirror/language";
import { highlightCode } from "@lezer/highlight";
import type { LanguageSupport, Language } from "@codemirror/language";
import { indentWithTab } from "@codemirror/commands";

await init(wasmUrl);

// --- Theme detection ---
const darkQuery = window.matchMedia("(prefers-color-scheme: dark)");

function isDark(): boolean {
  return darkQuery.matches;
}

// --- Language map ---
const langs: Record<string, () => LanguageSupport> = {
  javascript,
  js: javascript,
  typescript: () => javascript({ typescript: true }),
  ts: () => javascript({ typescript: true }),
  jsx: () => javascript({ jsx: true }),
  tsx: () => javascript({ jsx: true, typescript: true }),
  rust,
  html: langHtml,
  css,
  json,
  python,
  py: python,
  yaml,
  yml: yaml,
  markdown,
  md: markdown,
};

function highlightCodeString(code: string, langName: string): string {
  const langFactory = langs[langName];
  if (!langFactory) return escapeHtml(code);

  const support = langFactory();
  const language: Language = support.language;
  const tree = language.parser.parse(code);
  let result = "";
  let pos = 0;
  const style = isDark() ? oneDarkHighlightStyle : defaultHighlightStyle;

  highlightCode(
    code,
    tree,
    style,
    (text, classes) => {
      const escaped = escapeHtml(text);
      result += classes ? `<span class="${classes}">${escaped}</span>` : escaped;
      pos += text.length;
    },
    () => {
      result += "\n";
      pos++;
    },
  );

  if (pos < code.length) {
    result += escapeHtml(code.slice(pos));
  }

  return result;
}

function escapeHtml(s: string): string {
  return s.replace(/&/g, "&amp;").replace(/</g, "&lt;").replace(/>/g, "&gt;");
}

const DEFAULT_MARKDOWN = `# Markdown Playground

Write **markdown** on the left and see the _rendered HTML_ on the right.

## Features

- Live preview as you type
- Supports **bold**, *italic*, and \`code\`
- Links: [Example](https://example.com)
- Images: ![alt](https://via.placeholder.com/100)

## Code Block

\`\`\`rust
fn main() {
    println!("Hello, world!");
}
\`\`\`

## Table

| Name  | Score | Grade |
| :---- | ----: | :---: |
| Alice |    95 |   A   |
| Bob   |    82 |   B   |

## Blockquote

> Markdown is a lightweight markup language
> that you can use to add formatting to plain text.

---

1. First item
2. Second item
   - Nested bullet
   - Another one
3. Third item
`;

const app = document.querySelector<HTMLDivElement>("#app")!;

app.innerHTML = `
  <div class="h-full flex flex-col bg-white dark:bg-zinc-950 text-zinc-900 dark:text-zinc-100 transition-colors">
    <header class="flex items-center justify-between px-3 py-2 md:px-5 md:py-3 border-b border-zinc-200 dark:border-zinc-800 shrink-0">
      <div class="flex items-center gap-2 md:gap-3 min-w-0">
        <h1 class="text-sm md:text-base font-semibold tracking-tight truncate">Markdown Playground</h1>
        <span class="text-xs text-zinc-400 dark:text-zinc-500 font-mono shrink-0">v${version}</span>
      </div>
      <div class="flex items-center gap-2 md:gap-3 shrink-0">
        <div id="status" class="text-xs text-zinc-400 dark:text-zinc-500 font-mono">loading wasm…</div>
        <div class="flex items-center gap-2">
          <a href="https://github.com/ph1p/ironmark" target="_blank" rel="noopener noreferrer" class="text-zinc-400 dark:text-zinc-500 hover:text-zinc-600 dark:hover:text-zinc-300 transition-colors" title="GitHub">
            <svg width="16" height="16" viewBox="0 0 24 24" fill="currentColor"><path d="M12 0C5.37 0 0 5.37 0 12c0 5.31 3.435 9.795 8.205 11.385.6.105.825-.255.825-.57 0-.285-.015-1.23-.015-2.235-3.015.555-3.795-.735-4.035-1.41-.135-.345-.72-1.41-1.23-1.695-.42-.225-1.02-.78-.015-.795.945-.015 1.62.87 1.845 1.23 1.08 1.815 2.805 1.305 3.495.99.105-.78.42-1.305.765-1.605-2.67-.3-5.46-1.335-5.46-5.925 0-1.305.465-2.385 1.23-3.225-.12-.3-.54-1.53.12-3.18 0 0 1.005-.315 3.3 1.23.96-.27 1.98-.405 3-.405s2.04.135 3 .405c2.295-1.56 3.3-1.23 3.3-1.23.66 1.65.24 2.88.12 3.18.765.84 1.23 1.905 1.23 3.225 0 4.605-2.805 5.625-5.475 5.925.435.375.81 1.095.81 2.22 0 1.605-.015 2.895-.015 3.3 0 .315.225.69.825.57A12.02 12.02 0 0024 12c0-6.63-5.37-12-12-12z"/></svg>
          </a>
          <a href="https://www.npmjs.com/package/ironmark" target="_blank" rel="noopener noreferrer" class="text-zinc-400 dark:text-zinc-500 hover:text-zinc-600 dark:hover:text-zinc-300 transition-colors" title="npm">
            <svg width="16" height="16" viewBox="0 0 24 24" fill="currentColor"><path d="M0 7.334v8h6.666v1.332H12v-1.332h12v-8H0zm6.666 6.664H5.334v-4H3.999v4H1.335V8.667h5.331v5.331zm4 0v1.336H8.001V8.667h5.334v5.332h-2.669v-.001zm12.001 0h-1.33v-4h-1.336v4h-1.335v-4h-1.33v4h-2.671V8.667h8.002v5.331z"/></svg>
          </a>
          <a href="https://crates.io/crates/ironmark" target="_blank" rel="noopener noreferrer" class="text-zinc-400 dark:text-zinc-500 hover:text-zinc-600 dark:hover:text-zinc-300 transition-colors" title="crates.io">
            <svg width="16" height="16" viewBox="0 0 24 24" fill="currentColor"><path d="M23.998 12.014c-.003-2.298-.656-4.408-1.782-6.2l-.063-.09-2.636 1.536c-.263-.4-.555-.78-.876-1.132l2.636-1.534a11.94 11.94 0 00-4.598-3.86L16.6 0.61l-1.524 2.642a10.923 10.923 0 00-1.4-.322V0h-.07A11.922 11.922 0 0012.002 0h-.07v3.056c-.482.06-.955.155-1.414.326L8.994.74l-.08.044a11.918 11.918 0 00-4.6 3.862l2.637 1.535c-.32.35-.613.73-.876 1.13L3.44 5.776l-.063.09A11.94 11.94 0 001.595 12.07h3.06c.012.482.068.955.168 1.414l-2.642 1.524.044.08a11.926 11.926 0 003.862 4.6l1.534-2.637c.35.32.732.613 1.132.876l-1.536 2.636.09.063a11.924 11.924 0 006.2 1.782v-3.072c.478-.016.95-.07 1.414-.172l1.524 2.642.08-.044a11.918 11.918 0 004.6-3.862l-2.637-1.534c.32-.352.613-.732.876-1.132l2.636 1.536.063-.09a11.924 11.924 0 001.782-6.202v-.07h-3.06a10.927 10.927 0 00-.168-1.414l2.642-1.524-.044-.08a11.926 11.926 0 00-3.862-4.6zM12 16.5a4.5 4.5 0 110-9 4.5 4.5 0 010 9z"/></svg>
          </a>
        </div>
      </div>
    </header>
    <!-- Mobile panel tabs -->
    <div class="flex md:hidden border-b border-zinc-200 dark:border-zinc-800 bg-zinc-50 dark:bg-zinc-900/50 shrink-0">
      <button id="mobile-tab-editor" class="mobile-tab flex-1 px-4 py-2 text-xs font-medium uppercase tracking-wider text-zinc-900 dark:text-zinc-100 border-b-2 border-zinc-900 dark:border-zinc-100">
        Editor
      </button>
      <button id="mobile-tab-preview" class="mobile-tab flex-1 px-4 py-2 text-xs font-medium uppercase tracking-wider text-zinc-400 dark:text-zinc-500 border-b-2 border-transparent hover:text-zinc-600 dark:hover:text-zinc-300">
        Preview
      </button>
      <button id="mobile-tab-html" class="mobile-tab flex-1 px-4 py-2 text-xs font-medium uppercase tracking-wider text-zinc-400 dark:text-zinc-500 border-b-2 border-transparent hover:text-zinc-600 dark:hover:text-zinc-300">
        HTML
      </button>
    </div>
    <div id="main-split" class="flex flex-col md:flex-row flex-1 min-h-0">
      <div id="panel-editor" class="flex-1 flex flex-col md:border-r border-zinc-200 dark:border-zinc-800 min-h-0">
        <div class="hidden md:block px-4 py-2 text-xs font-medium text-zinc-400 uppercase tracking-wider border-b border-zinc-200 dark:border-zinc-800 bg-zinc-50 dark:bg-zinc-900/50">
          Markdown
        </div>
        <div id="editor-container" class="flex-1 min-h-0 overflow-hidden"></div>
      </div>
      <div id="panel-output" class="hidden md:flex flex-1 flex-col min-h-0">
        <div class="hidden md:flex border-b border-zinc-200 dark:border-zinc-800 bg-zinc-50 dark:bg-zinc-900/50">
          <button id="tab-preview" class="tab-btn px-4 py-2 text-xs font-medium uppercase tracking-wider text-zinc-900 dark:text-zinc-100 border-b-2 border-zinc-900 dark:border-zinc-100">
            Preview
          </button>
          <button id="tab-html" class="tab-btn px-4 py-2 text-xs font-medium uppercase tracking-wider text-zinc-400 dark:text-zinc-500 border-b-2 border-transparent hover:text-zinc-600 dark:hover:text-zinc-300">
            HTML
          </button>
        </div>
        <div id="preview-panel" class="flex-1 overflow-auto p-3 md:p-5">
          <div id="preview" class="prose"></div>
        </div>
        <div id="html-panel" class="flex-1 min-h-0 overflow-hidden hidden">
          <div id="html-source" class="h-full"></div>
        </div>
      </div>
    </div>
  </div>
`;

const editorContainer = document.querySelector<HTMLDivElement>("#editor-container")!;
const preview = document.querySelector<HTMLDivElement>("#preview")!;
const htmlSourceContainer = document.querySelector<HTMLDivElement>("#html-source")!;
const status = document.querySelector<HTMLDivElement>("#status")!;
const previewPanel = document.querySelector<HTMLDivElement>("#preview-panel")!;
const htmlPanel = document.querySelector<HTMLDivElement>("#html-panel")!;
const panelEditor = document.querySelector<HTMLDivElement>("#panel-editor")!;
const panelOutput = document.querySelector<HTMLDivElement>("#panel-output")!;
const tabPreview = document.querySelector<HTMLButtonElement>("#tab-preview")!;
const tabHtml = document.querySelector<HTMLButtonElement>("#tab-html")!;
const mobileTabEditor = document.querySelector<HTMLButtonElement>("#mobile-tab-editor")!;
const mobileTabPreview = document.querySelector<HTMLButtonElement>("#mobile-tab-preview")!;
const mobileTabHtml = document.querySelector<HTMLButtonElement>("#mobile-tab-html")!;

// --- Tab helpers ---
const TAB_ACTIVE = [
  "text-zinc-900",
  "dark:text-zinc-100",
  "border-zinc-900",
  "dark:border-zinc-100",
];
const TAB_INACTIVE = ["text-zinc-400", "dark:text-zinc-500", "border-transparent"];

function setActiveTab(btn: HTMLButtonElement, siblings: HTMLButtonElement[]) {
  for (const s of siblings) {
    s.classList.remove(...TAB_ACTIVE);
    s.classList.add(...TAB_INACTIVE);
  }
  btn.classList.remove(...TAB_INACTIVE);
  btn.classList.add(...TAB_ACTIVE);
}

// Desktop output tab (preview vs html)
function setOutputTab(tab: "preview" | "html") {
  const isPreview = tab === "preview";
  previewPanel.classList.toggle("hidden", !isPreview);
  htmlPanel.classList.toggle("hidden", isPreview);
  setActiveTab(isPreview ? tabPreview : tabHtml, [tabPreview, tabHtml]);
}

tabPreview.addEventListener("click", () => setOutputTab("preview"));
tabHtml.addEventListener("click", () => setOutputTab("html"));

// Mobile panel switching
function setMobilePanel(panel: "editor" | "preview" | "html") {
  const mobileTabs = [mobileTabEditor, mobileTabPreview, mobileTabHtml];

  panelEditor.classList.toggle("hidden", panel !== "editor");
  panelEditor.classList.toggle("flex", panel === "editor");
  panelOutput.classList.toggle("hidden", panel === "editor");
  panelOutput.classList.toggle("flex", panel !== "editor");

  if (panel === "editor") {
    setActiveTab(mobileTabEditor, mobileTabs);
  } else if (panel === "preview") {
    setActiveTab(mobileTabPreview, mobileTabs);
    setOutputTab("preview");
  } else {
    setActiveTab(mobileTabHtml, mobileTabs);
    setOutputTab("html");
  }
}

mobileTabEditor.addEventListener("click", () => setMobilePanel("editor"));
mobileTabPreview.addEventListener("click", () => setMobilePanel("preview"));
mobileTabHtml.addEventListener("click", () => setMobilePanel("html"));

// --- HTML formatter ---
const VOID_ELEMENTS = new Set([
  "area",
  "base",
  "br",
  "col",
  "embed",
  "hr",
  "img",
  "input",
  "link",
  "meta",
  "param",
  "source",
  "track",
  "wbr",
]);

function formatHtml(html: string): string {
  const tokens: string[] = [];
  let i = 0;
  while (i < html.length) {
    if (html[i] === "<") {
      const end = html.indexOf(">", i);
      if (end === -1) {
        tokens.push(html.slice(i));
        break;
      }
      tokens.push(html.slice(i, end + 1));
      i = end + 1;
    } else {
      const end = html.indexOf("<", i);
      const text = end === -1 ? html.slice(i) : html.slice(i, end);
      if (text.trim()) tokens.push(text);
      i = end === -1 ? html.length : end;
    }
  }

  const lines: string[] = [];
  let indent = 0;

  for (const token of tokens) {
    if (token.startsWith("</")) {
      indent = Math.max(0, indent - 1);
      lines.push("  ".repeat(indent) + token);
    } else if (token.startsWith("<")) {
      lines.push("  ".repeat(indent) + token);
      const match = token.match(/^<([a-zA-Z][a-zA-Z0-9]*)/);
      if (match && !token.endsWith("/>") && !VOID_ELEMENTS.has(match[1].toLowerCase())) {
        indent++;
      }
    } else {
      lines.push("  ".repeat(indent) + token);
    }
  }

  return lines.join("\n");
}

// --- CodeMirror setup ---
// zinc-950: #09090b, zinc-900: #18181b, zinc-800: #27272a, zinc-500: #71717a, zinc-300: #d4d4d8
const darkThemeExt = [
  syntaxHighlighting(oneDarkHighlightStyle),
  EditorView.theme(
    {
      "&": { backgroundColor: "#09090b" },
      ".cm-content": { caretColor: "#d4d4d8" },
      ".cm-cursor, .cm-dropCursor": { borderLeftColor: "#d4d4d8" },
      ".cm-gutters": {
        backgroundColor: "#09090b",
        color: "rgba(113, 113, 122, 0.5)",
        borderRight: "1px solid #27272a",
      },
      ".cm-activeLineGutter": { backgroundColor: "transparent" },
      ".cm-activeLine": { backgroundColor: "transparent" },
      "&.cm-focused > .cm-scroller > .cm-selectionLayer .cm-selectionBackground, .cm-selectionBackground, .cm-content ::selection":
        { backgroundColor: "rgba(113, 113, 122, 0.3)" },
      ".cm-panels": { backgroundColor: "#18181b", color: "#d4d4d8" },
      ".cm-searchMatch": { backgroundColor: "#d4d4d820", outline: "1px solid #71717a40" },
      ".cm-searchMatch.cm-searchMatch-selected": { backgroundColor: "#71717a40" },
      ".cm-selectionMatch": { backgroundColor: "#71717a30" },
      ".cm-matchingBracket, .cm-nonmatchingBracket": {
        backgroundColor: "#71717a40",
        outline: "1px solid #71717a80",
      },
      ".cm-foldPlaceholder": {
        backgroundColor: "transparent",
        border: "none",
        color: "#71717a",
      },
      ".cm-tooltip": { backgroundColor: "#18181b", border: "1px solid #27272a" },
      ".cm-tooltip .cm-tooltip-arrow:before": {
        borderTopColor: "transparent",
        borderBottomColor: "transparent",
      },
      ".cm-tooltip .cm-tooltip-arrow:after": {
        borderTopColor: "#18181b",
        borderBottomColor: "#18181b",
      },
      ".cm-tooltip-autocomplete": { "& > ul > li[aria-selected]": { backgroundColor: "#27272a" } },
    },
    { dark: true },
  ),
];

// white: #fff, zinc-50: #fafafa, zinc-100: #f4f4f5, zinc-200: #e4e4e7, zinc-400: #a1a1aa, zinc-900: #18181b
const lightThemeExt = [
  syntaxHighlighting(defaultHighlightStyle),
  EditorView.theme(
    {
      "&": { backgroundColor: "#ffffff" },
      ".cm-content": { caretColor: "#18181b" },
      ".cm-cursor, .cm-dropCursor": { borderLeftColor: "#18181b" },
      ".cm-gutters": {
        backgroundColor: "#ffffff",
        color: "rgba(161, 161, 170, 0.6)",
        borderRight: "1px solid #e4e4e7",
      },
      ".cm-activeLineGutter": { backgroundColor: "transparent" },
      ".cm-activeLine": { backgroundColor: "transparent" },
      "&.cm-focused > .cm-scroller > .cm-selectionLayer .cm-selectionBackground, .cm-selectionBackground, .cm-content ::selection":
        { backgroundColor: "rgba(0, 0, 0, 0.08)" },
      ".cm-panels": { backgroundColor: "#f4f4f5", color: "#18181b" },
      ".cm-searchMatch": { backgroundColor: "#e4e4e7", outline: "1px solid #a1a1aa40" },
      ".cm-searchMatch.cm-searchMatch-selected": { backgroundColor: "#d4d4d8" },
      ".cm-selectionMatch": { backgroundColor: "#e4e4e740" },
      ".cm-matchingBracket, .cm-nonmatchingBracket": {
        backgroundColor: "#e4e4e7",
        outline: "1px solid #a1a1aa80",
      },
      ".cm-foldPlaceholder": {
        backgroundColor: "transparent",
        border: "none",
        color: "#a1a1aa",
      },
      ".cm-tooltip": { backgroundColor: "#ffffff", border: "1px solid #e4e4e7" },
      ".cm-tooltip-autocomplete": { "& > ul > li[aria-selected]": { backgroundColor: "#f4f4f5" } },
    },
    { dark: false },
  ),
];

const editorThemeCompartment = new Compartment();
const htmlThemeCompartment = new Compartment();

function cmThemeExtension() {
  return isDark() ? darkThemeExt : lightThemeExt;
}

const baseTheme = EditorView.theme({
  "&": {
    height: "100%",
    fontSize: "0.875rem",
  },
  ".cm-scroller": {
    fontFamily: '"JetBrains Mono", ui-monospace, monospace',
    lineHeight: "1.625",
  },
  ".cm-gutters": {
    paddingRight: "4px",
  },
  ".cm-lineNumbers .cm-gutterElement": {
    paddingLeft: "12px",
    paddingRight: "8px",
    minWidth: "3em",
  },
});

const readonlyTheme = EditorView.theme({
  ".cm-cursor": {
    display: "none !important",
  },
});

const htmlView = new EditorView({
  state: EditorState.create({
    doc: "",
    extensions: [
      langHtml(),
      baseTheme,
      readonlyTheme,
      lineNumbers(),
      htmlThemeCompartment.of(cmThemeExtension()),
      EditorState.readOnly.of(true),
      EditorView.editable.of(false),
    ],
  }),
  parent: htmlSourceContainer,
});

function parseMarkdown(md: string) {
  const start = performance.now();
  const html = parse(md);
  const elapsed = (performance.now() - start).toFixed(2);
  preview.innerHTML = html;
  status.textContent = `${elapsed}ms`;

  preview.querySelectorAll("pre code").forEach((block) => {
    const lang =
      [...block.classList].find((c) => c.startsWith("language-"))?.replace("language-", "") || "";
    const code = block.textContent || "";
    if (lang) {
      const highlighted = highlightCodeString(code, lang);
      if (highlighted !== escapeHtml(code)) {
        (block as HTMLElement).innerHTML = highlighted;
      }
    }
  });

  const formatted = formatHtml(html);
  htmlView.dispatch({
    changes: { from: 0, to: htmlView.state.doc.length, insert: formatted },
  });
}

let timer: ReturnType<typeof setTimeout>;

const editorView = new EditorView({
  state: EditorState.create({
    doc: DEFAULT_MARKDOWN,
    extensions: [
      markdown(),
      baseTheme,
      lineNumbers(),
      editorThemeCompartment.of(cmThemeExtension()),
      keymap.of([indentWithTab]),
      EditorView.updateListener.of((update) => {
        if (update.docChanged) {
          clearTimeout(timer);
          timer = setTimeout(() => parseMarkdown(update.state.doc.toString()), 50);
        }
      }),
    ],
  }),
  parent: editorContainer,
});

// --- Auto theme switching ---
darkQuery.addEventListener("change", () => {
  const ext = cmThemeExtension();
  editorView.dispatch({ effects: editorThemeCompartment.reconfigure(ext) });
  htmlView.dispatch({ effects: htmlThemeCompartment.reconfigure(ext) });
  parseMarkdown(editorView.state.doc.toString());
});

status.textContent = "";
editorView.focus();
parseMarkdown(DEFAULT_MARKDOWN);
