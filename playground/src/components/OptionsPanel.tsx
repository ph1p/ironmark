import { useState } from "react";
import { DEFAULT_PARSE_OPTIONS, type ParseOptions } from "./types";

type OptionsPanelProps = {
  options: ParseOptions;
  onChange: (options: ParseOptions) => void;
};

type OptionEntry =
  | { key: keyof ParseOptions; label: string; description: string }
  | { separator: string };

const OPTION_LABELS: OptionEntry[] = [
  { separator: "Core extensions" },
  { key: "hard_breaks", label: "Hard breaks", description: "Newlines become <br />" },
  { key: "enable_highlight", label: "Highlight", description: "==text== → <mark>" },
  { key: "enable_strikethrough", label: "Strikethrough", description: "~~text~~ → <del>" },
  { key: "enable_underline", label: "Underline", description: "++text++ → <u>" },
  { key: "enable_tables", label: "Tables", description: "Pipe table syntax" },
  { key: "enable_autolink", label: "Autolink", description: "Bare URLs and emails" },
  { key: "enable_task_lists", label: "Task lists", description: "- [x] checkbox items" },
  {
    key: "enable_indented_code_blocks",
    label: "Indented code blocks",
    description: "4-space indent → code block",
  },
  { separator: "Extra extensions" },
  { key: "enable_wiki_links", label: "Wiki links", description: "[[link]] → <a href>" },
  { key: "enable_latex_math", label: "LaTeX math", description: "$inline$ and $$display$$ math" },
  {
    key: "enable_mdx",
    label: "MDX",
    description: "Multi-file workspace: JSX, {expressions}, imports, live React preview",
  },
  { key: "enable_heading_ids", label: "Heading IDs", description: "Auto-generate id= on headings" },
  {
    key: "enable_heading_anchors",
    label: "Heading anchors",
    description: "Add ¶ anchor link in headings",
  },
  {
    key: "permissive_atx_headers",
    label: "Permissive headings",
    description: "#Heading without space after #",
  },
  { separator: "HTML & security" },
  {
    key: "disable_raw_html",
    label: "Disable raw HTML",
    description: "Escape all HTML (blocks + spans)",
  },
  {
    key: "no_html_blocks",
    label: "No HTML blocks",
    description: "Disable block-level HTML constructs",
  },
  { key: "no_html_spans", label: "No inline HTML", description: "Disable inline HTML spans" },
  { key: "tag_filter", label: "Tag filter", description: "GFM: escape <script>, <iframe>, etc." },
  { separator: "Text processing" },
  {
    key: "collapse_whitespace",
    label: "Collapse whitespace",
    description: "Multiple spaces → single space",
  },
];

export function OptionsPanel({ options, onChange }: OptionsPanelProps) {
  const [open, setOpen] = useState(false);

  const toggle = (key: keyof ParseOptions) => {
    onChange({ ...options, [key]: !options[key] });
  };

  const activeCount = OPTION_LABELS.filter(
    (e): e is Extract<OptionEntry, { key: keyof ParseOptions }> => "key" in e,
  ).filter((e) => options[e.key] !== DEFAULT_PARSE_OPTIONS[e.key]).length;

  return (
    <div className="border-b border-zinc-200 dark:border-zinc-800">
      <button
        type="button"
        onClick={() => setOpen(!open)}
        className="w-full flex items-center justify-between px-4 py-2 text-xs font-medium text-zinc-400 uppercase tracking-wider bg-zinc-50 dark:bg-zinc-900/50 hover:text-zinc-600 dark:hover:text-zinc-300 transition-colors cursor-pointer"
      >
        <span className="flex items-center gap-2">
          Options
          {activeCount > 0 && (
            <span className="px-1.5 py-0.5 rounded text-[10px] font-bold bg-zinc-200 dark:bg-zinc-700 text-zinc-600 dark:text-zinc-300">
              {activeCount} changed
            </span>
          )}
        </span>
        <svg
          width="12"
          height="12"
          viewBox="0 0 12 12"
          fill="none"
          stroke="currentColor"
          strokeWidth="1.5"
          strokeLinecap="round"
          strokeLinejoin="round"
          className={`transition-transform ${open ? "rotate-180" : ""}`}
        >
          <path d="M3 4.5L6 7.5L9 4.5" />
        </svg>
      </button>
      {open && (
        <div className="px-4 py-2.5 bg-zinc-50/50 dark:bg-zinc-900/30 max-h-72 overflow-y-auto">
          <div className="grid grid-cols-2 gap-x-4 gap-y-0.5">
            {OPTION_LABELS.map((entry) => {
              if ("separator" in entry) {
                return (
                  <div
                    key={entry.separator}
                    className="col-span-2 pt-2 pb-0.5 text-[10px] font-semibold uppercase tracking-widest text-zinc-400 dark:text-zinc-600"
                  >
                    {entry.separator}
                  </div>
                );
              }
              const { key, label, description } = entry;
              return (
                <label
                  key={key}
                  className="flex items-center gap-2 py-1 cursor-pointer group"
                  title={description}
                >
                  <button
                    type="button"
                    role="switch"
                    aria-checked={options[key]}
                    onClick={() => toggle(key)}
                    className={`relative shrink-0 w-7 h-4 rounded-full transition-colors ${
                      options[key] ? "bg-zinc-900 dark:bg-zinc-100" : "bg-zinc-300 dark:bg-zinc-700"
                    }`}
                  >
                    <span
                      className={`absolute top-0.5 left-0.5 w-3 h-3 rounded-full transition-transform ${
                        options[key]
                          ? "translate-x-3 bg-white dark:bg-zinc-900"
                          : "translate-x-0 bg-white dark:bg-zinc-400"
                      }`}
                    />
                  </button>
                  <span className="text-xs text-zinc-600 dark:text-zinc-400 group-hover:text-zinc-900 dark:group-hover:text-zinc-200 transition-colors select-none">
                    {label}
                  </span>
                </label>
              );
            })}
          </div>
        </div>
      )}
    </div>
  );
}
