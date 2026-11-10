import { useEffect, useRef, useState } from "react";
import { Compartment, EditorState } from "@codemirror/state";
import { EditorView, lineNumbers } from "@codemirror/view";
import { html } from "@codemirror/lang-html";
import { markdown } from "@codemirror/lang-markdown";
import wasmUrl from "ironmark/ironmark.wasm?url";
import { CopyButton } from "./CopyButton";
import { baseTheme, readonlyTheme } from "../editor/setup";
import { cmThemeExtension, subscribeThemeChange } from "../editor/theme";

const DEFAULT_HTML = `<h1>Hello World</h1>

<p>This is a <strong>bold</strong> and <em>italic</em> paragraph.</p>

<h2>Features</h2>

<ul>
  <li>Converts HTML to Markdown</li>
  <li>Supports <a href="https://example.com">links</a></li>
  <li>Handles <code>inline code</code></li>
</ul>

<blockquote>
  <p>This is a blockquote with <mark>highlighted</mark> text.</p>
</blockquote>

<pre><code class="language-javascript">function greet(name) {
  console.log(\`Hello, \${name}!\`);
}
</code></pre>

<h3>Table Example</h3>

<table>
  <thead>
    <tr>
      <th>Name</th>
      <th>Value</th>
    </tr>
  </thead>
  <tbody>
    <tr>
      <td>Alpha</td>
      <td>1</td>
    </tr>
    <tr>
      <td>Beta</td>
      <td>2</td>
    </tr>
  </tbody>
</table>

<hr>

<p>End of document.</p>`;

export function HtmlToMarkdownPage() {
  const [output, setOutput] = useState("");
  const [error, setError] = useState<string | null>(null);
  const [isLoading, setIsLoading] = useState(true);
  const [preserveUnknown, setPreserveUnknown] = useState(false);
  const inputRef = useRef<HTMLDivElement>(null);
  const outputRef = useRef<HTMLDivElement>(null);
  const inputEditorRef = useRef<EditorView | null>(null);
  const outputEditorRef = useRef<EditorView | null>(null);
  const ironmarkRef = useRef<typeof import("ironmark") | null>(null);
  const currentHtmlRef = useRef(DEFAULT_HTML);
  const preserveUnknownRef = useRef(preserveUnknown);
  const inputThemeCompartment = useRef(new Compartment());
  const outputThemeCompartment = useRef(new Compartment());

  preserveUnknownRef.current = preserveUnknown;

  const doConvert = (htmlContent: string) => {
    if (!ironmarkRef.current) return;

    try {
      const md = ironmarkRef.current.htmlToMarkdown(htmlContent, {
        preserveUnknownAsHtml: preserveUnknownRef.current,
      });
      setOutput(md);
      setError(null);
    } catch (e) {
      setError(e instanceof Error ? e.message : "Conversion failed");
    }
  };

  // Initialize ironmark
  useEffect(() => {
    import("ironmark").then(async (mod) => {
      await mod.init(wasmUrl);
      ironmarkRef.current = mod;
      setIsLoading(false);
      doConvert(currentHtmlRef.current);
    });
  }, []);

  // Setup input editor
  useEffect(() => {
    if (!inputRef.current) return;

    const view = new EditorView({
      state: EditorState.create({
        doc: DEFAULT_HTML,
        extensions: [
          html(),
          baseTheme,
          lineNumbers(),
          inputThemeCompartment.current.of(cmThemeExtension()),
          EditorView.lineWrapping,
          EditorView.updateListener.of((update) => {
            if (update.docChanged) {
              const htmlContent = update.state.doc.toString();
              currentHtmlRef.current = htmlContent;
              doConvert(htmlContent);
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

  // Re-convert when preserveUnknown changes
  useEffect(() => {
    if (ironmarkRef.current) {
      doConvert(currentHtmlRef.current);
    }
  }, [preserveUnknown]);

  return (
    <div className="flex-1 flex flex-col overflow-hidden">
      {/* Options Bar */}
      <div className="px-4 py-2 border-b border-zinc-200 dark:border-zinc-800 bg-zinc-50 dark:bg-zinc-900 shrink-0 flex items-center gap-4">
        <label className="flex items-center gap-2 text-sm">
          <input
            type="checkbox"
            checked={preserveUnknown}
            onChange={(e) => setPreserveUnknown(e.target.checked)}
            className="rounded"
          />
          <span className="text-zinc-700 dark:text-zinc-300">Preserve unknown tags as HTML</span>
        </label>
        {isLoading && (
          <span className="text-xs text-zinc-500 dark:text-zinc-400">Loading WASM...</span>
        )}
      </div>

      <div className="flex-1 flex flex-col md:flex-row overflow-hidden">
        {/* Input Panel */}
        <div className="flex-1 flex flex-col min-h-0 border-r border-zinc-200 dark:border-zinc-800">
          <div className="px-4 py-2 border-b border-zinc-200 dark:border-zinc-800 bg-zinc-50 dark:bg-zinc-900 shrink-0 flex items-center justify-between">
            <div>
              <h2 className="text-sm font-medium">HTML Input</h2>
              <p className="text-xs text-zinc-500 dark:text-zinc-400">Enter HTML to convert</p>
            </div>
            <CopyButton getText={() => currentHtmlRef.current} />
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
              ) : (
                <p className="text-xs text-zinc-500 dark:text-zinc-400">Generated markdown</p>
              )}
            </div>
            <CopyButton getText={() => output} />
          </div>
          <div ref={outputRef} className="flex-1 overflow-hidden" />
        </div>
      </div>
    </div>
  );
}
