import { Compartment, EditorState, type Extension } from "@codemirror/state";
import { EditorView, keymap, lineNumbers } from "@codemirror/view";
import { markdown } from "@codemirror/lang-markdown";
import { html as langHtml } from "@codemirror/lang-html";
import { javascript } from "@codemirror/lang-javascript";
import { indentWithTab } from "@codemirror/commands";
import { cmThemeExtension } from "./theme";

export const htmlThemeCompartment = new Compartment();

export const baseTheme = EditorView.theme({
  "&": { height: "100%", fontSize: "0.875rem" },
  ".cm-scroller": {
    fontFamily: '"JetBrains Mono", ui-monospace, monospace',
    lineHeight: "1.625",
    overflow: "auto",
  },
  ".cm-gutters": { paddingRight: "4px" },
  ".cm-lineNumbers .cm-gutterElement": {
    paddingLeft: "12px",
    paddingRight: "8px",
    minWidth: "3em",
  },
});

export const readonlyTheme = EditorView.theme({
  ".cm-cursor": { display: "none !important" },
});

export const markdownEditorExtensions = [
  markdown(),
  baseTheme,
  lineNumbers(),
  keymap.of([indentWithTab]),
];

export const jsxEditorExtensions = [
  javascript({ jsx: true }),
  baseTheme,
  lineNumbers(),
  keymap.of([indentWithTab]),
];

export function createHtmlView(parent: HTMLElement): EditorView {
  return createReadonlyView(parent, langHtml());
}

export function createJsxView(parent: HTMLElement): EditorView {
  return createReadonlyView(parent, javascript({ jsx: true }));
}

function createReadonlyView(parent: HTMLElement, language: Extension): EditorView {
  return new EditorView({
    state: EditorState.create({
      doc: "",
      extensions: [
        language,
        baseTheme,
        readonlyTheme,
        lineNumbers(),
        htmlThemeCompartment.of(cmThemeExtension()),
        EditorState.readOnly.of(true),
        EditorView.editable.of(false),
      ],
    }),
    parent,
  });
}
