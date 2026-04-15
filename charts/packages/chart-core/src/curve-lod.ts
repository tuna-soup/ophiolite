import type { CurveSeries, DepthBinnedPoint } from "@ophiolite/charts-data-models";
import { mapNativeDepthToPanelDepth } from "./depth-mapping";

export function buildDepthBinnedCurveLod(
  series: CurveSeries,
  mapping: { nativeDepth: number; panelDepth: number }[],
  depthStart: number,
  depthEnd: number,
  bucketCount: number
): DepthBinnedPoint[] {
  const bins = Array.from({ length: Math.max(1, bucketCount) }, () => ({
    depth: 0,
    minValue: Number.POSITIVE_INFINITY,
    maxValue: Number.NEGATIVE_INFINITY,
    count: 0
  }));

  const depthSpan = Math.max(1e-6, depthEnd - depthStart);

  for (let index = 0; index < series.values.length; index += 1) {
    const panelDepth = mapNativeDepthToPanelDepth(mapping, series.nativeDepths[index]!);
    if (panelDepth < depthStart || panelDepth > depthEnd) {
      continue;
    }
    const value = series.values[index]!;
    const bucketIndex = Math.min(
      bins.length - 1,
      Math.max(0, Math.floor(((panelDepth - depthStart) / depthSpan) * bins.length))
    );
    const bucket = bins[bucketIndex]!;
    bucket.depth += panelDepth;
    bucket.minValue = Math.min(bucket.minValue, value);
    bucket.maxValue = Math.max(bucket.maxValue, value);
    bucket.count += 1;
  }

  return bins
    .filter((bucket) => bucket.count > 0)
    .map((bucket) => ({
      depth: bucket.depth / bucket.count,
      minValue: bucket.minValue,
      maxValue: bucket.maxValue
    }));
}

