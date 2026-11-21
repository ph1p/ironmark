import type { Plugin } from "esbuild";
import type { MdxPluginOptions, PluginOptions } from "./index.js";

export type { MdxPluginOptions, PluginOptions };

/** esbuild (and `Bun.build()`) plugin: import `.md` and `.mdx` files as components. */
export default function ironmark(options?: PluginOptions): Plugin;
