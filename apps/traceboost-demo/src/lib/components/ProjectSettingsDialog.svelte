<svelte:options runes={true} />

<script lang="ts">
  import { pickProjectFolder } from "../file-dialog";
  import { getViewerModelContext } from "../viewer-model.svelte";

  interface Props {
    open: boolean;
    close: () => void;
  }

  let { open, close }: Props = $props();

  const viewerModel = getViewerModelContext();
  const activeSurveyMapSurvey = $derived(viewerModel.surveyMapSource?.surveys[0] ?? null);
  const selectedProjectSurvey = $derived(viewerModel.selectedProjectSurveyAsset);
  const selectedProjectSurveyCompatibility = $derived(
    viewerModel.selectedProjectSurveyDisplayCompatibility
  );
  const selectedProjectSurveyCompatibilityMessage = $derived(
    viewerModel.selectedProjectSurveyDisplayCompatibilityMessage
  );
  const showSelectedProjectSurveyCompatibilityMessage = $derived(
    !!selectedProjectSurveyCompatibilityMessage &&
      !!selectedProjectSurveyCompatibility &&
      (!selectedProjectSurveyCompatibility.canResolveProjectMap ||
        selectedProjectSurveyCompatibility.transformStatus === "display_degraded")
  );
  const selectedProjectWellboreCompatibility = $derived(
    viewerModel.selectedProjectWellboreDisplayCompatibility
  );
  const selectedProjectWellboreCompatibilityMessage = $derived(
    viewerModel.selectedProjectWellboreDisplayCompatibilityMessage
  );
  const showSelectedProjectWellboreCompatibilityMessage = $derived(
    !!selectedProjectWellboreCompatibilityMessage &&
      !!selectedProjectWellboreCompatibility &&
      (!selectedProjectWellboreCompatibility.canResolveProjectMap ||
        selectedProjectWellboreCompatibility.transformStatus === "display_degraded")
  );

  function transformStatusLabel(status: string | null | undefined): string {
    switch (status) {
      case "display_equivalent":
        return "Display CRS matches native";
      case "display_transformed":
        return "Display transform active";
      case "display_degraded":
        return "Degraded display transform";
      case "display_unavailable":
        return "Display transform unavailable";
      case "native_only":
        return "Native coordinates only";
      default:
        return "No map transform";
    }
  }

  async function handlePickProjectRoot(): Promise<void> {
    const projectRoot = await pickProjectFolder();
    if (!projectRoot) {
      return;
    }
    await viewerModel.setProjectRoot(projectRoot);
  }

  async function handleResolveProjectWellOverlays(): Promise<void> {
    try {
      await viewerModel.resolveConfiguredProjectSectionWellOverlays();
    } catch (error) {
      viewerModel.note(
        "Failed to resolve configured project well overlays.",
        "backend",
        "warn",
        error instanceof Error ? error.message : String(error)
      );
    }
  }

  function handleBackdropClick(event: MouseEvent): void {
    if (event.target === event.currentTarget) {
      close();
    }
  }
</script>

<svelte:window
  onkeydown={(event) => {
    if (open && event.key === "Escape") {
      close();
    }
  }}
/>

{#if open}
  <div class="settings-backdrop" role="presentation" onclick={handleBackdropClick}>
    <div class="settings-dialog" role="dialog" aria-modal="true" aria-label="Project settings">
      <header class="settings-header">
        <div>
          <h2>Project Settings</h2>
          <p>Coordinate systems, project wells, and section overlays.</p>
        </div>
        <button class="icon-button" type="button" onclick={close} aria-label="Close settings">
          Close
        </button>
      </header>

      <div class="settings-grid">
        <section class="settings-section">
          <div class="section-heading">
            <h3>Coordinate Systems</h3>
            <p>Display and native CRS for the active seismic survey.</p>
          </div>

          <label class="field">
            <span>Display Mode</span>
            <select
              bind:value={viewerModel.projectDisplayCoordinateReferenceMode}
              disabled={viewerModel.projectGeospatialSettingsSaving}
              onchange={(event) =>
                viewerModel.setProjectDisplayCoordinateReferenceMode(
                  (event.currentTarget as HTMLSelectElement).value as
                    | "native_engineering"
                    | "coordinate_reference_id"
                )}
            >
              <option value="native_engineering">Native engineering coordinates</option>
              <option value="coordinate_reference_id">Specific CRS identifier</option>
            </select>
          </label>

          <label class="field">
            <span>Display CRS Identifier</span>
            <input
              bind:value={viewerModel.projectDisplayCoordinateReferenceIdDraft}
              type="text"
              placeholder="EPSG:23031"
              disabled={
                viewerModel.projectDisplayCoordinateReferenceMode !== "coordinate_reference_id" ||
                viewerModel.projectGeospatialSettingsSaving
              }
              onkeydown={(event) => {
                if (event.key === "Enter") {
                  void viewerModel.saveProjectDisplaySettings();
                }
              }}
            />
          </label>

          <div class="action-row">
            <button
              type="button"
              disabled={viewerModel.projectGeospatialSettingsSaving}
              onclick={() => void viewerModel.saveProjectDisplaySettings()}
            >
              {viewerModel.projectGeospatialSettingsSaving ? "Saving…" : "Apply Project CRS"}
            </button>
            {#if viewerModel.suggestedProjectDisplayCoordinateReferenceId}
              <button
                type="button"
                class="secondary"
                disabled={viewerModel.projectGeospatialSettingsSaving}
                onclick={() =>
                  void viewerModel.saveProjectDisplaySettings("user_selected", {
                    kind: "coordinate_reference_id",
                    coordinateReferenceId: viewerModel.suggestedProjectDisplayCoordinateReferenceId ?? ""
                  })}
              >
                Use Survey CRS
              </button>
            {/if}
          </div>

          <div class="meta-grid">
            <div class="meta-card">
              <span>Detected native CRS</span>
              <strong>
                {viewerModel.activeDetectedNativeCoordinateReferenceId ??
                  viewerModel.activeDetectedNativeCoordinateReferenceName ??
                  "Unknown"}
              </strong>
            </div>
            <div class="meta-card">
              <span>Effective native CRS</span>
              <strong>
                {viewerModel.activeEffectiveNativeCoordinateReferenceId ??
                  viewerModel.activeEffectiveNativeCoordinateReferenceName ??
                  "Unknown"}
              </strong>
            </div>
            <div class="meta-card">
              <span>Map transform</span>
              <strong>{transformStatusLabel(activeSurveyMapSurvey?.transform_status)}</strong>
            </div>
            <div class="meta-card">
              <span>Imported horizons</span>
              <strong>{viewerModel.surveyMapSource?.horizons.length ?? 0}</strong>
            </div>
            <div class="meta-card">
              <span>Project CRS Source</span>
              <strong>{viewerModel.projectGeospatialSettingsSource ?? "Unresolved"}</strong>
            </div>
            <div class="meta-card">
              <span>Suggested CRS</span>
              <strong>{viewerModel.suggestedProjectDisplayCoordinateReferenceId ?? "None"}</strong>
            </div>
            <div class="meta-card">
              <span>Selected project survey CRS</span>
              <strong>
                {selectedProjectSurvey?.effectiveCoordinateReferenceId ??
                  selectedProjectSurvey?.effectiveCoordinateReferenceName ??
                  "Unknown"}
              </strong>
            </div>
            <div class="meta-card">
              <span>Project map readiness</span>
              <strong>
                {transformStatusLabel(selectedProjectSurveyCompatibility?.transformStatus)}
              </strong>
            </div>
          </div>

          {#if showSelectedProjectSurveyCompatibilityMessage}
            <div class="warning-list">
              <p>{selectedProjectSurveyCompatibilityMessage}</p>
            </div>
          {/if}

          <label class="field">
            <span>Override native CRS</span>
            <input
              bind:value={viewerModel.nativeCoordinateReferenceOverrideIdDraft}
              type="text"
              placeholder="EPSG:23031"
              disabled={!viewerModel.comparePrimaryStorePath || viewerModel.loading}
            />
          </label>

          <label class="field">
            <span>Override label</span>
            <input
              bind:value={viewerModel.nativeCoordinateReferenceOverrideNameDraft}
              type="text"
              placeholder="ED50 / UTM zone 31N"
              disabled={!viewerModel.comparePrimaryStorePath || viewerModel.loading}
            />
          </label>

          <div class="action-row">
            <button
              type="button"
              disabled={
                !viewerModel.comparePrimaryStorePath ||
                viewerModel.loading ||
                !viewerModel.nativeCoordinateReferenceOverrideIdDraft.trim()
              }
              onclick={() =>
                void viewerModel.setActiveDatasetNativeCoordinateReference(
                  viewerModel.nativeCoordinateReferenceOverrideIdDraft,
                  viewerModel.nativeCoordinateReferenceOverrideNameDraft
                )}
            >
              Apply CRS
            </button>
            <button
              type="button"
              class="secondary"
              disabled={!viewerModel.comparePrimaryStorePath || viewerModel.loading}
              onclick={() => void viewerModel.setActiveDatasetNativeCoordinateReference(null, null)}
            >
              Clear Override
            </button>
          </div>

          {#if viewerModel.workspaceCoordinateReferenceWarnings.length}
            <div class="warning-list">
              {#each viewerModel.workspaceCoordinateReferenceWarnings as warning (warning)}
                <p>{warning}</p>
              {/each}
            </div>
          {/if}
        </section>

        <section class="settings-section">
          <div class="section-heading">
            <h3>Project Wells</h3>
            <p>Project inventory, active wellbore, and section overlay resolution.</p>
          </div>

          <label class="field">
            <span>Project Root</span>
            <div class="field-row">
              <input
                bind:value={viewerModel.projectRoot}
                type="text"
                placeholder="C:\\data\\ophiolite-project"
                onblur={() => void viewerModel.setProjectRoot(viewerModel.projectRoot)}
                onkeydown={(event) => {
                  if (event.key === "Enter") {
                    void viewerModel.setProjectRoot(viewerModel.projectRoot);
                  }
                }}
              />
              <button type="button" class="secondary" onclick={() => void handlePickProjectRoot()}>
                Browse…
              </button>
            </div>
          </label>

          <div class="action-row">
            <button
              type="button"
              class="secondary"
              disabled={viewerModel.loading || !viewerModel.projectRoot}
              onclick={() => void viewerModel.refreshProjectWellOverlayInventory(viewerModel.projectRoot)}
            >
              Refresh Inventory
            </button>
            <button
              type="button"
              disabled={!viewerModel.canResolveConfiguredProjectSectionWellOverlays || viewerModel.projectSectionWellOverlaysLoading || viewerModel.loading}
              onclick={() => void handleResolveProjectWellOverlays()}
            >
              {viewerModel.projectSectionWellOverlaysLoading ? "Resolving…" : "Resolve Overlays"}
            </button>
            <button
              type="button"
              class="secondary"
              disabled={!viewerModel.sectionWellOverlays.length && !viewerModel.projectSectionWellOverlays}
              onclick={() => viewerModel.clearProjectSectionWellOverlays()}
            >
              Clear
            </button>
          </div>

          {#if viewerModel.projectSectionWellOverlayResolveBlocker}
            <div class="warning-list">
              <p>{viewerModel.projectSectionWellOverlayResolveBlocker}</p>
            </div>
          {/if}

          {#if viewerModel.projectSurveyDisplayCompatibilitySummaryLine}
            <div class="meta-grid compact-meta-grid">
              <div class="meta-card">
                <span>Survey readiness</span>
                <strong>{viewerModel.projectSurveyDisplayCompatibilitySummaryLine}</strong>
              </div>
              {#if viewerModel.projectWellboreDisplayCompatibilitySummaryLine}
                <div class="meta-card">
                  <span>Wellbore readiness</span>
                  <strong>{viewerModel.projectWellboreDisplayCompatibilitySummaryLine}</strong>
                </div>
              {/if}
            </div>
          {/if}

          {#if viewerModel.projectDisplayCompatibilityBlockingMessages.length}
            <div class="warning-list">
              {#each viewerModel.projectDisplayCompatibilityBlockingMessages as message (message)}
                <p>{message}</p>
              {/each}
            </div>
          {/if}

          <label class="field">
            <span>Survey Asset</span>
            <select
              bind:value={viewerModel.projectSurveyAssetId}
              disabled={viewerModel.projectWellOverlayInventoryLoading || !viewerModel.projectSurveyAssets.length}
              onchange={() => viewerModel.setProjectSurveyAssetId(viewerModel.projectSurveyAssetId)}
            >
              {#if viewerModel.projectSurveyAssets.length}
                {#each viewerModel.projectSurveySelectionGroups as group (group.label)}
                  <optgroup label={group.label}>
                    {#each group.surveys as survey (survey.assetId)}
                      <option value={survey.assetId}>{viewerModel.projectSurveyOptionLabel(survey)}</option>
                    {/each}
                  </optgroup>
                {/each}
              {:else}
                <option value="">
                  {viewerModel.projectWellOverlayInventoryLoading ? "Loading surveys..." : "No survey assets found"}
                </option>
              {/if}
            </select>
          </label>

          <label class="field">
            <span>Wellbore</span>
            <select
              bind:value={viewerModel.projectWellboreId}
              disabled={viewerModel.projectWellOverlayInventoryLoading || !viewerModel.projectWellboreInventory.length}
              onchange={() => viewerModel.setProjectWellboreId(viewerModel.projectWellboreId)}
            >
              {#if viewerModel.projectWellboreInventory.length}
                {#each viewerModel.projectWellboreSelectionGroups as group (group.label)}
                  <optgroup label={group.label}>
                    {#each group.wellbores as wellbore (wellbore.wellboreId)}
                      <option value={wellbore.wellboreId}>
                        {viewerModel.projectWellboreOptionLabel(wellbore)}
                      </option>
                    {/each}
                  </optgroup>
                {/each}
              {:else}
                <option value="">
                  {viewerModel.projectWellOverlayInventoryLoading ? "Loading wellbores..." : "No wellbores found"}
                </option>
              {/if}
            </select>
          </label>

          <label class="field">
            <span>Tolerance (m)</span>
            <input
              bind:value={viewerModel.projectSectionToleranceM}
              type="number"
              min="0.1"
              step="0.1"
              onblur={() => viewerModel.setProjectSectionToleranceM(viewerModel.projectSectionToleranceM)}
            />
          </label>

          {#if viewerModel.projectWellOverlayInventoryError}
            <p class="status error">{viewerModel.projectWellOverlayInventoryError}</p>
          {:else if viewerModel.projectWellOverlayInventoryLoading}
            <p class="status">Loading project inventory...</p>
          {/if}

          <div class="meta-grid">
            <div class="meta-card">
              <span>Survey assets</span>
              <strong>{viewerModel.projectSurveyAssets.length}</strong>
            </div>
            <div class="meta-card">
              <span>Wellbores</span>
              <strong>{viewerModel.projectWellboreInventory.length}</strong>
            </div>
            <div class="meta-card">
              <span>Observation sets</span>
              <strong>{viewerModel.projectWellTimeDepthObservationSets.length}</strong>
            </div>
            <div class="meta-card">
              <span>Compiled models</span>
              <strong>{viewerModel.projectWellTimeDepthModels.length}</strong>
            </div>
          </div>

          {#if viewerModel.selectedProjectWellboreInventoryItem}
            <div class="selected-block">
              <h4>Selected Wellbore</h4>
              <p>
                {viewerModel.selectedProjectWellboreInventoryItem.wellName} |
                {viewerModel.selectedProjectWellboreInventoryItem.wellboreName}
              </p>
              <p>
                {viewerModel.projectWellboreStatusLabel(viewerModel.selectedProjectWellboreInventoryItem)}
              </p>
              <p>
                Trajectories {viewerModel.selectedProjectWellboreInventoryItem.trajectoryAssetCount} | Models {viewerModel.selectedProjectWellboreInventoryItem.wellTimeDepthModelCount}
              </p>
              {#if viewerModel.selectedProjectWellboreInventoryItem.activeWellTimeDepthModelAssetId}
                <p>Project active model {viewerModel.selectedProjectWellboreInventoryItem.activeWellTimeDepthModelAssetId}</p>
              {/if}
            </div>
          {/if}

          {#if showSelectedProjectWellboreCompatibilityMessage}
            <div class="warning-list">
              <p>{selectedProjectWellboreCompatibilityMessage}</p>
            </div>
          {/if}

          {#if viewerModel.projectWellTimeDepthModelsError}
            <p class="status error">{viewerModel.projectWellTimeDepthModelsError}</p>
          {:else if viewerModel.projectWellTimeDepthModelsLoading}
            <p class="status">Loading well models...</p>
          {/if}

          <div class="footnote">
            <p>Use <strong>File &gt; Import</strong> for seismic, horizons, velocity functions, and well-model JSON.</p>
          </div>
        </section>
      </div>
    </div>
  </div>
{/if}

<style>
  .settings-backdrop {
    position: fixed;
    inset: 0;
    z-index: 40;
    display: flex;
    align-items: center;
    justify-content: center;
    padding: 24px;
    background: rgba(38, 55, 71, 0.2);
    backdrop-filter: blur(4px);
  }

  .settings-dialog {
    width: min(1080px, calc(100vw - 48px));
    max-height: min(820px, calc(100vh - 48px));
    overflow: auto;
    display: grid;
    gap: 18px;
    padding: 22px;
    border: 1px solid var(--app-border);
    border-radius: 8px;
    background: var(--panel-bg);
    color: var(--text-primary);
    box-shadow: 0 20px 60px rgba(42, 64, 84, 0.18);
  }

  .settings-header {
    display: flex;
    align-items: flex-start;
    justify-content: space-between;
    gap: 16px;
  }

  .settings-header h2,
  .section-heading h3,
  .selected-block h4 {
    margin: 0;
    font-size: 16px;
    font-weight: 650;
  }

  .settings-header p,
  .section-heading p,
  .selected-block p,
  .footnote p {
    margin: 4px 0 0;
    color: var(--text-muted);
  }

  .settings-grid {
    display: grid;
    grid-template-columns: repeat(2, minmax(0, 1fr));
    gap: 16px;
  }

  .settings-section {
    display: grid;
    gap: 14px;
    padding: 16px;
    border: 1px solid var(--app-border);
    border-radius: 8px;
    background: var(--surface-bg);
  }

  .field,
  .meta-card,
  .selected-block {
    display: grid;
    gap: 6px;
  }

  .field span,
  .meta-card span {
    font-size: 11px;
    text-transform: uppercase;
    color: var(--text-dim);
    letter-spacing: 0.04em;
  }

  .field input,
  .field select {
    min-width: 0;
    padding: 9px 10px;
    border: 1px solid var(--app-border-strong);
    border-radius: 6px;
    background: #fff;
    color: var(--text-primary);
    font: inherit;
  }

  .field input:disabled,
  .field select:disabled {
    background: var(--surface-subtle);
    color: var(--text-muted);
  }

  .field-row,
  .action-row {
    display: flex;
    flex-wrap: wrap;
    gap: 8px;
  }

  .field-row input {
    flex: 1 1 240px;
  }

  button {
    padding: 9px 12px;
    border: 1px solid var(--accent-border);
    border-radius: 6px;
    background: var(--accent-bg);
    color: var(--accent-text);
    font: inherit;
    cursor: pointer;
  }

  button.secondary,
  .icon-button {
    border-color: var(--app-border-strong);
    background: var(--surface-subtle);
    color: var(--text-primary);
  }

  button:disabled {
    opacity: 0.55;
    cursor: not-allowed;
  }

  .meta-grid {
    display: grid;
    grid-template-columns: repeat(2, minmax(0, 1fr));
    gap: 10px;
  }

  .compact-meta-grid {
    grid-template-columns: 1fr;
  }

  .meta-card,
  .selected-block {
    padding: 10px 12px;
    border: 1px solid var(--app-border);
    border-radius: 6px;
    background: #fff;
  }

  .meta-card strong {
    color: var(--text-primary);
    font-weight: 600;
  }

  .warning-list {
    display: grid;
    gap: 8px;
    padding: 10px 12px;
    border: 1px solid #d7d8ad;
    border-radius: 6px;
    background: #fffde8;
    color: #705c1c;
  }

  .warning-list p,
  .status {
    margin: 0;
  }

  .status {
    color: var(--text-muted);
  }

  .status.error {
    color: #a74646;
  }

  .footnote {
    padding-top: 4px;
    border-top: 1px solid var(--app-border);
  }

  @media (max-width: 900px) {
    .settings-backdrop {
      padding: 12px;
    }

    .settings-dialog {
      width: 100%;
      max-height: calc(100vh - 24px);
      padding: 16px;
    }

    .settings-grid,
    .meta-grid {
      grid-template-columns: 1fr;
    }
  }
</style>
