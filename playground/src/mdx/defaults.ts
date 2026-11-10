import type { PlaygroundFile } from "./runtime";

/** MDX files compile through ironmark; everything else is plain JS/JSX. */
export const isMdx = (name: string) => name.endsWith(".mdx");

const INDEX_MDX = `import { Badge, Counter } from "./components.jsx"

export const version = "2.1"
export const features = ["CommonMark", "GFM", "MDX"]

# Hello from MDX <Badge>v{version}</Badge>

This document is compiled by **ironmark** and rendered live with React.
Components come from \`components.jsx\` — edit it in the tab above.

<Note kind="tip">
  \`Note\` is not imported: it is passed in through the \`components\` export.
</Note>

Expressions work too: 6 × 7 = {6 * 7}.

<ul>{features.map((f) => <li key={f}>{f}</li>)}</ul>

<Counter start={3} />

| Feature | Status |
| ------- | :----: |
| Imports | ✅ |
| Live preview | ✅ |
`;

const COMPONENTS_JSX = `import { useState } from "react";

export function Badge({ children }) {
  return <span className="mdx-badge">{children}</span>;
}

export function Note({ kind = "info", children }) {
  return (
    <aside className={\`mdx-note mdx-note-\${kind}\`}>
      <strong>{kind === "tip" ? "Tip" : "Note"}</strong>
      {children}
    </aside>
  );
}

export function Counter({ start = 0 }) {
  const [count, setCount] = useState(start);
  return (
    <button type="button" className="mdx-button" onClick={() => setCount(count + 1)}>
      Clicked {count} times
    </button>
  );
}

/** Every \`components\` export is passed to the main document. */
export const components = {
  Note,
  a: (props) => <a target="_blank" rel="noreferrer" {...props} />,
};
`;

export const DEFAULT_FILES: PlaygroundFile[] = [
  { name: "index.mdx", content: INDEX_MDX },
  { name: "components.jsx", content: COMPONENTS_JSX },
];
