import type { MdxPluginOptions, PluginOptions } from "./index.js";

export type { MdxPluginOptions, PluginOptions };

/** Rollup and Rolldown plugin: import `.md` and `.mdx` files as components. */
export default function ironmark(options?: PluginOptions): {
  name: string;
  options(input: object): object | null;
  transform(source: string, id: string): { code: string; map: null; moduleType: "jsx" } | null;
};
