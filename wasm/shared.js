const decoder = new TextDecoder("utf-8");

function toStr(markdown) {
  if (typeof markdown === "string") return markdown;
  if (markdown instanceof Uint8Array) return decoder.decode(markdown);
  if (markdown instanceof ArrayBuffer) return decoder.decode(new Uint8Array(markdown));
  if (ArrayBuffer.isView(markdown))
    return decoder.decode(
      new Uint8Array(markdown.buffer, markdown.byteOffset, markdown.byteLength),
    );
  throw new TypeError("markdown must be a string, Uint8Array, ArrayBuffer, or Buffer");
}

function optionArgs(markdown, options) {
  return [
    toStr(markdown),
    options?.hardBreaks ?? undefined,
    options?.enableHighlight ?? undefined,
    options?.enableStrikethrough ?? undefined,
    options?.enableUnderline ?? undefined,
    options?.enableTables ?? undefined,
    options?.enableAutolink ?? undefined,
    options?.enableTaskLists ?? undefined,
  ];
}

export function createParse(wasmParse) {
  return function parse(markdown, options) {
    return wasmParse(...optionArgs(markdown, options));
  };
}

export function createParseToAst(wasmParseToAst) {
  return function parseToAst(markdown, options) {
    return wasmParseToAst(...optionArgs(markdown, options));
  };
}
