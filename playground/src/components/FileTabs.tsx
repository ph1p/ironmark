import { useState } from "react";
import type { PlaygroundFile } from "../mdx/runtime";
import { ACTIVE_TAB, INACTIVE_TAB } from "./tabs";

type FileTabsProps = {
  files: PlaygroundFile[];
  active: string;
  onSelect: (name: string) => void;
  onAdd: (name: string) => void;
  onRemove: (name: string) => void;
};

const FILE_NAME = /^[\w.-]+\.(mdx|jsx|js)$/;

/** File switcher for the MDX workspace; the first file is the rendered entry. */
export function FileTabs({ files, active, onSelect, onAdd, onRemove }: FileTabsProps) {
  const [draft, setDraft] = useState<string | null>(null);
  const valid = draft !== null && FILE_NAME.test(draft) && !files.some((f) => f.name === draft);

  const commit = () => {
    if (valid) onAdd(draft);
    setDraft(null);
  };

  return (
    <div
      role="tablist"
      aria-label="Workspace files"
      className="flex items-center overflow-x-auto border-b border-zinc-200 dark:border-zinc-800 bg-zinc-50 dark:bg-zinc-900/50 font-mono text-xs"
    >
      {files.map((file, i) => (
        <div
          key={file.name}
          className={`group flex items-center border-b-2 ${file.name === active ? ACTIVE_TAB : INACTIVE_TAB}`}
        >
          <button
            type="button"
            role="tab"
            aria-selected={file.name === active}
            title={i === 0 ? "Entry document (rendered in Preview)" : undefined}
            className="pl-3 pr-1 py-1.5"
            onClick={() => onSelect(file.name)}
          >
            {file.name}
          </button>
          {i > 0 ? (
            <button
              type="button"
              aria-label={`Remove ${file.name}`}
              className="px-1.5 py-1.5 opacity-40 hover:opacity-100"
              onClick={() => onRemove(file.name)}
            >
              ×
            </button>
          ) : (
            <span className="pr-2" />
          )}
        </div>
      ))}
      {draft === null ? (
        <button
          type="button"
          aria-label="Add file"
          title="Add a .jsx, .js or .mdx file"
          className={`px-3 py-1.5 border-b-2 ${INACTIVE_TAB}`}
          onClick={() => setDraft("")}
        >
          +
        </button>
      ) : (
        <input
          autoFocus
          value={draft}
          placeholder="Button.jsx"
          aria-label="New file name"
          aria-invalid={draft !== "" && !valid}
          className={`mx-2 my-1 w-32 rounded border bg-white dark:bg-zinc-900 px-1.5 py-0.5 outline-none ${draft !== "" && !valid ? "border-red-400" : "border-zinc-300 dark:border-zinc-700"}`}
          onChange={(e) => setDraft(e.target.value.trim())}
          onBlur={commit}
          onKeyDown={(e) => {
            if (e.key === "Enter") commit();
            if (e.key === "Escape") setDraft(null);
          }}
        />
      )}
    </div>
  );
}
