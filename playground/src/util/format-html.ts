const VOID = new Set([
  "area",
  "base",
  "br",
  "col",
  "embed",
  "hr",
  "img",
  "input",
  "link",
  "meta",
  "param",
  "source",
  "track",
  "wbr",
]);

const INDENT: string[] = [];
for (let i = 0; i < 32; i++) INDENT[i] = "  ".repeat(i);
const indent = (n: number) => (n < INDENT.length ? INDENT[n] : "  ".repeat(n));

const TAG_RE = /^<([a-zA-Z][a-zA-Z0-9]*)/;

export function formatHtml(html: string): string {
  const tokens: string[] = [];
  let i = 0;
  while (i < html.length) {
    if (html.charCodeAt(i) === 60) {
      const end = html.indexOf(">", i);
      if (end === -1) {
        tokens.push(html.slice(i));
        break;
      }
      tokens.push(html.slice(i, end + 1));
      i = end + 1;
    } else {
      const end = html.indexOf("<", i);
      const text = end === -1 ? html.slice(i) : html.slice(i, end);
      if (text.trim()) tokens.push(text);
      i = end === -1 ? html.length : end;
    }
  }

  const lines: string[] = [];
  let depth = 0;

  for (const token of tokens) {
    if (token.charCodeAt(0) === 60) {
      if (token.charCodeAt(1) === 47) {
        depth = Math.max(0, depth - 1);
        lines.push(indent(depth) + token);
      } else {
        lines.push(indent(depth) + token);
        const m = TAG_RE.exec(token);
        if (m && token.charCodeAt(token.length - 2) !== 47 && !VOID.has(m[1].toLowerCase())) {
          depth++;
        }
      }
    } else {
      lines.push(indent(depth) + token);
    }
  }

  return lines.join("\n");
}
