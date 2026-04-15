import { createMockSection, createMockWellPanel } from "@ophiolite/charts-data-models";
import { SeismicViewerController, WellCorrelationController } from "@ophiolite/charts-domain";
import { MockCanvasRenderer, WellCorrelationCanvasRenderer } from "@ophiolite/charts-renderer";
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
    </main>
  </div>
`;

const seismicElement = document.querySelector<HTMLElement>("#seismic-viewer");
const correlationElement = document.querySelector<HTMLElement>("#correlation-viewer");
if (!seismicElement || !correlationElement) {
  throw new Error("Viewer roots not found.");
}

const seismicController = new SeismicViewerController(new MockCanvasRenderer());
const correlationController = new WellCorrelationController(new WellCorrelationCanvasRenderer());
const seismicSection = createMockSection();
const correlationPanel = createMockWellPanel();

seismicController.mount(seismicElement);
seismicController.setSection(seismicSection);
correlationController.mount(correlationElement);
correlationController.setPanel(correlationPanel);

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
}

function toLocalPoint(element: HTMLElement, event: PointerEvent): { x: number; y: number } {
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
