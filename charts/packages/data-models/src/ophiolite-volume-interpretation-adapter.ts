import type {
  VolumeInterpretationAnnotation,
  VolumeInterpretationBounds,
  VolumeInterpretationCapabilities,
  VolumeInterpretationColorMap,
  VolumeInterpretationDataSource,
  VolumeInterpretationHorizonSurface,
  VolumeInterpretationMarker,
  VolumeInterpretationModel,
  VolumeInterpretationSampleDomain,
  VolumeInterpretationScalarAssociation,
  VolumeInterpretationScalarKind,
  VolumeInterpretationSlicePlane,
  VolumeInterpretationVolume,
  VolumeInterpretationWellTrajectory
} from "./volume-interpretation";

export interface OphioliteResolvedVolumeBoundsDto {
  min_x: number;
  min_y: number;
  min_z: number;
  max_x: number;
  max_y: number;
  max_z: number;
}

export interface OphioliteResolvedVolumeScalarFieldDto {
  id: string;
  name: string;
  kind: VolumeInterpretationScalarKind;
  association: VolumeInterpretationScalarAssociation;
  sample_format: "f32" | "f16" | "i16" | "u8-scale-bias";
  unit?: string;
  min_value?: number;
  max_value?: number;
  colormap?: VolumeInterpretationColorMap;
}

export interface OphioliteResolvedVolumeDto {
  id: string;
  name: string;
  sample_domain: VolumeInterpretationSampleDomain;
  bounds: OphioliteResolvedVolumeBoundsDto;
  dimensions: {
    inline: number;
    xline: number;
    sample: number;
  };
  fields?: OphioliteResolvedVolumeScalarFieldDto[];
  active_field_id?: string;
  data_source?: VolumeInterpretationDataSource;
  display_defaults?: {
    colormap?: VolumeInterpretationColorMap;
    gain?: number;
    clip_min?: number;
    clip_max?: number;
    opacity?: number;
  };
}

export interface OphioliteResolvedVolumeSlicePlaneDto {
  id: string;
  name: string;
  volume_id: string;
  axis: VolumeInterpretationSlicePlane["axis"];
  position: number;
  visible: boolean;
  style: {
    colormap: VolumeInterpretationColorMap;
    gain: number;
    clip_min?: number;
    clip_max?: number;
    opacity: number;
    show_border?: boolean;
  };
}

export interface OphioliteResolvedVolumeHorizonSurfaceDto {
  id: string;
  name: string;
  visible: boolean;
  columns: number;
  rows: number;
  points: Float32Array;
  color_values?: Float32Array;
  style: {
    fill_color?: string;
    fill_opacity: number;
    show_contours: boolean;
    contour_color?: string;
    contour_interval?: number;
    edge_color?: string;
    edge_width?: number;
  };
}

export interface OphioliteResolvedVolumeWellTrajectoryDto {
  id: string;
  name: string;
  visible: boolean;
  points: Float32Array;
  style: {
    mode: "line" | "tube";
    color: string;
    width: number;
    show_markers: boolean;
    show_labels: boolean;
  };
}

export interface OphioliteResolvedVolumeMarkerDto {
  id: string;
  name: string;
  well_id?: string;
  visible: boolean;
  x: number;
  y: number;
  z: number;
  color: string;
  size: number;
}

export interface OphioliteResolvedVolumeAnnotationDto {
  id: string;
  text: string;
  visible: boolean;
  x: number;
  y: number;
  z: number;
  color?: string;
}

export interface OphioliteResolvedVolumeInterpretationSource {
  id: string;
  name: string;
  sample_domain: VolumeInterpretationSampleDomain;
  scene_bounds: OphioliteResolvedVolumeBoundsDto;
  crop_box?: OphioliteResolvedVolumeBoundsDto;
  volumes: OphioliteResolvedVolumeDto[];
  slice_planes: OphioliteResolvedVolumeSlicePlaneDto[];
  horizons: OphioliteResolvedVolumeHorizonSurfaceDto[];
  wells: OphioliteResolvedVolumeWellTrajectoryDto[];
  markers: OphioliteResolvedVolumeMarkerDto[];
  annotations?: OphioliteResolvedVolumeAnnotationDto[];
  capabilities: VolumeInterpretationCapabilities;
}

export interface VolumeInterpretationValidationIssue {
  code: string;
  path: string;
  message: string;
}

export class OphioliteVolumeInterpretationValidationError extends Error {
  readonly issues: VolumeInterpretationValidationIssue[];

  constructor(issues: VolumeInterpretationValidationIssue[]) {
    super([
      "Volume interpretation source validation failed.",
      ...issues.map((entry) => `- [${entry.code}] ${entry.path}: ${entry.message}`)
    ].join("\n"));
    this.name = "OphioliteVolumeInterpretationValidationError";
    this.issues = issues;
  }
}

export function adaptOphioliteVolumeInterpretationToChart(
  source: OphioliteResolvedVolumeInterpretationSource
): VolumeInterpretationModel {
  const issues = validateOphioliteVolumeInterpretationSource(source);
  if (issues.length > 0) {
    throw new OphioliteVolumeInterpretationValidationError(issues);
  }

  return {
    id: source.id,
    name: source.name,
    sampleDomain: source.sample_domain,
    sceneBounds: adaptBounds(source.scene_bounds),
    cropBox: source.crop_box ? adaptBounds(source.crop_box) : undefined,
    volumes: source.volumes.map(adaptVolume),
    slicePlanes: source.slice_planes.map(adaptSlicePlane),
    horizons: source.horizons.map(adaptHorizon),
    wells: source.wells.map(adaptWell),
    markers: source.markers.map(adaptMarker),
    annotations: source.annotations?.map(adaptAnnotation),
    capabilities: { ...source.capabilities }
  };
}

export function validateOphioliteVolumeInterpretationSource(
  source: OphioliteResolvedVolumeInterpretationSource
): VolumeInterpretationValidationIssue[] {
  const issues: VolumeInterpretationValidationIssue[] = [];
  const volumeIds = new Set(source.volumes.map((volume) => volume.id));
  validateBounds(source.scene_bounds, "scene_bounds", issues);
  source.crop_box && validateBounds(source.crop_box, "crop_box", issues);
  source.volumes.forEach((volume, index) => {
    validateBounds(volume.bounds, `volumes[${index}].bounds`, issues);
    validatePositiveInt(volume.dimensions.inline, `volumes[${index}].dimensions.inline`, issues);
    validatePositiveInt(volume.dimensions.xline, `volumes[${index}].dimensions.xline`, issues);
    validatePositiveInt(volume.dimensions.sample, `volumes[${index}].dimensions.sample`, issues);
  });
  source.slice_planes.forEach((plane, index) => {
    if (!volumeIds.has(plane.volume_id)) {
      issues.push(issue("unknown-volume", `slice_planes[${index}].volume_id`, `Unknown volume id '${plane.volume_id}'.`));
    }
    validateFinite(plane.position, `slice_planes[${index}].position`, issues);
  });
  source.horizons.forEach((horizon, index) => {
    validatePositiveInt(horizon.columns, `horizons[${index}].columns`, issues);
    validatePositiveInt(horizon.rows, `horizons[${index}].rows`, issues);
    if (horizon.points.length !== horizon.columns * horizon.rows * 3) {
      issues.push(issue("invalid-horizon-points", `horizons[${index}].points`, "Horizon points must be columns * rows * 3."));
    }
    if (horizon.color_values && horizon.color_values.length !== horizon.columns * horizon.rows) {
      issues.push(issue("invalid-horizon-colors", `horizons[${index}].color_values`, "Horizon color values must be columns * rows."));
    }
  });
  return issues;
}

function adaptBounds(source: OphioliteResolvedVolumeBoundsDto): VolumeInterpretationBounds {
  return {
    minX: source.min_x,
    minY: source.min_y,
    minZ: source.min_z,
    maxX: source.max_x,
    maxY: source.max_y,
    maxZ: source.max_z
  };
}

function adaptVolume(source: OphioliteResolvedVolumeDto): VolumeInterpretationVolume {
  return {
    id: source.id,
    name: source.name,
    sampleDomain: source.sample_domain,
    bounds: adaptBounds(source.bounds),
    dimensions: { ...source.dimensions },
    fields: source.fields?.map((field) => ({
      id: field.id,
      name: field.name,
      kind: field.kind,
      association: field.association,
      sampleFormat: field.sample_format,
      unit: field.unit,
      valueRange:
        field.min_value !== undefined && field.max_value !== undefined
          ? {
              min: field.min_value,
              max: field.max_value
            }
          : undefined,
      colormap: field.colormap
    })),
    activeFieldId: source.active_field_id,
    dataSource: source.data_source,
    displayDefaults: source.display_defaults
      ? {
          colormap: source.display_defaults.colormap,
          gain: source.display_defaults.gain,
          clipMin: source.display_defaults.clip_min,
          clipMax: source.display_defaults.clip_max,
          opacity: source.display_defaults.opacity
        }
      : undefined
  };
}

function adaptSlicePlane(source: OphioliteResolvedVolumeSlicePlaneDto): VolumeInterpretationSlicePlane {
  return {
    id: source.id,
    name: source.name,
    volumeId: source.volume_id,
    axis: source.axis,
    position: source.position,
    visible: source.visible,
    style: {
      colormap: source.style.colormap,
      gain: source.style.gain,
      clipMin: source.style.clip_min,
      clipMax: source.style.clip_max,
      opacity: source.style.opacity,
      showBorder: source.style.show_border
    }
  };
}

function adaptHorizon(source: OphioliteResolvedVolumeHorizonSurfaceDto): VolumeInterpretationHorizonSurface {
  return {
    id: source.id,
    name: source.name,
    visible: source.visible,
    columns: source.columns,
    rows: source.rows,
    points: source.points,
    colorValues: source.color_values,
    style: {
      fillColor: source.style.fill_color,
      fillOpacity: source.style.fill_opacity,
      showContours: source.style.show_contours,
      contourColor: source.style.contour_color,
      contourInterval: source.style.contour_interval,
      edgeColor: source.style.edge_color,
      edgeWidth: source.style.edge_width
    }
  };
}

function adaptWell(source: OphioliteResolvedVolumeWellTrajectoryDto): VolumeInterpretationWellTrajectory {
  return {
    id: source.id,
    name: source.name,
    visible: source.visible,
    points: source.points,
    style: {
      mode: source.style.mode,
      color: source.style.color,
      width: source.style.width,
      showMarkers: source.style.show_markers,
      showLabels: source.style.show_labels
    }
  };
}

function adaptMarker(source: OphioliteResolvedVolumeMarkerDto): VolumeInterpretationMarker {
  return {
    id: source.id,
    name: source.name,
    wellId: source.well_id,
    visible: source.visible,
    x: source.x,
    y: source.y,
    z: source.z,
    color: source.color,
    size: source.size
  };
}

function adaptAnnotation(source: OphioliteResolvedVolumeAnnotationDto): VolumeInterpretationAnnotation {
  return { ...source };
}

function validateBounds(
  bounds: OphioliteResolvedVolumeBoundsDto,
  path: string,
  issues: VolumeInterpretationValidationIssue[]
): void {
  for (const [key, value] of Object.entries(bounds)) {
    validateFinite(value, `${path}.${key}`, issues);
  }
  if (bounds.min_x >= bounds.max_x || bounds.min_y >= bounds.max_y || bounds.min_z >= bounds.max_z) {
    issues.push(issue("invalid-bounds", path, "Bounds min values must be lower than max values."));
  }
}

function validatePositiveInt(value: number, path: string, issues: VolumeInterpretationValidationIssue[]): void {
  if (!Number.isInteger(value) || value <= 0) {
    issues.push(issue("invalid-positive-int", path, "Value must be a positive integer."));
  }
}

function validateFinite(value: number, path: string, issues: VolumeInterpretationValidationIssue[]): void {
  if (!Number.isFinite(value)) {
    issues.push(issue("invalid-number", path, "Value must be finite."));
  }
}

function issue(code: string, path: string, message: string): VolumeInterpretationValidationIssue {
  return { code, path, message };
}
