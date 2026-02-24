use ironmark::{ParseOptions, parse};
use serde::Deserialize;
use std::collections::BTreeMap;

#[derive(Deserialize)]
struct SpecTest {
    markdown: String,
    html: String,
    example: u32,
    section: String,
}

#[test]
fn commonmark_0_31_2_spec() {
    let spec_json = include_str!("./spec/spec-0.31.2.json");
    let tests: Vec<SpecTest> = serde_json::from_str(spec_json).expect("Failed to parse spec JSON");

    let mut pass = 0u32;
    let mut fail = 0u32;
    let mut failures = Vec::new();
    let mut section_stats: BTreeMap<String, (u32, u32)> = BTreeMap::new();

    for test in &tests {
        let opts = ParseOptions {
            hard_breaks: false,
            enable_autolink: false,
            ..Default::default()
        };
        let result = parse(&test.markdown, &opts);
        let entry = section_stats.entry(test.section.clone()).or_insert((0, 0));
        if result == test.html {
            pass += 1;
            entry.0 += 1;
        } else {
            fail += 1;
            entry.1 += 1;
            if failures.len() < 300 {
                failures.push(format!(
                    "FAIL example {} ({})\n  input:    {:?}\n  expected: {:?}\n  got:      {:?}",
                    test.example, test.section, test.markdown, test.html, result
                ));
            }
        }
    }

    eprintln!("\n=== CommonMark 0.31.2 Spec Results ===");
    eprintln!("{pass}/{} passed ({fail} failed)\n", pass + fail);

    eprintln!("Section breakdown:");
    for (section, (p, f)) in &section_stats {
        let total = p + f;
        let status = if *f == 0 { "  OK" } else { "FAIL" };
        eprintln!("  {status} {section}: {p}/{total}");
    }

    if !failures.is_empty() {
        eprintln!("\nFirst {} failures:", failures.len());
        for f in &failures {
            eprintln!("{f}");
        }
    }

    if fail > 0 {
        panic!("{fail} spec tests failed (see details above)");
    }
}
