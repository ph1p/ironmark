use criterion::{BenchmarkId, Criterion, black_box, criterion_group, criterion_main};
use ironmark::{ParseOptions, parse};

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

// --- Parser wrappers ---

type ParserFn = fn(&str) -> String;

const PARSERS: &[(&str, ParserFn)] = &[
    ("ironmark", parse_ironmark),
    ("pulldown_cmark", parse_pulldown_cmark),
    ("comrak", parse_comrak),
    ("markdown_rs", parse_markdown_rs),
];

fn parse_ironmark(input: &str) -> String {
    let opts = ParseOptions::default();
    parse(input, &opts)
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

// --- Benchmark helper ---

fn bench_group(c: &mut Criterion, group_name: &str, input: &str) {
    let label = format!("{} bytes", input.len());
    let mut group = c.benchmark_group(group_name);
    for &(name, func) in PARSERS {
        group.bench_with_input(BenchmarkId::new(name, &label), input, |b, input| {
            b.iter(|| func(black_box(input)))
        });
    }
    group.finish();
}

// --- Benchmarks ---

fn bench_spec(c: &mut Criterion) {
    let input = load_spec_markdown();
    bench_group(c, "commonmark_spec", &input);
}

fn bench_sizes(c: &mut Criterion) {
    let base = gen_inline_heavy();
    for &size in &[1_000, 10_000, 100_000] {
        let input: String = base.chars().cycle().take(size).collect();
        bench_group(c, &format!("document_size/{size} bytes"), &input);
    }
}

fn bench_block_types(c: &mut Criterion) {
    let cases: Vec<(&str, String)> = vec![
        ("headings", gen_heading_doc(200)),
        ("nested_lists", gen_nested_list(50)),
        ("table", gen_table(100, 10)),
        ("code_blocks", gen_code_blocks(100)),
    ];
    for (name, input) in &cases {
        bench_group(c, &format!("block_types/{name}"), input);
    }
}

fn bench_inline(c: &mut Criterion) {
    let input = gen_inline_heavy();
    bench_group(c, "inline_heavy", &input);
}

criterion_group!(
    benches,
    bench_spec,
    bench_sizes,
    bench_block_types,
    bench_inline,
);
criterion_main!(benches);
