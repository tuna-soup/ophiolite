import type {
  WellTieChartModel,
  WellTieMetric,
  WellTieSectionPanel,
  WellTieTrack,
  WellTieWavelet
} from "./well-tie";

interface MockWellTieOptions {
  id?: string;
  name?: string;
  wellName?: string;
  timeStartMs?: number;
  timeEndMs?: number;
}

const DEFAULT_TIME_START_MS = 900;
const DEFAULT_TIME_END_MS = 2250;
const SAMPLE_STEP_MS = 4;

export function createMockWellTieChartModel(options: MockWellTieOptions = {}): WellTieChartModel {
  const timeStartMs = options.timeStartMs ?? DEFAULT_TIME_START_MS;
  const timeEndMs = options.timeEndMs ?? DEFAULT_TIME_END_MS;
  const sampleCount = Math.max(48, Math.round((timeEndMs - timeStartMs) / SAMPLE_STEP_MS) + 1);
  const timesMs = Float32Array.from(
    { length: sampleCount },
    (_, index) => timeStartMs + index * SAMPLE_STEP_MS
  );

  const acousticImpedance = new Float32Array(sampleCount);
  const reflectivity = new Float32Array(sampleCount);
  for (let index = 0; index < sampleCount; index += 1) {
    const timeMs = timesMs[index]!;
    const eventA = gaussian(timeMs, 1185, 34, 0.9);
    const eventB = gaussian(timeMs, 1440, 58, -0.7);
    const eventC = gaussian(timeMs, 1765, 45, 1.1);
    const eventD = gaussian(timeMs, 2010, 36, -0.55);

    acousticImpedance[index] =
      8200 +
      2100 * eventA +
      1450 * eventB +
      2600 * eventC +
      1100 * eventD +
      Math.sin(timeMs / 92) * 210 +
      Math.cos(timeMs / 41) * 120;
  }

  for (let index = 1; index < sampleCount; index += 1) {
    const left = acousticImpedance[index - 1]!;
    const right = acousticImpedance[index]!;
    reflectivity[index] = (right - left) / Math.max(1, right + left);
  }

  const wavelet = createWavelet();
  const synthetic = convolve(reflectivity, wavelet.amplitudes);
  const bestSeismic = createMatchedTrace(synthetic, 3, 0.92, 0.035);
  const wellSeismic = createMatchedTrace(synthetic, -2, 0.84, 0.06);
  const section = createSectionPanel(timesMs, synthetic);
  const metrics = createMetrics();

  const tracks: WellTieTrack[] = [
    {
      kind: "curve",
      id: "acoustic-impedance",
      label: "AI",
      unit: "m/s.g/cc",
      color: "#30455c",
      fillColor: "rgba(123, 172, 210, 0.16)",
      timesMs,
      values: acousticImpedance,
      valueRange: {
        min: 5200,
        max: 12600
      }
    },
    {
      kind: "wiggle",
      id: "best-seismic",
      label: "Best Seis",
      timesMs,
      amplitudes: bestSeismic,
      lineColor: "#213140",
      positiveFill: "rgba(221, 70, 61, 0.84)",
      negativeFill: "rgba(52, 89, 178, 0.82)",
      amplitudeScale: 1
    },
    {
      kind: "wiggle",
      id: "synthetic",
      label: "Syn",
      timesMs,
      amplitudes: synthetic,
      lineColor: "#213140",
      positiveFill: "rgba(221, 70, 61, 0.84)",
      negativeFill: "rgba(52, 89, 178, 0.82)",
      amplitudeScale: 1
    },
    {
      kind: "wiggle",
      id: "well-seismic",
      label: "Well Seis",
      timesMs,
      amplitudes: wellSeismic,
      lineColor: "#213140",
      positiveFill: "rgba(221, 70, 61, 0.84)",
      negativeFill: "rgba(52, 89, 178, 0.82)",
      amplitudeScale: 1
    }
  ];

  const wellName = options.wellName?.trim() || "Selected Well";
  return {
    id: options.id ?? "mock-well-tie",
    name: options.name ?? `${wellName} Well Tie`,
    timeRangeMs: {
      start: timeStartMs,
      end: timeEndMs,
      unit: "ms"
    },
    depthRangeM: {
      start: 1720,
      end: 2860
    },
    tracks,
    metrics,
    markers: [
      { id: "m1", label: "Top Sand", timeMs: 1188, color: "#d45d2c" },
      { id: "m2", label: "Base Sand", timeMs: 1768, color: "#108d76" }
    ],
    section,
    wavelet,
    notes: [
      "Preview payload for the phase-1 single-well post-stack tie workbench.",
      "The runtime tie engine can replace this model without changing the chart contract."
    ]
  };
}

function createWavelet(): WellTieWavelet {
  const sampleCount = 49;
  const timesMs = Float32Array.from({ length: sampleCount }, (_, index) => -96 + index * 4);
  const amplitudes = new Float32Array(sampleCount);

  for (let index = 0; index < sampleCount; index += 1) {
    const timeSeconds = (timesMs[index] ?? 0) / 1000;
    amplitudes[index] = ricker(timeSeconds, 28);
  }

  return {
    id: "extracted-wavelet",
    label: "Extracted Wavelet",
    timesMs,
    amplitudes,
    amplitudeRange: {
      min: -1,
      max: 1
    },
    state: "extracted",
    detail: "Least-squares estimate from the matched seismic trace"
  };
}

function createSectionPanel(timesMs: Float32Array, seedTrace: Float32Array): WellTieSectionPanel {
  const traceOffsetsM = Float32Array.from(
    { length: 17 },
    (_, index) => -200 + (400 / 16) * index
  );
  const sampleCount = timesMs.length;
  const traceCount = traceOffsetsM.length;
  const amplitudes = new Float32Array(sampleCount * traceCount);

  for (let traceIndex = 0; traceIndex < traceCount; traceIndex += 1) {
    const lateralBias = (traceIndex - 8) * 0.45;
    const shiftSamples = Math.round((traceIndex - 8) * 0.65);
    const energyScale = 0.92 - Math.abs(traceIndex - 8) * 0.028;

    for (let sampleIndex = 0; sampleIndex < sampleCount; sampleIndex += 1) {
      const sourceIndex = clampIndex(sampleIndex + shiftSamples, sampleCount);
      const noise =
        Math.sin(sampleIndex / 13 + traceIndex * 0.7) * 0.07 +
        Math.cos(sampleIndex / 7.5 + traceIndex * 0.35) * 0.045;
      amplitudes[traceIndex * sampleCount + sampleIndex] =
        seedTrace[sourceIndex]! * energyScale +
        lateralBias * gaussian(timesMs[sampleIndex]!, 1765, 120, 0.13) +
        noise;
    }
  }

  return {
    id: "local-seismic-section",
    label: "Local Seismic Window",
    timesMs,
    traceOffsetsM,
    amplitudes,
    traceCount,
    sampleCount,
    wellTraceIndex: 8,
    matchTraceIndex: 9,
    matchOffsetM: 25,
    wellLabel: "Well",
    matchLabel: "Best Match"
  };
}

function createMetrics(): WellTieMetric[] {
  return [
    { id: "corr", label: "Corr", value: "0.83", emphasis: "good" },
    { id: "shift", label: "Bulk Shift", value: "-12 ms", emphasis: "warn" },
    { id: "stretch", label: "Stretch", value: "1.04x", emphasis: "neutral" },
    { id: "search", label: "Best Match", value: "+25 m", emphasis: "neutral" },
    { id: "wavelet", label: "Wavelet", value: "Extracted", emphasis: "good" }
  ];
}

function createMatchedTrace(
  seed: Float32Array,
  bulkShiftSamples: number,
  amplitudeScale: number,
  noiseScale: number
): Float32Array {
  const output = new Float32Array(seed.length);
  for (let index = 0; index < seed.length; index += 1) {
    const shiftedIndex = clampIndex(index + bulkShiftSamples, seed.length);
    output[index] =
      seed[shiftedIndex]! * amplitudeScale +
      Math.sin(index / 8.5) * noiseScale +
      Math.cos(index / 19) * (noiseScale * 0.6);
  }
  return output;
}

function convolve(signal: Float32Array, kernel: Float32Array): Float32Array {
  const output = new Float32Array(signal.length);
  const halfKernel = Math.floor(kernel.length / 2);

  for (let sampleIndex = 0; sampleIndex < signal.length; sampleIndex += 1) {
    let sum = 0;
    for (let kernelIndex = 0; kernelIndex < kernel.length; kernelIndex += 1) {
      const signalIndex = sampleIndex + kernelIndex - halfKernel;
      if (signalIndex < 0 || signalIndex >= signal.length) {
        continue;
      }
      sum += signal[signalIndex]! * kernel[kernelIndex]!;
    }
    output[sampleIndex] = sum;
  }

  normalizeInPlace(output);
  return output;
}

function normalizeInPlace(values: Float32Array): void {
  let maxAmplitude = 0;
  for (const value of values) {
    maxAmplitude = Math.max(maxAmplitude, Math.abs(value));
  }

  if (maxAmplitude <= 0) {
    return;
  }

  for (let index = 0; index < values.length; index += 1) {
    values[index] = values[index]! / maxAmplitude;
  }
}

function ricker(timeSeconds: number, frequencyHz: number): number {
  const omega = Math.PI * frequencyHz * timeSeconds;
  const omegaSquared = omega * omega;
  return (1 - 2 * omegaSquared) * Math.exp(-omegaSquared);
}

function gaussian(value: number, center: number, width: number, amplitude: number): number {
  const normalized = (value - center) / Math.max(width, 1);
  return amplitude * Math.exp(-(normalized * normalized));
}

function clampIndex(index: number, length: number): number {
  return Math.max(0, Math.min(length - 1, index));
}
