import path from "node:path";
import { fileURLToPath } from "node:url";
import { defineConfig } from "vite";
import { resolveChartSourceAliases } from "../../scripts/source-aliases";
import { chartsManualChunks } from "../../scripts/vite-chunking";

const configDir = path.dirname(fileURLToPath(import.meta.url));

export default defineConfig({
  build: {
    chunkSizeWarningLimit: 1000,
    rollupOptions: {
      output: {
        manualChunks: chartsManualChunks
      }
    }
  },
  resolve: {
    alias: resolveChartSourceAliases(configDir)
  }
});
