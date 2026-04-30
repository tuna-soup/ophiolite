#!/usr/bin/env bun

import { mkdirSync, writeFileSync } from "node:fs";
import path from "node:path";
import { performance } from "node:perf_hooks";
import { fileURLToPath } from "node:url";

import type {
  DisplayTransform,
  SectionPayload,
  SectionViewport
} from "../packages/data-models/src";
import {
  prepareWiggleData,
  prepareWiggleInstances,
  visibleAmplitudeMaxAbs
} from "../packages/renderer/src/seismic/mock/renderModel";
import type { PlotRect } from "../packages/renderer/src/seismic/mock/wiggleGeometry";

interface BenchCase {
  name: string;
  traces: number;
  samples: number;
  viewport: SectionViewport;
  iterations: number;
  warmup: number;
}

interface IterationResult {
  prepareMs: number;
  uploadBytes: number;
  drawnTraces: number;
  lineVertexCount?: number;
  fillVertexCount?: number;
  sampleCount?: number;
}

type WiggleGeometryMode = "expanded" | "instanced" | "instanced-cached";

const scriptDir = path.dirname(fileURLToPath(import.meta.url));
const chartsRoot = path.resolve(scriptDir, "..");
const repoRoot = path.resolve(chartsRoot, "..");
const defaultOutputPath = path.join(
  repoRoot,
  "articles/benchmarking/results/2026-04-30-charts-wiggle-geometry-benchmark.json"
);

const plotRect: PlotRect = {
  x: 68,
  y: 72,
  width: 1400,
  height: 840
};

const canvasWidth = 1600;
const canvasHeight = 1000;

const displayTransform: DisplayTransform = {
  renderMode: "wiggle",
  gain: 1.15,
  colormap: "grayscale",
  polarity: "normal"
};

const cases: BenchCase[] = [
  {
    name: "focus-256x256",
    traces: 256,
    samples: 256,
    viewport: { traceStart: 0, traceEnd: 256, sampleStart: 0, sampleEnd: 256 },
    iterations: 60,
    warmup: 5
  },
  {
    name: "overview-957x500",
    traces: 957,
    samples: 500,
    viewport: { traceStart: 0, traceEnd: 957, sampleStart: 0, sampleEnd: 500 },
    iterations: 30,
    warmup: 4
  },
  {
    name: "full-f3-small-3826x2000",
    traces: 3826,
    samples: 2000,
    viewport: { traceStart: 0, traceEnd: 3826, sampleStart: 0, sampleEnd: 2000 },
    iterations: 8,
    warmup: 2
  },
  {
    name: "zoomed-f3-512x512",
    traces: 3826,
    samples: 2000,
    viewport: { traceStart: 1657, traceEnd: 2169, sampleStart: 744, sampleEnd: 1256 },
    iterations: 40,
    warmup: 4
  }
];

function parseOutputPath(): string {
  const outputIndex = process.argv.indexOf("--output");
  if (outputIndex >= 0) {
    const value = process.argv[outputIndex + 1];
    if (!value) {
      throw new Error("Missing value after --output.");
    }
    return path.resolve(process.cwd(), value);
  }
  return defaultOutputPath;
}

function makeSection(traces: number, samples: number): SectionPayload {
  const horizontalAxis = new Float64Array(traces);
  const inlineAxis = new Float64Array(traces);
  const sampleAxis = new Float32Array(samples);
  const amplitudes = new Float32Array(traces * samples);

  for (let trace = 0; trace < traces; trace += 1) {
    horizontalAxis[trace] = trace + 1;
    inlineAxis[trace] = 10_000 + trace;
    for (let sample = 0; sample < samples; sample += 1) {
      if (trace === 0) {
        sampleAxis[sample] = sample;
      }
      amplitudes[trace * samples + sample] =
        Math.sin(sample * 0.026) * Math.cos(trace * 0.021) +
        0.35 * Math.sin((sample + trace) * 0.006) +
        0.08 * Math.cos(sample * 0.17);
    }
  }

  return {
    axis: "inline",
    coordinate: {
      index: 0,
      value: 10_000
    },
    horizontalAxis,
    inlineAxis,
    sampleAxis,
    amplitudes,
    dimensions: {
      traces,
      samples
    },
    units: {
      horizontal: "xline",
      sample: "ms",
      amplitude: "arb"
    }
  };
}

function median(values: number[]): number {
  const sorted = [...values].sort((left, right) => left - right);
  const middle = Math.floor(sorted.length / 2);
  if (sorted.length % 2 === 0) {
    return ((sorted[middle - 1] ?? 0) + (sorted[middle] ?? 0)) / 2;
  }
  return sorted[middle] ?? 0;
}

function mean(values: number[]): number {
  return values.reduce((sum, value) => sum + value, 0) / Math.max(1, values.length);
}

function p95(values: number[]): number {
  const sorted = [...values].sort((left, right) => left - right);
  return sorted[Math.min(sorted.length - 1, Math.floor(sorted.length * 0.95))] ?? 0;
}

function summarize(values: number[]) {
  return {
    medianMs: median(values),
    meanMs: mean(values),
    p95Ms: p95(values),
    minMs: Math.min(...values),
    maxMs: Math.max(...values)
  };
}

function runExpanded(section: SectionPayload, benchCase: BenchCase): IterationResult {
  const start = performance.now();
  const prepared = prepareWiggleData(
    section,
    benchCase.viewport,
    displayTransform,
    plotRect,
    canvasWidth,
    canvasHeight
  );
  const end = performance.now();
  return {
    prepareMs: end - start,
    uploadBytes: prepared.lineVertices.byteLength + prepared.fillVertices.byteLength,
    drawnTraces: Math.ceil((benchCase.viewport.traceEnd - benchCase.viewport.traceStart) / prepared.traceStride),
    lineVertexCount: prepared.lineVertices.length / 2,
    fillVertexCount: prepared.fillVertices.length / 2
  };
}

function runInstanced(section: SectionPayload, benchCase: BenchCase, cachedVisibleAmplitudeMaxAbs?: number): IterationResult {
  const start = performance.now();
  const prepared = prepareWiggleInstances(
    section,
    benchCase.viewport,
    displayTransform,
    plotRect,
    canvasWidth,
    cachedVisibleAmplitudeMaxAbs === undefined
      ? undefined
      : {
          visibleAmplitudeMaxAbs: cachedVisibleAmplitudeMaxAbs
        }
  );
  const interleaved = new Float32Array(prepared.traceIndices.length * 3);
  for (let index = 0; index < prepared.traceIndices.length; index += 1) {
    const offset = index * 3;
    interleaved[offset] = prepared.traceIndices[index]!;
    interleaved[offset + 1] = prepared.baselineClipX[index]!;
    interleaved[offset + 2] = prepared.amplitudeScaleClip[index]!;
  }
  const end = performance.now();
  return {
    prepareMs: end - start,
    uploadBytes: interleaved.byteLength,
    drawnTraces: prepared.traceIndices.length,
    sampleCount: prepared.sampleCount
  };
}

function runMode(
  mode: WiggleGeometryMode,
  section: SectionPayload,
  benchCase: BenchCase
) {
  const iterations: IterationResult[] = [];
  let cachePrimeMs: number | null = null;
  let cachedVisibleAmplitudeMaxAbs: number | undefined;
  if (mode === "instanced-cached") {
    const cachePrimeStart = performance.now();
    cachedVisibleAmplitudeMaxAbs = visibleAmplitudeMaxAbs(
      section,
      benchCase.viewport.traceStart,
      benchCase.viewport.traceEnd,
      benchCase.viewport.sampleStart,
      benchCase.viewport.sampleEnd
    );
    cachePrimeMs = performance.now() - cachePrimeStart;
  }
  const runner = (mode: WiggleGeometryMode): IterationResult => {
    if (mode === "expanded") {
      return runExpanded(section, benchCase);
    }
    return runInstanced(section, benchCase, cachedVisibleAmplitudeMaxAbs);
  };

  for (let warmup = 0; warmup < benchCase.warmup; warmup += 1) {
    runner(mode);
  }
  for (let iteration = 0; iteration < benchCase.iterations; iteration += 1) {
    iterations.push(runner(mode));
  }

  return {
    mode,
    iterations: benchCase.iterations,
    cachePrimeMs,
    prepare: summarize(iterations.map((entry) => entry.prepareMs)),
    uploadBytesMedian: median(iterations.map((entry) => entry.uploadBytes)),
    drawnTracesMedian: median(iterations.map((entry) => entry.drawnTraces)),
    lineVertexCountMedian: median(iterations.map((entry) => entry.lineVertexCount ?? 0)),
    fillVertexCountMedian: median(iterations.map((entry) => entry.fillVertexCount ?? 0)),
    sampleCountMedian: median(iterations.map((entry) => entry.sampleCount ?? 0))
  };
}

function runCase(benchCase: BenchCase) {
  const section = makeSection(benchCase.traces, benchCase.samples);
  const viewportTraces = benchCase.viewport.traceEnd - benchCase.viewport.traceStart;
  const viewportSamples = benchCase.viewport.sampleEnd - benchCase.viewport.sampleStart;
  const expanded = runMode("expanded", section, benchCase);
  const instanced = runMode("instanced", section, benchCase);
  const instancedCached = runMode("instanced-cached", section, benchCase);

  return {
    name: benchCase.name,
    traces: benchCase.traces,
    samples: benchCase.samples,
    viewport: benchCase.viewport,
    viewportTraces,
    viewportSamples,
    sourceAmplitudeBytes: section.amplitudes.byteLength,
    results: [expanded, instanced, instancedCached],
    ratios: {
      prepareMedianInstancedOverExpanded:
        instanced.prepare.medianMs / Math.max(expanded.prepare.medianMs, 1e-12),
      prepareMedianInstancedCachedOverExpanded:
        instancedCached.prepare.medianMs / Math.max(expanded.prepare.medianMs, 1e-12),
      prepareMedianInstancedCachedOverInstanced:
        instancedCached.prepare.medianMs / Math.max(instanced.prepare.medianMs, 1e-12),
      uploadBytesInstancedOverExpanded:
        instanced.uploadBytesMedian / Math.max(expanded.uploadBytesMedian, 1)
    }
  };
}

const outputPath = parseOutputPath();
const report = {
  status: "local exploratory baseline",
  generatedAt: new Date().toISOString(),
  benchmark: "charts-wiggle-geometry",
  plotRect,
  canvas: {
    width: canvasWidth,
    height: canvasHeight
  },
  cases: cases.map(runCase)
};

mkdirSync(path.dirname(outputPath), { recursive: true });
writeFileSync(outputPath, `${JSON.stringify(report, null, 2)}\n`);
console.log(`Wrote ${path.relative(repoRoot, outputPath)}`);
