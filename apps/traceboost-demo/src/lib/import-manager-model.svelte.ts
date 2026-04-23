import { createContext } from "svelte";
import type {
  BeginImportSessionRequest,
  ImportProviderDescriptor,
  ImportProviderId,
  ImportSessionEnvelope
} from "./bridge";
import { beginImportSession, listImportProviders } from "./bridge";
import type { ViewerModel } from "./viewer-model.svelte";
import {
  type ImportManagerContextSnapshot,
  type ImportManagerNormalizedResult,
  type ImportManagerProviderMatch,
  type ImportManagerRequirement,
  inferProviderForSourceRefs,
  normalizeImportSourceRefs,
  providerRequirementState
} from "./import-manager-types";

export type ImportManagerPendingAction = "open" | "import" | "deep_link";

export interface OpenImportManagerOptions {
  providerId?: ImportProviderId | null;
  sourceRefs?: string[];
  pendingAction?: ImportManagerPendingAction;
}

interface ImportManagerModelOptions {
  viewerModel: ViewerModel;
}

interface ProviderSessionCache {
  sourceRefs: string[];
  lastResult: ImportManagerNormalizedResult | null;
}

export function commonWellImportRoot(inputPaths: string[]): string | null {
  const normalized = normalizeImportSourceRefs(inputPaths).map((value) => value.replace(/\\/g, "/"));
  if (normalized.length === 0) {
    return null;
  }

  const splitParent = (value: string): string[] => {
    const parts = value.split("/");
    parts.pop();
    return parts.filter((part, index) => part.length > 0 || index === 0);
  };

  let shared = splitParent(normalized[0]!);
  for (const path of normalized.slice(1)) {
    const next = splitParent(path);
    const sharedLength = Math.min(shared.length, next.length);
    let index = 0;
    while (index < sharedLength && shared[index] === next[index]) {
      index += 1;
    }
    shared = shared.slice(0, index);
  }

  if (shared.length === 0) {
    const fallbackPath = normalized[0]!;
    const separatorIndex = fallbackPath.lastIndexOf("/");
    return separatorIndex > 0 ? fallbackPath.slice(0, separatorIndex) : null;
  }
  return shared.join("/") || null;
}

export class ImportManagerModel {
  readonly viewerModel: ViewerModel;

  open = $state(false);
  loading = $state(false);
  applyingEffects = $state(false);
  error = $state<string | null>(null);
  providers = $state.raw<ImportProviderDescriptor[]>([]);
  activeProviderId = $state<ImportProviderId | null>(null);
  pendingAction = $state<ImportManagerPendingAction>("import");
  session = $state.raw<ImportSessionEnvelope | null>(null);
  sourceRefs = $state.raw<string[]>([]);
  currentResult = $state.raw<ImportManagerNormalizedResult | null>(null);
  contextNotice = $state<string | null>(null);
  dragDropActive = $state(false);
  dragDropSourceRefs = $state.raw<string[]>([]);
  lastProviderMatch = $state.raw<ImportManagerProviderMatch>({
    providerId: null,
    ambiguous: false,
    matches: []
  });
  recentSourcesByProvider = $state.raw<Record<string, string[]>>({});
  providerSessionCache = $state.raw<Record<string, ProviderSessionCache>>({});
  lastContextFingerprint = "";

  constructor(options: ImportManagerModelOptions) {
    this.viewerModel = options.viewerModel;
  }

  get currentProvider(): ImportProviderDescriptor | null {
    if (!this.activeProviderId) {
      return null;
    }
    return this.providers.find((provider) => provider.providerId === this.activeProviderId) ?? null;
  }

  get contextSnapshot(): ImportManagerContextSnapshot {
    const selectedWellbore = this.viewerModel.selectedProjectWellboreInventoryItem;
    return {
      tauriRuntime: this.viewerModel.tauriRuntime,
      activeStorePath: this.viewerModel.activeStorePath.trim(),
      projectRoot: this.viewerModel.projectRoot.trim(),
      projectSurveyAssetId: this.viewerModel.projectSurveyAssetId.trim(),
      projectWellboreId: this.viewerModel.projectWellboreId.trim(),
      selectedWellboreLabel: selectedWellbore
        ? `${selectedWellbore.wellName} | ${selectedWellbore.wellboreName}`
        : null
    };
  }

  get currentSourceRefs(): string[] {
    return this.session?.sourceRefs ?? this.sourceRefs;
  }

  get sourceCount(): number {
    return this.currentSourceRefs.length;
  }

  get currentSourceRef(): string | null {
    return this.currentSourceRefs[0] ?? null;
  }

  get currentRequirements(): ImportManagerRequirement[] {
    if (!this.currentProvider) {
      return [];
    }
    return providerRequirementState(this.currentProvider, this.contextSnapshot);
  }

  get currentBlockedRequirements(): ImportManagerRequirement[] {
    return this.currentRequirements.filter((requirement) => !requirement.satisfied);
  }

  get currentRecentSources(): string[] {
    if (!this.activeProviderId) {
      return [];
    }
    return this.recentSourcesByProvider[this.activeProviderId] ?? [];
  }

  async ensureProviders(): Promise<void> {
    if (this.providers.length > 0) {
      return;
    }
    this.providers = await listImportProviders();
  }

  async openManager(options: OpenImportManagerOptions = {}): Promise<void> {
    this.open = true;
    this.error = null;
    this.contextNotice = null;
    this.currentResult = null;
    this.pendingAction = options.pendingAction ?? "import";
    this.sourceRefs = normalizeImportSourceRefs(options.sourceRefs);
    await this.ensureProviders();

    const inferredMatch =
      options.providerId || this.sourceRefs.length === 0
        ? { providerId: null, ambiguous: false, matches: [] }
        : inferProviderForSourceRefs(this.providers, this.sourceRefs);
    this.lastProviderMatch = inferredMatch;

    const nextProviderId =
      options.providerId ??
      inferredMatch.providerId ??
      this.activeProviderId ??
      (this.providers.length === 1 ? this.providers[0]!.providerId : null);

    this.activeProviderId = nextProviderId ?? null;
    if (this.activeProviderId && this.sourceRefs.length === 0) {
      this.sourceRefs = this.providerSessionCache[this.activeProviderId]?.sourceRefs ?? [];
      this.currentResult = this.providerSessionCache[this.activeProviderId]?.lastResult ?? null;
    }

    this.lastContextFingerprint = this.captureContextFingerprint();
    if (this.activeProviderId) {
      await this.startSession({
        providerId: this.activeProviderId,
        sourceRefs: this.sourceRefs
      });
      return;
    }

    this.session = null;
  }

  closeManager(): void {
    this.open = false;
    this.error = null;
    this.session = null;
    this.currentResult = null;
    this.contextNotice = null;
    this.dragDropActive = false;
    this.dragDropSourceRefs = [];
    this.sourceRefs = [];
    this.pendingAction = "import";
  }

  async selectProvider(providerId: ImportProviderId): Promise<void> {
    if (providerId === this.activeProviderId && this.session) {
      return;
    }
    const cached = this.providerSessionCache[providerId];
    this.activeProviderId = providerId;
    this.sourceRefs = cached?.sourceRefs ?? [];
    this.error = null;
    this.currentResult = cached?.lastResult ?? null;
    await this.startSession({
      providerId,
      sourceRefs: this.sourceRefs
    });
  }

  async replaceSourceRefs(sourceRefs: string[]): Promise<void> {
    this.sourceRefs = normalizeImportSourceRefs(sourceRefs);
    this.rememberRecentSources(this.activeProviderId, this.sourceRefs);
    this.cacheActiveProviderState();
    if (!this.activeProviderId) {
      this.session = null;
      return;
    }
    await this.startSession({
      providerId: this.activeProviderId,
      sourceRefs: this.sourceRefs
    });
  }

  async clearSourceRefs(): Promise<void> {
    await this.replaceSourceRefs([]);
  }

  async handleDroppedSourceRefs(sourceRefs: string[]): Promise<void> {
    await this.ensureProviders();
    const normalizedSourceRefs = normalizeImportSourceRefs(sourceRefs);
    if (normalizedSourceRefs.length === 0) {
      return;
    }
    if (normalizedSourceRefs.length === 1 && /\.tbvol$/i.test(normalizedSourceRefs[0]!)) {
      await this.viewerModel.openManagedVolumePath(normalizedSourceRefs[0]!);
      return;
    }
    const match = inferProviderForSourceRefs(this.providers, normalizedSourceRefs);
    this.lastProviderMatch = match;
    await this.openManager({
      providerId: match.providerId,
      sourceRefs: normalizedSourceRefs,
      pendingAction: match.providerId ? "deep_link" : "import"
    });
  }

  setDragDropState(active: boolean, sourceRefs: string[] = []): void {
    this.dragDropActive = active;
    this.dragDropSourceRefs = normalizeImportSourceRefs(sourceRefs);
  }

  async applyNormalizedResult(result: ImportManagerNormalizedResult): Promise<void> {
    this.currentResult = result;
    this.cacheActiveProviderState();
    if (result.status !== "commit_succeeded") {
      return;
    }

    this.applyingEffects = true;
    try {
      for (const scope of result.refreshScopes) {
        await this.applyRefreshScope(scope);
      }
      for (const effect of result.activationEffects) {
        await this.applyActivationEffect(effect);
      }
      if (result.requestActions?.includes("close_after_success")) {
        this.closeManager();
      }
    } finally {
      this.applyingEffects = false;
    }
  }

  syncContext(): void {
    const nextFingerprint = this.captureContextFingerprint();
    if (nextFingerprint === this.lastContextFingerprint) {
      return;
    }
    const previousFingerprint = this.lastContextFingerprint;
    this.lastContextFingerprint = nextFingerprint;
    if (!this.open || !this.activeProviderId) {
      return;
    }
    this.contextNotice = previousFingerprint
      ? "Import context changed. Requirements and previews were re-evaluated against the current project and survey state."
      : null;
    void this.startSession({
      providerId: this.activeProviderId,
      sourceRefs: this.sourceRefs
    });
  }

  private captureContextFingerprint(): string {
    return JSON.stringify(this.contextSnapshot);
  }

  private cacheActiveProviderState(): void {
    if (!this.activeProviderId) {
      return;
    }
    this.providerSessionCache = {
      ...this.providerSessionCache,
      [this.activeProviderId]: {
        sourceRefs: [...this.sourceRefs],
        lastResult: this.currentResult
      }
    };
  }

  private rememberRecentSources(providerId: ImportProviderId | null, sourceRefs: string[]): void {
    if (!providerId || sourceRefs.length === 0) {
      return;
    }
    const previous = this.recentSourcesByProvider[providerId] ?? [];
    const nextRecentSources = [...sourceRefs, ...previous].filter(
      (sourceRef, index, values) => values.indexOf(sourceRef) === index
    );
    this.recentSourcesByProvider = {
      ...this.recentSourcesByProvider,
      [providerId]: nextRecentSources.slice(0, 8)
    };
  }

  private async applyRefreshScope(
    scope: ImportManagerNormalizedResult["refreshScopes"][number]
  ): Promise<void> {
    const context = this.contextSnapshot;
    switch (scope) {
      case "workspace_registry":
        await this.viewerModel.refreshWorkspaceState();
        return;
      case "active_store":
        if (context.activeStorePath) {
          await this.viewerModel.load(this.viewerModel.axis, this.viewerModel.index);
        }
        return;
      case "horizon_assets":
        await this.viewerModel.refreshHorizonAssets(context.activeStorePath);
        return;
      case "velocity_models":
        await this.viewerModel.refreshVelocityModels(context.activeStorePath);
        return;
      case "project_well_inventory":
        if (context.projectRoot) {
          await this.viewerModel.refreshProjectWellOverlayInventory(
            context.projectRoot,
            this.viewerModel.displayCoordinateReferenceId
          );
        }
        return;
      case "project_well_time_depth_models":
        if (context.projectRoot && context.projectWellboreId) {
          await this.viewerModel.refreshProjectWellTimeDepthModels(
            context.projectRoot,
            context.projectWellboreId
          );
        }
        return;
      case "project_survey_horizons":
        if (context.projectRoot && context.projectSurveyAssetId) {
          await this.viewerModel.refreshProjectSurveyHorizons(
            context.projectRoot,
            context.projectSurveyAssetId
          );
        }
        return;
    }
  }

  private async applyActivationEffect(
    effect: ImportManagerNormalizedResult["activationEffects"][number]
  ): Promise<void> {
    switch (effect.kind) {
      case "open_runtime_store":
        await this.viewerModel.openManagedVolumePath(effect.storePath);
        return;
      case "activate_velocity_model":
        await this.viewerModel.activateVelocityModel(effect.assetId);
        return;
      case "refresh_section":
        if (this.viewerModel.activeStorePath.trim()) {
          await this.viewerModel.load(this.viewerModel.axis, this.viewerModel.index);
        }
        return;
      case "open_project_settings":
        this.viewerModel.openProjectSettings();
        return;
    }
  }

  private async startSession(request: BeginImportSessionRequest): Promise<void> {
    this.loading = true;
    this.error = null;
    try {
      this.session = await beginImportSession(request);
      this.cacheActiveProviderState();
    } catch (error) {
      this.session = null;
      this.error = error instanceof Error ? error.message : String(error);
    } finally {
      this.loading = false;
    }
  }
}

const [internalGetImportManagerContext, internalSetImportManagerContext] =
  createContext<ImportManagerModel>();

export function getImportManagerContext(): ImportManagerModel {
  const importManager = internalGetImportManagerContext();
  if (!importManager) {
    throw new Error("Import manager context not found");
  }
  return importManager;
}

export function setImportManagerContext(importManager: ImportManagerModel): ImportManagerModel {
  internalSetImportManagerContext(importManager);
  return importManager;
}
