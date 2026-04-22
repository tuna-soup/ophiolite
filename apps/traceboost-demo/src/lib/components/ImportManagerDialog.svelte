<script lang="ts">
  import SegyImportDialog from "./SegyImportDialog.svelte";
  import HorizonImportDialog from "./HorizonImportDialog.svelte";
  import WellSourceImportDialog from "./WellFolderImportDialog.svelte";
  import WellTimeDepthImportDialog from "./WellTimeDepthImportDialog.svelte";
  import VendorProjectImportDialog from "./VendorProjectImportDialog.svelte";
  import { getImportManagerContext, commonWellImportRoot } from "../import-manager-model.svelte";
  import { getViewerModelContext } from "../viewer-model.svelte";
  import {
    defaultImportStorePath,
    type PreviewProjectWellTimeDepthAssetRequest,
    type ProjectAssetBindingInput,
    type ImportProviderDescriptor,
    type ImportProviderId
  } from "../bridge";
  import {
    pickHorizonFiles,
    pickImportSeismicFile,
    pickVendorProjectFolder,
    pickVelocityFunctionsFile,
    pickWellImportFiles
    ,
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
  const wellSourceRootPath = $derived(commonWellImportRoot(currentSourceRefs) ?? "");
  const projectRoot = $derived(viewerModel.projectRoot.trim());
  const activeStorePath = $derived(viewerModel.activeStorePath.trim());
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
  const isSegySource = $derived(
    !!currentSourceRef && /\.(sgy|segy)$/i.test(currentSourceRef)
  );
  const isDirectSeismicSource = $derived(
    !!currentSourceRef && /\.(mdio|zarr)$/i.test(currentSourceRef)
  );
  const canShowDirectImport = $derived(
    selectedProvider?.providerId === "seismic_volume" && !!currentSourceRef && isDirectSeismicSource
  );
  const timeDepthProviderConfig = $derived.by(() => mapTimeDepthProvider(selectedProvider?.providerId ?? null));
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
      if (!sourcePath) {
        return;
      }
      await importManager.replaceSourceRefs([sourcePath]);
      return;
    }
    if (providerId === "horizons") {
      const sourcePaths = await pickHorizonFiles();
      if (sourcePaths.length === 0) {
        return;
      }
      await importManager.replaceSourceRefs(sourcePaths);
      return;
    }
    if (providerId === "velocity_functions") {
      const sourcePath = await pickVelocityFunctionsFile();
      if (!sourcePath) {
        return;
      }
      await importManager.replaceSourceRefs([sourcePath]);
      return;
    }
    if (providerId === "vendor_project") {
      const sourcePath = await pickVendorProjectFolder("Select Petrel Export Root");
      if (!sourcePath) {
        return;
      }
      await importManager.replaceSourceRefs([sourcePath]);
      return;
    }
    const timeDepthConfig = mapTimeDepthProvider(providerId);
    if (timeDepthConfig) {
      const sourcePath = await pickWellTimeDepthJsonFile(timeDepthConfig.pickerTitle);
      if (!sourcePath) {
        return;
      }
      await importManager.replaceSourceRefs([sourcePath]);
      return;
    }

    const sourcePaths = await pickWellImportFiles();
    if (sourcePaths.length === 0) {
      return;
    }
    await importManager.replaceSourceRefs(sourcePaths);
  }

  async function selectProvider(provider: ImportProviderDescriptor): Promise<void> {
    await importManager.selectProvider(provider.providerId);
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
      await viewerModel.importDataset({
        inputPath: sourcePath,
        outputStorePath,
        sourcePath,
        makeActive: true,
        loadSection: true,
        reuseExistingStore: true
      });
      if (!viewerModel.error) {
        importManager.closeManager();
      }
    } catch (error) {
      directImportError = error instanceof Error ? error.message : String(error);
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
      await viewerModel.importVelocityFunctionsFile(sourcePath, "interval");
      if (!viewerModel.velocityModelsError) {
        importManager.closeManager();
      }
    } finally {
      velocityImportLoading = false;
      velocityImportError = viewerModel.velocityModelsError;
    }
  }
</script>

{#if importManager.open}
  <div class="import-manager-backdrop" role="presentation">
    <div
      class="import-manager-dialog"
      role="dialog"
      aria-modal="true"
      aria-labelledby="import-manager-title"
    >
      <header class="import-manager-header">
        <div>
          <h2 id="import-manager-title">Import Data</h2>
          <p>
            Use one centralized import flow for external data. Managed runtime stores still open
            through <strong>Open Volume...</strong>.
          </p>
        </div>
        <button type="button" class="ghost-button" onclick={() => importManager.closeManager()}>
          Close
        </button>
      </header>

      <div class="import-manager-layout">
        <aside class="provider-list" aria-label="Import providers">
          {#each importManager.providers as provider (provider.providerId)}
            <button
              type="button"
              class={["provider-button", importManager.activeProviderId === provider.providerId && "active"]}
              onclick={() => void selectProvider(provider)}
            >
              <strong>{provider.label}</strong>
              <span>{provider.description}</span>
            </button>
          {/each}
        </aside>

        <section class="import-manager-main">
          {#if importManager.error}
            <div class="manager-status error">
              <strong>Import manager could not initialize.</strong>
              <p>{importManager.error}</p>
            </div>
          {/if}

          {#if selectedProvider}
            <section class="provider-summary-card">
              <div>
                <span class="summary-label">Provider</span>
                <strong>{selectedProvider.label}</strong>
              </div>
              <div>
                <span class="summary-label">Destination</span>
                <strong>{selectedProvider.destinationKind}</strong>
              </div>
              <div>
                <span class="summary-label">Source Selection</span>
                <strong>{sourceSelectionLabel}</strong>
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
            </section>

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
              {:else if isSegySource}
                <SegyImportDialog
                  open={true}
                  inputPath={currentSourceRef}
                  {viewerModel}
                  onClose={() => importManager.closeManager()}
                  embedded={true}
                />
              {:else if canShowDirectImport}
                <div class="provider-panel">
                  <div class="detail-block">
                    <strong>Direct seismic import</strong>
                    <p>
                      <code>{basename(currentSourceRef)}</code> can import directly into a managed
                      runtime store. This route is centralized here instead of being hidden behind
                      <strong>Open Volume...</strong>.
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
                  <p>
                    The selected source does not match the current seismic-volume import surface.
                  </p>
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
                  embedded={true}
                />
              {/if}
            {:else if selectedProvider.providerId === "well_sources"}
              {#if currentSourceRefs.length === 0}
                <div class="provider-empty-state">
                  <strong>Choose well source files to begin.</strong>
                  <p>The manager will reuse the existing canonical draft and review flow for well sources in embedded mode.</p>
                </div>
              {:else}
                <WellSourceImportDialog
                  sourceRootPath={wellSourceRootPath}
                  sourcePaths={currentSourceRefs}
                  close={() => importManager.closeManager()}
                  embedded={true}
                />
              {/if}
            {:else if selectedProvider.providerId === "velocity_functions"}
              {#if !activeStorePath}
                <div class="manager-status error">
                  <strong>Open a seismic volume first.</strong>
                  <p>Velocity-functions import needs an active seismic volume so the compiled model has a survey target.</p>
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
                  <p>The manager will reuse the existing preview, draft, and commit flow for this asset family in embedded mode.</p>
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
                  embedded={true}
                  initialProjectRoot={currentSourceRef}
                />
              {/if}
            {/if}
          {:else}
            <div class="provider-empty-state">
              <strong>Select an import provider.</strong>
              <p>Use the provider list to choose which kind of external data you want to bring into TraceBoost.</p>
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
    background: rgb(5 10 16 / 0.62);
  }

  .import-manager-dialog {
    width: min(1320px, calc(100vw - 32px));
    height: min(900px, calc(100vh - 32px));
    display: grid;
    grid-template-rows: auto 1fr;
    border: 1px solid rgb(255 255 255 / 0.1);
    border-radius: 12px;
    overflow: hidden;
    background: #0f141b;
    color: var(--text-primary);
    box-shadow: 0 24px 70px rgb(0 0 0 / 0.4);
  }

  .import-manager-header {
    display: flex;
    align-items: flex-start;
    justify-content: space-between;
    gap: 16px;
    padding: 18px 20px;
    border-bottom: 1px solid rgb(255 255 255 / 0.08);
  }

  .import-manager-header h2,
  .import-manager-header p {
    margin: 0;
  }

  .import-manager-header p {
    margin-top: 6px;
    max-width: 760px;
    color: rgb(255 255 255 / 0.72);
  }

  .import-manager-layout {
    min-height: 0;
    display: grid;
    grid-template-columns: 280px 1fr;
  }

  .provider-list {
    min-height: 0;
    overflow: auto;
    display: grid;
    align-content: start;
    gap: 10px;
    padding: 18px;
    border-right: 1px solid rgb(255 255 255 / 0.08);
    background: rgb(255 255 255 / 0.02);
  }

  .provider-button {
    display: grid;
    gap: 6px;
    padding: 14px;
    border: 1px solid rgb(255 255 255 / 0.1);
    border-radius: 10px;
    background: rgb(255 255 255 / 0.02);
    color: inherit;
    text-align: left;
    font: inherit;
  }

  .provider-button.active {
    border-color: rgb(96 165 250 / 0.8);
    background: rgb(59 130 246 / 0.14);
  }

  .provider-button span {
    color: rgb(255 255 255 / 0.72);
    font-size: 0.94rem;
    line-height: 1.4;
  }

  .import-manager-main {
    min-height: 0;
    overflow: auto;
    display: grid;
    align-content: start;
    gap: 16px;
    padding: 18px;
  }

  .provider-summary-card,
  .manager-status,
  .provider-empty-state,
  .provider-panel,
  .detail-block {
    display: grid;
    gap: 10px;
    padding: 16px;
    border: 1px solid rgb(255 255 255 / 0.08);
    border-radius: 10px;
    background: rgb(255 255 255 / 0.02);
  }

  .provider-summary-card {
    grid-template-columns: repeat(auto-fit, minmax(180px, 1fr));
    align-items: end;
  }

  .summary-label {
    display: block;
    margin-bottom: 4px;
    color: rgb(255 255 255 / 0.62);
    font-size: 0.82rem;
    text-transform: uppercase;
    letter-spacing: 0.06em;
  }

  .provider-summary-actions,
  .panel-actions {
    display: flex;
    flex-wrap: wrap;
    gap: 10px;
  }

  .manager-status.error {
    border-color: rgb(248 113 113 / 0.6);
    background: rgb(127 29 29 / 0.18);
  }

  .manager-status ul {
    margin: 0;
    padding-left: 18px;
    display: grid;
    gap: 6px;
  }

  .provider-empty-state p,
  .detail-block p {
    margin: 0;
    color: rgb(255 255 255 / 0.72);
  }

  .field {
    display: grid;
    gap: 6px;
  }

  .field span {
    color: rgb(255 255 255 / 0.72);
    font-size: 0.92rem;
  }

  .field input {
    width: 100%;
    padding: 10px 12px;
    border: 1px solid rgb(255 255 255 / 0.12);
    border-radius: 8px;
    background: rgb(9 12 18 / 0.8);
    color: inherit;
    font: inherit;
  }

  .ghost-button,
  .secondary-button,
  .primary-button {
    padding: 10px 14px;
    border-radius: 8px;
    font: inherit;
    color: inherit;
  }

  .ghost-button,
  .secondary-button {
    border: 1px solid rgb(255 255 255 / 0.14);
    background: rgb(255 255 255 / 0.03);
  }

  .primary-button {
    border: 1px solid rgb(96 165 250 / 0.8);
    background: rgb(37 99 235 / 0.9);
    color: #fff;
  }

  @media (max-width: 980px) {
    .import-manager-dialog {
      width: min(100vw - 20px, 100%);
      height: min(100vh - 20px, 100%);
    }

    .import-manager-layout {
      grid-template-columns: 1fr;
      grid-template-rows: auto 1fr;
    }

    .provider-list {
      grid-auto-flow: column;
      grid-auto-columns: minmax(220px, 1fr);
      overflow: auto;
      border-right: none;
      border-bottom: 1px solid rgb(255 255 255 / 0.08);
    }
  }
</style>
