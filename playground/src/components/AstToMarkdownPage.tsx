import { useEffect, useRef, useState } from "react";
import { Compartment, EditorState } from "@codemirror/state";
import { EditorView, lineNumbers } from "@codemirror/view";
import { json } from "@codemirror/lang-json";
import { markdown } from "@codemirror/lang-markdown";
import wasmUrl from "ironmark/ironmark.wasm?url";
import { CopyButton } from "./CopyButton";
import { baseTheme, readonlyTheme } from "../editor/setup";
import { cmThemeExtension, subscribeThemeChange } from "../editor/theme";

const DEFAULT_AST = `{
  "Document": {
    "children": [
      {
        "Heading": {
          "level": 1,
          "raw": "Hello World"
        }
      },
      {
        "Paragraph": {
          "raw": "This is a **bold** and *italic* paragraph."
        }
      },
      {
        "List": {
          "kind": { "Bullet": 45 },
          "start": 1,
          "tight": true,
          "children": [
            {
              "ListItem": {
                "children": [{ "Paragraph": { "raw": "Item 1" } }],
                "checked": null
              }
            },
            {
              "ListItem": {
                "children": [{ "Paragraph": { "raw": "Item 2" } }],
                "checked": null
              }
            }
          ]
        }
      }
    ]
  }
}`;

export function AstToMarkdownPage() {
  const [output, setOutput] = useState("");
  const [error, setError] = useState<string | null>(null);
  const [isLoading, setIsLoading] = useState(true);
  const inputRef = useRef<HTMLDivElement>(null);
  const outputRef = useRef<HTMLDivElement>(null);
  const inputEditorRef = useRef<EditorView | null>(null);
  const outputEditorRef = useRef<EditorView | null>(null);
  const ironmarkRef = useRef<typeof import("ironmark") | null>(null);
  const currentAstRef = useRef(DEFAULT_AST);
  const inputThemeCompartment = useRef(new Compartment());
  const outputThemeCompartment = useRef(new Compartment());

  const doConvert = (astJson: string) => {
    if (!ironmarkRef.current) return;

    try {
      JSON.parse(astJson);
      const md = ironmarkRef.current.astToMarkdown(astJson);
      setOutput(md);
      setError(null);
    } catch (e) {
      setError(e instanceof Error ? e.message : "Invalid JSON");
    }
  };

  // Initialize ironmark
  useEffect(() => {
    import("ironmark").then(async (mod) => {
      await mod.init(wasmUrl);
      ironmarkRef.current = mod;
      setIsLoading(false);
      doConvert(currentAstRef.current);
    });
  }, []);

  // Setup input editor
  useEffect(() => {
    if (!inputRef.current) return;

    const view = new EditorView({
      state: EditorState.create({
        doc: DEFAULT_AST,
        extensions: [
          json(),
          baseTheme,
          lineNumbers(),
          inputThemeCompartment.current.of(cmThemeExtension()),
          EditorView.lineWrapping,
          EditorView.updateListener.of((update) => {
            if (update.docChanged) {
              const value = update.state.doc.toString();
              currentAstRef.current = value;
              doConvert(value);
            }
          }),
        ],
      }),
      parent: inputRef.current,
    });

    inputEditorRef.current = view;

    return () => {
      view.destroy();
    };
  }, []);

  // Setup output editor
  useEffect(() => {
    if (!outputRef.current) return;

    const view = new EditorView({
      state: EditorState.create({
        doc: output,
        extensions: [
          markdown(),
          baseTheme,
          readonlyTheme,
          lineNumbers(),
          outputThemeCompartment.current.of(cmThemeExtension()),
          EditorView.lineWrapping,
          EditorState.readOnly.of(true),
          EditorView.editable.of(false),
        ],
      }),
      parent: outputRef.current,
    });

    outputEditorRef.current = view;

    return () => {
      view.destroy();
    };
  }, []);

  // Subscribe to theme changes
  useEffect(() => {
    return subscribeThemeChange(() => {
      if (inputEditorRef.current) {
        inputEditorRef.current.dispatch({
          effects: inputThemeCompartment.current.reconfigure(cmThemeExtension()),
        });
      }
      if (outputEditorRef.current) {
        outputEditorRef.current.dispatch({
          effects: outputThemeCompartment.current.reconfigure(cmThemeExtension()),
        });
      }
    });
  }, []);

  // Update output editor when output changes
  useEffect(() => {
    if (outputEditorRef.current) {
      outputEditorRef.current.dispatch({
        changes: {
          from: 0,
          to: outputEditorRef.current.state.doc.length,
          insert: output,
        },
      });
    }
  }, [output]);

  return (
    <div className="flex-1 flex flex-col md:flex-row overflow-hidden">
      {/* Input Panel */}
      <div className="flex-1 flex flex-col min-h-0 border-r border-zinc-200 dark:border-zinc-800">
        <div className="px-4 py-2 border-b border-zinc-200 dark:border-zinc-800 bg-zinc-50 dark:bg-zinc-900 shrink-0 flex items-center justify-between">
          <div>
            <h2 className="text-sm font-medium">AST (JSON)</h2>
            <p className="text-xs text-zinc-500 dark:text-zinc-400">
              Paste an ironmark AST in JSON format
            </p>
          </div>
          <CopyButton getText={() => currentAstRef.current} />
        </div>
        <div ref={inputRef} className="flex-1 overflow-hidden" />
      </div>

      {/* Output Panel */}
      <div className="flex-1 flex flex-col min-h-0">
        <div className="px-4 py-2 border-b border-zinc-200 dark:border-zinc-800 bg-zinc-50 dark:bg-zinc-900 shrink-0 flex items-center justify-between">
          <div>
            <h2 className="text-sm font-medium">Markdown Output</h2>
            {error ? (
              <p className="text-xs text-red-500">{error}</p>
            ) : isLoading ? (
              <p className="text-xs text-zinc-500">Loading...</p>
            ) : (
              <p className="text-xs text-zinc-500 dark:text-zinc-400">Generated markdown</p>
            )}
          </div>
          <CopyButton getText={() => output} />
        </div>
        <div ref={outputRef} className="flex-1 overflow-hidden" />
      </div>
    </div>
  );
}
