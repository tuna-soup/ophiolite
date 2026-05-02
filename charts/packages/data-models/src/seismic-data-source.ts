import type { ComparisonMode, SectionPayload, SectionViewport } from "./seismic";
import { resolveLogicalSectionDimensions, sectionWindowCoversViewport } from "./seismic";

export interface SeismicSectionWindowRequest {
  viewport: SectionViewport;
  traceRange: readonly [number, number];
  sampleRange: readonly [number, number];
  lod: number;
  reason: "viewport" | "prefetch" | "initial";
  comparisonMode?: ComparisonMode;
}

export interface SeismicSectionDataSourceContext {
  chartId?: string;
  viewId?: string;
  currentSection: SectionPayload | null;
  signal?: AbortSignal;
}

export interface SeismicSectionDataSourceCachePolicy {
  maxEntries?: number;
  maxBytes?: number;
}

export interface SeismicSectionDataSourcePrefetchPolicy {
  adjacentViewportWindows?: number;
}

export interface SeismicSectionDataSourceMetrics {
  viewportRequests: number;
  cacheHits: number;
  fetches: number;
  fetchErrors: number;
  prefetchRequests: number;
  prefetchCacheHits: number;
  prefetchErrors: number;
  evictions: number;
  cacheEntries: number;
  cacheBytes: number;
}

export interface SeismicSectionDataSource {
  id?: string;
  debounceMs?: number;
  forceLoad?: boolean | ((section: SectionPayload, viewport: SectionViewport) => boolean);
  halo?: {
    traces?: number;
    samples?: number;
  };
  cachePolicy?: SeismicSectionDataSourceCachePolicy;
  prefetchPolicy?: SeismicSectionDataSourcePrefetchPolicy;
  chooseLod?: (viewport: SectionViewport) => number;
  getRequestKey?: (request: SeismicSectionWindowRequest) => string;
  estimateBytes?: (section: SectionPayload) => number;
  loadWindow: (
    request: SeismicSectionWindowRequest,
    context: SeismicSectionDataSourceContext
  ) => Promise<SectionPayload>;
}

export interface SeismicSectionDataSourceState {
  status: "idle" | "scheduled" | "loading" | "ready" | "error";
  request: SeismicSectionWindowRequest | null;
  source?: "section" | "cache" | "network";
  metrics: SeismicSectionDataSourceMetrics;
  cacheKey?: string | null;
  errorMessage?: string | null;
}

export interface SeismicSectionViewportLoaderOptions {
  traceHalo?: number;
  sampleHalo?: number;
  chooseLod?: (viewport: SectionViewport) => number;
}

export function createSeismicSectionWindowRequest(
  section: SectionPayload,
  viewport: SectionViewport,
  options: SeismicSectionViewportLoaderOptions = {}
): SeismicSectionWindowRequest {
  const logical = resolveLogicalSectionDimensions(section);
  const traceHalo = Math.max(0, Math.floor(options.traceHalo ?? 32));
  const sampleHalo = Math.max(0, Math.floor(options.sampleHalo ?? 64));
  const chooseLod = options.chooseLod ?? defaultSeismicSectionLod;

  return {
    viewport: { ...viewport },
    traceRange: [
      clamp(Math.floor(viewport.traceStart) - traceHalo, 0, logical.traces),
      clamp(Math.ceil(viewport.traceEnd) + traceHalo, 1, logical.traces)
    ],
    sampleRange: [
      clamp(Math.floor(viewport.sampleStart) - sampleHalo, 0, logical.samples),
      clamp(Math.ceil(viewport.sampleEnd) + sampleHalo, 1, logical.samples)
    ],
    lod: chooseLod(viewport),
    reason: "viewport"
  };
}

export function needsSeismicSectionWindowLoad(section: SectionPayload | null, viewport: SectionViewport | null): boolean {
  if (!section || !viewport) {
    return false;
  }
  return !sectionWindowCoversViewport(section, viewport);
}

export function defaultSeismicSectionLod(viewport: SectionViewport): number {
  const traceSpan = Math.max(1, viewport.traceEnd - viewport.traceStart);
  const sampleSpan = Math.max(1, viewport.sampleEnd - viewport.sampleStart);
  const dominantSpan = Math.max(traceSpan, sampleSpan);
  if (dominantSpan <= 384) {
    return 0;
  }
  if (dominantSpan <= 768) {
    return 1;
  }
  if (dominantSpan <= 1536) {
    return 2;
  }
  return 3;
}

export function createSeismicSectionDataSourceMetrics(): SeismicSectionDataSourceMetrics {
  return {
    viewportRequests: 0,
    cacheHits: 0,
    fetches: 0,
    fetchErrors: 0,
    prefetchRequests: 0,
    prefetchCacheHits: 0,
    prefetchErrors: 0,
    evictions: 0,
    cacheEntries: 0,
    cacheBytes: 0
  };
}

function clamp(value: number, min: number, max: number): number {
  return Math.min(Math.max(value, min), max);
}
