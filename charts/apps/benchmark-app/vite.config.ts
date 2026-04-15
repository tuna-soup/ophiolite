import path from "node:path";
import { fileURLToPath } from "node:url";
import { defineConfig } from "vite";

const configDir = path.dirname(fileURLToPath(import.meta.url));

export default defineConfig({
  resolve: {
    alias: {
      "@ophiolite/charts-data-models": path.resolve(configDir, "../../packages/data-models/src/index.ts"),
      "@ophiolite/charts-core": path.resolve(configDir, "../../packages/chart-core/src/index.ts"),
      "@ophiolite/charts-renderer": path.resolve(configDir, "../../packages/renderer/src/index.ts"),
      "@ophiolite/charts-domain": path.resolve(configDir, "../../packages/domain-geoscience/src/index.ts")
    }
  }
});
