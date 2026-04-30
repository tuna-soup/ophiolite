#!/usr/bin/env bun

import { mkdirSync, writeFileSync } from "node:fs";
import path from "node:path";
import { performance } from "node:perf_hooks";
import { fileURLToPath } from "node:url";

import { parsePackedSectionTileResponse } from "../src/lib/transport/packed-sections";
import {
  adaptTransportWindowedSectionToChartData,
  createDecodeStats,
  type DecodeCopyMode
} from "../src/lib/section-adapter";

interface BenchCase {
  name: string;
  traces: number;
  samples: number;
  iterations: number;
}

interface IterationResult {
  parseMs: number;
  adaptMs: number;
  totalMs: number;
  copiedBytes: number;
  viewedBytes: number;
  copiedBuffers: number;
  viewedBuffers: number;
}

const scriptDir = path.dirname(fileURLToPath(import.meta.url));
const appRoot = path.resolve(scriptDir, "..");
const repoRoot = path.resolve(appRoot, "../..");
const defaultOutputPath = path.join(
  repoRoot,
  "articles/benchmarking/results/2026-04-30-packed-section-adaptation-benchmark.json"
);

const encoder = new TextEncoder();

const cases: BenchCase[] = [
  { name: "focus-256x256", traces: 256, samples: 256, iterations: 80 },
  { name: "overview-957x500", traces: 957, samples: 500, iterations: 40 },
  { name: "full-f3-small-3826x2000", traces: 3826, samples: 2000, iterations: 8 }
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

function alignUp(value: number, alignment: number): number {
  return Math.ceil(value / alignment) * alignment;
}

function bytesOf(values: Float32Array | Float64Array): Uint8Array {
  return new Uint8Array(values.buffer, values.byteOffset, values.byteLength);
}

function sequenceF64(length: number, start = 0): Float64Array {
  const values = new Float64Array(length);
  for (let index = 0; index < values.length; index += 1) {
    values[index] = start + index;
  }
  return values;
}

function sequenceF32(length: number, start = 0): Float32Array {
  const values = new Float32Array(length);
  for (let index = 0; index < values.length; index += 1) {
    values[index] = start + index;
  }
  return values;
}

function amplitudes(traces: number, samples: number): Float32Array {
  const values = new Float32Array(traces * samples);
  for (let trace = 0; trace < traces; trace += 1) {
    for (let sample = 0; sample < samples; sample += 1) {
      values[trace * samples + sample] = Math.sin(sample * 0.01) * Math.cos(trace * 0.03);
    }
  }
  return values;
}

function buildPackedTilePayload(benchCase: BenchCase): Uint8Array {
  const horizontalAxis = bytesOf(sequenceF64(benchCase.traces, 1000));
  const inlineAxis = bytesOf(sequenceF64(benchCase.traces, 1000));
  const sampleAxis = bytesOf(sequenceF32(benchCase.samples, 0));
  const amplitudeBytes = bytesOf(amplitudes(benchCase.traces, benchCase.samples));

  const header = {
    section: {
      datasetId: "synthetic-packed-section-benchmark",
      axis: "inline",
      coordinate: {
        index: 128,
        value: 128
      },
      traces: benchCase.traces,
      samples: benchCase.samples,
      horizontalAxisBytes: horizontalAxis.byteLength,
      inlineAxisBytes: inlineAxis.byteLength,
      xlineAxisBytes: null,
      sampleAxisBytes: sampleAxis.byteLength,
      amplitudesBytes: amplitudeBytes.byteLength,
      units: null,
      metadata: null,
      displayDefaults: null
    },
    traceRange: [0, benchCase.traces],
    sampleRange: [0, benchCase.samples],
    lod: 0,
    traceStep: 1,
    sampleStep: 1
  };

  const headerBytes = encoder.encode(JSON.stringify(header));
  const dataOffset = alignUp(16 + headerBytes.byteLength, 8);
  const totalLength =
    dataOffset +
    horizontalAxis.byteLength +
    inlineAxis.byteLength +
    sampleAxis.byteLength +
    amplitudeBytes.byteLength;

  const payload = new Uint8Array(totalLength);
  payload.set(encoder.encode("TBTIL001"), 0);
  new DataView(payload.buffer).setUint32(8, headerBytes.byteLength, true);
  new DataView(payload.buffer).setUint32(12, dataOffset, true);
  payload.set(headerBytes, 16);

  let cursor = dataOffset;
  for (const part of [horizontalAxis, inlineAxis, sampleAxis, amplitudeBytes]) {
    payload.set(part, cursor);
    cursor += part.byteLength;
  }

  return payload;
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

function summarize(iterations: IterationResult[]) {
  return {
    parseMedianMs: median(iterations.map((entry) => entry.parseMs)),
    parseMeanMs: mean(iterations.map((entry) => entry.parseMs)),
    adaptMedianMs: median(iterations.map((entry) => entry.adaptMs)),
    adaptMeanMs: mean(iterations.map((entry) => entry.adaptMs)),
    totalMedianMs: median(iterations.map((entry) => entry.totalMs)),
    totalMeanMs: mean(iterations.map((entry) => entry.totalMs)),
    copiedBytesMedian: median(iterations.map((entry) => entry.copiedBytes)),
    viewedBytesMedian: median(iterations.map((entry) => entry.viewedBytes)),
    copiedBuffersMedian: median(iterations.map((entry) => entry.copiedBuffers)),
    viewedBuffersMedian: median(iterations.map((entry) => entry.viewedBuffers))
  };
}

function runOne(payload: Uint8Array, benchCase: BenchCase, copyMode: DecodeCopyMode): IterationResult {
  const parseStart = performance.now();
  const tile = parsePackedSectionTileResponse(payload);
  const parseEnd = performance.now();
  const stats = createDecodeStats();
  const windowed = {
    ...tile.section,
    logical_dimensions: {
      traces: benchCase.traces,
      samples: benchCase.samples
    },
    window: {
      trace_start: tile.trace_range[0],
      trace_end: tile.trace_range[1],
      sample_start: tile.sample_range[0],
      sample_end: tile.sample_range[1],
      lod: tile.lod
    }
  };
  const chartSection = adaptTransportWindowedSectionToChartData(windowed, { copyMode, stats });
  const adaptEnd = performance.now();

  if (chartSection.amplitudes.length !== benchCase.traces * benchCase.samples) {
    throw new Error(`Unexpected amplitude length for ${benchCase.name}.`);
  }

  return {
    parseMs: parseEnd - parseStart,
    adaptMs: adaptEnd - parseEnd,
    totalMs: adaptEnd - parseStart,
    copiedBytes: stats.copiedBytes,
    viewedBytes: stats.viewedBytes,
    copiedBuffers: stats.copiedBuffers,
    viewedBuffers: stats.viewedBuffers
  };
}

function runCase(benchCase: BenchCase) {
  const payload = buildPackedTilePayload(benchCase);
  const modes: DecodeCopyMode[] = ["copy", "view"];
  const results = [];

  for (const mode of modes) {
    for (let warmup = 0; warmup < 5; warmup += 1) {
      runOne(payload, benchCase, mode);
    }

    const iterations: IterationResult[] = [];
    for (let iteration = 0; iteration < benchCase.iterations; iteration += 1) {
      iterations.push(runOne(payload, benchCase, mode));
    }

    results.push({
      mode,
      iterations: benchCase.iterations,
      ...summarize(iterations)
    });
  }

  return {
    name: benchCase.name,
    traces: benchCase.traces,
    samples: benchCase.samples,
    payloadBytes: payload.byteLength,
    results
  };
}

const outputPath = parseOutputPath();
const report = {
  status: "local exploratory baseline",
  generatedAt: new Date().toISOString(),
  benchmark: "packed-section-adaptation",
  cases: cases.map(runCase)
};

mkdirSync(path.dirname(outputPath), { recursive: true });
writeFileSync(outputPath, `${JSON.stringify(report, null, 2)}\n`);
console.log(`Wrote ${path.relative(repoRoot, outputPath)}`);
