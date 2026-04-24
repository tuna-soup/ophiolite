import assert from "node:assert/strict";
import test from "node:test";
import { layoutWellCorrelationPanel } from "../src/layout";
import { normalizeWellPanelModel } from "../src/well-panel-normalize";
import { createWellCorrelationPanelModel } from "../../../tests/fixtures/well-correlation";

test("legacy well correlation panels normalize into chart-core track families", () => {
  const normalized = normalizeWellPanelModel(createWellCorrelationPanelModel());

  assert.ok(normalized);
  assert.deepEqual(
    normalized.wells.map((well) => well.tracks.map((track) => track.kind)),
    [
      ["reference", "scalar", "reference"],
      ["reference", "scalar", "reference"]
    ]
  );
  assert.equal(normalized.wells[0]?.tracks[1]?.kind, "scalar");
  assert.equal(normalized.wells[0]?.tracks[1]?.layers.length, 2);
  assert.equal(normalized.wells[0]?.tracks[2]?.kind, "reference");
  assert.equal(normalized.wells[0]?.tracks[2]?.topOverlays.length, 1);
});

test("well correlation layout keeps a deterministic top-left anchored geometry", () => {
  const normalized = normalizeWellPanelModel(createWellCorrelationPanelModel());
  assert.ok(normalized);

  const layout = layoutWellCorrelationPanel(normalized, 640, 320);

  assert.deepEqual(layout.plotRect, {
    x: 108,
    y: 84,
    width: 556,
    height: 224
  });
  assert.equal(layout.trackHeaderHeight, 44);
  assert.equal(layout.contentWidth, 792);
  assert.equal(layout.viewportWidth, 622);
  assert.deepEqual(layout.scrollbarRect, {
    x: 622,
    y: 84,
    width: 18,
    height: 224
  });
  assert.equal(layout.columns.length, 2);
  assert.deepEqual(layout.columns[0]?.headerRect, {
    x: 108,
    y: 12,
    width: 270,
    height: 24
  });
  assert.deepEqual(layout.columns[1]?.headerRect, {
    x: 404,
    y: 12,
    width: 260,
    height: 24
  });
});
