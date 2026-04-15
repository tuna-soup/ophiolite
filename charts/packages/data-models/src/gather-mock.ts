import type { GatherView } from "@ophiolite/contracts";

export function createMockGatherView(): GatherView {
  const traces = 13;
  const samples = 256;
  const amplitudes = new Float32Array(traces * samples);
  const horizontalAxis = Float64Array.from({ length: traces }, (_, index) => index);
  const sampleAxis = Float32Array.from({ length: samples }, (_, index) => 2000 + index * 2);

  for (let trace = 0; trace < traces; trace += 1) {
    const angle = horizontalAxis[trace] ?? 0;
    for (let sample = 0; sample < samples; sample += 1) {
      const time = sampleAxis[sample] ?? 0;
      const index = trace * samples + sample;
      const waveletA = Math.sin(sample / 7 + trace / 3.4) * 0.18;
      const waveletB = Math.sin(sample / 14 - trace / 5.1) * 0.11;
      const event40ms = Math.exp(-((time - 2080) ** 2) / 220) * Math.cos((angle - 6) / 2.6) * 1.2;
      const event228 = Math.exp(-((time - 2280) ** 2) / 110) * (0.55 + Math.sin(angle / 3.5) * 0.95);
      const event245 = Math.exp(-((time - 2450) ** 2) / 150) * Math.cos((angle - 8) / 3.2) * -0.85;
      amplitudes[index] = waveletA + waveletB + event40ms + event228 + event245;
    }
  }

  return {
    dataset_id: "mock-prestack-gather",
    label: "Synthetic Angle Gather",
    gather_axis_kind: "angle",
    sample_domain: "time",
    traces,
    samples,
    horizontal_axis_f64le: encodeFloat64(horizontalAxis),
    sample_axis_f32le: encodeFloat32(sampleAxis),
    amplitudes_f32le: encodeFloat32(amplitudes),
    units: {
      horizontal: "deg",
      sample: "ms",
      amplitude: "arb"
    },
    metadata: {
      store_id: "mock-prestack-store",
      derived_from: null,
      notes: ["Synthetic single-gather demo for wiggle and heatmap prestack rendering."]
    },
    display_defaults: {
      gain: 1,
      clip_min: -1.2,
      clip_max: 1.2,
      render_mode: "wiggle",
      colormap: "red_white_blue",
      polarity: "normal"
    }
  };
}

function encodeFloat32(values: Float32Array): number[] {
  return Array.from(new Uint8Array(values.buffer.slice(0)));
}

function encodeFloat64(values: Float64Array): number[] {
  return Array.from(new Uint8Array(values.buffer.slice(0)));
}
