//! `ironmark` CLI — render Markdown in various formats.
//!
//! # Usage
//!
//! ```text
//! ironmark [--format <html|ansi|ast>] [OPTIONS] [FILE...]
//! ```
//!
//! When no `FILE` arguments are given, input is read from stdin.
//! Use `-` as a file path to explicitly read from stdin.
//! Default format is `html`.
//!
//! # Options
//!
//! | Flag | Description |
//! |---|---|
//! | `-f`, `--format <html\|ansi\|ast>` | Output format: html (default), ansi, ast (alias: json) |
//! | `-w`, `--width N` | Terminal column width for word-wrap (ansi only; default: auto-detect) |
//! | `--no-color` / `--no-colour` | Disable ANSI escape codes (ansi only) |
//! | `-n`, `--line-numbers` | Show line numbers in fenced code blocks (ansi only) |
//! | `-p`, `--padding N` | Horizontal padding (ansi only) |
//! | `--no-hard-breaks` | Don't convert newlines inside paragraphs to `<br>` |
//! | `--no-tables` | Disable pipe table syntax |
//! | `--no-highlight` | Disable `==highlight==` syntax |
//! | `--no-strikethrough` | Disable `~~strikethrough~~` syntax |
//! | `--no-underline` | Disable `++underline++` syntax |
//! | `--no-autolink` | Disable bare URL auto-linking |
//! | `--no-task-lists` | Disable `- [x]` task list syntax |
//! | `--math` | Enable `$inline$` and `$$display$$` math syntax |
//! | `--wiki-links` | Enable `[[wiki link]]` syntax |
//! | `--frontmatter` | Skip a leading `---`/`+++` front matter block |
//! | `--max-size N` | Truncate input to N bytes (0 = unlimited, default: 0) |
//! | `--help`, `-h` | Print this help and exit |
//! | `--version`, `-V` | Print version and exit |
//!
//! # Examples
//!
//! ```text
//! echo '# Hello' | ironmark
//! ironmark -f ansi README.md
//! ironmark -f ast README.md
//! ironmark -f ansi -w 80 README.md
//! cat doc.md | ironmark -f ansi --math --wiki-links
//! ```

use std::io::{self, IsTerminal, Read};

const VERSION: &str = env!("CARGO_PKG_VERSION");

const HELP: &str = "\
ironmark — render Markdown in various output formats

USAGE:
    ironmark [--format <html|ansi|ast>] [OPTIONS] [FILE...]

    When no FILE is given, reads from stdin. Use '-' for stdin explicitly.
    Default format is html.

OPTIONS (all formats):
    -f, --format <html|ansi|ast>  Output format; ast also accepts 'json' (default: html)
    --no-hard-breaks              Don't turn soft newlines into hard line breaks
    --no-tables                   Disable pipe table syntax
    --no-highlight                Disable ==highlight== syntax
    --no-strikethrough            Disable ~~strikethrough~~ syntax
    --no-underline                Disable ++underline++ syntax
    --no-autolink                 Disable bare URL auto-linking
    --no-task-lists               Disable - [x] task list syntax
    --math                        Enable $inline$ and $$display$$ math
    --wiki-links                  Enable [[wiki link]] syntax
    --frontmatter                 Skip a leading ---/+++ front matter block
    --max-size N                  Truncate input to N bytes (0 = unlimited)
    -h, --help                    Print this help and exit
    -V, --version                 Print version and exit

OPTIONS (ansi format only):
    -w, --width N        Terminal column width for word-wrap and heading underlines
                         (default: auto-detect via $COLUMNS / tput cols, fallback 80)
    --no-color           Disable ANSI escape codes (plain text)
    -n, --line-numbers   Show line numbers in fenced code blocks
    -p, --padding N      Add horizontal padding to output

EXAMPLES:
    echo '# Hello' | ironmark
    echo '# Hello' | ironmark --format html
    ironmark --format ansi README.md
    ironmark --format ast README.md
    ironmark --format ansi --width 120 --no-color README.md | less
    cat doc.md | ironmark --format ansi --math --wiki-links
";

fn parse_usize_arg(flag: &str, value: &str) -> usize {
    value.parse::<usize>().unwrap_or_else(|_| {
        eprintln!("error: {flag} value must be a non-negative integer");
        std::process::exit(2);
    })
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let mut files: Vec<String> = Vec::new();
    let mut format: Option<String> = None;
    let mut color = true;
    let mut line_numbers = false;
    let mut width: Option<usize> = None;
    let mut padding: usize = 0;
    let mut parse_opts = ironmark::ParseOptions::default();

    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "-h" | "--help" => {
                print!("{HELP}");
                return;
            }
            "-V" | "--version" => {
                println!("ironmark {VERSION}");
                return;
            }
            "-f" | "--format" => {
                i += 1;
                if i >= args.len() {
                    eprintln!("error: --format requires a value");
                    std::process::exit(2);
                }
                format = Some(args[i].clone());
            }
            "--no-color" | "--no-colour" => color = false,
            "--line-numbers" | "-n" => line_numbers = true,
            "--no-hard-breaks" => parse_opts.hard_breaks = false,
            "--no-tables" => parse_opts.enable_tables = false,
            "--no-highlight" => parse_opts.enable_highlight = false,
            "--no-strikethrough" => parse_opts.enable_strikethrough = false,
            "--no-underline" => parse_opts.enable_underline = false,
            "--no-autolink" => parse_opts.enable_autolink = false,
            "--no-task-lists" => parse_opts.enable_task_lists = false,
            "--math" => parse_opts.enable_latex_math = true,
            "--wiki-links" => parse_opts.enable_wiki_links = true,
            "--frontmatter" => parse_opts.enable_frontmatter = true,
            "-w" | "--width" => {
                i += 1;
                if i >= args.len() {
                    eprintln!("error: --width requires a value");
                    std::process::exit(2);
                }
                width = Some(parse_usize_arg("--width", &args[i]));
            }
            "-p" | "--padding" => {
                i += 1;
                if i >= args.len() {
                    eprintln!("error: --padding requires a value");
                    std::process::exit(2);
                }
                padding = parse_usize_arg("--padding", &args[i]);
            }
            "--max-size" => {
                i += 1;
                if i >= args.len() {
                    eprintln!("error: --max-size requires a value");
                    std::process::exit(2);
                }
                parse_opts.max_input_size = parse_usize_arg("--max-size", &args[i]);
            }
            arg if arg.starts_with("--format=") => {
                format = Some(arg["--format=".len()..].to_string());
            }
            arg if arg.starts_with("--width=") => {
                width = Some(parse_usize_arg("--width", &arg["--width=".len()..]));
            }
            arg if arg.starts_with("--padding=") => {
                padding = parse_usize_arg("--padding", &arg["--padding=".len()..]);
            }
            arg if arg.starts_with("--max-size=") => {
                parse_opts.max_input_size =
                    parse_usize_arg("--max-size", &arg["--max-size=".len()..]);
            }
            arg if arg.starts_with('-') && arg != "-" => {
                eprintln!("error: unknown flag: {arg}");
                eprintln!("Run 'ironmark --help' for usage.");
                std::process::exit(2);
            }
            path => files.push(path.to_string()),
        }
        i += 1;
    }

    let fmt = format.as_deref().unwrap_or("html");

    if files.is_empty() && io::stdin().is_terminal() {
        eprintln!("error: no input provided");
        eprintln!("Run 'ironmark --help' for usage.");
        std::process::exit(2);
    }

    // Resolve terminal width: explicit --width → $COLUMNS env var → tput cols → fallback 80
    let resolved_width = width.unwrap_or_else(|| {
        if let Some(w) = std::env::var("COLUMNS")
            .ok()
            .and_then(|v| v.trim().parse().ok())
        {
            return w;
        }
        if let Ok(out) = std::process::Command::new("tput").arg("cols").output()
            && out.status.success()
            && let Ok(s) = std::str::from_utf8(&out.stdout)
            && let Ok(n) = s.trim().parse::<usize>()
            && n > 0
        {
            return n;
        }
        80
    });

    let aopts = ironmark::AnsiOptions {
        width: resolved_width,
        color,
        line_numbers,
        padding,
    };

    let mut input = String::new();

    if files.is_empty() {
        io::stdin().read_to_string(&mut input).unwrap_or_else(|e| {
            eprintln!("error: failed to read stdin: {e}");
            std::process::exit(1);
        });
    } else {
        for path in &files {
            if path == "-" {
                io::stdin().read_to_string(&mut input).unwrap_or_else(|e| {
                    eprintln!("error: failed to read stdin: {e}");
                    std::process::exit(1);
                });
            } else {
                match std::fs::read_to_string(path) {
                    Ok(content) => input.push_str(&content),
                    Err(e) => {
                        eprintln!("error: {path}: {e}");
                        std::process::exit(1);
                    }
                }
            }
        }
    }

    match fmt {
        "html" => print!("{}", ironmark::render_html(&input, &parse_opts)),
        "ansi" => print!(
            "{}",
            ironmark::render_ansi_terminal(&input, &parse_opts, Some(&aopts))
        ),
        "ast" | "json" => {
            let ast = ironmark::parse_markdown(&input, &parse_opts);
            println!("{ast:#?}");
        }
        _ => {
            eprintln!("error: unknown format '{fmt}' — use html, ansi, or ast");
            std::process::exit(2);
        }
    }
}
