import type { AstroIntegration } from "astro";
import type { MdxPluginOptions, PluginOptions } from "./index.js";

export type { MdxPluginOptions, PluginOptions };

/** Astro integration: import `.mdx` files as React or Preact components. */
export default function ironmark(options?: PluginOptions): AstroIntegration;
