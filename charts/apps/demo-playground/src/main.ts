import {
  STANDARD_ROCK_PHYSICS_TEMPLATE_IDS,
  createMockRockPhysicsCrossplotModel,
  createMockSection,
  createMockWellPanel,
  getDefaultRockPhysicsMockColorMode,
  getRockPhysicsMockColorModes,
  getRockPhysicsTemplateSpec,
  type RockPhysicsMockColorMode,
  type RockPhysicsMockOptions
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
import "./styles.css";

const app = document.querySelector<HTMLDivElement>("#app");
if (!app) {
  throw new Error("Demo root not found.");
}

app.innerHTML = `
  <div class="layout">
    <aside class="sidebar">
      <div>
        <h1>Ophiolite Charts</h1>
        <p>Fresh SDK workspace proving a shared engine for seismic sections and well correlation panels.</p>
      </div>
      <section class="group">
        <h2>Seismic</h2>
        <button id="seismic-zoom-in">Zoom In</button>
        <button id="seismic-zoom-out">Zoom Out</button>
        <button id="seismic-pan-left">Pan Left</button>
        <button id="seismic-pan-right">Pan Right</button>
        <button id="seismic-mode">Switch To Wiggles</button>
        <button id="seismic-color">Switch To Red/White/Blue</button>
      </section>
      <div class="readout" id="seismic-readout">Move over the seismic section.</div>
      <section class="group">
        <h2>Correlation</h2>
        <button id="corr-zoom-in">Zoom In</button>
        <button id="corr-zoom-out">Zoom Out</button>
        <button id="corr-pan-up">Pan Up</button>
        <button id="corr-pan-down">Pan Down</button>
        <input id="corr-marker-name" value="New Marker" />
        <input id="corr-marker-color" value="#3366cc" />
        <button id="corr-arm-marker">Arm Marker Pick</button>
      </section>
      <div class="readout" id="corr-readout">Move over the well panel.</div>
      <section class="group">
        <h2>Rock Physics</h2>
        <button id="rock-template">Next Template</button>
        <button id="rock-color">Switch To Well Colors</button>
        <button id="rock-density">Load Dense Spike</button>
      </section>
      <div class="readout" id="rock-readout">Rock-physics template gallery ready.</div>
    </aside>
    <main class="content">
      <section class="card">
        <header>
          <h2>Seismic Section Chart</h2>
          <p>Heatmap and wiggle modes, probe, and horizon-ready interaction surface.</p>
        </header>
        <div id="seismic-viewer" class="viewer"></div>
      </section>
      <section class="card">
        <header>
          <h2>Well Correlation Panel</h2>
          <p>Shared panel depth with scalar tracks, point overlays, multi-trace wiggles, and section lanes.</p>
        </header>
        <div id="correlation-viewer" class="viewer"></div>
      </section>
      <section class="card">
        <header>
          <h2>Rock Physics Template Gallery</h2>
          <p>Point-cloud crossplots over one strict chart family with shared interactions and standardized rock-physics templates.</p>
        </header>
        <div id="rock-physics-viewer" class="viewer"></div>
      </section>
    </main>
  </div>
`;

const seismicElement = document.querySelector<HTMLElement>("#seismic-viewer");
const correlationElement = document.querySelector<HTMLElement>("#correlation-viewer");
const rockPhysicsElement = document.querySelector<HTMLElement>("#rock-physics-viewer");
if (!seismicElement || !correlationElement || !rockPhysicsElement) {
  throw new Error("Viewer roots not found.");
}

const seismicController = new SeismicViewerController(new MockCanvasRenderer());
const correlationController = new WellCorrelationController(new WellCorrelationCanvasRenderer());
const rockPhysicsController = new RockPhysicsCrossplotController(new PointCloudSpikeRenderer());
const seismicSection = createMockSection();
const correlationPanel = createMockWellPanel();
let rockPhysicsTemplateId: (typeof STANDARD_ROCK_PHYSICS_TEMPLATE_IDS)[number] = "vp-vs-vs-ai";
let rockPhysicsColorMode: RockPhysicsMockColorMode = getDefaultRockPhysicsMockColorMode("vp-vs-vs-ai");
let rockPhysicsDense = false;
let rockPhysicsModel = buildRockPhysicsModel();

seismicController.mount(seismicElement);
seismicController.setSection(seismicSection);
correlationController.mount(correlationElement);
correlationController.setPanel(correlationPanel);
rockPhysicsController.mount(rockPhysicsElement);
rockPhysicsController.setModel(rockPhysicsModel);

let seismicWiggle = false;
let seismicColor = false;
let markerArmed = false;

document.querySelector<HTMLButtonElement>("#seismic-zoom-in")?.addEventListener("click", () => seismicController.zoom(1.4));
document.querySelector<HTMLButtonElement>("#seismic-zoom-out")?.addEventListener("click", () => seismicController.zoom(0.7));
document.querySelector<HTMLButtonElement>("#seismic-pan-left")?.addEventListener("click", () => seismicController.pan(-16, 0));
document.querySelector<HTMLButtonElement>("#seismic-pan-right")?.addEventListener("click", () => seismicController.pan(16, 0));
document.querySelector<HTMLButtonElement>("#seismic-mode")?.addEventListener("click", (event) => {
  seismicWiggle = !seismicWiggle;
  seismicController.setDisplayTransform({ renderMode: seismicWiggle ? "wiggle" : "heatmap" });
  if (event.currentTarget instanceof HTMLButtonElement) {
    event.currentTarget.textContent = seismicWiggle ? "Switch To Heatmap" : "Switch To Wiggles";
  }
});
document.querySelector<HTMLButtonElement>("#seismic-color")?.addEventListener("click", (event) => {
  seismicColor = !seismicColor;
  seismicController.setDisplayTransform({ colormap: seismicColor ? "red-white-blue" : "grayscale" });
  if (event.currentTarget instanceof HTMLButtonElement) {
    event.currentTarget.textContent = seismicColor ? "Switch To Grayscale" : "Switch To Red/White/Blue";
  }
});

document.querySelector<HTMLButtonElement>("#corr-zoom-in")?.addEventListener("click", () => correlationController.zoomVertical(1.35));
document.querySelector<HTMLButtonElement>("#corr-zoom-out")?.addEventListener("click", () => correlationController.zoomVertical(0.74));
document.querySelector<HTMLButtonElement>("#corr-pan-up")?.addEventListener("click", () => correlationController.panVertical(-30));
document.querySelector<HTMLButtonElement>("#corr-pan-down")?.addEventListener("click", () => correlationController.panVertical(30));
document.querySelector<HTMLButtonElement>("#corr-arm-marker")?.addEventListener("click", (event) => {
  const name = (document.querySelector<HTMLInputElement>("#corr-marker-name")?.value || "New Marker").trim();
  const color = document.querySelector<HTMLInputElement>("#corr-marker-color")?.value || "#3366cc";
  correlationController.setActiveMarker(name, color);
  markerArmed = true;
  if (event.currentTarget instanceof HTMLButtonElement) {
    event.currentTarget.textContent = `Marker Armed: ${name}`;
  }
});
document.querySelector<HTMLButtonElement>("#rock-template")?.addEventListener("click", (event) => {
  const currentIndex = STANDARD_ROCK_PHYSICS_TEMPLATE_IDS.indexOf(rockPhysicsTemplateId);
  const nextIndex = (currentIndex + 1) % STANDARD_ROCK_PHYSICS_TEMPLATE_IDS.length;
  rockPhysicsTemplateId = STANDARD_ROCK_PHYSICS_TEMPLATE_IDS[nextIndex]!;
  rockPhysicsColorMode = getDefaultRockPhysicsMockColorMode(rockPhysicsTemplateId);
  rockPhysicsModel = buildRockPhysicsModel();
  rockPhysicsController.setModel(rockPhysicsModel);
  if (event.currentTarget instanceof HTMLButtonElement) {
    event.currentTarget.textContent = `Next Template: ${getRockPhysicsTemplateSpec(rockPhysicsTemplateId).title}`;
  }
  const colorButton = document.querySelector<HTMLButtonElement>("#rock-color");
  if (colorButton) {
    colorButton.textContent = `Next Color: ${rockPhysicsModel.colorBinding.label}`;
  }
  syncReadouts();
});
document.querySelector<HTMLButtonElement>("#rock-color")?.addEventListener("click", (event) => {
  const modes = getRockPhysicsMockColorModes(rockPhysicsTemplateId);
  const currentIndex = Math.max(0, modes.indexOf(rockPhysicsColorMode));
  rockPhysicsColorMode = modes[(currentIndex + 1) % modes.length]!;
  rockPhysicsModel = buildRockPhysicsModel();
  rockPhysicsController.setModel(rockPhysicsModel);
  if (event.currentTarget instanceof HTMLButtonElement) {
    event.currentTarget.textContent = `Next Color: ${rockPhysicsModel.colorBinding.label}`;
  }
  syncReadouts();
});
document.querySelector<HTMLButtonElement>("#rock-density")?.addEventListener("click", (event) => {
  rockPhysicsDense = !rockPhysicsDense;
  rockPhysicsModel = buildRockPhysicsModel();
  rockPhysicsController.setModel(rockPhysicsModel);
  if (event.currentTarget instanceof HTMLButtonElement) {
    event.currentTarget.textContent = rockPhysicsDense ? "Load Standard Spike" : "Load Dense Spike";
  }
  syncReadouts();
});

seismicElement.addEventListener("pointermove", (event) => {
  const point = toLocalPoint(seismicElement, event);
  seismicController.updatePointer(point.x, point.y, seismicElement.clientWidth, seismicElement.clientHeight);
  syncReadouts();
});
seismicElement.addEventListener("pointerleave", () => {
  seismicController.clearPointer();
  syncReadouts();
});

correlationElement.addEventListener("pointermove", (event) => {
  const point = toCorrelationLocalPoint(correlationElement, event);
  correlationController.updatePointer(point.x, point.y, correlationElement.clientWidth, correlationElement.clientHeight);
  syncReadouts();
});
correlationElement.addEventListener("pointerleave", () => {
  correlationController.clearPointer();
  syncReadouts();
});
correlationElement.addEventListener("click", (event) => {
  if (!markerArmed) {
    return;
  }
  const point = toCorrelationLocalPoint(correlationElement, event);
  correlationController.pickMarker(point.x, point.y, correlationElement.clientWidth, correlationElement.clientHeight);
  markerArmed = false;
  const armButton = document.querySelector<HTMLButtonElement>("#corr-arm-marker");
  if (armButton) {
    armButton.textContent = "Arm Marker Pick";
  }
  syncReadouts();
});
correlationElement.addEventListener("wheel", (event) => {
  const scrollHost = getCorrelationScrollHost(correlationElement);
  if (event.shiftKey) {
    scrollHost.scrollLeft += event.deltaY + event.deltaX;
    event.preventDefault();
    return;
  }

  if (event.ctrlKey || event.metaKey) {
    const point = toCorrelationLocalPoint(correlationElement, event);
    const panelDepth = correlationController.getPanelDepthAtViewY(point.y, correlationElement.clientWidth, correlationElement.clientHeight);
    if (panelDepth !== null) {
      correlationController.zoomVerticalAround(panelDepth, event.deltaY < 0 ? 1.12 : 0.89);
      syncReadouts();
      event.preventDefault();
    }
    return;
  }

  correlationController.panVertical(event.deltaY * 0.35);
  syncReadouts();
  event.preventDefault();
}, { passive: false });
correlationElement.addEventListener("ophiolite-charts:correlation-viewport-request", (event) => {
  const detail = (event as CustomEvent<{ depthStart: number; depthEnd: number }>).detail;
  correlationController.setViewport(detail);
  syncReadouts();
});
rockPhysicsElement.addEventListener("pointermove", (event) => {
  const point = toLocalPoint(rockPhysicsElement, event);
  rockPhysicsController.updatePointer(point.x, point.y, rockPhysicsElement.clientWidth, rockPhysicsElement.clientHeight);
  syncReadouts();
});
rockPhysicsElement.addEventListener("pointerleave", () => {
  rockPhysicsController.clearPointer();
  syncReadouts();
});
rockPhysicsElement.addEventListener(
  "wheel",
  (event) => {
    const state = rockPhysicsController.getState();
    if (!state.viewport) {
      return;
    }
    const point = toLocalPoint(rockPhysicsElement, event);
    const xRatio = point.x / Math.max(1, rockPhysicsElement.clientWidth);
    const yRatio = 1 - point.y / Math.max(1, rockPhysicsElement.clientHeight);
    const focusX = state.viewport.xMin + xRatio * (state.viewport.xMax - state.viewport.xMin);
    const focusY = state.viewport.yMin + yRatio * (state.viewport.yMax - state.viewport.yMin);
    rockPhysicsController.zoomAround(focusX, focusY, event.deltaY < 0 ? 1.12 : 0.89);
    syncReadouts();
    event.preventDefault();
  },
  { passive: false }
);

syncReadouts();

function syncReadouts(): void {
  const seismicReadout = document.querySelector<HTMLElement>("#seismic-readout");
  const seismicProbe = seismicController.getState().probe;
  if (seismicReadout) {
    seismicReadout.textContent = seismicProbe
      ? [
          `trace ${seismicProbe.traceIndex} (${seismicProbe.traceCoordinate.toFixed(1)})`,
          `sample ${seismicProbe.sampleIndex} (${seismicProbe.sampleValue.toFixed(1)})`,
          `amplitude ${seismicProbe.amplitude.toFixed(4)}`
        ].join("\n")
      : "Move over the seismic section.";
  }

  const corrReadout = document.querySelector<HTMLElement>("#corr-readout");
  const corrState = correlationController.getState();
  if (corrReadout) {
    corrReadout.textContent = corrState.probe
      ? [
          `${corrState.probe.wellName} / ${corrState.probe.trackTitle}`,
          `panel depth ${corrState.probe.panelDepth.toFixed(1)} native ${corrState.probe.nativeDepth.toFixed(1)}`,
          corrState.probe.markerName
            ? `marker ${corrState.probe.markerName}`
            : `value ${corrState.probe.value?.toFixed(3) ?? "n/a"}`,
          `correlation lines ${corrState.correlationLines.length}`
        ].join("\n")
      : `Move over the well panel.\nCorrelation lines ${corrState.correlationLines.length}`;
  }

  const rockReadout = document.querySelector<HTMLElement>("#rock-readout");
  const rockState = rockPhysicsController.getState();
  if (rockReadout) {
    rockReadout.textContent = rockState.probe
      ? [
          `${rockState.probe.wellName}`,
          `x ${formatRockPhysicsNumber(rockState.probe.xValue)} y ${formatRockPhysicsNumber(rockState.probe.yValue)}`,
          `depth ${rockState.probe.sampleDepthM.toFixed(1)} m`,
          rockState.probe.colorValue !== undefined
            ? `color ${formatRockPhysicsNumber(rockState.probe.colorValue)}`
            : `color ${rockState.probe.colorCategoryLabel ?? "n/a"}`
        ].join("\n")
      : [
          `${rockPhysicsModel.title} - ${rockPhysicsModel.pointCount.toLocaleString()} samples`,
          `wells ${rockPhysicsModel.wells.length}`,
          `color ${rockPhysicsModel.colorBinding.label}`,
          `guides ${rockPhysicsModel.templateOverlays?.length ?? rockPhysicsModel.templateLines?.length ?? 0}`
        ].join("\n");
  }
}

function toLocalPoint(element: HTMLElement, event: PointerEvent | WheelEvent): { x: number; y: number } {
  const rect = element.getBoundingClientRect();
  return {
    x: event.clientX - rect.left,
    y: event.clientY - rect.top
  };
}

function toCorrelationLocalPoint(
  element: HTMLElement,
  event: PointerEvent | WheelEvent
): { x: number; y: number } {
  const rect = element.getBoundingClientRect();
  const scrollHost = getCorrelationScrollHost(element);
  return {
    x: event.clientX - rect.left + scrollHost.scrollLeft,
    y: event.clientY - rect.top
  };
}

function getCorrelationScrollHost(element: HTMLElement): HTMLElement {
  return element.querySelector<HTMLElement>(".ophiolite-charts-correlation-scroll-host") ?? element;
}

function buildRockPhysicsModel() {
  return createMockRockPhysicsCrossplotModel({
    templateId: rockPhysicsTemplateId,
    pointCount: rockPhysicsDense ? 120_000 : 8_000,
    wellCount: rockPhysicsDense ? 10 : 6,
    colorMode: rockPhysicsColorMode
  });
}

function formatRockPhysicsNumber(value: number): string {
  if (Math.abs(value) >= 1_000) {
    return Math.round(value).toString();
  }
  if (Math.abs(value) >= 100) {
    return value.toFixed(1).replace(/\.0$/, "");
  }
  if (Math.abs(value) >= 10) {
    return value.toFixed(2).replace(/\.00$/, "");
  }
  return value.toFixed(3).replace(/\.000$/, "");
}
