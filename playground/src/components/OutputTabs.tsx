import { ACTIVE_TAB, INACTIVE_TAB, OUTPUT_TABS, OUTPUT_TAB_LABELS } from "./tabs";
import type { OutputTab } from "./types";
import { CopyButton } from "./CopyButton";

type OutputTabsProps = {
  outputTab: OutputTab;
  onChange: (tab: OutputTab) => void;
  getHtml?: () => string;
  getAst?: () => string;
  getJsx?: () => string;
};

export function OutputTabs({ outputTab, onChange, getHtml, getAst, getJsx }: OutputTabsProps) {
  const copyGetters: Partial<Record<OutputTab, () => string>> = {
    html: getHtml,
    ast: getAst,
    jsx: getJsx,
  };
  const copyGetter = copyGetters[outputTab] ?? null;

  return (
    <div
      role="tablist"
      aria-label="Output view tabs"
      className="hidden md:flex border-b border-zinc-200 dark:border-zinc-800 bg-zinc-50 dark:bg-zinc-900/50 items-center"
    >
      {OUTPUT_TABS.map((tab) => (
        <button
          key={tab}
          id={`tab-${tab}`}
          type="button"
          role="tab"
          aria-selected={outputTab === tab}
          aria-controls={`panel-${tab}`}
          className={`tab-btn px-4 py-2 text-xs font-medium uppercase tracking-wider border-b-2 ${outputTab === tab ? ACTIVE_TAB : INACTIVE_TAB}`}
          onClick={() => onChange(tab)}
        >
          {OUTPUT_TAB_LABELS[tab]}
        </button>
      ))}
      <div className="flex-1" />
      {copyGetter && <CopyButton getText={copyGetter} className="mr-2" />}
    </div>
  );
}
