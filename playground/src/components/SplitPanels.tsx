import type { RefObject } from "react";
import { OutputTabs } from "./OutputTabs";
import { OptionsPanel } from "./OptionsPanel";
import type { MobilePanel, OutputTab, ParseOptions } from "./types";
import { MarkdownEditor } from "./MarkdownEditor";
import { CopyButton } from "./CopyButton";
import type { EditorView } from "@codemirror/view";
import { FileTabs } from "./FileTabs";
import type { PlaygroundFile } from "../mdx/runtime";
import { isMdx } from "../mdx/defaults";

/** MDX mode swaps the single markdown document for a multi-file workspace. */
export type Workspace = {
  files: PlaygroundFile[];
  active: string;
  onSelect: (name: string) => void;
  onAdd: (name: string) => void;
  onRemove: (name: string) => void;
  onValueChange: (value: string) => void;
  onDocChange: (value: string) => void;
};

type SplitPanelsProps = {
  mobilePanel: MobilePanel;
  outputTab: OutputTab;
  onOutputTabChange: (tab: OutputTab) => void;
  markdown: string;
  onMarkdownValueChange: (value: string) => void;
  onMarkdownDocChange: (value: string) => void;
  onEditorReady: (view: EditorView) => void;
  options: ParseOptions;
  onOptionsChange: (options: ParseOptions) => void;
  previewRef: RefObject<HTMLDivElement | null>;
  mdxPreviewRef: RefObject<HTMLDivElement | null>;
  workspace: Workspace | null;
  htmlSourceContainerRef: RefObject<HTMLDivElement | null>;
  astSourceContainerRef: RefObject<HTMLDivElement | null>;
  jsxSourceContainerRef: RefObject<HTMLDivElement | null>;
  getHtml?: () => string;
  getAst?: () => string;
  getJsx?: () => string;
};

export function SplitPanels({
  mobilePanel,
  outputTab,
  onOutputTabChange,
  markdown,
  onMarkdownValueChange,
  onMarkdownDocChange,
  onEditorReady,
  options,
  onOptionsChange,
  previewRef,
  mdxPreviewRef,
  workspace,
  htmlSourceContainerRef,
  astSourceContainerRef,
  jsxSourceContainerRef,
  getHtml,
  getAst,
  getJsx,
}: SplitPanelsProps) {
  const showEditor = mobilePanel === "editor";
  const activeFile =
    workspace?.files.find((f) => f.name === workspace.active) ?? workspace?.files[0];

  return (
    <div id="main-split" className="flex flex-col md:flex-row flex-1 min-h-0">
      <div
        id="panel-editor"
        className={`${showEditor ? "flex" : "hidden"} md:flex flex-1 md:flex-[0_0_50%] md:min-w-0 flex-col md:border-r border-zinc-200 dark:border-zinc-800 min-h-0`}
      >
        <div className="hidden md:flex px-4 pt-2.5 pb-2 text-xs font-medium text-zinc-400 uppercase tracking-wider border-b border-zinc-200 dark:border-zinc-800 bg-zinc-50 dark:bg-zinc-900/50 items-center justify-between">
          <span>{workspace ? "MDX workspace" : "Markdown"}</span>
          <CopyButton getText={() => activeFile?.content ?? markdown} />
        </div>
        <OptionsPanel options={options} onChange={onOptionsChange} />
        {workspace && activeFile && (
          <FileTabs
            files={workspace.files}
            active={activeFile.name}
            onSelect={workspace.onSelect}
            onAdd={workspace.onAdd}
            onRemove={workspace.onRemove}
          />
        )}
        <div className="flex-1 min-h-0 overflow-hidden">
          <MarkdownEditor
            key={activeFile?.name}
            value={activeFile?.content ?? markdown}
            language={activeFile && !isMdx(activeFile.name) ? "jsx" : "markdown"}
            onValueChange={workspace?.onValueChange ?? onMarkdownValueChange}
            onDocChange={workspace?.onDocChange ?? onMarkdownDocChange}
            onEditorReady={onEditorReady}
          />
        </div>
      </div>
      <div
        id="panel-output"
        className={`${showEditor ? "hidden" : "flex"} md:flex md:flex-[0_0_50%] md:min-w-0 flex-1 flex-col min-h-0`}
      >
        <OutputTabs
          outputTab={outputTab}
          onChange={onOutputTabChange}
          getHtml={getHtml}
          getAst={getAst}
          getJsx={getJsx}
        />
        <div
          id="panel-preview"
          role="tabpanel"
          aria-labelledby="tab-preview"
          className={`${outputTab === "preview" ? "flex" : "hidden"} flex-1 min-h-0 overflow-auto p-3 md:p-5`}
        >
          <div ref={previewRef} className={`prose ${workspace ? "hidden" : ""}`} />
          <div ref={mdxPreviewRef} className={`prose ${workspace ? "" : "hidden"}`} />
        </div>
        <div
          id="panel-html"
          role="tabpanel"
          aria-labelledby="tab-html"
          className={`${outputTab === "html" ? "block" : "hidden"} flex-1 min-h-0 overflow-hidden`}
        >
          <div ref={htmlSourceContainerRef} className="h-full" />
        </div>
        <div
          id="panel-ast"
          role="tabpanel"
          aria-labelledby="tab-ast"
          className={`${outputTab === "ast" ? "block" : "hidden"} flex-1 min-h-0 overflow-hidden`}
        >
          <div ref={astSourceContainerRef} className="h-full" />
        </div>
        <div
          id="panel-jsx"
          role="tabpanel"
          aria-labelledby="tab-jsx"
          className={`${outputTab === "jsx" ? "block" : "hidden"} flex-1 min-h-0 overflow-hidden`}
        >
          <div ref={jsxSourceContainerRef} className="h-full" />
        </div>
      </div>
    </div>
  );
}
