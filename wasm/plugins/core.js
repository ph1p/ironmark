// Shared Markdown/MDX compiler behind every build-tool plugin (`ironmark/vite`, `/esbuild`, `/rollup`,
// `/webpack`, `/astro`). Plugins run in Node or Bun, so this loads the synchronous,
// MDX-only binary from `ironmark/mdx`.
import ironmark from "../mdx-node.js";

const MDX_FILE = /\.mdx$/i;
const ASSET_QUERY = /(?:^|&)(?:raw|url|inline|worker|sharedworker)(?:&|=|$)/;

/**
 * Normalize plugin options once and return a compiler for `.md` and `.mdx` sources.
 * `.mdx` files compile as MDX; every other matched file compiles as plain Markdown.
 *
 * @param {import("./index.js").MdxPluginOptions} [options]
 */
export function createMdxCompiler(options = {}) {
  const {
    include = /\.mdx?$/,
    jsxImportSource = "react",
    components = true,
    providerImportSource,
    frontmatter,
    ...parseOptions
  } = options;
  // Front matter is always stripped: in MDX a leading `---` would otherwise be a thematic break.
  const md = ironmark.with({ enableFrontmatter: true, ...parseOptions });
  const render = providerImportSource ? { components, providerImportSource } : { components };
  const markdownRender = { ...render, markdown: true };
  // Pragmas let any downstream JSX transform (esbuild, Bun, Oxc, SWC, Babel) pick the
  // automatic runtime without per-tool configuration.
  const pragma = `/** @jsxRuntime automatic */\n/** @jsxImportSource ${jsxImportSource} */\n`;

  return {
    include,
    jsxImportSource,
    /** Whether `id` (a path, possibly with a `?query`) is a file this compiler handles. */
    test(id) {
      if (id.startsWith("\0")) return false;
      const [path, query] = id.split("?");
      // `?raw`, `?url`, … imports want the file itself, not a component.
      if (query && ASSET_QUERY.test(query)) return false;
      include.lastIndex = 0;
      return include.test(path);
    },
    /**
     * Compile one source to an ES module that still contains JSX. Plain Markdown modules
     * also export `html`, `headings` and `frontmatter`.
     *
     * @param {string} source
     * @param {string} file - Picks MDX or Markdown, and is used in error messages and passed to `frontmatter`.
     * @returns {string}
     */
    compile(source, file) {
      const mdx = MDX_FILE.test(file.split("?")[0]);
      const doc = md.parse(source);
      let code;
      try {
        code = pragma + doc.jsx(mdx ? render : markdownRender).module;
      } catch (error) {
        throw new Error(`${file}: ${error.message}`, { cause: error });
      }
      const raw = frontmatter || !mdx ? doc.frontmatter() : null;
      if (frontmatter) {
        const value = raw === null ? {} : frontmatter(raw, file);
        code += `\nexport const frontmatter = ${JSON.stringify(value) ?? "undefined"};\n`;
      } else if (!mdx) {
        code += `\nexport const frontmatter = ${JSON.stringify(raw)};\n`;
      }
      if (!mdx) {
        code += `export const html = ${JSON.stringify(doc.html())};\n`;
        code += `export const headings = ${JSON.stringify(doc.headings())};\n`;
      }
      return code;
    },
  };
}
