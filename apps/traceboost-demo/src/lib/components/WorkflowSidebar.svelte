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
  const activePreviewHorizonId = $derived(viewerModel.surveyMapSource?.scalar_field_horizon_id ?? null);
  const activeWellboreId = $derived(viewerModel.selectedProjectWellboreInventoryItem?.wellboreId ?? null);
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
    <section class="tree-section">
      <button class="section-header" type="button" onclick={() => toggleSection("seismic")} aria-expanded={expandedSections.seismic}>
        <span class="section-arrow">{expandedSections.seismic ? "▾" : "▸"}</span>
        <span class="section-title">Seismic</span>
        <span class="section-count">{viewerModel.workspaceEntries.length}</span>
      </button>

      {#if expandedSections.seismic}
        <div class="section-body">
          {#if viewerModel.workspaceEntries.length}
            {#each viewerModel.workspaceEntries as entry (entry.entry_id)}
              {@const visibleLabel = datasetLabel(
                entry.display_name,
                entry.source_path ?? entry.imported_store_path ?? entry.preferred_store_path,
                entry.entry_id
              )}
              <div class="tree-row">
                <button
                  class:active={viewerModel.activeEntryId === entry.entry_id}
                  class="tree-item"
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
                  <span class="item-icon">▦</span>
                  <span class="item-copy">
                    <span class="item-label">{visibleLabel}</span>
                    <span class="item-subtitle">
                      {entry.source_path ? "Imported source" : "Runtime store"} | {entryStorePath(entry) || "No store path"}
                    </span>
                  </span>
                </button>
                <button
                  class="item-remove"
                  type="button"
                  onclick={() => void viewerModel.removeWorkspaceEntry(entry.entry_id)}
                  disabled={viewerModel.loading}
                  aria-label={`Remove ${visibleLabel}`}
                  title={`Remove ${visibleLabel}`}
                >
                  X
                </button>
              </div>
            {/each}
          {:else}
            <div class="section-empty">
              <p>No seismic volumes loaded.</p>
              <p>Use <strong>File &gt; Open Volume...</strong> or <strong>File &gt; Import</strong>.</p>
            </div>
          {/if}
        </div>
      {/if}
    </section>

    <section class="tree-section">
      <button class="section-header" type="button" onclick={() => toggleSection("horizons")} aria-expanded={expandedSections.horizons}>
        <span class="section-arrow">{expandedSections.horizons ? "▾" : "▸"}</span>
        <span class="section-title">Horizons</span>
        <span class="section-count">{importedHorizons.length}</span>
      </button>

      {#if expandedSections.horizons}
        <div class="section-body">
          {#if importedHorizons.length}
            {#each importedHorizons as horizon (horizon.id)}
              <div class:active={activePreviewHorizonId === horizon.id} class="tree-item static">
                <span class="item-icon">◌</span>
                <span class="item-copy">
                  <span class="item-label">{horizon.name}</span>
                  <span class="item-subtitle">{activePreviewHorizonId === horizon.id ? "Preview surface" : "Imported horizon"}</span>
                </span>
              </div>
            {/each}
          {:else if viewerModel.activeStorePath}
            <div class="section-empty">
              <p>No imported horizons for the active volume.</p>
            </div>
          {:else}
            <div class="section-empty">
              <p>Open a seismic volume to inspect imported horizons.</p>
            </div>
          {/if}
        </div>
      {/if}
    </section>

    <section class="tree-section">
      <button
        class="section-header"
        type="button"
        onclick={() => toggleSection("velocityModels")}
        aria-expanded={expandedSections.velocityModels}
      >
        <span class="section-arrow">{expandedSections.velocityModels ? "▾" : "▸"}</span>
        <span class="section-title">Velocity Models</span>
        <span class="section-count">{viewerModel.availableVelocityModels.length + 1}</span>
      </button>

      {#if expandedSections.velocityModels}
        <div class="section-body">
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
            class:active={!viewerModel.activeVelocityModelAssetId}
            class="tree-item"
            type="button"
            disabled={!viewerModel.activeStorePath || viewerModel.loading}
            onclick={() => void viewerModel.activateVelocityModel(null)}
          >
            <span class="item-icon">△</span>
            <span class="item-copy">
              <span class="item-label">Global 1D fallback</span>
              <span class="item-subtitle">Constant or 1D velocity function</span>
            </span>
          </button>

          {#if viewerModel.velocityModelsError}
            <p class="status error">{viewerModel.velocityModelsError}</p>
          {:else if viewerModel.velocityModelsLoading}
            <p class="status">Loading velocity models...</p>
          {:else if viewerModel.availableVelocityModels.length}
            {#each viewerModel.availableVelocityModels as model (model.id)}
              <button
                class:active={viewerModel.activeVelocityModelAssetId === model.id}
                class="tree-item"
                type="button"
                disabled={viewerModel.loading}
                onclick={() => void viewerModel.activateVelocityModel(model.id)}
                title={model.name}
              >
                <span class="item-icon">△</span>
                <span class="item-copy">
                  <span class="item-label">{model.name}</span>
                  <span class="item-subtitle">
                    {velocitySourceKindLabel(model.source_kind)} | {model.coverage.relationship}
                  </span>
                </span>
              </button>
            {/each}
          {:else if viewerModel.activeStorePath}
            <div class="section-empty">
              <p>No survey velocity models registered for the active volume.</p>
            </div>
          {:else}
            <div class="section-empty">
              <p>Open a seismic volume to inspect velocity models.</p>
            </div>
          {/if}
        </div>
      {/if}
    </section>

    <section class="tree-section">
      <button class="section-header" type="button" onclick={() => toggleSection("wells")} aria-expanded={expandedSections.wells}>
        <span class="section-arrow">{expandedSections.wells ? "▾" : "▸"}</span>
        <span class="section-title">Wells</span>
        <span class="section-count">{viewerModel.projectWellboreInventory.length}</span>
      </button>

      {#if expandedSections.wells}
        <div class="section-body">
          {#if viewerModel.projectWellOverlayInventoryError}
            <p class="status error">{viewerModel.projectWellOverlayInventoryError}</p>
          {:else if viewerModel.projectWellOverlayInventoryLoading}
            <p class="status">Loading project inventory...</p>
          {:else if viewerModel.projectWellboreInventory.length}
            {#if viewerModel.selectedProjectSurveyAsset}
              <p class="status">
                Survey {viewerModel.selectedProjectSurveyAsset.name} | {viewerModel.selectedProjectSurveyAsset.wellboreName}
              </p>
            {/if}
            {#if viewerModel.projectWellboreDisplayCompatibilitySummaryLine}
              <p class="status">{viewerModel.projectWellboreDisplayCompatibilitySummaryLine}</p>
            {/if}

            {#if viewerModel.projectSectionWellOverlayResolveBlocker}
              <p class="status error">{viewerModel.projectSectionWellOverlayResolveBlocker}</p>
            {/if}

            {#each viewerModel.projectWellboreSelectionGroups as group (group.label)}
              {#if viewerModel.projectWellboreSelectionGroups.length > 1}
                <div class="inventory-group-label">
                  <span>{group.label}</span>
                  <span>{group.wellbores.length}</span>
                </div>
              {/if}

              {#each group.wellbores as wellbore (wellbore.wellboreId)}
                <button
                  class:active={activeWellboreId === wellbore.wellboreId}
                  class="tree-item"
                  type="button"
                  onclick={() => viewerModel.setProjectWellboreId(wellbore.wellboreId)}
                  title={viewerModel.projectWellboreOptionLabel(wellbore)}
                >
                  <span class="item-icon">◫</span>
                  <span class="item-copy">
                    <span class="item-label">{wellbore.wellName} | {wellbore.wellboreName}</span>
                    <span class="item-subtitle">{viewerModel.projectWellboreStatusLabel(wellbore)}</span>
                  </span>
                </button>

                {#if activeWellboreId === wellbore.wellboreId}
                  <div class="child-list">
                    {#if viewerModel.projectWellTimeDepthObservationSets.length}
                      <div class="tree-group">
                        <div class="tree-group-label">
                          Observation Sets
                          <span>{viewerModel.projectWellTimeDepthObservationSets.length}</span>
                        </div>
                        {#each viewerModel.projectWellTimeDepthObservationSets as asset (asset.assetId)}
                          {#if asset.assetKind === "well_tie_observation_set"}
                            <button
                              class:active={viewerModel.selectedProjectWellTieObservationAssetId === asset.assetId}
                              class="tree-item child"
                              type="button"
                              onclick={() => viewerModel.resumeWellTieWorkbenchFromObservation(asset.assetId)}
                              title={`Resume ${asset.name}`}
                            >
                              <span class="item-icon">·</span>
                              <span class="item-copy">
                                <span class="item-label">{asset.name}</span>
                                <span class="item-subtitle">{wellTimeDepthObservationSubtitle(asset)}</span>
                              </span>
                            </button>
                          {:else}
                            <div class="tree-item static child">
                              <span class="item-icon">·</span>
                              <span class="item-copy">
                                <span class="item-label">{asset.name}</span>
                                <span class="item-subtitle">{wellTimeDepthObservationSubtitle(asset)}</span>
                              </span>
                            </div>
                          {/if}
                        {/each}
                      </div>
                    {/if}

                    {#if viewerModel.projectWellTimeDepthAuthoredModels.length}
                      <div class="tree-group">
                        <div class="tree-group-label">
                          Authored Models
                          <span>{viewerModel.projectWellTimeDepthAuthoredModels.length}</span>
                        </div>
                        {#each viewerModel.projectWellTimeDepthAuthoredModels as model (model.assetId)}
                          <div class="tree-item static child">
                            <span class="item-icon">·</span>
                            <span class="item-copy">
                              <span class="item-label">{model.name}</span>
                              <span class="item-subtitle">
                                {pluralize(model.sourceBindingCount, "source binding")} | {pluralize(model.assumptionIntervalCount, "interval assumption")}
                              </span>
                            </span>
                          </div>
                        {/each}
                      </div>
                    {/if}

                    {#if viewerModel.projectWellTimeDepthModels.length}
                      <div class="tree-group">
                        <div class="tree-group-label">
                          Compiled Models
                          <span>{viewerModel.projectWellTimeDepthModels.length}</span>
                        </div>
                        {#each viewerModel.projectWellTimeDepthModels as model (model.assetId)}
                          <button
                            class:active={viewerModel.selectedProjectWellTimeDepthModelAssetId === model.assetId}
                            class="tree-item child"
                            type="button"
                            onclick={() => viewerModel.setSelectedProjectWellTimeDepthModelAssetId(model.assetId)}
                            title={model.name}
                          >
                            <span class="item-icon">·</span>
                            <span class="item-copy">
                              <span class="item-label">{model.name}</span>
                              <span class="item-subtitle">
                                {velocitySourceKindLabel(model.sourceKind)} | {pluralize(model.sampleCount, "sample")}{model.isActiveProjectModel ? " | project active" : ""}
                              </span>
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
            <div class="section-empty">
              <p>No project well inventory loaded.</p>
              <p>Configure the project root and wellbore in <strong>TraceBoost &gt; Settings...</strong>.</p>
            </div>
          {/if}
        </div>
      {/if}
    </section>
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

  .tree-shell {
    min-height: 0;
    overflow: auto;
    padding: var(--sidebar-shell-padding);
    display: grid;
    gap: var(--ui-space-2);
    align-content: start;
    outline: none;
  }

  .tree-section {
    display: grid;
    gap: var(--sidebar-section-gap);
  }

  .section-header {
    display: grid;
    grid-template-columns: auto 1fr auto;
    align-items: center;
    gap: var(--ui-space-1);
    width: 100%;
    min-height: var(--sidebar-section-header-height);
    padding: 0 var(--ui-space-2);
    border: 1px solid var(--app-border);
    border-radius: var(--ui-radius-sm);
    background: var(--surface-bg);
    color: var(--text-primary);
    text-align: left;
    cursor: pointer;
    line-height: 1;
  }

  .section-arrow,
  .section-count {
    color: var(--text-muted);
  }

  .section-title {
    font-size: 11px;
    font-weight: 650;
    letter-spacing: 0.06em;
    text-transform: uppercase;
    line-height: 1;
  }

  .section-body {
    display: grid;
    gap: var(--ui-space-1);
  }

  .section-actions {
    display: grid;
    grid-template-columns: repeat(2, minmax(0, 1fr));
    gap: var(--ui-space-1);
  }

  .section-action {
    border: 1px solid var(--app-border);
    border-radius: var(--ui-radius-sm);
    background: var(--surface-bg);
    color: var(--text-primary);
    min-height: var(--sidebar-item-min-height);
    padding: 0 var(--ui-space-2);
    text-align: center;
    cursor: pointer;
  }

  .section-action:disabled {
    opacity: 0.55;
    cursor: default;
  }

  .section-empty {
    min-height: var(--sidebar-section-empty-min-height);
    padding: var(--ui-space-2) var(--ui-space-3);
    border: 1px dashed var(--app-border-strong);
    border-radius: var(--ui-radius-sm);
    background: #fff;
    color: var(--text-muted);
    align-content: center;
  }

  .section-empty p,
  .status {
    margin: 0;
  }

  .section-empty p + p {
    margin-top: 4px;
  }

  .status {
    padding: var(--ui-space-1) var(--ui-space-2);
    color: var(--text-muted);
  }

  .status.error {
    color: #a74646;
  }

  .tree-row {
    display: grid;
    grid-template-columns: minmax(0, 1fr) auto;
    gap: var(--ui-space-1);
  }

  .tree-item {
    min-width: 0;
    display: grid;
    grid-template-columns: auto minmax(0, 1fr);
    gap: var(--ui-space-1);
    align-items: center;
    min-height: var(--sidebar-item-min-height);
    padding: var(--ui-space-1) var(--ui-space-2);
    border: 1px solid var(--app-border);
    border-radius: var(--ui-radius-sm);
    background: #fff;
    color: var(--text-primary);
    text-align: left;
    cursor: pointer;
  }

  .tree-item.static {
    cursor: default;
  }

  .tree-item:hover:not(:disabled):not(.static) {
    border-color: var(--app-border-strong);
    background: var(--surface-bg);
  }

  .tree-item.active {
    border-color: #b0d4ee;
    background: #e8f3fb;
  }

  .tree-item.child {
    margin-left: var(--sidebar-item-indent);
  }

  .item-icon {
    color: #3a8cc2;
    line-height: 1.2;
  }

  .item-copy {
    min-width: 0;
    display: grid;
    gap: 2px;
    align-content: center;
  }

  .item-label {
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    line-height: 1.2;
  }

  .item-subtitle {
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    color: var(--text-muted);
    font-size: 10px;
    line-height: 1.2;
  }

  .item-remove {
    width: 22px;
    height: 22px;
    border: 1px solid var(--app-border);
    border-radius: var(--ui-radius-sm);
    background: var(--surface-subtle);
    color: var(--text-muted);
    cursor: pointer;
    font-size: 10px;
  }

  .child-list {
    display: grid;
    gap: var(--ui-space-1);
  }

  .tree-group {
    display: grid;
    gap: var(--ui-space-1);
  }

  .tree-group-label {
    margin: 2px 0 0 18px;
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: var(--ui-space-3);
    font-size: 10px;
    letter-spacing: 0.08em;
    text-transform: uppercase;
    color: var(--text-dim);
  }

  .tree-group-label span {
    color: var(--text-muted);
  }

  .inventory-group-label {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: var(--ui-space-3);
    padding: 0 var(--ui-space-1);
    font-size: 10px;
    letter-spacing: 0.08em;
    text-transform: uppercase;
    color: var(--text-dim);
  }

  .inventory-group-label span:last-child {
    color: var(--text-muted);
  }

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
