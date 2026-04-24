import type {
  ChartBackendId,
  ChartBackendPreference,
  ChartRendererStatus,
  ChartSupportTier
} from "@ophiolite/charts-data-models";
import { normalizeChartBackendPreference } from "@ophiolite/charts-data-models";

export interface ResolveChartRendererStatusOptions {
  chartDefinitionId: string;
  rendererKernel: string;
  supportTier: ChartSupportTier;
  supportedBackends: readonly ChartBackendId[];
  defaultBackend?: ChartBackendId | null;
  preference?: ChartBackendPreference;
  availableBackends?: readonly ChartBackendId[];
}

export function resolveChartRendererStatus({
  chartDefinitionId,
  rendererKernel,
  supportTier,
  supportedBackends,
  defaultBackend,
  preference = "auto",
  availableBackends = supportedBackends
}: ResolveChartRendererStatusOptions): ChartRendererStatus {
  const requested = normalizeChartBackendPreference(preference);
  const supportedSet = new Set(supportedBackends);
  const availableSet = new Set(availableBackends.filter((backend) => supportedSet.has(backend)));
  const preferredCandidates = requested.length > 0 ? requested : defaultBackend ? [defaultBackend] : supportedBackends;
  const activeBackend = preferredCandidates.find((backend) => availableSet.has(backend)) ?? null;

  if (activeBackend) {
    const requestedIndex = requested.indexOf(activeBackend);
    const usingRequestedBackend = requestedIndex >= 0;
    const availability = requestedIndex > 0 ? "fallback" : "available";
    const reason =
      availability === "fallback"
        ? "preferred-backend-unavailable"
        : usingRequestedBackend
          ? "using-requested-backend"
          : "using-default-backend";
    return {
      chartDefinitionId,
      rendererKernel,
      supportTier,
      requested: preference,
      activeBackend,
      supportedBackends: [...supportedBackends],
      availableBackends: [...availableSet],
      availability,
      reason,
      detail:
        availability === "fallback"
          ? `Preferred backend ${requested[0]} was unavailable. Using ${activeBackend} instead.`
          : `Using ${activeBackend} renderer backend.`
    };
  }

  const reason =
    requested.length > 0 && requested.some((backend) => !supportedSet.has(backend))
      ? "backend-unsupported-by-chart"
      : "no-supported-backend";

  return {
    chartDefinitionId,
    rendererKernel,
    supportTier,
    requested: preference,
    activeBackend: null,
    supportedBackends: [...supportedBackends],
    availableBackends: [...availableSet],
    availability: "unavailable",
    reason,
    detail:
      reason === "backend-unsupported-by-chart"
        ? `Requested backend(s) ${requested.join(", ")} are not supported by this chart.`
        : "No supported renderer backend is currently available."
  };
}
