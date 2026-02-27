import { ACTIVE_TAB, INACTIVE_TAB, OUTPUT_TAB_LABELS } from "./tabs";
import type { OutputTab } from "./types";

type OutputTabsProps = {
  outputTab: OutputTab;
  onChange: (tab: OutputTab) => void;
};

export function OutputTabs({ outputTab, onChange }: OutputTabsProps) {
  return (
    <div
      role="tablist"
      aria-label="Output view tabs"
      className="hidden md:flex border-b border-zinc-200 dark:border-zinc-800 bg-zinc-50 dark:bg-zinc-900/50"
    >
      <button
        id="tab-preview"
        type="button"
        role="tab"
        aria-selected={outputTab === "preview"}
        aria-controls="panel-preview"
        className={`tab-btn px-4 py-2 text-xs font-medium uppercase tracking-wider border-b-2 ${outputTab === "preview" ? ACTIVE_TAB : INACTIVE_TAB}`}
        onClick={() => onChange("preview")}
      >
        {OUTPUT_TAB_LABELS.preview}
      </button>
      <button
        id="tab-html"
        type="button"
        role="tab"
        aria-selected={outputTab === "html"}
        aria-controls="panel-html"
        className={`tab-btn px-4 py-2 text-xs font-medium uppercase tracking-wider border-b-2 ${outputTab === "html" ? ACTIVE_TAB : INACTIVE_TAB}`}
        onClick={() => onChange("html")}
      >
        {OUTPUT_TAB_LABELS.html}
      </button>
      <button
        id="tab-ast"
        type="button"
        role="tab"
        aria-selected={outputTab === "ast"}
        aria-controls="panel-ast"
        className={`tab-btn px-4 py-2 text-xs font-medium uppercase tracking-wider border-b-2 ${outputTab === "ast" ? ACTIVE_TAB : INACTIVE_TAB}`}
        onClick={() => onChange("ast")}
      >
        {OUTPUT_TAB_LABELS.ast}
      </button>
    </div>
  );
}
