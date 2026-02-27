import { oneDarkHighlightStyle } from "@codemirror/theme-one-dark";
import { defaultHighlightStyle } from "@codemirror/language";
import { highlightCode } from "@lezer/highlight";
import type { LanguageSupport, Language } from "@codemirror/language";
import { isDark } from "./theme";

const js = () => import("@codemirror/lang-javascript");

const langLoaders: Record<string, () => Promise<LanguageSupport>> = {
  javascript: () => js().then((m) => m.javascript()),
  js: () => js().then((m) => m.javascript()),
  typescript: () => js().then((m) => m.javascript({ typescript: true })),
  ts: () => js().then((m) => m.javascript({ typescript: true })),
  jsx: () => js().then((m) => m.javascript({ jsx: true })),
  tsx: () => js().then((m) => m.javascript({ jsx: true, typescript: true })),
  rust: () => import("@codemirror/lang-rust").then((m) => m.rust()),
  html: () => import("@codemirror/lang-html").then((m) => m.html()),
  css: () => import("@codemirror/lang-css").then((m) => m.css()),
  json: () => import("@codemirror/lang-json").then((m) => m.json()),
  python: () => import("@codemirror/lang-python").then((m) => m.python()),
  py: () => import("@codemirror/lang-python").then((m) => m.python()),
  yaml: () => import("@codemirror/lang-yaml").then((m) => m.yaml()),
  yml: () => import("@codemirror/lang-yaml").then((m) => m.yaml()),
  markdown: () => import("@codemirror/lang-markdown").then((m) => m.markdown()),
  md: () => import("@codemirror/lang-markdown").then((m) => m.markdown()),
};

const langCache = new Map<string, Language>();
const langLoadingCache = new Map<string, Promise<Language>>();

function ensureLanguage(name: string): Promise<Language> | undefined {
  if (langCache.has(name)) return undefined;
  let pending = langLoadingCache.get(name);
  if (pending) return pending;
  const loader = langLoaders[name];
  if (!loader) return undefined;
  pending = loader().then((support) => {
    langCache.set(name, support.language);
    langLoadingCache.delete(name);
    return support.language;
  });
  langLoadingCache.set(name, pending);
  return pending;
}

const escapeRe = /[&<>]/g;
const escapeMap: Record<string, string> = { "&": "&amp;", "<": "&lt;", ">": "&gt;" };
const escapeHtml = (s: string) => s.replace(escapeRe, (ch) => escapeMap[ch]);

function highlightCodeString(code: string, lang: string): string {
  const language = langCache.get(lang);
  if (!language) return escapeHtml(code);

  const tree = language.parser.parse(code);
  const parts: string[] = [];
  let pos = 0;
  const style = isDark() ? oneDarkHighlightStyle : defaultHighlightStyle;

  highlightCode(
    code,
    tree,
    style,
    (text, classes) => {
      const escaped = escapeHtml(text);
      parts.push(classes ? `<span class="${classes}">${escaped}</span>` : escaped);
      pos += text.length;
    },
    () => {
      parts.push("\n");
      pos++;
    },
  );

  if (pos < code.length) parts.push(escapeHtml(code.slice(pos)));
  return parts.join("");
}

function highlightBlock(block: HTMLElement, lang: string) {
  const code = block.textContent || "";
  if (code) block.innerHTML = highlightCodeString(code, lang);
}

export function highlightCodeBlocks(container: HTMLElement) {
  const blocks = container.querySelectorAll("pre code[class*='language-']");
  const pending: Promise<void>[] = [];

  for (let i = 0; i < blocks.length; i++) {
    const block = blocks[i] as HTMLElement;
    const match = block.className.match(/language-(\S+)/);
    if (!match || !langLoaders[match[1]]) continue;
    const lang = match[1];

    if (langCache.has(lang)) {
      highlightBlock(block, lang);
    } else {
      const promise = ensureLanguage(lang);
      if (promise) pending.push(promise.then(() => highlightBlock(block, lang)));
    }
  }

  if (pending.length) Promise.all(pending);
}
