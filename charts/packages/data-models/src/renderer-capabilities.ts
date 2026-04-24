export type ChartSupportTier = "public-launch" | "public-adapter" | "preview" | "internal";

export type ChartBackendId = "canvas-2d" | "webgl" | "vtkjs";
export type ChartBackendPreference = "auto" | ChartBackendId | readonly ChartBackendId[];
export type ChartRendererAvailability = "available" | "fallback" | "runtime-failure" | "unavailable";
export type ChartRendererReasonCode =
  | "using-default-backend"
  | "using-requested-backend"
  | "preferred-backend-unavailable"
  | "runtime-error"
  | "backend-unsupported-by-chart"
  | "no-supported-backend";

export type ChartRendererTelemetryKind =
  | "backend-selected"
  | "fallback-used"
  | "mount-failed"
  | "frame-failed"
  | "context-lost"
  | "context-restored"
  | "warning";

export type ChartRendererTelemetryPhase = "probe" | "mount" | "render" | "resize" | "worker";

export interface ChartRendererBackendContract {
  id: ChartBackendId;
  default?: boolean;
  requiredFeatures?: readonly string[];
}

export interface ChartRendererConsumerGuarantee {
  id:
    | "public-package-entrypoint"
    | "neutral-data-model"
    | "ophiolite-adapter-subpath"
    | "traceboost-demo-consumer"
    | "public-docs-coverage";
  summary: string;
}

export interface ChartRendererStatus {
  chartDefinitionId: string;
  rendererKernel: string;
  supportTier: ChartSupportTier;
  requested: ChartBackendPreference;
  activeBackend: ChartBackendId | null;
  supportedBackends: readonly ChartBackendId[];
  availableBackends: readonly ChartBackendId[];
  availability: ChartRendererAvailability;
  reason: ChartRendererReasonCode;
  detail?: string;
}

export interface ChartRendererTelemetryEvent {
  kind: ChartRendererTelemetryKind;
  phase: ChartRendererTelemetryPhase;
  backend: ChartBackendId | null;
  previousBackend?: ChartBackendId | null;
  recoverable: boolean;
  message: string;
  detail?: string;
  timestampMs: number;
}

export function normalizeChartBackendPreference(preference: ChartBackendPreference): readonly ChartBackendId[] {
  if (preference === "auto") {
    return [];
  }
  return typeof preference === "string" ? [preference] : [...preference];
}
