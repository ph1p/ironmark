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
    ("markdown_it", parse_markdown_it),
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

fn parse_markdown_it(input: &str) -> String {
    let parser = &mut markdown_it::MarkdownIt::new();
    markdown_it::plugins::cmark::add(parser);
    markdown_it::plugins::extra::add(parser);
    let ast = parser.parse(input);
    ast.render()
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
    // Large file benchmarks need fewer samples
    for &size in &[1_000_000, 5_000_000] {
        let input: String = base.chars().cycle().take(size).collect();
        let label = format!("{} bytes", input.len());
        let mut group = c.benchmark_group(format!("document_size/{size} bytes"));
        group.sample_size(10);
        for &(name, func) in PARSERS {
            group.bench_with_input(BenchmarkId::new(name, &label), &*input, |b, input| {
                b.iter(|| func(black_box(input)))
            });
        }
        group.finish();
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

fn gen_pathological_backticks(n: usize) -> String {
    (1..=n).map(|i| "`".repeat(i) + " ").collect()
}

fn gen_pathological_emphasis(n: usize) -> String {
    "*a ".repeat(n)
}

fn gen_many_ref_links(n: usize) -> String {
    let mut s = String::from("[refdef]: http://example.com/very/long/url \"Title\"\n\n");
    for _ in 0..n {
        s.push_str("See [refdef] and [refdef] and [refdef].\n\n");
    }
    s
}

fn bench_pathological(c: &mut Criterion) {
    let cases: Vec<(&str, String)> = vec![
        ("backticks_500", gen_pathological_backticks(500)),
        ("emphasis_10k", gen_pathological_emphasis(10_000)),
        ("table_5k_rows", gen_table(5_000, 20)),
        ("ref_links_5k", gen_many_ref_links(5_000)),
    ];
    for (name, input) in &cases {
        let label = format!("{} bytes", input.len());
        let mut group = c.benchmark_group(format!("pathological/{name}"));
        group.sample_size(10);
        for &(pname, func) in PARSERS {
            group.bench_with_input(BenchmarkId::new(pname, &label), &**input, |b, input| {
                b.iter(|| func(black_box(input)))
            });
        }
        group.finish();
    }
}

/// Generate a realistic mixed-content document with the given number of lines.
fn gen_mixed_doc(lines: usize) -> String {
    let mut s = String::new();
    let mut line = 0;
    while line < lines {
        let section = line / 20 % 6;
        match section {
            0 => {
                // Paragraphs with inline formatting
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
                // Bullet list
                for i in 0..6.min(lines - line) {
                    s.push_str(&format!("- List item {i} with some text\n"));
                    line += 1;
                }
                s.push('\n');
                line += 1;
            }
            2 => {
                // Code block
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
                // Blockquote
                for _ in 0..3.min(lines - line) {
                    s.push_str("> Quoted text with *emphasis* and **strong**.\n");
                    line += 1;
                }
                s.push('\n');
                line += 1;
            }
            4 => {
                // Plain paragraphs (no inline markup)
                for _ in 0..5.min(lines - line) {
                    s.push_str(
                        "Plain text without any special formatting or markup characters at all.\n",
                    );
                    line += 1;
                }
                s.push('\n');
                line += 1;
            }
            _ => {
                // Ordered list with inline
                for i in 0..4.min(lines - line) {
                    s.push_str(&format!("{}. Item with `code` and **bold**\n", i + 1));
                    line += 1;
                }
                s.push('\n');
                line += 1;
            }
        }
    }
    s
}

fn bench_large_lines(c: &mut Criterion) {
    for &num_lines in &[10_000, 20_000] {
        let input = gen_mixed_doc(num_lines);
        let label = format!("{} lines / {} bytes", num_lines, input.len());
        let mut group = c.benchmark_group(format!("large_lines/{num_lines}"));
        group.sample_size(10);
        for &(name, func) in PARSERS {
            group.bench_with_input(BenchmarkId::new(name, &label), &*input, |b, input| {
                b.iter(|| func(black_box(input)))
            });
        }
        group.finish();
    }
}

criterion_group!(
    benches,
    bench_spec,
    bench_sizes,
    bench_block_types,
    bench_inline,
    bench_pathological,
    bench_large_lines,
);
criterion_main!(benches);
