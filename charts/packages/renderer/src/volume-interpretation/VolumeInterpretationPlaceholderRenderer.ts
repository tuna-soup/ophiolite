import type {
  VolumeInterpretationAnnotation,
  VolumeInterpretationBounds,
  VolumeInterpretationHorizonSurface,
  VolumeInterpretationMarker,
  VolumeInterpretationModel,
  VolumeInterpretationSlicePlane,
  VolumeInterpretationView,
  VolumeInterpretationVolume,
  VolumeInterpretationWellTrajectory
} from "@ophiolite/charts-data-models";
import type {
  VolumeInterpretationPickDebugCandidate,
  VolumeInterpretationPickDebugSnapshot,
  VolumeInterpretationPickResult,
  VolumeInterpretationRenderFrame,
  VolumeInterpretationRendererAdapter
} from "./adapter";

interface Point2D {
  x: number;
  y: number;
  depth: number;
}

interface ProjectedPolygonTarget {
  type: "polygon";
  pick: VolumeInterpretationPickResult;
  points: Point2D[];
  depth: number;
}

interface ProjectedPolylineTarget {
  type: "polyline";
  pick: VolumeInterpretationPickResult;
  points: Point2D[];
  strokeWidth: number;
  depth: number;
}

interface ProjectedPointTarget {
  type: "point";
  pick: VolumeInterpretationPickResult;
  point: Point2D;
  radius: number;
  depth: number;
}

type ProjectedTarget = ProjectedPolygonTarget | ProjectedPolylineTarget | ProjectedPointTarget;

const BG_TOP = "#0d1822";
const BG_BOTTOM = "#152c3c";
export class VolumeInterpretationPlaceholderRenderer implements VolumeInterpretationRendererAdapter {
  private container: HTMLElement | null = null;
  private canvas: HTMLCanvasElement | null = null;
  private context: CanvasRenderingContext2D | null = null;
  private projectedTargets: ProjectedTarget[] = [];
  private frame: VolumeInterpretationRenderFrame | null = null;

  mount(container: HTMLElement): void {
    this.container = container;
    this.canvas = document.createElement("canvas");
    this.canvas.style.width = "100%";
    this.canvas.style.height = "100%";
    this.canvas.style.display = "block";
    this.context = this.canvas.getContext("2d");
    container.appendChild(this.canvas);
  }

  render(frame: VolumeInterpretationRenderFrame): void {
    this.frame = frame;
    if (!this.container || !this.canvas || !this.context) {
      return;
    }

    const width = Math.max(1, this.container.clientWidth);
    const height = Math.max(1, this.container.clientHeight);
    const dpr = Math.max(1, Math.min(2, window.devicePixelRatio || 1));
    if (this.canvas.width !== Math.round(width * dpr) || this.canvas.height !== Math.round(height * dpr)) {
      this.canvas.width = Math.round(width * dpr);
      this.canvas.height = Math.round(height * dpr);
    }

    const context = this.context;
    context.setTransform(dpr, 0, 0, dpr, 0, 0);
    context.clearRect(0, 0, width, height);

    drawBackground(context, width, height);
    this.projectedTargets = [];

    const { model, view, selection, probe } = frame.state;
    if (!model || !view) {
      drawEmptyState(context, width, height);
      return;
    }

    const projector = createProjector(model.sceneBounds, view, width, height);

    model.slicePlanes
      .filter((plane) => plane.visible)
      .forEach((plane) => {
        const volume = model.volumes.find((candidate) => candidate.id === plane.volumeId);
        if (!volume) {
          return;
        }
        const result = drawSlicePlane(context, plane, volume, projector);
        if (result) {
          this.projectedTargets.push(result);
        }
      });

    model.horizons
      .filter((horizon) => horizon.visible)
      .forEach((horizon) => {
        const result = drawHorizon(context, horizon, projector);
        this.projectedTargets.push(...result);
      });

    model.wells
      .filter((well) => well.visible)
      .forEach((well) => {
        const result = drawWell(context, well, projector);
        if (result) {
          this.projectedTargets.push(result);
        }
      });

    model.markers
      .filter((marker) => marker.visible)
      .forEach((marker) => {
        const result = drawMarker(context, marker, projector);
        if (result) {
          this.projectedTargets.push(result);
        }
      });

    model.annotations
      ?.filter((annotation) => annotation.visible)
      .forEach((annotation) => {
        const result = drawAnnotation(context, annotation, projector);
        if (result) {
          this.projectedTargets.push(result);
        }
      });

    if (selection) {
      highlightSelection(context, selection.itemId, this.projectedTargets);
    }
    if (probe) {
      highlightProbe(context, probe.screenX, probe.screenY);
    }
  }

  pick(screenX: number, screenY: number): VolumeInterpretationPickResult | null {
    return this.debugPick(screenX, screenY).winner;
  }

  projectWorldToScreen(worldX: number, worldY: number, worldZ: number): Point2D | null {
    if (!this.container || !this.frame?.state.model || !this.frame.state.view) {
      return null;
    }
    const width = Math.max(1, this.container.clientWidth);
    const height = Math.max(1, this.container.clientHeight);
    const projector = createProjector(this.frame.state.model.sceneBounds, this.frame.state.view, width, height);
    return projector(worldX, worldY, worldZ);
  }

  debugPick(screenX: number, screenY: number): VolumeInterpretationPickDebugSnapshot {
    const ranked = rankProjectedTargets(this.projectedTargets, screenX, screenY);
    const syntheticWinnerTarget = ranked.find((candidate) => candidate.hit)?.target ?? null;
    const syntheticWinner = syntheticWinnerTarget
      ? {
          ...syntheticWinnerTarget.pick,
          screenX,
          screenY
        }
      : null;
    return {
      pointerX: screenX,
      pointerY: screenY,
      renderPointerX: screenX,
      renderPointerY: screenY,
      renderScaleX: 1,
      renderScaleY: 1,
      actualWinner: null,
      actualPickedCount: 0,
      actualMatchedBy: null,
      syntheticWinner,
      winner: syntheticWinner,
      candidates: ranked.slice(0, 8).map((candidate) => toDebugCandidate(candidate.target, candidate.hit, candidate.score))
    };
  }

  dispose(): void {
    if (this.canvas?.parentNode) {
      this.canvas.parentNode.removeChild(this.canvas);
    }
    this.projectedTargets = [];
    this.frame = null;
    this.context = null;
    this.canvas = null;
    this.container = null;
  }
}

function drawBackground(context: CanvasRenderingContext2D, width: number, height: number): void {
  const gradient = context.createLinearGradient(0, 0, 0, height);
  gradient.addColorStop(0, BG_TOP);
  gradient.addColorStop(1, BG_BOTTOM);
  context.fillStyle = gradient;
  context.fillRect(0, 0, width, height);
}

function drawEmptyState(context: CanvasRenderingContext2D, width: number, height: number): void {
  context.fillStyle = "rgba(234, 242, 246, 0.78)";
  context.font = "600 16px Segoe UI, sans-serif";
  context.textAlign = "center";
  context.textBaseline = "middle";
  context.fillText("Volume interpretation scene unavailable.", width / 2, height / 2);
}

function createProjector(
  bounds: VolumeInterpretationBounds,
  view: VolumeInterpretationView,
  width: number,
  height: number
): (x: number, y: number, z: number) => Point2D {
  const centerX = width / 2;
  const centerY = height / 2;
  const span = Math.max(bounds.maxX - bounds.minX, bounds.maxY - bounds.minY, bounds.maxZ - bounds.minZ, 1);
  const yaw = (view.yawDeg * Math.PI) / 180;
  const pitch = (view.pitchDeg * Math.PI) / 180;
  const baseScale = (Math.min(width, height) * 0.44) / span;
  const horizonScale = 0.28 + Math.sin(pitch) * 0.18;
  const depthScale = 0.9 + Math.cos(pitch) * 0.06;

  return (x: number, y: number, z: number): Point2D => {
    const dx = x - view.focusX;
    const dy = y - view.focusY;
    const dz = z - view.focusZ;
    const rx = dx * Math.cos(yaw) - dy * Math.sin(yaw);
    const ry = dx * Math.sin(yaw) + dy * Math.cos(yaw);
    const depth = rx * Math.cos(pitch) + ry * Math.sin(pitch) + dz * Math.cos(pitch);
    return {
      x: centerX + (rx - ry * 0.92) * baseScale * view.zoom,
      y: centerY + ((rx + ry) * horizonScale - dz * depthScale) * baseScale * view.zoom,
      depth
    };
  };
}

function drawSlicePlane(
  context: CanvasRenderingContext2D,
  plane: VolumeInterpretationSlicePlane,
  volume: VolumeInterpretationVolume,
  projector: (x: number, y: number, z: number) => Point2D
): ProjectedPolygonTarget | null {
  const corners = slicePlaneCorners(plane, volume).map((point) => projector(point.x, point.y, point.z));
  if (corners.length !== 4) {
    return null;
  }

  context.save();
  context.beginPath();
  context.moveTo(corners[0]!.x, corners[0]!.y);
  for (let index = 1; index < corners.length; index += 1) {
    context.lineTo(corners[index]!.x, corners[index]!.y);
  }
  context.closePath();
  context.clip();
  fillSeismicPattern(context, corners, plane.id, plane.style.opacity, plane.style.colormap);
  context.restore();

  if (plane.style.showBorder !== false) {
    context.strokeStyle = "rgba(255, 255, 255, 0.74)";
    context.lineWidth = 1.2;
    context.beginPath();
    context.moveTo(corners[0]!.x, corners[0]!.y);
    for (let index = 1; index < corners.length; index += 1) {
      context.lineTo(corners[index]!.x, corners[index]!.y);
    }
    context.closePath();
    context.stroke();
  }

  const center = polygonCenter(corners);
  return {
    type: "polygon",
    depth: center.depth,
    pick: {
      kind: "slice-plane",
      itemId: plane.id,
      itemName: plane.name,
      worldX: plane.axis === "inline" ? plane.position : (volume.bounds.minX + volume.bounds.maxX) / 2,
      worldY: plane.axis === "xline" ? plane.position : (volume.bounds.minY + volume.bounds.maxY) / 2,
      worldZ: plane.axis === "sample" ? plane.position : (volume.bounds.minZ + volume.bounds.maxZ) / 2,
      screenX: center.x,
      screenY: center.y
    },
    points: corners
  };
}

function fillSeismicPattern(
  context: CanvasRenderingContext2D,
  corners: Point2D[],
  seed: string,
  opacity: number,
  colormap: "grayscale" | "red-white-blue"
): void {
  const bounds = polygonBounds(corners);
  context.fillStyle = `rgba(255,255,255,${Math.max(0.1, Math.min(0.7, opacity * 0.55))})`;
  context.fillRect(bounds.minX, bounds.minY, bounds.maxX - bounds.minX, bounds.maxY - bounds.minY);
  const seedValue = Array.from(seed).reduce((sum, char) => sum + char.charCodeAt(0), 0);
  const lineCount = 56;
  for (let index = 0; index < lineCount; index += 1) {
    const ratio = index / Math.max(1, lineCount - 1);
    const y = bounds.minY + ratio * (bounds.maxY - bounds.minY);
    const amplitude = 8 + Math.sin((index + seedValue) * 0.32) * 3;
    context.beginPath();
    for (let x = bounds.minX; x <= bounds.maxX; x += 9) {
      const phase = (x * 0.03) + index * 0.18 + seedValue * 0.004;
      const waveY = y + Math.sin(phase) * amplitude;
      if (x === bounds.minX) {
        context.moveTo(x, waveY);
      } else {
        context.lineTo(x, waveY);
      }
    }
    context.strokeStyle =
      colormap === "grayscale"
        ? index % 2 === 0
          ? "rgba(56, 72, 86, 0.34)"
          : "rgba(225, 233, 239, 0.42)"
        : index % 2 === 0
          ? "rgba(42, 88, 178, 0.46)"
          : "rgba(210, 61, 61, 0.42)";
    context.lineWidth = 1;
    context.stroke();
  }
}

function drawHorizon(
  context: CanvasRenderingContext2D,
  horizon: VolumeInterpretationHorizonSurface,
  projector: (x: number, y: number, z: number) => Point2D
): ProjectedTarget[] {
  const targets: ProjectedTarget[] = [];
  const { columns, rows, points, colorValues } = horizon;
  if (columns < 2 || rows < 2) {
    return targets;
  }

  const [minValue, maxValue] = colorValues ? minMax(colorValues) : [0, 1];

  for (let row = 0; row < rows - 1; row += 1) {
    for (let column = 0; column < columns - 1; column += 1) {
      const indices = [
        row * columns + column,
        row * columns + column + 1,
        (row + 1) * columns + column + 1,
        (row + 1) * columns + column
      ];
      const polygon = indices.map((index) =>
        projector(points[index * 3]!, points[index * 3 + 1]!, points[index * 3 + 2]!)
      );
      const fillColor = colorValues
        ? samplePalette((average(indices.map((index) => colorValues[index]!)) - minValue) / Math.max(1e-6, maxValue - minValue))
        : horizon.style.fillColor ?? "#4cc9f0";
      context.fillStyle = withAlpha(fillColor, horizon.style.fillOpacity);
      context.beginPath();
      context.moveTo(polygon[0]!.x, polygon[0]!.y);
      for (let index = 1; index < polygon.length; index += 1) {
        context.lineTo(polygon[index]!.x, polygon[index]!.y);
      }
      context.closePath();
      context.fill();
    }
  }

  context.strokeStyle = horizon.style.edgeColor ?? "rgba(16, 30, 40, 0.8)";
  context.lineWidth = horizon.style.edgeWidth ?? 1;
  for (let row = 0; row < rows; row += 1) {
    context.beginPath();
    for (let column = 0; column < columns; column += 1) {
      const index = row * columns + column;
      const point = projector(points[index * 3]!, points[index * 3 + 1]!, points[index * 3 + 2]!);
      if (column === 0) {
        context.moveTo(point.x, point.y);
      } else {
        context.lineTo(point.x, point.y);
      }
    }
    context.stroke();
  }
  for (let column = 0; column < columns; column += 1) {
    context.beginPath();
    for (let row = 0; row < rows; row += 1) {
      const index = row * columns + column;
      const point = projector(points[index * 3]!, points[index * 3 + 1]!, points[index * 3 + 2]!);
      if (row === 0) {
        context.moveTo(point.x, point.y);
      } else {
        context.lineTo(point.x, point.y);
      }
    }
    context.stroke();
  }

  if (horizon.style.showContours) {
    const contourColor = horizon.style.contourColor ?? "rgba(20, 44, 58, 0.72)";
    context.strokeStyle = contourColor;
    context.lineWidth = 1.4;
    for (let row = 0; row < rows; row += 4) {
      context.beginPath();
      for (let column = 0; column < columns; column += 1) {
        const index = row * columns + column;
        const point = projector(points[index * 3]!, points[index * 3 + 1]!, points[index * 3 + 2]!);
        if (column === 0) {
          context.moveTo(point.x, point.y);
        } else {
          context.lineTo(point.x, point.y);
        }
      }
      context.stroke();
    }
  }

  const centerIndex = Math.floor((rows * columns) / 2);
  const center = projector(points[centerIndex * 3]!, points[centerIndex * 3 + 1]!, points[centerIndex * 3 + 2]!);
  targets.push({
    type: "point",
    point: center,
    radius: 18,
    depth: center.depth,
    pick: {
      kind: "horizon-surface",
      itemId: horizon.id,
      itemName: horizon.name,
      worldX: points[centerIndex * 3]!,
      worldY: points[centerIndex * 3 + 1]!,
      worldZ: points[centerIndex * 3 + 2]!,
      screenX: center.x,
      screenY: center.y
    }
  });
  return targets;
}

function drawWell(
  context: CanvasRenderingContext2D,
  well: VolumeInterpretationWellTrajectory,
  projector: (x: number, y: number, z: number) => Point2D
): ProjectedPolylineTarget | null {
  if (well.points.length < 6) {
    return null;
  }

  const points: Point2D[] = [];
  context.beginPath();
  for (let index = 0; index < well.points.length; index += 3) {
    const point = projector(well.points[index]!, well.points[index + 1]!, well.points[index + 2]!);
    points.push(point);
    if (index === 0) {
      context.moveTo(point.x, point.y);
    } else {
      context.lineTo(point.x, point.y);
    }
  }
  context.strokeStyle = well.style.color;
  context.lineWidth = well.style.mode === "tube" ? Math.max(4, well.style.width) : Math.max(2, well.style.width);
  context.stroke();

  if (well.style.showLabels) {
    const surface = points[0]!;
    context.fillStyle = "rgba(236, 244, 248, 0.92)";
    context.font = "600 12px Segoe UI, sans-serif";
    context.textAlign = "left";
    context.textBaseline = "bottom";
    context.fillText(well.name, surface.x + 8, surface.y - 6);
  }

  return {
    type: "polyline",
    points,
    strokeWidth: well.style.mode === "tube" ? Math.max(4, well.style.width) : Math.max(2, well.style.width),
    depth: average(points.map((point) => point.depth)),
    pick: {
      kind: "well-trajectory",
      itemId: well.id,
      itemName: well.name,
      worldX: well.points[0]!,
      worldY: well.points[1]!,
      worldZ: well.points[2]!,
      screenX: points[0]!.x,
      screenY: points[0]!.y
    }
  };
}

function drawMarker(
  context: CanvasRenderingContext2D,
  marker: VolumeInterpretationMarker,
  projector: (x: number, y: number, z: number) => Point2D
): ProjectedPointTarget | null {
  const point = projector(marker.x, marker.y, marker.z);
  context.beginPath();
  context.fillStyle = marker.color;
  context.arc(point.x, point.y, marker.size, 0, Math.PI * 2);
  context.fill();
  context.lineWidth = 1.5;
  context.strokeStyle = "rgba(255,255,255,0.75)";
  context.stroke();
  return {
    type: "point",
    point,
    radius: marker.size + 4,
    depth: point.depth,
    pick: {
      kind: "well-marker",
      itemId: marker.id,
      itemName: marker.name,
      worldX: marker.x,
      worldY: marker.y,
      worldZ: marker.z,
      screenX: point.x,
      screenY: point.y
    }
  };
}

function drawAnnotation(
  context: CanvasRenderingContext2D,
  annotation: VolumeInterpretationAnnotation,
  projector: (x: number, y: number, z: number) => Point2D
): ProjectedPointTarget | null {
  const point = projector(annotation.x, annotation.y, annotation.z);
  context.fillStyle = annotation.color ?? "rgba(228, 239, 245, 0.92)";
  context.font = "600 12px Segoe UI, sans-serif";
  context.textAlign = "left";
  context.textBaseline = "middle";
  context.fillText(annotation.text, point.x, point.y);
  return {
    type: "point",
    point,
    radius: 14,
    depth: point.depth,
    pick: {
      kind: "annotation",
      itemId: annotation.id,
      itemName: annotation.text,
      worldX: annotation.x,
      worldY: annotation.y,
      worldZ: annotation.z,
      screenX: point.x,
      screenY: point.y
    }
  };
}

function highlightSelection(
  context: CanvasRenderingContext2D,
  itemId: string,
  projectedTargets: ProjectedTarget[]
): void {
  const target = projectedTargets.find((candidate) => candidate.pick.itemId === itemId);
  if (!target) {
    return;
  }

  context.save();
  context.strokeStyle = "rgba(92, 181, 255, 0.96)";
  if (target.type === "point") {
    context.lineWidth = 4;
    context.beginPath();
    context.arc(target.point.x, target.point.y, target.radius + 5, 0, Math.PI * 2);
    context.stroke();
  } else if (target.type === "polyline") {
    context.lineWidth = Math.max(4, target.strokeWidth + 2.5);
    context.beginPath();
    context.moveTo(target.points[0]!.x, target.points[0]!.y);
    for (let index = 1; index < target.points.length; index += 1) {
      context.lineTo(target.points[index]!.x, target.points[index]!.y);
    }
    context.stroke();
  } else {
    context.lineWidth = 4.5;
    context.beginPath();
    context.moveTo(target.points[0]!.x, target.points[0]!.y);
    for (let index = 1; index < target.points.length; index += 1) {
      context.lineTo(target.points[index]!.x, target.points[index]!.y);
    }
    context.closePath();
    context.stroke();
  }
  context.restore();
}

function highlightProbe(context: CanvasRenderingContext2D, x: number, y: number): void {
  context.save();
  context.strokeStyle = "rgba(255,255,255,0.9)";
  context.lineWidth = 1;
  context.beginPath();
  context.arc(x, y, 10, 0, Math.PI * 2);
  context.stroke();
  context.restore();
}

function slicePlaneCorners(
  plane: VolumeInterpretationSlicePlane,
  volume: VolumeInterpretationVolume
): Array<{ x: number; y: number; z: number }> {
  switch (plane.axis) {
    case "inline":
      return [
        { x: plane.position, y: volume.bounds.minY, z: volume.bounds.minZ },
        { x: plane.position, y: volume.bounds.maxY, z: volume.bounds.minZ },
        { x: plane.position, y: volume.bounds.maxY, z: volume.bounds.maxZ },
        { x: plane.position, y: volume.bounds.minY, z: volume.bounds.maxZ }
      ];
    case "xline":
      return [
        { x: volume.bounds.minX, y: plane.position, z: volume.bounds.minZ },
        { x: volume.bounds.maxX, y: plane.position, z: volume.bounds.minZ },
        { x: volume.bounds.maxX, y: plane.position, z: volume.bounds.maxZ },
        { x: volume.bounds.minX, y: plane.position, z: volume.bounds.maxZ }
      ];
    default:
      return [
        { x: volume.bounds.minX, y: volume.bounds.minY, z: plane.position },
        { x: volume.bounds.maxX, y: volume.bounds.minY, z: plane.position },
        { x: volume.bounds.maxX, y: volume.bounds.maxY, z: plane.position },
        { x: volume.bounds.minX, y: volume.bounds.maxY, z: plane.position }
      ];
  }
}

function polygonCenter(points: Point2D[]): Point2D {
  const sum = points.reduce(
    (accumulator, point) => ({
      x: accumulator.x + point.x,
      y: accumulator.y + point.y,
      depth: accumulator.depth + point.depth
    }),
    { x: 0, y: 0, depth: 0 }
  );
  return {
    x: sum.x / Math.max(1, points.length),
    y: sum.y / Math.max(1, points.length),
    depth: sum.depth / Math.max(1, points.length)
  };
}

function polygonBounds(points: Point2D[]): { minX: number; minY: number; maxX: number; maxY: number } {
  return {
    minX: Math.min(...points.map((point) => point.x)),
    minY: Math.min(...points.map((point) => point.y)),
    maxX: Math.max(...points.map((point) => point.x)),
    maxY: Math.max(...points.map((point) => point.y))
  };
}

function samplePalette(ratio: number): string {
  const clamped = Math.max(0, Math.min(1, ratio));
  const hue = 260 - clamped * 220;
  return `hsl(${hue} 78% 58%)`;
}

function withAlpha(color: string, alpha: number): string {
  const clampedAlpha = Math.max(0, Math.min(1, alpha));
  if (color.startsWith("#")) {
    const normalized =
      color.length === 4
        ? `#${color[1]}${color[1]}${color[2]}${color[2]}${color[3]}${color[3]}`
        : color;
    if (normalized.length === 7) {
      const red = parseInt(normalized.slice(1, 3), 16);
      const green = parseInt(normalized.slice(3, 5), 16);
      const blue = parseInt(normalized.slice(5, 7), 16);
      return `rgba(${red}, ${green}, ${blue}, ${clampedAlpha})`;
    }
  }
  if (color.startsWith("hsl(") && color.endsWith(")")) {
    return `${color.slice(0, -1)} / ${clampedAlpha})`;
  }
  return color;
}

function average(values: number[]): number {
  return values.reduce((sum, value) => sum + value, 0) / Math.max(1, values.length);
}

function minMax(values: Float32Array): [number, number] {
  let min = Number.POSITIVE_INFINITY;
  let max = Number.NEGATIVE_INFINITY;
  for (const value of values) {
    min = Math.min(min, value);
    max = Math.max(max, value);
  }
  return [min, max];
}

function pickScore(target: ProjectedTarget, screenX: number, screenY: number): number | null {
  if (target.type === "point") {
    const distance = Math.hypot(target.point.x - screenX, target.point.y - screenY);
    return distance <= target.radius ? distance : null;
  }
  if (target.type === "polyline") {
    const distance = polylineDistance(target.points, screenX, screenY);
    return distance <= Math.max(8, target.strokeWidth + 4) ? distance : null;
  }
  if (pointInPolygon(target.points, screenX, screenY)) {
    return 0;
  }
  const distance = polygonDistance(target.points, screenX, screenY);
  return distance <= 8 ? distance : null;
}

function rankProjectedTargets(
  targets: ProjectedTarget[],
  screenX: number,
  screenY: number
): Array<{ target: ProjectedTarget; hit: boolean; score: number | null }> {
  return targets
    .map((target) => {
      const score = pickScore(target, screenX, screenY);
      return {
        target,
        hit: score !== null,
        score
      };
    })
    .sort((left, right) => compareCandidateRank(left, right));
}

function compareCandidateRank(
  left: { target: ProjectedTarget; hit: boolean; score: number | null },
  right: { target: ProjectedTarget; hit: boolean; score: number | null }
): number {
  if (left.hit !== right.hit) {
    return left.hit ? -1 : 1;
  }
  const leftScore = left.score ?? Number.POSITIVE_INFINITY;
  const rightScore = right.score ?? Number.POSITIVE_INFINITY;
  if (Math.abs(leftScore - rightScore) > 1e-6) {
    return leftScore - rightScore;
  }
  if (Math.abs(left.target.depth - right.target.depth) > 1e-6) {
    return left.target.depth - right.target.depth;
  }
  return left.target.pick.itemId.localeCompare(right.target.pick.itemId);
}

function toDebugCandidate(
  target: ProjectedTarget,
  hit: boolean,
  score: number | null
): VolumeInterpretationPickDebugCandidate {
  return {
    targetType: target.type,
    kind: target.pick.kind,
    itemId: target.pick.itemId,
    itemName: target.pick.itemName,
    hit,
    score,
    depth: target.depth,
    screenX: target.pick.screenX,
    screenY: target.pick.screenY,
    worldX: target.pick.worldX,
    worldY: target.pick.worldY,
    worldZ: target.pick.worldZ
  };
}

function polylineDistance(points: Point2D[], x: number, y: number): number {
  let best = Number.POSITIVE_INFINITY;
  for (let index = 0; index < points.length - 1; index += 1) {
    best = Math.min(best, segmentDistance(points[index]!, points[index + 1]!, x, y));
  }
  return best;
}

function polygonDistance(points: Point2D[], x: number, y: number): number {
  let best = Number.POSITIVE_INFINITY;
  for (let index = 0; index < points.length; index += 1) {
    const next = (index + 1) % points.length;
    best = Math.min(best, segmentDistance(points[index]!, points[next]!, x, y));
  }
  return best;
}

function segmentDistance(a: Point2D, b: Point2D, x: number, y: number): number {
  const dx = b.x - a.x;
  const dy = b.y - a.y;
  const lengthSquared = dx * dx + dy * dy;
  if (lengthSquared <= 1e-6) {
    return Math.hypot(x - a.x, y - a.y);
  }
  const ratio = Math.max(0, Math.min(1, ((x - a.x) * dx + (y - a.y) * dy) / lengthSquared));
  const projectedX = a.x + ratio * dx;
  const projectedY = a.y + ratio * dy;
  return Math.hypot(x - projectedX, y - projectedY);
}

function pointInPolygon(points: Point2D[], x: number, y: number): boolean {
  let inside = false;
  for (let left = 0, right = points.length - 1; left < points.length; right = left++) {
    const pointLeft = points[left]!;
    const pointRight = points[right]!;
    const intersects =
      pointLeft.y > y !== pointRight.y > y &&
      x < ((pointRight.x - pointLeft.x) * (y - pointLeft.y)) / Math.max(1e-6, pointRight.y - pointLeft.y) + pointLeft.x;
    if (intersects) {
      inside = !inside;
    }
  }
  return inside;
}
