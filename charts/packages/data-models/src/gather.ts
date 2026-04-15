import type { DisplayTransform, SectionPayload, SectionDimensions } from "./seismic";
import { validateGatherPayload, validateSectionPayload, OphioliteSeismicValidationError } from "./ophiolite-seismic-adapter";

export type GatherAxisKind =
  | "offset"
  | "angle"
  | "azimuth"
  | "shot"
  | "receiver"
  | "cmp"
  | "trace-ordinal"
  | "unknown";

export type GatherSampleDomain = "time" | "depth";

export interface GatherPayload {
  label: string;
  gatherAxisKind: GatherAxisKind;
  sampleDomain: GatherSampleDomain;
  horizontalAxis: Float64Array;
  sampleAxis: Float32Array;
  amplitudes: Float32Array;
  dimensions: SectionDimensions;
  units?: {
    horizontal?: string;
    sample?: string;
    amplitude?: string;
  };
  metadata?: {
    storeId?: string;
    derivedFrom?: string;
    notes?: string[];
  };
  displayDefaults?: Partial<DisplayTransform>;
}

export function gatherToSectionPayload(gather: GatherPayload): SectionPayload {
  const gatherIssues = validateGatherPayload(gather);
  if (gatherIssues.length > 0) {
    throw new OphioliteSeismicValidationError(gatherIssues);
  }

  const payload: SectionPayload = {
    axis: "inline",
    coordinate: { index: 0, value: 0 },
    horizontalAxis: gather.horizontalAxis,
    sampleAxis: gather.sampleAxis,
    amplitudes: gather.amplitudes,
    dimensions: gather.dimensions,
    units: gather.units,
    metadata: gather.metadata,
    displayDefaults: gather.displayDefaults,
    presentation: {
      title: gather.label,
      sampleAxisLabel: gather.sampleDomain === "depth" ? "Depth" : "Time",
      topAxisRows: [
        {
          label: horizontalAxisLabel(gather.gatherAxisKind),
          values: gather.horizontalAxis
        }
      ]
    }
  };
  const sectionIssues = validateSectionPayload(payload);
  if (sectionIssues.length > 0) {
    throw new OphioliteSeismicValidationError(sectionIssues);
  }
  return payload;
}

function horizontalAxisLabel(kind: GatherAxisKind): string {
  switch (kind) {
    case "angle":
      return "Angle";
    case "offset":
      return "Offset";
    case "azimuth":
      return "Azimuth";
    case "shot":
      return "Shot";
    case "receiver":
      return "Receiver";
    case "cmp":
      return "CMP";
    case "trace-ordinal":
      return "Trace";
    default:
      return "Gather";
  }
}
