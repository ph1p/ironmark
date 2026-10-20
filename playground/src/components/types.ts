export type OutputTab = "preview" | "html" | "ast";
export type MobilePanel = "editor" | OutputTab;
export type AppView = "playground" | "benchmarks" | "ast-to-md" | "html-to-md";

export type BenchResult = {
  median_ns: number;
  mean_ns?: number;
  p25_ns?: number;
  p75_ns?: number;
  p95_ns?: number;
  samples?: number;
  /** false when this parser's output differs from the majority's. */
  matches_majority?: boolean;
};
export type BenchEntry = { name: string; bytes: number; results: Record<string, BenchResult> };
export type BenchSection = { title: string; benches: BenchEntry[] };
/** ironmark in one tracked case; ratio = time ÷ fastest other parser's time. */
export type TrendCase = { ns: number; ratio?: number; rival?: string };
export type TrendPoint = {
  timestamp: string;
  /** Feature profile of the run; missing on snapshots before profiles existed. */
  profile?: string;
  machine?: string;
  cases: Partial<Record<"wasm" | "bun" | "rust", Record<string, TrendCase>>>;
};
export type BenchEnv = {
  cpu: string;
  cores: number;
  memory_gb?: number;
  os: string;
  node?: string;
  bun?: string;
  rustc?: string;
};
export type BenchmarkData = {
  generatedAt: string;
  /** Machine/toolchain of the newest snapshot that recorded one. */
  env?: BenchEnv | null;
  profile?: string | null;
  latest: {
    wasm: { sections: BenchSection[] } | null;
    rust: { sections: BenchSection[] } | null;
    bun: { sections: BenchSection[] } | null;
  };
  trend: TrendPoint[];
};

export type ParseOptions = {
  hard_breaks: boolean;
  enable_highlight: boolean;
  enable_strikethrough: boolean;
  enable_underline: boolean;
  enable_tables: boolean;
  enable_autolink: boolean;
  enable_task_lists: boolean;
  disable_raw_html: boolean;
  enable_heading_ids: boolean;
  enable_heading_anchors: boolean;
  enable_indented_code_blocks: boolean;
  no_html_blocks: boolean;
  no_html_spans: boolean;
  tag_filter: boolean;
  collapse_whitespace: boolean;
  permissive_atx_headers: boolean;
  enable_wiki_links: boolean;
  enable_latex_math: boolean;
};

export const DEFAULT_PARSE_OPTIONS: ParseOptions = {
  hard_breaks: true,
  enable_highlight: true,
  enable_strikethrough: true,
  enable_underline: true,
  enable_tables: true,
  enable_autolink: true,
  enable_task_lists: true,
  disable_raw_html: false,
  enable_heading_ids: false,
  enable_heading_anchors: false,
  enable_indented_code_blocks: true,
  no_html_blocks: false,
  no_html_spans: false,
  tag_filter: false,
  collapse_whitespace: false,
  permissive_atx_headers: false,
  enable_wiki_links: false,
  enable_latex_math: false,
};
