import assert from "node:assert/strict";
import test from "node:test";
import {
  CHART_DEFINITIONS,
  CHART_FAMILIES,
  getChartDefinition,
  getChartFamilyDefinition,
  listChartDefinitionsByFamily
} from "../src/chart-registry";

test("chart registry uses unique ids and public surfaces with non-empty constraints", () => {
  assert.equal(new Set(CHART_DEFINITIONS.map((definition) => definition.id)).size, CHART_DEFINITIONS.length);
  assert.equal(
    new Set(CHART_DEFINITIONS.map((definition) => definition.publicSurface)).size,
    CHART_DEFINITIONS.length
  );

  for (const definition of CHART_DEFINITIONS) {
    assert.ok(definition.allowedAssetFamilies.length > 0, `${definition.id} should declare asset families`);
    assert.ok(definition.canonicalBoundaries.length > 0, `${definition.id} should declare canonical boundaries`);
    assert.ok(definition.constraints.length > 0, `${definition.id} should declare constraints`);
    assert.equal(
      new Set(definition.allowedAssetFamilies).size,
      definition.allowedAssetFamilies.length,
      `${definition.id} should not duplicate asset families`
    );
    assert.equal(
      new Set(definition.interactionProfile.tools).size,
      definition.interactionProfile.tools.length,
      `${definition.id} should not duplicate interaction tools`
    );
    assert.equal(
      new Set(definition.interactionProfile.actions).size,
      definition.interactionProfile.actions.length,
      `${definition.id} should not duplicate interaction actions`
    );
    assert.equal(getChartDefinition(definition.id), definition);
  }
});

test("family registry stays aligned with chart definitions", () => {
  for (const family of CHART_FAMILIES) {
    const definitions = listChartDefinitionsByFamily(family.id);

    assert.equal(getChartFamilyDefinition(family.id), family);
    assert.deepEqual(
      definitions.map((definition) => definition.id).sort(),
      [...family.chartIds].sort(),
      `${family.id} should list the same chart ids as the family definition`
    );
    assert.deepEqual(
      [...new Set(definitions.map((definition) => definition.rendererKernel))].sort(),
      [...family.rendererKernels].sort(),
      `${family.id} should expose the same renderer kernels as its chart definitions`
    );
    assert.deepEqual(
      [...new Set(definitions.flatMap((definition) => definition.canonicalBoundaries))].sort(),
      [...family.canonicalBoundaries].sort(),
      `${family.id} should expose the same canonical boundaries as its chart definitions`
    );
    assert.ok(definitions.length > 0, `${family.id} should contain at least one chart definition`);
  }
});

test("chart families partition the registered chart ids without duplication", () => {
  const familyChartIds = CHART_FAMILIES.flatMap((family) => family.chartIds);

  assert.equal(familyChartIds.length, CHART_DEFINITIONS.length);
  assert.equal(new Set(familyChartIds).size, CHART_DEFINITIONS.length);
  assert.deepEqual(
    [...familyChartIds].sort(),
    CHART_DEFINITIONS.map((definition) => definition.id).sort()
  );
});
