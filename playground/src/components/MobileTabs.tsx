import type { MobilePanel } from "./types";
import { ACTIVE_TAB, INACTIVE_TAB, OUTPUT_TAB_LABELS } from "./tabs";

type MobileTabsProps = {
  panel: MobilePanel;
  onChange: (panel: MobilePanel) => void;
};

export function MobileTabs({ panel, onChange }: MobileTabsProps) {
  return (
    <div
      role="tablist"
      aria-label="Mobile view tabs"
      className="flex md:hidden border-b border-zinc-200 dark:border-zinc-800 bg-zinc-50 dark:bg-zinc-900/50 shrink-0"
    >
      <button
        id="mobile-tab-editor"
        type="button"
        role="tab"
        aria-selected={panel === "editor"}
        aria-controls="panel-editor"
        className={`mobile-tab flex-1 px-4 py-2 text-xs font-medium uppercase tracking-wider border-b-2 ${panel === "editor" ? ACTIVE_TAB : INACTIVE_TAB}`}
        onClick={() => onChange("editor")}
      >
        Editor
      </button>
      <button
        id="mobile-tab-preview"
        type="button"
        role="tab"
        aria-selected={panel === "preview"}
        aria-controls="panel-output"
        className={`mobile-tab flex-1 px-4 py-2 text-xs font-medium uppercase tracking-wider border-b-2 ${panel === "preview" ? ACTIVE_TAB : INACTIVE_TAB}`}
        onClick={() => onChange("preview")}
      >
        {OUTPUT_TAB_LABELS.preview}
      </button>
      <button
        id="mobile-tab-html"
        type="button"
        role="tab"
        aria-selected={panel === "html"}
        aria-controls="panel-output"
        className={`mobile-tab flex-1 px-4 py-2 text-xs font-medium uppercase tracking-wider border-b-2 ${panel === "html" ? ACTIVE_TAB : INACTIVE_TAB}`}
        onClick={() => onChange("html")}
      >
        {OUTPUT_TAB_LABELS.html}
      </button>
      <button
        id="mobile-tab-ast"
        type="button"
        role="tab"
        aria-selected={panel === "ast"}
        aria-controls="panel-output"
        className={`mobile-tab flex-1 px-4 py-2 text-xs font-medium uppercase tracking-wider border-b-2 ${panel === "ast" ? ACTIVE_TAB : INACTIVE_TAB}`}
        onClick={() => onChange("ast")}
      >
        {OUTPUT_TAB_LABELS.ast}
      </button>
    </div>
  );
}
