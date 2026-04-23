import { defineConfig } from "@playwright/test";

export default defineConfig({
  testDir: "./tests/visual",
  snapshotPathTemplate: "{testDir}/{testFilePath}-snapshots/{arg}{ext}",
  use: {
    baseURL: "http://127.0.0.1:4173",
    viewport: {
      width: 1440,
      height: 1800
    },
    colorScheme: "light",
    deviceScaleFactor: 1
  },
  webServer: {
    command: "bun --filter @ophiolite/charts-docs dev -- --host 127.0.0.1 --port 4173",
    port: 4173,
    reuseExistingServer: !process.env.CI,
    timeout: 120000
  }
});
