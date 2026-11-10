#!/usr/bin/env node
import { readFileSync } from "node:fs";
import {
  markdownToHtml,
  markdownToAst,
  markdownToAnsi,
  CAPABILITIES,
  DEFAULT_OPTIONS,
  PRESETS,
} from "./node.js";

const VERSION = JSON.parse(
  readFileSync(new URL("../package.json", import.meta.url), "utf8"),
).version;

const HELP = `ironmark — render Markdown in various output formats

USAGE:
    ironmark [--format <html|ansi|ast>] [OPTIONS] [FILE...]

    When no FILE is given, reads from stdin. Use '-' for stdin explicitly.
    Default format is html.

OPTIONS (all formats):
    --format <html|ansi|ast>    Output format; ast also accepts 'json' (default: html)
    --preset <name>             Apply a named option preset: default, safe, strict, llm
    --safe                      Alias for --preset safe (disable raw HTML, enable tag filter)
    --no-hard-breaks            Don't turn soft newlines into hard line breaks
    --no-tables                 Disable pipe table syntax
    --no-highlight              Disable ==highlight== syntax
    --no-strikethrough          Disable ~~strikethrough~~ syntax
    --no-underline              Disable ++underline++ syntax
    --no-autolink               Disable bare URL auto-linking
    --no-task-lists             Disable - [x] task list syntax
    --math                      Enable $inline$ and $$display$$ math
    --wiki-links                Enable [[wiki link]] syntax
    --capabilities              Print machine-readable capabilities JSON and exit
    --default-options           Print resolved default options JSON and exit
    --list-presets              Print all preset names and their options and exit
    -h, --help                  Print this help and exit
    -V, --version               Print version and exit

OPTIONS (ansi format only):
    --width N            Terminal column width (default: auto-detect, fallback 80)
    --no-color           Disable ANSI escape codes (plain text)
    -n, --line-numbers   Show line numbers in fenced code blocks
    --padding N          Horizontal padding on both sides of output (default: 0)

EXAMPLES:
    echo '# Hello' | npx ironmark
    echo '# Hello' | npx ironmark --format html
    npx ironmark --format ansi README.md
    npx ironmark --format ast README.md
    npx ironmark --format ansi --width 120 --no-color README.md | less
    cat doc.md | npx ironmark --format ansi --math --wiki-links
    npx ironmark --safe README.md
    npx ironmark --preset llm README.md
    npx ironmark --capabilities
`;

const args = process.argv.slice(2);
const files = [];
const parseOptions = {};
const ansiOptions = {};
let format = null;

for (let i = 0; i < args.length; i++) {
  switch (args[i]) {
    case "-h":
    case "--help":
      process.stdout.write(HELP);
      process.exit(0);
      break;
    case "-V":
    case "--version":
      console.log(`ironmark ${VERSION}`);
      process.exit(0);
      break;
    case "--capabilities":
      process.stdout.write(JSON.stringify(CAPABILITIES, null, 2) + "\n");
      process.exit(0);
      break;
    case "--default-options":
      process.stdout.write(JSON.stringify(DEFAULT_OPTIONS, null, 2) + "\n");
      process.exit(0);
      break;
    case "--list-presets":
      process.stdout.write(JSON.stringify(PRESETS, null, 2) + "\n");
      process.exit(0);
      break;
    case "--format": {
      const val = args[++i];
      if (val === undefined) {
        console.error("error: --format requires a value");
        process.exit(2);
      }
      format = val;
      break;
    }
    case "--preset": {
      const val = args[++i];
      if (val === undefined) {
        console.error("error: --preset requires a value (default, safe, strict, llm)");
        process.exit(2);
      }
      parseOptions.preset = val;
      break;
    }
    case "--safe":
      parseOptions.safe = true;
      break;
    case "--no-color":
    case "--no-colour":
      ansiOptions.color = false;
      break;
    case "-n":
    case "--line-numbers":
      ansiOptions.lineNumbers = true;
      break;
    case "--width": {
      const val = args[++i];
      if (val === undefined || Number.isNaN(Number(val))) {
        console.error("error: --width requires a numeric value");
        process.exit(2);
      }
      ansiOptions.width = Number(val);
      break;
    }
    case "--padding": {
      const val = args[++i];
      if (val === undefined || Number.isNaN(Number(val))) {
        console.error("error: --padding requires a numeric value");
        process.exit(2);
      }
      ansiOptions.padding = Number(val);
      break;
    }
    case "--no-hard-breaks":
      parseOptions.hardBreaks = false;
      break;
    case "--no-tables":
      parseOptions.enableTables = false;
      break;
    case "--no-highlight":
      parseOptions.enableHighlight = false;
      break;
    case "--no-strikethrough":
      parseOptions.enableStrikethrough = false;
      break;
    case "--no-underline":
      parseOptions.enableUnderline = false;
      break;
    case "--no-autolink":
      parseOptions.enableAutolink = false;
      break;
    case "--no-task-lists":
      parseOptions.enableTaskLists = false;
      break;
    case "--math":
      parseOptions.enableLatexMath = true;
      break;
    case "--wiki-links":
      parseOptions.enableWikiLinks = true;
      break;
    default:
      if (args[i].startsWith("--format=")) {
        format = args[i].slice("--format=".length);
      } else if (args[i].startsWith("--width=")) {
        const val = Number(args[i].slice("--width=".length));
        if (Number.isNaN(val)) {
          console.error("error: --width requires a numeric value");
          process.exit(2);
        }
        ansiOptions.width = val;
      } else if (args[i].startsWith("--padding=")) {
        const val = Number(args[i].slice("--padding=".length));
        if (Number.isNaN(val)) {
          console.error("error: --padding requires a numeric value");
          process.exit(2);
        }
        ansiOptions.padding = val;
      } else if (args[i].startsWith("-") && args[i] !== "-") {
        console.error(`error: unknown flag: ${args[i]}`);
        console.error("Run 'ironmark --help' for usage.");
        process.exit(2);
      } else {
        files.push(args[i]);
      }
  }
}

const fmt = format ?? "html";

if (fmt !== "html" && fmt !== "ansi" && fmt !== "ast" && fmt !== "json") {
  console.error(`error: unknown format '${fmt}' — use html, ansi, or ast`);
  process.exit(2);
}

if (files.length === 0 && process.stdin.isTTY) {
  console.error("error: no input provided");
  console.error("Run 'ironmark --help' for usage.");
  process.exit(2);
}

// Auto-detect terminal width for ansi format
if (ansiOptions.width === undefined) {
  ansiOptions.width = process.stdout.columns || 80;
}

let input = "";

if (files.length === 0) {
  input = readFileSync(0, "utf8");
} else {
  for (const file of files) {
    if (file === "-") {
      input += readFileSync(0, "utf8");
    } else {
      try {
        input += readFileSync(file, "utf8");
      } catch (err) {
        console.error(`error: ${file}: ${err.message}`);
        process.exit(1);
      }
    }
  }
}

if (fmt === "html") {
  process.stdout.write(markdownToHtml(input, parseOptions));
} else if (fmt === "ansi") {
  process.stdout.write(markdownToAnsi(input, { ...parseOptions, ...ansiOptions }));
} else {
  process.stdout.write(JSON.stringify(markdownToAst(input, parseOptions), null, 2) + "\n");
}
