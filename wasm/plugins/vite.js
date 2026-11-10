import { createRequire } from "node:module";
import { dirname, join } from "node:path";
import { pathToFileURL } from "node:url";
import { createMdxCompiler } from "./core.js";

/**
 * Vite plugin: import `.mdx` files as components. Also works in Astro, SvelteKit, Nuxt,
 * Storybook, Vitest and anything else built on Vite (5 or newer).
 *
 * @param {import("./index.js").MdxPluginOptions} [options]
 * @returns {import("vite").Plugin}
 */
export default function ironmarkMdx(options) {
  const mdx = createMdxCompiler(options);
  let root = process.cwd();
  let transformJsx;
  return {
    name: "ironmark:mdx",
    enforce: "pre",
    configResolved(config) {
      root = config.root;
    },
    async transform(source, id) {
      if (!mdx.test(id)) return null;
      const [file] = id.split("?");
      let code;
      try {
        code = mdx.compile(source, file);
      } catch (error) {
        this.error(error.message);
      }
      transformJsx ??= await loadJsxTransform(root, mdx.jsxImportSource);
      return transformJsx(code, `${file}.jsx`);
    },
  };
}

/**
 * Load the JSX transform of the Vite that runs this plugin: Oxc in Vite 8, esbuild before.
 * Rolldown-Vite's `moduleType: "jsx"` would avoid this, but its dev server ignores it.
 */
async function loadJsxTransform(root, importSource) {
  const vite = await importVite(root);
  if (vite.transformWithOxc) {
    return async (code, file) => {
      const { code: out, map } = await vite.transformWithOxc(code, file, {
        lang: "jsx",
        jsx: { runtime: "automatic", importSource },
      });
      return { code: out, map };
    };
  }
  return async (code, file) => {
    const { code: out, map } = await vite.transformWithEsbuild(code, file, {
      loader: "jsx",
      jsx: "automatic",
      jsxImportSource: importSource,
    });
    return { code: out, map };
  };
}

/** Resolve `vite` from the project first, so linked or hoisted ironmark installs still find it. */
async function importVite(root) {
  try {
    // `dist/node/index.js` is the ESM entry from Vite 5 to 8; `require` would pick index.cjs.
    const pkg = createRequire(join(root, "package.json")).resolve("vite/package.json");
    return await import(pathToFileURL(join(dirname(pkg), "dist/node/index.js")).href);
  } catch {
    return import("vite");
  }
}
