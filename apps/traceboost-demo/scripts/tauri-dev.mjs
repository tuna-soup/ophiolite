import net from "node:net";
import path from "node:path";
import { fileURLToPath } from "node:url";
import { spawn } from "node:child_process";

const scriptDir = path.dirname(fileURLToPath(import.meta.url));
const appRoot = path.resolve(scriptDir, "..");

function canListen(port) {
  return new Promise((resolve) => {
    const server = net.createServer();
    server.unref();
    server.on("error", () => resolve(false));
    server.listen(port, "127.0.0.1", () => {
      server.close(() => resolve(true));
    });
  });
}

async function findPort(startPort, attempts = 20) {
  for (let offset = 0; offset < attempts; offset += 1) {
    const port = startPort + offset;
    if (await canListen(port)) {
      return port;
    }
  }
  throw new Error(`Unable to find a free dev port starting at ${startPort}.`);
}

const frontendPort = await findPort(1420);
const hmrPort = await findPort(frontendPort === 1420 ? 1421 : frontendPort + 1);
const devUrl = `http://localhost:${frontendPort}`;
const tauriBinaryName =
  process.platform === "win32"
    ? "tauri.exe"
    : "tauri";
const tauriBinary = path.join(appRoot, "node_modules", ".bin", tauriBinaryName);

const configOverride = JSON.stringify({
  build: {
    devUrl
  }
});

console.log(`Starting traceboost-demo on ${devUrl}`);

const child = spawn(tauriBinary, ["dev", "--config", configOverride], {
  cwd: appRoot,
  stdio: "inherit",
  env: {
    ...process.env,
    TRACEBOOST_DEV_PORT: String(frontendPort),
    TRACEBOOST_HMR_PORT: String(hmrPort)
  }
});

child.on("exit", (code, signal) => {
  if (signal) {
    process.kill(process.pid, signal);
    return;
  }
  process.exit(code ?? 1);
});
