import assert from "node:assert/strict";
import test from "node:test";
import { shouldPromptForMissingNativeCoordinateReference } from "./missing-native-coordinate-reference-prompt";

test("shouldPromptForMissingNativeCoordinateReference prompts for an active interactive dataset with no effective CRS", () => {
  assert.equal(
    shouldPromptForMissingNativeCoordinateReference({
      makeActive: true,
      promptRequested: true,
      restoringWorkspace: false,
      storePath: "/tmp/example.tbvol",
      effectiveCoordinateReferenceId: null,
      effectiveCoordinateReferenceName: null,
      acceptedNativeEngineeringStorePaths: new Set()
    }),
    true
  );
});

test("shouldPromptForMissingNativeCoordinateReference skips restored workspace datasets", () => {
  assert.equal(
    shouldPromptForMissingNativeCoordinateReference({
      makeActive: true,
      promptRequested: true,
      restoringWorkspace: true,
      storePath: "/tmp/example.tbvol",
      effectiveCoordinateReferenceId: null,
      effectiveCoordinateReferenceName: null,
      acceptedNativeEngineeringStorePaths: new Set()
    }),
    false
  );
});

test("shouldPromptForMissingNativeCoordinateReference skips dismissed datasets and resolved CRS cases", () => {
  assert.equal(
    shouldPromptForMissingNativeCoordinateReference({
      makeActive: true,
      promptRequested: true,
      restoringWorkspace: false,
      storePath: "/tmp/example.tbvol",
      effectiveCoordinateReferenceId: null,
      effectiveCoordinateReferenceName: null,
      acceptedNativeEngineeringStorePaths: new Set(["/tmp/example.tbvol"])
    }),
    false
  );

  assert.equal(
    shouldPromptForMissingNativeCoordinateReference({
      makeActive: true,
      promptRequested: true,
      restoringWorkspace: false,
      storePath: "/tmp/example.tbvol",
      effectiveCoordinateReferenceId: "EPSG:32632",
      effectiveCoordinateReferenceName: "WGS 84 / UTM zone 32N",
      acceptedNativeEngineeringStorePaths: new Set()
    }),
    false
  );
});
