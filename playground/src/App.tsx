import { useEffect, useRef, useState } from "react";
import type { EditorView } from "@codemirror/view";
import { Header } from "./components/Header";
import { MobileTabs } from "./components/MobileTabs";
import { SplitPanels } from "./components/SplitPanels";
import { DEFAULT_MARKDOWN, initPlayground, type PlaygroundController } from "./playground";
import type { MobilePanel, OutputTab } from "./components/types";

const MARKDOWN_STORAGE_KEY = "playground:markdown";
const OUTPUT_TAB_STORAGE_KEY = "playground:output-tab";
const MOBILE_PANEL_STORAGE_KEY = "playground:mobile-panel";

function readOutputTab(): OutputTab {
  const value = localStorage.getItem(OUTPUT_TAB_STORAGE_KEY);
  return value === "html" || value === "ast" ? value : "preview";
}

function readMobilePanel(): MobilePanel {
  const value = localStorage.getItem(MOBILE_PANEL_STORAGE_KEY);
  return value === "preview" || value === "html" || value === "ast" ? value : "editor";
}

export function App() {
  const [statusText, setStatusText] = useState("loading wasm...");
  const [outputTab, setOutputTab] = useState<OutputTab>(() => readOutputTab());
  const [mobilePanel, setMobilePanel] = useState<MobilePanel>(() => readMobilePanel());
  const [markdown, setMarkdown] = useState(() => localStorage.getItem(MARKDOWN_STORAGE_KEY) ?? DEFAULT_MARKDOWN);
  const outputTabRef = useRef(outputTab);
  const controllerRef = useRef<PlaygroundController | null>(null);
  const pendingEditorRef = useRef<EditorView | null>(null);
  const previewRef = useRef<HTMLDivElement>(null);
  const htmlSourceContainerRef = useRef<HTMLDivElement>(null);
  const astSourceContainerRef = useRef<HTMLDivElement>(null);

  outputTabRef.current = outputTab;

  useEffect(() => {
    let disposed = false;

    const preview = previewRef.current;
    const htmlSourceContainer = htmlSourceContainerRef.current;
    const astSourceContainer = astSourceContainerRef.current;
    if (!preview || !htmlSourceContainer || !astSourceContainer) {
      return;
    }

    void initPlayground({
      preview,
      htmlSourceContainer,
      astSourceContainer,
      getOutputTab: () => outputTabRef.current,
      onStatusChange: setStatusText,
    })
      .then((controller) => {
        if (disposed) {
          controller.dispose();
          return;
        }
        controllerRef.current = controller;
        controller.setOutputTab(outputTabRef.current);
        controller.updateMarkdown(markdown);
        if (pendingEditorRef.current) {
          controller.attachEditorView(pendingEditorRef.current);
        }
      })
      .catch(() => {
        if (!disposed) {
          setStatusText("failed to load wasm");
        }
      });

    return () => {
      disposed = true;
      controllerRef.current?.dispose();
      controllerRef.current = null;
    };
  }, []);

  useEffect(() => {
    controllerRef.current?.setOutputTab(outputTab);
  }, [outputTab]);

  useEffect(() => {
    localStorage.setItem(MARKDOWN_STORAGE_KEY, markdown);
  }, [markdown]);

  useEffect(() => {
    localStorage.setItem(OUTPUT_TAB_STORAGE_KEY, outputTab);
  }, [outputTab]);

  useEffect(() => {
    localStorage.setItem(MOBILE_PANEL_STORAGE_KEY, mobilePanel);
  }, [mobilePanel]);

  const onMobilePanelChange = (panel: MobilePanel) => {
    setMobilePanel(panel);
    if (panel !== "editor") {
      setOutputTab(panel);
    }
  };

  const onEditorReady = (view: EditorView) => {
    pendingEditorRef.current = view;
    controllerRef.current?.attachEditorView(view);
  };

  const onMarkdownDocChange = (value: string) => {
    controllerRef.current?.updateMarkdown(value);
  };

  return (
    <div className="h-full flex flex-col bg-white dark:bg-zinc-950 text-zinc-900 dark:text-zinc-100 transition-colors">
      <Header statusText={statusText} />
      <MobileTabs panel={mobilePanel} onChange={onMobilePanelChange} />
      <SplitPanels
        mobilePanel={mobilePanel}
        outputTab={outputTab}
        onOutputTabChange={setOutputTab}
        markdown={markdown}
        onMarkdownValueChange={setMarkdown}
        onMarkdownDocChange={onMarkdownDocChange}
        onEditorReady={onEditorReady}
        previewRef={previewRef}
        htmlSourceContainerRef={htmlSourceContainerRef}
        astSourceContainerRef={astSourceContainerRef}
      />
    </div>
  );
}
