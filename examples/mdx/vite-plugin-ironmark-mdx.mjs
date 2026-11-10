import { mdxToJsx } from "ironmark";
import { transformWithOxc } from "vite";

/**
 * Compile `.mdx` imports with ironmark.
 *
 * @param {import("ironmark").JsxOptions} [options] - Parse and JSX options.
 *   Defaults to `{ components: true }`; pass `providerImportSource: "@mdx-js/react"`
 *   to read components from an `MDXProvider` as well.
 * @returns {import("vite").Plugin}
 */
export default function ironmarkMdx(options = { components: true }) {
  return {
    name: "ironmark-mdx",
    enforce: "pre",
    async transform(code, id) {
      const [path] = id.split("?");
      if (!path.endsWith(".mdx")) return null;
      let module;
      try {
        ({ module } = mdxToJsx(code, options));
      } catch (err) {
        this.error({ id: path, message: err.message });
      }
      return transformWithOxc(module, `${path}.jsx`, {
        lang: "jsx",
        jsx: { runtime: "automatic" },
      });
    },
  };
}
