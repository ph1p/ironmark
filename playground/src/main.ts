import "./style.css";
import { parse } from "../../wasm/pkg/ironmark.js";

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
        <span class="text-xs text-zinc-500 font-mono">ironmark</span>
      </div>
      <div id="status" class="text-xs text-zinc-500 font-mono">loading wasm…</div>
    </header>
    <div class="flex flex-1 min-h-0">
      <div class="flex-1 flex flex-col border-r border-zinc-800">
        <div class="px-4 py-2 text-xs font-medium text-zinc-400 uppercase tracking-wider border-b border-zinc-800 bg-zinc-900/50">
          Markdown
        </div>
        <textarea
          id="editor"
          class="flex-1 w-full p-4 bg-transparent text-sm font-mono text-zinc-200 resize-none outline-none placeholder:text-zinc-600 leading-relaxed"
          spellcheck="false"
          placeholder="Type markdown here..."
          disabled
        ></textarea>
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
          <pre id="html-source" class="text-sm font-mono text-zinc-300 whitespace-pre-wrap break-words leading-relaxed"></pre>
        </div>
      </div>
    </div>
  </div>
`;

const editor = document.querySelector<HTMLTextAreaElement>("#editor")!;
const preview = document.querySelector<HTMLDivElement>("#preview")!;
const htmlSource = document.querySelector<HTMLPreElement>("#html-source")!;
const status = document.querySelector<HTMLDivElement>("#status")!;
const previewPanel = document.querySelector<HTMLDivElement>("#preview-panel")!;
const htmlPanel = document.querySelector<HTMLDivElement>("#html-panel")!;
const tabPreview = document.querySelector<HTMLButtonElement>("#tab-preview")!;
const tabHtml = document.querySelector<HTMLButtonElement>("#tab-html")!;

// Tab switching
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

// Parse markdown using WASM
function parseMarkdown(md: string) {
  const start = performance.now();
  const html = parse(md);
  const elapsed = (performance.now() - start).toFixed(2);
  preview.innerHTML = html;
  htmlSource.textContent = html;
  status.textContent = `${elapsed}ms`;
}

// Debounced input
let timer: ReturnType<typeof setTimeout>;

editor.addEventListener("input", () => {
  clearTimeout(timer);
  timer = setTimeout(() => parseMarkdown(editor.value), 50);
});

// Handle tab key in editor
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

// WASM initializes eagerly with vite-plugin-wasm, no init() needed
editor.disabled = false;
editor.value = DEFAULT_MARKDOWN;
editor.focus();
parseMarkdown(DEFAULT_MARKDOWN);
