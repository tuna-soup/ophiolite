<script lang="ts">
  import SegyImportDialog from "./SegyImportDialog.svelte";
  import HorizonImportDialog from "./HorizonImportDialog.svelte";
  import WellSourceImportDialog from "./WellFolderImportDialog.svelte";
  import WellTimeDepthImportDialog from "./WellTimeDepthImportDialog.svelte";
  import VendorProjectImportDialog from "./VendorProjectImportDialog.svelte";
  import { commonWellImportRoot, getImportManagerContext } from "../import-manager-model.svelte";
  import { getViewerModelContext } from "../viewer-model.svelte";
  import {
    defaultImportStorePath,
    importDataset,
    importVelocityFunctionsModel,
    type PreviewProjectWellTimeDepthAssetRequest,
    type ProjectAssetBindingInput,
    type ImportProviderDescriptor,
    type ImportProviderId
  } from "../bridge";
  import {
    providerGroupLabel,
    sourceLabelFromPath,
    type ImportManagerNormalizedResult
  } from "../import-manager-types";
  import {
    pickHorizonFiles,
    pickImportSeismicFile,
    pickVendorProjectFolder,
    pickVelocityFunctionsFile,
    pickWellImportFiles,
    pickWellTimeDepthJsonFile
  } from "../file-dialog";

  interface Props {
    openSettings: () => void;
  }

  let { openSettings }: Props = $props();

  const importManager = getImportManagerContext();
  const viewerModel = getViewerModelContext();
  let directImportOutputPath = $state("");
  let directImportLoading = $state(false);
  let directImportError = $state<string | null>(null);
  let velocityImportLoading = $state(false);
  let velocityImportError = $state<string | null>(null);
  let outputPathRequestId = 0;

  const selectedProvider = $derived(importManager.currentProvider);
  const currentSourceRefs = $derived(importManager.currentSourceRefs);
  const currentSourceRef = $derived(importManager.currentSourceRef);
  const currentSession = $derived(importManager.session);
  const currentResult = $derived(importManager.currentResult);
  const currentRequirements = $derived(importManager.currentRequirements);
  const blockedRequirements = $derived(importManager.currentBlockedRequirements);
  const recentSources = $derived(importManager.currentRecentSources);
  const wellSourceRootPath = $derived(commonWellImportRoot(currentSourceRefs) ?? "");
  const projectRoot = $derived(viewerModel.projectRoot.trim());
  const activeStorePath = $derived(viewerModel.activeStorePath.trim());
  const groupedProviders = $derived.by(() => {
    const groups = new Map<string, ImportProviderDescriptor[]>();
    for (const provider of importManager.providers) {
      const existing = groups.get(provider.group) ?? [];
      existing.push(provider);
      groups.set(provider.group, existing);
    }
    return [...groups.entries()].map(([group, providers]) => ({
      group,
      label: providerGroupLabel(group),
      providers
    }));
  });
  const selectedWellBinding = $derived.by<ProjectAssetBindingInput | null>(() => {
    const selectedWellbore = viewerModel.selectedProjectWellboreInventoryItem;
    if (!selectedWellbore) {
      return null;
    }
    return {
      well_name: selectedWellbore.wellName,
      wellbore_name: selectedWellbore.wellboreName,
      operator_aliases: []
    };
  });
  const isSegySource = $derived(!!currentSourceRef && /\.(sgy|segy)$/i.test(currentSourceRef));
  const isDirectSeismicSource = $derived(
    !!currentSourceRef && /\.(mdio|zarr)$/i.test(currentSourceRef)
  );
  const isFocusedSegyFlow = $derived(
    selectedProvider?.providerId === "seismic_volume" && (!currentSourceRef || isSegySource)
  );
  const canShowDirectImport = $derived(
    selectedProvider?.providerId === "seismic_volume" && !!currentSourceRef && isDirectSeismicSource
  );
  const timeDepthProviderConfig = $derived.by(() =>
    mapTimeDepthProvider(selectedProvider?.providerId ?? null)
  );
  const sourceSelectionLabel = $derived.by(() => {
    if (!selectedProvider) {
      return "Choose sources";
    }
    if (currentSourceRefs.length > 0) {
      return `${currentSourceRefs.length} selected`;
    }
    switch (selectedProvider.selectionMode) {
      case "directory":
        return "Choose a folder";
      case "single_file":
        return "Choose one source";
      default:
        return "Choose one or more sources";
    }
  });

  function mapTimeDepthProvider(providerId: ImportProviderId | null): {
    assetKind: PreviewProjectWellTimeDepthAssetRequest["assetKind"];
    dialogTitle: string;
    pickerTitle: string;
  } | null {
    switch (providerId) {
      case "checkshot_vsp":
        return {
          assetKind: "checkshot_vsp_observation_set",
          dialogTitle: "Import Checkshot/VSP Observation Set",
          pickerTitle: "Import Checkshot/VSP Observation Set"
        };
      case "manual_picks":
        return {
          assetKind: "manual_time_depth_pick_set",
          dialogTitle: "Import Manual Time-Depth Picks",
          pickerTitle: "Import Manual Time-Depth Picks"
        };
      case "authored_model":
        return {
          assetKind: "well_time_depth_authored_model",
          dialogTitle: "Import Well Time-Depth Authored Model",
          pickerTitle: "Import Well Time-Depth Authored Model"
        };
      case "compiled_model":
        return {
          assetKind: "well_time_depth_model",
          dialogTitle: "Import Compiled Well Time-Depth Model",
          pickerTitle: "Import Compiled Well Time-Depth Model"
        };
      default:
        return null;
    }
  }

  $effect(() => {
    const providerId = selectedProvider?.providerId;
    const sourceRef = currentSourceRef;
    if (providerId !== "seismic_volume" || !sourceRef || !isDirectImportExtension(sourceRef)) {
      directImportOutputPath = "";
      directImportError = null;
      return;
    }

    const requestId = ++outputPathRequestId;
    directImportError = null;
    void (async () => {
      try {
        const suggestedPath = await defaultImportStorePath(sourceRef);
        if (requestId !== outputPathRequestId) {
          return;
        }
        directImportOutputPath = suggestedPath;
      } catch (error) {
        if (requestId !== outputPathRequestId) {
          return;
        }
        directImportOutputPath = "";
        directImportError =
          error instanceof Error ? error.message : "Failed to suggest a runtime store path.";
      }
    })();
  });

  function isDirectImportExtension(sourceRef: string): boolean {
    return /\.(mdio|zarr)$/i.test(sourceRef.trim());
  }

  function basename(path: string | null | undefined): string {
    const normalized = path?.trim().replace(/\\/g, "/") ?? "";
    if (!normalized) {
      return "";
    }
    return normalized.split("/").pop() ?? normalized;
  }

  async function chooseSourcesForProvider(providerId: ImportProviderId): Promise<void> {
    if (providerId === "seismic_volume") {
      const sourcePath = await pickImportSeismicFile();
      if (sourcePath) {
        await importManager.replaceSourceRefs([sourcePath]);
      }
      return;
    }
    if (providerId === "horizons") {
      const sourcePaths = await pickHorizonFiles();
      if (sourcePaths.length > 0) {
        await importManager.replaceSourceRefs(sourcePaths);
      }
      return;
    }
    if (providerId === "velocity_functions") {
      const sourcePath = await pickVelocityFunctionsFile();
      if (sourcePath) {
        await importManager.replaceSourceRefs([sourcePath]);
      }
      return;
    }
    if (providerId === "vendor_project") {
      const sourcePath = await pickVendorProjectFolder("Select Petrel Export Root");
      if (sourcePath) {
        await importManager.replaceSourceRefs([sourcePath]);
      }
      return;
    }

    const timeDepthConfig = mapTimeDepthProvider(providerId);
    if (timeDepthConfig) {
      const sourcePath = await pickWellTimeDepthJsonFile(timeDepthConfig.pickerTitle);
      if (sourcePath) {
        await importManager.replaceSourceRefs([sourcePath]);
      }
      return;
    }

    const sourcePaths = await pickWellImportFiles();
    if (sourcePaths.length > 0) {
      await importManager.replaceSourceRefs(sourcePaths);
    }
  }

  async function selectProvider(provider: ImportProviderDescriptor): Promise<void> {
    await importManager.selectProvider(provider.providerId);
  }

  async function useRecentSource(sourceRef: string): Promise<void> {
    await importManager.replaceSourceRefs([sourceRef]);
  }

  async function handleProviderResult(result: ImportManagerNormalizedResult): Promise<void> {
    await importManager.applyNormalizedResult(result);
  }

  async function runDirectSeismicImport(): Promise<void> {
    const sourcePath = currentSourceRef?.trim() ?? "";
    const outputStorePath = directImportOutputPath.trim();
    if (!sourcePath || !outputStorePath) {
      directImportError = "Both source path and runtime store path are required.";
      return;
    }

    directImportLoading = true;
    directImportError = null;
    try {
      const response = await importDataset(sourcePath, outputStorePath, false);
      await handleProviderResult({
        providerId: "seismic_volume",
        status: "commit_succeeded",
        outcome: "canonical_commit",
        canonicalAssets: [
          {
            kind: "runtime_store",
            id: response.dataset.descriptor.store_id,
            label: basename(response.dataset.store_path),
            detail: sourcePath
          }
        ],
        preservedSources: [],
        droppedItems: [],
        warnings: [],
        blockers: [],
        diagnostics: [],
        refreshScopes: ["workspace_registry"],
        activationEffects: [
          {
            kind: "open_runtime_store",
            storePath: response.dataset.store_path,
            sourcePath
          }
        ],
        requestActions: ["close_after_success"],
        providerDetail: {
          storePath: response.dataset.store_path
        }
      });
    } catch (error) {
      directImportError = error instanceof Error ? error.message : String(error);
      await handleProviderResult({
        providerId: "seismic_volume",
        status: "commit_failed",
        outcome: "commit_failed",
        canonicalAssets: [],
        preservedSources: [],
        droppedItems: [],
        warnings: [],
        blockers: [directImportError],
        diagnostics: [directImportError],
        refreshScopes: [],
        activationEffects: [],
        providerDetail: null
      });
    } finally {
      directImportLoading = false;
    }
  }

  async function runVelocityFunctionsImport(): Promise<void> {
    const sourcePath = currentSourceRef?.trim() ?? "";
    if (!sourcePath) {
      velocityImportError = "Choose a velocity-functions file before importing.";
      return;
    }
    if (!activeStorePath) {
      velocityImportError = "Open a seismic volume before importing velocity functions.";
      return;
    }

    velocityImportLoading = true;
    velocityImportError = null;
    try {
      const response = await importVelocityFunctionsModel(activeStorePath, sourcePath, "interval");
      await handleProviderResult({
        providerId: "velocity_functions",
        status: "commit_succeeded",
        outcome: "canonical_commit",
        canonicalAssets: [
          {
            kind: "velocity_model",
            id: response.model.id,
            label: response.model.name,
            detail: `${response.profile_count} profiles`
          }
        ],
        preservedSources: [],
        droppedItems: [],
        warnings: [],
        blockers: [],
        diagnostics: [],
        refreshScopes: ["velocity_models"],
        activationEffects: [{ kind: "activate_velocity_model", assetId: response.model.id }],
        requestActions: ["close_after_success"],
        providerDetail: {
          inputPath: response.input_path,
          profileCount: response.profile_count,
          sampleCount: response.sample_count
        }
      });
    } catch (error) {
      velocityImportError = error instanceof Error ? error.message : String(error);
      await handleProviderResult({
        providerId: "velocity_functions",
        status: "commit_failed",
        outcome: "commit_failed",
        canonicalAssets: [],
        preservedSources: [],
        droppedItems: [],
        warnings: [],
        blockers: [velocityImportError],
        diagnostics: [velocityImportError],
        refreshScopes: [],
        activationEffects: [],
        providerDetail: null
      });
    } finally {
      velocityImportLoading = false;
    }
  }

  function resultTone(result: ImportManagerNormalizedResult | null): "success" | "warning" | "error" {
    if (!result) {
      return "success";
    }
    if (result.status === "commit_failed") {
      return "error";
    }
    if (result.warnings.length > 0 || result.outcome === "partial_canonical_commit") {
      return "warning";
    }
    return "success";
  }
</script>

{#if importManager.open}
  <div class="import-manager-backdrop" role="presentation">
    <div
      class={["import-manager-dialog", importManager.dragDropActive && "drag-target-active"]}
      role="dialog"
      aria-modal="true"
      aria-labelledby="import-manager-title"
    >
      <header class="import-manager-header">
        <div>
          <h2 id="import-manager-title">Import Data</h2>
          <p>
            Import external data through one managed flow. Managed runtime stores still open through
            <strong>Open Volume...</strong>.
          </p>
        </div>
        <button type="button" class="ghost-button" onclick={() => importManager.closeManager()}>
          Close
        </button>
      </header>

      <div class={["import-manager-layout", isFocusedSegyFlow && "focused-segy-layout"]}>
        {#if !isFocusedSegyFlow}
          <aside class="provider-list" aria-label="Import providers">
            {#each groupedProviders as group (group.group)}
              <section class="provider-group">
                <h3>{group.label}</h3>
                <div class="provider-group-list">
                  {#each group.providers as provider (provider.providerId)}
                    <button
                      type="button"
                      class={["provider-button", importManager.activeProviderId === provider.providerId && "active"]}
                      onclick={() => void selectProvider(provider)}
                    >
                      <span class="provider-icon">{provider.iconId.replace(/_/g, " ")}</span>
                      <strong>{provider.label}</strong>
                      <span>{provider.description}</span>
                    </button>
                  {/each}
                </div>
              </section>
            {/each}
          </aside>
        {/if}

        <section class={["import-manager-main", isFocusedSegyFlow && "focused-segy-main"]}>
          {#if importManager.error}
            <div class="manager-status error">
              <strong>Import manager could not initialize.</strong>
              <p>{importManager.error}</p>
            </div>
          {/if}

          {#if importManager.contextNotice}
            <div class="manager-status">
              <strong>Context updated</strong>
              <p>{importManager.contextNotice}</p>
            </div>
          {/if}

          {#if selectedProvider && isFocusedSegyFlow}
            <section class="focused-segy-shell">
              <div class="focused-segy-toolbar">
                <div>
                  <h3>SEG-Y Survey Import</h3>
                  <p>Confirm the detected seismic-volume parameters and import the survey.</p>
                </div>
                <div class="panel-actions">
                  <button
                    type="button"
                    class="secondary-button"
                    onclick={() => void chooseSourcesForProvider("seismic_volume")}
                    disabled={importManager.loading}
                  >
                    {currentSourceRef ? "Change SEG-Y File" : "Choose SEG-Y File"}
                  </button>
                  {#if currentSourceRef}
                    <button
                      type="button"
                      class="secondary-button"
                      onclick={() => void importManager.clearSourceRefs()}
                      disabled={importManager.loading}
                    >
                      Clear
                    </button>
                  {/if}
                </div>
              </div>

              {#if importManager.loading}
                <div class="manager-status">
                  <strong>Preparing import session...</strong>
                </div>
              {:else if !currentSourceRef}
                <div class="provider-empty-state focused">
                  <strong>Choose a SEG-Y file to begin.</strong>
                  <p>The import dialog will scan the file and ask only for the seismic parameters that matter.</p>
                </div>
              {:else}
                <SegyImportDialog
                  open={true}
                  inputPath={currentSourceRef}
                  {viewerModel}
                  onClose={() => importManager.closeManager()}
                  onCommitResult={handleProviderResult}
                  embedded={true}
                />
              {/if}
            </section>
          {:else if selectedProvider}
            <section class="provider-summary-card">
              <div>
                <span class="summary-label">Provider</span>
                <strong>{selectedProvider.label}</strong>
              </div>
              <div>
                <span class="summary-label">Group</span>
                <strong>{providerGroupLabel(selectedProvider.group)}</strong>
              </div>
              <div>
                <span class="summary-label">Destination</span>
                <strong>{selectedProvider.destinationKind.replace(/_/g, " ")}</strong>
              </div>
              <div>
                <span class="summary-label">Source Selection</span>
                <strong>{sourceSelectionLabel}</strong>
              </div>
            </section>

            <section class="context-grid">
              <div class="context-card">
                <span class="summary-label">Active Store</span>
                <strong>{activeStorePath || "Not open"}</strong>
              </div>
              <div class="context-card">
                <span class="summary-label">Project Root</span>
                <strong>{projectRoot || "Not configured"}</strong>
              </div>
              <div class="context-card">
                <span class="summary-label">Selected Wellbore</span>
                <strong>{importManager.contextSnapshot.selectedWellboreLabel || "Not selected"}</strong>
              </div>
            </section>

            <section class="source-panel">
              <div class="section-header">
                <div>
                  <h3>Sources</h3>
                  <p>Choose, replace, or drop external sources for the current provider.</p>
                </div>
                <div class="provider-summary-actions">
                  <button
                    type="button"
                    class="secondary-button"
                    onclick={() => void chooseSourcesForProvider(selectedProvider.providerId)}
                    disabled={importManager.loading}
                  >
                    {currentSourceRefs.length > 0 ? "Change Sources" : "Choose Sources"}
                  </button>
                  {#if currentSourceRefs.length > 0}
                    <button
                      type="button"
                      class="secondary-button"
                      onclick={() => void importManager.clearSourceRefs()}
                      disabled={importManager.loading}
                    >
                      Clear
                    </button>
                  {/if}
                </div>
              </div>

              <div class="source-list">
                {#if currentSourceRefs.length > 0}
                  {#each currentSourceRefs as sourceRef (sourceRef)}
                    <div class="source-chip">
                      <strong>{sourceLabelFromPath(sourceRef)}</strong>
                      <span>{sourceRef}</span>
                    </div>
                  {/each}
                {:else}
                  <div class="provider-empty-state compact">
                    <strong>No sources selected.</strong>
                    <p>Pick files or folders, or drop them anywhere into this manager.</p>
                  </div>
                {/if}
              </div>

              {#if recentSources.length > 0}
                <div class="recent-sources">
                  <span class="summary-label">Recent Sources</span>
                  <div class="recent-source-list">
                    {#each recentSources as sourceRef (sourceRef)}
                      <button
                        type="button"
                        class="recent-source-button"
                        onclick={() => void useRecentSource(sourceRef)}
                      >
                        <strong>{sourceLabelFromPath(sourceRef)}</strong>
                        <span>{sourceRef}</span>
                      </button>
                    {/each}
                  </div>
                </div>
              {/if}

              <div class="drop-hint">
                <strong>{importManager.dragDropActive ? "Drop to import" : "Drag and drop supported"}</strong>
                <p>
                  External files route into the import manager. Managed <code>.tbvol</code> stores still open directly.
                </p>
              </div>
            </section>

            {#if currentRequirements.length > 0}
              <section class="requirements-card">
                <div class="section-header">
                  <div>
                    <h3>Requirements</h3>
                    <p>Provider availability is driven from the backend registry contract.</p>
                  </div>
                </div>
                <div class="requirement-list">
                  {#each currentRequirements as requirement (requirement.key)}
                    <div class={["requirement-item", requirement.satisfied ? "ok" : "blocked"]}>
                      <strong>{requirement.satisfied ? "Ready" : "Action needed"}</strong>
                      <span>{requirement.message}</span>
                    </div>
                  {/each}
                </div>
              </section>
            {/if}

            {#if currentSession?.diagnostics.length}
              <div class="manager-status">
                <strong>Session</strong>
                <ul>
                  {#each currentSession.diagnostics as diagnostic (`session-diagnostic:${diagnostic.level}:${diagnostic.message}`)}
                    <li>{diagnostic.message}</li>
                  {/each}
                </ul>
              </div>
            {/if}

            {#if currentResult}
              <section class={["result-card", resultTone(currentResult)]}>
                <div class="section-header">
                  <div>
                    <h3>Last Result</h3>
                    <p>{currentResult.outcome.replace(/_/g, " ")}</p>
                  </div>
                  {#if importManager.applyingEffects}
                    <strong>Applying refresh and activation...</strong>
                  {/if}
                </div>

                {#if currentResult.canonicalAssets.length > 0}
                  <div class="result-list">
                    {#each currentResult.canonicalAssets as asset (`result-asset:${asset.kind}:${asset.id ?? asset.label}`)}
                      <div class="result-row">
                        <strong>{asset.label}</strong>
                        <span>{asset.kind}{asset.detail ? ` | ${asset.detail}` : ""}</span>
                      </div>
                    {/each}
                  </div>
                {/if}

                {#if currentResult.preservedSources.length > 0}
                  <div class="result-list">
                    {#each currentResult.preservedSources as source (`result-source:${source.kind}:${source.label}:${source.sourceRef ?? ""}`)}
                      <div class="result-row">
                        <strong>{source.label}</strong>
                        <span>{source.detail ?? source.sourceRef ?? source.kind}</span>
                      </div>
                    {/each}
                  </div>
                {/if}

                {#if currentResult.warnings.length > 0 || currentResult.blockers.length > 0}
                  <ul class="result-messages">
                    {#each [...currentResult.blockers, ...currentResult.warnings] as message (`result-message:${message}`)}
                      <li>{message}</li>
                    {/each}
                  </ul>
                {/if}
              </section>
            {/if}

            {#if importManager.loading}
              <div class="manager-status">
                <strong>Preparing import session...</strong>
              </div>
            {:else if selectedProvider.providerId === "seismic_volume"}
              {#if !currentSourceRef}
                <div class="provider-empty-state">
                  <strong>Choose a seismic source to begin.</strong>
                  <p>SEG-Y uses the full review flow. Zarr and MDIO use a direct runtime-store import path inside this manager.</p>
                </div>
              {:else if canShowDirectImport}
                <div class="provider-panel">
                  <div class="detail-block">
                    <strong>Direct seismic import</strong>
                    <p>
                      <code>{basename(currentSourceRef)}</code> can import directly into a managed runtime store.
                    </p>
                  </div>

                  <label class="field">
                    <span>Source Path</span>
                    <input type="text" value={currentSourceRef} readonly />
                  </label>

                  <label class="field">
                    <span>Runtime Store Path</span>
                    <input bind:value={directImportOutputPath} type="text" />
                  </label>

                  {#if directImportError}
                    <div class="manager-status error">
                      <strong>Direct import failed</strong>
                      <p>{directImportError}</p>
                    </div>
                  {/if}

                  <div class="panel-actions">
                    <button
                      type="button"
                      class="secondary-button"
                      onclick={() => void chooseSourcesForProvider("seismic_volume")}
                      disabled={directImportLoading}
                    >
                      Change Source
                    </button>
                    <button
                      type="button"
                      class="primary-button"
                      onclick={() => void runDirectSeismicImport()}
                      disabled={directImportLoading || directImportOutputPath.trim().length === 0}
                    >
                      {directImportLoading ? "Importing..." : "Import Runtime Store"}
                    </button>
                  </div>
                </div>
              {:else}
                <div class="manager-status error">
                  <strong>Unsupported seismic source</strong>
                  <p>The selected source does not match the current seismic-volume import surface.</p>
                </div>
              {/if}
            {:else if selectedProvider.providerId === "horizons"}
              {#if currentSourceRefs.length === 0}
                <div class="provider-empty-state">
                  <strong>Choose one or more horizon XYZ files to begin.</strong>
                  <p>The manager will parse them immediately and reuse the existing horizon review flow in embedded mode.</p>
                </div>
              {:else}
                <HorizonImportDialog
                  inputPaths={currentSourceRefs}
                  close={() => importManager.closeManager()}
                  onCommitResult={handleProviderResult}
                  embedded={true}
                />
              {/if}
            {:else if selectedProvider.providerId === "well_sources"}
              {#if currentSourceRefs.length === 0}
                <div class="provider-empty-state">
                  <strong>Choose well source files to begin.</strong>
                  <p>The manager reuses the existing canonical draft and review flow for well sources.</p>
                </div>
              {:else}
                <WellSourceImportDialog
                  sourceRootPath={wellSourceRootPath}
                  sourcePaths={currentSourceRefs}
                  close={() => importManager.closeManager()}
                  onCommitResult={handleProviderResult}
                  embedded={true}
                />
              {/if}
            {:else if selectedProvider.providerId === "velocity_functions"}
              {#if blockedRequirements.length > 0}
                <div class="manager-status error">
                  <strong>Velocity-functions import is not ready.</strong>
                  <p>{blockedRequirements[0]?.message}</p>
                </div>
              {:else if !currentSourceRef}
                <div class="provider-empty-state">
                  <strong>Choose a velocity-functions file to begin.</strong>
                  <p>The imported profiles will be compiled into a survey velocity model for the active seismic volume.</p>
                </div>
              {:else}
                <div class="provider-panel">
                  <div class="detail-block">
                    <strong>Velocity functions import</strong>
                    <p>Import <code>{basename(currentSourceRef)}</code> into <code>{basename(activeStorePath)}</code> and activate the compiled velocity model.</p>
                  </div>

                  <label class="field">
                    <span>Source File</span>
                    <input type="text" value={currentSourceRef} readonly />
                  </label>

                  <label class="field">
                    <span>Active Seismic Volume</span>
                    <input type="text" value={activeStorePath} readonly />
                  </label>

                  {#if velocityImportError}
                    <div class="manager-status error">
                      <strong>Velocity-functions import failed</strong>
                      <p>{velocityImportError}</p>
                    </div>
                  {/if}

                  <div class="panel-actions">
                    <button
                      type="button"
                      class="secondary-button"
                      onclick={() => void chooseSourcesForProvider("velocity_functions")}
                      disabled={velocityImportLoading}
                    >
                      Change Source
                    </button>
                    <button
                      type="button"
                      class="primary-button"
                      onclick={() => void runVelocityFunctionsImport()}
                      disabled={velocityImportLoading}
                    >
                      {velocityImportLoading ? "Importing..." : "Import Velocity Functions"}
                    </button>
                  </div>
                </div>
              {/if}
            {:else if timeDepthProviderConfig}
              {#if !projectRoot || !selectedWellBinding || viewerModel.projectWellAssetImportBlocker}
                <div class="manager-status error">
                  <strong>Project well import setup is incomplete.</strong>
                  <p>
                    {viewerModel.projectWellAssetImportBlocker ??
                      "Set the project root and select a project wellbore before importing well time-depth assets."}
                  </p>
                  <div class="panel-actions">
                    <button type="button" class="secondary-button" onclick={openSettings}>
                      Project Settings...
                    </button>
                  </div>
                </div>
              {:else if !currentSourceRef}
                <div class="provider-empty-state">
                  <strong>Choose a well time-depth JSON file to begin.</strong>
                  <p>The manager reuses the existing preview, draft, and commit flow for this asset family.</p>
                </div>
              {:else}
                <WellTimeDepthImportDialog
                  {projectRoot}
                  jsonPath={currentSourceRef}
                  binding={selectedWellBinding}
                  assetKind={timeDepthProviderConfig.assetKind}
                  dialogTitle={timeDepthProviderConfig.dialogTitle}
                  {openSettings}
                  close={() => importManager.closeManager()}
                  onCommitResult={handleProviderResult}
                  embedded={true}
                />
              {/if}
            {:else if selectedProvider.providerId === "vendor_project"}
              {#if !viewerModel.tauriRuntime}
                <div class="manager-status error">
                  <strong>Vendor-project import is desktop-only.</strong>
                  <p>Use the TraceBoost desktop runtime to scan and import Petrel export bundles.</p>
                </div>
              {:else}
                <VendorProjectImportDialog
                  {openSettings}
                  close={() => importManager.closeManager()}
                  onCommitResult={handleProviderResult}
                  embedded={true}
                  initialProjectRoot={currentSourceRef}
                />
              {/if}
            {/if}
          {:else}
            <div class="provider-empty-state">
              <strong>Select an import provider.</strong>
              <p>Choose the kind of external data you want to bring into TraceBoost.</p>
              {#if importManager.lastProviderMatch.ambiguous}
                <p>The dropped selection matched multiple providers. Pick one explicitly to continue.</p>
              {/if}
            </div>
          {/if}
        </section>
      </div>
    </div>
  </div>
{/if}

<style>
  .import-manager-backdrop {
    position: fixed;
    inset: 0;
    z-index: 85;
    display: flex;
    align-items: center;
    justify-content: center;
    padding: 20px;
    background: rgba(38, 55, 71, 0.2);
    backdrop-filter: blur(4px);
  }

  .import-manager-dialog {
    width: min(1380px, calc(100vw - 32px));
    height: min(920px, calc(100vh - 32px));
    display: grid;
    grid-template-rows: auto 1fr;
    border: 1px solid var(--app-border);
    border-radius: var(--ui-radius-lg);
    overflow: hidden;
    background: var(--panel-bg);
    color: var(--text-primary);
    box-shadow: var(--ui-shadow-dialog);
  }

  .import-manager-dialog.drag-target-active {
    border-color: var(--accent-border);
    box-shadow: 0 0 0 2px rgba(69, 120, 165, 0.28), var(--ui-shadow-dialog);
  }

  .import-manager-header {
    display: flex;
    align-items: flex-start;
    justify-content: space-between;
    gap: 16px;
    padding: 18px 20px;
    border-bottom: 1px solid var(--app-border);
  }

  .import-manager-header h2,
  .import-manager-header p,
  .section-header h3,
  .section-header p {
    margin: 0;
  }

  .import-manager-header p,
  .section-header p {
    margin-top: 6px;
    color: var(--text-muted);
  }

  .import-manager-layout {
    min-height: 0;
    display: grid;
    grid-template-columns: 320px 1fr;
  }

  .import-manager-layout.focused-segy-layout {
    grid-template-columns: 1fr;
  }

  .provider-list {
    min-height: 0;
    overflow: auto;
    display: grid;
    align-content: start;
    gap: 18px;
    padding: 18px;
    border-right: 1px solid var(--app-border);
    background: var(--surface-subtle);
  }

  .provider-group {
    display: grid;
    gap: 10px;
  }

  .provider-group h3 {
    margin: 0;
    font-size: 0.88rem;
    text-transform: uppercase;
    letter-spacing: 0.08em;
    color: var(--text-dim);
  }

  .provider-group-list {
    display: grid;
    gap: 10px;
  }

  .provider-button {
    display: grid;
    gap: 6px;
    padding: 14px;
    border: 1px solid var(--app-border);
    border-radius: var(--ui-radius-lg);
    background: #fff;
    color: inherit;
    text-align: left;
    font: inherit;
  }

  .provider-button.active {
    border-color: var(--accent-border);
    background: var(--accent-bg);
  }

  .provider-button span {
    color: var(--text-muted);
    font-size: 0.94rem;
    line-height: 1.4;
  }

  .provider-icon {
    display: inline-block;
    color: var(--text-dim);
    font-size: 0.75rem;
    text-transform: uppercase;
    letter-spacing: 0.08em;
  }

  .import-manager-main {
    min-height: 0;
    overflow: auto;
    display: grid;
    align-content: start;
    gap: 16px;
    padding: 18px;
  }

  .import-manager-main.focused-segy-main {
    padding: 18px;
    background: var(--surface-subtle);
  }

  .provider-summary-card,
  .context-grid,
  .source-panel,
  .requirements-card,
  .result-card,
  .manager-status,
  .provider-empty-state,
  .provider-panel,
  .detail-block,
  .focused-segy-shell {
    display: grid;
    gap: 10px;
    padding: 16px;
    border: 1px solid var(--app-border);
    border-radius: var(--ui-radius-lg);
    background: var(--surface-bg);
  }

  .focused-segy-shell {
    gap: 16px;
    padding: 18px;
  }

  .focused-segy-toolbar {
    display: flex;
    align-items: flex-start;
    justify-content: space-between;
    gap: 12px;
  }

  .focused-segy-toolbar h3,
  .focused-segy-toolbar p {
    margin: 0;
  }

  .focused-segy-toolbar p {
    margin-top: 6px;
    color: var(--text-muted);
  }

  .provider-summary-card,
  .context-grid {
    grid-template-columns: repeat(auto-fit, minmax(180px, 1fr));
    align-items: end;
  }

  .summary-label {
    display: block;
    margin-bottom: 4px;
    color: var(--text-dim);
    font-size: 0.82rem;
    text-transform: uppercase;
    letter-spacing: 0.06em;
  }

  .section-header,
  .provider-summary-actions,
  .panel-actions {
    display: flex;
    flex-wrap: wrap;
    align-items: flex-start;
    justify-content: space-between;
    gap: 10px;
  }

  .source-list,
  .recent-source-list,
  .result-list,
  .requirement-list {
    display: grid;
    gap: 10px;
  }

  .source-chip,
  .result-row,
  .requirement-item {
    display: grid;
    gap: 4px;
    padding: 12px;
    border: 1px solid var(--app-border);
    border-radius: var(--ui-radius-lg);
    background: #fff;
  }

  .source-chip span,
  .result-row span,
  .requirement-item span,
  .drop-hint p,
  .provider-empty-state p,
  .detail-block p {
    color: var(--text-muted);
    margin: 0;
  }

  .recent-source-button {
    display: grid;
    gap: 4px;
    padding: 12px;
    border: 1px solid var(--app-border);
    border-radius: var(--ui-radius-lg);
    background: #fff;
    color: inherit;
    text-align: left;
    font: inherit;
  }

  .requirement-item.ok {
    border-color: rgba(46, 174, 107, 0.35);
  }

  .requirement-item.blocked,
  .manager-status.error,
  .result-card.error {
    border-color: var(--danger-border);
    background: var(--danger-bg);
  }

  .result-card.warning {
    border-color: var(--warn-border);
    background: var(--warn-bg);
  }

  .result-card.success {
    border-color: rgba(46, 174, 107, 0.35);
    background: rgba(46, 174, 107, 0.08);
  }

  .result-messages,
  .manager-status ul {
    margin: 0;
    padding-left: 18px;
    display: grid;
    gap: 6px;
  }

  .drop-hint {
    display: grid;
    gap: 6px;
    padding: 12px;
    border: 1px dashed var(--app-border-strong);
    border-radius: var(--ui-radius-lg);
    background: var(--surface-subtle);
  }

  .provider-empty-state.compact {
    padding: 12px;
  }

  .provider-empty-state.focused {
    min-height: 220px;
    align-content: center;
  }

  .field {
    display: grid;
    gap: 6px;
  }

  .field input {
    min-height: 38px;
    border: 1px solid var(--app-border-strong);
    border-radius: var(--ui-radius-lg);
    padding: 8px 10px;
    background: #fff;
    color: var(--text-primary);
  }

  .primary-button,
  .secondary-button,
  .ghost-button {
    min-height: 38px;
    padding: 0 14px;
    border-radius: var(--ui-radius-lg);
    font: inherit;
    cursor: pointer;
  }

  .primary-button {
    border: 1px solid var(--accent-text);
    background: var(--accent-text);
    color: #fff;
  }

  .secondary-button,
  .ghost-button {
    border: 1px solid var(--app-border-strong);
    background: var(--surface-subtle);
    color: var(--text-primary);
  }

  @media (max-width: 1080px) {
    .import-manager-layout {
      grid-template-columns: 1fr;
    }

    .provider-list {
      border-right: 0;
      border-bottom: 1px solid var(--app-border);
    }
  }

  @media (max-width: 760px) {
    .focused-segy-toolbar {
      flex-direction: column;
      align-items: stretch;
    }
  }
</style>
