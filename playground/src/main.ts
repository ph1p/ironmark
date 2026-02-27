import "./style.css";
import { init, parse, parseToAst } from "ironmark";
import wasmUrl from "ironmark/ironmark.wasm?url";

import {
  preview,
  htmlPanel,
  astPanel,
  htmlSourceContainer,
  astSourceContainer,
  editorContainer,
  status,
} from "./layout/app";
import { darkQuery, cmThemeExtension } from "./editor/theme";
import { highlightCodeBlocks } from "./editor/highlight";
import { formatHtml } from "./util/format-html";
import {
  createEditorView,
  createHtmlView,
  createAstView,
  editorThemeCompartment,
  htmlThemeCompartment,
  astThemeCompartment,
} from "./editor/setup";
import { initTabs } from "./layout/tabs";

await init(wasmUrl);

const DEFAULT_MARKDOWN = `# Markdown Playground

Write **markdown** on the left and see the _rendered HTML_ on the right.

## Features

- Live preview as you type
- Supports **bold**, *italic*, and \`code\`
- Links: [Example](https://example.com)
- Images: ![alt](https://placeholdit.com/200x200/dddddd/999999?font=inter)

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

const htmlState = { dirty: false, lastHtml: "", astDirty: false, lastAst: "" };
const htmlView = createHtmlView(htmlSourceContainer);
const astView = createAstView(astSourceContainer);
let highlightRaf = 0;
let htmlUpdateRaf = 0;
let astUpdateRaf = 0;

function parseMarkdown(md: string) {
  const t0 = performance.now();
  const html = parse(md);
  status.textContent = `${(performance.now() - t0).toFixed(2)}ms`;
  preview.innerHTML = html;
  htmlState.lastHtml = html;

  cancelAnimationFrame(highlightRaf);
  highlightRaf = requestAnimationFrame(() => highlightCodeBlocks(preview));

  cancelAnimationFrame(htmlUpdateRaf);
  if (!htmlPanel.classList.contains("hidden")) {
    htmlUpdateRaf = requestAnimationFrame(() => {
      htmlView.dispatch({
        changes: { from: 0, to: htmlView.state.doc.length, insert: formatHtml(html) },
      });
    });
  } else {
    htmlState.dirty = true;
  }

  const astJson = parseToAst(md);
  let formatted: string;
  try {
    formatted = JSON.stringify(JSON.parse(astJson), null, 2);
  } catch {
    formatted = astJson;
  }
  htmlState.lastAst = formatted;

  cancelAnimationFrame(astUpdateRaf);
  if (!astPanel.classList.contains("hidden")) {
    astUpdateRaf = requestAnimationFrame(() => {
      astView.dispatch({
        changes: { from: 0, to: astView.state.doc.length, insert: formatted },
      });
    });
  } else {
    htmlState.astDirty = true;
  }
}

const editorView = createEditorView(editorContainer, DEFAULT_MARKDOWN, parseMarkdown);
initTabs(htmlView, astView, htmlState);

darkQuery.addEventListener("change", () => {
  const ext = cmThemeExtension();
  editorView.dispatch({ effects: editorThemeCompartment.reconfigure(ext) });
  htmlView.dispatch({ effects: htmlThemeCompartment.reconfigure(ext) });
  astView.dispatch({ effects: astThemeCompartment.reconfigure(ext) });
  parseMarkdown(editorView.state.doc.toString());
});

status.textContent = "";
editorView.focus();
parseMarkdown(DEFAULT_MARKDOWN);
