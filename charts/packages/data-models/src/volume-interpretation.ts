export type VolumeInterpretationSampleDomain = "time" | "depth";
export type VolumeInterpretationAxis = "inline" | "xline" | "sample";
export type VolumeInterpretationColorMap = "grayscale" | "red-white-blue";
export type VolumeInterpretationTool =
  | "pointer"
  | "orbit"
  | "pan"
  | "slice-drag"
  | "crop"
  | "select"
  | "interpret-seed";
export type VolumeInterpretationAction = "fitToData" | "topView" | "sideView" | "centerSelection";
export type VolumeInterpretationSelectionGesture = "shiftDragMove" | "delete" | "centerSelection";
export type VolumeInterpretationDeleteRequest =
  | {
      kind: "delete-slice-plane";
      itemId: string;
      itemName?: string;
    }
  | {
      kind: "delete-horizon-surface";
      itemId: string;
      itemName?: string;
    };
export interface VolumeInterpretationMoveSlicePlaneRequest {
  kind: "move-slice-plane";
  phase: "preview" | "commit";
  itemId: string;
  itemName?: string;
  axis: VolumeInterpretationAxis;
  volumeId: string;
  originalPosition: number;
  position: number;
  deltaWorld: number;
}
export type VolumeInterpretationEditRequest =
  | VolumeInterpretationDeleteRequest
  | VolumeInterpretationMoveSlicePlaneRequest;
export type VolumeInterpretationSelectionKind =
  | "slice-plane"
  | "horizon-surface"
  | "well-trajectory"
  | "well-marker"
  | "annotation";

export interface VolumeInterpretationBounds {
  minX: number;
  minY: number;
  minZ: number;
  maxX: number;
  maxY: number;
  maxZ: number;
}

export interface VolumeInterpretationCropBox extends VolumeInterpretationBounds {}

export interface VolumeInterpretationView {
  yawDeg: number;
  pitchDeg: number;
  zoom: number;
  focusX: number;
  focusY: number;
  focusZ: number;
}

export interface VolumeInterpretationVolume {
  id: string;
  name: string;
  sampleDomain: VolumeInterpretationSampleDomain;
  bounds: VolumeInterpretationBounds;
  dimensions: {
    inline: number;
    xline: number;
    sample: number;
  };
  displayDefaults?: {
    colormap?: VolumeInterpretationColorMap;
    gain?: number;
    clipMin?: number;
    clipMax?: number;
    opacity?: number;
  };
}

export interface VolumeInterpretationSlicePlaneStyle {
  colormap: VolumeInterpretationColorMap;
  gain: number;
  clipMin?: number;
  clipMax?: number;
  opacity: number;
  showBorder?: boolean;
}

export interface VolumeInterpretationSlicePlane {
  id: string;
  name: string;
  volumeId: string;
  axis: VolumeInterpretationAxis;
  position: number;
  visible: boolean;
  style: VolumeInterpretationSlicePlaneStyle;
}

export interface VolumeInterpretationHorizonStyle {
  fillColor?: string;
  fillOpacity: number;
  showContours: boolean;
  contourColor?: string;
  contourInterval?: number;
  edgeColor?: string;
  edgeWidth?: number;
}

export interface VolumeInterpretationHorizonSurface {
  id: string;
  name: string;
  visible: boolean;
  columns: number;
  rows: number;
  points: Float32Array;
  colorValues?: Float32Array;
  style: VolumeInterpretationHorizonStyle;
}

export interface VolumeInterpretationWellStyle {
  mode: "line" | "tube";
  color: string;
  width: number;
  showMarkers: boolean;
  showLabels: boolean;
}

export interface VolumeInterpretationWellTrajectory {
  id: string;
  name: string;
  visible: boolean;
  points: Float32Array;
  style: VolumeInterpretationWellStyle;
}

export interface VolumeInterpretationMarker {
  id: string;
  name: string;
  wellId?: string;
  visible: boolean;
  x: number;
  y: number;
  z: number;
  color: string;
  size: number;
}

export interface VolumeInterpretationAnnotation {
  id: string;
  text: string;
  visible: boolean;
  x: number;
  y: number;
  z: number;
  color?: string;
}

export interface VolumeInterpretationCapabilities {
  canRenderVolume: boolean;
  canMoveSlices: boolean;
  canCrop: boolean;
  canTriggerAutotrack: boolean;
  canEditSeeds: boolean;
  canShowContours: boolean;
}

export interface VolumeInterpretationProbeTarget {
  kind:
    | "slice-plane"
    | "slice-sample"
    | "horizon-surface"
    | "horizon-contour"
    | "well-trajectory"
    | "well-marker"
    | "annotation";
  itemId: string;
  itemName?: string;
}

export interface VolumeInterpretationProbe {
  target: VolumeInterpretationProbeTarget;
  worldX: number;
  worldY: number;
  worldZ: number;
  screenX: number;
  screenY: number;
}

export interface VolumeInterpretationSelection {
  kind: VolumeInterpretationSelectionKind;
  itemId: string;
  itemName?: string;
}

export interface VolumeInterpretationSelectionContext {
  selection: VolumeInterpretationSelection;
  allowedGestures: VolumeInterpretationSelectionGesture[];
}

export interface VolumeInterpretationInterpretationRequest {
  kind: "seed-horizon";
  targetHorizonId?: string;
  sourceVolumeId?: string;
  slicePlaneId?: string;
  worldX: number;
  worldY: number;
  worldZ: number;
}

export interface VolumeInterpretationModel {
  id: string;
  name: string;
  sampleDomain: VolumeInterpretationSampleDomain;
  sceneBounds: VolumeInterpretationBounds;
  cropBox?: VolumeInterpretationCropBox;
  volumes: VolumeInterpretationVolume[];
  slicePlanes: VolumeInterpretationSlicePlane[];
  horizons: VolumeInterpretationHorizonSurface[];
  wells: VolumeInterpretationWellTrajectory[];
  markers: VolumeInterpretationMarker[];
  annotations?: VolumeInterpretationAnnotation[];
  capabilities: VolumeInterpretationCapabilities;
}

export function createDefaultVolumeInterpretationView(
  bounds: VolumeInterpretationBounds
): VolumeInterpretationView {
  return {
    yawDeg: -36,
    pitchDeg: 24,
    zoom: 1,
    focusX: (bounds.minX + bounds.maxX) / 2,
    focusY: (bounds.minY + bounds.maxY) / 2,
    focusZ: (bounds.minZ + bounds.maxZ) / 2
  };
}

export function sceneSpan(bounds: VolumeInterpretationBounds): number {
  return Math.max(bounds.maxX - bounds.minX, bounds.maxY - bounds.minY, bounds.maxZ - bounds.minZ, 1);
}

export function clampSlicePlanePosition(
  plane: VolumeInterpretationSlicePlane,
  model: VolumeInterpretationModel
): VolumeInterpretationSlicePlane {
  const volume = model.volumes.find((candidate) => candidate.id === plane.volumeId);
  if (!volume) {
    return plane;
  }
  const [min, max] =
    plane.axis === "inline"
      ? [volume.bounds.minX, volume.bounds.maxX]
      : plane.axis === "xline"
        ? [volume.bounds.minY, volume.bounds.maxY]
        : [volume.bounds.minZ, volume.bounds.maxZ];

  return {
    ...plane,
    position: Math.min(Math.max(plane.position, min), max)
  };
}

export function slicePlaneWorldPoint(
  plane: VolumeInterpretationSlicePlane,
  volume: VolumeInterpretationVolume
): { x: number; y: number; z: number } {
  switch (plane.axis) {
    case "inline":
      return {
        x: plane.position,
        y: (volume.bounds.minY + volume.bounds.maxY) / 2,
        z: (volume.bounds.minZ + volume.bounds.maxZ) / 2
      };
    case "xline":
      return {
        x: (volume.bounds.minX + volume.bounds.maxX) / 2,
        y: plane.position,
        z: (volume.bounds.minZ + volume.bounds.maxZ) / 2
      };
    default:
      return {
        x: (volume.bounds.minX + volume.bounds.maxX) / 2,
        y: (volume.bounds.minY + volume.bounds.maxY) / 2,
        z: plane.position
      };
  }
}
