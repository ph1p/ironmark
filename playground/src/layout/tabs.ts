import type { EditorView } from "@codemirror/view";
import {
  previewPanel,
  htmlPanel,
  astPanel,
  tabPreview,
  tabHtml,
  tabAst,
  panelEditor,
  panelOutput,
  mobileTabEditor,
  mobileTabPreview,
  mobileTabHtml,
  mobileTabAst,
} from "./app";
import { formatHtml } from "../util/format-html";

const ACTIVE = ["text-zinc-900", "dark:text-zinc-100", "border-zinc-900", "dark:border-zinc-100"];
const INACTIVE = ["text-zinc-400", "dark:text-zinc-500", "border-transparent"];

function setActiveTab(btn: HTMLButtonElement, siblings: HTMLButtonElement[]) {
  for (const s of siblings) {
    s.classList.remove(...ACTIVE);
    s.classList.add(...INACTIVE);
  }
  btn.classList.remove(...INACTIVE);
  btn.classList.add(...ACTIVE);
}

let htmlView: EditorView;
let astView: EditorView;
let state: { dirty: boolean; lastHtml: string; astDirty: boolean; lastAst: string };

type OutputTab = "preview" | "html" | "ast";

function setOutputTab(tab: OutputTab) {
  const desktopTabs = [tabPreview, tabHtml, tabAst];
  previewPanel.classList.toggle("hidden", tab !== "preview");
  htmlPanel.classList.toggle("hidden", tab !== "html");
  astPanel.classList.toggle("hidden", tab !== "ast");

  const activeBtn = tab === "preview" ? tabPreview : tab === "html" ? tabHtml : tabAst;
  setActiveTab(activeBtn, desktopTabs);

  if (tab === "html" && state.dirty) {
    state.dirty = false;
    htmlView.dispatch({
      changes: { from: 0, to: htmlView.state.doc.length, insert: formatHtml(state.lastHtml) },
    });
  }

  if (tab === "ast" && state.astDirty) {
    state.astDirty = false;
    astView.dispatch({
      changes: { from: 0, to: astView.state.doc.length, insert: state.lastAst },
    });
  }
}

function setMobilePanel(panel: "editor" | "preview" | "html" | "ast") {
  const tabs = [mobileTabEditor, mobileTabPreview, mobileTabHtml, mobileTabAst];
  panelEditor.classList.toggle("hidden", panel !== "editor");
  panelEditor.classList.toggle("flex", panel === "editor");
  panelOutput.classList.toggle("hidden", panel === "editor");
  panelOutput.classList.toggle("flex", panel !== "editor");

  if (panel === "editor") setActiveTab(mobileTabEditor, tabs);
  else if (panel === "preview") {
    setActiveTab(mobileTabPreview, tabs);
    setOutputTab("preview");
  } else if (panel === "html") {
    setActiveTab(mobileTabHtml, tabs);
    setOutputTab("html");
  } else {
    setActiveTab(mobileTabAst, tabs);
    setOutputTab("ast");
  }
}

export function initTabs(
  hView: EditorView,
  aView: EditorView,
  dirtyRef: { dirty: boolean; lastHtml: string; astDirty: boolean; lastAst: string },
) {
  htmlView = hView;
  astView = aView;
  state = dirtyRef;
  tabPreview.addEventListener("click", () => setOutputTab("preview"));
  tabHtml.addEventListener("click", () => setOutputTab("html"));
  tabAst.addEventListener("click", () => setOutputTab("ast"));
  mobileTabEditor.addEventListener("click", () => setMobilePanel("editor"));
  mobileTabPreview.addEventListener("click", () => setMobilePanel("preview"));
  mobileTabHtml.addEventListener("click", () => setMobilePanel("html"));
  mobileTabAst.addEventListener("click", () => setMobilePanel("ast"));
}
