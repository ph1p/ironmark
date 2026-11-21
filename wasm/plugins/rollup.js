import { createCompiler } from "./core.js";

/**
 * Rollup (4.24 or newer) and Rolldown plugin: import `.md` and `.mdx` files as components.
 *
 * Rolldown transforms the JSX itself. Under Rollup the plugin turns on Rollup's own JSX
 * transform (`jsx: { mode: "automatic" }`) unless the config already sets `jsx`.
 *
 * @param {import("./index.js").PluginOptions} [options]
 */
export default function ironmark(options) {
  const mdx = createCompiler(options);
  return {
    name: "ironmark",
    options(input) {
      if (this.meta.rolldownVersion || input.jsx !== undefined) return null;
      const importSource = mdx.jsxImportSource;
      return {
        ...input,
        jsx: {
          mode: "automatic",
          factory: "React.createElement",
          importSource,
          jsxImportSource: `${importSource}/jsx-runtime`,
        },
      };
    },
    transform(source, id) {
      if (!mdx.test(id)) return null;
      try {
        return { code: mdx.compile(source, id.split("?")[0]), map: null, moduleType: "jsx" };
      } catch (error) {
        this.error(error.message);
      }
    },
  };
}
