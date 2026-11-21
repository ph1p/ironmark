import react from "@vitejs/plugin-react";
import ironmark from "ironmark/vite";
import { defineConfig } from "vite";

export default defineConfig({
  plugins: [ironmark(), react()],
});
