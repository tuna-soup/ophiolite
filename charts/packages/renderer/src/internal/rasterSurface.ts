export interface RasterSurfaceMetrics {
  cssWidth: number;
  cssHeight: number;
  pixelRatio: number;
  pixelWidth: number;
  pixelHeight: number;
}

export interface RasterRect {
  x: number;
  y: number;
  width: number;
  height: number;
}

export function resolveClampedDevicePixelRatio(rawRatio?: number): number {
  const ratio =
    typeof rawRatio === "number" && Number.isFinite(rawRatio)
      ? rawRatio
      : typeof window !== "undefined"
        ? window.devicePixelRatio || 1
        : 1;
  return Math.max(1, Math.min(2, ratio));
}

export function createRasterSurfaceMetrics(
  cssWidth: number,
  cssHeight: number,
  pixelRatio = resolveClampedDevicePixelRatio()
): RasterSurfaceMetrics {
  const safeCssWidth = Math.max(1, Math.round(cssWidth));
  const safeCssHeight = Math.max(1, Math.round(cssHeight));
  return {
    cssWidth: safeCssWidth,
    cssHeight: safeCssHeight,
    pixelRatio,
    pixelWidth: Math.max(1, Math.round(safeCssWidth * pixelRatio)),
    pixelHeight: Math.max(1, Math.round(safeCssHeight * pixelRatio))
  };
}

export function resizeCanvasBackingStore(canvas: HTMLCanvasElement, surface: RasterSurfaceMetrics): boolean {
  if (canvas.width === surface.pixelWidth && canvas.height === surface.pixelHeight) {
    return false;
  }
  canvas.width = surface.pixelWidth;
  canvas.height = surface.pixelHeight;
  return true;
}

export function applyCanvasSurfaceTransform(
  context: CanvasRenderingContext2D,
  surface: RasterSurfaceMetrics
): void {
  context.setTransform(surface.pixelRatio, 0, 0, surface.pixelRatio, 0, 0);
}

export function scaleRasterRect(rect: RasterRect, pixelRatio: number): RasterRect {
  const x = Math.round(rect.x * pixelRatio);
  const y = Math.round(rect.y * pixelRatio);
  const right = Math.round((rect.x + rect.width) * pixelRatio);
  const bottom = Math.round((rect.y + rect.height) * pixelRatio);
  return {
    x,
    y,
    width: Math.max(1, right - x),
    height: Math.max(1, bottom - y)
  };
}
