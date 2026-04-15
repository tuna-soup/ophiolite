export interface WellTieValueRange {
  min: number;
  max: number;
}

export interface WellTieMetric {
  id: string;
  label: string;
  value: string;
  emphasis?: "neutral" | "good" | "warn";
}

export interface WellTieMarker {
  id: string;
  label: string;
  timeMs: number;
  color?: string;
}

export interface WellTieCurveTrack {
  kind: "curve";
  id: string;
  label: string;
  unit?: string;
  color: string;
  timesMs: Float32Array;
  values: Float32Array;
  valueRange?: WellTieValueRange;
  fillColor?: string;
}

export interface WellTieWiggleTrack {
  kind: "wiggle";
  id: string;
  label: string;
  timesMs: Float32Array;
  amplitudes: Float32Array;
  lineColor?: string;
  positiveFill?: string;
  negativeFill?: string;
  amplitudeScale?: number;
}

export type WellTieTrack = WellTieCurveTrack | WellTieWiggleTrack;

export interface WellTieSectionPanel {
  id: string;
  label: string;
  timesMs: Float32Array;
  traceOffsetsM: Float32Array;
  amplitudes: Float32Array;
  traceCount: number;
  sampleCount: number;
  wellTraceIndex?: number;
  matchTraceIndex?: number;
  matchOffsetM?: number;
  wellLabel?: string;
  matchLabel?: string;
}

export type WellTieWaveletState = "provisional" | "extracted";

export interface WellTieWavelet {
  id: string;
  label: string;
  timesMs: Float32Array;
  amplitudes: Float32Array;
  amplitudeRange?: WellTieValueRange;
  state?: WellTieWaveletState;
  detail?: string;
}

export interface WellTieChartModel {
  id: string;
  name: string;
  timeRangeMs: {
    start: number;
    end: number;
    unit?: string;
  };
  depthRangeM?: {
    start: number;
    end: number;
  };
  tracks: WellTieTrack[];
  metrics?: WellTieMetric[];
  markers?: WellTieMarker[];
  section?: WellTieSectionPanel | null;
  wavelet?: WellTieWavelet | null;
  notes?: string[];
}
