import assert from "node:assert/strict";
import test from "node:test";
import { buildStartupSetupBlockers } from "./startup-setup";

test("buildStartupSetupBlockers allows session-only startup without a project root", () => {
  assert.deepStrictEqual(
    buildStartupSetupBlockers({
      workspaceReady: true,
      hasProjectRoot: false,
      projectGeospatialSettingsResolved: true,
      hasActiveStore: true,
      activeEffectiveNativeCoordinateReferenceId: "WGS84",
      activeEffectiveNativeCoordinateReferenceName: "WGS84"
    }),
    []
  );
});

test("buildStartupSetupBlockers does not block project-backed startup when display settings are still loading", () => {
  assert.deepStrictEqual(
    buildStartupSetupBlockers({
      workspaceReady: true,
      hasProjectRoot: true,
      projectGeospatialSettingsResolved: false,
      hasActiveStore: false,
      activeEffectiveNativeCoordinateReferenceId: null,
      activeEffectiveNativeCoordinateReferenceName: null
    }),
    []
  );
});

test("buildStartupSetupBlockers does not block section viewing when the active survey native CRS is unknown", () => {
  assert.deepStrictEqual(
    buildStartupSetupBlockers({
      workspaceReady: true,
      hasProjectRoot: false,
      projectGeospatialSettingsResolved: true,
      hasActiveStore: true,
      activeEffectiveNativeCoordinateReferenceId: null,
      activeEffectiveNativeCoordinateReferenceName: null
    }),
    []
  );
});
