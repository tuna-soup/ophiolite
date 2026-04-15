import type {
  VolumeInterpretationBounds,
  VolumeInterpretationHorizonSurface,
  VolumeInterpretationModel,
  VolumeInterpretationVolume
} from "./volume-interpretation";

export function createMockVolumeInterpretationModel(): VolumeInterpretationModel {
  const bounds: VolumeInterpretationBounds = {
    minX: 0,
    minY: 0,
    minZ: 0,
    maxX: 1200,
    maxY: 960,
    maxZ: 900
  };

  const volume: VolumeInterpretationVolume = {
    id: "mock-volume",
    name: "F3 Synthetic Volume",
    sampleDomain: "time",
    bounds,
    dimensions: {
      inline: 160,
      xline: 128,
      sample: 256
    },
    displayDefaults: {
      colormap: "red-white-blue",
      gain: 1.1,
      opacity: 0.9
    }
  };

  return {
    id: "mock-volume-interpretation",
    name: "Synthetic Volume Interpretation Scene",
    sampleDomain: "time",
    sceneBounds: bounds,
    cropBox: {
      minX: 80,
      minY: 60,
      minZ: 70,
      maxX: 1120,
      maxY: 920,
      maxZ: 860
    },
    volumes: [volume],
    slicePlanes: [
      {
        id: "slice-inline",
        name: "Inline 742",
        volumeId: volume.id,
        axis: "inline",
        position: 430,
        visible: true,
        style: {
          colormap: "red-white-blue",
          gain: 1.1,
          opacity: 0.94,
          showBorder: true
        }
      },
      {
        id: "slice-xline",
        name: "Xline 318",
        volumeId: volume.id,
        axis: "xline",
        position: 520,
        visible: true,
        style: {
          colormap: "red-white-blue",
          gain: 1.05,
          opacity: 0.94,
          showBorder: true
        }
      },
      {
        id: "slice-sample",
        name: "Time Slice 1480 ms",
        volumeId: volume.id,
        axis: "sample",
        position: 520,
        visible: true,
        style: {
          colormap: "red-white-blue",
          gain: 0.95,
          opacity: 0.92,
          showBorder: true
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
          showMarkers: true,
          showLabels: true
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
          showMarkers: true,
          showLabels: true
        }
      }
    ],
    markers: [
      {
        id: "marker-1",
        name: "Top Reservoir Pick",
        wellId: "well-1",
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
}): VolumeInterpretationHorizonSurface {
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
    colorValues,
    style: {
      fillColor: options.fillColor,
      fillOpacity: 0.84,
      showContours: true,
      contourColor: options.contourColor,
      contourInterval: 28,
      edgeColor: "#101e28",
      edgeWidth: 1
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
