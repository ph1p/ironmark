import "./style.css";
import { init, parse } from "ironmark";
import wasmUrl from "ironmark/ironmark.wasm?url";
import { createHighlighter } from "shiki";
import { version } from "../../package.json";

const [, highlighter] = await Promise.all([
  init(wasmUrl),
  createHighlighter({
    themes: ["github-dark-default"],
    langs: [
      "javascript",
      "typescript",
      "rust",
      "html",
      "css",
      "json",
      "bash",
      "python",
      "markdown",
      "yaml",
      "toml",
    ],
  }),
]);

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
  <div class="h-full flex flex-col bg-zinc-950 text-zinc-100">
    <header class="flex items-center justify-between px-5 py-3 border-b border-zinc-800 shrink-0">
      <div class="flex items-center gap-3">
        <h1 class="text-base font-semibold tracking-tight">Markdown Playground</h1>
        <span class="text-xs text-zinc-500 font-mono">ironmark v${version}</span>
      </div>
      <div class="flex items-center gap-3">
        <div id="status" class="text-xs text-zinc-500 font-mono">loading wasm…</div>
        <div class="flex items-center gap-2">
          <a href="https://github.com/ph1p/ironmark" target="_blank" rel="noopener noreferrer" class="text-zinc-500 hover:text-zinc-300 transition-colors" title="GitHub">
            <svg width="16" height="16" viewBox="0 0 24 24" fill="currentColor"><path d="M12 0C5.37 0 0 5.37 0 12c0 5.31 3.435 9.795 8.205 11.385.6.105.825-.255.825-.57 0-.285-.015-1.23-.015-2.235-3.015.555-3.795-.735-4.035-1.41-.135-.345-.72-1.41-1.23-1.695-.42-.225-1.02-.78-.015-.795.945-.015 1.62.87 1.845 1.23 1.08 1.815 2.805 1.305 3.495.99.105-.78.42-1.305.765-1.605-2.67-.3-5.46-1.335-5.46-5.925 0-1.305.465-2.385 1.23-3.225-.12-.3-.54-1.53.12-3.18 0 0 1.005-.315 3.3 1.23.96-.27 1.98-.405 3-.405s2.04.135 3 .405c2.295-1.56 3.3-1.23 3.3-1.23.66 1.65.24 2.88.12 3.18.765.84 1.23 1.905 1.23 3.225 0 4.605-2.805 5.625-5.475 5.925.435.375.81 1.095.81 2.22 0 1.605-.015 2.895-.015 3.3 0 .315.225.69.825.57A12.02 12.02 0 0024 12c0-6.63-5.37-12-12-12z"/></svg>
          </a>
          <a href="https://www.npmjs.com/package/ironmark" target="_blank" rel="noopener noreferrer" class="text-zinc-500 hover:text-zinc-300 transition-colors" title="npm">
            <svg width="16" height="16" viewBox="0 0 24 24" fill="currentColor"><path d="M0 7.334v8h6.666v1.332H12v-1.332h12v-8H0zm6.666 6.664H5.334v-4H3.999v4H1.335V8.667h5.331v5.331zm4 0v1.336H8.001V8.667h5.334v5.332h-2.669v-.001zm12.001 0h-1.33v-4h-1.336v4h-1.335v-4h-1.33v4h-2.671V8.667h8.002v5.331z"/></svg>
          </a>
          <a href="https://crates.io/crates/ironmark" target="_blank" rel="noopener noreferrer" class="text-zinc-500 hover:text-zinc-300 transition-colors" title="crates.io">
            <svg width="16" height="16" viewBox="0 0 24 24" fill="currentColor"><path d="M23.998 12.014c-.003-2.298-.656-4.408-1.782-6.2l-.063-.09-2.636 1.536c-.263-.4-.555-.78-.876-1.132l2.636-1.534a11.94 11.94 0 00-4.598-3.86L16.6 0.61l-1.524 2.642a10.923 10.923 0 00-1.4-.322V0h-.07A11.922 11.922 0 0012.002 0h-.07v3.056c-.482.06-.955.155-1.414.326L8.994.74l-.08.044a11.918 11.918 0 00-4.6 3.862l2.637 1.535c-.32.35-.613.73-.876 1.13L3.44 5.776l-.063.09A11.94 11.94 0 001.595 12.07h3.06c.012.482.068.955.168 1.414l-2.642 1.524.044.08a11.926 11.926 0 003.862 4.6l1.534-2.637c.35.32.732.613 1.132.876l-1.536 2.636.09.063a11.924 11.924 0 006.2 1.782v-3.072c.478-.016.95-.07 1.414-.172l1.524 2.642.08-.044a11.918 11.918 0 004.6-3.862l-2.637-1.534c.32-.352.613-.732.876-1.132l2.636 1.536.063-.09a11.924 11.924 0 001.782-6.202v-.07h-3.06a10.927 10.927 0 00-.168-1.414l2.642-1.524-.044-.08a11.926 11.926 0 00-3.862-4.6zM12 16.5a4.5 4.5 0 110-9 4.5 4.5 0 010 9z"/></svg>
          </a>
        </div>
      </div>
    </header>
    <div class="flex flex-1 min-h-0">
      <div class="flex-1 flex flex-col border-r border-zinc-800">
        <div class="px-4 py-2 text-xs font-medium text-zinc-400 uppercase tracking-wider border-b border-zinc-800 bg-zinc-900/50">
          Markdown
        </div>
        <div class="flex-1 relative min-h-0">
          <div id="editor-highlight" class="absolute inset-0 p-4 overflow-auto pointer-events-none text-sm font-mono leading-relaxed whitespace-pre-wrap break-words" aria-hidden="true"></div>
          <textarea
            id="editor"
            class="absolute inset-0 w-full h-full p-4 bg-transparent text-sm font-mono text-transparent caret-zinc-200 resize-none outline-none placeholder:text-zinc-600 leading-relaxed whitespace-pre-wrap break-words"
            spellcheck="false"
            placeholder="Type markdown here..."
            disabled
          ></textarea>
        </div>
      </div>
      <div class="flex-1 flex flex-col">
        <div class="flex border-b border-zinc-800 bg-zinc-900/50">
          <button id="tab-preview" class="tab-btn px-4 py-2 text-xs font-medium uppercase tracking-wider text-zinc-100 border-b-2 border-zinc-100">
            Preview
          </button>
          <button id="tab-html" class="tab-btn px-4 py-2 text-xs font-medium uppercase tracking-wider text-zinc-500 border-b-2 border-transparent hover:text-zinc-300">
            HTML
          </button>
        </div>
        <div id="preview-panel" class="flex-1 overflow-auto p-5">
          <div id="preview" class="prose"></div>
        </div>
        <div id="html-panel" class="flex-1 overflow-auto p-4 hidden">
          <div id="html-source" class="text-sm font-mono leading-relaxed"></div>
        </div>
      </div>
    </div>
  </div>
`;

const editor = document.querySelector<HTMLTextAreaElement>("#editor")!;
const editorHighlight = document.querySelector<HTMLDivElement>("#editor-highlight")!;
const preview = document.querySelector<HTMLDivElement>("#preview")!;
const htmlSource = document.querySelector<HTMLDivElement>("#html-source")!;
const status = document.querySelector<HTMLDivElement>("#status")!;
const previewPanel = document.querySelector<HTMLDivElement>("#preview-panel")!;
const htmlPanel = document.querySelector<HTMLDivElement>("#html-panel")!;
const tabPreview = document.querySelector<HTMLButtonElement>("#tab-preview")!;
const tabHtml = document.querySelector<HTMLButtonElement>("#tab-html")!;

function setTab(tab: "preview" | "html") {
  const isPreview = tab === "preview";
  previewPanel.classList.toggle("hidden", !isPreview);
  htmlPanel.classList.toggle("hidden", isPreview);
  tabPreview.classList.toggle("text-zinc-100", isPreview);
  tabPreview.classList.toggle("border-zinc-100", isPreview);
  tabPreview.classList.toggle("text-zinc-500", !isPreview);
  tabPreview.classList.toggle("border-transparent", !isPreview);
  tabHtml.classList.toggle("text-zinc-100", !isPreview);
  tabHtml.classList.toggle("border-zinc-100", !isPreview);
  tabHtml.classList.toggle("text-zinc-500", isPreview);
  tabHtml.classList.toggle("border-transparent", isPreview);
}

tabPreview.addEventListener("click", () => setTab("preview"));
tabHtml.addEventListener("click", () => setTab("html"));

function highlightEditor(md: string) {
  editorHighlight.innerHTML = highlighter.codeToHtml(md, {
    lang: "markdown",
    theme: "github-dark-default",
  });

  const pre = editorHighlight.querySelector("pre");
  if (pre) {
    pre.style.margin = "0";
    pre.style.padding = "0";
    pre.style.background = "transparent";
    pre.style.whiteSpace = "pre-wrap";
    pre.style.wordBreak = "break-word";
  }
}

function parseMarkdown(md: string) {
  const start = performance.now();
  const html = parse(md);
  const elapsed = (performance.now() - start).toFixed(2);
  preview.innerHTML = html;
  status.textContent = `${elapsed}ms`;

  preview.querySelectorAll("pre code").forEach((block) => {
    const lang =
      [...block.classList].find((c) => c.startsWith("language-"))?.replace("language-", "") ||
      "text";
    const code = block.textContent || "";
    try {
      const highlighted = highlighter.codeToHtml(code, {
        lang,
        theme: "github-dark-default",
      });
      block.parentElement!.outerHTML = highlighted;
    } catch {
      // Language not loaded — leave as-is
    }
  });

  htmlSource.innerHTML = highlighter.codeToHtml(html, {
    lang: "html",
    theme: "github-dark-default",
  });

  highlightEditor(md);
}

editor.addEventListener("scroll", () => {
  editorHighlight.scrollTop = editor.scrollTop;
  editorHighlight.scrollLeft = editor.scrollLeft;
});

let timer: ReturnType<typeof setTimeout>;

editor.addEventListener("input", () => {
  clearTimeout(timer);
  timer = setTimeout(() => parseMarkdown(editor.value), 50);
});

editor.addEventListener("keydown", (e) => {
  if (e.key === "Tab") {
    e.preventDefault();
    const start = editor.selectionStart;
    const end = editor.selectionEnd;
    editor.value = editor.value.substring(0, start) + "  " + editor.value.substring(end);
    editor.selectionStart = editor.selectionEnd = start + 2;
    editor.dispatchEvent(new Event("input"));
  }
});

editor.disabled = false;
editor.value = DEFAULT_MARKDOWN;
editor.focus();
parseMarkdown(DEFAULT_MARKDOWN);
