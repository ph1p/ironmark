import ironmarkMdx from "./vite.js";

/** UI integrations whose components `.mdx` files can compile to, by JSX import source. */
const RENDERERS = { "@astrojs/react": "react", "@astrojs/preact": "preact" };

/**
 * Astro integration: import `.mdx` files from `.astro` files as React (or Preact)
 * components. Needs `@astrojs/react` or `@astrojs/preact`; replaces `@astrojs/mdx`.
 *
 * @param {import("./index.js").MdxPluginOptions} [options]
 * @returns {import("astro").AstroIntegration}
 */
export default function ironmarkAstro(options = {}) {
  return {
    name: "ironmark",
    hooks: {
      "astro:config:setup": ({ config, updateConfig, logger }) => {
        const names = config.integrations.map((integration) => integration.name);
        if (names.includes("@astrojs/mdx")) {
          logger.warn("@astrojs/mdx also compiles .mdx files; remove one of the two integrations.");
        }
        let { jsxImportSource } = options;
        if (!jsxImportSource) {
          jsxImportSource = names.map((name) => RENDERERS[name]).find(Boolean);
          if (!jsxImportSource) {
            throw new Error(
              "ironmark/astro renders MDX as React or Preact components: add @astrojs/react or @astrojs/preact, or set jsxImportSource.",
            );
          }
        }
        updateConfig({ vite: { plugins: [ironmarkMdx({ ...options, jsxImportSource })] } });
      },
    },
  };
}
