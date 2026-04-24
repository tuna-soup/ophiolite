import assert from "node:assert/strict";
import test from "node:test";
import {
  CHART_DEFINITIONS,
  CHART_FAMILIES,
  listChartDefinitionsByFamily,
  listChartDefinitionsBySupportTier
} from "../src/chart-registry";

test("support-tier queries partition chart definitions without duplication", () => {
  const tiers = ["public-launch", "public-adapter", "preview", "internal"] as const;
  const grouped = tiers.flatMap((tier) => listChartDefinitionsBySupportTier(tier).map((definition) => definition.id));

  assert.deepEqual(
    [...grouped].sort(),
    CHART_DEFINITIONS.map((definition) => definition.id).sort()
  );
  assert.equal(new Set(grouped).size, CHART_DEFINITIONS.length);
});

test("chart families do not overstate support tier relative to their charts", () => {
  const rank = {
    "public-launch": 0,
    "public-adapter": 1,
    preview: 2,
    internal: 3
  } as const;

  for (const family of CHART_FAMILIES) {
    const charts = listChartDefinitionsByFamily(family.id);
    const highestChartTier = charts.reduce(
      (highest, chart) => Math.max(highest, rank[chart.supportTier]),
      rank["public-launch"]
    );

    assert.equal(
      rank[family.supportTier],
      highestChartTier,
      `${family.id} family tier should match the most restrictive chart tier in the family`
    );
  }
});

test("launch charts keep required consumer guarantees and backend defaults", () => {
  const launchCharts = listChartDefinitionsBySupportTier("public-launch");

  assert.ok(launchCharts.length > 0);
  for (const definition of launchCharts) {
    const guaranteeIds = definition.consumerGuarantees.map((guarantee) => guarantee.id);

    assert.ok(guaranteeIds.includes("public-package-entrypoint"), `${definition.id} should keep a package guarantee`);
    assert.ok(guaranteeIds.includes("traceboost-demo-consumer"), `${definition.id} should protect TraceBoost usage`);
    assert.ok(guaranteeIds.includes("public-docs-coverage"), `${definition.id} should stay represented in docs`);
    assert.equal(new Set(guaranteeIds).size, guaranteeIds.length, `${definition.id} should not duplicate guarantees`);
    assert.ok(
      definition.rendererBackends.some((backend) => backend.default),
      `${definition.id} should declare a default renderer backend`
    );
  }
});
