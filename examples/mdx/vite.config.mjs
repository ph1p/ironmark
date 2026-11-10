import react from "@vitejs/plugin-react";
import ironmarkMdx from "ironmark/vite";
import { defineConfig } from "vite";

export default defineConfig({
  plugins: [ironmarkMdx(), react()],
});
