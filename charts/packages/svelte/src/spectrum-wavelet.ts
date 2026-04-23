import type { DerivedWavelet, SpectrumResponseLike } from "./types";

export function deriveZeroPhaseWavelet(response: SpectrumResponseLike | null): DerivedWavelet | null {
  if (!response) {
    return null;
  }

  const frequencies = response.curve.frequenciesHz;
  const amplitudes = response.curve.amplitudes;
  const sampleIntervalMs = response.sampleIntervalMs;
  if (frequencies.length < 2 || amplitudes.length < 2 || frequencies.length !== amplitudes.length) {
    return null;
  }
  if (!Number.isFinite(sampleIntervalMs) || sampleIntervalMs <= 0) {
    return null;
  }

  const sampleCount = Math.max(2, (frequencies.length - 1) * 2);
  const halfNyquistIndex = frequencies.length - 1;
  const waveform = new Array<number>(sampleCount).fill(0);

  for (let sampleIndex = 0; sampleIndex < sampleCount; sampleIndex += 1) {
    let value = amplitudes[0] ?? 0;

    for (let frequencyIndex = 1; frequencyIndex < halfNyquistIndex; frequencyIndex += 1) {
      const amplitude = amplitudes[frequencyIndex] ?? 0;
      value += amplitude * Math.cos((2 * Math.PI * frequencyIndex * sampleIndex) / sampleCount);
    }

    if (halfNyquistIndex > 0) {
      const nyquistAmplitude = amplitudes[halfNyquistIndex] ?? 0;
      value += nyquistAmplitude * Math.cos(Math.PI * sampleIndex);
    }

    waveform[sampleIndex] = value;
  }

  const centered = circularShift(waveform, Math.floor(sampleCount / 2));
  const dominantFrequencyHz = resolveDominantFrequency(frequencies, amplitudes);
  const halfWindowMs = resolveWaveletHalfWindowMs(dominantFrequencyHz);
  const halfWindowSamples = Math.max(
    2,
    Math.min(Math.floor(centered.length / 2), Math.round(halfWindowMs / sampleIntervalMs))
  );
  const centerIndex = Math.floor(centered.length / 2);
  const startIndex = Math.max(0, centerIndex - halfWindowSamples);
  const endIndex = Math.min(centered.length, centerIndex + halfWindowSamples + 1);
  const cropped = centered.slice(startIndex, endIndex);
  const peakAmplitude = Math.max(1.0e-12, ...cropped.map((value) => Math.abs(value)));

  return {
    assumption: "zero_phase",
    dominantFrequencyHz,
    timesMs: cropped.map((_, index) => (startIndex + index - centerIndex) * sampleIntervalMs),
    amplitudes: cropped.map((value) => value / peakAmplitude)
  };
}

function resolveDominantFrequency(frequenciesHz: number[], amplitudes: number[]): number | null {
  let bestIndex = -1;
  let bestAmplitude = Number.NEGATIVE_INFINITY;

  for (let index = 1; index < frequenciesHz.length; index += 1) {
    const amplitude = amplitudes[index] ?? 0;
    if (Number.isFinite(amplitude) && amplitude > bestAmplitude) {
      bestAmplitude = amplitude;
      bestIndex = index;
    }
  }

  return bestIndex > 0 ? (frequenciesHz[bestIndex] ?? null) : null;
}

function resolveWaveletHalfWindowMs(dominantFrequencyHz: number | null): number {
  if (!dominantFrequencyHz || !Number.isFinite(dominantFrequencyHz) || dominantFrequencyHz <= 0) {
    return 80;
  }

  const dominantPeriodMs = 1000 / dominantFrequencyHz;
  return Math.max(32, Math.min(180, dominantPeriodMs * 3));
}

function circularShift(values: number[], offset: number): number[] {
  const size = values.length;
  if (size === 0) {
    return [];
  }

  const shift = ((offset % size) + size) % size;
  return values.map((_, index) => values[(index + shift) % size] ?? 0);
}
