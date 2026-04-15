import assert from "node:assert/strict";
import test from "node:test";
import {
  describeProjectDisplayCompatibilityBlockingReasonCode,
  describeProjectSurveyDisplayCompatibility,
  describeProjectWellboreDisplayCompatibility,
  projectSurveyDisplayCompatibilityStatusLabel,
  projectWellboreDisplayCompatibilityStatusLabel
} from "./project-display-compatibility";

test("describeProjectSurveyDisplayCompatibility prefers stable reasonCode messaging", () => {
  assert.equal(
    describeProjectSurveyDisplayCompatibility({
      canResolveProjectMap: false,
      transformStatus: "display_unavailable",
      sourceCoordinateReferenceId: "EPSG:26917",
      displayCoordinateReferenceId: "EPSG:4326",
      reasonCode: "source_crs_unsupported",
      reason:
        "Survey effective native CRS 'EPSG:26917' is not yet supported for project map reprojection; phase 2 currently accepts only EPSG identifiers."
    }),
    "Selected project survey native CRS EPSG:26917 is not yet supported for project map reprojection."
  );
});

test("describeProjectWellboreDisplayCompatibility appends backend detail for degraded geometry", () => {
  assert.equal(
    describeProjectWellboreDisplayCompatibility({
      canResolveProjectMap: true,
      transformStatus: "display_degraded",
      sourceCoordinateReferenceId: "EPSG:23031",
      displayCoordinateReferenceId: "EPSG:3857",
      reasonCode: "display_degraded",
      reason: "1 of 4 well trajectory stations could not be transformed."
    }),
    "Selected project wellbore is only partially available in the project display CRS. 1 of 4 well trajectory stations could not be transformed."
  );
});

test("describeProjectWellboreDisplayCompatibility falls back to reason text when no reasonCode exists", () => {
  assert.equal(
    describeProjectWellboreDisplayCompatibility({
      canResolveProjectMap: false,
      transformStatus: "display_unavailable",
      sourceCoordinateReferenceId: null,
      displayCoordinateReferenceId: "EPSG:3857",
      reasonCode: null,
      reason: "Legacy compatibility message."
    }),
    "Legacy compatibility message."
  );
});

test("projectSurveyDisplayCompatibilityStatusLabel exposes specific unavailable causes", () => {
  assert.equal(
    projectSurveyDisplayCompatibilityStatusLabel({
      canResolveProjectMap: false,
      transformStatus: "display_unavailable",
      sourceCoordinateReferenceId: null,
      displayCoordinateReferenceId: "EPSG:3857",
      reasonCode: "source_crs_unknown",
      reason: null
    }),
    "unavailable - survey CRS unknown"
  );
});

test("projectWellboreDisplayCompatibilityStatusLabel exposes degraded and missing-geometry cases", () => {
  assert.equal(
    projectWellboreDisplayCompatibilityStatusLabel({
      canResolveProjectMap: true,
      transformStatus: "display_degraded",
      sourceCoordinateReferenceId: "EPSG:23031",
      displayCoordinateReferenceId: "EPSG:3857",
      reasonCode: "display_degraded",
      reason: "partial geometry"
    }),
    "degraded - partial geometry"
  );
  assert.equal(
    projectWellboreDisplayCompatibilityStatusLabel({
      canResolveProjectMap: false,
      transformStatus: "display_unavailable",
      sourceCoordinateReferenceId: null,
      displayCoordinateReferenceId: "EPSG:3857",
      reasonCode: "resolved_geometry_missing",
      reason: null
    }),
    "unavailable - geometry unresolved"
  );
});

test("describeProjectDisplayCompatibilityBlockingReasonCode formats project-wide blockers", () => {
  assert.equal(
    describeProjectDisplayCompatibilityBlockingReasonCode(
      "display_crs_unsupported",
      "EPSG:32632"
    ),
    "Project display CRS EPSG:32632 is not yet supported for project map reprojection."
  );
  assert.equal(
    describeProjectDisplayCompatibilityBlockingReasonCode("resolved_geometry_missing", null),
    "At least one project wellbore has no resolved geometry in the current project display CRS."
  );
});
