import assert from "node:assert/strict";
import test from "node:test";
import { buildWorkspaceCoordinateReferenceWarnings } from "./coordinate-reference-warnings";

test("buildWorkspaceCoordinateReferenceWarnings keeps unresolved project CRS as the single project-level action before compatibility exists", () => {
  assert.deepStrictEqual(
    buildWorkspaceCoordinateReferenceWarnings({
      requiresProjectGeospatialSettingsSelection: true,
      suggestedProjectDisplayCoordinateReferenceId: "EPSG:26917",
      canEvaluateProjectDisplayCompatibility: false,
      hasProjectRoot: true,
      projectDisplayCompatibilityBlockingWarnings: [
        "Project display CRS is unresolved. Choose a project CRS before composing project maps."
      ],
      hasSelectedProjectSurvey: true,
      selectedProjectSurveyCanResolveProjectMap: false,
      selectedProjectSurveyReason:
        "Project display CRS is unresolved. Choose a project CRS before composing project maps.",
      hasSelectedProjectWellbore: true,
      selectedProjectWellboreCanResolveProjectMap: false,
      selectedProjectWellboreReason:
        "Project display CRS is unresolved. Choose a project CRS before resolving well trajectories in project display coordinates.",
      hasActiveDataset: false,
      activeStoreAcceptedInNativeEngineering: false,
      displayCoordinateReferenceId: null,
      activeEffectiveNativeCoordinateReferenceId: null,
      activeSurveyMapTransformStatus: null,
      surveyMapError: null,
      surveyMapWellTransformWarnings: []
    }),
    [
      "Project display CRS is unresolved. Choose native engineering coordinates or set EPSG:26917 in Project Settings before relying on project overlays and map composition."
    ]
  );
});

test("buildWorkspaceCoordinateReferenceWarnings dedupes repeated downstream warnings while preserving priority order", () => {
  assert.deepStrictEqual(
    buildWorkspaceCoordinateReferenceWarnings({
      requiresProjectGeospatialSettingsSelection: false,
      suggestedProjectDisplayCoordinateReferenceId: null,
      canEvaluateProjectDisplayCompatibility: true,
      hasProjectRoot: true,
      projectDisplayCompatibilityBlockingWarnings: [
        "Selected project survey cannot be resolved in the project display CRS."
      ],
      hasSelectedProjectSurvey: true,
      selectedProjectSurveyCanResolveProjectMap: false,
      selectedProjectSurveyReason:
        "Selected project survey cannot be resolved in the project display CRS.",
      hasSelectedProjectWellbore: true,
      selectedProjectWellboreCanResolveProjectMap: false,
      selectedProjectWellboreReason:
        "Selected project wellbore cannot be resolved in the project display CRS.",
      hasActiveDataset: true,
      activeStoreAcceptedInNativeEngineering: false,
      displayCoordinateReferenceId: "EPSG:3857",
      activeEffectiveNativeCoordinateReferenceId: "EPSG:4326",
      activeSurveyMapTransformStatus: "display_unavailable",
      surveyMapError: "Selected project survey cannot be resolved in the project display CRS.",
      surveyMapWellTransformWarnings: [
        "Selected project wellbore cannot be resolved in the project display CRS.",
        "2 wells could not be projected into display CRS EPSG:3857."
      ]
    }),
    [
      "Selected project survey cannot be resolved in the project display CRS.",
      "Selected project wellbore cannot be resolved in the project display CRS.",
      "Display CRS EPSG:3857 differs from active survey native CRS EPSG:4326, but no display transform is currently available. To clear this alert, assign survey CRS EPSG:3857, switch display mode to native engineering coordinates, or choose a display CRS that matches EPSG:4326.",
      "2 wells could not be projected into display CRS EPSG:3857."
    ]
  );
});

test("buildWorkspaceCoordinateReferenceWarnings uses the combined display/native warning when display CRS exists but active native CRS is missing", () => {
  assert.deepStrictEqual(
    buildWorkspaceCoordinateReferenceWarnings({
      requiresProjectGeospatialSettingsSelection: false,
      suggestedProjectDisplayCoordinateReferenceId: null,
      canEvaluateProjectDisplayCompatibility: true,
      hasProjectRoot: false,
      projectDisplayCompatibilityBlockingWarnings: [],
      hasSelectedProjectSurvey: false,
      selectedProjectSurveyCanResolveProjectMap: null,
      selectedProjectSurveyReason: null,
      hasSelectedProjectWellbore: false,
      selectedProjectWellboreCanResolveProjectMap: null,
      selectedProjectWellboreReason: null,
      hasActiveDataset: true,
      activeStoreAcceptedInNativeEngineering: false,
      displayCoordinateReferenceId: "EPSG:3857",
      activeEffectiveNativeCoordinateReferenceId: null,
      activeSurveyMapTransformStatus: "native_only",
      surveyMapError: null,
      surveyMapWellTransformWarnings: []
    }),
    [
      "Display CRS EPSG:3857 is set, but the active survey has no effective native CRS. To clear this alert, assign a matching survey CRS or switch display mode to native engineering coordinates."
    ]
  );
});

test("buildWorkspaceCoordinateReferenceWarnings stays quiet for native-engineering sessions with no active native CRS", () => {
  assert.deepStrictEqual(
    buildWorkspaceCoordinateReferenceWarnings({
      requiresProjectGeospatialSettingsSelection: false,
      suggestedProjectDisplayCoordinateReferenceId: null,
      canEvaluateProjectDisplayCompatibility: false,
      hasProjectRoot: false,
      projectDisplayCompatibilityBlockingWarnings: [],
      hasSelectedProjectSurvey: false,
      selectedProjectSurveyCanResolveProjectMap: null,
      selectedProjectSurveyReason: null,
      hasSelectedProjectWellbore: false,
      selectedProjectWellboreCanResolveProjectMap: null,
      selectedProjectWellboreReason: null,
      hasActiveDataset: true,
      activeStoreAcceptedInNativeEngineering: false,
      displayCoordinateReferenceId: null,
      activeEffectiveNativeCoordinateReferenceId: null,
      activeSurveyMapTransformStatus: null,
      surveyMapError: null,
      surveyMapWellTransformWarnings: []
    }),
    []
  );
});

test("buildWorkspaceCoordinateReferenceWarnings surfaces project-wide compatibility blockers before selected-item details", () => {
  assert.deepStrictEqual(
    buildWorkspaceCoordinateReferenceWarnings({
      requiresProjectGeospatialSettingsSelection: false,
      suggestedProjectDisplayCoordinateReferenceId: null,
      canEvaluateProjectDisplayCompatibility: true,
      hasProjectRoot: true,
      projectDisplayCompatibilityBlockingWarnings: [
        "2 project surveys cannot be transformed into display CRS EPSG:3857.",
        "1 project well cannot be resolved in display CRS EPSG:3857."
      ],
      hasSelectedProjectSurvey: true,
      selectedProjectSurveyCanResolveProjectMap: false,
      selectedProjectSurveyReason:
        "Selected project survey cannot be resolved in the project display CRS.",
      hasSelectedProjectWellbore: true,
      selectedProjectWellboreCanResolveProjectMap: false,
      selectedProjectWellboreReason:
        "Selected project wellbore cannot be resolved in the project display CRS.",
      hasActiveDataset: false,
      activeStoreAcceptedInNativeEngineering: false,
      displayCoordinateReferenceId: "EPSG:3857",
      activeEffectiveNativeCoordinateReferenceId: "EPSG:4326",
      activeSurveyMapTransformStatus: "display_unavailable",
      surveyMapError: null,
      surveyMapWellTransformWarnings: []
    }),
    [
      "2 project surveys cannot be transformed into display CRS EPSG:3857.",
      "1 project well cannot be resolved in display CRS EPSG:3857.",
      "Selected project survey cannot be resolved in the project display CRS.",
      "Selected project wellbore cannot be resolved in the project display CRS."
    ]
  );
});

test("buildWorkspaceCoordinateReferenceWarnings suppresses active-survey CRS alerts after native engineering is accepted for the active store", () => {
  assert.deepStrictEqual(
    buildWorkspaceCoordinateReferenceWarnings({
      requiresProjectGeospatialSettingsSelection: false,
      suggestedProjectDisplayCoordinateReferenceId: null,
      canEvaluateProjectDisplayCompatibility: false,
      hasProjectRoot: false,
      projectDisplayCompatibilityBlockingWarnings: [],
      hasSelectedProjectSurvey: false,
      selectedProjectSurveyCanResolveProjectMap: null,
      selectedProjectSurveyReason: null,
      hasSelectedProjectWellbore: false,
      selectedProjectWellboreCanResolveProjectMap: null,
      selectedProjectWellboreReason: null,
      hasActiveDataset: true,
      activeStoreAcceptedInNativeEngineering: true,
      displayCoordinateReferenceId: "EPSG:3857",
      activeEffectiveNativeCoordinateReferenceId: null,
      activeSurveyMapTransformStatus: "native_only",
      surveyMapError: null,
      surveyMapWellTransformWarnings: []
    }),
    []
  );
});
