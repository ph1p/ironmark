// Shared MDX compiler behind every build-tool plugin (`ironmark/vite`, `/esbuild`, `/rollup`,
// `/webpack`, `/astro`). Plugins run in Node or Bun, so this loads the synchronous,
// MDX-only binary from `ironmark/mdx`.
import ironmark from "../mdx-node.js";

/**
 * Normalize plugin options once and return a compiler for `.mdx` sources.
 *
 * @param {import("./index.js").MdxPluginOptions} [options]
 */
export function createMdxCompiler(options = {}) {
  const {
    include = /\.mdx$/,
    jsxImportSource = "react",
    components = true,
    providerImportSource,
    frontmatter,
    ...parseOptions
  } = options;
  // Front matter is always stripped: in MDX a leading `---` would otherwise be a thematic break.
  const md = ironmark.with({ enableFrontmatter: true, ...parseOptions });
  const render = providerImportSource ? { components, providerImportSource } : { components };
  // Pragmas let any downstream JSX transform (esbuild, Bun, Oxc, SWC, Babel) pick the
  // automatic runtime without per-tool configuration.
  const pragma = `/** @jsxRuntime automatic */\n/** @jsxImportSource ${jsxImportSource} */\n`;

  return {
    include,
    jsxImportSource,
    /** Whether `id` (a path, possibly with a `?query`) is an MDX file this compiler handles. */
    test(id) {
      if (id.startsWith("\0")) return false;
      include.lastIndex = 0;
      return include.test(id.split("?")[0]);
    },
    /**
     * Compile one MDX source to an ES module that still contains JSX.
     *
     * @param {string} source
     * @param {string} file - Used in error messages and passed to `frontmatter`.
     * @returns {string}
     */
    compile(source, file) {
      const doc = md.parse(source);
      let code;
      try {
        code = pragma + doc.jsx(render).module;
      } catch (error) {
        throw new Error(`${file}: ${error.message}`, { cause: error });
      }
      if (frontmatter) {
        const raw = doc.frontmatter();
        const value = raw === null ? {} : frontmatter(raw, file);
        code += `\nexport const frontmatter = ${JSON.stringify(value) ?? "undefined"};\n`;
      }
      return code;
    },
  };
}
