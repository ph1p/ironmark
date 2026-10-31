import assert from "node:assert/strict";
import { mkdirSync, mkdtempSync, readdirSync, rmSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import test from "node:test";
import { brotliCompressSync, constants, gzipSync } from "node:zlib";
import { ROOT } from "./generators.mjs";
import {
  artifactSizes,
  entryUrl,
  measure,
  packageSizes,
  parseArgs,
  parseChild,
  parsePack,
  runCommand,
  startup,
  summarize,
} from "./measure.mjs";

export function fixture(t) {
  mkdirSync(join(ROOT, "target"), { recursive: true });
  const dir = mkdtempSync(join(ROOT, "target/measurement-test-"));
  t.after(() => rmSync(dir, { recursive: true, force: true }));
  return dir;
}

test("artifact sizes match Node compression of tiny arbitrary bytes", (t) => {
  const path = join(fixture(t), "tiny.wasm");
  const bytes = Buffer.from([0, 97, 115, 109, 1, 0, 0, 0, 255, 0]);
  writeFileSync(path, bytes);
  const result = artifactSizes(path);
  assert.equal(result.raw_bytes, bytes.length);
  assert.equal(result.gzip_bytes, gzipSync(bytes, { level: 9 }).length);
  assert.equal(
    result.brotli_bytes,
    brotliCompressSync(bytes, {
      params: { [constants.BROTLI_PARAM_QUALITY]: 11 },
    }).length,
  );
  const alternate = measure(["--artifact", path]);
  assert.equal(alternate.mode, "artifact-only");
  assert.equal(alternate.package, undefined);
  assert.equal(alternate.startup, undefined);
  assert.equal(alternate.wasm.path, path);
});

test("missing artifacts and entries fail clearly without building", (t) => {
  const missing = join(fixture(t), "missing");
  assert.throws(() => artifactSizes(missing), /Missing or unreadable artifact/);
  assert.throws(() => entryUrl(missing), /Missing entry/);
  assert.throws(() => startup(missing), /Missing entry/);
});

test("statistics validate inputs and interpolate sorted quartiles", () => {
  assert.deepEqual(summarize([4, 1, 3, 2]), {
    samples: 4,
    median_ms: 2.5,
    p25_ms: 1.75,
    p75_ms: 3.25,
  });
  assert.equal(summarize([0]).median_ms, 0);
  for (const samples of [[], [NaN], [Infinity], [-1], null]) {
    assert.throws(() => summarize(samples), /nonempty finite nonnegative/);
  }
});

test("malformed child and npm output is rejected, never zero-filled", () => {
  for (const text of ["noise", "", "{}{}", "null", "{}", '{"import_ms":-1}', "[]"]) {
    assert.throws(() => parseChild(text), /Malformed/);
  }
  assert.deepEqual(parseChild('{"import_ms":1,"first_render_ms":0}'), {
    import_ms: 1,
    first_render_ms: 0,
  });
  for (const text of [
    "npm noise",
    "[]",
    "null",
    '[{"filename":"../oops.tgz","files":[{"size":1}]}]',
    '[{"filename":"oops.tgz","files":[{"size":-1}]}]',
    '[{"filename":"oops.tgz","files":[]}]',
    '[{"filename":"oops.tgz","files":[{"size":1.5}]}]',
  ]) {
    assert.throws(() => parsePack(text), /Malformed/);
  }
});

test("pack uses exact script-free commands and cleans owned paths on failure", (t) => {
  const root = fixture(t);
  const seen = [];
  assert.throws(
    () =>
      packageSizes(root, (command, args, options) => {
        seen.push({ command, args, options });
        return "not JSON";
      }),
    /Malformed npm pack JSON/,
  );
  assert.equal(seen[0].command, "npm");
  assert.deepEqual(seen[0].args, ["pack", "--dry-run", "--ignore-scripts", "--json"]);
  assert.equal(seen[0].options.cwd, root);
  assert.equal(seen[0].options.env.npm_config_cache, join(root, "target/npm-cache"));
  assert.deepEqual(readdirSync(join(root, "target")), []);
});

test("real npm pack measures tarball and unpacked sum without running prepack", (t) => {
  const root = fixture(t);
  writeFileSync(
    join(root, "package.json"),
    JSON.stringify({
      name: "ironmark-measurement-fixture",
      version: "1.0.0",
      files: ["index.mjs"],
      scripts: { prepack: "node -e 'process.exit(42)'" },
    }),
  );
  writeFileSync(join(root, "index.mjs"), "export const answer = 42;\n");
  const calls = [];
  const result = packageSizes(root, (cmd, args, opts) => {
    calls.push(args);
    return runCommand(cmd, args, opts);
  });
  assert.equal(calls.length, 2);
  assert.deepEqual(calls[1].slice(0, 4), [
    "pack",
    "--ignore-scripts",
    "--json",
    "--pack-destination",
  ]);
  assert.ok(result.tarball_bytes > 0);
  assert.ok(result.unpacked_bytes > 25);
  assert.equal(result.files, 2);
  assert.ok(!readdirSync(join(root, "target")).some((name) => name.startsWith("measure-pack-")));
});

test("startup uses twenty genuinely independent processes and selected entry", (t) => {
  const entry = join(fixture(t), "entry.mjs");
  // Any reuse within a child would make its render fail; each import starts fresh.
  writeFileSync(
    entry,
    "let calls = 0; export function renderHtml() { if (++calls !== 1) throw Error('reused'); return '<p>fixture</p>'; }\n",
  );
  const argsSeen = [];
  const result = startup(entry, 20, (cmd, args, opts) => {
    argsSeen.push(args);
    return runCommand(cmd, args, opts);
  });
  assert.equal(result.entry, entryUrl(entry));
  assert.equal(argsSeen.length, 20);
  assert.ok(argsSeen.every((args) => args.at(-1) === entryUrl(entry)));
  for (const name of ["import", "first_render", "process_wall"]) {
    assert.equal(result[name].samples, 20);
    assert.ok(result[name].median_ms >= 0);
  }
  assert.throws(() => startup(entry, 19), /At least 20/);
  assert.throws(() => startup(entry, 20, () => "bad output"), /Malformed startup child JSON/);
});

test("entry/argument errors and failed children are surfaced", () => {
  assert.deepEqual(parseArgs(["--entry", "alternate.mjs"], ["--entry"]), {
    "--entry": "alternate.mjs",
  });
  for (const args of [["--entry"], ["--unknown", "x"], ["--entry", "x", "--entry", "y"]]) {
    assert.throws(() => parseArgs(args, ["--entry"]), /Expected/);
  }
  assert.throws(() => measure(["--artifact", "x", "--entry", "y"]), /--artifact is size-only/);
  assert.throws(() => runCommand(process.execPath, ["-e", "process.exit(7)"]), /failed/);
});
