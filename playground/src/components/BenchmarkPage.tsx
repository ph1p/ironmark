import { Fragment, useEffect, useMemo, useState } from "react";
import type {
  BenchEntry,
  BenchEnv,
  BenchmarkData,
  BenchResult,
  BenchSection,
  TrendPoint,
} from "./types";

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
  marked: "#5c940d",
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
  marked: "marked",
  bun: "Bun built-in",
};

// How each parser runs; ironmark is native in Rust and WASM in JS runtimes.
const KINDS: Record<string, string> = {
  pulldown_cmark: "Rust",
  comrak: "Rust",
  markdown_rs: "Rust",
  markdown_it: "Rust",
  md4c: "C",
  "markdown-wasm": "WASM",
  md4w: "WASM",
  marked: "JS",
  bun: "native",
};

type Runtime = "wasm" | "bun" | "rust";

const RUNTIMES: { key: Runtime; label: string; blurb: string }[] = [
  {
    key: "wasm",
    label: "Node.js",
    blurb:
      "npm packages called from Node.js. ironmark, markdown-wasm and md4w are WebAssembly builds; marked is plain JavaScript.",
  },
  {
    key: "bun",
    label: "Bun",
    blurb:
      "The same npm packages under Bun, plus Bun's built-in Markdown parser, which runs as native code without a WebAssembly boundary.",
  },
  {
    key: "rust",
    label: "Native Rust",
    blurb:
      "Rust crates called directly, plus the C library md4c through FFI. No JavaScript involved.",
  },
];

function parserLabel(lib: string): string {
  return LABELS[lib] ?? lib;
}

function parserKind(lib: string, runtime: Runtime): string {
  if (lib === "ironmark") return runtime === "rust" ? "Rust" : "WASM";
  return KINDS[lib] ?? "";
}

// ─── Helpers ─────────────────────────────────────────────────────────────────

function fmtNs(ns: number): string {
  if (ns < 1000) return `${ns.toFixed(0)} ns`;
  if (ns < 1e6) return `${(ns / 1000).toFixed(1)} µs`;
  if (ns < 1e9) return `${(ns / 1e6).toFixed(2)} ms`;
  return `${(ns / 1e9).toFixed(2)} s`;
}

function fmtThroughput(bytes: number, ns: number): string {
  if (!bytes || !ns) return "–";
  return `${((bytes / ns) * 1e3).toFixed(1)} MB/s`;
}

function fmtMultiple(x: number): string {
  return x >= 10 ? `${x.toFixed(0)}×` : `${x.toFixed(2)}×`;
}

// Half the interquartile range relative to the median (older snapshots: null).
function relSpread(r: BenchResult): number | null {
  if (r.p25_ns == null || r.p75_ns == null || !r.median_ns) return null;
  return (r.p75_ns - r.p25_ns) / 2 / r.median_ns;
}

// Overlapping interquartile ranges: the difference is within noise.
function withinNoise(a: BenchResult, b: BenchResult): boolean {
  if (a.p25_ns == null || a.p75_ns == null || b.p25_ns == null || b.p75_ns == null) return false;
  return a.p25_ns <= b.p75_ns && b.p25_ns <= a.p75_ns;
}

function fmtEnv(env: BenchEnv): string {
  return [
    env.cpu,
    `${env.cores} cores`,
    env.memory_gb && `${env.memory_gb} GB`,
    env.os,
    env.node && `Node ${env.node}`,
    env.bun && `Bun ${env.bun}`,
    env.rustc && `rustc ${env.rustc}`,
  ]
    .filter(Boolean)
    .join(" · ");
}

function fmtBytes(b: number): string {
  if (b < 1000) return `${b} B`;
  if (b < 1e6) return `${(b / 1000).toFixed(1)} KB`;
  return `${(b / 1e6).toFixed(1)} MB`;
}

function fmtDate(ts: string): string {
  // Handles both YYYY-MM-DD_HH-MM-SS (history files) and ISO strings (generatedAt)
  const s = ts.slice(0, 16).replace("T", "_");
  const [date, time] = s.split("_");
  return time ? `${date} ${time.replace(/-/g, ":")}` : date;
}

// Criterion group names (Rust) → the wording the JS runner uses.
function caseName(bench: BenchEntry): string {
  if (bench.name === "commonmark_spec") return "spec (all examples)";
  // Size names are generator targets; the real size is shown next to the name.
  if (/^\d+ bytes$/.test(bench.name) || /^mixed \d/.test(bench.name)) return "mixed";
  return bench.name.replace(/_/g, " ");
}

function geomean(xs: number[]): number {
  return Math.exp(xs.reduce((s, x) => s + Math.log(x), 0) / xs.length);
}

// ─── Per-runtime analysis ────────────────────────────────────────────────────

type Cell = BenchResult & { ratio: number; fastest: boolean; tied: boolean };
type Row = { bench: BenchEntry; name: string; cells: Record<string, Cell> };
type RowGroup = { title: string; rows: Row[] };
type ParserSummary = { lib: string; typical: number; best: number; cases: number };
type Analysis = {
  groups: RowGroup[];
  parsers: ParserSummary[];
  caseCount: number;
  ironmark: { wins: number; ties: number; vsNextBest: number | null } | null;
};

function analyze(sections: BenchSection[]): Analysis {
  const groups: RowGroup[] = [];
  const ratios: Record<string, number[]> = {};
  const best: Record<string, number> = {};
  let wins = 0;
  let ties = 0;
  const vsNext: number[] = [];

  for (const section of sections) {
    const benches = [...section.benches];
    if (section.title === "Document Sizes") benches.sort((a, b) => a.bytes - b.bytes);
    const rows: Row[] = [];
    for (const bench of benches) {
      const valid = Object.entries(bench.results).filter(([, r]) => r.median_ns > 0);
      if (valid.length === 0) continue;
      const [fastLib, fastRes] = valid.reduce((a, b) => (b[1].median_ns < a[1].median_ns ? b : a));
      const cells: Record<string, Cell> = {};
      for (const [lib, r] of valid) {
        const fastest = lib === fastLib;
        const tied = !fastest && withinNoise(r, fastRes);
        cells[lib] = { ...r, ratio: r.median_ns / fastRes.median_ns, fastest, tied };
        (ratios[lib] ??= []).push(cells[lib].ratio);
        if (fastest || tied) best[lib] = (best[lib] ?? 0) + 1;
      }
      const im = cells.ironmark;
      if (im) {
        if (im.fastest) wins++;
        else if (im.tied) ties++;
        const others = valid.filter(([lib]) => lib !== "ironmark").map(([, r]) => r.median_ns);
        if (others.length) vsNext.push(Math.min(...others) / im.median_ns);
      }
      rows.push({ bench, name: caseName(bench), cells });
    }
    if (rows.length) groups.push({ title: section.title, rows });
  }

  const parsers = Object.entries(ratios)
    .map(([lib, rs]) => ({ lib, typical: geomean(rs), best: best[lib] ?? 0, cases: rs.length }))
    .sort((a, b) => a.typical - b.typical);
  const caseCount = groups.reduce((n, g) => n + g.rows.length, 0);

  return {
    groups,
    parsers,
    caseCount,
    ironmark: ratios.ironmark
      ? { wins, ties, vsNextBest: vsNext.length ? geomean(vsNext) : null }
      : null,
  };
}

// ─── Summary ─────────────────────────────────────────────────────────────────

function Summary({ analysis, runtime }: { analysis: Analysis; runtime: Runtime }) {
  const { ironmark: im, parsers, caseCount } = analysis;
  const maxTypical = Math.max(...parsers.map((p) => p.typical));
  // Log scale: a 10× slower parser must not squash the differences that matter.
  const barPct = (x: number) =>
    maxTypical > 1 ? 8 + (92 * Math.log(x)) / Math.log(maxTypical) : 100;

  return (
    <div className="rounded-lg bg-zinc-900 border border-zinc-800 p-4 space-y-4">
      {im && (
        <p className="text-sm text-zinc-300 leading-relaxed">
          ironmark is the fastest in{" "}
          <strong className="text-zinc-100">
            {im.wins} of {caseCount}
          </strong>{" "}
          cases
          {im.ties > 0 && <> and within measurement noise of the fastest in {im.ties} more</>}.
          {im.vsNextBest != null && (
            <>
              {" "}
              Against the best other parser on each case it is typically{" "}
              <strong className={im.vsNextBest >= 1 ? "text-emerald-400" : "text-amber-400"}>
                {im.vsNextBest >= 1
                  ? `${im.vsNextBest.toFixed(2)}× faster`
                  : `${(1 / im.vsNextBest).toFixed(2)}× slower`}
              </strong>
              .
            </>
          )}
        </p>
      )}

      <div className="space-y-1.5">
        <div className="flex items-baseline justify-between gap-2">
          <h3 className="text-xs font-semibold uppercase tracking-wider text-zinc-500">
            Typical time
          </h3>
          <span className="text-xs text-zinc-600">
            relative to the fastest parser · shorter is better
          </span>
        </div>
        {parsers.map((p) => (
          <div key={p.lib} className="flex items-center gap-2 text-xs">
            <span
              className={`w-32 shrink-0 truncate ${p.lib === "ironmark" ? "font-semibold text-zinc-100" : "text-zinc-400"}`}
            >
              {parserLabel(p.lib)}
              <span className="ml-1 text-zinc-600 font-normal">{parserKind(p.lib, runtime)}</span>
            </span>
            <div className="flex-1 h-4 rounded bg-zinc-800 overflow-hidden min-w-0">
              <div
                className="h-full rounded"
                style={{
                  width: `${barPct(p.typical).toFixed(1)}%`,
                  backgroundColor: COLORS[p.lib] ?? "#888",
                }}
              />
            </div>
            <span className="w-14 shrink-0 text-right tabular-nums text-zinc-300">
              {fmtMultiple(p.typical)}
            </span>
            <span className="w-24 shrink-0 text-right tabular-nums text-zinc-600 hidden sm:block">
              best in {p.best}/{p.cases}
            </span>
          </div>
        ))}
        <p className="text-xs text-zinc-600 pt-1">
          Geometric mean over all cases of time ÷ fastest time, so 1.00× would mean fastest
          everywhere. &ldquo;Best in&rdquo; counts cases where the parser is fastest or within noise
          of it.
        </p>
      </div>
    </div>
  );
}

// ─── Results table ───────────────────────────────────────────────────────────

function cellClass(c: Cell): string {
  if (c.fastest) return "bg-emerald-500/15 text-emerald-300 font-semibold";
  if (c.tied) return "bg-emerald-500/5 text-emerald-200/80";
  if (c.ratio < 1.5) return "text-zinc-300";
  if (c.ratio < 3) return "text-zinc-400";
  return "text-zinc-500";
}

function cellTitle(lib: string, c: Cell, bytes: number): string {
  const spread = relSpread(c);
  return [
    `${parserLabel(lib)}: ${fmtNs(c.median_ns)}`,
    spread != null ? `±${(spread * 100).toFixed(1)}%` : null,
    bytes ? fmtThroughput(bytes, c.median_ns) : null,
    c.fastest
      ? "fastest"
      : c.tied
        ? "tied with fastest (within noise)"
        : `${fmtMultiple(c.ratio)} the fastest time`,
    c.matches_majority === false ? "output differs from the other parsers" : null,
  ]
    .filter(Boolean)
    .join(" · ");
}

function CaseDetail({ row, runtime }: { row: Row; runtime: Runtime }) {
  const entries = Object.entries(row.cells).sort(([, a], [, b]) => a.median_ns - b.median_ns);
  const slowest = Math.max(...entries.map(([, c]) => c.median_ns));
  return (
    <div className="space-y-1.5 py-1">
      {entries.map(([lib, c]) => {
        const spread = relSpread(c);
        return (
          <div key={lib} className="flex items-center gap-2 text-xs">
            <span className="w-32 shrink-0 truncate text-zinc-400">
              {c.matches_majority === false && <span className="text-amber-500 mr-1">≠</span>}
              {parserLabel(lib)}
              <span className="ml-1 text-zinc-600">{parserKind(lib, runtime)}</span>
            </span>
            <div className="flex-1 h-3 rounded bg-zinc-800 overflow-hidden min-w-0">
              <div
                className="h-full rounded"
                style={{
                  width: `${Math.max(1, (c.median_ns / slowest) * 100).toFixed(1)}%`,
                  backgroundColor: COLORS[lib] ?? "#888",
                }}
              />
            </div>
            <span className="w-16 shrink-0 text-right tabular-nums text-zinc-300">
              {fmtNs(c.median_ns)}
            </span>
            <span className="w-12 shrink-0 text-right tabular-nums text-zinc-600 hidden sm:block">
              {spread != null ? `±${(spread * 100).toFixed(0)}%` : ""}
            </span>
            <span className="w-20 shrink-0 text-right tabular-nums text-zinc-600 hidden sm:block">
              {fmtThroughput(row.bench.bytes, c.median_ns)}
            </span>
          </div>
        );
      })}
      <p className="text-xs text-zinc-600 pt-1">
        Median time per call; bars show time, so shorter is faster.
        {entries.some(([, c]) => relSpread(c) != null) && " ± is half the interquartile range."}
      </p>
    </div>
  );
}

function ResultsTable({ analysis, runtime }: { analysis: Analysis; runtime: Runtime }) {
  const [open, setOpen] = useState<string | null>(null);
  const libs = analysis.parsers.map((p) => p.lib);
  const anyDiffers = analysis.groups.some((g) =>
    g.rows.some((r) => Object.values(r.cells).some((c) => c.matches_majority === false)),
  );

  return (
    <div className="space-y-2">
      <div className="rounded-lg border border-zinc-800 overflow-x-auto">
        <table className="w-full text-xs tabular-nums border-collapse">
          <thead>
            <tr className="bg-zinc-900 text-zinc-400">
              <th className="sticky left-0 bg-zinc-900 text-left font-medium px-3 py-2">Case</th>
              {libs.map((lib) => (
                <th key={lib} className="px-2 py-2 text-right font-medium whitespace-nowrap">
                  <span
                    className="inline-block w-2 h-2 rounded-full mr-1.5"
                    style={{ backgroundColor: COLORS[lib] ?? "#888" }}
                  />
                  <span className={lib === "ironmark" ? "text-zinc-100" : undefined}>
                    {parserLabel(lib)}
                  </span>
                </th>
              ))}
            </tr>
          </thead>
          <tbody>
            {analysis.groups.map((g) => (
              <Fragment key={g.title}>
                <tr>
                  <td
                    colSpan={libs.length + 1}
                    className="px-3 pt-3 pb-1 text-[11px] font-semibold uppercase tracking-wider text-zinc-600 bg-zinc-950"
                  >
                    {g.title}
                  </td>
                </tr>
                {g.rows.map((row) => {
                  const key = `${g.title}/${row.bench.name}`;
                  const isOpen = open === key;
                  return (
                    <Fragment key={key}>
                      <tr
                        onClick={() => setOpen(isOpen ? null : key)}
                        onKeyDown={(e) => {
                          if (e.key === "Enter" || e.key === " ") {
                            e.preventDefault();
                            setOpen(isOpen ? null : key);
                          }
                        }}
                        tabIndex={0}
                        className="border-t border-zinc-800/60 cursor-pointer hover:bg-zinc-900/60 focus-visible:outline focus-visible:outline-1 focus-visible:outline-zinc-500"
                        aria-expanded={isOpen}
                      >
                        <td className="sticky left-0 bg-zinc-950 px-3 py-1.5 whitespace-nowrap text-zinc-300">
                          <span className="text-zinc-600 mr-1.5 inline-block w-2">
                            {isOpen ? "▾" : "▸"}
                          </span>
                          {row.name}
                          {row.bench.bytes > 0 && (
                            <span className="ml-1.5 text-zinc-600">
                              {fmtBytes(row.bench.bytes)}
                            </span>
                          )}
                        </td>
                        {libs.map((lib) => {
                          const c = row.cells[lib];
                          return (
                            <td
                              key={lib}
                              className={`px-2 py-1.5 text-right whitespace-nowrap ${c ? cellClass(c) : "text-zinc-700"}`}
                              title={c ? cellTitle(lib, c, row.bench.bytes) : "not measured"}
                            >
                              {c ? (
                                <>
                                  {c.matches_majority === false && (
                                    <span className="text-amber-500 mr-1">≠</span>
                                  )}
                                  {c.tied && "≈"}
                                  {c.fastest ? fmtNs(c.median_ns) : fmtMultiple(c.ratio)}
                                </>
                              ) : (
                                "–"
                              )}
                            </td>
                          );
                        })}
                      </tr>
                      {isOpen && (
                        <tr className="bg-zinc-900/40">
                          <td colSpan={libs.length + 1} className="px-3 py-2">
                            {/* Stays in view while the table scrolls sideways on narrow screens. */}
                            <div className="sticky left-3 max-w-[calc(100vw-4rem)]">
                              <CaseDetail row={row} runtime={runtime} />
                            </div>
                          </td>
                        </tr>
                      )}
                    </Fragment>
                  );
                })}
              </Fragment>
            ))}
          </tbody>
        </table>
      </div>
      <div className="flex gap-x-4 gap-y-1 flex-wrap text-xs text-zinc-500">
        <span>
          <span className="px-1 rounded bg-emerald-500/15 text-emerald-300 font-semibold">
            12.3 µs
          </span>{" "}
          fastest (its time)
        </span>
        <span>
          <span className="text-zinc-300">1.40×</span> takes 1.4 times as long
        </span>
        <span>
          <span className="text-emerald-200/80">≈</span> tied with fastest, within noise
        </span>
        {anyDiffers && (
          <span>
            <span className="text-amber-500">≠</span> output differs from the other parsers
          </span>
        )}
        <span>Click a row for times and throughput.</span>
      </div>
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

// ─── BenchmarkPage ───────────────────────────────────────────────────────────

type InnerTab = "results" | "history";

export function BenchmarkPage() {
  const [data, setData] = useState<BenchmarkData | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [loading, setLoading] = useState(true);
  const [innerTab, setInnerTab] = useState<InnerTab>("results");
  const [trendMode, setTrendMode] = useState<TrendMode>("relative");
  const [runtime, setRuntime] = useState<Runtime>("wasm");

  useEffect(() => {
    fetch(`${import.meta.env.BASE_URL}benchmark-data.json`)
      .then((r) => (r.ok ? r.json() : Promise.reject(r.status)))
      .then((d: BenchmarkData) => setData(d))
      .catch(() => setError("No benchmark data found. Run pnpm bench to generate it."))
      .finally(() => setLoading(false));
  }, []);

  const available = useMemo(
    () => RUNTIMES.filter((r) => (data?.latest[r.key]?.sections.length ?? 0) > 0),
    [data],
  );
  const active = available.some((r) => r.key === runtime) ? runtime : available[0]?.key;

  const analyses = useMemo(() => {
    const out: Partial<Record<Runtime, Analysis>> = {};
    for (const r of available) out[r.key] = analyze(data!.latest[r.key]!.sections);
    return out;
  }, [available, data]);
  const analysis = active ? analyses[active] : undefined;

  return (
    <div className="flex-1 overflow-y-auto bg-zinc-950 text-zinc-100">
      <div className="max-w-3xl mx-auto px-4 py-6 space-y-6">
        <div className="space-y-1.5">
          <h2 className="text-lg font-semibold">Benchmarks</h2>
          <p className="text-sm text-zinc-400 leading-relaxed">
            How long ironmark takes to turn Markdown into HTML compared with other parsers. Every
            parser is set up for the same features
            {data?.profile === "gfm"
              ? " (CommonMark plus GFM tables, strikethrough, task lists and autolinks)"
              : data?.profile === "commonmark"
                ? " (plain CommonMark, no extensions)"
                : ""}
            , and each environment is compared on its own.
          </p>
          {data && (
            <p className="text-xs text-zinc-600">
              {data.env ? `Measured on ${fmtEnv(data.env)} · ` : ""}
              updated {fmtDate(data.generatedAt)} ·{" "}
              <a
                href="https://github.com/ph1p/ironmark/tree/main/benchmark#readme"
                target="_blank"
                rel="noreferrer"
                className="underline decoration-zinc-700 hover:text-zinc-400"
              >
                methodology &amp; how to run it yourself
              </a>
            </p>
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
            <div className="flex gap-4 border-b border-zinc-800">
              {(
                [
                  ["results", "Results"],
                  ["history", "History"],
                ] as const
              ).map(([tab, label]) => (
                <button
                  key={tab}
                  onClick={() => setInnerTab(tab)}
                  className={`pb-2 text-sm font-medium border-b-2 -mb-px transition-colors ${
                    innerTab === tab
                      ? "border-zinc-100 text-zinc-100"
                      : "border-transparent text-zinc-500 hover:text-zinc-300"
                  }`}
                >
                  {label}
                </button>
              ))}
            </div>

            {innerTab === "results" && active && analysis && (
              <div className="space-y-5">
                <div className="space-y-2">
                  <div
                    role="tablist"
                    aria-label="Environment"
                    className="inline-flex rounded-lg border border-zinc-800 bg-zinc-900 p-0.5"
                  >
                    {available.map((r) => (
                      <button
                        key={r.key}
                        role="tab"
                        aria-selected={active === r.key}
                        onClick={() => setRuntime(r.key)}
                        className={`rounded-md px-3 py-1.5 text-sm transition-colors ${
                          active === r.key
                            ? "bg-zinc-700 text-zinc-100"
                            : "text-zinc-400 hover:text-zinc-200"
                        }`}
                      >
                        {r.label}
                      </button>
                    ))}
                  </div>
                  <p className="text-xs text-zinc-500 leading-relaxed">
                    {RUNTIMES.find((r) => r.key === active)!.blurb}
                  </p>
                </div>

                <Summary analysis={analysis} runtime={active} />
                <ResultsTable key={active} analysis={analysis} runtime={active} />

                <p className="text-xs text-zinc-600 leading-relaxed">
                  Numbers come from one machine and are only comparable with each other, not with
                  other machines. Differences of a few percent are usually noise.
                </p>
              </div>
            )}

            {innerTab === "history" && (
              <div className="space-y-4">
                <p className="text-sm text-zinc-400 leading-relaxed">
                  ironmark on the CommonMark spec document (all 652 examples) across recorded runs.
                </p>
                <div className="flex gap-2 flex-wrap">
                  {(
                    [
                      ["relative", "Compared with the fastest other parser"],
                      ["absolute", "Parse time"],
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
                    ? "Median time per parse. Lower is faster. Runs on different machines are not comparable."
                    : "ironmark's time divided by the fastest other parser's time in the same environment. Below 1.00× means ironmark is faster (0.60× = takes 60% as long)."}
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
