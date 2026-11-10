import type { MdxPluginOptions } from "./index.js";

export type { MdxPluginOptions };

/** Rollup and Rolldown plugin: import `.mdx` files as components. */
export default function ironmarkMdx(options?: MdxPluginOptions): {
  name: string;
  options(input: object): object | null;
  transform(source: string, id: string): { code: string; map: null; moduleType: "jsx" } | null;
};
