import {
  layoutWellCorrelationPanel,
  type CorrelationPanelLayout,
  type TrackFrame,
  type WellColumnFrame
} from "./layout";
import type {
  NormalizedTrack,
  NormalizedWellColumn,
  NormalizedWellPanelModel
} from "./well-panel-normalize";

export interface WellCorrelationTrackHit {
  well: NormalizedWellColumn;
  column: WellColumnFrame;
  track: NormalizedTrack;
  trackFrame: TrackFrame;
}

export interface WellCorrelationLayoutCache {
  panel: NormalizedWellPanelModel;
  width: number;
  height: number;
  layout: CorrelationPanelLayout;
  trackHits: WellCorrelationTrackHit[];
  trackHitsByWellId: Map<string, readonly WellCorrelationTrackHit[]>;
}

export function buildWellCorrelationLayoutCache(
  panel: NormalizedWellPanelModel,
  width: number,
  height: number
): WellCorrelationLayoutCache {
  const layout = layoutWellCorrelationPanel(panel, width, height);
  const trackHits: WellCorrelationTrackHit[] = [];
  const trackHitsByWellId = new Map<string, readonly WellCorrelationTrackHit[]>();
  const wellsById = new Map(panel.wells.map((well) => [well.id, well] as const));

  for (const column of layout.columns) {
    const well = wellsById.get(column.wellId);
    if (!well) {
      continue;
    }
    const trackById = new Map(well.tracks.map((track) => [track.id, track] as const));
    const wellTrackHits: WellCorrelationTrackHit[] = [];
    for (const trackFrame of column.trackFrames) {
      const track = trackById.get(trackFrame.trackId);
      if (!track) {
        continue;
      }
      const hit = {
        well,
        column,
        track,
        trackFrame
      } satisfies WellCorrelationTrackHit;
      trackHits.push(hit);
      wellTrackHits.push(hit);
    }
    trackHitsByWellId.set(well.id, wellTrackHits);
  }

  return {
    panel,
    width,
    height,
    layout,
    trackHits,
    trackHitsByWellId
  };
}

export function hitTestWellTrack(
  cache: WellCorrelationLayoutCache,
  x: number,
  y: number
): WellCorrelationTrackHit | null {
  for (const hit of cache.trackHits) {
    if (pointInRect(x, y, hit.trackFrame.bodyRect)) {
      return hit;
    }
  }
  return null;
}

export function trackHitsForWell(
  cache: WellCorrelationLayoutCache,
  wellId: string
): readonly WellCorrelationTrackHit[] {
  return cache.trackHitsByWellId.get(wellId) ?? [];
}

function pointInRect(
  x: number,
  y: number,
  rect: { x: number; y: number; width: number; height: number }
): boolean {
  return x >= rect.x && x <= rect.x + rect.width && y >= rect.y && y <= rect.y + rect.height;
}
