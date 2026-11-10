// Options shared by the build-tool plugins (`ironmark/vite`, `/esbuild`, `/rollup`, `/webpack`, `/astro`).
import type { JsxOptions } from "../index.js";

export interface MdxPluginOptions extends JsxOptions {
  /** Files to compile. Default: `/\.mdx$/`. esbuild reads only the pattern's source, not its flags. */
  include?: RegExp;
  /**
   * Package providing `jsx-runtime` for the compiled components (`"react"`, `"preact"`,
   * `"vue"`, `"hono/jsx"`, …). Default: `"react"`; `ironmark/astro` picks it from the
   * installed UI integration.
   */
  jsxImportSource?: string;
  /**
   * Route elements through `props.components`, like `@mdx-js/mdx`. Default: `true` in the
   * plugins (`mdxToJsx` itself defaults to `false`).
   */
  components?: boolean;
  /**
   * Parse the raw front matter (YAML or TOML text, without its fences) and export the
   * result as `frontmatter`. Files without front matter export `{}`. Front matter is
   * stripped from the content either way.
   *
   * @example frontmatter: (raw) => yaml.parse(raw)
   */
  frontmatter?: (raw: string, file: string) => unknown;
}
