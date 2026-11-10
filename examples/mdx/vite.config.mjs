import react from "@vitejs/plugin-react";
import { defineConfig } from "vite";
import ironmarkMdx from "./vite-plugin-ironmark-mdx.mjs";

export default defineConfig({
  plugins: [ironmarkMdx(), react()],
});
