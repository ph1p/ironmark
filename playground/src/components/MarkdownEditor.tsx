import { useEffect, useMemo, useRef, useState } from "react";
import CodeMirror from "@uiw/react-codemirror";
import type { EditorView } from "@codemirror/view";
import { Compartment, type Extension } from "@codemirror/state";
import { jsxEditorExtensions, markdownEditorExtensions } from "../editor/setup";
import { cmThemeExtension, subscribeThemeChange } from "../editor/theme";

type MarkdownEditorProps = {
  value: string;
  onValueChange: (value: string) => void;
  onDocChange: (value: string) => void;
  onEditorReady: (view: EditorView) => void;
  /** Editor language; component files of the MDX workspace use `jsx`. */
  language?: "markdown" | "jsx";
};

export function MarkdownEditor({
  value,
  onValueChange,
  onDocChange,
  onEditorReady,
  language = "markdown",
}: MarkdownEditorProps) {
  const debounceRef = useRef<ReturnType<typeof setTimeout> | null>(null);
  const editorRef = useRef<EditorView | null>(null);
  const [uiwTheme, setUiwTheme] = useState<Extension>(cmThemeExtension());
  const themeCompartmentRef = useRef(new Compartment());
  const extensions = useMemo(
    () => [
      ...(language === "jsx" ? jsxEditorExtensions : markdownEditorExtensions),
      themeCompartmentRef.current.of(cmThemeExtension()),
    ],
    [language],
  );

  useEffect(() => {
    return () => {
      if (debounceRef.current) {
        clearTimeout(debounceRef.current);
      }
    };
  }, []);

  useEffect(() => {
    return subscribeThemeChange(() => {
      setUiwTheme(cmThemeExtension());
      if (!editorRef.current) {
        return;
      }
      editorRef.current.dispatch({
        effects: themeCompartmentRef.current.reconfigure(cmThemeExtension()),
      });
    });
  }, []);

  return (
    <CodeMirror
      value={value}
      height="100%"
      style={{ height: "100%" }}
      theme={uiwTheme}
      extensions={extensions}
      onCreateEditor={(view) => {
        editorRef.current = view;
        onEditorReady(view);
      }}
      onChange={(next) => {
        onValueChange(next);
        if (debounceRef.current) {
          clearTimeout(debounceRef.current);
        }
        debounceRef.current = setTimeout(() => {
          requestAnimationFrame(() => onDocChange(next));
        }, 50);
      }}
      basicSetup={{
        lineNumbers: false,
        foldGutter: false,
        highlightActiveLine: false,
        highlightActiveLineGutter: false,
      }}
    />
  );
}
