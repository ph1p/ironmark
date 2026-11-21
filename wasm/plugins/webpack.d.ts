import type { MdxPluginOptions, PluginOptions } from "./index.js";

export type { MdxPluginOptions, PluginOptions };

/** webpack and Rspack loader: compile `.md` and `.mdx` files to JSX modules. */
export default function ironmarkLoader(
  this: { getOptions(): PluginOptions; query: unknown; resourcePath: string },
  source: string,
): string;
