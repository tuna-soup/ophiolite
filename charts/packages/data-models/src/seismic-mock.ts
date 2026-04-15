import type {
  SectionHorizonOverlay,
  SectionPayload,
  SectionScalarOverlay,
  SectionScalarOverlayColorMap,
  SectionWellOverlay
} from "./seismic";

export type MockSectionKind = "inline" | "arbitrary";
export type MockSectionDomain = "time" | "depth";

export const MOCK_SECTION_VELOCITY_MODEL_LABEL = "Synthetic Spatial Interval Velocity Field";

interface MockSectionModel {
  traces: number;
  samples: number;
  horizontalAxis: Float64Array;
  inlineAxis?: Float64Array;
  xlineAxis?: Float64Array;
  timeAxis: Float32Array;
  depthAxis: Float32Array;
  amplitudesTime: Float32Array;
  amplitudesDepth: Float32Array;
  velocityTime: Float32Array;
  velocityDepth: Float32Array;
  depthByTrace: Float32Array;
  qcMask: Uint8Array;
}

interface MockSectionScalarOverlayOptions {
  opacity?: number;
  colorMap?: SectionScalarOverlayColorMap;
}

export function createMockSection(
  kind: MockSectionKind = "inline",
  domain: MockSectionDomain = "time"
): SectionPayload {
  const model = buildMockSectionModel(kind);
  const sampleAxis = domain === "depth" ? model.depthAxis : model.timeAxis;
  const amplitudes = domain === "depth" ? model.amplitudesDepth : model.amplitudesTime;
  const sampleUnit = domain === "depth" ? "m" : "ms";
  const sampleAxisLabel = domain === "depth" ? "Depth" : "Time";
  const titleSuffix = domain === "depth" ? "Depth" : "TWT";

  return {
    axis: "inline",
    coordinate: kind === "arbitrary" ? { index: 42, value: 1042 } : { index: 0, value: 111 },
    horizontalAxis: model.horizontalAxis,
    inlineAxis: model.inlineAxis,
    xlineAxis: model.xlineAxis,
    sampleAxis,
    amplitudes,
    dimensions: { traces: model.traces, samples: model.samples },
    units: {
      horizontal: kind === "arbitrary" ? "trace" : "xline",
      sample: sampleUnit,
      amplitude: "arb"
    },
    metadata: {
      storeId: "mock-zarr-store",
      notes: [
        `sample_domain:${domain}`,
        kind === "arbitrary"
          ? "Synthetic arbitrary section annotation demo."
          : "Synthetic inline section demo resembling inline 111.",
        `velocity_model:${MOCK_SECTION_VELOCITY_MODEL_LABEL}`
      ]
    },
    displayDefaults: {
      gain: 1.15,
      renderMode: "heatmap",
      colormap: "grayscale"
    },
    presentation: {
      title: kind === "arbitrary" ? `Arbitrary Section ${titleSuffix}` : `Inline 111 ${titleSuffix}`,
      sampleAxisLabel
    },
    overlay: {
      kind: "occupancy",
      width: model.traces,
      height: model.samples,
      values: model.qcMask,
      opacity: 0.24
    }
  };
}

export function createMockSectionScalarOverlays(
  kind: MockSectionKind = "inline",
  domain: MockSectionDomain = "time",
  options: MockSectionScalarOverlayOptions = {}
): SectionScalarOverlay[] {
  const model = buildMockSectionModel(kind);
  return [
    {
      id: `${kind}-${domain}-velocity`,
      name: "Velocity Model",
      width: model.traces,
      height: model.samples,
      values: domain === "depth" ? model.velocityDepth : model.velocityTime,
      colorMap: options.colorMap ?? "turbo",
      opacity: options.opacity ?? 0.58,
      valueRange: { min: 1500, max: 3600 },
      units: "m/s"
    }
  ];
}

export function createMockSectionHorizons(
  kind: MockSectionKind = "inline",
  domain: MockSectionDomain = "time"
): SectionHorizonOverlay[] {
  const model = buildMockSectionModel(kind);

  return [
    {
      id: `${kind}-${domain}-horizon-cyan`,
      name: "Cyan Horizon",
      color: "#67e8f9",
      lineWidth: 3,
      lineStyle: "solid",
      samples: buildSectionHorizonSamples(model, domain, (traceIndex) => 118 + Math.sin(traceIndex / 14) * 5 + traceIndex * 0.04)
    },
    {
      id: `${kind}-${domain}-horizon-red`,
      name: "Red Horizon",
      color: "#ef4444",
      lineWidth: 4,
      lineStyle: "solid",
      samples: buildSectionHorizonSamples(model, domain, (traceIndex) => 132 + Math.sin(traceIndex / 18) * 6 + traceIndex * 0.22)
    },
    {
      id: `${kind}-${domain}-horizon-amber`,
      name: "Amber Horizon",
      color: "#f59e0b",
      lineWidth: 2,
      lineStyle: "dotted",
      opacity: 0.9,
      samples: buildSectionHorizonSamples(
        model,
        domain,
        (traceIndex) => 124 + Math.cos(traceIndex / 11) * 4 + traceIndex * 0.11,
        (traceIndex) => traceIndex < 48 || traceIndex > 140
      )
    }
  ];
}

export function createMockSectionWellOverlays(
  kind: MockSectionKind = "inline",
  domain: MockSectionDomain = "time"
): SectionWellOverlay[] {
  const model = buildMockSectionModel(kind);
  return [
    {
      id: `${kind}-${domain}-well-alpha`,
      name: "Well Alpha",
      color: "#f97316",
      lineWidth: 2.5,
      lineStyle: domain === "time" ? "dashed" : "solid",
      segments: [
        {
          samples: buildSectionWellSamples(model, domain, (traceIndex) => ({
            traceIndex,
            samplePosition: 74 + traceIndex * 0.42 + Math.sin(traceIndex / 16) * 3.2
          }))
        }
      ]
    },
    {
      id: `${kind}-${domain}-well-bravo`,
      name: "Well Bravo",
      color: "#22c55e",
      lineWidth: 2.5,
      lineStyle: domain === "time" ? "dashed" : "solid",
      segments: [
        {
          samples: buildSectionWellSamples(
            model,
            domain,
            (traceIndex) => ({
              traceIndex,
              samplePosition: 108 + Math.sin(traceIndex / 13) * 5.5 + traceIndex * 0.18
            }),
            (traceIndex) => traceIndex >= 36 && traceIndex <= 154
          )
        }
      ]
    }
  ];
}

function buildMockSectionModel(kind: MockSectionKind): MockSectionModel {
  const traces = 192;
  const samples = 256;
  const dtMs = 4;
  const amplitudesTime = new Float32Array(traces * samples);
  const velocityTime = new Float32Array(traces * samples);
  const depthByTrace = new Float32Array(traces * samples);
  const qcMask = new Uint8Array(traces * samples);
  const horizontalAxis =
    kind === "arbitrary"
      ? Float64Array.from({ length: traces }, (_, index) => index + 1)
      : Float64Array.from({ length: traces }, (_, index) => 875 + index);
  const inlineAxis =
    kind === "arbitrary"
      ? Float64Array.from({ length: traces }, (_, index) => 1042 + index * 0.38 + Math.sin(index / 19) * 6)
      : undefined;
  const xlineAxis =
    kind === "arbitrary"
      ? Float64Array.from({ length: traces }, (_, index) => 4000 + index * 1.6 + Math.cos(index / 23) * 12)
      : undefined;
  const timeAxis = Float32Array.from({ length: samples }, (_, index) => index * dtMs);

  for (let trace = 0; trace < traces; trace += 1) {
    const traceOffset = trace * samples;
    let cumulativeDepth = 0;
    const layer1 = 46 + Math.sin(trace / 24) * 6;
    const layer2 = 92 + Math.sin(trace / 17 + 0.6) * 8;
    const layer3 = 136 + Math.sin(trace / 20 + 1.3) * 10;
    const layer4 = 182 + Math.sin(trace / 28 + 1.8) * 12;
    const uplift = Math.exp(-((trace - 112) ** 2) / 900) * 14;

    for (let sample = 0; sample < samples; sample += 1) {
      const index = traceOffset + sample;
      const timeMs = timeAxis[sample]!;
      const timeSeconds = timeMs / 1000;
      const adjustedSample = sample - uplift;
      const layerIndex =
        adjustedSample < layer1 ? 0 : adjustedSample < layer2 ? 1 : adjustedSample < layer3 ? 2 : adjustedSample < layer4 ? 3 : 4;
      const baseVelocity = [1525, 1825, 2225, 2740, 3320][layerIndex]!;
      const lateralTrend = Math.sin(trace / 18) * 115 + Math.cos(trace / 33 + sample / 27) * 65;
      const localUndulation = Math.sin(sample / 15 + trace / 29) * 38;
      const velocity = baseVelocity + timeSeconds * 260 + lateralTrend + localUndulation;

      velocityTime[index] = velocity;
      cumulativeDepth += velocity * (dtMs / 1000) * 0.5;
      depthByTrace[index] = cumulativeDepth;

      const layered = Math.sin(sample / 8 + trace / 20) * 0.52;
      const dipping = Math.sin(sample / 20 - trace / 13) * 0.34;
      const reflector1 = gaussian(sample, layer1 + 6, 2.4) * 0.9;
      const reflector2 = gaussian(sample, layer2 + 5, 2.6) * -1.1;
      const reflector3 = gaussian(sample, layer3 + 3, 3.2) * 0.85;
      const reflector4 = gaussian(sample, layer4, 3.4) * -0.95;
      const brightSpot = Math.exp(-((trace - 108) ** 2 + (sample - 150) ** 2) / 1500) * 0.9;

      amplitudesTime[index] = layered + dipping + reflector1 + reflector2 + reflector3 + reflector4 + brightSpot;
      qcMask[index] = sample > 176 && trace % 29 < 2 ? 1 : 0;
    }
  }

  const depthAxis = buildDepthAxis(depthByTrace, traces, samples);
  const amplitudesDepth = resampleSectionToDepth(amplitudesTime, depthByTrace, traces, samples, depthAxis);
  const velocityDepth = resampleSectionToDepth(velocityTime, depthByTrace, traces, samples, depthAxis);

  return {
    traces,
    samples,
    horizontalAxis,
    inlineAxis,
    xlineAxis,
    timeAxis,
    depthAxis,
    amplitudesTime,
    amplitudesDepth,
    velocityTime,
    velocityDepth,
    depthByTrace,
    qcMask
  };
}

function buildDepthAxis(depthByTrace: Float32Array, traces: number, samples: number): Float32Array {
  let maxDepth = 0;
  for (let trace = 0; trace < traces; trace += 1) {
    maxDepth = Math.max(maxDepth, depthByTrace[trace * samples + samples - 1]!);
  }

  return Float32Array.from({ length: samples }, (_, index) => (index / Math.max(1, samples - 1)) * maxDepth);
}

function resampleSectionToDepth(
  sourceValues: Float32Array,
  depthByTrace: Float32Array,
  traces: number,
  samples: number,
  depthAxis: Float32Array
): Float32Array {
  const result = new Float32Array(traces * samples);

  for (let trace = 0; trace < traces; trace += 1) {
    const traceOffset = trace * samples;
    let lowerSample = 0;

    for (let depthSample = 0; depthSample < samples; depthSample += 1) {
      const targetDepth = depthAxis[depthSample]!;
      while (
        lowerSample < samples - 2 &&
        depthByTrace[traceOffset + lowerSample + 1]! < targetDepth
      ) {
        lowerSample += 1;
      }

      const lowerDepth = depthByTrace[traceOffset + lowerSample]!;
      const upperIndex = Math.min(samples - 1, lowerSample + 1);
      const upperDepth = depthByTrace[traceOffset + upperIndex]!;
      const ratio =
        upperDepth <= lowerDepth ? 0 : clamp01((targetDepth - lowerDepth) / Math.max(1e-6, upperDepth - lowerDepth));
      const sourcePosition = lowerSample + ratio * (upperIndex - lowerSample);
      result[traceOffset + depthSample] = sampleTraceValue(sourceValues, traceOffset, samples, sourcePosition);
    }
  }

  return result;
}

function buildSectionHorizonSamples(
  model: MockSectionModel,
  domain: MockSectionDomain,
  sampleIndexAtTrace: (traceIndex: number) => number,
  includeTrace: (traceIndex: number) => boolean = () => true
): SectionHorizonOverlay["samples"] {
  const samples: SectionHorizonOverlay["samples"] = [];

  for (let traceIndex = 0; traceIndex < model.traces; traceIndex += 1) {
    if (!includeTrace(traceIndex)) {
      continue;
    }

    const timeSample = clamp(sampleIndexAtTrace(traceIndex), 0, model.samples - 1);
    if (domain === "time") {
      const sampleIndex = Math.round(timeSample);
      samples.push({
        traceIndex,
        sampleIndex,
        sampleValue: model.timeAxis[sampleIndex]
      });
      continue;
    }

    const depth = samplePositionToDepth(model.depthByTrace, model.samples, traceIndex, timeSample);
    const sampleIndex = nearestSampleIndex(model.depthAxis, depth);
    samples.push({
      traceIndex,
      sampleIndex,
      sampleValue: model.depthAxis[sampleIndex]
    });
  }

  return samples;
}

function buildSectionWellSamples(
  model: MockSectionModel,
  domain: MockSectionDomain,
  sampleAtTrace: (traceIndex: number) => { traceIndex: number; samplePosition: number },
  includeTrace: (traceIndex: number) => boolean = () => true
): SectionWellOverlay["segments"][number]["samples"] {
  const samples: SectionWellOverlay["segments"][number]["samples"] = [];

  for (let traceIndex = 0; traceIndex < model.traces; traceIndex += 1) {
    if (!includeTrace(traceIndex)) {
      continue;
    }

    const station = sampleAtTrace(traceIndex);
    const timeSample = clamp(station.samplePosition, 0, model.samples - 1);
    const measuredDepthM = 900 + traceIndex * 4 + timeSample * 9.5;

    if (domain === "time") {
      samples.push({
        traceIndex: station.traceIndex,
        traceCoordinate: model.horizontalAxis[station.traceIndex],
        sampleValue: model.timeAxis[Math.round(timeSample)],
        measuredDepthM,
        twtMs: model.timeAxis[Math.round(timeSample)]
      });
      continue;
    }

    const depth = samplePositionToDepth(model.depthByTrace, model.samples, station.traceIndex, timeSample);
    samples.push({
      traceIndex: station.traceIndex,
      traceCoordinate: model.horizontalAxis[station.traceIndex],
      sampleValue: depth,
      measuredDepthM,
      trueVerticalDepthM: depth
    });
  }

  return samples;
}

function samplePositionToDepth(depthByTrace: Float32Array, samples: number, traceIndex: number, samplePosition: number): number {
  const traceOffset = traceIndex * samples;
  const lower = Math.max(0, Math.min(samples - 1, Math.floor(samplePosition)));
  const upper = Math.max(0, Math.min(samples - 1, Math.ceil(samplePosition)));
  const ratio = clamp01(samplePosition - lower);
  const lowerDepth = depthByTrace[traceOffset + lower]!;
  const upperDepth = depthByTrace[traceOffset + upper]!;
  return lowerDepth + (upperDepth - lowerDepth) * ratio;
}

function sampleTraceValue(values: Float32Array, traceOffset: number, samples: number, samplePosition: number): number {
  const lower = Math.max(0, Math.min(samples - 1, Math.floor(samplePosition)));
  const upper = Math.max(0, Math.min(samples - 1, Math.ceil(samplePosition)));
  const ratio = clamp01(samplePosition - lower);
  const lowerValue = values[traceOffset + lower]!;
  const upperValue = values[traceOffset + upper]!;
  return lowerValue + (upperValue - lowerValue) * ratio;
}

function nearestSampleIndex(axis: Float32Array, value: number): number {
  let nearestIndex = 0;
  let nearestDistance = Number.POSITIVE_INFINITY;

  for (let index = 0; index < axis.length; index += 1) {
    const distance = Math.abs(axis[index]! - value);
    if (distance < nearestDistance) {
      nearestDistance = distance;
      nearestIndex = index;
    }
  }

  return nearestIndex;
}

function gaussian(sample: number, center: number, width: number): number {
  return Math.exp(-((sample - center) ** 2) / (2 * width ** 2));
}

function clamp(value: number, min: number, max: number): number {
  return Math.min(Math.max(value, min), max);
}

function clamp01(value: number): number {
  return clamp(value, 0, 1);
}
