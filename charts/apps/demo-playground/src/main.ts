import {
  STANDARD_ROCK_PHYSICS_TEMPLATE_IDS,
  createMockVolumeInterpretationModel,
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
import { VolumeInterpretationController } from "@ophiolite/charts-domain/preview";
import {
  MockCanvasRenderer,
  PointCloudSpikeRenderer,
  WellCorrelationCanvasRenderer
} from "@ophiolite/charts-renderer";
import { VolumeInterpretationVtkRenderer } from "@ophiolite/charts-renderer/preview";
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
        <p>Shared chart workspace spanning sections, panels, crossplots, and a VTK-backed volume interpretation spike.</p>
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
      <section class="group">
        <h2>Volume Interpretation</h2>
        <button id="volume-tool-pointer">Pointer</button>
        <button id="volume-tool-orbit">Orbit</button>
        <button id="volume-tool-pan">Pan</button>
        <button id="volume-tool-slice">Slice Drag</button>
        <button id="volume-tool-seed">Interpret Seed</button>
        <button id="volume-fit">Fit View</button>
        <button id="volume-center">Center Selection</button>
      </section>
      <div class="readout" id="volume-readout">Volume scene ready.</div>
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
      <section class="card">
        <header>
          <h2>Volume Interpretation Workspace</h2>
          <p>Orthogonal slice planes, horizon surfaces, wells, picks, and semantic interpretation requests over one resolved scene DTO.</p>
        </header>
        <div id="volume-interpretation-viewer" class="viewer"></div>
      </section>
    </main>
  </div>
`;

const seismicElement = document.querySelector<HTMLElement>("#seismic-viewer");
const correlationElement = document.querySelector<HTMLElement>("#correlation-viewer");
const rockPhysicsElement = document.querySelector<HTMLElement>("#rock-physics-viewer");
const volumeInterpretationElement = document.querySelector<HTMLElement>("#volume-interpretation-viewer");
if (!seismicElement || !correlationElement || !rockPhysicsElement || !volumeInterpretationElement) {
  throw new Error("Viewer roots not found.");
}

const seismicController = new SeismicViewerController(new MockCanvasRenderer());
const correlationController = new WellCorrelationController(new WellCorrelationCanvasRenderer());
const rockPhysicsController = new RockPhysicsCrossplotController(new PointCloudSpikeRenderer());
const volumeInterpretationController = new VolumeInterpretationController(new VolumeInterpretationVtkRenderer());
const seismicSection = createMockSection();
const correlationPanel = createMockWellPanel();
let rockPhysicsTemplateId: (typeof STANDARD_ROCK_PHYSICS_TEMPLATE_IDS)[number] = "vp-vs-vs-ai";
let rockPhysicsColorMode: RockPhysicsMockColorMode = getDefaultRockPhysicsMockColorMode("vp-vs-vs-ai");
let rockPhysicsDense = false;
let rockPhysicsModel = buildRockPhysicsModel();
let volumeInterpretationModel = createMockVolumeInterpretationModel();
let volumeInterpretationTool: "pointer" | "orbit" | "pan" | "slice-drag" | "interpret-seed" = "pointer";
let volumeInterpretationPendingMessage = "Volume scene ready.";
let volumePointerDrag:
  | {
      pointerId: number;
      x: number;
      y: number;
    }
  | null = null;

seismicController.mount(seismicElement);
seismicController.setSection(seismicSection);
correlationController.mount(correlationElement);
correlationController.setPanel(correlationPanel);
rockPhysicsController.mount(rockPhysicsElement);
rockPhysicsController.setModel(rockPhysicsModel);
volumeInterpretationController.mount(volumeInterpretationElement);
volumeInterpretationController.setModel(volumeInterpretationModel);
volumeInterpretationController.setTool(volumeInterpretationTool);

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
document.querySelector<HTMLButtonElement>("#volume-tool-pointer")?.addEventListener("click", () => setVolumeInterpretationTool("pointer"));
document.querySelector<HTMLButtonElement>("#volume-tool-orbit")?.addEventListener("click", () => setVolumeInterpretationTool("orbit"));
document.querySelector<HTMLButtonElement>("#volume-tool-pan")?.addEventListener("click", () => setVolumeInterpretationTool("pan"));
document.querySelector<HTMLButtonElement>("#volume-tool-slice")?.addEventListener("click", () => setVolumeInterpretationTool("slice-drag"));
document.querySelector<HTMLButtonElement>("#volume-tool-seed")?.addEventListener("click", () => setVolumeInterpretationTool("interpret-seed"));
document.querySelector<HTMLButtonElement>("#volume-fit")?.addEventListener("click", () => {
  volumeInterpretationController.fitToData();
  syncReadouts();
});
document.querySelector<HTMLButtonElement>("#volume-center")?.addEventListener("click", () => {
  volumeInterpretationController.centerSelection();
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
volumeInterpretationElement.addEventListener("pointermove", (event) => {
  const point = toLocalPoint(volumeInterpretationElement, event);
  if (volumePointerDrag?.pointerId === event.pointerId) {
    const deltaX = point.x - volumePointerDrag.x;
    const deltaY = point.y - volumePointerDrag.y;
    if (volumeInterpretationTool === "orbit") {
      volumeInterpretationController.orbit(deltaX * 0.35, deltaY * 0.28);
    } else if (volumeInterpretationTool === "pan") {
      volumeInterpretationController.pan(deltaX, deltaY);
    } else if (volumeInterpretationTool === "slice-drag") {
      volumeInterpretationController.moveActiveSlice(-deltaY * 2.2);
    }
    volumePointerDrag = {
      pointerId: event.pointerId,
      x: point.x,
      y: point.y
    };
  }
  volumeInterpretationController.updatePointer(point.x, point.y);
  syncReadouts();
});
volumeInterpretationElement.addEventListener("pointerdown", (event) => {
  volumePointerDrag = {
    pointerId: event.pointerId,
    x: event.offsetX,
    y: event.offsetY
  };
  volumeInterpretationElement.setPointerCapture(event.pointerId);
});
volumeInterpretationElement.addEventListener("pointerup", (event) => {
  if (volumePointerDrag?.pointerId === event.pointerId) {
    volumePointerDrag = null;
  }
  if (volumeInterpretationElement.hasPointerCapture(event.pointerId)) {
    volumeInterpretationElement.releasePointerCapture(event.pointerId);
  }
});
volumeInterpretationElement.addEventListener("pointercancel", (event) => {
  if (volumePointerDrag?.pointerId === event.pointerId) {
    volumePointerDrag = null;
  }
  if (volumeInterpretationElement.hasPointerCapture(event.pointerId)) {
    volumeInterpretationElement.releasePointerCapture(event.pointerId);
  }
});
volumeInterpretationElement.addEventListener("pointerleave", () => {
  volumeInterpretationController.clearPointer();
  syncReadouts();
});
volumeInterpretationElement.addEventListener("click", (event) => {
  const point = toLocalPoint(volumeInterpretationElement, event);
  volumeInterpretationController.handlePrimaryAction(point.x, point.y);
  syncReadouts();
});
volumeInterpretationElement.addEventListener(
  "wheel",
  (event) => {
    if (volumeInterpretationTool === "orbit") {
      volumeInterpretationController.orbit(event.deltaX * 0.08, event.deltaY * 0.08);
    } else if (volumeInterpretationTool === "slice-drag") {
      volumeInterpretationController.moveActiveSlice(event.deltaY * 2.4);
    } else {
      volumeInterpretationController.zoom(event.deltaY < 0 ? 1.08 : 0.92);
    }
    syncReadouts();
    event.preventDefault();
  },
  { passive: false }
);
volumeInterpretationController.onInterpretationRequest((request) => {
  const targetHorizonId = request.targetHorizonId ?? volumeInterpretationModel.horizons[0]?.id;
  if (!targetHorizonId) {
    return;
  }
  volumeInterpretationPendingMessage = `Interpretation request: ${request.kind} @ (${request.worldX.toFixed(0)}, ${request.worldY.toFixed(0)}, ${request.worldZ.toFixed(0)})`;
  volumeInterpretationModel = {
    ...volumeInterpretationModel,
    horizons: volumeInterpretationModel.horizons.map((horizon) =>
      horizon.id === targetHorizonId
        ? {
            ...horizon,
            points: Float32Array.from(horizon.points, (value, index) =>
              index % 3 === 2
                ? value + Math.sin(request.worldX * 0.002 + request.worldY * 0.001 + index * 0.03) * 18
                : value
            )
          }
        : horizon
    )
  };
  volumeInterpretationController.setModel(volumeInterpretationModel);
  volumeInterpretationController.setTool(volumeInterpretationTool);
  syncReadouts();
});

updateVolumeInterpretationToolButtons();
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

  const volumeReadout = document.querySelector<HTMLElement>("#volume-readout");
  const volumeState = volumeInterpretationController.getState();
  if (volumeReadout) {
    volumeReadout.textContent = volumeState.probe
      ? [
          `${volumeState.tool} / ${volumeState.probe.target.itemName ?? volumeState.probe.target.itemId ?? volumeState.probe.target.kind}`,
          `world ${volumeState.probe.worldX.toFixed(0)}, ${volumeState.probe.worldY.toFixed(0)}, ${volumeState.probe.worldZ.toFixed(0)}`,
          volumeState.selection
            ? `selection ${volumeState.selection.kind} ${volumeState.selection.itemName ?? volumeState.selection.itemId}`
            : "selection none",
          volumeInterpretationPendingMessage
        ].join("\n")
      : [
          `${volumeInterpretationModel.name} / tool ${volumeState.tool}`,
          `volumes ${volumeInterpretationModel.volumes.length} horizons ${volumeInterpretationModel.horizons.length}`,
          `wells ${volumeInterpretationModel.wells.length} markers ${volumeInterpretationModel.markers.length}`,
          volumeInterpretationPendingMessage
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

function setVolumeInterpretationTool(
  tool: "pointer" | "orbit" | "pan" | "slice-drag" | "interpret-seed"
): void {
  volumeInterpretationTool = tool;
  volumeInterpretationController.setTool(tool);
  updateVolumeInterpretationToolButtons();
  syncReadouts();
}

function updateVolumeInterpretationToolButtons(): void {
  const buttons = [
    ["#volume-tool-pointer", "pointer", "Pointer"],
    ["#volume-tool-orbit", "orbit", "Orbit"],
    ["#volume-tool-pan", "pan", "Pan"],
    ["#volume-tool-slice", "slice-drag", "Slice Drag"],
    ["#volume-tool-seed", "interpret-seed", "Interpret Seed"]
  ] as const;

  for (const [selector, tool, label] of buttons) {
    const button = document.querySelector<HTMLButtonElement>(selector);
    if (button) {
      button.textContent = volumeInterpretationTool === tool ? `${label} Active` : label;
    }
  }
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
