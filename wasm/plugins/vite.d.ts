import type { Plugin } from "vite";
import type { MdxPluginOptions, PluginOptions } from "./index.js";

export type { MdxPluginOptions, PluginOptions };

/** Vite plugin: import `.md` and `.mdx` files as components. */
export default function ironmark(options?: PluginOptions): Plugin;
