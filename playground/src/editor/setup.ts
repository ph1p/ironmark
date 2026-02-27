import { Compartment, EditorState } from "@codemirror/state";
import { EditorView, keymap, lineNumbers } from "@codemirror/view";
import { markdown } from "@codemirror/lang-markdown";
import { html as langHtml } from "@codemirror/lang-html";
import { json as langJson } from "@codemirror/lang-json";
import { indentWithTab } from "@codemirror/commands";
import { cmThemeExtension } from "./theme";

export const editorThemeCompartment = new Compartment();
export const htmlThemeCompartment = new Compartment();
export const astThemeCompartment = new Compartment();

const baseTheme = EditorView.theme({
  "&": { height: "100%", fontSize: "0.875rem" },
  ".cm-scroller": {
    fontFamily: '"JetBrains Mono", ui-monospace, monospace',
    lineHeight: "1.625",
  },
  ".cm-gutters": { paddingRight: "4px" },
  ".cm-lineNumbers .cm-gutterElement": {
    paddingLeft: "12px",
    paddingRight: "8px",
    minWidth: "3em",
  },
});

const readonlyTheme = EditorView.theme({
  ".cm-cursor": { display: "none !important" },
});

export function createHtmlView(parent: HTMLElement): EditorView {
  return new EditorView({
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
    parent,
  });
}

export function createAstView(parent: HTMLElement): EditorView {
  return new EditorView({
    state: EditorState.create({
      doc: "",
      extensions: [
        langJson(),
        baseTheme,
        readonlyTheme,
        lineNumbers(),
        astThemeCompartment.of(cmThemeExtension()),
        EditorState.readOnly.of(true),
        EditorView.editable.of(false),
      ],
    }),
    parent,
  });
}

export function createEditorView(
  parent: HTMLElement,
  doc: string,
  onDocChanged: (doc: string) => void,
): EditorView {
  let timer: ReturnType<typeof setTimeout>;
  return new EditorView({
    state: EditorState.create({
      doc,
      extensions: [
        markdown(),
        baseTheme,
        lineNumbers(),
        editorThemeCompartment.of(cmThemeExtension()),
        keymap.of([indentWithTab]),
        EditorView.updateListener.of((update) => {
          if (update.docChanged) {
            clearTimeout(timer);
            timer = setTimeout(() => {
              requestAnimationFrame(() => onDocChanged(update.state.doc.toString()));
            }, 50);
          }
        }),
      ],
    }),
    parent,
  });
}
