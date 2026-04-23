import path from "node:path";
import { fileURLToPath } from "node:url";
import { svelte } from "@sveltejs/vite-plugin-svelte";
import { defineConfig } from "vite";
import { resolveSvelteChartSourceAliases } from "../../scripts/source-aliases";

const configDir = path.dirname(fileURLToPath(import.meta.url));

export default defineConfig({
  plugins: [svelte()],
  resolve: {
    alias: resolveSvelteChartSourceAliases(configDir)
  },
  server: {
    port: 5173,
    strictPort: true
  }
});
