import path from "node:path";
import { fileURLToPath } from "node:url";

import { build } from "vite";
import { svelte } from "@sveltejs/vite-plugin-svelte";

const packageDir = path.dirname(fileURLToPath(import.meta.url));
const staticOutDir = path.resolve(packageDir, "../../../python/jupyter/src/ophiolite_jupyter/static");

const widgetBuilds = [
  {
    entry: path.resolve(packageDir, "src/afm/avo-response.ts"),
    fileName: "widget-avo-response.js",
    emptyOutDir: true
  },
  {
    entry: path.resolve(packageDir, "src/afm/avo-crossplot.ts"),
    fileName: "widget-avo-crossplot.js",
    emptyOutDir: false
  }
];

for (const widgetBuild of widgetBuilds) {
  await build({
    configFile: false,
    root: packageDir,
    plugins: [svelte()],
    build: {
      outDir: staticOutDir,
      emptyOutDir: widgetBuild.emptyOutDir,
      cssCodeSplit: false,
      assetsInlineLimit: Number.MAX_SAFE_INTEGER,
      lib: {
        entry: widgetBuild.entry,
        formats: ["es"],
        fileName: () => widgetBuild.fileName
      },
      rollupOptions: {
        output: {
          inlineDynamicImports: true,
          assetFileNames: (assetInfo) => {
            if (assetInfo.name?.endsWith(".css")) {
              return "widget.css";
            }
            return "[name][extname]";
          }
        }
      }
    }
  });
}
