#!/usr/bin/env node

import { mkdirSync, writeFileSync } from "node:fs";
import os from "node:os";
import path from "node:path";
import { fileURLToPath } from "node:url";

import { chromium } from "@playwright/test";

const scriptDir = path.dirname(fileURLToPath(import.meta.url));
const chartsRoot = path.resolve(scriptDir, "..");
const repoRoot = path.resolve(chartsRoot, "..");
const defaultOutputPath = path.join(
  repoRoot,
  "articles/benchmarking/results/2026-04-30-charts-texture-upload-benchmark.json"
);

function parseOutputPath() {
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

const cases = [
  { name: "focus-256x256", traces: 256, samples: 256, iterations: 30, warmup: 4 },
  { name: "overview-957x500", traces: 957, samples: 500, iterations: 20, warmup: 3 },
  { name: "full-f3-small-3826x2000", traces: 3826, samples: 2000, iterations: 6, warmup: 2 }
];

const browser = await chromium.launch({ headless: true });
const page = await browser.newPage({
  viewport: { width: 1280, height: 720 },
  deviceScaleFactor: 1
});

await page.setContent(`
  <!doctype html>
  <html>
    <body style="margin:0">
      <canvas id="gl" width="16" height="16"></canvas>
    </body>
  </html>
`);

const browserReport = await page.evaluate(async (benchmarkCases) => {
  const canvas = document.querySelector("canvas");
  if (!canvas) {
    throw new Error("Benchmark canvas missing.");
  }
  const gl = canvas.getContext("webgl2");
  if (!gl) {
    throw new Error("WebGL2 is unavailable.");
  }

  gl.getExtension("EXT_color_buffer_float");

  function median(values) {
    const sorted = [...values].sort((left, right) => left - right);
    const middle = Math.floor(sorted.length / 2);
    if (sorted.length % 2 === 0) {
      return ((sorted[middle - 1] ?? 0) + (sorted[middle] ?? 0)) / 2;
    }
    return sorted[middle] ?? 0;
  }

  function mean(values) {
    return values.reduce((sum, value) => sum + value, 0) / Math.max(1, values.length);
  }

  function p95(values) {
    const sorted = [...values].sort((left, right) => left - right);
    return sorted[Math.min(sorted.length - 1, Math.floor(sorted.length * 0.95))] ?? 0;
  }

  function summarize(values) {
    return {
      medianMs: median(values),
      meanMs: mean(values),
      p95Ms: p95(values),
      minMs: Math.min(...values),
      maxMs: Math.max(...values)
    };
  }

  function makeAmplitudes(traces, samples) {
    const values = new Float32Array(traces * samples);
    for (let trace = 0; trace < traces; trace += 1) {
      for (let sample = 0; sample < samples; sample += 1) {
        const index = trace * samples + sample;
        values[index] =
          Math.sin(sample * 0.011) * Math.cos(trace * 0.017) +
          0.18 * Math.sin((sample + trace) * 0.003);
      }
    }
    return values;
  }

  function floatToHalf(value) {
    if (Number.isNaN(value)) {
      return 0x7e00;
    }
    if (value === Infinity) {
      return 0x7c00;
    }
    if (value === -Infinity) {
      return 0xfc00;
    }
    const sign = value < 0 || Object.is(value, -0) ? 0x8000 : 0;
    const absolute = Math.abs(value);
    if (absolute === 0) {
      return sign;
    }
    if (absolute >= 65504) {
      return sign | 0x7bff;
    }
    if (absolute < 0.00006103515625) {
      return sign | Math.round(absolute / 0.000000059604644775390625);
    }
    let exponent = Math.floor(Math.log2(absolute));
    let mantissa = absolute / 2 ** exponent - 1;
    let halfExponent = exponent + 15;
    let halfMantissa = Math.round(mantissa * 1024);
    if (halfMantissa === 1024) {
      halfMantissa = 0;
      halfExponent += 1;
    }
    if (halfExponent >= 31) {
      return sign | 0x7bff;
    }
    return sign | (halfExponent << 10) | (halfMantissa & 0x3ff);
  }

  function halfToFloat(bits) {
    const sign = bits & 0x8000 ? -1 : 1;
    const exponent = (bits >> 10) & 0x1f;
    const mantissa = bits & 0x3ff;
    if (exponent === 0) {
      return sign * 2 ** -14 * (mantissa / 1024);
    }
    if (exponent === 31) {
      return mantissa ? Number.NaN : sign * Infinity;
    }
    return sign * 2 ** (exponent - 15) * (1 + mantissa / 1024);
  }

  function packR16F(source) {
    const packed = new Uint16Array(source.length);
    for (let index = 0; index < source.length; index += 1) {
      packed[index] = floatToHalf(source[index] ?? 0);
    }
    return packed;
  }

  function packR8(source) {
    let min = Infinity;
    let max = -Infinity;
    for (let index = 0; index < source.length; index += 1) {
      const value = source[index] ?? 0;
      if (value < min) min = value;
      if (value > max) max = value;
    }
    const extent = Math.max(1e-12, max - min);
    const scale = 255 / extent;
    const packed = new Uint8Array(source.length);
    for (let index = 0; index < source.length; index += 1) {
      packed[index] = Math.max(0, Math.min(255, Math.round(((source[index] ?? 0) - min) * scale)));
    }
    return { packed, min, extent };
  }

  function errorSummary(source, candidate, decode) {
    let maxAbs = 0;
    let sumSquared = 0;
    const stride = Math.max(1, Math.floor(source.length / 500000));
    let count = 0;
    for (let index = 0; index < source.length; index += stride) {
      const error = Math.abs((source[index] ?? 0) - decode(candidate, index));
      maxAbs = Math.max(maxAbs, error);
      sumSquared += error * error;
      count += 1;
    }
    return {
      sampledCount: count,
      maxAbsError: maxAbs,
      rmse: Math.sqrt(sumSquared / Math.max(1, count))
    };
  }

  function createTexture() {
    const texture = gl.createTexture();
    if (!texture) {
      throw new Error("Unable to create WebGL texture.");
    }
    gl.bindTexture(gl.TEXTURE_2D, texture);
    gl.pixelStorei(gl.UNPACK_ALIGNMENT, 1);
    gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_MIN_FILTER, gl.NEAREST);
    gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_MAG_FILTER, gl.NEAREST);
    gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_WRAP_S, gl.CLAMP_TO_EDGE);
    gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_WRAP_T, gl.CLAMP_TO_EDGE);
    return texture;
  }

  function upload(width, height, mode, data) {
    if (mode === "r32f") {
      gl.texImage2D(gl.TEXTURE_2D, 0, gl.R32F, width, height, 0, gl.RED, gl.FLOAT, data);
    } else if (mode === "r16f") {
      gl.texImage2D(gl.TEXTURE_2D, 0, gl.R16F, width, height, 0, gl.RED, gl.HALF_FLOAT, data);
    } else {
      gl.texImage2D(gl.TEXTURE_2D, 0, gl.R8, width, height, 0, gl.RED, gl.UNSIGNED_BYTE, data);
    }
    const error = gl.getError();
    if (error !== gl.NO_ERROR) {
      throw new Error(`WebGL upload failed for ${mode}: ${error}`);
    }
    gl.finish();
  }

  function benchmarkMode(benchCase, amplitudes, mode) {
    const texture = createTexture();
    const packTimes = [];
    const uploadTimes = [];
    const totalTimes = [];
    let gpuBytes = amplitudes.byteLength;
    let error = { sampledCount: amplitudes.length, maxAbsError: 0, rmse: 0 };

    function pack() {
      if (mode === "r32f") {
        gpuBytes = amplitudes.byteLength;
        return amplitudes;
      }
      if (mode === "r16f") {
        const packed = packR16F(amplitudes);
        gpuBytes = packed.byteLength;
        return packed;
      }
      const packed = packR8(amplitudes);
      gpuBytes = packed.packed.byteLength;
      return packed;
    }

    const errorCandidate = pack();
    if (mode === "r16f") {
      error = errorSummary(amplitudes, errorCandidate, (candidate, index) => halfToFloat(candidate[index] ?? 0));
    } else if (mode === "r8") {
      error = errorSummary(
        amplitudes,
        errorCandidate,
        (candidate, index) => candidate.min + ((candidate.packed[index] ?? 0) / 255) * candidate.extent
      );
    }

    for (let iteration = 0; iteration < benchCase.warmup + benchCase.iterations; iteration += 1) {
      const totalStart = performance.now();
      const packStart = performance.now();
      const packed = pack();
      const packEnd = performance.now();
      const uploadStart = performance.now();
      upload(
        benchCase.samples,
        benchCase.traces,
        mode,
        mode === "r8" ? packed.packed : packed
      );
      const uploadEnd = performance.now();
      if (iteration >= benchCase.warmup) {
        packTimes.push(packEnd - packStart);
        uploadTimes.push(uploadEnd - uploadStart);
        totalTimes.push(uploadEnd - totalStart);
      }
    }

    gl.deleteTexture(texture);

    return {
      mode,
      iterations: benchCase.iterations,
      gpuBytes,
      sourceBytes: amplitudes.byteLength,
      byteFractionOfR32F: gpuBytes / amplitudes.byteLength,
      pack: summarize(packTimes),
      upload: summarize(uploadTimes),
      total: summarize(totalTimes),
      error
    };
  }

  const results = [];
  for (const benchCase of benchmarkCases) {
    const amplitudes = makeAmplitudes(benchCase.traces, benchCase.samples);
    results.push({
      name: benchCase.name,
      traces: benchCase.traces,
      samples: benchCase.samples,
      sourceBytes: amplitudes.byteLength,
      results: [
        benchmarkMode(benchCase, amplitudes, "r32f"),
        benchmarkMode(benchCase, amplitudes, "r16f"),
        benchmarkMode(benchCase, amplitudes, "r8")
      ]
    });
  }

  return {
    browser: {
      userAgent: navigator.userAgent,
      platform: navigator.platform,
      hardwareConcurrency: navigator.hardwareConcurrency,
      devicePixelRatio: window.devicePixelRatio,
      webglRenderer: gl.getParameter(gl.RENDERER),
      webglVendor: gl.getParameter(gl.VENDOR),
      webglVersion: gl.getParameter(gl.VERSION)
    },
    cases: results
  };
}, cases);

await browser.close();

const report = {
  status: "local exploratory baseline",
  generatedAt: new Date().toISOString(),
  benchmark: "charts-texture-upload",
  host: {
    platform: os.platform(),
    arch: os.arch(),
    release: os.release(),
    cpus: os.cpus().length
  },
  ...browserReport
};

const outputPath = parseOutputPath();
mkdirSync(path.dirname(outputPath), { recursive: true });
writeFileSync(outputPath, `${JSON.stringify(report, null, 2)}\n`);
console.log(`Wrote ${path.relative(repoRoot, outputPath)}`);
