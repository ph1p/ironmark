# MDX example

`src/Guide.mdx` uses the main MDX features: an imported component (`Badge`), a component
supplied by the host app (`Note`), `{expressions}`, exports, and GFM tables and task lists.
It is compiled two ways.

## Rust

```sh
cargo run --example mdx                 # plain JSX; components must be imported
cargo run --example mdx -- --components # @mdx-js/mdx-style `_components` mapping
cargo run --example mdx -- --provider   # also reads components from @mdx-js/react's MDXProvider
```

Each command prints the compiled JavaScript module.

## Vite + React

```sh
pnpm install
pnpm dev     # builds the ironmark WASM package first, then starts Vite
```

[`vite.config.mjs`](vite.config.mjs) adds `ironmark/vite`, which compiles each `.mdx` import
with `components: true` and then runs Vite's own JSX transform. [`src/main.jsx`](src/main.jsx)
renders the guide and passes `components`:

- `Note` fills in the component that the guide does not import.
- `h1` and `a` override the markdown-generated tags.
- `wrapper` wraps the whole document.

To use an `MDXProvider` instead of `props.components`, add `@mdx-js/react` and call
`ironmarkMdx({ providerImportSource: "@mdx-js/react" })`.
