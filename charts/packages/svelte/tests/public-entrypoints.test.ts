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

test("root entrypoint only exposes launch chart wrappers", () => {
  assert.match(rootEntry, /SeismicSectionChart/);
  assert.match(rootEntry, /SeismicGatherChart/);
  assert.match(rootEntry, /SurveyMapChart/);
  assert.match(rootEntry, /RockPhysicsCrossplotChart/);
  assert.match(rootEntry, /WellCorrelationPanelChart/);

  assert.doesNotMatch(rootEntry, /AvoResponseChart/);
  assert.doesNotMatch(rootEntry, /AvoInterceptGradientCrossplotChart/);
  assert.doesNotMatch(rootEntry, /AvoChiProjectionHistogramChart/);
  assert.doesNotMatch(rootEntry, /VolumeInterpretationChart/);
  assert.doesNotMatch(rootEntry, /AmplitudeDistributionInspector/);
  assert.doesNotMatch(rootEntry, /WellTieChart/);
});

test("preview and extras entrypoints hold non-launch surfaces", () => {
  assert.match(previewEntry, /AvoResponseChart/);
  assert.match(previewEntry, /VolumeInterpretationChart/);
  assert.match(extrasEntry, /AmplitudeDistributionInspector/);
  assert.match(extrasEntry, /WellTieChart/);
});

test("package exports publish explicit preview, extras, and adapter subpaths", () => {
  assert.ok(packageJson.exports?.["./preview"]);
  assert.ok(packageJson.exports?.["./extras"]);
  assert.ok(packageJson.exports?.["./adapters/ophiolite"]);
});

test("ophiolite adapter entrypoint exposes launch-family adapter helpers", () => {
  assert.match(adapterEntry, /adaptOphioliteSectionViewToSeismicSectionData/);
  assert.match(adapterEntry, /adaptOphioliteGatherViewToSeismicGatherData/);
  assert.match(adapterEntry, /adaptOphioliteSurveyMapToChart/);
  assert.match(adapterEntry, /adaptOphioliteRockPhysicsCrossplotToChart/);
  assert.match(adapterEntry, /adaptOphioliteWellPanelToChart/);
});
