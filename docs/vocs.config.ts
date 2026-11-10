import { defineConfig } from "vocs/config";

// Served from GitHub Pages at https://ph1p.js.org/ironmark/, with the playground
// built into /ironmark/playground/ by .github/workflows/pages.yml.
const PLAYGROUND = "https://ph1p.js.org/ironmark/playground/";
const REPO = "https://github.com/ph1p/ironmark";

export default defineConfig({
  title: "ironmark",
  description:
    "Fast, CommonMark-compliant Markdown parser in Rust: HTML, text, AST, ANSI, Markdown and MDX, for Rust and JavaScript via WebAssembly.",
  basePath: "/ironmark",
  baseUrl: "https://ph1p.js.org",
  renderStrategy: "full-static",
  editLink: {
    // Vocs serializes this function for the client, so it cannot use module-level constants.
    link: (filePath) => `https://github.com/ph1p/ironmark/edit/main/docs/src/pages/${filePath}`,
    text: "Edit this page on GitHub",
  },
  socials: [{ icon: "github", link: REPO }],
  topNav: [
    { text: "Guide", link: "/getting-started" },
    { text: "Rust", link: "/rust" },
    { text: "Playground", link: PLAYGROUND, external: true },
    { text: "Benchmarks", link: `${PLAYGROUND}#benchmark`, external: true },
  ],
  sidebar: [
    {
      text: "Introduction",
      items: [
        { text: "Getting started", link: "/getting-started" },
        { text: "Which function?", link: "/which-function" },
        { text: "Playground", link: PLAYGROUND },
      ],
    },
    {
      text: "JavaScript",
      items: [
        { text: "Documents", link: "/js/documents" },
        { text: "HTML", link: "/js/html" },
        { text: "Plain text", link: "/js/text" },
        { text: "AST", link: "/js/ast" },
        { text: "Terminal (ANSI)", link: "/js/ansi" },
        { text: "HTML to Markdown", link: "/js/html-to-markdown" },
        { text: "Browser and bundlers", link: "/js/browser" },
        { text: "Subpath imports", link: "/js/subpaths" },
        { text: "AI pipelines", link: "/js/ai" },
        { text: "Constants and utilities", link: "/js/constants" },
        { text: "Migrating from 2.x", link: "/js/migration" },
      ],
    },
    {
      text: "Configuration",
      items: [
        { text: "Options", link: "/config/options" },
        { text: "Presets", link: "/config/presets" },
        { text: "Security", link: "/config/security" },
      ],
    },
    {
      text: "MDX",
      items: [
        { text: "Overview", link: "/mdx" },
        { text: "Syntax", link: "/mdx/syntax" },
        { text: "Components", link: "/mdx/components" },
        { text: "Other renderers", link: "/mdx/renderers" },
      ],
    },
    {
      text: "Build tool plugins",
      items: [
        { text: "Overview", link: "/plugins" },
        { text: "Vite", link: "/plugins/vite" },
        { text: "esbuild and Bun", link: "/plugins/esbuild" },
        { text: "Rollup and Rolldown", link: "/plugins/rollup" },
        { text: "webpack and Rspack", link: "/plugins/webpack" },
        { text: "Astro", link: "/plugins/astro" },
      ],
    },
    {
      text: "Rust",
      items: [
        { text: "Getting started", link: "/rust" },
        { text: "AST", link: "/rust/ast" },
        { text: "Text, headings and links", link: "/rust/text" },
        { text: "Terminal (ANSI)", link: "/rust/ansi" },
        { text: "HTML to Markdown", link: "/rust/html-to-markdown" },
        { text: "C and C++", link: "/rust/c" },
      ],
    },
    {
      text: "More",
      items: [
        { text: "CLI", link: "/cli" },
        { text: "Benchmarks", link: "/benchmarks" },
        { text: "Development", link: "/development" },
      ],
    },
  ],
});
