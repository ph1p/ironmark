import assert from "node:assert/strict";
import test from "node:test";
import { renderHtml } from "./node.js";

const LIMIT = 10 * 1024 * 1024;
const FENCE = "ordinary paragraph\n\n```\n";
const encoder = new TextEncoder();

function inputForms(bytes) {
  const padded = new Uint8Array(bytes.length + 8);
  padded.fill(0x21);
  padded.set(bytes, 3);
  return [
    ["Uint8Array", bytes],
    ["Buffer", Buffer.from(bytes.buffer, bytes.byteOffset, bytes.byteLength)],
    ["ArrayBuffer", bytes.slice().buffer],
    ["sliced Uint8Array", padded.subarray(3, 3 + bytes.length)],
    ["sliced Buffer", Buffer.from(padded.buffer).subarray(3, 3 + bytes.length)],
    ["DataView", new DataView(padded.buffer, 3, bytes.length)],
  ];
}

function assertVariants(input, expected, label, large = false) {
  for (const [optionsLabel, run] of [
    ["omitted", () => renderHtml(input)],
    ["empty", () => renderHtml(input, {})],
    ["default preset", () => renderHtml(input, { preset: "default" })],
    ["null", () => renderHtml(input, null)],
  ]) {
    const actual = run();
    if (large) {
      // Exact comparison without a multi-megabyte assertion diff on failure.
      assert.ok(
        actual === expected,
        `${label}/${optionsLabel}: HTML mismatch (actual ${actual.length}, expected ${expected.length} characters)`,
      );
    } else {
      assert.equal(actual, expected, `${label}/${optionsLabel}`);
    }
  }
}

test("ordinary input, escaping, and default extensions have exact HTML on all paths", () => {
  for (const [input, expected] of [
    ["", ""],
    ["ordinary paragraph", "<p>ordinary paragraph</p>\n"],
    ["A & B < C > D", "<p>A &amp; B &lt; C &gt; D</p>\n"],
    [
      "**bold** *em* ~~del~~ ==mark== ++u++ `code`\nnext",
      "<p><strong>bold</strong> <em>em</em> <del>del</del> <mark>mark</mark> <u>u</u> <code>code</code><br />\nnext</p>\n",
    ],
    ["```\nx", "<pre><code>x\n</code></pre>\n"],
  ]) {
    assertVariants(input, expected, "string");
    for (const [label, bytes] of inputForms(encoder.encode(input))) {
      assertVariants(bytes, expected, label);
    }
  }
});

test("malformed UTF-8 replacement and leading byte BOM handling stay unchanged", () => {
  for (const [bytes, expected] of [
    [new Uint8Array([0x61, 0xff, 0x62, 0xc3]), "<p>a\uFFFDb\uFFFD</p>\n"],
    [new Uint8Array([0xef, 0xbb, 0xbf, 0x61, 0x26, 0x62]), "<p>a&amp;b</p>\n"],
    [new Uint8Array([0xef, 0xbb, 0xbf]), ""],
  ]) {
    for (const [label, input] of inputForms(bytes)) {
      assertVariants(input, expected, label);
    }
  }
});

function fencedInput(bytes) {
  return FENCE + "x".repeat(bytes - FENCE.length);
}

function expectedFenced(prefix) {
  assert.ok(prefix.startsWith(FENCE));
  return `<p>ordinary paragraph</p>\n<pre><code>${prefix.slice(FENCE.length)}\n</code></pre>\n`;
}

test("exact limit and one byte below/above use the same truncation policy", () => {
  for (const size of [LIMIT - 1, LIMIT, LIMIT + 1]) {
    const input = fencedInput(size);
    const prefix = input.slice(0, LIMIT); // ASCII: bytes equal characters.
    const expected = expectedFenced(prefix);
    assertVariants(input, expected, `string/${size}`, true);
    for (const [label, bytes] of inputForms(encoder.encode(input))) {
      assertVariants(bytes, expected, `${label}/${size}`, true);
    }
  }
});

test("oversized suffix is discarded and multibyte boundary truncates whole characters", () => {
  for (const [input, prefix] of [
    [fencedInput(LIMIT) + "SUFFIX_BEYOND_WASM_LIMIT", fencedInput(LIMIT)],
    [fencedInput(LIMIT - 1) + "éSUFFIX", fencedInput(LIMIT - 1)],
    [fencedInput(LIMIT - 4) + "😀SUFFIX", fencedInput(LIMIT - 4) + "😀"],
  ]) {
    const expected = expectedFenced(prefix);
    assert.ok(!expected.includes("SUFFIX"));
    assertVariants(input, expected, "string boundary", true);
    for (const [label, bytes] of inputForms(encoder.encode(input))) {
      assertVariants(bytes, expected, `${label} boundary`, true);
    }
  }
});

test("limit applies to decoded UTF-8 bytes, preserving malformed-input semantics", () => {
  const prefix = fencedInput(LIMIT - 2);
  const bytes = new Uint8Array(LIMIT);
  bytes.set(encoder.encode(prefix));
  bytes.set([0xff, 0x61], LIMIT - 2);
  // U+FFFD takes three decoded UTF-8 bytes and straddles the remaining two.
  const expected = expectedFenced(prefix);
  for (const [label, input] of inputForms(bytes)) {
    assertVariants(input, expected, label, true);
  }
});
