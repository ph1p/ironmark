import { EditorView } from "@codemirror/view";
import { oneDarkHighlightStyle } from "@codemirror/theme-one-dark";
import { defaultHighlightStyle, syntaxHighlighting } from "@codemirror/language";

export const darkQuery = window.matchMedia("(prefers-color-scheme: dark)");

export const isDark = () => darkQuery.matches;

const darkThemeExt = [
  syntaxHighlighting(oneDarkHighlightStyle),
  EditorView.theme(
    {
      "&": { backgroundColor: "#09090b" },
      ".cm-content": { caretColor: "#d4d4d8" },
      ".cm-cursor, .cm-dropCursor": { borderLeftColor: "#d4d4d8" },
      ".cm-gutters": {
        backgroundColor: "#09090b",
        color: "rgba(113, 113, 122, 0.5)",
        borderRight: "1px solid #27272a",
      },
      ".cm-activeLineGutter": { backgroundColor: "transparent" },
      ".cm-activeLine": { backgroundColor: "transparent" },
      "&.cm-focused > .cm-scroller > .cm-selectionLayer .cm-selectionBackground, .cm-selectionBackground, .cm-content ::selection":
        { backgroundColor: "rgba(113, 113, 122, 0.3)" },
      ".cm-panels": { backgroundColor: "#18181b", color: "#d4d4d8" },
      ".cm-searchMatch": { backgroundColor: "#d4d4d820", outline: "1px solid #71717a40" },
      ".cm-searchMatch.cm-searchMatch-selected": { backgroundColor: "#71717a40" },
      ".cm-selectionMatch": { backgroundColor: "#71717a30" },
      ".cm-matchingBracket, .cm-nonmatchingBracket": {
        backgroundColor: "#71717a40",
        outline: "1px solid #71717a80",
      },
      ".cm-foldPlaceholder": { backgroundColor: "transparent", border: "none", color: "#71717a" },
      ".cm-tooltip": { backgroundColor: "#18181b", border: "1px solid #27272a" },
      ".cm-tooltip .cm-tooltip-arrow:before": {
        borderTopColor: "transparent",
        borderBottomColor: "transparent",
      },
      ".cm-tooltip .cm-tooltip-arrow:after": {
        borderTopColor: "#18181b",
        borderBottomColor: "#18181b",
      },
      ".cm-tooltip-autocomplete": {
        "& > ul > li[aria-selected]": { backgroundColor: "#27272a" },
      },
    },
    { dark: true },
  ),
];

const lightThemeExt = [
  syntaxHighlighting(defaultHighlightStyle),
  EditorView.theme(
    {
      "&": { backgroundColor: "#ffffff" },
      ".cm-content": { caretColor: "#18181b" },
      ".cm-cursor, .cm-dropCursor": { borderLeftColor: "#18181b" },
      ".cm-gutters": {
        backgroundColor: "#ffffff",
        color: "rgba(161, 161, 170, 0.6)",
        borderRight: "1px solid #e4e4e7",
      },
      ".cm-activeLineGutter": { backgroundColor: "transparent" },
      ".cm-activeLine": { backgroundColor: "transparent" },
      "&.cm-focused > .cm-scroller > .cm-selectionLayer .cm-selectionBackground, .cm-selectionBackground, .cm-content ::selection":
        { backgroundColor: "rgba(0, 0, 0, 0.08)" },
      ".cm-panels": { backgroundColor: "#f4f4f5", color: "#18181b" },
      ".cm-searchMatch": { backgroundColor: "#e4e4e7", outline: "1px solid #a1a1aa40" },
      ".cm-searchMatch.cm-searchMatch-selected": { backgroundColor: "#d4d4d8" },
      ".cm-selectionMatch": { backgroundColor: "#e4e4e740" },
      ".cm-matchingBracket, .cm-nonmatchingBracket": {
        backgroundColor: "#e4e4e7",
        outline: "1px solid #a1a1aa80",
      },
      ".cm-foldPlaceholder": { backgroundColor: "transparent", border: "none", color: "#a1a1aa" },
      ".cm-tooltip": { backgroundColor: "#ffffff", border: "1px solid #e4e4e7" },
      ".cm-tooltip-autocomplete": {
        "& > ul > li[aria-selected]": { backgroundColor: "#f4f4f5" },
      },
    },
    { dark: false },
  ),
];

export const cmThemeExtension = () => (isDark() ? darkThemeExt : lightThemeExt);
