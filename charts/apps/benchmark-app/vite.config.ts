import path from "node:path";
import { fileURLToPath } from "node:url";
import { defineConfig } from "vite";
import { resolveChartSourceAliases } from "../../scripts/source-aliases";

const configDir = path.dirname(fileURLToPath(import.meta.url));

export default defineConfig({
  resolve: {
    alias: resolveChartSourceAliases(configDir)
  }
});
