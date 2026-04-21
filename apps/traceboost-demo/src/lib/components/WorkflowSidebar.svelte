<svelte:options runes={true} />

<script lang="ts">
  import { getViewerModelContext } from "../viewer-model.svelte";

  interface Props {
    showSidebar: boolean;
    hideSidebar: () => void;
  }

  let { showSidebar, hideSidebar }: Props = $props();

  const viewerModel = getViewerModelContext();
  let volumeContextMenu = $state.raw<{
    entryId: string;
    label: string;
    x: number;
    y: number;
    exportable: boolean;
  } | null>(null);
  let expandedSections = $state({
    seismic: true,
    horizons: true,
    velocityModels: true,
    wells: true
  });

  function basename(filePath: string): string {
    return filePath.split(/[\\/]/).pop() ?? filePath;
  }

  function fileStem(filePath: string | null | undefined): string {
    const filename = basename(filePath ?? "");
    return filename.replace(/\.[^.]+$/, "");
  }

  function stripGeneratedHashSuffix(value: string): string {
    return value.replace(/-[0-9a-f]{16}$/i, "");
  }

  function normalizeGeneratedSeparators(value: string): string {
    return value.replace(/\s*(?:Â·|·)\s*/g, " | ");
  }

  function datasetLabel(displayName: string, fallbackPath: string | null | undefined, entryId: string): string {
    const trimmedDisplayName = displayName.trim();
    if (trimmedDisplayName) {
      return normalizeGeneratedSeparators(stripGeneratedHashSuffix(trimmedDisplayName));
    }

    const preferredPathLabel = fileStem(fallbackPath);
    if (preferredPathLabel) {
      return normalizeGeneratedSeparators(stripGeneratedHashSuffix(preferredPathLabel));
    }

    return entryId;
  }

  function entryStorePath(entry: {
    last_dataset?: { store_path: string } | null;
    imported_store_path?: string | null;
    preferred_store_path?: string | null;
  }): string {
    return entry.last_dataset?.store_path ?? entry.imported_store_path ?? entry.preferred_store_path ?? "";
  }

  function velocitySourceKindLabel(sourceKind: string): string {
    switch (sourceKind) {
      case "velocity_grid3_d":
        return "3D grid";
      case "horizon_layer_model":
        return "Horizon model";
      case "checkshot_model1_d":
        return "Checkshot";
      case "sonic_log1_d":
        return "Sonic";
      case "vp_log1_d":
        return "Vp";
      case "velocity_function1_d":
        return "1D function";
      case "constant_velocity":
        return "Constant";
      default:
        return sourceKind;
    }
  }

  function wellTimeDepthAssetKindLabel(assetKind: string): string {
    switch (assetKind) {
      case "checkshot_vsp_observation_set":
        return "Checkshot/VSP";
      case "manual_time_depth_pick_set":
        return "Manual Picks";
      case "well_tie_observation_set":
        return "Well Tie";
      case "well_time_depth_authored_model":
        return "Authored";
      case "well_time_depth_model":
        return "Compiled";
      default:
        return assetKind;
    }
  }

  function pluralize(count: number, singular: string, plural = `${singular}s`): string {
    return `${count} ${count === 1 ? singular : plural}`;
  }

  function wellTimeDepthObservationSubtitle(
    asset: (typeof viewerModel.projectWellTimeDepthObservationSets)[number]
  ): string {
    const parts = [wellTimeDepthAssetKindLabel(asset.assetKind)];
    if (
      asset.tieWindowStartMs !== undefined &&
      asset.tieWindowStartMs !== null &&
      asset.tieWindowEndMs !== undefined &&
      asset.tieWindowEndMs !== null
    ) {
      parts.push(`${asset.tieWindowStartMs.toFixed(0)}-${asset.tieWindowEndMs.toFixed(0)} ms`);
    }
    if (asset.correlation !== undefined && asset.correlation !== null) {
      parts.push(`corr ${asset.correlation.toFixed(2)}`);
    }
    parts.push(pluralize(asset.sampleCount, "sample"));
    return parts.join(" | ");
  }

  function residualSubtitle(
    asset: (typeof viewerModel.projectResidualAssets)[number]
  ): string {
    const parts: string[] = [];
    if (asset.markerName) {
      parts.push(asset.markerName);
    } else if (asset.markerNames.length) {
      parts.push(asset.markerNames.join(", "));
    }
    if (asset.horizonId) {
      parts.push(asset.horizonId);
    }
    parts.push(pluralize(asset.pointCount, "point"));
    return parts.join(" | ");
  }

  function closeVolumeContextMenu(): void {
    volumeContextMenu = null;
  }

  function openVolumeContextMenu(
    event: MouseEvent,
    entryId: string,
    label: string,
    exportable: boolean
  ): void {
    event.preventDefault();
    event.stopPropagation();
    const menuWidth = 192;
    const menuHeight = 54;
    volumeContextMenu = {
      entryId,
      label,
      x: Math.min(event.clientX, Math.max(12, window.innerWidth - menuWidth - 12)),
      y: Math.min(event.clientY, Math.max(12, window.innerHeight - menuHeight - 12)),
      exportable
    };
  }

  async function handleContextMenuExport(): Promise<void> {
    const context = volumeContextMenu;
    if (!context?.exportable) {
      return;
    }
    closeVolumeContextMenu();
    await viewerModel.openDatasetExportDialog(context.entryId);
  }

  function handleVolumeListKeyDown(event: KeyboardEvent): void {
    if (event.key === "Escape" && volumeContextMenu) {
      closeVolumeContextMenu();
      return;
    }

    if (!(event.ctrlKey || event.metaKey)) {
      return;
    }

    const key = event.key.toLowerCase();
    if (key === "c" && viewerModel.activeEntryId) {
      event.preventDefault();
      viewerModel.copyActiveWorkspaceEntry();
    }

    if (key === "v") {
      event.preventDefault();
      void viewerModel.pasteCopiedWorkspaceEntry();
    }
  }

  function toggleSection(section: "seismic" | "horizons" | "velocityModels" | "wells"): void {
    expandedSections = {
      ...expandedSections,
      [section]: !expandedSections[section]
    };
  }

  function openVelocityModelWorkbench(): void {
    viewerModel.openVelocityModelWorkbench();
  }

  function openDepthConversionWorkbench(): void {
    const blocker = viewerModel.depthConversionBlocker;
    if (blocker) {
      viewerModel.note(blocker, "ui", "warn");
      return;
    }
    viewerModel.openDepthConversionWorkbench();
  }

  const importedHorizons = $derived(viewerModel.availableHorizonAssets);
  const projectHorizons = $derived(viewerModel.projectSurveyHorizonAssets);
  const horizonInventory = $derived(projectHorizons.length ? projectHorizons : importedHorizons);
  const activePreviewHorizonId = $derived(viewerModel.surveyMapSource?.scalar_field_horizon_id ?? null);
  const activeWellboreId = $derived(viewerModel.selectedProjectWellboreInventoryItem?.wellboreId ?? null);
  const activeSelectedHorizonId = $derived(
    viewerModel.selectedProjectHorizonAsset?.id ?? activePreviewHorizonId
  );
  const activeSelectedMarkerName = $derived(viewerModel.selectedProjectWellMarker?.name ?? null);
  const activeResidualAssetId = $derived(viewerModel.selectedProjectResidualAsset?.assetId ?? null);
</script>

<svelte:window onclick={closeVolumeContextMenu} />

<aside class:hidden={!showSidebar} class="sidebar">
  <div class="sidebar-header">
    <div class="logo-row">
      <svg
        class="logo-icon"
        viewBox="0 0 24 24"
        width="32"
        height="32"
        fill="none"
        stroke="currentColor"
        stroke-width="1.5"
      >
        <path
          d="M3 20 L6 8 L9 14 L12 4 L15 16 L18 10 L21 20"
          stroke-linecap="round"
          stroke-linejoin="round"
        />
      </svg>
      <div class="logo-copy">
        <h1>TraceBoost <span class="version">v0.1.0</span></h1>
        <p class="subtitle">Session Data</p>
      </div>
      <button class="collapse-button" onclick={hideSidebar} aria-label="Hide sidebar">
        <svg viewBox="0 0 24 24" width="18" height="18" fill="none" stroke="currentColor" stroke-width="2">
          <polyline points="15 18 9 12 15 6" />
        </svg>
      </button>
    </div>
  </div>

  <div class="tree-shell" role="tree" tabindex="0" onkeydown={handleVolumeListKeyDown}>
    <!-- Seismic Volumes -->
    <div class="tree-branch">
      <button class="tree-root" type="button" onclick={() => toggleSection("seismic")} aria-expanded={expandedSections.seismic}>
        <span class="disclosure">{expandedSections.seismic ? "\u25BE" : "\u25B8"}</span>
        <span class="root-label">Seismic Volumes</span>
        <span class="root-count">{viewerModel.workspaceEntries.length}</span>
      </button>

      {#if expandedSections.seismic}
        <div class="tree-children">
          {#if viewerModel.workspaceEntries.length}
            {#each viewerModel.workspaceEntries as entry (entry.entry_id)}
              {@const visibleLabel = datasetLabel(
                entry.display_name,
                entry.source_path ?? entry.imported_store_path ?? entry.preferred_store_path,
                entry.entry_id
              )}
              <div class="tree-leaf-row">
                <button
                  class="tree-leaf"
                  class:is-active={viewerModel.activeEntryId === entry.entry_id}
                  type="button"
                  onclick={() => void viewerModel.activateDatasetEntry(entry.entry_id)}
                  oncontextmenu={(event) =>
                    openVolumeContextMenu(
                      event,
                      entry.entry_id,
                      visibleLabel,
                      entryStorePath(entry).trim().length > 0
                    )}
                  disabled={viewerModel.loading}
                  title={visibleLabel}
                >
                  <span class="leaf-label">{visibleLabel}</span>
                  <span class="leaf-meta">
                    {entry.source_path ? "Imported" : "Runtime"} | {entryStorePath(entry) || "No store path"}
                  </span>
                </button>
                <button
                  class="leaf-remove"
                  type="button"
                  onclick={() => void viewerModel.removeWorkspaceEntry(entry.entry_id)}
                  disabled={viewerModel.loading}
                  aria-label={`Remove ${visibleLabel}`}
                  title={`Remove ${visibleLabel}`}
                >
                  &times;
                </button>
              </div>
            {/each}
          {:else}
            <p class="tree-empty">No seismic volumes loaded.</p>
          {/if}
        </div>
      {/if}
    </div>

    <!-- Horizons -->
    <div class="tree-branch">
      <button class="tree-root" type="button" onclick={() => toggleSection("horizons")} aria-expanded={expandedSections.horizons}>
        <span class="disclosure">{expandedSections.horizons ? "\u25BE" : "\u25B8"}</span>
        <span class="root-label">Horizons</span>
        <span class="root-count">{horizonInventory.length}</span>
      </button>

      {#if expandedSections.horizons}
        <div class="tree-children">
          {#if horizonInventory.length}
            {#each horizonInventory as horizon (horizon.id)}
              <button
                class="tree-leaf"
                class:is-active={activeSelectedHorizonId === horizon.id}
                type="button"
                onclick={() => viewerModel.setSelectedProjectHorizonId(horizon.id)}
                title={horizon.name}
              >
                <span class="leaf-label">{horizon.name}</span>
                <span class="leaf-meta">
                  {horizon.vertical_domain === "depth" ? "Depth" : "TWT"} | {activePreviewHorizonId === horizon.id ? "Preview surface" : "Imported"}
                </span>
              </button>
            {/each}
          {:else if viewerModel.activeStorePath}
            <p class="tree-empty">No imported horizons for the active volume.</p>
          {:else}
            <p class="tree-empty">Open a seismic volume to inspect horizons.</p>
          {/if}
        </div>
      {/if}
    </div>

    <!-- Velocity Models -->
    <div class="tree-branch">
      <button class="tree-root" type="button" onclick={() => toggleSection("velocityModels")} aria-expanded={expandedSections.velocityModels}>
        <span class="disclosure">{expandedSections.velocityModels ? "\u25BE" : "\u25B8"}</span>
        <span class="root-label">Velocity Models</span>
        <span class="root-count">{viewerModel.availableVelocityModels.length + 1}</span>
      </button>

      {#if expandedSections.velocityModels}
        <div class="tree-children">
          <div class="section-actions">
            <button class="section-action" type="button" onclick={openVelocityModelWorkbench}>
              Velocity Model...
            </button>
            <button
              class="section-action"
              type="button"
              onclick={openDepthConversionWorkbench}
              disabled={!viewerModel.canOpenDepthConversionWorkbench}
            >
              Depth Conversion...
            </button>
          </div>

          <button
            class="tree-leaf"
            class:is-active={!viewerModel.activeVelocityModelAssetId}
            type="button"
            disabled={!viewerModel.activeStorePath || viewerModel.loading}
            onclick={() => void viewerModel.activateVelocityModel(null)}
          >
            <span class="leaf-label">Global 1D fallback</span>
            <span class="leaf-meta">Constant or 1D velocity function</span>
          </button>

          {#if viewerModel.velocityModelsError}
            <p class="tree-status error">{viewerModel.velocityModelsError}</p>
          {:else if viewerModel.velocityModelsLoading}
            <p class="tree-status">Loading velocity models...</p>
          {:else if viewerModel.availableVelocityModels.length}
            {#each viewerModel.availableVelocityModels as model (model.id)}
              <button
                class="tree-leaf"
                class:is-active={viewerModel.activeVelocityModelAssetId === model.id}
                type="button"
                disabled={viewerModel.loading}
                onclick={() => void viewerModel.activateVelocityModel(model.id)}
                title={model.name}
              >
                <span class="leaf-label">{model.name}</span>
                <span class="leaf-meta">
                  {velocitySourceKindLabel(model.source_kind)} | {model.coverage.relationship}
                </span>
              </button>
            {/each}
          {:else if viewerModel.activeStorePath}
            <p class="tree-empty">No velocity models for the active volume.</p>
          {:else}
            <p class="tree-empty">Open a seismic volume to inspect velocity models.</p>
          {/if}
        </div>
      {/if}
    </div>

    <!-- Wells -->
    <div class="tree-branch">
      <button class="tree-root" type="button" onclick={() => toggleSection("wells")} aria-expanded={expandedSections.wells}>
        <span class="disclosure">{expandedSections.wells ? "\u25BE" : "\u25B8"}</span>
        <span class="root-label">Wells</span>
        <span class="root-count">{viewerModel.projectWellboreInventory.length}</span>
      </button>

      {#if expandedSections.wells}
        <div class="tree-children">
          {#if viewerModel.projectWellOverlayInventoryError}
            <p class="tree-status error">{viewerModel.projectWellOverlayInventoryError}</p>
          {:else if viewerModel.projectWellOverlayInventoryLoading}
            <p class="tree-status">Loading project inventory...</p>
          {:else if viewerModel.projectWellboreInventory.length}
            {#if viewerModel.selectedProjectSurveyAsset}
              <p class="tree-status">
                Survey {viewerModel.selectedProjectSurveyAsset.name} | {viewerModel.selectedProjectSurveyAsset.wellboreName}
              </p>
            {/if}
            {#if viewerModel.projectWellboreDisplayCompatibilitySummaryLine}
              <p class="tree-status">{viewerModel.projectWellboreDisplayCompatibilitySummaryLine}</p>
            {/if}

            {#if viewerModel.projectSectionWellOverlayResolveBlocker}
              <p class="tree-status error">{viewerModel.projectSectionWellOverlayResolveBlocker}</p>
            {/if}

            {#each viewerModel.projectWellboreSelectionGroups as group (group.label)}
              {#if viewerModel.projectWellboreSelectionGroups.length > 1}
                <div class="tree-group-header">
                  <span>{group.label}</span>
                  <span class="root-count">{group.wellbores.length}</span>
                </div>
              {/if}

              {#each group.wellbores as wellbore (wellbore.wellboreId)}
                <button
                  class="tree-leaf"
                  class:is-active={activeWellboreId === wellbore.wellboreId}
                  type="button"
                  onclick={() => viewerModel.setProjectWellboreId(wellbore.wellboreId)}
                  title={viewerModel.projectWellboreOptionLabel(wellbore)}
                >
                  <span class="leaf-label">{wellbore.wellName} | {wellbore.wellboreName}</span>
                  <span class="leaf-meta">{viewerModel.projectWellboreStatusLabel(wellbore)}</span>
                </button>

                {#if activeWellboreId === wellbore.wellboreId}
                  <div class="tree-nested">
                    {#if viewerModel.projectWellMarkers.length}
                      <div class="tree-sub-branch">
                        <div class="tree-sub-header">
                          <span>Markers</span>
                          <span class="root-count">{viewerModel.projectWellMarkers.length}</span>
                        </div>
                        {#each viewerModel.projectWellMarkers as marker (marker.name)}
                          <button
                            class="tree-leaf depth-2"
                            class:is-active={activeSelectedMarkerName === marker.name}
                            type="button"
                            onclick={() => viewerModel.setSelectedProjectWellMarkerName(marker.name)}
                            title={marker.name}
                          >
                            <span class="leaf-label">{marker.name}</span>
                            <span class="leaf-meta">
                              {marker.markerKind ?? "marker"} | {marker.topDepth.toFixed(2)}{marker.depthReference ? ` ${marker.depthReference}` : ""}
                            </span>
                          </button>
                        {/each}
                      </div>
                    {/if}

                    {#if viewerModel.projectResidualAssets.length}
                      <div class="tree-sub-branch">
                        <div class="tree-sub-header">
                          <span>Residuals</span>
                          <span class="root-count">{viewerModel.projectResidualAssets.length}</span>
                        </div>
                        {#each viewerModel.projectResidualAssets as residualAsset (residualAsset.assetId)}
                          <button
                            class="tree-leaf depth-2"
                            class:is-active={activeResidualAssetId === residualAsset.assetId}
                            type="button"
                            onclick={() => viewerModel.setSelectedProjectResidualAssetId(residualAsset.assetId)}
                            title={residualAsset.name}
                          >
                            <span class="leaf-label">{residualAsset.name}</span>
                            <span class="leaf-meta">{residualSubtitle(residualAsset)}</span>
                          </button>
                        {/each}
                      </div>
                    {/if}

                    {#if viewerModel.projectWellTimeDepthObservationSets.length}
                      <div class="tree-sub-branch">
                        <div class="tree-sub-header">
                          <span>Observation Sets</span>
                          <span class="root-count">{viewerModel.projectWellTimeDepthObservationSets.length}</span>
                        </div>
                        {#each viewerModel.projectWellTimeDepthObservationSets as asset (asset.assetId)}
                          {#if asset.assetKind === "well_tie_observation_set"}
                            <button
                              class="tree-leaf depth-2"
                              class:is-active={viewerModel.selectedProjectWellTieObservationAssetId === asset.assetId}
                              type="button"
                              onclick={() => viewerModel.resumeWellTieWorkbenchFromObservation(asset.assetId)}
                              title={`Resume ${asset.name}`}
                            >
                              <span class="leaf-label">{asset.name}</span>
                              <span class="leaf-meta">{wellTimeDepthObservationSubtitle(asset)}</span>
                            </button>
                          {:else}
                            <div class="tree-leaf static depth-2">
                              <span class="leaf-label">{asset.name}</span>
                              <span class="leaf-meta">{wellTimeDepthObservationSubtitle(asset)}</span>
                            </div>
                          {/if}
                        {/each}
                      </div>
                    {/if}

                    {#if viewerModel.projectWellTimeDepthAuthoredModels.length}
                      <div class="tree-sub-branch">
                        <div class="tree-sub-header">
                          <span>Authored Models</span>
                          <span class="root-count">{viewerModel.projectWellTimeDepthAuthoredModels.length}</span>
                        </div>
                        {#each viewerModel.projectWellTimeDepthAuthoredModels as model (model.assetId)}
                          <div class="tree-leaf static depth-2">
                            <span class="leaf-label">{model.name}</span>
                            <span class="leaf-meta">
                              {pluralize(model.sourceBindingCount, "source binding")} | {pluralize(model.assumptionIntervalCount, "interval assumption")}
                            </span>
                          </div>
                        {/each}
                      </div>
                    {/if}

                    {#if viewerModel.projectWellTimeDepthModels.length}
                      <div class="tree-sub-branch">
                        <div class="tree-sub-header">
                          <span>Compiled Models</span>
                          <span class="root-count">{viewerModel.projectWellTimeDepthModels.length}</span>
                        </div>
                        {#each viewerModel.projectWellTimeDepthModels as model (model.assetId)}
                          <button
                            class="tree-leaf depth-2"
                            class:is-active={viewerModel.selectedProjectWellTimeDepthModelAssetId === model.assetId}
                            type="button"
                            onclick={() => viewerModel.setSelectedProjectWellTimeDepthModelAssetId(model.assetId)}
                            title={model.name}
                          >
                            <span class="leaf-label">{model.name}</span>
                            <span class="leaf-meta">
                              {velocitySourceKindLabel(model.sourceKind)} | {pluralize(model.sampleCount, "sample")}{model.isActiveProjectModel ? " | project active" : ""}
                            </span>
                          </button>
                        {/each}
                      </div>
                    {/if}
                  </div>
                {/if}
              {/each}
            {/each}
          {:else}
            <p class="tree-empty">No project well inventory loaded.</p>
          {/if}
        </div>
      {/if}
    </div>
  </div>
</aside>

{#if volumeContextMenu}
  <div
    class="volume-context-menu"
    style={`left:${volumeContextMenu.x}px; top:${volumeContextMenu.y}px;`}
    role="menu"
    tabindex="0"
    onclick={(event) => event.stopPropagation()}
    onkeydown={(event) => {
      event.stopPropagation();
      if (event.key === "Escape") {
        closeVolumeContextMenu();
      }
    }}
  >
    <button
      class="volume-context-item"
      type="button"
      role="menuitem"
      disabled={!volumeContextMenu.exportable}
      onclick={() => void handleContextMenuExport()}
      title={
        volumeContextMenu.exportable
          ? `Export ${volumeContextMenu.label}`
          : "Export is unavailable because this entry has no runtime store path."
      }
    >
      Export...
    </button>
  </div>
{/if}

<style>
  .sidebar {
    min-height: 100vh;
    display: grid;
    grid-template-rows: auto minmax(0, 1fr);
    background: var(--panel-bg);
    border-right: 1px solid var(--app-border);
  }

  .sidebar.hidden {
    display: none;
  }

  .sidebar-header {
    padding: var(--sidebar-header-padding);
    border-bottom: 1px solid var(--app-border);
  }

  .logo-row {
    display: grid;
    grid-template-columns: auto minmax(0, 1fr) auto;
    align-items: center;
    gap: var(--ui-space-2);
  }

  .logo-icon {
    color: #3a8cc2;
  }

  .logo-copy h1 {
    margin: 0;
    font-size: 16px;
    font-weight: 650;
    color: var(--text-primary);
    line-height: 1.15;
  }

  .version {
    font-size: 11px;
    color: var(--text-dim);
    font-weight: 500;
  }

  .subtitle {
    margin: 2px 0 0;
    font-size: 11px;
    color: var(--text-muted);
    line-height: 1.2;
  }

  .collapse-button {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    width: 24px;
    height: 24px;
    border-radius: var(--ui-radius-sm);
    border: 1px solid var(--app-border-strong);
    background: var(--surface-subtle);
    color: var(--text-muted);
    cursor: pointer;
  }

  /* ── Tree shell ── */

  .tree-shell {
    min-height: 0;
    overflow: auto;
    padding: 6px 4px;
    display: flex;
    flex-direction: column;
    outline: none;
    font-size: 12px;
    line-height: 1.35;
  }

  /* ── Root branch toggle ── */

  .tree-branch {
    display: flex;
    flex-direction: column;
  }

  .tree-branch + .tree-branch {
    margin-top: 2px;
  }

  .tree-root {
    display: flex;
    align-items: center;
    gap: 2px;
    width: 100%;
    padding: 3px 6px;
    border: none;
    border-radius: 3px;
    background: transparent;
    color: var(--text-primary);
    text-align: left;
    cursor: pointer;
    font: inherit;
    font-size: 12px;
    line-height: 1.35;
  }

  .tree-root:hover {
    background: var(--surface-bg);
  }

  .disclosure {
    flex-shrink: 0;
    width: 12px;
    font-size: 10px;
    color: var(--text-muted);
    text-align: center;
  }

  .root-label {
    font-weight: 650;
  }

  .root-count {
    margin-left: auto;
    font-size: 10px;
    color: var(--text-muted);
    font-weight: 400;
  }

  /* ── Children container (depth 1) ── */

  .tree-children {
    display: flex;
    flex-direction: column;
    padding-left: 14px;
  }

  /* ── Leaf items ── */

  .tree-leaf {
    display: flex;
    flex-direction: column;
    gap: 1px;
    min-width: 0;
    padding: 2px 6px;
    border: none;
    border-radius: 3px;
    background: transparent;
    color: var(--text-primary);
    text-align: left;
    cursor: pointer;
    font: inherit;
    font-size: 12px;
    line-height: 1.35;
  }

  .tree-leaf.static {
    cursor: default;
  }

  .tree-leaf:hover:not(:disabled):not(.static) {
    background: var(--surface-bg);
  }

  .tree-leaf.is-active {
    background: var(--surface-bg);
  }

  .tree-leaf.is-active .leaf-label {
    font-weight: 650;
  }

  .tree-leaf:disabled {
    opacity: 0.55;
    cursor: default;
  }

  .leaf-label {
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .leaf-meta {
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    color: var(--text-muted);
    font-size: 10px;
    line-height: 1.3;
  }

  /* ── Leaf row with remove button ── */

  .tree-leaf-row {
    display: flex;
    align-items: start;
  }

  .tree-leaf-row .tree-leaf {
    flex: 1;
    min-width: 0;
  }

  .leaf-remove {
    flex-shrink: 0;
    width: 18px;
    height: 18px;
    margin-top: 2px;
    border: none;
    border-radius: 3px;
    background: transparent;
    color: var(--text-muted);
    cursor: pointer;
    font-size: 12px;
    line-height: 1;
    text-align: center;
    opacity: 0;
    transition: opacity 0.1s;
  }

  .tree-leaf-row:hover .leaf-remove {
    opacity: 1;
  }

  .leaf-remove:hover {
    background: var(--surface-bg);
    color: var(--text-primary);
  }

  /* ── Nested well children (depth 2) ── */

  .tree-nested {
    display: flex;
    flex-direction: column;
    padding-left: 14px;
  }

  .tree-sub-branch {
    display: flex;
    flex-direction: column;
  }

  .tree-sub-header {
    display: flex;
    align-items: center;
    gap: 4px;
    padding: 3px 6px 1px;
    font-size: 10px;
    font-weight: 600;
    letter-spacing: 0.04em;
    text-transform: uppercase;
    color: var(--text-dim);
  }

  .tree-sub-header .root-count {
    font-weight: 400;
  }

  .tree-leaf.depth-2 {
    padding-left: 12px;
  }

  /* ── Group header (multi-group wells) ── */

  .tree-group-header {
    display: flex;
    align-items: center;
    gap: 4px;
    padding: 4px 6px 1px;
    font-size: 10px;
    font-weight: 600;
    letter-spacing: 0.04em;
    text-transform: uppercase;
    color: var(--text-dim);
  }

  /* ── Empty / status text ── */

  .tree-empty {
    margin: 0;
    padding: 2px 6px;
    font-size: 11px;
    font-style: italic;
    color: var(--text-muted);
  }

  .tree-status {
    margin: 0;
    padding: 2px 6px;
    font-size: 11px;
    color: var(--text-muted);
  }

  .tree-status.error {
    color: #a74646;
  }

  /* ── Action buttons (velocity workbench) ── */

  .section-actions {
    display: grid;
    grid-template-columns: repeat(2, minmax(0, 1fr));
    gap: 4px;
    padding: 2px 0 4px;
  }

  .section-action {
    border: 1px solid var(--app-border);
    border-radius: 3px;
    background: var(--surface-bg);
    color: var(--text-primary);
    padding: 3px 6px;
    font-size: 11px;
    text-align: center;
    cursor: pointer;
  }

  .section-action:disabled {
    opacity: 0.55;
    cursor: default;
  }

  /* ── Context menu (unchanged) ── */

  .volume-context-menu {
    position: fixed;
    z-index: 50;
    min-width: 168px;
    padding: var(--ui-space-2);
    border: 1px solid var(--app-border-strong);
    border-radius: var(--ui-radius-md);
    background: var(--panel-bg);
    box-shadow: var(--ui-shadow-popover);
  }

  .volume-context-item {
    width: 100%;
    min-height: var(--ui-button-height);
    padding: 0 var(--ui-button-padding-x);
    border: 0;
    border-radius: var(--ui-radius-md);
    background: transparent;
    color: var(--text-primary);
    text-align: left;
    cursor: pointer;
  }

  .volume-context-item:hover:not(:disabled) {
    background: var(--surface-bg);
  }
</style>
