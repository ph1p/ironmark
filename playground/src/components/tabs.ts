import type { OutputTab } from "./types";

export const ACTIVE_TAB = "text-zinc-900 dark:text-zinc-100 border-zinc-900 dark:border-zinc-100";
export const INACTIVE_TAB =
  "text-zinc-400 dark:text-zinc-500 border-transparent hover:text-zinc-600 dark:hover:text-zinc-300";

export const OUTPUT_TAB_LABELS: Record<OutputTab, string> = {
  preview: "Preview",
  html: "HTML",
  ast: "AST",
  jsx: "JSX",
};

export const OUTPUT_TABS: OutputTab[] = ["preview", "html", "ast", "jsx"];

export function isOutputTab(value: string | null): value is OutputTab {
  return OUTPUT_TABS.includes(value as OutputTab);
}
