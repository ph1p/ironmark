import type { AstroIntegration } from "astro";
import type { MdxPluginOptions } from "./index.js";

export type { MdxPluginOptions };

/** Astro integration: import `.mdx` files as React or Preact components. */
export default function ironmarkAstro(options?: MdxPluginOptions): AstroIntegration;
