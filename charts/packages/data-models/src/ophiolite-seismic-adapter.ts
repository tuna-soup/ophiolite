import type {
  GatherAxisKind,
  GatherSampleDomain,
  GatherView,
  SectionColorMap,
  SectionPolarity,
  SectionRenderMode,
  SectionView
} from "@ophiolite/contracts";
import type { GatherPayload } from "./gather";
import type { DisplayTransform, SectionPayload } from "./seismic";

export type EncodedSeismicBytes = number[] | Uint8Array;

export interface OphioliteEncodedSectionView extends Omit<
  SectionView,
  "horizontal_axis_f64le" | "inline_axis_f64le" | "xline_axis_f64le" | "sample_axis_f32le" | "amplitudes_f32le"
> {
  horizontal_axis_f64le: EncodedSeismicBytes;
  inline_axis_f64le: EncodedSeismicBytes | null;
  xline_axis_f64le: EncodedSeismicBytes | null;
  sample_axis_f32le: EncodedSeismicBytes;
  amplitudes_f32le: EncodedSeismicBytes;
  logical_dimensions?: {
    traces: number;
    samples: number;
  };
  window?: {
    trace_start: number;
    trace_end: number;
    sample_start: number;
    sample_end: number;
    lod?: number;
  };
}

export interface OphioliteEncodedGatherView extends Omit<
  GatherView,
  "horizontal_axis_f64le" | "sample_axis_f32le" | "amplitudes_f32le"
> {
  horizontal_axis_f64le: EncodedSeismicBytes;
  sample_axis_f32le: EncodedSeismicBytes;
  amplitudes_f32le: EncodedSeismicBytes;
}

export interface SeismicValidationIssue {
  code: string;
  path: string;
  message: string;
}

export class OphioliteSeismicValidationError extends Error {
  readonly issues: SeismicValidationIssue[];

  constructor(issues: SeismicValidationIssue[]) {
    super([
      "Seismic payload validation failed.",
      ...issues.map((entry) => `- [${entry.code}] ${entry.path}: ${entry.message}`)
    ].join("\n"));
    this.name = "OphioliteSeismicValidationError";
    this.issues = issues;
  }
}

export function adaptOphioliteSectionViewToPayload(contract: OphioliteEncodedSectionView): SectionPayload {
  const payload: SectionPayload = {
    axis: contract.axis,
    coordinate: {
      index: contract.coordinate.index,
      value: contract.coordinate.value
    },
    horizontalAxis: decodeFloat64(contract.horizontal_axis_f64le, "horizontal_axis_f64le"),
    inlineAxis: decodeOptionalFloat64(contract.inline_axis_f64le, "inline_axis_f64le"),
    xlineAxis: decodeOptionalFloat64(contract.xline_axis_f64le, "xline_axis_f64le"),
    sampleAxis: decodeFloat32(contract.sample_axis_f32le, "sample_axis_f32le"),
    amplitudes: decodeFloat32(contract.amplitudes_f32le, "amplitudes_f32le"),
    dimensions: {
      traces: contract.traces,
      samples: contract.samples
    },
    units: contract.units
      ? {
          horizontal: contract.units.horizontal ?? undefined,
          sample: contract.units.sample ?? undefined,
          amplitude: contract.units.amplitude ?? undefined
        }
      : undefined,
    metadata: contract.metadata
      ? {
          storeId: contract.metadata.store_id ?? undefined,
          derivedFrom: contract.metadata.derived_from ?? undefined,
          notes: contract.metadata.notes
        }
      : undefined,
    logicalDimensions: contract.logical_dimensions
      ? {
          traces: contract.logical_dimensions.traces,
          samples: contract.logical_dimensions.samples
        }
      : undefined,
    window: contract.window
      ? {
          traceStart: contract.window.trace_start,
          traceEnd: contract.window.trace_end,
          sampleStart: contract.window.sample_start,
          sampleEnd: contract.window.sample_end,
          lod: contract.window.lod
        }
      : undefined,
    presentation: decodeSectionPresentation(contract),
    displayDefaults: contract.display_defaults
      ? {
          gain: contract.display_defaults.gain,
          clipMin: contract.display_defaults.clip_min ?? undefined,
          clipMax: contract.display_defaults.clip_max ?? undefined,
          renderMode: fromContractRenderMode(contract.display_defaults.render_mode),
          colormap: fromContractColorMap(contract.display_defaults.colormap),
          polarity: fromContractPolarity(contract.display_defaults.polarity)
        }
      : undefined
  };

  assertValid("section", validateSectionPayload(payload));
  return payload;
}

export function adaptOphioliteGatherViewToPayload(contract: OphioliteEncodedGatherView): GatherPayload {
  const payload: GatherPayload = {
    label: contract.label,
    gatherAxisKind: fromContractGatherAxisKind(contract.gather_axis_kind),
    sampleDomain: fromContractGatherSampleDomain(contract.sample_domain),
    horizontalAxis: decodeFloat64(contract.horizontal_axis_f64le, "horizontal_axis_f64le"),
    sampleAxis: decodeFloat32(contract.sample_axis_f32le, "sample_axis_f32le"),
    amplitudes: decodeFloat32(contract.amplitudes_f32le, "amplitudes_f32le"),
    dimensions: {
      traces: contract.traces,
      samples: contract.samples
    },
    units: contract.units
      ? {
          horizontal: contract.units.horizontal ?? undefined,
          sample: contract.units.sample ?? undefined,
          amplitude: contract.units.amplitude ?? undefined
        }
      : undefined,
    metadata: contract.metadata
      ? {
          storeId: contract.metadata.store_id ?? undefined,
          derivedFrom: contract.metadata.derived_from ?? undefined,
          notes: contract.metadata.notes
        }
      : undefined,
    displayDefaults: contract.display_defaults
      ? {
          gain: contract.display_defaults.gain,
          clipMin: contract.display_defaults.clip_min ?? undefined,
          clipMax: contract.display_defaults.clip_max ?? undefined,
          renderMode: fromContractRenderMode(contract.display_defaults.render_mode),
          colormap: fromContractColorMap(contract.display_defaults.colormap),
          polarity: fromContractPolarity(contract.display_defaults.polarity)
        }
      : undefined
  };

  assertValid("gather", validateGatherPayload(payload));
  return payload;
}

export function validateSectionPayload(section: SectionPayload): SeismicValidationIssue[] {
  const issues: SeismicValidationIssue[] = [];
  validateDimensions(section.dimensions.traces, section.dimensions.samples, "dimensions", issues);
  validateFiniteNumber(section.coordinate.index, "coordinate.index", issues);
  validateFiniteNumber(section.coordinate.value, "coordinate.value", issues);
  validateArray(section.horizontalAxis, "horizontalAxis", issues, {
    expectedLength: section.dimensions.traces
  });
  validateArray(section.inlineAxis, "inlineAxis", issues, {
    expectedLength: section.dimensions.traces
  });
  validateArray(section.xlineAxis, "xlineAxis", issues, {
    expectedLength: section.dimensions.traces
  });
  validateArray(section.sampleAxis, "sampleAxis", issues, {
    expectedLength: section.dimensions.samples,
    strictlyIncreasing: true
  });
  validateArray(section.amplitudes, "amplitudes", issues, {
    expectedLength: section.dimensions.traces * section.dimensions.samples
  });

  if (section.overlay) {
    if (section.overlay.width !== section.dimensions.traces || section.overlay.height !== section.dimensions.samples) {
      issues.push(
        issue(
          "overlay-dimension-mismatch",
          "overlay",
          "Overlay width/height must match section traces/samples."
        )
      );
    }
    if (section.overlay.values.length !== section.overlay.width * section.overlay.height) {
      issues.push(
        issue(
          "overlay-value-length-mismatch",
          "overlay.values",
          "Overlay value count must equal overlay width * height."
        )
      );
    }
    if (
      section.overlay.opacity !== undefined &&
      (!Number.isFinite(section.overlay.opacity) || section.overlay.opacity < 0 || section.overlay.opacity > 1)
    ) {
      issues.push(issue("invalid-overlay-opacity", "overlay.opacity", "Overlay opacity must be between 0 and 1."));
    }
  }

  section.presentation?.topAxisRows?.forEach((row, index) => {
    validateArray(row.values, `presentation.topAxisRows[${index}].values`, issues, {
      expectedLength: section.dimensions.traces
    });
  });

  validateDisplayTransform(section.displayDefaults, "displayDefaults", issues);

  if (section.window) {
    if (
      !Number.isFinite(section.window.traceStart) ||
      !Number.isFinite(section.window.traceEnd) ||
      !Number.isFinite(section.window.sampleStart) ||
      !Number.isFinite(section.window.sampleEnd)
    ) {
      issues.push(issue("invalid-section-window", "window", "Window bounds must be finite numbers."));
    }
    const logical = section.logicalDimensions ?? section.dimensions;
    if (
      section.window.traceStart < 0 ||
      section.window.traceEnd > logical.traces ||
      section.window.traceEnd - section.window.traceStart !== section.dimensions.traces
    ) {
      issues.push(
        issue(
          "section-window-trace-mismatch",
          "window.trace",
          "Window trace bounds must match the loaded trace dimension and stay within logical dimensions."
        )
      );
    }
    if (
      section.window.sampleStart < 0 ||
      section.window.sampleEnd > logical.samples ||
      section.window.sampleEnd - section.window.sampleStart !== section.dimensions.samples
    ) {
      issues.push(
        issue(
          "section-window-sample-mismatch",
          "window.sample",
          "Window sample bounds must match the loaded sample dimension and stay within logical dimensions."
        )
      );
    }
  }
  return issues;
}

export function validateGatherPayload(gather: GatherPayload): SeismicValidationIssue[] {
  const issues: SeismicValidationIssue[] = [];
  validateDimensions(gather.dimensions.traces, gather.dimensions.samples, "dimensions", issues);
  validateArray(gather.horizontalAxis, "horizontalAxis", issues, {
    expectedLength: gather.dimensions.traces
  });
  validateArray(gather.sampleAxis, "sampleAxis", issues, {
    expectedLength: gather.dimensions.samples,
    strictlyIncreasing: true
  });
  validateArray(gather.amplitudes, "amplitudes", issues, {
    expectedLength: gather.dimensions.traces * gather.dimensions.samples
  });
  validateDisplayTransform(gather.displayDefaults, "displayDefaults", issues);
  return issues;
}

export function decodeFloat32(bytes: EncodedSeismicBytes, path = "bytes"): Float32Array {
  const array = normalizeBytes(bytes);
  if (array.byteLength === 0) {
    return new Float32Array(0);
  }
  if (array.byteLength % Float32Array.BYTES_PER_ELEMENT !== 0) {
    throw new OphioliteSeismicValidationError([
      issue("invalid-byte-length", path, "Byte length must be divisible by 4 for Float32 decoding.")
    ]);
  }
  return new Float32Array(array.buffer, array.byteOffset, array.byteLength / Float32Array.BYTES_PER_ELEMENT);
}

export function decodeFloat64(bytes: EncodedSeismicBytes, path = "bytes"): Float64Array {
  const array = normalizeBytes(bytes);
  if (array.byteLength === 0) {
    return new Float64Array(0);
  }
  if (array.byteLength % Float64Array.BYTES_PER_ELEMENT !== 0) {
    throw new OphioliteSeismicValidationError([
      issue("invalid-byte-length", path, "Byte length must be divisible by 8 for Float64 decoding.")
    ]);
  }
  return new Float64Array(array.buffer, array.byteOffset, array.byteLength / Float64Array.BYTES_PER_ELEMENT);
}

export function decodeOptionalFloat64(
  bytes: EncodedSeismicBytes | null | undefined,
  path = "bytes"
): Float64Array | undefined {
  if (!bytes) {
    return undefined;
  }
  return decodeFloat64(bytes, path);
}

export function fromContractRenderMode(value: SectionRenderMode): DisplayTransform["renderMode"] {
  return value === "wiggle" ? "wiggle" : "heatmap";
}

export function fromContractColorMap(value: SectionColorMap): DisplayTransform["colormap"] {
  return value === "red_white_blue" ? "red-white-blue" : "grayscale";
}

export function fromContractPolarity(value: SectionPolarity): DisplayTransform["polarity"] {
  return value === "reversed" ? "reversed" : "normal";
}

function fromContractGatherAxisKind(value: GatherAxisKind): GatherPayload["gatherAxisKind"] {
  switch (value) {
    case "offset":
      return "offset";
    case "angle":
      return "angle";
    case "azimuth":
      return "azimuth";
    case "shot":
      return "shot";
    case "receiver":
      return "receiver";
    case "cmp":
      return "cmp";
    case "trace_ordinal":
      return "trace-ordinal";
    default:
      return "unknown";
  }
}

function fromContractGatherSampleDomain(value: GatherSampleDomain): GatherPayload["sampleDomain"] {
  return value === "depth" ? "depth" : "time";
}

function decodeSectionPresentation(contract: OphioliteEncodedSectionView): SectionPayload["presentation"] | undefined {
  const sampleAxisLabel = sectionSampleAxisLabel(contract);
  if (!sampleAxisLabel) {
    return undefined;
  }
  return { sampleAxisLabel };
}

function sectionSampleAxisLabel(contract: OphioliteEncodedSectionView): string | undefined {
  const notes = contract.metadata?.notes ?? [];
  for (const note of notes) {
    if (note === "sample_domain:depth") {
      return "Depth";
    }
    if (note === "sample_domain:time") {
      return "Time";
    }
  }
  const unit = contract.units?.sample?.toLowerCase();
  if (!unit) {
    return undefined;
  }
  if (unit === "ms" || unit === "s") {
    return "Time";
  }
  if (unit === "m" || unit === "ft") {
    return "Depth";
  }
  return undefined;
}

function validateDimensions(
  traces: number,
  samples: number,
  path: string,
  issues: SeismicValidationIssue[]
): void {
  if (!Number.isInteger(traces) || traces <= 0) {
    issues.push(issue("invalid-trace-count", `${path}.traces`, "Trace count must be a positive integer."));
  }
  if (!Number.isInteger(samples) || samples <= 0) {
    issues.push(issue("invalid-sample-count", `${path}.samples`, "Sample count must be a positive integer."));
  }
}

function validateArray(
  values: ArrayLike<number> | undefined,
  path: string,
  issues: SeismicValidationIssue[],
  options: {
    expectedLength?: number;
    strictlyIncreasing?: boolean;
  } = {}
): void {
  if (!values) {
    return;
  }
  if (options.expectedLength !== undefined && values.length !== options.expectedLength) {
    issues.push(
      issue(
        "array-length-mismatch",
        path,
        `Expected length ${options.expectedLength}, got ${values.length}.`
      )
    );
  }
  let previous = Number.NEGATIVE_INFINITY;
  for (let index = 0; index < values.length; index += 1) {
    const value = values[index];
    if (!Number.isFinite(value)) {
      issues.push(issue("invalid-number", `${path}[${index}]`, `Expected a finite number at '${path}[${index}]'.`));
      break;
    }
    if (options.strictlyIncreasing) {
      if (index > 0 && value <= previous) {
        issues.push(issue("non-monotonic-array", path, `Expected '${path}' to be strictly increasing.`));
        break;
      }
      previous = value;
    }
  }
}

function validateDisplayTransform(
  transform: Partial<DisplayTransform> | undefined,
  path: string,
  issues: SeismicValidationIssue[]
): void {
  if (!transform) {
    return;
  }
  if (transform.gain !== undefined && (!Number.isFinite(transform.gain) || transform.gain <= 0)) {
    issues.push(issue("invalid-gain", `${path}.gain`, "Gain must be a finite number greater than zero."));
  }
  if (
    transform.clipMin !== undefined &&
    transform.clipMax !== undefined &&
    Number.isFinite(transform.clipMin) &&
    Number.isFinite(transform.clipMax) &&
    transform.clipMin >= transform.clipMax
  ) {
    issues.push(issue("invalid-clip-range", path, "clipMin must be less than clipMax."));
  }
}

function normalizeBytes(bytes: EncodedSeismicBytes): Uint8Array {
  return bytes instanceof Uint8Array ? bytes : Uint8Array.from(bytes);
}

function assertValid(kind: "section" | "gather", issues: SeismicValidationIssue[]): void {
  if (issues.length > 0) {
    throw new OphioliteSeismicValidationError([
      issue("invalid-payload-kind", kind, `Invalid ${kind} payload.`),
      ...issues
    ]);
  }
}

function validateFiniteNumber(value: number, path: string, issues: SeismicValidationIssue[]): void {
  if (!Number.isFinite(value)) {
    issues.push(issue("invalid-number", path, `Expected a finite number at '${path}'.`));
  }
}

function issue(code: string, path: string, message: string): SeismicValidationIssue {
  return { code, path, message };
}
