export interface WorkspaceCoordinateReferenceWarningsInput {
  requiresProjectGeospatialSettingsSelection: boolean;
  suggestedProjectDisplayCoordinateReferenceId: string | null;
  canEvaluateProjectDisplayCompatibility: boolean;
  hasProjectRoot: boolean;
  projectDisplayCompatibilityBlockingWarnings: string[];
  hasSelectedProjectSurvey: boolean;
  selectedProjectSurveyCanResolveProjectMap: boolean | null;
  selectedProjectSurveyReason: string | null;
  hasSelectedProjectWellbore: boolean;
  selectedProjectWellboreCanResolveProjectMap: boolean | null;
  selectedProjectWellboreReason: string | null;
  hasActiveDataset: boolean;
  displayCoordinateReferenceId: string | null;
  activeEffectiveNativeCoordinateReferenceId: string | null;
  activeSurveyMapTransformStatus: string | null;
  surveyMapError: string | null;
  surveyMapWellTransformWarnings: string[];
}

function pushUniqueWarning(warnings: string[], warning: string | null | undefined): void {
  const normalizedWarning = warning?.trim() ?? "";
  if (!normalizedWarning || warnings.includes(normalizedWarning)) {
    return;
  }
  warnings.push(normalizedWarning);
}

function unresolvedProjectDisplayCoordinateReferenceWarning(
  suggestedProjectDisplayCoordinateReferenceId: string | null
): string {
  return suggestedProjectDisplayCoordinateReferenceId
    ? `Project display CRS is unresolved. Choose native engineering coordinates or set ${suggestedProjectDisplayCoordinateReferenceId} in Project Settings before relying on project overlays and map composition.`
    : "Project display CRS is unresolved. Choose native engineering coordinates or a specific CRS in Project Settings before relying on project overlays and map composition.";
}

export function buildWorkspaceCoordinateReferenceWarnings(
  input: WorkspaceCoordinateReferenceWarningsInput
): string[] {
  const warnings: string[] = [];

  if (input.requiresProjectGeospatialSettingsSelection) {
    pushUniqueWarning(
      warnings,
      unresolvedProjectDisplayCoordinateReferenceWarning(
        input.suggestedProjectDisplayCoordinateReferenceId
      )
    );
  }

  if (input.hasProjectRoot && input.canEvaluateProjectDisplayCompatibility) {
    for (const warning of input.projectDisplayCompatibilityBlockingWarnings) {
      pushUniqueWarning(warnings, warning);
    }
  }

  if (
    input.hasProjectRoot &&
    input.canEvaluateProjectDisplayCompatibility &&
    input.hasSelectedProjectSurvey &&
    input.selectedProjectSurveyCanResolveProjectMap === false
  ) {
    pushUniqueWarning(warnings, input.selectedProjectSurveyReason);
  }

  if (
    input.hasProjectRoot &&
    input.canEvaluateProjectDisplayCompatibility &&
    input.hasSelectedProjectWellbore &&
    input.selectedProjectWellboreCanResolveProjectMap === false
  ) {
    pushUniqueWarning(warnings, input.selectedProjectWellboreReason);
  }

  if (!input.hasActiveDataset) {
    return warnings;
  }

  if (input.displayCoordinateReferenceId && !input.activeEffectiveNativeCoordinateReferenceId) {
    pushUniqueWarning(
      warnings,
      `Display CRS ${input.displayCoordinateReferenceId} is set, but the active survey has no effective native CRS. Assign a native CRS override before relying on cross-survey map alignment.`
    );
  } else if (!input.activeEffectiveNativeCoordinateReferenceId) {
    pushUniqueWarning(
      warnings,
      "Active survey native CRS is unknown. Assign an override before relying on cross-survey map alignment."
    );
  } else if (
    input.displayCoordinateReferenceId &&
    input.displayCoordinateReferenceId.toLowerCase() !==
      input.activeEffectiveNativeCoordinateReferenceId.toLowerCase()
  ) {
    const transformStatus = input.activeSurveyMapTransformStatus ?? "native_only";
    if (transformStatus === "display_unavailable") {
      pushUniqueWarning(
        warnings,
        `Display CRS ${input.displayCoordinateReferenceId} differs from active survey native CRS ${input.activeEffectiveNativeCoordinateReferenceId}, but no display transform is currently available.`
      );
    } else if (transformStatus === "display_degraded") {
      pushUniqueWarning(
        warnings,
        `Display CRS ${input.displayCoordinateReferenceId} differs from active survey native CRS ${input.activeEffectiveNativeCoordinateReferenceId}. The current map preview uses a degraded transform.`
      );
    } else if (transformStatus === "native_only") {
      pushUniqueWarning(
        warnings,
        `Display CRS ${input.displayCoordinateReferenceId} differs from active survey native CRS ${input.activeEffectiveNativeCoordinateReferenceId}. The current map preview is still in native coordinates.`
      );
    }
  }

  pushUniqueWarning(warnings, input.surveyMapError);
  for (const warning of input.surveyMapWellTransformWarnings) {
    pushUniqueWarning(warnings, warning);
  }

  return warnings;
}
