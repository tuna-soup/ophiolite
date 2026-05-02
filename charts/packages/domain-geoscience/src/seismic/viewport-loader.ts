import type {
  SectionPayload,
  SectionViewport,
  SeismicSectionDataSource,
  SeismicSectionDataSourceMetrics,
  SeismicSectionDataSourceState,
  SeismicSectionWindowRequest
} from "@ophiolite/charts-data-models";
import {
  createSeismicSectionDataSourceMetrics,
  createSeismicSectionWindowRequest,
  needsSeismicSectionWindowLoad,
  resolveLogicalSectionDimensions
} from "@ophiolite/charts-data-models";

export interface SectionViewportLoaderContext {
  chartId?: string;
  viewId?: string;
}

export interface SectionViewportLoaderCallbacks {
  onSectionLoaded?: (section: SectionPayload, request: SeismicSectionWindowRequest, cacheKey: string | null) => void;
  onStateChange?: (state: SeismicSectionDataSourceState) => void;
}

interface LoaderCacheEntry {
  section: SectionPayload;
  bytes: number;
  lastUsedAt: number;
}

export class SectionViewportLoader {
  private readonly cache = new Map<string, LoaderCacheEntry>();
  private readonly callbacks: SectionViewportLoaderCallbacks;
  private dataSource: SeismicSectionDataSource | null;
  private readonly context: SectionViewportLoaderContext;
  private readonly options: {
    traceHalo: number;
    sampleHalo: number;
  };
  private metrics: SeismicSectionDataSourceMetrics = createSeismicSectionDataSourceMetrics();
  private currentState: SeismicSectionDataSourceState = {
    status: "idle",
    request: null,
    source: undefined,
    metrics: createSeismicSectionDataSourceMetrics(),
    cacheKey: null,
    errorMessage: null
  };
  private timer: ReturnType<typeof setTimeout> | null = null;
  private abortController: AbortController | null = null;
  private readonly prefetchAbortControllers = new Set<AbortController>();
  private requestSequence = 0;
  private lastScheduledKey: string | null = null;

  constructor(
    dataSource: SeismicSectionDataSource | null,
    context: SectionViewportLoaderContext,
    callbacks: SectionViewportLoaderCallbacks = {}
  ) {
    this.dataSource = dataSource;
    this.context = context;
    this.callbacks = callbacks;
    this.options = {
      traceHalo: Math.max(0, dataSource?.halo?.traces ?? 32),
      sampleHalo: Math.max(0, dataSource?.halo?.samples ?? 64)
    };
  }

  setDataSource(dataSource: SeismicSectionDataSource | null): void {
    this.cancelPending();
    this.dataSource = dataSource;
    this.cache.clear();
    this.metrics = createSeismicSectionDataSourceMetrics();
    this.lastScheduledKey = null;
    this.emitState({
      status: "idle",
      request: null,
      source: undefined,
      cacheKey: null,
      errorMessage: null
    });
  }

  sync(section: SectionPayload | null, viewport: SectionViewport | null): void {
    if (!this.dataSource || !section || !viewport) {
      this.emitState({
        status: "idle",
        request: null,
        source: undefined,
        cacheKey: null,
        errorMessage: null
      });
      return;
    }
    if (!this.shouldForceLoad(section, viewport) && !needsSeismicSectionWindowLoad(section, viewport)) {
      this.emitState({
        status: "ready",
        request: null,
        source: "section",
        cacheKey: null,
        errorMessage: null
      });
      return;
    }

    const request = createSeismicSectionWindowRequest(section, viewport, {
      traceHalo: this.dataSource.halo?.traces ?? this.options.traceHalo,
      sampleHalo: this.dataSource.halo?.samples ?? this.options.sampleHalo,
      chooseLod: this.dataSource.chooseLod
    });
    this.metrics.viewportRequests += 1;
    const cacheKey = this.dataSource.getRequestKey?.(request) ?? JSON.stringify(request);
    if (this.cache.has(cacheKey)) {
      this.metrics.cacheHits += 1;
      this.touchCacheEntry(cacheKey);
      this.emitState({
        status: "ready",
        request,
        source: "cache",
        cacheKey,
        errorMessage: null
      });
      this.callbacks.onSectionLoaded?.(this.cache.get(cacheKey)!.section, request, cacheKey);
      return;
    }
    if (cacheKey === this.lastScheduledKey) {
      return;
    }

    this.cancelPending();
    this.lastScheduledKey = cacheKey;
    const debounceMs = Math.max(0, this.dataSource.debounceMs ?? 80);
    this.emitState({
      status: "scheduled",
      request,
      source: "network",
      cacheKey,
      errorMessage: null
    });
    this.timer = setTimeout(() => {
      this.timer = null;
      void this.loadRequest(request, section, cacheKey);
    }, debounceMs);
  }

  private shouldForceLoad(section: SectionPayload, viewport: SectionViewport): boolean {
    const forceLoad = this.dataSource?.forceLoad;
    if (typeof forceLoad === "function") {
      return forceLoad(section, viewport);
    }
    return Boolean(forceLoad);
  }

  dispose(): void {
    this.cancelPending();
    this.cache.clear();
  }

  private async loadRequest(
    request: SeismicSectionWindowRequest,
    currentSection: SectionPayload,
    cacheKey: string
  ): Promise<void> {
    if (!this.dataSource) {
      return;
    }

    const sequence = ++this.requestSequence;
    const abortController = new AbortController();
    this.abortController = abortController;
    this.metrics.fetches += 1;
    this.emitState({
      status: "loading",
      request,
      source: "network",
      cacheKey,
      errorMessage: null
    });

    try {
      const section = await this.dataSource.loadWindow(request, {
        chartId: this.context.chartId,
        viewId: this.context.viewId,
        currentSection,
        signal: abortController.signal
      });
      if (sequence !== this.requestSequence) {
        return;
      }
      this.storeCacheEntry(cacheKey, section);
      this.emitState({
        status: "ready",
        request,
        source: "network",
        cacheKey,
        errorMessage: null
      });
      this.callbacks.onSectionLoaded?.(section, request, cacheKey);
      void this.prefetchAdjacentWindows(request, section);
    } catch (error) {
      if (abortController.signal.aborted) {
        return;
      }
      this.metrics.fetchErrors += 1;
      this.emitState({
        status: "error",
        request,
        source: "network",
        cacheKey,
        errorMessage: error instanceof Error ? error.message : String(error)
      });
    }
  }

  private emitState(
    state: Omit<SeismicSectionDataSourceState, "metrics"> & { metrics?: SeismicSectionDataSourceMetrics }
  ): void {
    this.currentState = {
      status: state.status,
      request: state.request
        ? {
            ...state.request,
            viewport: { ...state.request.viewport },
            traceRange: [...state.request.traceRange] as [number, number],
            sampleRange: [...state.request.sampleRange] as [number, number]
          }
        : null,
      source: state.source,
      metrics: cloneMetrics(state.metrics ?? this.metrics),
      cacheKey: state.cacheKey ?? null,
      errorMessage: state.errorMessage ?? null
    };
    this.callbacks.onStateChange?.(this.currentState);
  }

  private storeCacheEntry(cacheKey: string, section: SectionPayload): void {
    const existing = this.cache.get(cacheKey);
    if (existing) {
      this.metrics.cacheBytes -= existing.bytes;
    }
    const bytes = this.estimateBytes(section);
    this.cache.set(cacheKey, {
      section,
      bytes,
      lastUsedAt: nowMs()
    });
    this.metrics.cacheEntries = this.cache.size;
    this.metrics.cacheBytes += bytes;
    this.trimCache();
  }

  private touchCacheEntry(cacheKey: string): void {
    const entry = this.cache.get(cacheKey);
    if (!entry) {
      return;
    }
    entry.lastUsedAt = nowMs();
  }

  private trimCache(): void {
    const maxEntries = Math.max(0, this.dataSource?.cachePolicy?.maxEntries ?? Number.POSITIVE_INFINITY);
    const maxBytes = Math.max(0, this.dataSource?.cachePolicy?.maxBytes ?? Number.POSITIVE_INFINITY);
    if (this.cache.size <= maxEntries && this.metrics.cacheBytes <= maxBytes) {
      this.metrics.cacheEntries = this.cache.size;
      return;
    }

    const entries = [...this.cache.entries()].sort((left, right) => left[1].lastUsedAt - right[1].lastUsedAt);
    for (const [cacheKey, entry] of entries) {
      if (this.cache.size <= maxEntries && this.metrics.cacheBytes <= maxBytes) {
        break;
      }
      this.cache.delete(cacheKey);
      this.metrics.evictions += 1;
      this.metrics.cacheBytes -= entry.bytes;
    }
    this.metrics.cacheEntries = this.cache.size;
  }

  private estimateBytes(section: SectionPayload): number {
    return this.dataSource?.estimateBytes?.(section) ?? defaultEstimateSectionBytes(section);
  }

  private async prefetchAdjacentWindows(
    request: SeismicSectionWindowRequest,
    currentSection: SectionPayload
  ): Promise<void> {
    const dataSource = this.dataSource;
    const adjacentViewportWindows = Math.max(0, dataSource?.prefetchPolicy?.adjacentViewportWindows ?? 0);
    if (!dataSource || adjacentViewportWindows === 0) {
      return;
    }

    const requests = createAdjacentPrefetchRequests(currentSection, request, adjacentViewportWindows, {
      traceHalo: dataSource.halo?.traces ?? this.options.traceHalo,
      sampleHalo: dataSource.halo?.samples ?? this.options.sampleHalo,
      chooseLod: dataSource.chooseLod
    });
    for (const prefetchRequest of requests) {
      const prefetchCacheKey = dataSource.getRequestKey?.(prefetchRequest) ?? JSON.stringify(prefetchRequest);
      this.metrics.prefetchRequests += 1;
      if (this.cache.has(prefetchCacheKey)) {
        this.metrics.prefetchCacheHits += 1;
        this.touchCacheEntry(prefetchCacheKey);
        this.emitCurrentState();
        continue;
      }

      const abortController = new AbortController();
      this.prefetchAbortControllers.add(abortController);
      try {
        const section = await dataSource.loadWindow(prefetchRequest, {
          chartId: this.context.chartId,
          viewId: this.context.viewId,
          currentSection,
          signal: abortController.signal
        });
        if (abortController.signal.aborted) {
          continue;
        }
        this.storeCacheEntry(prefetchCacheKey, section);
        this.emitCurrentState();
      } catch (error) {
        if (abortController.signal.aborted) {
          continue;
        }
        this.metrics.prefetchErrors += 1;
        this.emitCurrentState();
      } finally {
        this.prefetchAbortControllers.delete(abortController);
      }
    }
  }

  private emitCurrentState(): void {
    this.emitState(this.currentState);
  }

  private cancelPending(): void {
    if (this.timer !== null) {
      clearTimeout(this.timer);
      this.timer = null;
    }
    if (this.abortController) {
      this.abortController.abort();
      this.abortController = null;
    }
    for (const abortController of this.prefetchAbortControllers) {
      abortController.abort();
    }
    this.prefetchAbortControllers.clear();
  }
}

function cloneMetrics(metrics: SeismicSectionDataSourceMetrics): SeismicSectionDataSourceMetrics {
  return { ...metrics };
}

function defaultEstimateSectionBytes(section: SectionPayload): number {
  const horizontalBytes = section.horizontalAxis.byteLength;
  const inlineBytes = section.inlineAxis?.byteLength ?? 0;
  const xlineBytes = section.xlineAxis?.byteLength ?? 0;
  const sampleBytes = section.sampleAxis.byteLength;
  const amplitudeBytes = section.amplitudes.byteLength;
  const overlayBytes = section.overlay?.values.byteLength ?? 0;
  return horizontalBytes + inlineBytes + xlineBytes + sampleBytes + amplitudeBytes + overlayBytes;
}

function nowMs(): number {
  return typeof performance !== "undefined" ? performance.now() : Date.now();
}

function createAdjacentPrefetchRequests(
  section: SectionPayload,
  request: SeismicSectionWindowRequest,
  adjacentViewportWindows: number,
  options: {
    traceHalo: number;
    sampleHalo: number;
    chooseLod?: (viewport: SectionViewport) => number;
  }
): SeismicSectionWindowRequest[] {
  const logical = resolveLogicalSectionDimensions(section);
  const viewportTraceSpan = Math.max(1, request.viewport.traceEnd - request.viewport.traceStart);
  const requests: SeismicSectionWindowRequest[] = [];

  for (let offset = 1; offset <= adjacentViewportWindows; offset += 1) {
    const forwardViewport = shiftViewportTraceWindow(request.viewport, logical.traces, viewportTraceSpan * offset);
    if (forwardViewport) {
      const prefetchRequest = createSeismicSectionWindowRequest(section, forwardViewport, options);
      requests.push({
        ...prefetchRequest,
        reason: "prefetch",
        comparisonMode: request.comparisonMode
      });
    }

    const backwardViewport = shiftViewportTraceWindow(request.viewport, logical.traces, -viewportTraceSpan * offset);
    if (backwardViewport) {
      const prefetchRequest = createSeismicSectionWindowRequest(section, backwardViewport, options);
      requests.push({
        ...prefetchRequest,
        reason: "prefetch",
        comparisonMode: request.comparisonMode
      });
    }
  }

  return requests;
}

function shiftViewportTraceWindow(
  viewport: SectionViewport,
  totalTraces: number,
  deltaTraces: number
): SectionViewport | null {
  const width = Math.max(1, viewport.traceEnd - viewport.traceStart);
  const nextStart = viewport.traceStart + deltaTraces;
  const nextEnd = nextStart + width;
  if (nextEnd <= 0 || nextStart >= totalTraces) {
    return null;
  }
  const clampedStart = clampViewportEdge(nextStart, 0, Math.max(0, totalTraces - width));
  const clampedEnd = clampViewportEdge(clampedStart + width, 1, totalTraces);
  if (clampedStart === viewport.traceStart && clampedEnd === viewport.traceEnd) {
    return null;
  }
  return {
    traceStart: clampedStart,
    traceEnd: clampedEnd,
    sampleStart: viewport.sampleStart,
    sampleEnd: viewport.sampleEnd
  };
}

function clampViewportEdge(value: number, min: number, max: number): number {
  return Math.min(Math.max(value, min), max);
}
