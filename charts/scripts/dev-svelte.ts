import path from "node:path";
import { fileURLToPath } from "node:url";

const scriptDir = path.dirname(fileURLToPath(import.meta.url));
const appDir = path.resolve(scriptDir, "../apps/svelte-playground");
const forwardedArgs = process.argv.slice(2);
const cmd = [process.execPath, "run", "dev"];

if (forwardedArgs.length > 0) {
  cmd.push("--", ...forwardedArgs);
}

const proc = Bun.spawn({
  cmd,
  cwd: appDir,
  stdin: "inherit",
  stdout: "inherit",
  stderr: "inherit"
});

let shuttingDown = false;

function killProcessTree(pid: number) {
  if (process.platform === "win32") {
    Bun.spawnSync({
      cmd: ["taskkill", "/PID", String(pid), "/T", "/F"],
      stdout: "ignore",
      stderr: "ignore"
    });
    return;
  }

  proc.kill("SIGTERM");
}

async function shutdown(exitCode = 0) {
  if (shuttingDown) {
    return;
  }

  shuttingDown = true;
  killProcessTree(proc.pid);
  await proc.exited;
  process.exit(exitCode);
}

process.on("SIGINT", () => {
  void shutdown(130);
});

process.on("SIGTERM", () => {
  void shutdown(143);
});

const exitCode = await proc.exited;
process.exit(exitCode);
