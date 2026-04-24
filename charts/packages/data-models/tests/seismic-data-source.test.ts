import assert from "node:assert/strict";
import test from "node:test";
import {
  createSeismicSectionWindowRequest,
  defaultSeismicSectionLod,
  needsSeismicSectionWindowLoad,
  normalizeChartBackendPreference
} from "../src";
import { createSectionPayload, createSectionViewport } from "../../../tests/fixtures/seismic";

test("backend preference normalization keeps auto empty and preserves explicit order", () => {
  assert.deepEqual(normalizeChartBackendPreference("auto"), []);
  assert.deepEqual(normalizeChartBackendPreference("canvas-2d"), ["canvas-2d"]);
  assert.deepEqual(normalizeChartBackendPreference(["vtkjs", "webgl"]), ["vtkjs", "webgl"]);
});

test("window requests apply halo, logical dimensions, and LOD selection", () => {
  const section = createSectionPayload({
    traces: 64,
    samples: 128,
    logicalDimensions: {
      traces: 512,
      samples: 2048
    },
    window: {
      traceStart: 24,
      traceEnd: 88,
      sampleStart: 80,
      sampleEnd: 208,
      lod: 1
    }
  });
  const viewport = createSectionViewport();

  const request = createSeismicSectionWindowRequest(section, viewport, {
    traceHalo: 12,
    sampleHalo: 20
  });

  assert.deepEqual(request.viewport, viewport);
  assert.deepEqual(request.traceRange, [0, 42]);
  assert.deepEqual(request.sampleRange, [0, 48]);
  assert.equal(request.lod, 0);
  assert.equal(request.reason, "viewport");
});

test("default seismic LOD increases with dominant viewport span", () => {
  assert.equal(defaultSeismicSectionLod({ traceStart: 0, traceEnd: 200, sampleStart: 0, sampleEnd: 320 }), 0);
  assert.equal(defaultSeismicSectionLod({ traceStart: 0, traceEnd: 900, sampleStart: 0, sampleEnd: 100 }), 2);
  assert.equal(defaultSeismicSectionLod({ traceStart: 0, traceEnd: 1800, sampleStart: 0, sampleEnd: 100 }), 3);
});

test("window-load checks depend on the currently loaded section window", () => {
  const viewport = createSectionViewport();
  const fullyLoaded = createSectionPayload({
    window: {
      traceStart: 0,
      traceEnd: 64,
      sampleStart: 0,
      sampleEnd: 128,
      lod: 0
    }
  });
  const partial = createSectionPayload({
    window: {
      traceStart: 12,
      traceEnd: 28,
      sampleStart: 6,
      sampleEnd: 20,
      lod: 1
    }
  });

  assert.equal(needsSeismicSectionWindowLoad(null, viewport), false);
  assert.equal(needsSeismicSectionWindowLoad(fullyLoaded, viewport), false);
  assert.equal(needsSeismicSectionWindowLoad(partial, viewport), true);
});
