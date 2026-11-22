// Seed each target's corpus from the CommonMark spec examples.
// Usage: node fuzz/seed.mjs   (writes fuzz/corpus/<target>/, gitignored)
import { mkdirSync, readFileSync, writeFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const here = dirname(fileURLToPath(import.meta.url));
const spec = JSON.parse(readFileSync(join(here, "../tests/spec/spec-0.31.2.json"), "utf8"));

// Option prefixes: ironmark defaults (+ indented code), all bits, and
// defaults with raw HTML disabled.
const OPTS = [
  [0x7f, 0x04, 0, 0],
  [0xff, 0xff, 0x0f, 0],
  [0xff, 0x04, 0, 0],
];
const targets = {
  render_html: (md, i) => [...OPTS[i % OPTS.length], ...md],
  ast_roundtrip: (md, i) => [...OPTS[i % OPTS.length], ...md],
  render_ansi: (md, i) => [i & 0xff, ...OPTS[i % OPTS.length], ...md],
  url_sanitize: (md, i) => [i & 0xff, ...md],
  html_to_markdown: (_md, i, html) => [i & 0xff, ...html],
};

for (const [name, make] of Object.entries(targets)) {
  const dir = join(here, "corpus", name);
  mkdirSync(dir, { recursive: true });
  spec.forEach((ex, i) => {
    const bytes = make(Buffer.from(ex.markdown), i, Buffer.from(ex.html));
    writeFileSync(join(dir, `spec-${ex.example}`), Buffer.from(bytes));
  });
}
console.log(`seeded ${Object.keys(targets).length} corpora with ${spec.length} examples each`);
