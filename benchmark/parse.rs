use criterion::{BenchmarkId, Criterion, criterion_group, criterion_main};
use ironmark::{ParseOptions, render_html};
// Hotspot/diagnostic entry points are only used by the `bench-extra` groups.
#[cfg(feature = "bench-extra")]
use ironmark::{
    __benchmark_heading_slug, __benchmark_parse_inline, __benchmark_parse_table_row,
    __benchmark_render_html_parse_phase, parse_markdown,
};
use std::hint::black_box;

type BenchCase = (&'static str, fn() -> String);

// --- md4c FFI (only when the "bench-md4c" feature is enabled) ---

#[cfg(feature = "bench-md4c")]
mod md4c_ffi {
    use std::ffi::c_void;

    #[link(name = "md4c-html")]
    unsafe extern "C" {
        pub fn md_html(
            input: *const u8,
            input_size: u32,
            process_output: unsafe extern "C" fn(*const u8, u32, *mut c_void),
            userdata: *mut c_void,
            parser_flags: u32,
            renderer_flags: u32,
        ) -> i32;
    }

    pub unsafe extern "C" fn output_cb(text: *const u8, size: u32, userdata: *mut c_void) {
        // SAFETY: userdata is &mut String cast to *mut c_void
        let buf = unsafe { &mut *(userdata as *mut String) };
        let slice = unsafe { std::slice::from_raw_parts(text, size as usize) };
        buf.push_str(&String::from_utf8_lossy(slice));
    }
}

// md4c parser flag constants (from md4c.h)
#[cfg(feature = "bench-md4c")]
const MD_FLAG_PERMISSIVEURLAUTOLINKS: u32 = 0x0004;
#[cfg(feature = "bench-md4c")]
const MD_FLAG_PERMISSIVEEMAILAUTOLINKS: u32 = 0x0008;
#[cfg(feature = "bench-md4c")]
const MD_FLAG_TABLES: u32 = 0x0100;
#[cfg(feature = "bench-md4c")]
const MD_FLAG_STRIKETHROUGH: u32 = 0x0200;
#[cfg(feature = "bench-md4c")]
const MD_FLAG_TASKLISTS: u32 = 0x0800;
#[cfg(feature = "bench-md4c")]
const MD_FLAG_UNDERLINE: u32 = 0x4000;
#[cfg(feature = "bench-md4c")]
const MD_FLAG_HARD_SOFT_BREAKS: u32 = 0x8000;

#[cfg(feature = "bench-md4c")]
fn parse_md4c(input: &str) -> String {
    use md4c_ffi::{md_html, output_cb};
    // Match ironmark's default feature set as closely as md4c allows.
    // md4c has no Highlight (==) equivalent — excluded from both sides.
    const FLAGS: u32 = MD_FLAG_TABLES
        | MD_FLAG_STRIKETHROUGH
        | MD_FLAG_TASKLISTS
        | MD_FLAG_PERMISSIVEURLAUTOLINKS
        | MD_FLAG_PERMISSIVEEMAILAUTOLINKS
        | MD_FLAG_HARD_SOFT_BREAKS
        | MD_FLAG_UNDERLINE;
    let mut output = String::with_capacity(input.len() * 2);
    // SAFETY: md_html does not store the pointers beyond the call; output lives for the duration
    unsafe {
        md_html(
            input.as_ptr(),
            input.len() as u32,
            output_cb,
            &raw mut output as *mut std::ffi::c_void,
            FLAGS,
            0,
        );
    }
    output
}

// --- Input generators ---

fn load_spec_markdown() -> String {
    let json = include_str!("../tests/spec/spec-0.31.2.json");
    let specs: Vec<serde_json::Value> = serde_json::from_str(json).unwrap();
    specs
        .iter()
        .map(|s| s["markdown"].as_str().unwrap())
        .collect::<Vec<_>>()
        .join("\n")
}

fn gen_heading_doc(n: usize) -> String {
    (1..=n)
        .map(|i| format!("# Heading {i}\n\nSome paragraph text under heading {i}.\n"))
        .collect()
}

fn gen_nested_list(depth: usize) -> String {
    let mut s = String::new();
    for i in 0..depth {
        s.push_str(&"  ".repeat(i));
        s.push_str(&format!("- item {i}\n"));
    }
    s
}

fn gen_table(rows: usize, cols: usize) -> String {
    let mut s = String::new();
    s.push('|');
    for c in 0..cols {
        s.push_str(&format!(" col{c} |"));
    }
    s.push('\n');
    s.push('|');
    for _ in 0..cols {
        s.push_str(" --- |");
    }
    s.push('\n');
    for r in 0..rows {
        s.push('|');
        for c in 0..cols {
            s.push_str(&format!(" r{r}c{c} |"));
        }
        s.push('\n');
    }
    s
}

fn gen_inline_heavy() -> String {
    let mut s = String::new();
    for i in 0..200 {
        s.push_str(&format!(
            "This has **bold**, *italic*, `code`, ~~strike~~, [link](http://x.com/{i}), and more.\n\n"
        ));
    }
    s
}

fn gen_code_blocks(n: usize) -> String {
    (0..n)
        .map(|i| format!("```rust\nfn func_{i}() {{\n    println!(\"hello\");\n}}\n```\n\n"))
        .collect()
}

fn gen_all_features_doc() -> &'static str {
    include_str!("all_features.md")
}

#[cfg(feature = "bench-extra")]
fn gen_mixed_doc(lines: usize) -> String {
    let mut s = String::new();
    let mut line = 0;
    while line < lines {
        match line / 20 % 6 {
            0 => {
                s.push_str(&format!("# Section {}\n\n", line / 20));
                line += 2;
                for i in 0..4.min(lines - line) {
                    s.push_str(&format!(
                        "This is paragraph text with **bold**, *italic*, `code`, and [a link](http://example.com/{i}).\n"
                    ));
                    line += 1;
                }
                s.push('\n');
                line += 1;
            }
            1 => {
                for i in 0..6.min(lines - line) {
                    s.push_str(&format!("- List item {i} with some text\n"));
                    line += 1;
                }
                s.push('\n');
                line += 1;
            }
            2 => {
                s.push_str("```rust\n");
                line += 1;
                for i in 0..5.min(lines - line) {
                    s.push_str(&format!("    let x_{i} = compute({i});\n"));
                    line += 1;
                }
                s.push_str("```\n\n");
                line += 2;
            }
            3 => {
                for _ in 0..3.min(lines - line) {
                    s.push_str("> Quoted text with *emphasis* and **strong**.\n");
                    line += 1;
                }
                s.push('\n');
                line += 1;
            }
            4 => {
                for _ in 0..5.min(lines - line) {
                    s.push_str(
                        "Plain text without any special formatting or markup characters at all.\n",
                    );
                    line += 1;
                }
                s.push('\n');
                line += 1;
            }
            5 => {
                for i in 0..4.min(lines - line) {
                    s.push_str(&format!("{}. Item with `code` and **bold**\n", i + 1));
                    line += 1;
                }
                s.push('\n');
                line += 1;
            }
            _ => unreachable!(),
        }
    }
    s
}

#[cfg(feature = "bench-extra")]
fn gen_pathological_backticks(n: usize) -> String {
    (1..=n).map(|i| "`".repeat(i) + " ").collect()
}

#[cfg(feature = "bench-extra")]
fn gen_pathological_emphasis(n: usize) -> String {
    "*a ".repeat(n)
}

#[cfg(feature = "bench-extra")]
fn gen_many_ref_links(n: usize) -> String {
    let mut s = String::from("[refdef]: http://example.com/very/long/url \"Title\"\n\n");
    for _ in 0..n {
        s.push_str("See [refdef] and [refdef] and [refdef].\n\n");
    }
    s
}

// --- Parser wrappers ---

type ParserFn = fn(&str) -> String;

fn parsers() -> Vec<(&'static str, ParserFn)> {
    #[allow(unused_mut)]
    let mut v: Vec<(&'static str, ParserFn)> = vec![
        ("ironmark", parse_ironmark),
        ("pulldown_cmark", parse_pulldown_cmark),
        ("comrak", parse_comrak),
        ("markdown_rs", parse_markdown_rs),
        ("markdown_it", parse_markdown_it),
    ];
    #[cfg(feature = "bench-md4c")]
    v.push(("md4c", parse_md4c));
    v
}

fn parse_ironmark(input: &str) -> String {
    render_html(input, &ParseOptions::default())
}

fn parse_pulldown_cmark(input: &str) -> String {
    let mut opts = pulldown_cmark::Options::empty();
    opts.insert(pulldown_cmark::Options::ENABLE_STRIKETHROUGH);
    opts.insert(pulldown_cmark::Options::ENABLE_TABLES);
    opts.insert(pulldown_cmark::Options::ENABLE_TASKLISTS);
    let parser = pulldown_cmark::Parser::new_ext(input, opts);
    let mut html = String::new();
    pulldown_cmark::html::push_html(&mut html, parser);
    html
}

fn parse_comrak(input: &str) -> String {
    let mut options = comrak::Options::default();
    options.extension.strikethrough = true;
    options.extension.table = true;
    options.extension.autolink = true;
    options.extension.tasklist = true;
    comrak::markdown_to_html(input, &options)
}

fn parse_markdown_rs(input: &str) -> String {
    markdown::to_html(input)
}

fn parse_markdown_it(input: &str) -> String {
    use std::sync::OnceLock;
    static PARSER: OnceLock<markdown_it::MarkdownIt> = OnceLock::new();
    let parser = PARSER.get_or_init(|| {
        let mut p = markdown_it::MarkdownIt::new();
        markdown_it::plugins::cmark::add(&mut p);
        markdown_it::plugins::extra::add(&mut p);
        p
    });
    parser.parse(input).render()
}

// --- Timing configuration ---
//
// Tuned per group so total wall time stays reasonable:
//   - fast groups (ironmark ~µs range): 300ms warmup + 1s measurement, 50 samples
//   - slow groups (markdown_rs/markdown_it can hit ms range): capped at 10 samples

#[derive(Clone, Copy)]
struct TimingConfig {
    warmup_ms: u64,
    measure_ms: u64,
    sample_size: usize,
}

impl TimingConfig {
    // BENCH_QUICK=1 shrinks warmup/measurement for a fast dev loop; numbers
    // are noisier and must not be committed to history.
    fn effective(self) -> TimingConfig {
        if quick_mode() {
            TimingConfig {
                warmup_ms: 150,
                measure_ms: 400,
                sample_size: 10.max(self.sample_size / 5),
            }
        } else {
            self
        }
    }
}

const NORMAL: TimingConfig = TimingConfig {
    warmup_ms: 500,
    measure_ms: 2000,
    sample_size: 100,
};
const SLOW: TimingConfig = TimingConfig {
    warmup_ms: 500,
    measure_ms: 3000,
    sample_size: 50,
};
#[cfg(feature = "bench-extra")]
const PATHOLOGICAL: TimingConfig = TimingConfig {
    warmup_ms: 300,
    measure_ms: 1000,
    sample_size: 30,
};

// --- Benchmark helper ---

fn quick_mode() -> bool {
    std::env::var_os("BENCH_QUICK").is_some_and(|v| v != "0")
}

fn bench_group_cfg(c: &mut Criterion, group_name: &str, input: &str, cfg: &TimingConfig) {
    let label = format!("{} bytes", input.len());
    let mut group = c.benchmark_group(group_name);
    let cfg = cfg.effective();
    group.warm_up_time(std::time::Duration::from_millis(cfg.warmup_ms));
    group.measurement_time(std::time::Duration::from_millis(cfg.measure_ms));
    group.sample_size(cfg.sample_size);
    let parsers = parsers();
    for (name, func) in &parsers {
        group.bench_with_input(BenchmarkId::new(*name, &label), input, |b, input| {
            b.iter(|| func(black_box(input)))
        });
    }
    group.finish();
}

fn bench_group(c: &mut Criterion, group_name: &str, input: &str) {
    bench_group_cfg(c, group_name, input, &NORMAL);
}

// --- Benchmarks ---

fn bench_spec(c: &mut Criterion) {
    let input = load_spec_markdown();
    bench_group_cfg(c, "commonmark_spec", &input, &SLOW);
}

fn bench_sizes(c: &mut Criterion) {
    let base = gen_inline_heavy();
    for &size in &[1_000, 10_000, 100_000] {
        let input: String = base.chars().cycle().take(size).collect();
        bench_group(c, &format!("document_size_{size} bytes"), &input);
    }
    {
        let size = 1_000_000;
        let input: String = base.chars().cycle().take(size).collect();
        bench_group_cfg(c, &format!("document_size_{size} bytes"), &input, &SLOW);
    }
}

fn bench_block_types(c: &mut Criterion) {
    let cases: &[BenchCase] = &[
        ("headings", || gen_heading_doc(200)),
        ("nested_lists", || gen_nested_list(50)),
        ("table", || gen_table(100, 10)),
        ("code_blocks", || gen_code_blocks(100)),
    ];
    for (name, make) in cases {
        bench_group(c, &format!("block_types/{name}"), &make());
    }
}

fn bench_inline(c: &mut Criterion) {
    bench_group(c, "inline_heavy", &gen_inline_heavy());
}

#[cfg(feature = "bench-extra")]
fn bench_pathological(c: &mut Criterion) {
    let cases: &[BenchCase] = &[
        ("backticks_500", || gen_pathological_backticks(500)),
        ("emphasis_10k", || gen_pathological_emphasis(10_000)),
        ("table_1k_rows", || gen_table(1_000, 10)),
        ("ref_links_1k", || gen_many_ref_links(1_000)),
    ];
    for (name, make) in cases {
        bench_group_cfg(c, &format!("pathological/{name}"), &make(), &PATHOLOGICAL);
    }
}

#[cfg(feature = "bench-extra")]
fn bench_large_lines(c: &mut Criterion) {
    let input = gen_mixed_doc(10_000);
    bench_group_cfg(c, "large_lines/10000", &input, &SLOW);
}

fn bench_all_features(c: &mut Criterion) {
    bench_group(c, "all_features", gen_all_features_doc());
}

#[cfg(feature = "bench-extra")]
fn bench_code_blocks_split(c: &mut Criterion) {
    let input = gen_code_blocks(100);
    let opts = ParseOptions::default();
    let mut group = c.benchmark_group("code_blocks_split");
    group.warm_up_time(std::time::Duration::from_millis(300));
    group.measurement_time(std::time::Duration::from_millis(1000));
    group.sample_size(50);
    group.bench_function("parse_only_ast", |b| {
        b.iter(|| parse_markdown(black_box(&input), black_box(&opts)))
    });
    group.bench_function("render_html_parse_phase", |b| {
        b.iter(|| __benchmark_render_html_parse_phase(black_box(&input), black_box(&opts)))
    });
    group.bench_function("render_html_full", |b| {
        b.iter(|| render_html(black_box(&input), black_box(&opts)))
    });
    group.finish();
}

#[cfg(feature = "bench-extra")]
fn bench_hotspots(c: &mut Criterion) {
    let opts = ParseOptions {
        enable_heading_ids: true,
        enable_heading_anchors: true,
        ..ParseOptions::default()
    };
    let inline =
        "This has **bold**, *italic*, `code`, ~~strike~~, [link](http://x.com/1), and more.";
    let heading = "A Heading with **Bold**, `Code`, links [x](y), and Unicode äöü";
    let table_row =
        "| r0c0 | r0c1 with **bold** | r0c2 | r0c3 | r0c4 | r0c5 | r0c6 | r0c7 | r0c8 | r0c9 |";

    let mut group = c.benchmark_group("hotspots");
    group.warm_up_time(std::time::Duration::from_millis(300));
    group.measurement_time(std::time::Duration::from_millis(1000));
    group.sample_size(50);
    group.bench_function("inline_pass", |b| {
        b.iter(|| __benchmark_parse_inline(black_box(inline), black_box(&opts)))
    });
    group.bench_function("heading_slug", |b| {
        b.iter(|| __benchmark_heading_slug(black_box(heading)))
    });
    group.bench_function("table_row_parse", |b| {
        b.iter(|| __benchmark_parse_table_row(black_box(table_row), black_box(10)))
    });
    group.finish();
}

// --- CSV export ---
//
// Reads median_ns from criterion's own estimates.json files — no re-measurement.
// Writes to benchmark/history/YYYY-MM-DD.csv (overwrites same-day file).

fn csv_date() -> String {
    let secs = std::env::var("SOURCE_DATE_EPOCH")
        .ok()
        .and_then(|v| v.parse::<u64>().ok())
        .unwrap_or_else(|| {
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs()
        });
    let days = secs / 86400;
    let mut y = 1970u32;
    let mut d = days as u32;
    loop {
        let leap = (y.is_multiple_of(4) && !y.is_multiple_of(100)) || y.is_multiple_of(400);
        let days_in_year = if leap { 366 } else { 365 };
        if d < days_in_year {
            break;
        }
        d -= days_in_year;
        y += 1;
    }
    let leap = (y.is_multiple_of(4) && !y.is_multiple_of(100)) || y.is_multiple_of(400);
    let month_days: [u32; 12] = [
        31,
        if leap { 29 } else { 28 },
        31,
        30,
        31,
        30,
        31,
        31,
        30,
        31,
        30,
        31,
    ];
    let mut m = 0u32;
    while m < 12 && d >= month_days[m as usize] {
        d -= month_days[m as usize];
        m += 1;
    }
    format!("{y:04}-{:02}-{:02}", m + 1, d + 1)
}

/// Walk criterion's output directory and collect all estimates.
/// Returns (group_path, parser, input_bytes, median_ns).
fn collect_criterion_results() -> Vec<(String, String, usize, f64)> {
    let criterion_dir =
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("target/criterion");
    let known_parsers = [
        "ironmark",
        "pulldown_cmark",
        "comrak",
        "markdown_rs",
        "markdown_it",
        "md4c",
    ];
    let mut rows = Vec::new();
    collect_criterion_dir(&criterion_dir, &criterion_dir, &known_parsers, &mut rows);
    rows
}

fn collect_criterion_dir(
    base: &std::path::Path,
    dir: &std::path::Path,
    known_parsers: &[&str],
    out: &mut Vec<(String, String, usize, f64)>,
) {
    let entries = match std::fs::read_dir(dir) {
        Ok(e) => e,
        Err(_) => return,
    };
    for entry in entries.flatten() {
        if !entry.file_type().map(|t| t.is_dir()).unwrap_or(false) {
            continue;
        }
        let name = entry.file_name().into_string().unwrap_or_default();
        if name == "report" {
            continue;
        }
        let path = entry.path();
        let estimates = path.join("new/estimates.json");
        match std::fs::read_to_string(&estimates) {
            Ok(json) => {
                // Path relative to criterion dir: group/parser/label
                let rel = path.strip_prefix(base).unwrap_or(&path);
                let parts: Vec<&str> = rel.to_str().unwrap_or("").split('/').collect();
                if let Some(parser_idx) = parts.iter().position(|p| known_parsers.contains(p)) {
                    let group = parts[..parser_idx].join("/");
                    let parser = parts[parser_idx].to_string();
                    let label = parts[parser_idx + 1..].join("/");
                    let input_bytes: usize = label
                        .split_whitespace()
                        .next()
                        .and_then(|s| s.parse().ok())
                        .unwrap_or(0);
                    if let Ok(v) = serde_json::from_str::<serde_json::Value>(&json)
                        && let Some(ns) = v["median"]["point_estimate"].as_f64()
                    {
                        out.push((group, parser, input_bytes, ns));
                    }
                }
            }
            Err(_) => {
                collect_criterion_dir(base, &path, known_parsers, out);
            }
        }
    }
}

fn export_csv(c: &mut Criterion) {
    // No measurement — just read what criterion already wrote.
    let _ = c;

    // Quick-mode numbers are noisy and the run may be filtered to a subset of
    // parsers — mixing them with stale full-run data would corrupt the CSV.
    if quick_mode() {
        println!("export_csv: skipped (BENCH_QUICK set)");
        return;
    }

    let manifest = env!("CARGO_MANIFEST_DIR");
    let history_dir = format!("{manifest}/benchmark/history");
    std::fs::create_dir_all(&history_dir).expect("create benchmark/history");
    let date = csv_date();
    let dated_path = format!("{history_dir}/{date}.csv");

    let rows = collect_criterion_results();

    // Write all rows in one open — header + data, overwriting any same-day file.
    {
        use std::io::Write;
        let mut f = std::fs::File::create(&dated_path)
            .unwrap_or_else(|e| panic!("create {dated_path}: {e}"));
        writeln!(f, "date,group,parser,input_bytes,median_ns").unwrap();
        for (group, parser, input_bytes, median_ns) in &rows {
            writeln!(f, "{date},{group},{parser},{input_bytes},{median_ns:.0}").unwrap();
        }
    }

    if rows.is_empty() {
        eprintln!("export_csv: no criterion results found in target/criterion/");
    } else {
        eprintln!(
            "export_csv: wrote {} rows to history/{date}.csv",
            rows.len()
        );
    }
}

// Plot/report generation is pure overhead here — the history JSON is the
// record of note, and 60+ per-bench HTML reports add minutes of wall time.
fn config() -> Criterion {
    Criterion::default().without_plots()
}

// Core groups: exported to the history JSON, always run.
#[cfg(not(feature = "bench-extra"))]
criterion_group!(
    name = benches;
    config = config();
    targets = bench_spec,
    bench_sizes,
    bench_block_types,
    bench_inline,
    bench_all_features,
    export_csv,
);

// With `bench-extra`: also run the diagnostic groups (not exported to history).
#[cfg(feature = "bench-extra")]
criterion_group!(
    name = benches;
    config = config();
    targets = bench_spec,
    bench_sizes,
    bench_block_types,
    bench_inline,
    bench_pathological,
    bench_large_lines,
    bench_all_features,
    bench_hotspots,
    bench_code_blocks_split,
    export_csv,
);
criterion_main!(benches);
