//! Single-threaded allocation diagnostics; never linked into the production library.
use ironmark::{ParseOptions, parse_markdown, render_ansi_terminal, render_html};
use std::alloc::{GlobalAlloc, Layout, System};
use std::hint::black_box;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering::Relaxed};

struct CountingAllocator;
static ENABLED: AtomicBool = AtomicBool::new(false);
static ALLOCS: AtomicUsize = AtomicUsize::new(0);
static REALLOCS: AtomicUsize = AtomicUsize::new(0);
static REQUESTED: AtomicUsize = AtomicUsize::new(0);
static LIVE: AtomicUsize = AtomicUsize::new(0);
static PEAK: AtomicUsize = AtomicUsize::new(0);
static FAILURES: AtomicUsize = AtomicUsize::new(0);
static INVALID: AtomicBool = AtomicBool::new(false);

fn add_live(bytes: usize) {
    let live = LIVE.fetch_add(bytes, Relaxed) + bytes;
    PEAK.fetch_max(live, Relaxed);
}

fn remove_live(bytes: usize) {
    if LIVE
        .try_update(Relaxed, Relaxed, |live| live.checked_sub(bytes))
        .is_err()
    {
        INVALID.store(true, Relaxed);
    }
}

fn allocated(ptr: *mut u8, size: usize) {
    if ENABLED.load(Relaxed) {
        ALLOCS.fetch_add(1, Relaxed);
        REQUESTED.fetch_add(size, Relaxed);
        if ptr.is_null() {
            FAILURES.fetch_add(1, Relaxed);
        } else {
            add_live(size);
        }
    }
}

// SAFETY: every operation forwards its unchanged pointer/layout contract to System.
// The diagnostics use only atomics, never allocate, and never alter returned pointers.
unsafe impl GlobalAlloc for CountingAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        // SAFETY: caller supplies a valid allocation layout.
        let ptr = unsafe { System.alloc(layout) };
        allocated(ptr, layout.size());
        ptr
    }

    unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
        // SAFETY: caller supplies a valid allocation layout.
        let ptr = unsafe { System.alloc_zeroed(layout) };
        allocated(ptr, layout.size());
        ptr
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        if ENABLED.load(Relaxed) {
            remove_live(layout.size());
        }
        // SAFETY: caller guarantees ptr and layout describe a live System allocation.
        unsafe { System.dealloc(ptr, layout) };
    }

    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        // SAFETY: caller guarantees the original allocation and new size are valid.
        let new_ptr = unsafe { System.realloc(ptr, layout, new_size) };
        if ENABLED.load(Relaxed) {
            REALLOCS.fetch_add(1, Relaxed);
            REQUESTED.fetch_add(new_size, Relaxed);
            if new_ptr.is_null() {
                // Failed realloc leaves the original allocation live.
                FAILURES.fetch_add(1, Relaxed);
            } else {
                // Logical requested bytes, not transient allocator internals or RSS.
                remove_live(layout.size());
                add_live(new_size);
            }
        }
        new_ptr
    }
}

#[global_allocator]
static ALLOCATOR: CountingAllocator = CountingAllocator;

fn measure<T>(case: &str, api: &str, input_bytes: usize, mut run: impl FnMut() -> T) {
    drop(black_box(run())); // Warm all lazy setup outside the counted region.
    for counter in [&ALLOCS, &REALLOCS, &REQUESTED, &LIVE, &PEAK, &FAILURES] {
        counter.store(0, Relaxed);
    }
    INVALID.store(false, Relaxed);
    ENABLED.store(true, Relaxed);
    let result = black_box(run());
    let snapshot = (
        ALLOCS.load(Relaxed),
        REALLOCS.load(Relaxed),
        REQUESTED.load(Relaxed),
        LIVE.load(Relaxed),
        PEAK.load(Relaxed),
        FAILURES.load(Relaxed),
        INVALID.load(Relaxed),
    );
    ENABLED.store(false, Relaxed);
    assert!(!snapshot.6, "untracked allocation freed in counted region");
    let record = serde_json::json!({
        "case": case, "api": api, "input_bytes": input_bytes,
        "alloc_calls": snapshot.0, "realloc_calls": snapshot.1,
        "requested_bytes": snapshot.2, "live_requested_bytes": snapshot.3,
        "peak_live_requested_bytes": snapshot.4, "failed_calls": snapshot.5,
    });
    print!("{record}");
    drop(result); // Keep the returned output alive through the snapshot.
}

fn main() {
    // No threads are spawned. All input/options/formatting setup precedes counting.
    let options = ParseOptions::default();
    let mut cases = vec![
        ("empty".to_owned(), String::new()),
        ("short-plain".to_owned(), "Hello, world.\n".to_owned()),
        (
            "short-inline".to_owned(),
            "**bold** and *em* and `code`\n".to_owned(),
        ),
        (
            "complex-inline".to_owned(),
            "[**bold *em***](https://example.com) ~~del~~ ==mark== ++u++ &amp; ![alt](x)\n"
                .repeat(8),
        ),
        (
            "references".to_owned(),
            format!(
                "[r]: https://example.com \"title\"\n\n{}",
                "[r] and [r]\n\n".repeat(32)
            ),
        ),
        (
            "references-same-paragraph".to_owned(),
            "[r]: https://example.com \"title\"\nSee [r] and [r] without a blank separator.   \n"
                .to_owned(),
        ),
        (
            "code-blocks".to_owned(),
            "```rust\nfn main() { println!(\"hello\"); }\n```\n\n".repeat(16),
        ),
        (
            "table-discarded-escaped-pipe".to_owned(),
            "| h |\n| --- |\n| a\\|b | discarded | extra |\n".to_owned(),
        ),
    ];
    for cols in [1, 8, 9, 10] {
        let row = format!("|{}|\n", vec![" cell "; cols].join("|"));
        let separator = format!("|{}|\n", vec![" --- "; cols].join("|"));
        cases.push((
            format!("table-{cols}-cols"),
            format!("{row}{separator}{}", row.repeat(8)),
        ));
    }
    println!("{{\"options\":\"ParseOptions::default; ANSI defaults\",\"metrics\":[");
    let mut first = true;
    for (case, input) in &cases {
        for api in ["html", "ast", "ansi"] {
            if !first {
                println!(",");
            }
            first = false;
            match api {
                "html" => measure(case, api, input.len(), || render_html(input, &options)),
                "ast" => measure(case, api, input.len(), || parse_markdown(input, &options)),
                "ansi" => measure(case, api, input.len(), || {
                    render_ansi_terminal(input, &options, None)
                }),
                _ => unreachable!(),
            }
        }
    }
    println!("]}}");
}
