import type {
  VolumeInterpretationBounds,
  VolumeInterpretationDataSource,
  VolumeInterpretationSlicePayload,
  VolumeInterpretationSliceRequest,
  VolumeInterpretationModel
} from "./volume-interpretation";
import type {
  OphioliteResolvedVolumeHorizonSurfaceDto,
  OphioliteResolvedVolumeInterpretationSource,
  OphioliteResolvedVolumeDto
} from "./ophiolite-volume-interpretation-adapter";
import { adaptOphioliteVolumeInterpretationToChart } from "./ophiolite-volume-interpretation-adapter";

export function createMockVolumeInterpretationModel(): VolumeInterpretationModel {
  return adaptOphioliteVolumeInterpretationToChart(createMockOphioliteVolumeInterpretationSource());
}

export function createMockOphioliteVolumeInterpretationSource(): OphioliteResolvedVolumeInterpretationSource {
  const bounds: VolumeInterpretationBounds = {
    minX: 0,
    minY: 0,
    minZ: 0,
    maxX: 1200,
    maxY: 960,
    maxZ: 900
  };

  const sourceBounds = toResolvedBounds(bounds);
  const volume: OphioliteResolvedVolumeDto = {
    id: "mock-volume",
    name: "F3 Synthetic Volume",
    sample_domain: "time" as const,
    bounds: sourceBounds,
    dimensions: {
      inline: 160,
      xline: 128,
      sample: 256
    },
    fields: [
      {
        id: "amplitude",
        name: "Amplitude",
        kind: "amplitude",
        association: "point",
        sample_format: "f32",
        min_value: -1.6,
        max_value: 1.6,
        colormap: "red-white-blue"
      }
    ],
    active_field_id: "amplitude",
    data_source: createMockVolumeDataSource(bounds),
    display_defaults: {
      colormap: "red-white-blue",
      gain: 1.1,
      opacity: 0.9
    }
  };

  return {
    id: "mock-volume-interpretation",
    name: "Synthetic Volume Interpretation Scene",
    sample_domain: "time",
    scene_bounds: sourceBounds,
    crop_box: {
      min_x: 80,
      min_y: 60,
      min_z: 70,
      max_x: 1120,
      max_y: 920,
      max_z: 860
    },
    volumes: [volume],
    slice_planes: [
      {
        id: "slice-inline",
        name: "Inline 742",
        volume_id: volume.id,
        axis: "inline",
        position: 430,
        visible: true,
        style: {
          colormap: "red-white-blue",
          gain: 1.1,
          opacity: 0.94,
          show_border: true
        }
      },
      {
        id: "slice-xline",
        name: "Xline 318",
        volume_id: volume.id,
        axis: "xline",
        position: 520,
        visible: true,
        style: {
          colormap: "red-white-blue",
          gain: 1.05,
          opacity: 0.94,
          show_border: true
        }
      },
      {
        id: "slice-sample",
        name: "Time Slice 1480 ms",
        volume_id: volume.id,
        axis: "sample",
        position: 520,
        visible: true,
        style: {
          colormap: "red-white-blue",
          gain: 0.95,
          opacity: 0.92,
          show_border: true
        }
      }
    ],
    horizons: [
      createHorizonSurface({
        id: "horizon-a",
        name: "Top Reservoir",
        columns: 24,
        rows: 24,
        bounds,
        baseZ: 380,
        relief: 82,
        wavelength: 280,
        fillColor: "#4cc9f0",
        contourColor: "#0f5573"
      }),
      createHorizonSurface({
        id: "horizon-b",
        name: "Base Reservoir",
        columns: 24,
        rows: 24,
        bounds,
        baseZ: 615,
        relief: 68,
        wavelength: 340,
        fillColor: "#90be6d",
        contourColor: "#35572c"
      })
    ],
    wells: [
      {
        id: "well-1",
        name: "Well 1",
        visible: true,
        points: createTrajectoryPoints(bounds, 160, 220, 420, 180),
        style: {
          mode: "line",
          color: "#7bff6a",
          width: 3,
          show_markers: true,
          show_labels: true
        }
      },
      {
        id: "well-2",
        name: "Well 2",
        visible: true,
        points: createTrajectoryPoints(bounds, 980, 720, 220, -140),
        style: {
          mode: "tube",
          color: "#4cc9f0",
          width: 5,
          show_markers: true,
          show_labels: true
        }
      }
    ],
    markers: [
      {
        id: "marker-1",
        name: "Top Reservoir Pick",
        well_id: "well-1",
        visible: true,
        x: 350,
        y: 310,
        z: 405,
        color: "#f72585",
        size: 7
      },
      {
        id: "marker-2",
        name: "Autotrack Seed",
        visible: true,
        x: 610,
        y: 470,
        z: 420,
        color: "#ffd166",
        size: 8
      }
    ],
    annotations: [
      {
        id: "annotation-1",
        text: "Interpreter Workspace",
        visible: true,
        x: 1060,
        y: 110,
        z: 120,
        color: "#d7e9f2"
      }
    ],
    capabilities: {
      canRenderVolume: false,
      canMoveSlices: true,
      canCrop: true,
      canTriggerAutotrack: true,
      canEditSeeds: true,
      canShowContours: true
    }
  };
}

function toResolvedBounds(bounds: VolumeInterpretationBounds) {
  return {
    min_x: bounds.minX,
    min_y: bounds.minY,
    min_z: bounds.minZ,
    max_x: bounds.maxX,
    max_y: bounds.maxY,
    max_z: bounds.maxZ
  };
}

function createMockVolumeDataSource(bounds: VolumeInterpretationBounds): VolumeInterpretationDataSource {
  return {
    id: "mock-volume-slice-source",
    kind: "slice",
    preferredOwnership: "view",
    loadSlice: async (request) => createMockVolumeSlicePayload(request, bounds)
  };
}

function createMockVolumeSlicePayload(
  request: VolumeInterpretationSliceRequest,
  volumeBounds: VolumeInterpretationBounds
): VolumeInterpretationSlicePayload {
  const width = request.axis === "inline" ? 128 : 160;
  const height = request.axis === "sample" ? 128 : 256;
  const values = new Float32Array(width * height);
  const axisSpan =
    request.axis === "inline"
      ? volumeBounds.maxX - volumeBounds.minX
      : request.axis === "xline"
        ? volumeBounds.maxY - volumeBounds.minY
        : volumeBounds.maxZ - volumeBounds.minZ;
  const normalPosition =
    axisSpan > 0
      ? (request.position -
          (request.axis === "inline"
            ? volumeBounds.minX
            : request.axis === "xline"
              ? volumeBounds.minY
              : volumeBounds.minZ)) /
        axisSpan
      : 0.5;

  for (let row = 0; row < height; row += 1) {
    const rowNorm = height > 1 ? row / (height - 1) - 0.5 : 0;
    for (let column = 0; column < width; column += 1) {
      const columnNorm = width > 1 ? column / (width - 1) - 0.5 : 0;
      values[row * width + column] = syntheticAmplitude(columnNorm, rowNorm, normalPosition - 0.5);
    }
  }

  return {
    volumeId: request.volumeId,
    fieldId: request.fieldId,
    axis: request.axis,
    position: request.position,
    lod: request.lod ?? 0,
    bounds: slicePayloadBounds(request.axis, request.position, volumeBounds),
    dimensions: {
      width,
      height
    },
    sampleFormat: "f32",
    ownership: "view",
    values,
    valueRange: {
      min: -1.6,
      max: 1.6
    }
  };
}

function slicePayloadBounds(
  axis: VolumeInterpretationSliceRequest["axis"],
  position: number,
  bounds: VolumeInterpretationBounds
): VolumeInterpretationBounds {
  if (axis === "inline") {
    return { ...bounds, minX: position, maxX: position };
  }
  if (axis === "xline") {
    return { ...bounds, minY: position, maxY: position };
  }
  return { ...bounds, minZ: position, maxZ: position };
}

function syntheticAmplitude(columnNorm: number, rowNorm: number, positionNorm: number): number {
  const folded = Math.sin(rowNorm * 28 + positionNorm * 6 + Math.sin(columnNorm * 7) * 1.8);
  const stratigraphy = Math.sin(rowNorm * 44 + positionNorm * 9 - columnNorm * 5);
  const channel = Math.exp(-((positionNorm - columnNorm * 0.18) ** 2 * 22 + (rowNorm + 0.12) ** 2 * 85));
  const diapir = Math.exp(-((positionNorm + 0.05) ** 2 * 36 + (columnNorm - 0.08) ** 2 * 28)) * Math.cos(rowNorm * 18);
  return folded * 0.55 + stratigraphy * 0.32 - channel * 0.65 + diapir * 0.42;
}

function createHorizonSurface(options: {
  id: string;
  name: string;
  columns: number;
  rows: number;
  bounds: VolumeInterpretationBounds;
  baseZ: number;
  relief: number;
  wavelength: number;
  fillColor: string;
  contourColor: string;
}): OphioliteResolvedVolumeHorizonSurfaceDto {
  const { columns, rows, bounds, baseZ, relief, wavelength } = options;
  const points = new Float32Array(columns * rows * 3);
  const colorValues = new Float32Array(columns * rows);

  for (let row = 0; row < rows; row += 1) {
    const y = lerp(bounds.minY + 80, bounds.maxY - 80, row / Math.max(1, rows - 1));
    for (let column = 0; column < columns; column += 1) {
      const x = lerp(bounds.minX + 80, bounds.maxX - 80, column / Math.max(1, columns - 1));
      const ripple =
        Math.sin(x / wavelength) * relief +
        Math.cos(y / (wavelength * 0.8)) * relief * 0.55 +
        Math.sin((x + y) / (wavelength * 0.65)) * relief * 0.22;
      const z = baseZ + ripple;
      const index = row * columns + column;
      points[index * 3] = x;
      points[index * 3 + 1] = y;
      points[index * 3 + 2] = z;
      colorValues[index] = z;
    }
  }

  return {
    id: options.id,
    name: options.name,
    visible: true,
    columns,
    rows,
    points,
    color_values: colorValues,
    style: {
      fill_color: options.fillColor,
      fill_opacity: 0.84,
      show_contours: true,
      contour_color: options.contourColor,
      contour_interval: 28,
      edge_color: "#101e28",
      edge_width: 1
    }
  };
}

function createTrajectoryPoints(
  bounds: VolumeInterpretationBounds,
  surfaceX: number,
  surfaceY: number,
  driftX: number,
  driftY: number
): Float32Array {
  const stationCount = 28;
  const points = new Float32Array(stationCount * 3);
  for (let index = 0; index < stationCount; index += 1) {
    const ratio = index / Math.max(1, stationCount - 1);
    points[index * 3] = surfaceX + driftX * Math.sin(ratio * Math.PI * 0.85) * ratio;
    points[index * 3 + 1] = surfaceY + driftY * ratio + Math.sin(ratio * Math.PI * 1.4) * 26;
    points[index * 3 + 2] = lerp(bounds.minZ + 20, bounds.maxZ - 30, ratio);
  }
  return points;
}

function lerp(start: number, end: number, ratio: number): number {
  return start + (end - start) * ratio;
}
