import type { Plugin } from "vite";
import type { MdxPluginOptions } from "./index.js";

export type { MdxPluginOptions };

/** Vite plugin: import `.md` and `.mdx` files as components. */
export default function ironmarkMdx(options?: MdxPluginOptions): Plugin;
