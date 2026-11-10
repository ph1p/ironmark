import type { MobilePanel } from "./types";
import { ACTIVE_TAB, INACTIVE_TAB, OUTPUT_TABS, OUTPUT_TAB_LABELS } from "./tabs";

type MobileTabsProps = {
  panel: MobilePanel;
  onChange: (panel: MobilePanel) => void;
};

const MOBILE_PANELS: MobilePanel[] = ["editor", ...OUTPUT_TABS];

export function MobileTabs({ panel, onChange }: MobileTabsProps) {
  return (
    <div
      role="tablist"
      aria-label="Mobile view tabs"
      className="flex md:hidden border-b border-zinc-200 dark:border-zinc-800 bg-zinc-50 dark:bg-zinc-900/50 shrink-0"
    >
      {MOBILE_PANELS.map((p) => (
        <button
          key={p}
          id={`mobile-tab-${p}`}
          type="button"
          role="tab"
          aria-selected={panel === p}
          aria-controls={p === "editor" ? "panel-editor" : "panel-output"}
          className={`mobile-tab flex-1 px-4 py-2 text-xs font-medium uppercase tracking-wider border-b-2 ${panel === p ? ACTIVE_TAB : INACTIVE_TAB}`}
          onClick={() => onChange(p)}
        >
          {p === "editor" ? "Editor" : OUTPUT_TAB_LABELS[p]}
        </button>
      ))}
    </div>
  );
}
