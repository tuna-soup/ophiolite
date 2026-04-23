import type { SpectrumResponseLike } from "@ophiolite/charts";
import type { AmplitudeSpectrumResponse } from "@traceboost/seis-contracts";

export function adaptAmplitudeSpectrum(
  response: AmplitudeSpectrumResponse | null
): SpectrumResponseLike | null {
  if (!response) {
    return null;
  }

  return {
    curve: {
      frequenciesHz: response.curve.frequencies_hz,
      amplitudes: response.curve.amplitudes
    },
    sampleIntervalMs: response.sample_interval_ms,
    processingLabel: response.processing_label ?? null
  };
}
