import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import path from "node:path";
import test from "node:test";

const rootDir = path.resolve(import.meta.dir, "..");
const rootEntry = readFileSync(path.join(rootDir, "src", "index.ts"), "utf8");
const previewEntry = readFileSync(path.join(rootDir, "src", "preview.ts"), "utf8");
const extrasEntry = readFileSync(path.join(rootDir, "src", "extras.ts"), "utf8");
const adapterEntry = readFileSync(path.join(rootDir, "src", "adapters", "ophiolite.ts"), "utf8");
const packageJson = JSON.parse(readFileSync(path.join(rootDir, "package.json"), "utf8")) as {
  exports?: Record<string, unknown>;
};

const previewComponentNames = [
  "AvoResponseChart",
  "AvoInterceptGradientCrossplotChart",
  "AvoChiProjectionHistogramChart",
  "VolumeInterpretationChart"
] as const;

const extrasComponentNames = [
  "AmplitudeDistributionChart",
  "AmplitudeDistributionInspector",
  "SpectrumChart",
  "SpectrumInspector",
  "WaveletChart",
  "WellTieChart"
] as const;

test("root entrypoint only exposes launch chart wrappers", () => {
  assert.match(rootEntry, /SeismicSectionChart/);
  assert.match(rootEntry, /SeismicGatherChart/);
  assert.match(rootEntry, /SurveyMapChart/);
  assert.match(rootEntry, /RockPhysicsCrossplotChart/);
  assert.match(rootEntry, /WellCorrelationPanelChart/);
  assert.match(rootEntry, /ChartRendererStatus/);
  assert.match(rootEntry, /ChartRendererTelemetryEvent/);
  assert.match(rootEntry, /SeismicSectionDataSource/);
  assert.match(rootEntry, /SeismicSectionWindowRequest/);

  for (const componentName of [...previewComponentNames, ...extrasComponentNames]) {
    assert.doesNotMatch(rootEntry, new RegExp(`\\b${componentName}\\b`));
  }
  assert.doesNotMatch(rootEntry, /\bDebug\b/);
  assert.doesNotMatch(rootEntry, /\bPreview\b/);
  assert.doesNotMatch(rootEntry, /from\s+["']\.\/(?:preview|extras|contracts)["']/);
});

test("preview and extras entrypoints hold non-launch surfaces", () => {
  for (const componentName of previewComponentNames) {
    assert.match(previewEntry, new RegExp(`\\b${componentName}\\b`));
  }
  for (const componentName of extrasComponentNames) {
    assert.match(extrasEntry, new RegExp(`\\b${componentName}\\b`));
  }
});

test("root wildcard type export stays limited to the local public type barrel", () => {
  const typeStarExports = rootEntry.match(/^export\s+type\s+\*\s+from\s+["'][^"']+["'];$/gm) ?? [];
  assert.deepEqual(typeStarExports, ['export type * from "./types";']);
  assert.doesNotMatch(rootEntry, /^export\s+\*\s+from\s+["'][^"']+["'];$/m);
  assert.doesNotMatch(rootEntry, /from\s+["']\.\/(?:contracts|section-contract-adapter|gather-contract-adapter)["']/);
});

test("package exports publish explicit preview, extras, and adapter subpaths", () => {
  assert.deepEqual(Object.keys(packageJson.exports ?? {}).sort(), [
    ".",
    "./adapters/ophiolite",
    "./contracts",
    "./extras",
    "./preview",
    "./types"
  ]);
  assert.ok(packageJson.exports?.["./preview"]);
  assert.ok(packageJson.exports?.["./extras"]);
  assert.ok(packageJson.exports?.["./adapters/ophiolite"]);
  assert.ok(packageJson.exports?.["./contracts"]);
  assert.ok(packageJson.exports?.["./types"]);
  assert.ok(!Object.keys(packageJson.exports ?? {}).some((subpath) => subpath.includes("*")));
});

test("ophiolite adapter entrypoint exposes launch-family adapter helpers", () => {
  assert.match(adapterEntry, /adaptOphioliteSectionViewToSeismicSectionData/);
  assert.match(adapterEntry, /adaptOphioliteGatherViewToSeismicGatherData/);
  assert.match(adapterEntry, /adaptOphioliteSurveyMapToChart/);
  assert.match(adapterEntry, /adaptOphioliteRockPhysicsCrossplotToChart/);
  assert.match(adapterEntry, /adaptOphioliteWellPanelToChart/);
});

test("ophiolite adapter entrypoint does not leak raw contract internals", () => {
  assert.doesNotMatch(adapterEntry, /^export\s+\*\s+from\s+["']\.\.\/contracts["'];$/m);
  assert.doesNotMatch(adapterEntry, /from\s+["']\.\.\/(?:section-contract-adapter|gather-contract-adapter)["']/);
});
