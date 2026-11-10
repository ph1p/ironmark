import { Component, type ComponentType, type ReactNode } from "react";
import * as React from "react";
import * as jsxRuntime from "react/jsx-runtime";
import { createRoot, type Root } from "react-dom/client";
import { mdxToJsx } from "ironmark";
import { isMdx } from "./defaults";

export type PlaygroundFile = { name: string; content: string };

type Module = { exports: Record<string, unknown> };
type Transform = (code: string, filePath: string) => string;

/** Bare imports the preview can satisfy; everything else must be a workspace file. */
const BUILTINS: Record<string, unknown> = {
  react: React,
  "react/jsx-runtime": jsxRuntime,
  "react/jsx-dev-runtime": jsxRuntime,
};

let transformPromise: Promise<Transform> | null = null;

/** Sucrase is loaded on first use so the plain-markdown playground never pays for it. */
function loadTransform(): Promise<Transform> {
  transformPromise ??= import("sucrase").then(
    ({ transform }) =>
      (code, filePath) =>
        transform(code, {
          transforms: ["jsx", "imports"],
          jsxRuntime: "automatic",
          production: true,
          filePath,
        }).code,
  );
  return transformPromise;
}

function normalize(spec: string): string {
  return spec.replace(/^\.\//, "");
}

function resolve(spec: string, files: Map<string, PlaygroundFile>): string | null {
  const name = normalize(spec);
  for (const candidate of [name, `${name}.jsx`, `${name}.js`, `${name}.mdx`]) {
    if (files.has(candidate)) return candidate;
  }
  return null;
}

/**
 * Evaluates the workspace as CommonJS: each file is compiled (MDX through
 * ironmark, then JSX/ESM through sucrase) and run with a `require` that
 * resolves `./name` imports to sibling files and `react` to the page's React.
 */
function createLoader(
  files: Map<string, PlaygroundFile>,
  compile: (file: PlaygroundFile) => string,
): (name: string) => Record<string, unknown> {
  const cache = new Map<string, Module>();

  const load = (name: string): Record<string, unknown> => {
    const cached = cache.get(name);
    if (cached) return cached.exports;
    const file = files.get(name);
    if (!file) throw new Error(`Cannot find module "${name}"`);

    const module: Module = { exports: {} };
    cache.set(name, module);
    const code = compile(file);
    const require = (spec: string) => {
      if (spec in BUILTINS) return BUILTINS[spec];
      const target = resolve(spec, files);
      if (!target) throw new Error(`${name}: cannot resolve import "${spec}"`);
      return load(target);
    };
    // The workspace is the user's own local code (no share links), so it runs in-page.
    new Function("require", "module", "exports", code)(require, module, module.exports);
    return module.exports;
  };

  return load;
}

/** Remounted per render (`key`), so a fixed file clears the previous error. */
class ErrorBoundary extends Component<{ children: ReactNode }, { error: Error | null }> {
  state = { error: null as Error | null };

  static getDerivedStateFromError(error: Error) {
    return { error };
  }

  render() {
    return this.state.error ? <ErrorView error={this.state.error} /> : this.props.children;
  }
}

function ErrorView({ error }: { error: unknown }) {
  return (
    <pre className="mdx-error text-xs whitespace-pre-wrap text-red-600 dark:text-red-400">
      {error instanceof Error ? error.message : String(error)}
    </pre>
  );
}

/** Merges every `components` export from the non-entry JS files. */
function collectComponents(
  files: PlaygroundFile[],
  exportsOf: (name: string) => Record<string, unknown>,
) {
  const merged: Record<string, unknown> = {};
  for (const file of files.slice(1)) {
    if (!/\.jsx?$/.test(file.name)) continue;
    const value = exportsOf(file.name).components;
    if (value && typeof value === "object") Object.assign(merged, value);
  }
  return merged;
}

export type MdxPreview = {
  render: (files: PlaygroundFile[], mdxOptions: object) => Promise<void>;
  dispose: () => void;
};

export function createMdxPreview(container: HTMLElement): MdxPreview {
  let root: Root | null = createRoot(container);
  let generation = 0;
  // Compiled code per file, reused while its content and options are unchanged.
  const compiled = new Map<string, { key: string; code: string }>();

  return {
    async render(files, mdxOptions) {
      const run = ++generation;
      const transform = await loadTransform();
      if (run !== generation || !root || files.length === 0) return;

      const optionsKey = JSON.stringify(mdxOptions);
      const compile = ({ name, content }: PlaygroundFile) => {
        const key = isMdx(name) ? `${optionsKey}\0${content}` : content;
        const hit = compiled.get(name);
        if (hit?.key === key) return hit.code;
        let code: string;
        try {
          const source = isMdx(name)
            ? mdxToJsx(content, { ...mdxOptions, components: true }).module
            : content;
          code = transform(source, isMdx(name) ? `${name}.jsx` : name);
        } catch (err) {
          throw new Error(`${name}: ${err instanceof Error ? err.message : String(err)}`);
        }
        compiled.set(name, { key, code });
        return code;
      };

      const byName = new Map(files.map((f) => [f.name, f]));
      let node: ReactNode;
      try {
        const exportsOf = createLoader(byName, compile);
        const Content = exportsOf(files[0].name).default as ComponentType<{
          components?: Record<string, unknown>;
        }>;
        if (typeof Content !== "function")
          throw new Error(`${files[0].name} has no default export`);
        node = <Content components={collectComponents(files, exportsOf)} />;
      } catch (err) {
        node = <ErrorView error={err} />;
      }
      root.render(<ErrorBoundary key={run}>{node}</ErrorBoundary>);
    },
    dispose() {
      generation++;
      const r = root;
      root = null;
      // Unmounting synchronously during a React render warns; defer it.
      queueMicrotask(() => r?.unmount());
    },
  };
}
