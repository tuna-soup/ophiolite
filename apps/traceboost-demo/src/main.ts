import type {
  RunSectionBrowsingBenchmarkRequest,
  RunSectionBrowsingBenchmarkResponse
} from "./lib/bridge";
import "./lib/styles/ui.css";

declare global {
  interface Window {
    traceboostBenchmarks?: {
      runSectionBrowsingBenchmark: (
        request: RunSectionBrowsingBenchmarkRequest
      ) => Promise<RunSectionBrowsingBenchmarkResponse>;
    };
    __TRACEBOOST_STARTUP__?: {
      fail: (message: string, detail?: string) => void;
      markMounted: () => void;
    };
  }
}

type BridgeModule = typeof import("./lib/bridge");
type AppModule = typeof import("./App.svelte");

function startupState() {
  return typeof window !== "undefined" ? window.__TRACEBOOST_STARTUP__ : undefined;
}

function describeError(error: unknown): string {
  if (error instanceof Error) {
    return error.stack || error.message;
  }
  return String(error);
}

function failStartup(message: string, error?: unknown): void {
  startupState()?.fail(message, error ? describeError(error) : "");
}

function installGlobalErrorHandlers(): void {
  if (typeof window === "undefined") {
    return;
  }

  window.addEventListener("error", (event) => {
    const scriptTarget = event.target instanceof HTMLScriptElement ? event.target.src : "";
    const detail = scriptTarget || event.message || describeError(event.error);
    failStartup("TraceBoost hit an unhandled browser error during startup.", detail);
  });

  window.addEventListener("unhandledrejection", (event) => {
    failStartup("TraceBoost hit an unhandled promise rejection during startup.", event.reason);
  });
}

async function bootstrap() {
  installGlobalErrorHandlers();

  try {
    const [{ mount }, appModule, bridgeModule] = await Promise.all([
      import("svelte"),
      import("./App.svelte"),
      import("./lib/bridge")
    ]);

    const { default: App } = appModule as AppModule;
    const {
      runSectionBrowsingBenchmark
    } = bridgeModule as BridgeModule;

    if (typeof window !== "undefined") {
      window.traceboostBenchmarks = {
        runSectionBrowsingBenchmark
      };
    }

    const target = document.getElementById("app");
    if (!target) {
      throw new Error("Missing #app root element.");
    }

    const app = mount(App, { target });
    startupState()?.markMounted();
    return app;
  } catch (error) {
    failStartup("TraceBoost could not finish bootstrapping the frontend.", error);
    throw error;
  }
}

export default bootstrap();
