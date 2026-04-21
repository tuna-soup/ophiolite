import { mount } from "svelte";
import App from "./App.svelte";
import {
  runSectionBrowsingBenchmark,
  type RunSectionBrowsingBenchmarkRequest,
  type RunSectionBrowsingBenchmarkResponse
} from "./lib/bridge";
import "./lib/styles/ui.css";

declare global {
  interface Window {
    traceboostBenchmarks?: {
      runSectionBrowsingBenchmark: (
        request: RunSectionBrowsingBenchmarkRequest
      ) => Promise<RunSectionBrowsingBenchmarkResponse>;
    };
  }
}

if (typeof window !== "undefined") {
  window.traceboostBenchmarks = {
    runSectionBrowsingBenchmark
  };
}

const app = mount(App, {
  target: document.getElementById("app")!
});

export default app;
