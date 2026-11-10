import { readFile } from "node:fs/promises";
import { dirname } from "node:path";
import { createMdxCompiler } from "./core.js";

/**
 * esbuild plugin: import `.mdx` files as components. The same object works as a
 * `Bun.build()` / `Bun.plugin()` plugin, and in tools built on esbuild (tsup).
 *
 * @param {import("./index.js").MdxPluginOptions} [options]
 * @returns {import("esbuild").Plugin}
 */
export default function ironmarkMdx(options) {
  const mdx = createMdxCompiler(options);
  return {
    name: "ironmark:mdx",
    setup(build) {
      build.onLoad({ filter: mdx.include }, async ({ path }) => {
        const source = await readFile(path, "utf8");
        try {
          // The JSX pragmas in the output select the automatic runtime and import source.
          return { contents: mdx.compile(source, path), loader: "jsx", resolveDir: dirname(path) };
        } catch (error) {
          return {
            errors: [{ text: error.cause?.message ?? error.message, location: { file: path } }],
          };
        }
      });
    },
  };
}
