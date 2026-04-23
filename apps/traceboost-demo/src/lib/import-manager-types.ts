import type { ImportProviderDescriptor, ImportProviderId } from "./bridge";

export type ImportManagerProviderGroup =
  | "seismic"
  | "wells"
  | "well_time_depth"
  | "projects"
  | string;

export type ImportManagerRefreshScope =
  | "workspace_registry"
  | "active_store"
  | "horizon_assets"
  | "velocity_models"
  | "project_well_inventory"
  | "project_well_time_depth_models"
  | "project_survey_horizons";

export type ImportManagerRequestAction =
  | "open_project_settings"
  | "choose_sources"
  | "revalidate_session"
  | "close_after_success";

export interface ImportManagerCanonicalAssetSummary {
  kind: string;
  id?: string | null;
  label: string;
  detail?: string | null;
}

export interface ImportManagerPreservedSourceSummary {
  kind: string;
  label: string;
  sourceRef?: string | null;
  detail?: string | null;
}

export interface ImportManagerDroppedItemSummary {
  kind: string;
  label: string;
  reason: string;
}

export type ImportManagerActivationEffect =
  | {
      kind: "open_runtime_store";
      storePath: string;
      sourcePath?: string | null;
    }
  | {
      kind: "activate_velocity_model";
      assetId: string | null;
    }
  | {
      kind: "refresh_section";
    }
  | {
      kind: "open_project_settings";
    };

export type ImportManagerOutcome =
  | "preview_only"
  | "source_only_committed"
  | "partial_canonical_commit"
  | "canonical_commit"
  | "commit_failed";

export interface ImportManagerNormalizedResult {
  providerId: ImportProviderId;
  status: "preview_ready" | "commit_succeeded" | "commit_failed";
  outcome: ImportManagerOutcome;
  canonicalAssets: ImportManagerCanonicalAssetSummary[];
  preservedSources: ImportManagerPreservedSourceSummary[];
  droppedItems: ImportManagerDroppedItemSummary[];
  warnings: string[];
  blockers: string[];
  diagnostics: string[];
  refreshScopes: ImportManagerRefreshScope[];
  activationEffects: ImportManagerActivationEffect[];
  requestActions?: ImportManagerRequestAction[];
  providerDetail?: Record<string, unknown> | null;
}

export interface ImportManagerContextSnapshot {
  tauriRuntime: boolean;
  activeStorePath: string;
  projectRoot: string;
  projectSurveyAssetId: string;
  projectWellboreId: string;
  selectedWellboreLabel: string | null;
}

export interface ImportManagerRequirement {
  key:
    | "desktop_runtime"
    | "active_store"
    | "project_root"
    | "project_well_binding";
  satisfied: boolean;
  message: string;
}

export interface ImportManagerProviderMatch {
  providerId: ImportProviderId | null;
  ambiguous: boolean;
  matches: ImportProviderId[];
}

export function normalizeImportSourceRefs(sourceRefs: string[] | null | undefined): string[] {
  const normalized: string[] = [];
  for (const sourceRef of sourceRefs ?? []) {
    const trimmed = sourceRef.trim();
    if (trimmed && !normalized.includes(trimmed)) {
      normalized.push(trimmed);
    }
  }
  return normalized;
}

export function providerRequirementState(
  provider: ImportProviderDescriptor,
  context: ImportManagerContextSnapshot
): ImportManagerRequirement[] {
  return [
    {
      key: "desktop_runtime",
      satisfied: context.tauriRuntime,
      message: "Desktop runtime is required for import orchestration."
    },
    {
      key: "active_store",
      satisfied: !provider.requiresActiveStore || context.activeStorePath.length > 0,
      message: "Open a seismic volume before using this import flow."
    },
    {
      key: "project_root",
      satisfied: !provider.requiresProjectRoot || context.projectRoot.length > 0,
      message: "Set the Ophiolite project root before committing this import."
    },
    {
      key: "project_well_binding",
      satisfied:
        !provider.requiresProjectWellBinding || context.projectWellboreId.length > 0,
      message: "Select a project wellbore before using this import flow."
    }
  ];
}

export function providerGroupLabel(group: ImportManagerProviderGroup): string {
  switch (group) {
    case "seismic":
      return "Seismic";
    case "wells":
      return "Wells";
    case "well_time_depth":
      return "Well Time-Depth";
    case "projects":
      return "Projects";
    default:
      return group.replace(/_/g, " ");
  }
}

export function sourceLabelFromPath(path: string): string {
  const normalized = path.trim().replace(/\\/g, "/");
  if (!normalized) {
    return "";
  }
  return normalized.split("/").pop() ?? normalized;
}

export function inferProviderForSourceRefs(
  providers: ImportProviderDescriptor[],
  sourceRefs: string[]
): ImportManagerProviderMatch {
  const normalized = normalizeImportSourceRefs(sourceRefs);
  if (normalized.length === 0) {
    return { providerId: null, ambiguous: false, matches: [] };
  }

  if (
    normalized.length === 1 &&
    normalized[0] &&
    normalized[0].toLowerCase().endsWith(".tbvol")
  ) {
    return { providerId: null, ambiguous: false, matches: [] };
  }

  const matches = providers
    .filter((provider) => provider.supportsDragDrop)
    .filter((provider) =>
      normalized.every((sourceRef) => providerAcceptsSourceRef(provider, sourceRef))
    )
    .map((provider) => provider.providerId);

  if (matches.length === 1) {
    return { providerId: matches[0] ?? null, ambiguous: false, matches };
  }
  return { providerId: null, ambiguous: matches.length > 1, matches };
}

function providerAcceptsSourceRef(
  provider: ImportProviderDescriptor,
  sourceRef: string
): boolean {
  const normalized = sourceRef.trim().replace(/\\/g, "/");
  if (!normalized) {
    return false;
  }
  if (provider.supportsDirectory && !/\.[^/.\\]+$/u.test(normalized)) {
    return true;
  }
  const extension = normalized.split(".").pop()?.toLowerCase() ?? "";
  if (!extension) {
    return false;
  }
  return provider.supportedExtensions.some((candidate) => candidate.toLowerCase() === extension);
}
