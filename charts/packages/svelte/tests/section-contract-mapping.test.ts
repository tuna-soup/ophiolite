import assert from "node:assert/strict";
import test from "node:test";
import {
  decodeSectionView,
  interactionToContract,
  mergeDisplayTransform,
  probeToContract,
  viewportFromContract,
  viewportToContract
} from "../src/section-contract-adapter";
import { createCursorProbe, createEncodedSectionView, createSectionViewport } from "../../../tests/fixtures/seismic";

test("viewport and probe helpers preserve contract field names and values", () => {
  const viewport = createSectionViewport();
  const contractViewport = {
    trace_start: viewport.traceStart,
    trace_end: viewport.traceEnd,
    sample_start: viewport.sampleStart,
    sample_end: viewport.sampleEnd
  };
  const probe = createCursorProbe();

  assert.deepEqual(viewportFromContract(contractViewport), viewport);
  assert.deepEqual(viewportToContract("chart-a", "view-a", viewport), {
    chart_id: "chart-a",
    view_id: "view-a",
    viewport: contractViewport
  });
  assert.deepEqual(probeToContract("chart-a", "view-a", probe), {
    chart_id: "chart-a",
    view_id: "view-a",
    probe: {
      trace_index: probe.traceIndex,
      trace_coordinate: probe.traceCoordinate,
      inline_coordinate: probe.inlineCoordinate,
      xline_coordinate: probe.xlineCoordinate,
      sample_index: probe.sampleIndex,
      sample_value: probe.sampleValue,
      amplitude: probe.amplitude
    }
  });
  assert.deepEqual(probeToContract("chart-a", "view-a", null), {
    chart_id: "chart-a",
    view_id: "view-a",
    probe: null
  });
});

test("interaction and display helpers map public wrapper state onto contracts", () => {
  const section = createEncodedSectionView({
    displayDefaults: {
      gain: 2,
      clipMin: -0.5,
      clipMax: 0.75,
      renderMode: "wiggle",
      colormap: "red_white_blue",
      polarity: "reversed"
    }
  });

  assert.deepEqual(interactionToContract("chart-a", "view-a", "panZoom", true), {
    chart_id: "chart-a",
    view_id: "view-a",
    primary_mode: "pan_zoom",
    crosshair_enabled: true
  });
  assert.deepEqual(mergeDisplayTransform(section, { gain: 4, clipMax: 1 }), {
    gain: 4,
    clipMin: -0.5,
    clipMax: 1,
    renderMode: "wiggle",
    colormap: "red-white-blue",
    polarity: "reversed"
  });
});

test("decodeSectionView caches adapted payloads for repeated contract inputs", () => {
  const section = createEncodedSectionView();
  const first = decodeSectionView(section);
  const second = decodeSectionView(section);

  assert.equal(first, second);
});
