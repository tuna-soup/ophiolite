import path from "node:path";
import { fileURLToPath } from "node:url";
import { svelte } from "@sveltejs/vite-plugin-svelte";
import { defineConfig } from "vite";
import { resolveSvelteChartSourceAliases } from "../../scripts/source-aliases";
import { chartsManualChunks } from "../../scripts/vite-chunking";

const configDir = path.dirname(fileURLToPath(import.meta.url));

export default defineConfig({
  plugins: [svelte()],
  build: {
    rollupOptions: {
      output: {
        manualChunks: chartsManualChunks
      }
    }
  },
  resolve: {
    alias: resolveSvelteChartSourceAliases(configDir)
  },
  server: {
    port: 4173,
    strictPort: true
  }
});
