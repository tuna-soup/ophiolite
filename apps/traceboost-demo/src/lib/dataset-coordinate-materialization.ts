function normalizeCoordinateReferenceId(value: string | null | undefined): string | null {
  const normalized = value?.trim() ?? "";
  return normalized || null;
}

export type DatasetCoordinateMaterializationReasonCode =
  | "missing_dataset"
  | "missing_display_crs"
  | "missing_native_crs"
  | "already_equivalent"
  | "transform_unavailable"
  | "backend_unavailable";

export interface DatasetCoordinateMaterializationAvailabilityInput {
  hasActiveDataset: boolean;
  displayCoordinateReferenceId: string | null;
  activeEffectiveNativeCoordinateReferenceId: string | null;
  activeSurveyMapTransformStatus: string | null;
}

export interface DatasetCoordinateMaterializationAvailability {
  reasonCode: DatasetCoordinateMaterializationReasonCode;
  title: string;
  message: string;
}

export function describeDatasetCoordinateMaterializationAvailability(
  input: DatasetCoordinateMaterializationAvailabilityInput
): DatasetCoordinateMaterializationAvailability {
  const displayCoordinateReferenceId = normalizeCoordinateReferenceId(
    input.displayCoordinateReferenceId
  );
  const activeEffectiveNativeCoordinateReferenceId = normalizeCoordinateReferenceId(
    input.activeEffectiveNativeCoordinateReferenceId
  );

  if (!input.hasActiveDataset) {
    return {
      reasonCode: "missing_dataset",
      title: "No active survey dataset",
      message: "Open a seismic volume before materializing a reprojected copy."
    };
  }

  if (!displayCoordinateReferenceId) {
    return {
      reasonCode: "missing_display_crs",
      title: "Display CRS required",
      message: "Choose a display CRS before materializing a reprojected copy."
    };
  }

  if (!activeEffectiveNativeCoordinateReferenceId) {
    return {
      reasonCode: "missing_native_crs",
      title: "Survey CRS required",
      message:
        "Assign the survey CRS first. That updates how TraceBoost interprets the current store's raw X/Y values without rewriting the dataset."
    };
  }

  if (
    displayCoordinateReferenceId.toLowerCase() ===
    activeEffectiveNativeCoordinateReferenceId.toLowerCase()
  ) {
    return {
      reasonCode: "already_equivalent",
      title: "No copy needed",
      message: `The active survey CRS already matches display CRS ${displayCoordinateReferenceId}. Assigning the survey CRS is enough; a reprojected copy would not change the stored coordinates.`
    };
  }

  if (input.activeSurveyMapTransformStatus === "display_unavailable") {
    return {
      reasonCode: "transform_unavailable",
      title: "No display transform available",
      message: `No display transform is currently available from ${activeEffectiveNativeCoordinateReferenceId} to ${displayCoordinateReferenceId}, so TraceBoost cannot stage a reprojected copy for this survey.`
    };
  }

  return {
    reasonCode: "backend_unavailable",
    title: "Reprojected copy not available yet",
    message: `Assigning the survey CRS updates interpretation only. Writing a new seismic volume store in display CRS ${displayCoordinateReferenceId} is not available yet.`
  };
}
