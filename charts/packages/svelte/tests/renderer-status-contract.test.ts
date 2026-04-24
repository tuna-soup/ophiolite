import assert from "node:assert/strict";
import test from "node:test";
import { emitRendererStatusForChart, applyRuntimeRendererStatusOverride, resolveRendererStatusPayloadForChart } from "../src/renderer-status";
import { resolveChartRendererStatus } from "../../renderer/src/capabilities";

test("renderer status picks the default backend for auto preference", () => {
  const status = resolveChartRendererStatus({
    chartDefinitionId: "seismic-section",
    rendererKernel: "raster-trace",
    supportTier: "public-launch",
    supportedBackends: ["canvas-2d", "webgl"],
    defaultBackend: "canvas-2d"
  });

  assert.equal(status.activeBackend, "canvas-2d");
  assert.equal(status.availability, "available");
  assert.equal(status.reason, "using-default-backend");
  assert.deepEqual(status.supportedBackends, ["canvas-2d", "webgl"]);
});

test("renderer status honors an available explicit backend request", () => {
  const status = resolveChartRendererStatus({
    chartDefinitionId: "volume-interpretation",
    rendererKernel: "volume-interpretation",
    supportTier: "preview",
    supportedBackends: ["webgl", "vtkjs"],
    defaultBackend: "webgl",
    preference: "vtkjs",
    availableBackends: ["vtkjs", "canvas-2d"]
  });

  assert.equal(status.activeBackend, "vtkjs");
  assert.equal(status.availability, "available");
  assert.equal(status.reason, "using-requested-backend");
  assert.deepEqual(status.availableBackends, ["vtkjs"]);
});

test("renderer status reports unsupported backend requests cleanly", () => {
  const status = resolveChartRendererStatus({
    chartDefinitionId: "survey-map",
    rendererKernel: "survey-map",
    supportTier: "public-launch",
    supportedBackends: ["canvas-2d"],
    defaultBackend: "canvas-2d",
    preference: "vtkjs",
    availableBackends: ["canvas-2d", "vtkjs"]
  });

  assert.equal(status.activeBackend, null);
  assert.equal(status.availability, "unavailable");
  assert.equal(status.reason, "backend-unsupported-by-chart");
  assert.match(status.detail ?? "", /not supported/i);
});

test("runtime override turns an available backend into a runtime failure without dropping backend context", () => {
  const baseStatus = resolveChartRendererStatus({
    chartDefinitionId: "seismic-section",
    rendererKernel: "raster-trace",
    supportTier: "public-launch",
    supportedBackends: ["canvas-2d", "webgl"],
    defaultBackend: "canvas-2d"
  });

  const status = applyRuntimeRendererStatusOverride(baseStatus, "WebGL context lost during first draw");

  assert.equal(status.activeBackend, "canvas-2d");
  assert.equal(status.availability, "runtime-failure");
  assert.equal(status.reason, "runtime-error");
  assert.match(status.detail ?? "", /context lost/i);
});

test("runtime override turns fallback resolution into a runtime failure and keeps fallback backend visible", () => {
  const baseStatus = resolveChartRendererStatus({
    chartDefinitionId: "rock-physics-crossplot",
    rendererKernel: "point-cloud",
    supportTier: "public-launch",
    supportedBackends: ["canvas-2d", "webgl"],
    defaultBackend: "canvas-2d",
    preference: ["webgl", "canvas-2d"],
    availableBackends: ["canvas-2d"]
  });

  const status = applyRuntimeRendererStatusOverride(baseStatus, "Canvas renderer initialization failed");

  assert.equal(baseStatus.availability, "fallback");
  assert.equal(baseStatus.reason, "preferred-backend-unavailable");
  assert.equal(status.activeBackend, "canvas-2d");
  assert.equal(status.availability, "runtime-failure");
  assert.equal(status.reason, "runtime-error");
  assert.deepEqual(status.availableBackends, ["canvas-2d"]);
});

test("runtime override does not mask a capability-level unavailable status", () => {
  const baseStatus = resolveChartRendererStatus({
    chartDefinitionId: "survey-map",
    rendererKernel: "survey-map",
    supportTier: "public-launch",
    supportedBackends: ["canvas-2d"],
    defaultBackend: "canvas-2d",
    preference: "vtkjs",
    availableBackends: ["canvas-2d", "vtkjs"]
  });

  const status = applyRuntimeRendererStatusOverride(baseStatus, "Renderer mount failed");

  assert.equal(status.availability, "unavailable");
  assert.equal(status.reason, "backend-unsupported-by-chart");
});

test("payload resolution applies runtime override from renderer config", () => {
  const payload = resolveRendererStatusPayloadForChart("survey-map", {
    chartId: "survey-map-main",
    renderer: {
      backendPreference: "canvas-2d",
      availableBackends: ["canvas-2d"],
      runtimeErrorMessage: "Overlay renderer failed after mount"
    }
  });

  assert.equal(payload.chartId, "survey-map-main");
  assert.equal(payload.status.activeBackend, "canvas-2d");
  assert.equal(payload.status.availability, "runtime-failure");
  assert.equal(payload.status.reason, "runtime-error");
  assert.match(payload.status.detail ?? "", /after mount/i);
});

test("emitter re-notifies consumers when runtime failure overrides an unchanged capability result", () => {
  const payloads: Array<{ availability: string; reason: string; detail?: string }> = [];

  let lastKey = "";
  lastKey = emitRendererStatusForChart(
    "seismic-gather",
    {
      chartId: "gather-a",
      viewId: "view-1",
      renderer: {
        backendPreference: "canvas-2d",
        availableBackends: ["canvas-2d"]
      }
    },
    lastKey,
    (payload) => {
      payloads.push({
        availability: payload.status.availability,
        reason: payload.status.reason,
        detail: payload.status.detail
      });
    }
  );

  lastKey = emitRendererStatusForChart(
    "seismic-gather",
    {
      chartId: "gather-a",
      viewId: "view-1",
      renderer: {
        backendPreference: "canvas-2d",
        availableBackends: ["canvas-2d"]
      }
    },
    lastKey,
    (payload) => {
      payloads.push({
        availability: payload.status.availability,
        reason: payload.status.reason,
        detail: payload.status.detail
      });
    }
  );

  lastKey = emitRendererStatusForChart(
    "seismic-gather",
    {
      chartId: "gather-a",
      viewId: "view-1",
      renderer: {
        backendPreference: "canvas-2d",
        availableBackends: ["canvas-2d"],
        runtimeErrorMessage: "Canvas renderer panicked after draw"
      }
    },
    lastKey,
    (payload) => {
      payloads.push({
        availability: payload.status.availability,
        reason: payload.status.reason,
        detail: payload.status.detail
      });
    }
  );

  assert.equal(payloads.length, 2);
  assert.deepEqual(payloads.map((payload) => payload.availability), ["available", "runtime-failure"]);
  assert.deepEqual(payloads.map((payload) => payload.reason), ["using-requested-backend", "runtime-error"]);
  assert.match(payloads[1]?.detail ?? "", /panicked after draw/i);
});
