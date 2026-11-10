export function Badge({ children }) {
  return <span className="badge">{children}</span>;
}

export function Note({ kind = "info", children }) {
  return (
    <aside className={`note note-${kind}`}>
      <strong>{kind === "tip" ? "Tip" : "Note"}</strong>
      {children}
    </aside>
  );
}

/** Overrides for markdown-generated elements, passed as `components`. */
export const components = {
  Note,
  h1: (props) => <h1 className="title" {...props} />,
  a: (props) => <a target="_blank" rel="noreferrer" {...props} />,
  wrapper: ({ children }) => <article className="prose">{children}</article>,
};
