import type {
  ChartDefinitionId,
  ChartRendererStatus,
  ChartRendererTelemetryEvent
} from "@ophiolite/charts-data-models";
import { getChartDefinition } from "@ophiolite/charts-data-models";
import { resolveChartRendererStatus } from "@ophiolite/charts-renderer";
import type { ChartRendererConfig, ChartRendererStatusPayload, ChartRendererStatusChangeHandler } from "./types";

const DEFAULT_AVAILABLE_BACKENDS = ["canvas-2d"] as const;

export function applyRuntimeRendererStatusOverride(
  status: ChartRendererStatus,
  runtimeErrorMessage?: string | null
): ChartRendererStatus {
  const message = runtimeErrorMessage?.trim();
  if (!message || status.availability === "unavailable" || status.availability === "runtime-failure") {
    return status;
  }
  return {
    ...status,
    availability: "runtime-failure",
    reason: "runtime-error",
    detail: `Renderer runtime failed after initialization: ${message}`
  };
}

export function applyRendererTelemetryToStatus(
  status: ChartRendererStatus,
  telemetryEvent?: ChartRendererTelemetryEvent | null
): ChartRendererStatus {
  if (!telemetryEvent) {
    return status;
  }

  if (telemetryEvent.kind === "fallback-used") {
    return {
      ...status,
      activeBackend: telemetryEvent.backend ?? status.activeBackend,
      availability: status.availability === "unavailable" ? "unavailable" : "fallback",
      reason: status.availability === "unavailable" ? status.reason : "preferred-backend-unavailable",
      detail: telemetryEvent.detail ?? telemetryEvent.message
    };
  }

  if (
    telemetryEvent.kind === "mount-failed" ||
    telemetryEvent.kind === "frame-failed" ||
    telemetryEvent.kind === "context-lost"
  ) {
    if (status.availability === "unavailable") {
      return status;
    }
    return {
      ...status,
      activeBackend: telemetryEvent.backend ?? status.activeBackend,
      availability: "runtime-failure",
      reason: "runtime-error",
      detail: telemetryEvent.detail ?? telemetryEvent.message
    };
  }

  if (telemetryEvent.kind === "backend-selected" || telemetryEvent.kind === "context-restored") {
    return {
      ...status,
      activeBackend: telemetryEvent.backend ?? status.activeBackend,
      detail: telemetryEvent.detail ?? telemetryEvent.message
    };
  }

  if (telemetryEvent.kind === "warning") {
    return {
      ...status,
      detail: telemetryEvent.detail ?? telemetryEvent.message
    };
  }

  return status;
}

export function resolveRendererStatusPayloadForChart(
  chartDefinitionId: ChartDefinitionId,
  payload: {
    chartId: string;
    viewId?: string;
    renderer?: ChartRendererConfig;
    telemetryEvent?: ChartRendererTelemetryEvent | null;
  }
): ChartRendererStatusPayload {
  const definition = getChartDefinition(chartDefinitionId);
  const status = applyRendererTelemetryToStatus(
    applyRuntimeRendererStatusOverride(
      resolveChartRendererStatus({
        chartDefinitionId: definition.id,
        rendererKernel: definition.rendererKernel,
        supportTier: definition.supportTier,
        supportedBackends: definition.rendererBackends.map((backend) => backend.id),
        defaultBackend: definition.rendererBackends.find((backend) => backend.default)?.id ?? null,
        preference: payload.renderer?.backendPreference ?? "auto",
        availableBackends: payload.renderer?.availableBackends ?? DEFAULT_AVAILABLE_BACKENDS
      }),
      payload.renderer?.runtimeErrorMessage
    ),
    payload.telemetryEvent
  );
  return {
    chartId: payload.chartId,
    viewId: payload.viewId,
    status
  };
}

export function emitRendererStatusForChart(
  chartDefinitionId: ChartDefinitionId,
  payload: {
    chartId: string;
    viewId?: string;
    renderer?: ChartRendererConfig;
    telemetryEvent?: ChartRendererTelemetryEvent | null;
  },
  lastKey: string,
  onRendererStatusChange?: ChartRendererStatusChangeHandler
): string {
  const nextPayload = resolveRendererStatusPayloadForChart(chartDefinitionId, payload);
  const nextKey = JSON.stringify(nextPayload);
  if (nextKey !== lastKey) {
    onRendererStatusChange?.(nextPayload);
  }
  return nextKey;
}
