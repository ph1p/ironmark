import { createMdxCompiler } from "./core.js";

// One compiler per rule: `this.query` is the rule's options object, or its `?query` string.
const byOptions = new WeakMap();
const byQuery = new Map();

/**
 * webpack (5) and Rspack loader: compile `.mdx` files to modules that still contain JSX.
 * Chain a JSX transform after it (`builtin:swc-loader`, `swc-loader`, `babel-loader`,
 * `esbuild-loader`); the output's pragmas select the automatic runtime.
 *
 * @this {{ getOptions(): import("./index.js").MdxPluginOptions; query: string | object; resourcePath: string }}
 * @param {string} source
 */
export default function ironmarkMdxLoader(source) {
  const cache = typeof this.query === "string" ? byQuery : byOptions;
  let mdx = cache.get(this.query);
  if (!mdx) {
    mdx = createMdxCompiler(this.getOptions());
    cache.set(this.query, mdx);
  }
  return mdx.compile(source, this.resourcePath);
}
