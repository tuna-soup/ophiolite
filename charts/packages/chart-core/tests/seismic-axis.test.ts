import assert from "node:assert/strict";
import test from "node:test";
import {
  buildSeismicTickIndices,
  buildSeismicTopAxisRows,
  isArbitrarySeismicSection,
  resolveSeismicSampleAxisTitle,
  resolveSeismicSectionTitle
} from "../src/seismic-axis";
import { createArbitrarySectionPayload, createSectionPayload } from "../../../tests/fixtures/seismic";

test("buildSeismicTickIndices spans the visible extent without duplicates", () => {
  assert.deepEqual(buildSeismicTickIndices(10, 15, 10), [10, 11, 12, 13, 14]);
  assert.deepEqual(buildSeismicTickIndices(4, 5, 1), [4]);
  assert.deepEqual(buildSeismicTickIndices(7, 7, 6), []);
});

test("arbitrary sections expose trace, inline, and xline presentation rows", () => {
  const section = createArbitrarySectionPayload();
  const rows = buildSeismicTopAxisRows(section);

  assert.equal(isArbitrarySeismicSection(section), true);
  assert.deepEqual(rows.map((row) => row.label), ["Trace", "IL", "XL"]);
  assert.equal(resolveSeismicSectionTitle(section), "Arbitrary Traverse");
  assert.equal(resolveSeismicSampleAxisTitle(section), "TWT (ms)");
});

test("standard sections use the axial title and a single top axis row", () => {
  const section = createSectionPayload({
    axis: "xline",
    coordinate: { index: 4, value: 25.4 },
    units: { sample: "ms" }
  });
  const rows = buildSeismicTopAxisRows(section);

  assert.equal(isArbitrarySeismicSection(section), false);
  assert.deepEqual(rows.map((row) => row.label), ["Inline"]);
  assert.equal(resolveSeismicSectionTitle(section), "Xline: 25.4");
  assert.equal(resolveSeismicSampleAxisTitle(section), "Sample (ms)");
});
