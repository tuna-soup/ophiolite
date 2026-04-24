import assert from "node:assert/strict";
import test from "node:test";
import { mapNativeDepthToPanelDepth, mapPanelDepthToNativeDepth } from "../src/depth-mapping";
import { createDepthMappingSamples } from "../../../tests/fixtures/well-correlation";

test("depth mapping interpolates and extrapolates native depth values", () => {
  const mapping = createDepthMappingSamples();

  assert.equal(mapNativeDepthToPanelDepth(mapping, 1050), 40);
  assert.equal(mapNativeDepthToPanelDepth(mapping, 1175), 140);
  assert.equal(mapNativeDepthToPanelDepth(mapping, 950), -40);
  assert.equal(mapNativeDepthToPanelDepth(mapping, 1300), 240);
});

test("depth mapping round-trips panel depth values through the inverse mapping", () => {
  const mapping = createDepthMappingSamples();

  assert.equal(mapPanelDepthToNativeDepth(mapping, 140), 1175);
  assert.equal(mapPanelDepthToNativeDepth(mapping, -40), 950);
  assert.equal(mapPanelDepthToNativeDepth(mapping, 240), 1300);
});

test("empty and single-sample mappings fall back predictably", () => {
  assert.equal(mapNativeDepthToPanelDepth([], 1234), 1234);
  assert.equal(mapPanelDepthToNativeDepth([], 567), 567);

  const single = [{ nativeDepth: 1000, panelDepth: 25 }];
  assert.equal(mapNativeDepthToPanelDepth(single, 1111), 25);
  assert.equal(mapPanelDepthToNativeDepth(single, 1111), 1000);
});
