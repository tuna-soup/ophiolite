import {
  buildWellCorrelationLayoutCache,
  type WellCorrelationLayoutCache
} from "./well-correlation-layout-cache";
import type { CorrelationPanelLayout, Rect } from "./layout";
import {
  buildWellCorrelationHeaderRows,
  chooseWellCorrelationDepthStep,
  formatWellCorrelationAxisValue
} from "./well-correlation-presentation";
import type { WellCorrelationViewport } from "@ophiolite/charts-data-models";
import type { NormalizedWellPanelModel } from "./well-panel-normalize";

export interface WellCorrelationChromeAxisLabels {
  min: string;
  max: string;
}

export interface WellCorrelationChromeHeaderRow {
  label: string;
  color: string;
  axisLabels: WellCorrelationChromeAxisLabels | null;
}

export interface WellCorrelationChromeDepthTick {
  depth: number;
  label: string;
  y: number;
}

export interface WellCorrelationChromeTrack {
  wellId: string;
  trackId: string;
  kind: WellCorrelationLayoutCache["trackHits"][number]["track"]["kind"];
  headerRect: Rect;
  bodyRect: Rect;
  headerRows: WellCorrelationChromeHeaderRow[];
  depthTicks: WellCorrelationChromeDepthTick[];
}

export interface WellCorrelationChromeColumn {
  wellId: string;
  wellName: string;
  headerRect: Rect;
  tracks: WellCorrelationChromeTrack[];
}

export interface WellCorrelationChromeModel {
  layout: CorrelationPanelLayout;
  columns: WellCorrelationChromeColumn[];
}

export function buildWellCorrelationChromeModel(
  panel: NormalizedWellPanelModel,
  viewport: WellCorrelationViewport,
  width: number,
  height: number
): WellCorrelationChromeModel {
  const layoutCache = buildWellCorrelationLayoutCache(panel, width, height);
  const depthTicks = buildDepthTicks(viewport);

  return {
    layout: layoutCache.layout,
    columns: layoutCache.layout.columns.flatMap((column) => {
      const hits = layoutCache.trackHitsByWellId.get(column.wellId);
      const well = hits?.[0]?.well;
      if (!well || !hits) {
        return [];
      }

      return [{
        wellId: column.wellId,
        wellName: well.name,
        headerRect: column.headerRect,
        tracks: hits.map((hit) => ({
          wellId: column.wellId,
          trackId: hit.trackFrame.trackId,
          kind: hit.track.kind,
          headerRect: hit.trackFrame.headerRect,
          bodyRect: hit.trackFrame.bodyRect,
          headerRows: buildWellCorrelationHeaderRows(hit.track, well.nativeDepthDatum).map((row) => ({
            label: row.label,
            color: row.color,
            axisLabels: row.axis
              ? {
                  min: formatWellCorrelationAxisValue(row.axis.min),
                  max: formatWellCorrelationAxisValue(row.axis.max)
                }
              : null
          })),
          depthTicks:
            hit.track.kind === "reference"
              ? depthTicks.map((tick) => ({
                  depth: tick,
                  label: tick.toFixed(0),
                  y: depthToScreenY(hit.trackFrame.bodyRect, viewport, tick)
                }))
              : []
        }))
      }] satisfies WellCorrelationChromeColumn[];
    })
  };
}

function buildDepthTicks(viewport: WellCorrelationViewport): number[] {
  const ticks: number[] = [];
  const step = chooseWellCorrelationDepthStep(viewport.depthEnd - viewport.depthStart);
  const firstTick = Math.ceil(viewport.depthStart / step) * step;
  for (let depth = firstTick; depth <= viewport.depthEnd; depth += step) {
    ticks.push(depth);
  }
  return ticks;
}

function depthToScreenY(rect: Rect, viewport: WellCorrelationViewport, depth: number): number {
  return rect.y + ((depth - viewport.depthStart) / Math.max(1e-6, viewport.depthEnd - viewport.depthStart)) * rect.height;
}
