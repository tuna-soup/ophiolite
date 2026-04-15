import type {
  ProjectDisplayCompatibilityBlockingReasonCode,
  ProjectSurveyDisplayCompatibility,
  ProjectWellboreDisplayCompatibility
} from "./bridge";

function trimOrNull(value: string | null | undefined): string | null {
  const normalized = value?.trim() ?? "";
  return normalized || null;
}

function sameMessage(primary: string, secondary: string | null): boolean {
  return !!secondary && primary.trim().toLowerCase() === secondary.trim().toLowerCase();
}

function appendDetail(primary: string, detail: string | null): string {
  if (!detail || sameMessage(primary, detail)) {
    return primary;
  }
  return `${primary} ${detail}`;
}

export function describeProjectSurveyDisplayCompatibility(
  compatibility: ProjectSurveyDisplayCompatibility | null | undefined
): string | null {
  if (!compatibility) {
    return null;
  }

  const displayCoordinateReferenceId = trimOrNull(compatibility.displayCoordinateReferenceId);
  const sourceCoordinateReferenceId = trimOrNull(compatibility.sourceCoordinateReferenceId);

  switch (compatibility.reasonCode) {
    case "project_display_crs_unresolved":
      return "Project display CRS is unresolved. Choose a project CRS before composing project maps.";
    case "display_crs_unsupported":
      return displayCoordinateReferenceId
        ? `Project display CRS ${displayCoordinateReferenceId} is not yet supported for project map reprojection.`
        : "Project display CRS is not yet supported for project map reprojection.";
    case "source_crs_unknown":
      return "Selected project survey native CRS is unknown, so project map reprojection is unavailable.";
    case "source_crs_unsupported":
      return sourceCoordinateReferenceId
        ? `Selected project survey native CRS ${sourceCoordinateReferenceId} is not yet supported for project map reprojection.`
        : "Selected project survey native CRS is not yet supported for project map reprojection.";
    case "display_equivalent":
      return displayCoordinateReferenceId
        ? `Selected project survey native CRS already matches project display CRS ${displayCoordinateReferenceId}.`
        : "Selected project survey native CRS already matches the project display CRS.";
    case "display_transformed":
      if (sourceCoordinateReferenceId && displayCoordinateReferenceId) {
        return `Selected project survey can be reprojected from ${sourceCoordinateReferenceId} to ${displayCoordinateReferenceId}.`;
      }
      return "Selected project survey can be reprojected into the project display CRS.";
    default:
      return compatibility.reason ?? null;
  }
}

export function projectSurveyDisplayCompatibilityStatusLabel(
  compatibility: ProjectSurveyDisplayCompatibility | null | undefined
): string {
  if (!compatibility) {
    return "unavailable";
  }

  switch (compatibility.reasonCode) {
    case "display_equivalent":
      return "ready - native matches project CRS";
    case "display_transformed":
      return "ready - reprojection available";
    case "project_display_crs_unresolved":
      return "unavailable - project CRS unresolved";
    case "display_crs_unsupported":
      return "unavailable - project CRS unsupported";
    case "source_crs_unknown":
      return "unavailable - survey CRS unknown";
    case "source_crs_unsupported":
      return "unavailable - survey CRS unsupported";
    default:
      return compatibility.canResolveProjectMap ? "ready" : "unavailable";
  }
}

export function describeProjectWellboreDisplayCompatibility(
  compatibility: ProjectWellboreDisplayCompatibility | null | undefined
): string | null {
  if (!compatibility) {
    return null;
  }

  const displayCoordinateReferenceId = trimOrNull(compatibility.displayCoordinateReferenceId);
  const sourceCoordinateReferenceId = trimOrNull(compatibility.sourceCoordinateReferenceId);
  const detail = trimOrNull(compatibility.reason);

  switch (compatibility.reasonCode) {
    case "project_display_crs_unresolved":
      return "Project display CRS is unresolved. Choose a project CRS before resolving well trajectories in project display coordinates.";
    case "resolved_geometry_missing":
      return appendDetail(
        "Selected project wellbore geometry could not be resolved for the current project display CRS.",
        detail
      );
    case "display_equivalent":
      return displayCoordinateReferenceId
        ? `Selected project wellbore native CRS already matches project display CRS ${displayCoordinateReferenceId}.`
        : "Selected project wellbore native CRS already matches the project display CRS.";
    case "display_transformed":
      if (sourceCoordinateReferenceId && displayCoordinateReferenceId) {
        return `Selected project wellbore can be reprojected from ${sourceCoordinateReferenceId} to ${displayCoordinateReferenceId}.`;
      }
      return "Selected project wellbore can be resolved in the project display CRS.";
    case "display_degraded":
      return appendDetail(
        "Selected project wellbore is only partially available in the project display CRS.",
        detail
      );
    case "display_unavailable":
      return appendDetail(
        "Selected project wellbore cannot be resolved in the project display CRS.",
        detail
      );
    case "resolution_error":
      return appendDetail(
        "Selected project wellbore resolution failed for the project display CRS.",
        detail
      );
    default:
      return compatibility.reason ?? null;
  }
}

export function projectWellboreDisplayCompatibilityStatusLabel(
  compatibility: ProjectWellboreDisplayCompatibility | null | undefined
): string {
  if (!compatibility) {
    return "unavailable";
  }

  switch (compatibility.reasonCode) {
    case "display_equivalent":
      return "ready - native matches project CRS";
    case "display_transformed":
      return "ready - reprojection available";
    case "display_degraded":
      return "degraded - partial geometry";
    case "project_display_crs_unresolved":
      return "unavailable - project CRS unresolved";
    case "resolved_geometry_missing":
      return "unavailable - geometry unresolved";
    case "resolution_error":
      return "unavailable - resolution failed";
    case "display_unavailable":
      return "unavailable - no display transform";
    default:
      return compatibility.canResolveProjectMap ? "ready" : "unavailable";
  }
}

export function describeProjectDisplayCompatibilityBlockingReasonCode(
  reasonCode: ProjectDisplayCompatibilityBlockingReasonCode,
  displayCoordinateReferenceId: string | null | undefined
): string {
  switch (reasonCode) {
    case "project_display_crs_unresolved":
      return "Project display CRS is unresolved. Choose a project CRS before relying on project overlays and map composition.";
    case "display_crs_unsupported":
      return displayCoordinateReferenceId
        ? `Project display CRS ${displayCoordinateReferenceId} is not yet supported for project map reprojection.`
        : "Project display CRS is not yet supported for project map reprojection.";
    case "source_crs_unknown":
      return "At least one project survey has no effective native CRS, so project map reprojection remains unavailable.";
    case "source_crs_unsupported":
      return "At least one project survey uses a native CRS that is not yet supported for project map reprojection.";
    case "resolved_geometry_missing":
      return "At least one project wellbore has no resolved geometry in the current project display CRS.";
    case "display_unavailable":
      return "At least one project wellbore cannot be resolved in the current project display CRS.";
    case "resolution_error":
      return "At least one project wellbore failed resolution in the current project display CRS.";
  }
}
