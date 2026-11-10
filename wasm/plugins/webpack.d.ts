import type { MdxPluginOptions } from "./index.js";

export type { MdxPluginOptions };

/** webpack and Rspack loader: compile `.mdx` files to JSX modules. */
export default function ironmarkMdxLoader(
  this: { getOptions(): MdxPluginOptions; query: unknown; resourcePath: string },
  source: string,
): string;
