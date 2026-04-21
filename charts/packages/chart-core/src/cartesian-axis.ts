import type {
  CartesianAxisId,
  CartesianAxisOverride,
  CartesianAxisOverrides
} from "@ophiolite/charts-data-models";

interface RectLike {
  x: number;
  y: number;
  width: number;
  height: number;
}

export interface CartesianAxisBandRects {
  x: RectLike;
  y: RectLike;
}

const DEFAULT_TICK_COUNT = 6;

export function cloneCartesianAxisOverride(override: CartesianAxisOverride | undefined): CartesianAxisOverride | undefined {
  return override ? { ...override } : undefined;
}

export function cloneCartesianAxisOverrides(overrides: CartesianAxisOverrides | null | undefined): CartesianAxisOverrides {
  return {
    x: cloneCartesianAxisOverride(overrides?.x),
    y: cloneCartesianAxisOverride(overrides?.y)
  };
}

export function resolveCartesianAxisTitle(
  fallbackLabel: string,
  baseLabel: string | null | undefined,
  baseUnit: string | null | undefined,
  override: CartesianAxisOverride | null | undefined
): string {
  const label = override?.label?.trim() || baseLabel?.trim() || fallbackLabel;
  const unit = override?.unit?.trim() || baseUnit?.trim() || "";
  return unit ? `${label} (${unit})` : label;
}

export function buildCartesianTicks(
  min: number,
  max: number,
  count: number = DEFAULT_TICK_COUNT
): number[] {
  if (!Number.isFinite(min) || !Number.isFinite(max) || max <= min || count <= 1) {
    return [min];
  }

  return Array.from({ length: count }, (_, index) => min + ((max - min) * index) / (count - 1));
}

export function resolveCartesianTickCount(
  override: CartesianAxisOverride | null | undefined,
  fallback: number = DEFAULT_TICK_COUNT
): number {
  const next = Math.round(override?.tickCount ?? fallback);
  return clamp(next, 2, 12);
}

export function formatCartesianTick(
  value: number,
  tickFormat: string | null | undefined
): string {
  if (!Number.isFinite(value)) {
    return "0";
  }

  const normalized = tickFormat?.trim().toLowerCase() ?? "auto";
  if (normalized === "scientific") {
    return value.toExponential(2);
  }

  if (normalized.startsWith("fixed:")) {
    const digits = clamp(Number.parseInt(normalized.slice("fixed:".length), 10) || 0, 0, 6);
    return value.toFixed(digits);
  }

  const abs = Math.abs(value);
  if (abs >= 1_000) {
    return value.toFixed(0);
  }
  if (abs >= 100) {
    return value.toFixed(1);
  }
  if (abs >= 10) {
    return value.toFixed(2);
  }
  if (abs >= 1) {
    return value.toFixed(3).replace(/\.?0+$/, "");
  }
  return value.toFixed(4).replace(/\.?0+$/, "");
}

export function getCartesianAxisBandRects(
  plotRect: RectLike,
  stageWidth: number,
  stageHeight: number
): CartesianAxisBandRects {
  return {
    x: {
      x: plotRect.x,
      y: plotRect.y + plotRect.height,
      width: plotRect.width,
      height: Math.max(1, stageHeight - (plotRect.y + plotRect.height))
    },
    y: {
      x: 0,
      y: plotRect.y,
      width: Math.max(1, plotRect.x),
      height: plotRect.height
    }
  };
}

export function hitTestCartesianAxisBand(
  x: number,
  y: number,
  plotRect: RectLike,
  stageWidth: number,
  stageHeight: number
): CartesianAxisId | null {
  const bandRects = getCartesianAxisBandRects(plotRect, stageWidth, stageHeight);
  if (pointInRect(x, y, bandRects.x)) {
    return "x";
  }
  if (pointInRect(x, y, bandRects.y)) {
    return "y";
  }
  return null;
}

export function applyViewportToAxisOverrides(
  overrides: CartesianAxisOverrides | null | undefined,
  viewport: { xMin: number; xMax: number; yMin: number; yMax: number } | null
): CartesianAxisOverrides {
  const next = cloneCartesianAxisOverrides(overrides);
  if (!viewport) {
    return next;
  }

  next.x = {
    ...next.x,
    min: viewport.xMin,
    max: viewport.xMax
  };
  next.y = {
    ...next.y,
    min: viewport.yMin,
    max: viewport.yMax
  };
  return next;
}

function pointInRect(x: number, y: number, rect: RectLike): boolean {
  return x >= rect.x && x <= rect.x + rect.width && y >= rect.y && y <= rect.y + rect.height;
}

function clamp(value: number, min: number, max: number): number {
  return Math.min(Math.max(value, min), max);
}
