import {
  getSurveyMapPlotRect,
  resolveSurveyMapViewMetrics,
  worldToScreen,
  SURVEY_MAP_MARGIN,
  type SurveyMapRect
} from "@ophiolite/charts-core";
import type { SurveyMapPoint, SurveyMapScalarField, SurveyMapViewport, SurveyMapWell } from "@ophiolite/charts-data-models";
import type { SurveyMapRenderFrame, SurveyMapRendererAdapter } from "./adapter";

const DEFAULT_BACKGROUND = "#f4f2ee";
const PLOT_BACKGROUND = "#fcfbf8";
const AXIS_COLOR = "#5f6468";
const GRID_COLOR = "rgba(58, 72, 84, 0.14)";
const OUTLINE_COLOR = "rgba(38, 49, 59, 0.9)";
const WELL_COLOR = "#f7fbff";
const TRAJECTORY_COLOR = "rgba(244, 247, 250, 0.72)";
const SELECTED_WELL_COLOR = "#ffe38a";
const HOVER_WELL_COLOR = "#ffffff";

export class SurveyMapCanvasRenderer implements SurveyMapRendererAdapter {
  private container: HTMLElement | null = null;
  private canvas: HTMLCanvasElement | null = null;
  private context: CanvasRenderingContext2D | null = null;

  mount(container: HTMLElement): void {
    this.dispose();
    this.container = container;
    this.canvas = document.createElement("canvas");
    this.canvas.className = "ophiolite-charts-survey-map-canvas";
    this.canvas.style.width = "100%";
    this.canvas.style.height = "100%";
    this.context = this.canvas.getContext("2d");
    container.appendChild(this.canvas);
  }

  render(frame: SurveyMapRenderFrame): void {
    if (!this.container || !this.canvas || !this.context) {
      return;
    }

    const width = Math.max(1, this.container.clientWidth);
    const height = Math.max(1, this.container.clientHeight);
    const dpr = window.devicePixelRatio || 1;

    if (this.canvas.width !== Math.round(width * dpr) || this.canvas.height !== Math.round(height * dpr)) {
      this.canvas.width = Math.round(width * dpr);
      this.canvas.height = Math.round(height * dpr);
    }

    this.context.setTransform(dpr, 0, 0, dpr, 0, 0);
    this.context.clearRect(0, 0, width, height);

    const map = frame.state.map;
    const viewport = frame.state.viewport;

    this.context.fillStyle = map?.background ?? DEFAULT_BACKGROUND;
    this.context.fillRect(0, 0, width, height);

    const plotRect = getSurveyMapPlotRect(width, height);
    this.context.fillStyle = PLOT_BACKGROUND;
    this.context.fillRect(plotRect.x, plotRect.y, plotRect.width, plotRect.height);

    if (!map || !viewport) {
      this.drawFrame(plotRect);
      return;
    }

    const viewMetrics = resolveSurveyMapViewMetrics(viewport, plotRect);

    this.context.save();
    this.context.beginPath();
    this.context.rect(viewMetrics.drawRect.x, viewMetrics.drawRect.y, viewMetrics.drawRect.width, viewMetrics.drawRect.height);
    this.context.clip();

    if (map.scalarField) {
      this.drawScalarField(map.scalarField, viewport, plotRect);
    } else {
      this.context.fillStyle = "#f7f5f1";
      this.context.fillRect(viewMetrics.drawRect.x, viewMetrics.drawRect.y, viewMetrics.drawRect.width, viewMetrics.drawRect.height);
    }

    this.drawSurveyAreas(frame, plotRect);
    this.drawWells(frame, plotRect);

    this.context.restore();

    this.drawAxes(map, viewport, plotRect);
    if (map.scalarField) {
      this.drawLegend(map.scalarField, width, plotRect);
    }
    this.drawFrame(plotRect);
  }

  dispose(): void {
    if (this.canvas?.parentNode) {
      this.canvas.parentNode.removeChild(this.canvas);
    }
    this.container = null;
    this.canvas = null;
    this.context = null;
  }

  private drawScalarField(field: SurveyMapScalarField, viewport: SurveyMapViewport, plotRect: SurveyMapRect): void {
    const context = this.context;
    if (!context) {
      return;
    }

    const { min, max } = scalarRange(field);
    const halfStepX = Math.abs(field.step.x) / 2;
    const halfStepY = Math.abs(field.step.y) / 2;

    for (let row = 0; row < field.rows; row += 1) {
      for (let column = 0; column < field.columns; column += 1) {
        const value = field.values[row * field.columns + column];
        if (!Number.isFinite(value)) {
          continue;
        }

        const centerX = field.origin.x + column * field.step.x;
        const centerY = field.origin.y + row * field.step.y;
        const worldLeft = centerX - halfStepX;
        const worldRight = centerX + halfStepX;
        const worldBottom = centerY - halfStepY;
        const worldTop = centerY + halfStepY;
        const topLeft = worldToScreen({ x: worldLeft, y: worldTop }, viewport, plotRect);
        const bottomRight = worldToScreen({ x: worldRight, y: worldBottom }, viewport, plotRect);
        const width = bottomRight.x - topLeft.x;
        const height = bottomRight.y - topLeft.y;

        context.fillStyle = scalarColor(value, min, max);
        context.fillRect(topLeft.x, topLeft.y, width, height);
      }
    }

    context.strokeStyle = GRID_COLOR;
    context.lineWidth = 0.5;
    context.beginPath();

    for (let column = 0; column <= field.columns; column += 1) {
      const x = field.origin.x + (column - 0.5) * field.step.x;
      const top = worldToScreen({ x, y: field.origin.y + (field.rows - 0.5) * field.step.y }, viewport, plotRect);
      const bottom = worldToScreen({ x, y: field.origin.y - 0.5 * field.step.y }, viewport, plotRect);
      context.moveTo(top.x, top.y);
      context.lineTo(bottom.x, bottom.y);
    }

    for (let row = 0; row <= field.rows; row += 1) {
      const y = field.origin.y + (row - 0.5) * field.step.y;
      const left = worldToScreen({ x: field.origin.x - 0.5 * field.step.x, y }, viewport, plotRect);
      const right = worldToScreen({ x: field.origin.x + (field.columns - 0.5) * field.step.x, y }, viewport, plotRect);
      context.moveTo(left.x, left.y);
      context.lineTo(right.x, right.y);
    }

    context.stroke();
  }

  private drawSurveyAreas(frame: SurveyMapRenderFrame, plotRect: SurveyMapRect): void {
    const context = this.context;
    if (!context || !frame.state.viewport || !frame.state.map) {
      return;
    }

    for (const survey of frame.state.map.surveys) {
      if (survey.outline.length < 2) {
        continue;
      }

      context.beginPath();
      survey.outline.forEach((point, index) => {
        const screen = worldToScreen(point, frame.state.viewport!, plotRect);
        if (index === 0) {
          context.moveTo(screen.x, screen.y);
        } else {
          context.lineTo(screen.x, screen.y);
        }
      });
      context.closePath();

      if (survey.fill) {
        context.fillStyle = survey.fill;
        context.fill();
      }

      context.strokeStyle = survey.stroke ?? OUTLINE_COLOR;
      context.lineWidth = 1.4;
      context.stroke();
    }
  }

  private drawWells(frame: SurveyMapRenderFrame, plotRect: SurveyMapRect): void {
    const context = this.context;
    if (!context || !frame.state.viewport || !frame.state.map) {
      return;
    }

    const hoveredWellId = frame.state.probe?.wellId ?? null;

    for (const well of frame.state.map.wells) {
      this.drawTrajectory(well, frame.state.viewport, plotRect);
      const screen = worldToScreen(well.surface, frame.state.viewport, plotRect);
      const selected = frame.state.selectedWellId === well.id;
      const hovered = hoveredWellId === well.id;
      const radius = selected ? 4.8 : hovered ? 4.2 : 3.7;

      context.beginPath();
      context.arc(screen.x, screen.y, radius, 0, Math.PI * 2);
      context.fillStyle = selected
        ? SELECTED_WELL_COLOR
        : hovered
          ? HOVER_WELL_COLOR
          : well.color ?? WELL_COLOR;
      context.fill();
      context.strokeStyle = selected ? "#4f4530" : "#4e5964";
      context.lineWidth = selected ? 1.8 : 1.1;
      context.stroke();

      if (selected || hovered) {
        this.drawWellLabel(well.name, screen.x + 7, screen.y - 8, selected);
      }
    }
  }

  private drawTrajectory(well: SurveyMapWell, viewport: SurveyMapViewport, plotRect: SurveyMapRect): void {
    const context = this.context;
    if (!context || !well.trajectory || well.trajectory.length < 2) {
      return;
    }

    context.beginPath();
    well.trajectory.forEach((point, index) => {
      const screen = worldToScreen(point, viewport, plotRect);
      if (index === 0) {
        context.moveTo(screen.x, screen.y);
      } else {
        context.lineTo(screen.x, screen.y);
      }
    });
    context.strokeStyle = well.color ?? TRAJECTORY_COLOR;
    context.lineWidth = 1.35;
    context.stroke();
  }

  private drawWellLabel(label: string, x: number, y: number, selected: boolean): void {
    if (!this.context) {
      return;
    }

    this.context.font = "600 11px sans-serif";
    const textWidth = this.context.measureText(label).width;
    const boxWidth = textWidth + 10;
    const boxHeight = 18;

    this.context.fillStyle = selected ? "rgba(56, 43, 14, 0.9)" : "rgba(31, 40, 48, 0.86)";
    this.context.fillRect(x, y - boxHeight + 3, boxWidth, boxHeight);
    this.context.fillStyle = "#ffffff";
    this.context.fillText(label, x + 5, y - 4);
  }

  private drawAxes(map: SurveyMapRenderFrame["state"]["map"], viewport: SurveyMapViewport, plotRect: SurveyMapRect): void {
    if (!this.context || !map) {
      return;
    }

    const metrics = resolveSurveyMapViewMetrics(viewport, plotRect);
    this.context.strokeStyle = AXIS_COLOR;
    this.context.fillStyle = AXIS_COLOR;
    this.context.lineWidth = 1;
    this.context.font = "11px sans-serif";

    const xTicks = ticks(viewport.xMin, viewport.xMax, 4);
    const yTicks = ticks(viewport.yMin, viewport.yMax, 4);

    for (const tick of xTicks) {
      const position = worldToScreen({ x: tick, y: viewport.yMin }, viewport, plotRect);
      this.context.beginPath();
      this.context.moveTo(position.x, metrics.drawRect.y + metrics.drawRect.height);
      this.context.lineTo(position.x, metrics.drawRect.y + metrics.drawRect.height + 4);
      this.context.stroke();
      this.context.textAlign = "center";
      this.context.textBaseline = "top";
      this.context.fillText(formatCoordinate(tick), position.x, metrics.drawRect.y + metrics.drawRect.height + 6);
    }

    for (const tick of yTicks) {
      const position = worldToScreen({ x: viewport.xMin, y: tick }, viewport, plotRect);
      this.context.beginPath();
      this.context.moveTo(metrics.drawRect.x - 4, position.y);
      this.context.lineTo(metrics.drawRect.x, position.y);
      this.context.stroke();
      this.context.textAlign = "right";
      this.context.textBaseline = "middle";
      this.context.fillText(formatCoordinate(tick), metrics.drawRect.x - 8, position.y);
    }

    this.context.textAlign = "center";
    this.context.textBaseline = "bottom";
    this.context.fillText(axisLabel(map.xLabel, map.coordinateUnit), plotRect.x + plotRect.width / 2, plotRect.y + plotRect.height + SURVEY_MAP_MARGIN.bottom - 4);

    this.context.save();
    this.context.translate(12, plotRect.y + plotRect.height / 2);
    this.context.rotate(-Math.PI / 2);
    this.context.textAlign = "center";
    this.context.textBaseline = "top";
    this.context.fillText(axisLabel(map.yLabel, map.coordinateUnit), 0, 0);
    this.context.restore();
  }

  private drawLegend(field: SurveyMapScalarField, width: number, plotRect: SurveyMapRect): void {
    if (!this.context) {
      return;
    }

    const { min, max } = scalarRange(field);
    const x = width - SURVEY_MAP_MARGIN.right + 20;
    const y = plotRect.y + 16;
    const barWidth = 12;
    const barHeight = 96;

    for (let index = 0; index < barHeight; index += 1) {
      const value = max - (index / Math.max(1, barHeight - 1)) * (max - min);
      this.context.fillStyle = scalarColor(value, min, max);
      this.context.fillRect(x, y + index, barWidth, 1);
    }

    this.context.strokeStyle = "rgba(41, 52, 62, 0.32)";
    this.context.lineWidth = 1;
    this.context.strokeRect(x, y, barWidth, barHeight);

    this.context.fillStyle = "#4a5158";
    this.context.font = "11px sans-serif";
    this.context.textAlign = "left";
    this.context.textBaseline = "middle";
    this.context.fillText(formatScalar(max), x + barWidth + 6, y + 4);
    this.context.fillText(formatScalar(min), x + barWidth + 6, y + barHeight - 4);
    this.context.textBaseline = "bottom";
    this.context.fillText(axisLabel(field.name, field.unit), x - 2, y - 6);
  }

  private drawFrame(plotRect: SurveyMapRect): void {
    if (!this.context) {
      return;
    }

    this.context.strokeStyle = "rgba(45, 56, 66, 0.42)";
    this.context.lineWidth = 1;
    this.context.strokeRect(plotRect.x, plotRect.y, plotRect.width, plotRect.height);
  }
}

function scalarRange(field: SurveyMapScalarField): { min: number; max: number } {
  const explicitMin = field.minValue;
  const explicitMax = field.maxValue;
  if (Number.isFinite(explicitMin) && Number.isFinite(explicitMax) && explicitMin! < explicitMax!) {
    return { min: explicitMin!, max: explicitMax! };
  }

  let min = Number.POSITIVE_INFINITY;
  let max = Number.NEGATIVE_INFINITY;
  for (const value of field.values) {
    if (!Number.isFinite(value)) {
      continue;
    }
    min = Math.min(min, value);
    max = Math.max(max, value);
  }

  if (!Number.isFinite(min) || !Number.isFinite(max) || min === max) {
    return { min: 0, max: 1 };
  }
  return { min, max };
}

function scalarColor(value: number, min: number, max: number): string {
  const ratio = clamp((value - min) / Math.max(1e-6, max - min), 0, 1);
  const stops = [
    [0, [22, 58, 163]],
    [0.25, [39, 147, 219]],
    [0.5, [71, 212, 221]],
    [0.72, [242, 225, 74]],
    [1, [210, 54, 38]]
  ] as const;

  for (let index = 1; index < stops.length; index += 1) {
    const [rightOffset, rightColor] = stops[index]!;
    const [leftOffset, leftColor] = stops[index - 1]!;
    if (ratio <= rightOffset) {
      const localRatio = (ratio - leftOffset) / Math.max(1e-6, rightOffset - leftOffset);
      const red = Math.round(leftColor[0] + (rightColor[0] - leftColor[0]) * localRatio);
      const green = Math.round(leftColor[1] + (rightColor[1] - leftColor[1]) * localRatio);
      const blue = Math.round(leftColor[2] + (rightColor[2] - leftColor[2]) * localRatio);
      return `rgb(${red}, ${green}, ${blue})`;
    }
  }

  return "rgb(210, 54, 38)";
}

function ticks(min: number, max: number, count: number): number[] {
  if (!Number.isFinite(min) || !Number.isFinite(max) || max <= min) {
    return [min];
  }
  const step = (max - min) / Math.max(1, count - 1);
  return Array.from({ length: count }, (_, index) => min + index * step);
}

function axisLabel(name: string | undefined, unit: string | undefined): string {
  if (name && unit) {
    return `${name} (${unit})`;
  }
  return name ?? unit ?? "";
}

function formatCoordinate(value: number): string {
  return Math.abs(value) >= 1000 ? value.toFixed(0) : value.toFixed(1);
}

function formatScalar(value: number): string {
  return Math.abs(value) >= 100 ? value.toFixed(0) : value.toFixed(1);
}

function clamp(value: number, min: number, max: number): number {
  return Math.min(Math.max(value, min), max);
}
