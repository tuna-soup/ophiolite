import assert from "node:assert/strict";
import test from "node:test";
import {
  canReuseSectionViewport,
  shouldIgnoreExternalSectionViewport
} from "../src/section-contract-adapter";
import type { SectionViewLike } from "../src/types";

function encodeFloat64(values: number[]): Uint8Array {
  return new Uint8Array(new Float64Array(values).buffer.slice(0));
}

function encodeFloat32(values: number[]): Uint8Array {
  return new Uint8Array(new Float32Array(values).buffer.slice(0));
}

function makeSection(traces: number, samples: number, axis: "inline" | "xline" = "inline"): SectionViewLike {
  return {
    dataset_id: "dataset-a",
    axis,
    coordinate: {
      index: 0,
      value: 100
    },
    traces,
    samples,
    horizontal_axis_f64le: encodeFloat64(new Array(traces).fill(0).map((_, index) => index)),
    inline_axis_f64le:
      axis === "inline" ? encodeFloat64(new Array(traces).fill(0).map((_, index) => index)) : null,
    xline_axis_f64le:
      axis === "xline" ? encodeFloat64(new Array(traces).fill(0).map((_, index) => index)) : null,
    sample_axis_f32le: encodeFloat32(new Array(samples).fill(0).map((_, index) => index)),
    amplitudes_f32le: encodeFloat32(new Array(traces * samples).fill(0)),
    units: null,
    metadata: null,
    display_defaults: null
  };
}

test("canReuseSectionViewport requires matching logical dimensions", () => {
  assert.equal(canReuseSectionViewport(makeSection(10, 20), makeSection(10, 20)), true);
  assert.equal(canReuseSectionViewport(makeSection(10, 20), makeSection(1, 20)), false);
});

test("shouldIgnoreExternalSectionViewport rejects stale viewport keys after incompatible section changes", () => {
  const previous = makeSection(10, 20);
  const next = makeSection(1, 200);
  const viewportKey = "viewer-a:{\"trace_start\":0}";

  assert.equal(
    shouldIgnoreExternalSectionViewport(previous, next, viewportKey, null),
    true
  );
  assert.equal(
    shouldIgnoreExternalSectionViewport(previous, next, viewportKey, viewportKey),
    true
  );
  assert.equal(
    shouldIgnoreExternalSectionViewport(null, next, viewportKey, null),
    false
  );
  assert.equal(
    shouldIgnoreExternalSectionViewport(previous, previous, viewportKey, null),
    false
  );
});
