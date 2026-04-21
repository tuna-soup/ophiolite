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

const app = document.querySelector<HTMLDivElement>("#app");
if (!app) {
  throw new Error("Benchmark root not found.");
}

app.innerHTML = `
  <style>
    body { margin: 0; font-family: Segoe UI, sans-serif; background: #10151a; color: #eef4f7; }
    .shell { padding: 20px; display: grid; gap: 16px; }
    .stats { padding: 12px 14px; background: #162029; border-radius: 12px; white-space: pre-wrap; }
    .viewer { height: 320px; border-radius: 14px; overflow: hidden; background: #0a1218; }
  </style>
  <div class="shell">
    <div class="stats" id="stats">Benchmarking...</div>
    <div id="seismic" class="viewer"></div>
    <div id="correlation" class="viewer"></div>
    <div id="rock-physics" class="viewer"></div>
  </div>
`;

const seismicContainer = document.querySelector<HTMLElement>("#seismic");
const correlationContainer = document.querySelector<HTMLElement>("#correlation");
const rockPhysicsContainer = document.querySelector<HTMLElement>("#rock-physics");
const stats = document.querySelector<HTMLElement>("#stats");
if (!seismicContainer || !correlationContainer || !rockPhysicsContainer || !stats) {
  throw new Error("Benchmark containers not found.");
}

const seismicElement = seismicContainer;
const correlationElement = correlationContainer;
const rockPhysicsElement = rockPhysicsContainer;
const statsElement = stats;

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

const seismicStart = performance.now();
seismicController.mount(seismicContainer);
seismicController.setSection(seismic);
const seismicEnd = performance.now();

const correlationStart = performance.now();
correlationController.mount(correlationContainer);
correlationController.setPanel(correlation);
const correlationEnd = performance.now();

const rockPhysicsStart = performance.now();
rockPhysicsController.mount(rockPhysicsContainer);
rockPhysicsController.setModel(rockPhysics);
const rockPhysicsEnd = performance.now();

runBenchmarks().catch((error: unknown) => {
  statsElement.textContent = error instanceof Error ? error.message : String(error);
});

async function runBenchmarks(): Promise<void> {
  await nextFrame();

  const seismicPointer = benchmarkPointerSweep(seismicElement, (x, y) => {
    seismicController.updatePointer(x, y, seismicElement.clientWidth, seismicElement.clientHeight);
  });

  const correlationPointer = benchmarkPointerSweep(correlationElement, (x, y) => {
    correlationController.updatePointer(x, y, correlationElement.clientWidth, correlationElement.clientHeight);
  });

  statsElement.textContent = [
    `Seismic traces: ${seismic.dimensions.traces}, samples: ${seismic.dimensions.samples}`,
    `Correlation wells: ${correlation.wells.length}, tracks: ${correlation.wells.reduce((sum, well) => sum + well.tracks.length, 0)}`,
    `Rock physics points: ${rockPhysics.pointCount.toLocaleString()}, wells: ${rockPhysics.wells.length}`,
    `Seismic setup: ${(seismicEnd - seismicStart).toFixed(2)} ms`,
    `Correlation setup: ${(correlationEnd - correlationStart).toFixed(2)} ms`,
    `Rock physics crossplot setup: ${(rockPhysicsEnd - rockPhysicsStart).toFixed(2)} ms`,
    `Seismic pointer sweep: mean ${seismicPointer.meanMs.toFixed(3)} ms, p95 ${seismicPointer.p95Ms.toFixed(3)} ms`,
    `Correlation pointer sweep: mean ${correlationPointer.meanMs.toFixed(3)} ms, p95 ${correlationPointer.p95Ms.toFixed(3)} ms`
  ].join("\n");
}

function benchmarkPointerSweep(
  container: HTMLElement,
  updatePointer: (x: number, y: number) => void
): { meanMs: number; p95Ms: number } {
  const width = Math.max(1, container.clientWidth);
  const height = Math.max(1, container.clientHeight);
  const samples: number[] = [];
  const verticalSteps = 48;
  const horizontalSteps = 12;

  for (let vertical = 0; vertical < verticalSteps; vertical += 1) {
    const y = (height * (vertical + 0.5)) / verticalSteps;
    for (let horizontal = 0; horizontal < horizontalSteps; horizontal += 1) {
      const x = (width * (horizontal + 0.5)) / horizontalSteps;
      const start = performance.now();
      updatePointer(x, y);
      samples.push(performance.now() - start);
    }
  }

  samples.sort((left, right) => left - right);
  const total = samples.reduce((sum, sample) => sum + sample, 0);
  const p95Index = Math.min(samples.length - 1, Math.floor(samples.length * 0.95));
  return {
    meanMs: total / Math.max(1, samples.length),
    p95Ms: samples[p95Index] ?? 0
  };
}

function nextFrame(): Promise<void> {
  return new Promise((resolve) => {
    requestAnimationFrame(() => resolve());
  });
}
