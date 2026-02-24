import { defineConfig } from "vite";
import tailwindcss from "@tailwindcss/vite";
import wasm from "vite-plugin-wasm";

export default defineConfig({
  base: "/ironmark/",
  plugins: [tailwindcss(), wasm()],
});
