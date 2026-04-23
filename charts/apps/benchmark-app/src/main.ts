import {
  createMockRockPhysicsCrossplotModel,
  createMockSection,
  createMockWellPanel
} from "@ophiolite/charts-data-models";
import {
  RockPhysicsCrossplotController,
  SeismicViewerController,
  WellCorrelationController
} from "@ophiolite/charts-domain";
import {
  MockCanvasRenderer,
  PointCloudSpikeRenderer,
  WellCorrelationCanvasRenderer
} from "@ophiolite/charts-renderer";

type BenchmarkMode = "smoke" | "development" | "authoritative";

interface BenchmarkConfig {
  mode: BenchmarkMode;
  pointerSweepRepeats: number;
  pointerGrid: {
    verticalSteps: number;
    horizontalSteps: number;
  };
}

interface SummaryStats {
  meanMs: number;
  medianMs: number;
  p95Ms: number;
  minMs: number;
  maxMs: number;
}

interface PointerBenchmarkResult {
  chart: string;
  repeats: number;
  sampleCount: number;
  perSweepMs: number[];
  perEventMs: number[];
  summary: SummaryStats;
}

interface BenchmarkResults {
  timestamp: string;
  mode: BenchmarkMode;
  metadata: Record<string, string | number | boolean | null>;
  fixtures: Record<string, string | number>;
  setupMs: Record<string, number>;
  pointerBenchmarks: PointerBenchmarkResult[];
}

const MODE_CONFIG: Record<BenchmarkMode, BenchmarkConfig> = {
  smoke: {
    mode: "smoke",
    pointerSweepRepeats: 2,
    pointerGrid: {
      verticalSteps: 32,
      horizontalSteps: 10
    }
  },
  development: {
    mode: "development",
    pointerSweepRepeats: 6,
    pointerGrid: {
      verticalSteps: 48,
      horizontalSteps: 12
    }
  },
  authoritative: {
    mode: "authoritative",
    pointerSweepRepeats: 12,
    pointerGrid: {
      verticalSteps: 64,
      horizontalSteps: 16
    }
  }
};

const params = new URLSearchParams(window.location.search);
const mode = parseMode(params.get("mode"));
const config = MODE_CONFIG[mode];

const app = document.querySelector<HTMLDivElement>("#app");
if (!app) {
  throw new Error("Benchmark root not found.");
}

app.innerHTML = `
  <style>
    :root { color-scheme: dark; }
    body { margin: 0; font-family: Segoe UI, sans-serif; background: #10151a; color: #eef4f7; }
    .shell { padding: 20px; display: grid; gap: 16px; }
    .mode { display: flex; flex-wrap: wrap; gap: 10px; align-items: center; }
    .chip { padding: 6px 10px; border-radius: 999px; background: #1d2a35; color: #dcebf2; font-size: 13px; }
    .stats,
    .json { padding: 12px 14px; background: #162029; border-radius: 12px; white-space: pre-wrap; }
    .json { max-height: 320px; overflow: auto; font-family: Consolas, monospace; font-size: 12px; }
    .viewer-grid { display: grid; gap: 16px; grid-template-columns: repeat(auto-fit, minmax(280px, 1fr)); }
    .viewer-card { display: grid; gap: 10px; }
    .viewer-title { font-size: 13px; text-transform: uppercase; letter-spacing: 0.08em; color: #9eb6c3; }
    .viewer { height: 320px; border-radius: 14px; overflow: hidden; background: #0a1218; }
  </style>
  <div class="shell">
    <div class="mode">
      <span class="chip">Mode: ${config.mode}</span>
      <span class="chip">Repeats: ${config.pointerSweepRepeats}</span>
      <span class="chip">Grid: ${config.pointerGrid.verticalSteps}x${config.pointerGrid.horizontalSteps}</span>
    </div>
    <div class="stats" id="stats">Benchmarking...</div>
    <div class="json" id="raw-results">Preparing raw results...</div>
    <div class="viewer-grid">
      <div class="viewer-card">
        <div class="viewer-title">Seismic Section</div>
        <div id="seismic" class="viewer"></div>
      </div>
      <div class="viewer-card">
        <div class="viewer-title">Well Correlation</div>
        <div id="correlation" class="viewer"></div>
      </div>
      <div class="viewer-card">
        <div class="viewer-title">Rock Physics</div>
        <div id="rock-physics" class="viewer"></div>
      </div>
    </div>
  </div>
`;

const seismicContainer = document.querySelector<HTMLElement>("#seismic");
const correlationContainer = document.querySelector<HTMLElement>("#correlation");
const rockPhysicsContainer = document.querySelector<HTMLElement>("#rock-physics");
const statsElement = document.querySelector<HTMLElement>("#stats");
const rawResultsElement = document.querySelector<HTMLElement>("#raw-results");
if (!seismicContainer || !correlationContainer || !rockPhysicsContainer || !statsElement || !rawResultsElement) {
  throw new Error("Benchmark containers not found.");
}

const seismicHost = seismicContainer;
const correlationHost = correlationContainer;
const rockPhysicsHost = rockPhysicsContainer;
const statsHost = statsElement;
const rawResultsHost = rawResultsElement;

const seismic = createMockSection();
const correlation = createMockWellPanel();
const rockPhysics = createMockRockPhysicsCrossplotModel({
  pointCount: 120_000,
  wellCount: 10,
  colorMode: "water-saturation"
});

const seismicController = new SeismicViewerController(new MockCanvasRenderer());
const correlationController = new WellCorrelationController(new WellCorrelationCanvasRenderer());
const rockPhysicsController = new RockPhysicsCrossplotController(new PointCloudSpikeRenderer());

const seismicSetupMs = time(() => {
  seismicController.mount(seismicHost);
  seismicController.setSection(seismic);
});
const correlationSetupMs = time(() => {
  correlationController.mount(correlationHost);
  correlationController.setPanel(correlation);
});
const rockPhysicsSetupMs = time(() => {
  rockPhysicsController.mount(rockPhysicsHost);
  rockPhysicsController.setModel(rockPhysics);
});

runBenchmarks().catch((error: unknown) => {
  const message = error instanceof Error ? error.message : String(error);
  statsHost.textContent = message;
  rawResultsHost.textContent = message;
});

async function runBenchmarks(): Promise<void> {
  await nextFrame();

  const pointerBenchmarks: PointerBenchmarkResult[] = [
    benchmarkPointerSweeps("Seismic Section", seismicHost, (x, y) => {
      seismicController.updatePointer(x, y, seismicHost.clientWidth, seismicHost.clientHeight);
    }),
    benchmarkPointerSweeps("Well Correlation", correlationHost, (x, y) => {
      correlationController.updatePointer(x, y, correlationHost.clientWidth, correlationHost.clientHeight);
    })
  ];

  const results: BenchmarkResults = {
    timestamp: new Date().toISOString(),
    mode: config.mode,
    metadata: collectRuntimeMetadata(),
    fixtures: {
      seismicTraces: seismic.dimensions.traces,
      seismicSamples: seismic.dimensions.samples,
      correlationWells: correlation.wells.length,
      correlationTracks: correlation.wells.reduce((sum, well) => sum + well.tracks.length, 0),
      rockPhysicsPoints: rockPhysics.pointCount,
      rockPhysicsWells: rockPhysics.wells.length
    },
    setupMs: {
      seismicSection: seismicSetupMs,
      wellCorrelation: correlationSetupMs,
      rockPhysics: rockPhysicsSetupMs
    },
    pointerBenchmarks
  };

  statsHost.textContent = formatSummary(results);
  rawResultsHost.textContent = JSON.stringify(results, null, 2);
  Object.assign(window, { __OPHIOLITE_CHARTS_BENCHMARK_RESULTS__: results });
}

function benchmarkPointerSweeps(
  chart: string,
  container: HTMLElement,
  updatePointer: (x: number, y: number) => void
): PointerBenchmarkResult {
  const width = Math.max(1, container.clientWidth);
  const height = Math.max(1, container.clientHeight);
  const perEventMs: number[] = [];
  const perSweepMs: number[] = [];

  for (let repeat = 0; repeat < config.pointerSweepRepeats; repeat += 1) {
    const sweepStart = performance.now();
    for (let vertical = 0; vertical < config.pointerGrid.verticalSteps; vertical += 1) {
      const y = (height * (vertical + 0.5)) / config.pointerGrid.verticalSteps;
      for (let horizontal = 0; horizontal < config.pointerGrid.horizontalSteps; horizontal += 1) {
        const x = (width * (horizontal + 0.5)) / config.pointerGrid.horizontalSteps;
        const start = performance.now();
        updatePointer(x, y);
        perEventMs.push(performance.now() - start);
      }
    }
    perSweepMs.push(performance.now() - sweepStart);
  }

  return {
    chart,
    repeats: config.pointerSweepRepeats,
    sampleCount: perEventMs.length,
    perSweepMs,
    perEventMs,
    summary: summarize(perEventMs)
  };
}

function summarize(samples: number[]): SummaryStats {
  const sorted = [...samples].sort((left, right) => left - right);
  const total = sorted.reduce((sum, sample) => sum + sample, 0);
  const middle = Math.floor(sorted.length / 2);
  const medianMs =
    sorted.length % 2 === 0
      ? ((sorted[middle - 1] ?? 0) + (sorted[middle] ?? 0)) / 2
      : (sorted[middle] ?? 0);
  const p95Index = Math.min(sorted.length - 1, Math.floor(sorted.length * 0.95));

  return {
    meanMs: total / Math.max(1, sorted.length),
    medianMs,
    p95Ms: sorted[p95Index] ?? 0,
    minMs: sorted[0] ?? 0,
    maxMs: sorted[sorted.length - 1] ?? 0
  };
}

function collectRuntimeMetadata(): Record<string, string | number | boolean | null> {
  const nav = navigator as Navigator & {
    deviceMemory?: number;
  };

  return {
    userAgent: navigator.userAgent,
    language: navigator.language,
    platform: navigator.platform,
    hardwareConcurrency: navigator.hardwareConcurrency,
    deviceMemoryGb: nav.deviceMemory ?? null,
    devicePixelRatio: window.devicePixelRatio,
    viewportWidth: window.innerWidth,
    viewportHeight: window.innerHeight,
    online: navigator.onLine
  };
}

function formatSummary(results: BenchmarkResults): string {
  return [
    `Mode: ${results.mode}`,
    `Timestamp: ${results.timestamp}`,
    `Seismic setup: ${results.setupMs.seismicSection.toFixed(2)} ms`,
    `Well correlation setup: ${results.setupMs.wellCorrelation.toFixed(2)} ms`,
    `Rock physics setup: ${results.setupMs.rockPhysics.toFixed(2)} ms`,
    ...results.pointerBenchmarks.map((benchmark) =>
      [
        `${benchmark.chart}:`,
        `  mean ${benchmark.summary.meanMs.toFixed(3)} ms`,
        `  median ${benchmark.summary.medianMs.toFixed(3)} ms`,
        `  p95 ${benchmark.summary.p95Ms.toFixed(3)} ms`,
        `  min ${benchmark.summary.minMs.toFixed(3)} ms`,
        `  max ${benchmark.summary.maxMs.toFixed(3)} ms`,
        `  repeats ${benchmark.repeats}, samples ${benchmark.sampleCount}`
      ].join("\n")
    )
  ].join("\n");
}

function parseMode(value: string | null): BenchmarkMode {
  if (value === "development" || value === "authoritative") {
    return value;
  }
  return "smoke";
}

function time(callback: () => void): number {
  const start = performance.now();
  callback();
  return performance.now() - start;
}

function nextFrame(): Promise<void> {
  return new Promise((resolve) => {
    requestAnimationFrame(() => resolve());
  });
}
