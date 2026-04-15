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

stats.textContent = [
  `Seismic traces: ${seismic.dimensions.traces}, samples: ${seismic.dimensions.samples}`,
  `Correlation wells: ${correlation.wells.length}, tracks: ${correlation.wells.reduce((sum, well) => sum + well.tracks.length, 0)}`,
  `Rock physics points: ${rockPhysics.pointCount.toLocaleString()}, wells: ${rockPhysics.wells.length}`,
  `Seismic setup: ${(seismicEnd - seismicStart).toFixed(2)} ms`,
  `Correlation setup: ${(correlationEnd - correlationStart).toFixed(2)} ms`,
  `Rock physics crossplot setup: ${(rockPhysicsEnd - rockPhysicsStart).toFixed(2)} ms`
].join("\n");
