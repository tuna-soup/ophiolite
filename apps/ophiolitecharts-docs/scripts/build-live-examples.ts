import path from "node:path";
import { fileURLToPath } from "node:url";

const scriptDir = path.dirname(fileURLToPath(import.meta.url));
const appDir = path.resolve(scriptDir, "..");
const repoRoot = path.resolve(appDir, "../..");
const playgroundDir = path.resolve(repoRoot, "charts/apps/svelte-playground");
const outputDir = path.resolve(appDir, "public/live");

const proc = Bun.spawn({
  cmd: [
    "bun",
    "run",
    "build",
    "--",
    "--base",
    "/live/",
    "--outDir",
    outputDir,
    "--emptyOutDir"
  ],
  cwd: playgroundDir,
  stdout: "inherit",
  stderr: "inherit"
});

const exitCode = await proc.exited;
if (exitCode !== 0) {
  process.exit(exitCode);
}
