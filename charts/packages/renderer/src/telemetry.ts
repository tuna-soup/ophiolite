import type {
  ChartBackendId,
  ChartRendererTelemetryEvent,
  ChartRendererTelemetryKind,
  ChartRendererTelemetryPhase
} from "@ophiolite/charts-data-models";

export type RendererTelemetryListener = (event: ChartRendererTelemetryEvent) => void;

export interface RendererTelemetrySource {
  setTelemetryListener(listener: RendererTelemetryListener | null): void;
}

export function createRendererTelemetryEvent(input: {
  kind: ChartRendererTelemetryKind;
  phase: ChartRendererTelemetryPhase;
  backend: ChartBackendId | null;
  previousBackend?: ChartBackendId | null;
  recoverable: boolean;
  message: string;
  detail?: string;
}): ChartRendererTelemetryEvent {
  return {
    ...input,
    previousBackend: input.previousBackend ?? null,
    timestampMs: nowMs()
  };
}

function nowMs(): number {
  return typeof performance !== "undefined" ? performance.now() : Date.now();
}
