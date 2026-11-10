import type { Plugin } from "esbuild";
import type { MdxPluginOptions } from "./index.js";

export type { MdxPluginOptions };

/** esbuild (and `Bun.build()`) plugin: import `.md` and `.mdx` files as components. */
export default function ironmarkMdx(options?: MdxPluginOptions): Plugin;
