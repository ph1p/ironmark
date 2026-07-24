import { useEffect, useMemo, useState } from "react";
import type { BenchEntry, BenchmarkData, BenchSection, TrendPoint } from "./types";

// ─── Parser display config ───────────────────────────────────────────────────

const COLORS: Record<string, string> = {
  ironmark: "#e8590c",
  pulldown_cmark: "#1971c2",
  comrak: "#2f9e44",
  markdown_rs: "#c2185b",
  markdown_it: "#6741d9",
  md4c: "#d97706",
  "markdown-wasm": "#7048e8",
  md4w: "#0ea5e9",
  bun: "#f4d9a0",
};

const LABELS: Record<string, string> = {
  ironmark: "ironmark",
  pulldown_cmark: "pulldown-cmark",
  comrak: "comrak",
  markdown_rs: "markdown-rs",
  markdown_it: "markdown-it",
  md4c: "md4c",
  "markdown-wasm": "markdown-wasm",
  md4w: "md4w",
  bun: "Bun built-in",
};

// ─── Helpers ─────────────────────────────────────────────────────────────────

function fmtNs(ns: number): string {
  if (ns < 1000) return `${ns.toFixed(0)} ns`;
  if (ns < 1e6) return `${(ns / 1000).toFixed(1)} µs`;
  return `${(ns / 1e6).toFixed(2)} ms`;
}

function fmtThroughput(bytes: number, ns: number): string {
  if (!bytes || !ns) return "-";
  return `${(bytes / (1024 * 1024) / (ns / 1e9)).toFixed(1)} MB/s`;
}

function fmtBytes(b: number): string {
  if (b < 1024) return `${b} B`;
  if (b < 1024 * 1024) return `${(b / 1024).toFixed(1)} KB`;
  return `${(b / (1024 * 1024)).toFixed(1)} MB`;
}

function fmtDate(ts: string): string {
  // Handles both YYYY-MM-DD_HH-MM-SS (history files) and ISO strings (generatedAt)
  const s = ts.slice(0, 16).replace("T", "_");
  const [date, time] = s.split("_");
  return time ? `${date} ${time.replace(/-/g, ":")}` : date;
}

// ─── BenchCard ───────────────────────────────────────────────────────────────

function BenchCard({ bench }: { bench: BenchEntry }) {
  const entries = Object.entries(bench.results)
    .map(([lib, r]) => ({ lib, ...r }))
    .filter((e) => e.median_ns > 0)
    .sort((a, b) => a.median_ns - b.median_ns);

  if (entries.length === 0) return null;

  const winner = entries[0];
  const maxTp = Math.max(
    ...entries.map((e) => (bench.bytes ? bench.bytes / e.median_ns : 1 / e.median_ns)),
  );
  const speedup =
    entries.length >= 2
      ? entries[1].median_ns / winner.median_ns > 1.01
        ? `${(entries[1].median_ns / winner.median_ns).toFixed(1)}× faster`
        : "~tied"
      : null;

  return (
    <div className="rounded-lg bg-zinc-900 border border-zinc-800 p-4">
      <div className="flex items-center justify-between mb-3 gap-2 flex-wrap">
        <span className="text-xs text-zinc-400 font-medium">
          {bench.name}
          {bench.bytes > 0 && <span className="text-zinc-600 ml-1">({fmtBytes(bench.bytes)})</span>}
        </span>
        {speedup && (
          <span
            className="text-xs font-semibold px-2 py-0.5 rounded border"
            style={{
              color: COLORS[winner.lib] ?? "#e8590c",
              borderColor: `${COLORS[winner.lib] ?? "#e8590c"}99`,
            }}
          >
            {LABELS[winner.lib] ?? winner.lib} · {speedup}
          </span>
        )}
      </div>

      <div className="flex flex-col gap-1.5">
        {entries.map((e) => {
          const tp = bench.bytes ? bench.bytes / e.median_ns : 1 / e.median_ns;
          const pct = tp / maxTp;
          const isWinner = e.lib === winner.lib;
          const color = COLORS[e.lib] ?? "#888";
          const label = LABELS[e.lib] ?? e.lib;

          return (
            <div key={e.lib} className="flex items-center gap-2 text-xs">
              <span
                className="w-28 shrink-0 text-right truncate"
                style={{ color: isWinner ? "#fff" : "#666", fontWeight: isWinner ? 600 : 400 }}
              >
                {label}
              </span>
              <div className="flex-1 h-5 rounded bg-zinc-800 overflow-hidden min-w-0">
                <div
                  className="h-full rounded"
                  style={{
                    width: `${Math.max(1, pct * 100).toFixed(1)}%`,
                    backgroundColor: color,
                  }}
                />
              </div>
              <span className="w-16 shrink-0 text-right text-zinc-300 tabular-nums">
                {fmtNs(e.median_ns)}
              </span>
              <span className="w-20 shrink-0 text-right text-zinc-600 tabular-nums hidden sm:block">
                {fmtThroughput(bench.bytes, e.median_ns)}
              </span>
            </div>
          );
        })}
      </div>
    </div>
  );
}

// ─── BenchSectionView ────────────────────────────────────────────────────────

function BenchSectionView({ section }: { section: BenchSection }) {
  return (
    <div className="space-y-3">
      <h3 className="text-sm font-semibold text-zinc-300">{section.title}</h3>
      {section.benches.map((b) => (
        <BenchCard key={b.name} bench={b} />
      ))}
    </div>
  );
}

// ─── TrendChart ──────────────────────────────────────────────────────────────

// CVD-validated trio for the dark surface (orange / aqua / blue).
const TREND_COLORS = { wasm: "#d95926", bun: "#199e70", rust: "#3987e5" };

const ABSOLUTE_TREND_SERIES: { key: keyof TrendPoint; label: string; color: string }[] = [
  { key: "ironmark_wasm_ns", label: "WASM (Node)", color: TREND_COLORS.wasm },
  { key: "ironmark_bun_ns", label: "Bun", color: TREND_COLORS.bun },
  { key: "ironmark_rust_ns", label: "Native Rust", color: TREND_COLORS.rust },
];

const RELATIVE_TREND_SERIES: { key: keyof TrendPoint; label: string; color: string }[] = [
  { key: "ironmark_wasm_ratio", label: "WASM (Node)", color: TREND_COLORS.wasm },
  { key: "ironmark_bun_ratio", label: "Bun", color: TREND_COLORS.bun },
  { key: "ironmark_rust_ratio", label: "Native Rust", color: TREND_COLORS.rust },
];

type TrendMode = "absolute" | "relative";

function fmtRatio(v: number): string {
  return `${v.toFixed(2)}x`;
}

/**
 * Monotone cubic (Fritsch–Carlson) path through the points. Smooth like a
 * spline but never overshoots — the curve stays within the data's range, so
 * it can't fake a dip or spike between two benchmark runs.
 */
function monotonePath(pts: { x: number; y: number }[]): string {
  const n = pts.length;
  if (n === 0) return "";
  if (n === 1) return `M${pts[0].x},${pts[0].y}`;
  const dx: number[] = [];
  const slope: number[] = [];
  for (let i = 0; i < n - 1; i++) {
    dx.push(pts[i + 1].x - pts[i].x);
    slope.push((pts[i + 1].y - pts[i].y) / dx[i]);
  }
  const tan = [slope[0]];
  for (let i = 1; i < n - 1; i++) {
    tan.push(slope[i - 1] * slope[i] <= 0 ? 0 : (slope[i - 1] + slope[i]) / 2);
  }
  tan.push(slope[n - 2]);
  for (let i = 0; i < n - 1; i++) {
    if (slope[i] === 0) {
      tan[i] = 0;
      tan[i + 1] = 0;
      continue;
    }
    const a = tan[i] / slope[i];
    const b = tan[i + 1] / slope[i];
    const s = a * a + b * b;
    if (s > 9) {
      const f = 3 / Math.sqrt(s);
      tan[i] = f * a * slope[i];
      tan[i + 1] = f * b * slope[i];
    }
  }
  let d = `M${pts[0].x.toFixed(1)},${pts[0].y.toFixed(1)}`;
  for (let i = 0; i < n - 1; i++) {
    const h = dx[i] / 3;
    d += ` C${(pts[i].x + h).toFixed(1)},${(pts[i].y + tan[i] * h).toFixed(1)} ${(
      pts[i + 1].x - h
    ).toFixed(1)},${(pts[i + 1].y - tan[i + 1] * h).toFixed(1)} ${pts[i + 1].x.toFixed(1)},${pts[
      i + 1
    ].y.toFixed(1)}`;
  }
  return d;
}

/** Nudge overlapping end-of-line labels apart (min 12px vertical gap). */
function spreadLabels(ys: number[], minGap = 12): number[] {
  const order = ys.map((y, i) => ({ y, i })).sort((a, b) => a.y - b.y);
  for (let k = 1; k < order.length; k++) {
    if (order[k].y - order[k - 1].y < minGap) order[k].y = order[k - 1].y + minGap;
  }
  const out = [...ys];
  for (const { y, i } of order) out[i] = y;
  return out;
}

function TrendChart({ trend, mode }: { trend: TrendPoint[]; mode: TrendMode }) {
  if (trend.length < 2) {
    return (
      <p className="text-sm text-zinc-500 text-center py-12">
        Run <code className="text-zinc-400">pnpm bench</code> multiple times to see a performance
        trend. Lower is faster.
      </p>
    );
  }

  const W = 600;
  const H = 220;
  const PAD = { top: 18, right: 96, bottom: 36, left: 64 };
  const chartW = W - PAD.left - PAD.right;
  const chartH = H - PAD.top - PAD.bottom;

  const series = mode === "absolute" ? ABSOLUTE_TREND_SERIES : RELATIVE_TREND_SERIES;
  const allValues = trend.flatMap((t) =>
    series.map((s) => t[s.key] as number | undefined).filter((v): v is number => v != null),
  );
  // Relative mode always shows the 1.00x parity line for context, so the
  // domain must include it; 8% headroom keeps curves off the frame.
  let lo = Math.min(...allValues);
  let hi = Math.max(...allValues);
  if (mode === "relative") {
    lo = Math.min(lo, 1);
    hi = Math.max(hi, 1);
  }
  const headroom = (hi - lo || 1) * 0.08;
  const minV = lo - headroom;
  const maxV = hi + headroom;
  const range = maxV - minV;

  const px = (i: number) => PAD.left + (i / (trend.length - 1)) * chartW;
  const py = (v: number) => PAD.top + chartH - ((v - minV) / range) * chartH;

  const activeSeries = series.filter((s) => trend.some((t) => t[s.key] != null));

  // End-of-line labels, nudged apart when lines converge.
  const endLabelYs = spreadLabels(
    activeSeries.map((s) => {
      for (let i = trend.length - 1; i >= 0; i--) {
        const v = trend[i][s.key] as number | undefined;
        if (v != null) return py(v);
      }
      return PAD.top + chartH;
    }),
  );

  return (
    <div className="space-y-3">
      <div className="flex gap-4 flex-wrap">
        {activeSeries.map((s) => (
          <div key={s.key} className="flex items-center gap-1.5 text-xs text-zinc-400">
            <span
              className="w-3 h-0.5 rounded-full inline-block"
              style={{ backgroundColor: s.color }}
            />
            {s.label}
          </div>
        ))}
      </div>
      <div className="overflow-x-auto">
        <svg
          viewBox={`0 0 ${W} ${H}`}
          width={W}
          height={H}
          className="max-w-full"
          style={{ fontFamily: "ui-monospace, monospace" }}
        >
          <line
            x1={PAD.left}
            y1={PAD.top}
            x2={PAD.left + chartW}
            y2={PAD.top}
            stroke="#333"
            strokeDasharray="3,3"
          />
          <line
            x1={PAD.left}
            y1={PAD.top + chartH}
            x2={PAD.left + chartW}
            y2={PAD.top + chartH}
            stroke="#333"
            strokeDasharray="3,3"
          />
          <text x={PAD.left - 6} y={PAD.top + 4} textAnchor="end" fill="#666" fontSize="10">
            {mode === "absolute" ? fmtNs(maxV) : fmtRatio(maxV)}
          </text>
          <text
            x={PAD.left - 6}
            y={PAD.top + chartH + 4}
            textAnchor="end"
            fill="#666"
            fontSize="10"
          >
            {mode === "absolute" ? fmtNs(minV) : fmtRatio(minV)}
          </text>

          {/* Reading direction: lower on the chart = faster. */}
          <text x={PAD.left - 6} y={PAD.top + 16} textAnchor="end" fill="#555" fontSize="9">
            slower ↑
          </text>
          <text x={PAD.left - 6} y={PAD.top + chartH - 8} textAnchor="end" fill="#888" fontSize="9">
            faster ↓
          </text>

          {/* Parity with the fastest competitor (relative mode only). */}
          {mode === "relative" && (
            <g>
              <line
                x1={PAD.left}
                y1={py(1)}
                x2={PAD.left + chartW}
                y2={py(1)}
                stroke="#52525b"
                strokeDasharray="5,4"
              />
              <text x={PAD.left + 4} y={py(1) - 4} fill="#71717a" fontSize="9" textAnchor="start">
                1.00x = fastest rival
              </text>
            </g>
          )}

          {activeSeries.map((s) => {
            const pts = trend
              .map((t, i) => {
                const v = t[s.key] as number | undefined;
                return v != null ? { x: px(i), y: py(v) } : null;
              })
              .filter((p): p is { x: number; y: number } => p != null);
            return (
              <path
                key={s.key}
                d={monotonePath(pts)}
                fill="none"
                stroke={s.color}
                strokeWidth="2"
                strokeLinejoin="round"
                strokeLinecap="round"
              />
            );
          })}

          {/* Direct end-of-line labels: identity readable without the legend. */}
          {activeSeries.map((s, si) => (
            <g key={`label-${s.key}`}>
              <line
                x1={PAD.left + chartW + 4}
                y1={endLabelYs[si]}
                x2={PAD.left + chartW + 14}
                y2={endLabelYs[si]}
                stroke={s.color}
                strokeWidth="2"
              />
              <text x={PAD.left + chartW + 18} y={endLabelYs[si] + 3} fill="#a1a1aa" fontSize="9">
                {s.label}
              </text>
            </g>
          ))}

          {trend.map((t, i) =>
            activeSeries.map((s) => {
              const v = t[s.key] as number | undefined;
              if (v == null) return null;
              return (
                // Invisible wide hit target carries the tooltip; the visible
                // dot stays small so the curve reads clean.
                <g key={`${s.key}-${i}`}>
                  <circle cx={px(i)} cy={py(v)} r="9" fill="transparent">
                    <title>
                      {fmtDate(t.timestamp)} · {s.label}:{" "}
                      {mode === "absolute"
                        ? fmtNs(v)
                        : `${fmtRatio(v)} vs fastest rival (${v < 1 ? "faster" : "slower"})`}
                    </title>
                  </circle>
                  <circle cx={px(i)} cy={py(v)} r="3" fill={s.color} pointerEvents="none" />
                </g>
              );
            }),
          )}

          {trend.map((t, i) =>
            trend.length <= 6 || i === 0 || i === trend.length - 1 ? (
              <text
                key={i}
                x={px(i)}
                y={H - 6}
                textAnchor={i === 0 ? "start" : i === trend.length - 1 ? "end" : "middle"}
                fill="#555"
                fontSize="9"
              >
                {fmtDate(t.timestamp)}
              </text>
            ) : null,
          )}
        </svg>
      </div>
    </div>
  );
}

// ─── RankingOverview ─────────────────────────────────────────────────────────

type RankEntry = { lib: string; wins: number; total: number; avgSpeedup: number };

function buildRanking(latest: BenchmarkData["latest"]): RankEntry[] {
  const wins: Record<string, number> = {};
  const total: Record<string, number> = {};
  const speedupSum: Record<string, number> = {};

  const allSections = [
    ...(latest.wasm?.sections ?? []),
    ...(latest.bun?.sections ?? []),
    ...(latest.rust?.sections ?? []),
  ];

  for (const section of allSections) {
    for (const bench of section.benches) {
      const entries = Object.entries(bench.results)
        .filter(([, r]) => r.median_ns > 0)
        .sort(([, a], [, b]) => a.median_ns - b.median_ns);
      if (entries.length === 0) continue;
      const fastestNs = entries[0][1].median_ns;
      for (const [lib, r] of entries) {
        total[lib] = (total[lib] ?? 0) + 1;
        speedupSum[lib] = (speedupSum[lib] ?? 0) + r.median_ns / fastestNs;
      }
      const [winner] = entries[0];
      wins[winner] = (wins[winner] ?? 0) + 1;
    }
  }

  return Object.keys(total)
    .map((lib) => ({
      lib,
      wins: wins[lib] ?? 0,
      total: total[lib],
      avgSpeedup: speedupSum[lib] / total[lib],
    }))
    .sort((a, b) => b.wins - a.wins || a.avgSpeedup - b.avgSpeedup);
}

function RankingOverview({ latest }: { latest: BenchmarkData["latest"] }) {
  const ranking = buildRanking(latest);
  if (ranking.length === 0) return null;

  const maxWins = ranking[0].wins;

  return (
    <div className="rounded-lg bg-zinc-900 border border-zinc-800 p-4 space-y-3">
      <h3 className="text-xs font-semibold uppercase tracking-wider text-zinc-500">
        Overall Ranking
      </h3>
      <div className="flex flex-col gap-2">
        {ranking.map((e, i) => {
          const color = COLORS[e.lib] ?? "#888";
          const label = LABELS[e.lib] ?? e.lib;
          const pct = maxWins > 0 ? e.wins / maxWins : 0;
          const medal = i === 0 ? "🥇" : i === 1 ? "🥈" : i === 2 ? "🥉" : null;
          return (
            <div key={e.lib} className="flex items-center gap-2 text-xs">
              <span className="w-5 shrink-0 text-center text-sm">
                {medal ?? <span className="text-zinc-600">{i + 1}</span>}
              </span>
              <span
                className="w-28 shrink-0 truncate font-medium"
                style={{ color: i === 0 ? "#fff" : "#999" }}
              >
                {label}
              </span>
              <div className="flex-1 h-4 rounded bg-zinc-800 overflow-hidden min-w-0">
                <div
                  className="h-full rounded"
                  style={{ width: `${Math.max(1, pct * 100).toFixed(1)}%`, backgroundColor: color }}
                />
              </div>
              <span className="w-20 shrink-0 text-right tabular-nums text-zinc-300">
                {e.wins}/{e.total} wins
              </span>
              <span className="w-16 shrink-0 text-right tabular-nums text-zinc-600 hidden sm:block">
                {e.avgSpeedup.toFixed(2)}× avg
              </span>
            </div>
          );
        })}
      </div>
      <p className="text-xs text-zinc-600">
        Wins = fastest in a benchmark · avg = mean slowdown vs winner (1.00× = always fastest)
      </p>
    </div>
  );
}

// ─── BenchmarkPage ───────────────────────────────────────────────────────────

type InnerTab = "latest" | "trend";
type RuntimeFilter = "all" | "wasm" | "bun" | "rust";
type LatestGroup = {
  runtime: Exclude<RuntimeFilter, "all">;
  heading: string;
  note?: string;
  section: BenchSection;
};

export function BenchmarkPage() {
  const [data, setData] = useState<BenchmarkData | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [loading, setLoading] = useState(true);
  const [innerTab, setInnerTab] = useState<InnerTab>("latest");
  const [trendMode, setTrendMode] = useState<TrendMode>("absolute");
  const [query, setQuery] = useState("");
  const [runtimeFilter, setRuntimeFilter] = useState<RuntimeFilter>("all");
  const [parserFilter, setParserFilter] = useState<string>("all");

  useEffect(() => {
    fetch(`${import.meta.env.BASE_URL}benchmark-data.json`)
      .then((r) => (r.ok ? r.json() : Promise.reject(r.status)))
      .then((d: BenchmarkData) => setData(d))
      .catch(() => setError("No benchmark data found. Run pnpm bench to generate it."))
      .finally(() => setLoading(false));
  }, []);

  const latestGroups = useMemo<LatestGroup[]>(() => {
    if (!data) return [];
    const groups: LatestGroup[] = [];
    if (data.latest.wasm) {
      groups.push(
        ...data.latest.wasm.sections.map((section) => ({
          runtime: "wasm" as const,
          heading: "WASM · Node.js",
          section,
        })),
      );
    }
    if (data.latest.bun) {
      groups.push(
        ...data.latest.bun.sections.map((section) => ({
          runtime: "bun" as const,
          heading: "WASM · Bun runtime",
          note: "(+ Bun built-in native)",
          section,
        })),
      );
    }
    if (data.latest.rust) {
      groups.push(
        ...data.latest.rust.sections.map((section) => ({
          runtime: "rust" as const,
          heading: "Native Rust",
          section,
        })),
      );
    }
    return groups;
  }, [data]);

  const parserOptions = useMemo(() => {
    const libs = new Set<string>();
    for (const group of latestGroups) {
      for (const bench of group.section.benches) {
        for (const [lib, result] of Object.entries(bench.results)) {
          if (result.median_ns > 0) libs.add(lib);
        }
      }
    }
    return Array.from(libs).sort((a, b) => (LABELS[a] ?? a).localeCompare(LABELS[b] ?? b));
  }, [latestGroups]);

  const filteredGroups = useMemo(() => {
    const q = query.trim().toLowerCase();
    return latestGroups
      .filter((group) => runtimeFilter === "all" || group.runtime === runtimeFilter)
      .map((group) => ({
        ...group,
        section: {
          ...group.section,
          benches: group.section.benches.filter((bench) => {
            const parserMatch =
              parserFilter === "all" ||
              Object.entries(bench.results).some(
                ([lib, result]) => lib === parserFilter && result.median_ns > 0,
              );
            if (!parserMatch) return false;
            if (!q) return true;
            const haystack = [
              group.heading,
              group.section.title,
              bench.name,
              ...Object.keys(bench.results).map((lib) => LABELS[lib] ?? lib),
            ]
              .join(" ")
              .toLowerCase();
            return haystack.includes(q);
          }),
        },
      }))
      .filter((group) => group.section.benches.length > 0);
  }, [latestGroups, parserFilter, query, runtimeFilter]);

  const filteredBenchCount = filteredGroups.reduce(
    (sum, group) => sum + group.section.benches.length,
    0,
  );

  return (
    <div className="flex-1 overflow-y-auto bg-zinc-950 text-zinc-100">
      <div className="max-w-3xl mx-auto px-4 py-6 space-y-6">
        <div>
          <h2 className="text-lg font-semibold">Benchmarks</h2>
          {data?.generatedAt && (
            <p className="text-xs text-zinc-500 mt-0.5">Generated {fmtDate(data.generatedAt)}</p>
          )}
        </div>

        {loading && <p className="text-sm text-zinc-500">Loading benchmark data…</p>}

        {error && (
          <div className="rounded-lg bg-zinc-900 border border-zinc-800 p-4">
            <p className="text-sm text-zinc-400">{error}</p>
            <p className="text-xs text-zinc-600 mt-2">
              Run <code className="text-zinc-500">pnpm bench</code> from the project root.
            </p>
          </div>
        )}

        {data && (
          <>
            {/* Inner tabs */}
            <div className="flex gap-4 border-b border-zinc-800">
              {(["latest", "trend"] as InnerTab[]).map((tab) => (
                <button
                  key={tab}
                  onClick={() => setInnerTab(tab)}
                  className={`pb-2 text-sm font-medium capitalize border-b-2 -mb-px transition-colors ${
                    innerTab === tab
                      ? "border-zinc-100 text-zinc-100"
                      : "border-transparent text-zinc-500 hover:text-zinc-300"
                  }`}
                >
                  {tab === "trend" ? "Trend" : "Latest"}
                </button>
              ))}
            </div>

            {/* Latest tab */}
            {innerTab === "latest" && (
              <div className="space-y-10">
                <RankingOverview latest={data.latest} />
                <div className="rounded-lg bg-zinc-900 border border-zinc-800 p-4 space-y-4">
                  <div className="flex items-center justify-between gap-3 flex-wrap">
                    <h3 className="text-xs font-semibold uppercase tracking-wider text-zinc-500">
                      Filter Benchmarks
                    </h3>
                    <span className="text-xs text-zinc-600">
                      {filteredBenchCount} benchmark{filteredBenchCount === 1 ? "" : "s"} shown
                    </span>
                  </div>
                  <div className="grid gap-3 md:grid-cols-[minmax(0,1.4fr)_auto_auto]">
                    <input
                      value={query}
                      onChange={(e) => setQuery(e.target.value)}
                      placeholder="Search by benchmark, section, runtime, or parser"
                      className="w-full rounded-md border border-zinc-800 bg-zinc-950 px-3 py-2 text-sm text-zinc-100 placeholder:text-zinc-600 outline-none focus:border-zinc-600"
                    />
                    <select
                      value={runtimeFilter}
                      onChange={(e) => setRuntimeFilter(e.target.value as RuntimeFilter)}
                      className="rounded-md border border-zinc-800 bg-zinc-950 px-3 py-2 text-sm text-zinc-300 outline-none focus:border-zinc-600"
                    >
                      <option value="all">All runtimes</option>
                      <option value="wasm">WASM · Node</option>
                      <option value="bun">Bun</option>
                      <option value="rust">Rust</option>
                    </select>
                    <select
                      value={parserFilter}
                      onChange={(e) => setParserFilter(e.target.value)}
                      className="rounded-md border border-zinc-800 bg-zinc-950 px-3 py-2 text-sm text-zinc-300 outline-none focus:border-zinc-600"
                    >
                      <option value="all">All parsers</option>
                      {parserOptions.map((lib) => (
                        <option key={lib} value={lib}>
                          {LABELS[lib] ?? lib}
                        </option>
                      ))}
                    </select>
                  </div>
                  <div className="flex gap-2 flex-wrap">
                    {(["all", "wasm", "bun", "rust"] as RuntimeFilter[]).map((value) => (
                      <button
                        key={value}
                        onClick={() => setRuntimeFilter(value)}
                        className={`rounded-full border px-2.5 py-1 text-xs transition-colors ${
                          runtimeFilter === value
                            ? "border-zinc-500 bg-zinc-800 text-zinc-100"
                            : "border-zinc-800 bg-zinc-950 text-zinc-500 hover:text-zinc-300"
                        }`}
                      >
                        {value === "all"
                          ? "All"
                          : value === "wasm"
                            ? "WASM · Node"
                            : value === "bun"
                              ? "Bun"
                              : "Rust"}
                      </button>
                    ))}
                  </div>
                </div>
                {filteredGroups.map((group, index) => {
                  const showHeading =
                    index === 0 ||
                    filteredGroups[index - 1].heading !== group.heading ||
                    filteredGroups[index - 1].note !== group.note;
                  return (
                    <div key={`${group.runtime}:${group.section.title}`} className="space-y-5">
                      {showHeading && (
                        <h3 className="text-xs font-semibold uppercase tracking-wider text-zinc-500">
                          {group.heading}{" "}
                          {group.note && (
                            <span className="normal-case font-normal text-zinc-600">
                              {group.note}
                            </span>
                          )}
                        </h3>
                      )}
                      <BenchSectionView section={group.section} />
                    </div>
                  );
                })}
                {filteredBenchCount === 0 && (
                  <div className="rounded-lg bg-zinc-900 border border-zinc-800 p-4">
                    <p className="text-sm text-zinc-400">
                      No benchmarks match the current filters.
                    </p>
                  </div>
                )}
                {!data.latest.rust && (
                  <p className="text-xs text-zinc-600">
                    Native Rust results not available — run{" "}
                    <code className="text-zinc-500">cargo bench</code> to include them.
                  </p>
                )}
              </div>
            )}

            {/* Trend tab */}
            {innerTab === "trend" && (
              <div className="space-y-4">
                <div className="flex gap-2 flex-wrap">
                  {(
                    [
                      ["absolute", "Absolute Time"],
                      ["relative", "Vs Fastest"],
                    ] as const
                  ).map(([value, label]) => (
                    <button
                      key={value}
                      onClick={() => setTrendMode(value)}
                      className={`rounded-full border px-2.5 py-1 text-xs transition-colors ${
                        trendMode === value
                          ? "border-zinc-500 bg-zinc-800 text-zinc-100"
                          : "border-zinc-800 bg-zinc-950 text-zinc-500 hover:text-zinc-300"
                      }`}
                    >
                      {label}
                    </button>
                  ))}
                </div>
                <p className="text-xs text-zinc-500">
                  {trendMode === "absolute"
                    ? "ironmark — CommonMark Spec median parse time across runs. Lower is faster; this is absolute time."
                    : "ironmark — CommonMark Spec time vs the fastest competitor in the same runtime. Below 1.00x = ironmark faster than the best rival (0.60x = 40% faster)."}
                </p>
                <TrendChart trend={data.trend} mode={trendMode} />
              </div>
            )}
          </>
        )}
      </div>
    </div>
  );
}
