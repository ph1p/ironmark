import { readFile } from "node:fs/promises";
import { dirname } from "node:path";
import { createCompiler } from "./core.js";

/**
 * esbuild plugin: import `.md` and `.mdx` files as components. The same object works as a
 * `Bun.build()` / `Bun.plugin()` plugin, and in tools built on esbuild (tsup).
 *
 * @param {import("./index.js").PluginOptions} [options]
 * @returns {import("esbuild").Plugin}
 */
export default function ironmark(options) {
  const mdx = createCompiler(options);
  return {
    name: "ironmark",
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
