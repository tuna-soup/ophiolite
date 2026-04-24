import { spawnSync } from "node:child_process";
import path from "node:path";
import { svelte } from "@sveltejs/vite-plugin-svelte";
import { defineConfig, type Plugin } from "vite";

const host = process.env.TAURI_DEV_HOST;
const devPort = Number(process.env.TRACEBOOST_DEV_PORT ?? "1420");
const hmrPort = Number(process.env.TRACEBOOST_HMR_PORT ?? String(devPort + 1));

type SegyHeaderValueType = "i16" | "i32";

interface SegyHeaderFieldBody {
  start_byte?: unknown;
  value_type?: unknown;
}

interface SegyGeometryOverrideBody {
  inline_3d?: SegyHeaderFieldBody | null;
  crossline_3d?: SegyHeaderFieldBody | null;
  third_axis?: SegyHeaderFieldBody | null;
}

function traceboostDevApi(): Plugin {
  const repoRoot = path.resolve(__dirname, "../..");

  function runCargo(args: string[]): string {
    const result = spawnSync("cargo", args, {
      cwd: repoRoot,
      encoding: "utf8"
    });
    if (result.status !== 0) {
      throw new Error(result.stderr || result.stdout || "cargo command failed");
    }
    return result.stdout.trim();
  }

  async function readJsonBody(
    req: NodeJS.ReadableStream & { setEncoding(encoding: BufferEncoding): void }
  ): Promise<Record<string, unknown>> {
    return await new Promise((resolve, reject) => {
      let body = "";
      req.setEncoding("utf8");
      req.on("data", (chunk) => {
        body += chunk;
      });
      req.on("end", () => {
        try {
          resolve(body ? JSON.parse(body) : {});
        } catch (error) {
          reject(error);
        }
      });
      req.on("error", reject);
    });
  }

  function appendGeometryOverrideArgs(args: string[], rawOverride: unknown): void {
    if (!rawOverride || typeof rawOverride !== "object") {
      return;
    }

    const geometryOverride = rawOverride as SegyGeometryOverrideBody;
    appendHeaderFieldArgs(args, "--inline-byte", "--inline-type", geometryOverride.inline_3d);
    appendHeaderFieldArgs(args, "--crossline-byte", "--crossline-type", geometryOverride.crossline_3d);
    appendHeaderFieldArgs(args, "--third-axis-byte", "--third-axis-type", geometryOverride.third_axis);
  }

  function appendHeaderFieldArgs(
    args: string[],
    byteFlag: string,
    typeFlag: string,
    rawField: SegyHeaderFieldBody | null | undefined
  ): void {
    if (!rawField || typeof rawField !== "object") {
      return;
    }

    const startByte = typeof rawField.start_byte === "number" ? Math.trunc(rawField.start_byte) : null;
    if (!startByte || startByte < 1) {
      return;
    }

    const valueType: SegyHeaderValueType = rawField.value_type === "i16" ? "i16" : "i32";
    args.push(byteFlag, String(startByte), typeFlag, valueType);
  }

  return {
    name: "traceboost-dev-api",
    configureServer(server) {
      server.middlewares.use("/api/preflight", async (req, res) => {
        try {
          const body = await readJsonBody(req);
          const inputPath = typeof body.inputPath === "string" ? body.inputPath.trim() : "";
          if (!inputPath) {
            res.statusCode = 400;
            res.end("Missing inputPath");
            return;
          }
          const payload = runCargo([
            "run",
            "-q",
            "-p",
            "traceboost-app",
            "--",
            "preflight-import",
            inputPath,
            ...(() => {
              const args: string[] = [];
              appendGeometryOverrideArgs(args, body.geometryOverride);
              return args;
            })()
          ]);
          res.setHeader("Content-Type", "application/json");
          res.end(payload);
        } catch (error) {
          res.statusCode = 500;
          res.end(error instanceof Error ? error.message : "Unknown preflight error");
        }
      });

      server.middlewares.use("/api/import", async (req, res) => {
        try {
          const body = await readJsonBody(req);
          const inputPath = typeof body.inputPath === "string" ? body.inputPath.trim() : "";
          const outputStorePath =
            typeof body.outputStorePath === "string" ? body.outputStorePath.trim() : "";
          const overwriteExisting = body.overwriteExisting === true;
          if (!inputPath || !outputStorePath) {
            res.statusCode = 400;
            res.end("Missing inputPath or outputStorePath");
            return;
          }
          const args = [
            "run",
            "-q",
            "-p",
            "traceboost-app",
            "--",
            "import-dataset",
            inputPath,
            outputStorePath
          ];
          appendGeometryOverrideArgs(args, body.geometryOverride);
          if (overwriteExisting) {
            args.push("--overwrite-existing");
          }
          const payload = runCargo(args);
          res.setHeader("Content-Type", "application/json");
          res.end(payload);
        } catch (error) {
          res.statusCode = 500;
          res.end(error instanceof Error ? error.message : "Unknown import error");
        }
      });

      server.middlewares.use("/api/open", async (req, res) => {
        try {
          const body = await readJsonBody(req);
          const storePath = typeof body.storePath === "string" ? body.storePath.trim() : "";
          if (!storePath) {
            res.statusCode = 400;
            res.end("Missing storePath");
            return;
          }
          const payload = runCargo([
            "run",
            "-q",
            "-p",
            "traceboost-app",
            "--",
            "open-dataset",
            storePath
          ]);
          res.setHeader("Content-Type", "application/json");
          res.end(payload);
        } catch (error) {
          res.statusCode = 500;
          res.end(error instanceof Error ? error.message : "Unknown open-store error");
        }
      });

      server.middlewares.use("/api/horizons/import", async (req, res) => {
        try {
          const body = await readJsonBody(req);
          const storePath = typeof body.storePath === "string" ? body.storePath.trim() : "";
          const inputPaths = Array.isArray(body.inputPaths)
            ? body.inputPaths
                .map((value) => (typeof value === "string" ? value.trim() : ""))
                .filter((value) => value.length > 0)
            : [];
          const sourceCoordinateReferenceId =
            typeof body.sourceCoordinateReferenceId === "string"
              ? body.sourceCoordinateReferenceId.trim()
              : "";
          const sourceCoordinateReferenceName =
            typeof body.sourceCoordinateReferenceName === "string"
              ? body.sourceCoordinateReferenceName.trim()
              : "";
          const assumeSameAsSurvey = body.assumeSameAsSurvey === true;
          if (!storePath || inputPaths.length === 0) {
            res.statusCode = 400;
            res.end("Missing storePath or inputPaths");
            return;
          }
          const args = [
            "run",
            "-q",
            "-p",
            "traceboost-app",
            "--",
            "import-horizons",
            storePath
          ];
          if (sourceCoordinateReferenceId) {
            args.push("--source-coordinate-reference-id", sourceCoordinateReferenceId);
          }
          if (sourceCoordinateReferenceName) {
            args.push("--source-coordinate-reference-name", sourceCoordinateReferenceName);
          }
          if (assumeSameAsSurvey) {
            args.push("--assume-same-as-survey");
          }
          const payload = runCargo([
            ...args,
            ...inputPaths
          ]);
          res.setHeader("Content-Type", "application/json");
          res.end(payload);
        } catch (error) {
          res.statusCode = 500;
          res.end(error instanceof Error ? error.message : "Unknown horizon import error");
        }
      });

      server.middlewares.use("/api/section", (req, res) => {
        try {
          const url = new URL(req.url ?? "/", "http://localhost");
          const storePath = url.searchParams.get("storePath")?.trim();
          const axis = url.searchParams.get("axis") ?? "inline";
          const index = url.searchParams.get("index") ?? "0";
          if (!storePath) {
            res.statusCode = 400;
            res.end("Missing storePath");
            return;
          }
          const body = runCargo([
            "run",
            "-q",
            "-p",
            "traceboost-app",
            "--",
            "view-section",
            storePath,
            axis,
            index
          ]);
          res.setHeader("Content-Type", "application/json");
          res.end(body);
        } catch (error) {
          res.statusCode = 500;
          res.setHeader("Content-Type", "application/json");
          res.end(
            JSON.stringify({
              message: error instanceof Error ? error.message : "Unknown backend bridge error"
            })
          );
        }
      });

      server.middlewares.use("/api/horizons/section", (req, res) => {
        try {
          const url = new URL(req.url ?? "/", "http://localhost");
          const storePath = url.searchParams.get("storePath")?.trim();
          const axis = url.searchParams.get("axis") ?? "inline";
          const index = url.searchParams.get("index") ?? "0";
          if (!storePath) {
            res.statusCode = 400;
            res.end("Missing storePath");
            return;
          }
          const body = runCargo([
            "run",
            "-q",
            "-p",
            "traceboost-app",
            "--",
            "view-section-horizons",
            storePath,
            axis,
            index
          ]);
          res.setHeader("Content-Type", "application/json");
          res.end(body);
        } catch (error) {
          res.statusCode = 500;
          res.setHeader("Content-Type", "application/json");
          res.end(
            JSON.stringify({
              message: error instanceof Error ? error.message : "Unknown backend bridge error"
            })
          );
        }
      });
    }
  };
}

export default defineConfig({
  plugins: [svelte(), traceboostDevApi()],
  clearScreen: false,
  cacheDir: path.join("node_modules", ".vite", process.platform),
  optimizeDeps: {
    force: true,
    exclude: [
      "@ophiolite/contracts",
      "@ophiolite/charts",
      "@ophiolite/charts-core",
      "@ophiolite/charts-data-models",
      "@ophiolite/charts-domain",
      "@ophiolite/charts-renderer",
      "@ophiolite/charts-toolbar",
      "@traceboost/seis-contracts"
    ]
  },
  server: {
    port: devPort,
    strictPort: true,
    host: host || false,
    hmr: host
      ? {
          protocol: "ws",
          host,
          port: hmrPort
        }
      : undefined,
    watch: {
      ignored: ["**/src-tauri/**"]
    }
  },
  envPrefix: ["VITE_", "TAURI_ENV_*"],
  build: {
    target: process.env.TAURI_ENV_PLATFORM
      ? process.env.TAURI_ENV_PLATFORM === "windows"
        ? "chrome105"
        : "safari13"
      : "es2020",
    minify: process.env.TAURI_ENV_DEBUG ? false : "esbuild",
    sourcemap: Boolean(process.env.TAURI_ENV_DEBUG)
  },
  resolve: {
    preserveSymlinks: true,
    dedupe: ["svelte"],
    alias: {
      "@ophiolite/charts/extras": path.resolve(
        __dirname,
        "../../charts/packages/svelte/src/extras.ts"
      ),
      "@ophiolite/charts": path.resolve(
        __dirname,
        "../../charts/packages/svelte/src/index.ts"
      ),
      "@ophiolite/charts-toolbar": path.resolve(
        __dirname,
        "../../charts/packages/svelte-toolbar/src/index.ts"
      ),
      "@ophiolite/charts-renderer": path.resolve(
        __dirname,
        "../../charts/packages/renderer/src/index.ts"
      ),
      "@ophiolite/charts-domain": path.resolve(
        __dirname,
        "../../charts/packages/domain-geoscience/src/index.ts"
      ),
      "@ophiolite/charts-data-models": path.resolve(
        __dirname,
        "../../charts/packages/data-models/src/index.ts"
      ),
      "@ophiolite/charts-core": path.resolve(
        __dirname,
        "../../charts/packages/chart-core/src/index.ts"
      ),
      "@ophiolite/contracts": path.resolve(
        __dirname,
        "../../contracts/ts/ophiolite-contracts/src/index.ts"
      ),
      "@traceboost/seis-contracts": path.resolve(
        __dirname,
        "../../traceboost/contracts/ts/seis-contracts/src/index.ts"
      )
    }
  }
});
