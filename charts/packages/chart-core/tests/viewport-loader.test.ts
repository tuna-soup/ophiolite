import assert from "node:assert/strict";
import test from "node:test";
import type {
  SectionPayload,
  SeismicSectionDataSource,
  SeismicSectionDataSourceState,
  SeismicSectionWindowRequest
} from "@ophiolite/charts-data-models";
import { createSeismicSectionDataSourceMetrics } from "@ophiolite/charts-data-models";
import { SectionViewportLoader } from "../../domain-geoscience/src/seismic/viewport-loader";
import { createSectionPayload, createSectionViewport } from "../../../tests/fixtures/seismic";

test("viewport loader stays ready when the loaded section already covers the viewport", () => {
  const section = createSectionPayload({
    traces: 64,
    samples: 128,
    window: {
      traceStart: 0,
      traceEnd: 64,
      sampleStart: 0,
      sampleEnd: 128,
      lod: 0
    }
  });
  const states: SeismicSectionDataSourceState[] = [];
  const loader = new SectionViewportLoader(
    {
      debounceMs: 0,
      async loadWindow() {
        throw new Error("should not load");
      }
    },
    { chartId: "chart-a", viewId: "view-a" },
    {
      onStateChange: (state) => states.push(state)
    }
  );

  loader.sync(section, createSectionViewport());
  loader.dispose();

  assert.deepEqual(states, [
    {
      status: "ready",
      request: null,
      source: "section",
      metrics: createSeismicSectionDataSourceMetrics(),
      cacheKey: null,
      errorMessage: null
    }
  ]);
});

test("viewport loader can force a window request even when the loaded section covers the viewport", async () => {
  const section = createSectionPayload({
    traces: 64,
    samples: 128,
    window: {
      traceStart: 0,
      traceEnd: 64,
      sampleStart: 0,
      sampleEnd: 128,
      lod: 0
    }
  });
  const states: SeismicSectionDataSourceState[] = [];
  const requests: SeismicSectionWindowRequest[] = [];
  const nextSection = createSectionPayload({
    traces: 64,
    samples: 128,
    window: {
      traceStart: 0,
      traceEnd: 64,
      sampleStart: 0,
      sampleEnd: 128,
      lod: 0
    }
  });
  const loader = new SectionViewportLoader(
    {
      debounceMs: 0,
      forceLoad: true,
      async loadWindow(request) {
        requests.push(request);
        return nextSection;
      }
    },
    { chartId: "chart-a", viewId: "view-a" },
    {
      onStateChange: (state) => states.push(state)
    }
  );

  loader.sync(section, createSectionViewport());
  await waitForSettled();
  loader.dispose();

  assert.equal(requests.length, 1);
  assert.equal(states[0]?.status, "scheduled");
  assert.equal(states[1]?.status, "loading");
  assert.equal(states[2]?.status, "ready");
  assert.equal(states[2]?.source, "network");
});

test("viewport loader debounces requests, forwards context, and reuses cache hits", async () => {
  const viewport = createSectionViewport();
  const section = createSectionPayload({
    traces: 64,
    samples: 128,
    logicalDimensions: {
      traces: 256,
      samples: 512
    },
    window: {
      traceStart: 12,
      traceEnd: 22,
      sampleStart: 6,
      sampleEnd: 18,
      lod: 1
    }
  });
  const loadedSections: Array<{ section: SectionPayload; request: SeismicSectionWindowRequest; cacheKey: string | null }> = [];
  const states: SeismicSectionDataSourceState[] = [];
  const requests: SeismicSectionWindowRequest[] = [];
  const contexts: Array<{ chartId?: string; viewId?: string; currentSection: SectionPayload | null }> = [];
  const nextSection = createSectionPayload({
    traces: 64,
    samples: 128,
    logicalDimensions: {
      traces: 256,
      samples: 512
    },
    window: {
      traceStart: 0,
      traceEnd: 64,
      sampleStart: 0,
      sampleEnd: 96,
      lod: 0
    }
  });
  const dataSource: SeismicSectionDataSource = {
    id: "fixture-source",
    debounceMs: 0,
    halo: {
      traces: 4,
      samples: 8
    },
    getRequestKey: (request) =>
      `${request.traceRange[0]}:${request.traceRange[1]}:${request.sampleRange[0]}:${request.sampleRange[1]}:${request.lod}`,
    async loadWindow(request, context) {
      requests.push(request);
      contexts.push({
        chartId: context.chartId,
        viewId: context.viewId,
        currentSection: context.currentSection
      });
      return nextSection;
    }
  };
  const loader = new SectionViewportLoader(dataSource, { chartId: "chart-a", viewId: "view-a" }, {
    onStateChange: (state) => states.push(state),
    onSectionLoaded: (loaded, request, cacheKey) => loadedSections.push({ section: loaded, request, cacheKey })
  });

  loader.sync(section, viewport);
  await waitForSettled();

  assert.equal(requests.length, 1);
  assert.deepEqual(requests[0]?.traceRange, [6, 34]);
  assert.deepEqual(requests[0]?.sampleRange, [0, 36]);
  assert.equal(requests[0]?.lod, 0);
  assert.deepEqual(contexts, [
    {
      chartId: "chart-a",
      viewId: "view-a",
      currentSection: section
    }
  ]);
  assert.deepEqual(states[0], {
    status: "scheduled",
    request: requests[0] ?? null,
    source: "network",
    metrics: {
      ...createSeismicSectionDataSourceMetrics(),
      viewportRequests: 1
    },
    cacheKey: "6:34:0:36:0",
    errorMessage: null
  });
  assert.deepEqual(states[1], {
    status: "loading",
    request: requests[0] ?? null,
    source: "network",
    metrics: {
      ...createSeismicSectionDataSourceMetrics(),
      viewportRequests: 1,
      fetches: 1
    },
    cacheKey: "6:34:0:36:0",
    errorMessage: null
  });
  assert.equal(states[2]?.status, "ready");
  assert.equal(states[2]?.source, "network");
  assert.equal(states[2]?.cacheKey, "6:34:0:36:0");
  assert.equal(states[2]?.metrics.viewportRequests, 1);
  assert.equal(states[2]?.metrics.fetches, 1);
  assert.equal(states[2]?.metrics.cacheEntries, 1);
  assert.ok((states[2]?.metrics.cacheBytes ?? 0) > 0);
  assert.equal(loadedSections.length, 1);
  assert.equal(loadedSections[0]?.section, nextSection);

  loader.sync(section, viewport);
  await waitForSettled();

  assert.equal(requests.length, 1);
  assert.deepEqual(states[3], {
    status: "ready",
    request: requests[0] ?? null,
    source: "cache",
    metrics: {
      ...states[2]!.metrics,
      viewportRequests: 2,
      cacheHits: 1
    },
    cacheKey: "6:34:0:36:0",
    errorMessage: null
  });
  assert.equal(loadedSections.length, 2);
  assert.equal(loadedSections[1]?.section, nextSection);
  assert.equal(loadedSections[1]?.cacheKey, loadedSections[0]?.cacheKey);

  loader.dispose();
});

test("viewport loader reports data-source failures without throwing", async () => {
  const section = createSectionPayload({
    window: {
      traceStart: 14,
      traceEnd: 24,
      sampleStart: 10,
      sampleEnd: 18,
      lod: 1
    }
  });
  const states: SeismicSectionDataSourceState[] = [];
  const loader = new SectionViewportLoader(
    {
      debounceMs: 0,
      async loadWindow() {
        throw new Error("loader exploded");
      }
    },
    { chartId: "chart-a", viewId: "view-a" },
    {
      onStateChange: (state) => states.push(state)
    }
  );

  loader.sync(section, createSectionViewport());
  await waitForSettled();
  loader.dispose();

  assert.deepEqual(states[0], {
    status: "scheduled",
    request: states[0]?.request ?? null,
    source: "network",
    metrics: {
      ...createSeismicSectionDataSourceMetrics(),
      viewportRequests: 1
    },
    cacheKey: states[0]?.cacheKey ?? null,
    errorMessage: null
  });
  assert.deepEqual(states[1], {
    status: "loading",
    request: states[0]?.request ?? null,
    source: "network",
    metrics: {
      ...createSeismicSectionDataSourceMetrics(),
      viewportRequests: 1,
      fetches: 1
    },
    cacheKey: states[0]?.cacheKey ?? null,
    errorMessage: null
  });
  assert.deepEqual(states[2], {
    status: "error",
    request: states[0]?.request ?? null,
    source: "network",
    metrics: {
      ...createSeismicSectionDataSourceMetrics(),
      viewportRequests: 1,
      fetches: 1,
      fetchErrors: 1
    },
    cacheKey: states[0]?.cacheKey ?? null,
    errorMessage: "loader exploded"
  });
});

test("viewport loader evicts older cache entries when the data-source cache policy is exceeded", async () => {
  const viewportA = createSectionViewport();
  const viewportB = {
    ...createSectionViewport(),
    traceStart: 100,
    traceEnd: 120
  };
  const sectionA = createSectionPayload({
    traces: 64,
    samples: 128,
    logicalDimensions: {
      traces: 256,
      samples: 512
    },
    window: {
      traceStart: 12,
      traceEnd: 22,
      sampleStart: 6,
      sampleEnd: 18,
      lod: 1
    }
  });
  const sectionB = createSectionPayload({
    traces: 64,
    samples: 128,
    logicalDimensions: {
      traces: 256,
      samples: 512
    },
    window: {
      traceStart: 64,
      traceEnd: 74,
      sampleStart: 6,
      sampleEnd: 18,
      lod: 1
    }
  });
  const requests: SeismicSectionWindowRequest[] = [];
  const states: SeismicSectionDataSourceState[] = [];
  const loader = new SectionViewportLoader(
    {
      debounceMs: 0,
      cachePolicy: {
        maxEntries: 1
      },
      estimateBytes: () => 10,
      getRequestKey: (request) => `${request.traceRange[0]}:${request.traceRange[1]}:${request.lod}`,
      async loadWindow(request) {
        requests.push(request);
        return createSectionPayload({
          window: {
            traceStart: request.traceRange[0],
            traceEnd: request.traceRange[1],
            sampleStart: request.sampleRange[0],
            sampleEnd: request.sampleRange[1],
            lod: request.lod
          }
        });
      }
    },
    { chartId: "chart-a", viewId: "view-a" },
    {
      onStateChange: (state) => states.push(state)
    }
  );

  loader.sync(sectionA, viewportA);
  await waitForSettled();
  loader.sync(sectionB, viewportB);
  await waitForSettled();
  loader.sync(sectionA, viewportA);
  await waitForSettled();
  loader.dispose();

  assert.equal(requests.length, 3);
  assert.deepEqual(
    requests.map((request) => request.traceRange),
    [
      [0, 62],
      [68, 152],
      [0, 62]
    ]
  );
  assert.equal(states.filter((state) => state.source === "cache").length, 0);
  assert.equal(states[8]?.status, "ready");
  assert.equal(states[8]?.source, "network");
  assert.deepEqual(states[8]?.metrics, {
    ...createSeismicSectionDataSourceMetrics(),
    viewportRequests: 3,
    fetches: 3,
    evictions: 2,
    cacheEntries: 1,
    cacheBytes: 10
  });
});

async function waitForSettled(): Promise<void> {
  await new Promise((resolve) => setTimeout(resolve, 0));
  await new Promise((resolve) => setTimeout(resolve, 0));
}
