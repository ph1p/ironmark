import type { EditorView } from "@codemirror/view";
import {
  previewPanel,
  htmlPanel,
  tabPreview,
  tabHtml,
  panelEditor,
  panelOutput,
  mobileTabEditor,
  mobileTabPreview,
  mobileTabHtml,
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
let state: { dirty: boolean; lastHtml: string };

function setOutputTab(tab: "preview" | "html") {
  const isPreview = tab === "preview";
  previewPanel.classList.toggle("hidden", !isPreview);
  htmlPanel.classList.toggle("hidden", isPreview);
  setActiveTab(isPreview ? tabPreview : tabHtml, [tabPreview, tabHtml]);

  if (!isPreview && state.dirty) {
    state.dirty = false;
    htmlView.dispatch({
      changes: { from: 0, to: htmlView.state.doc.length, insert: formatHtml(state.lastHtml) },
    });
  }
}

function setMobilePanel(panel: "editor" | "preview" | "html") {
  const tabs = [mobileTabEditor, mobileTabPreview, mobileTabHtml];
  panelEditor.classList.toggle("hidden", panel !== "editor");
  panelEditor.classList.toggle("flex", panel === "editor");
  panelOutput.classList.toggle("hidden", panel === "editor");
  panelOutput.classList.toggle("flex", panel !== "editor");

  if (panel === "editor") setActiveTab(mobileTabEditor, tabs);
  else if (panel === "preview") {
    setActiveTab(mobileTabPreview, tabs);
    setOutputTab("preview");
  } else {
    setActiveTab(mobileTabHtml, tabs);
    setOutputTab("html");
  }
}

export function initTabs(view: EditorView, dirtyRef: { dirty: boolean; lastHtml: string }) {
  htmlView = view;
  state = dirtyRef;
  tabPreview.addEventListener("click", () => setOutputTab("preview"));
  tabHtml.addEventListener("click", () => setOutputTab("html"));
  mobileTabEditor.addEventListener("click", () => setMobilePanel("editor"));
  mobileTabPreview.addEventListener("click", () => setMobilePanel("preview"));
  mobileTabHtml.addEventListener("click", () => setMobilePanel("html"));
}
