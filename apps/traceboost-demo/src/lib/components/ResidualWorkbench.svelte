<svelte:options runes={true} />

<script lang="ts">
  import { getViewerModelContext } from "../viewer-model.svelte";

  let { open = $bindable(false) }: { open?: boolean } = $props();

  const viewerModel = getViewerModelContext();

  let selectedHorizonId = $state("");
  let selectedMarkerName = $state("");

  const availableHorizons = $derived(viewerModel.projectSurveyHorizonAssets);
  const availableMarkers = $derived(viewerModel.projectWellMarkers);
  const selectedSurvey = $derived(viewerModel.selectedProjectSurveyAsset);
  const selectedWellbore = $derived(viewerModel.selectedProjectWellboreInventoryItem);
  const selectedHorizon =
    $derived(availableHorizons.find((horizon) => horizon.id === selectedHorizonId) ?? null);
  const selectedMarker =
    $derived(availableMarkers.find((marker) => marker.name === selectedMarkerName) ?? null);
  const computeBlocker = $derived.by(() => {
    if (viewerModel.residualWorkbenchBlocker) {
      return viewerModel.residualWorkbenchBlocker;
    }
    if (!selectedHorizon) {
      return "Choose a project survey horizon.";
    }
    if (selectedHorizon.vertical_domain !== "depth") {
      return "Residual computation requires a depth-domain horizon.";
    }
    if (!selectedMarker) {
      return "Choose a canonical well marker.";
    }
    return null;
  });
  const canCompute = $derived(!viewerModel.residualWorkbenchWorking && !computeBlocker);

  $effect(() => {
    if (!open) {
      return;
    }

    const nextSelectedHorizonId =
      viewerModel.selectedProjectHorizonAsset?.id ??
      availableHorizons.find((horizon) => horizon.vertical_domain === "depth")?.id ??
      availableHorizons[0]?.id ??
      "";
    const nextSelectedMarkerName =
      viewerModel.selectedProjectWellMarker?.name ?? availableMarkers[0]?.name ?? "";

    selectedHorizonId = nextSelectedHorizonId;
    selectedMarkerName = nextSelectedMarkerName;

    if (nextSelectedHorizonId && viewerModel.selectedProjectHorizonAsset?.id !== nextSelectedHorizonId) {
      viewerModel.setSelectedProjectHorizonId(nextSelectedHorizonId);
    }

    if (
      nextSelectedMarkerName &&
      viewerModel.selectedProjectWellMarker?.name !== nextSelectedMarkerName
    ) {
      viewerModel.setSelectedProjectWellMarkerName(nextSelectedMarkerName);
    }
  });

  function closeWorkbench(): void {
    open = false;
    viewerModel.closeResidualWorkbench();
  }

  function horizonDomainLabel(domain: string, unit: string): string {
    return `${domain === "depth" ? "Depth" : domain === "time" ? "TWT" : domain} | ${unit}`;
  }

  async function handleCompute(): Promise<void> {
    if (!selectedSurvey || !selectedWellbore || !selectedHorizon || !selectedMarker) {
      return;
    }

    try {
      await viewerModel.computeProjectResidual({
        projectRoot: viewerModel.projectRoot,
        wellboreId: selectedWellbore.wellboreId,
        surveyAssetId: selectedSurvey.assetId,
        horizonId: selectedHorizon.id,
        markerName: selectedMarker.name,
        outputCollectionName: `${selectedHorizon.name} | ${selectedMarker.name} Residual`
      });
      closeWorkbench();
    } catch {
      // ViewerModel owns user-facing error state.
    }
  }
</script>

{#if open}
  <div class="workbench-backdrop" role="presentation" onclick={closeWorkbench}>
    <div
      class="workbench-dialog"
      role="dialog"
      aria-modal="true"
      aria-label="Residual computation"
      tabindex="0"
      onclick={(event) => event.stopPropagation()}
      onkeydown={(event) => event.stopPropagation()}
    >
      <div class="workbench-header">
        <div>
          <h3>Residuals</h3>
          <p>
            Compute a depth residual from a canonical well marker and a depth-domain survey horizon.
            The result is stored back into the project as a residual asset with map-ready point data.
          </p>
        </div>
        <button class="close-btn" type="button" onclick={closeWorkbench}>Close</button>
      </div>

      <div class="workbench-layout">
        <section class="workbench-panel">
          <div class="field-grid">
            <label class="field">
              <span>Project Survey</span>
              <input value={selectedSurvey ? selectedSurvey.name : ""} disabled />
            </label>

            <label class="field">
              <span>Wellbore</span>
              <input
                value={selectedWellbore ? `${selectedWellbore.wellName} | ${selectedWellbore.wellboreName}` : ""}
                disabled
              />
            </label>

            <label class="field">
              <span>Horizon</span>
              <select
                bind:value={selectedHorizonId}
                onchange={() => viewerModel.setSelectedProjectHorizonId(selectedHorizonId)}
              >
                {#each availableHorizons as horizon (horizon.id)}
                  <option value={horizon.id}>
                    {horizon.name} | {horizonDomainLabel(horizon.vertical_domain, horizon.vertical_unit)}
                  </option>
                {/each}
              </select>
            </label>

            <label class="field">
              <span>Canonical Marker</span>
              <select
                bind:value={selectedMarkerName}
                onchange={() => viewerModel.setSelectedProjectWellMarkerName(selectedMarkerName)}
              >
                {#each availableMarkers as marker (marker.name)}
                  <option value={marker.name}>
                    {marker.name}{marker.markerKind ? ` | ${marker.markerKind}` : ""}{marker.depthReference ? ` | ${marker.depthReference}` : ""}
                  </option>
                {/each}
              </select>
            </label>
          </div>

          <div class="conversion-note">
            <strong>Residual definition</strong>
            <p>Residual = marker TVD - horizon depth. Positive values mean the marker is deeper than the horizon.</p>
          </div>
        </section>

        <aside class="workbench-panel workbench-sidebar">
          <div class="sidebar-block">
            <h4>Selected Inputs</h4>
            {#if selectedHorizon}
              <div class="summary-row">
                <span>Horizon</span>
                <strong>{selectedHorizon.name}</strong>
              </div>
              <div class="summary-row">
                <span>Domain</span>
                <code>{horizonDomainLabel(selectedHorizon.vertical_domain, selectedHorizon.vertical_unit)}</code>
              </div>
            {/if}
            {#if selectedMarker}
              <div class="summary-row">
                <span>Marker</span>
                <strong>{selectedMarker.name}</strong>
              </div>
              <div class="summary-row">
                <span>Depth</span>
                <code>{selectedMarker.topDepth.toFixed(2)}{selectedMarker.depthReference ? ` ${selectedMarker.depthReference}` : ""}</code>
              </div>
            {/if}
          </div>

          <div class="sidebar-block">
            <h4>Write Target</h4>
            <p>
              The output is added to the selected wellbore as a residual asset and can be reused as
              canonical residual point data.
            </p>
          </div>

          {#if computeBlocker}
            <p class="status error">{computeBlocker}</p>
          {/if}
          {#if viewerModel.residualWorkbenchError}
            <p class="status error">{viewerModel.residualWorkbenchError}</p>
          {/if}

          <div class="action-row">
            <button class="secondary" type="button" onclick={closeWorkbench}>Cancel</button>
            <button class="primary" type="button" onclick={() => void handleCompute()} disabled={!canCompute}>
              {viewerModel.residualWorkbenchWorking ? "Computing..." : "Compute Residual"}
            </button>
          </div>
        </aside>
      </div>
    </div>
  </div>
{/if}

<style>
  .workbench-backdrop {
    position: fixed;
    inset: 0;
    background: rgba(16, 23, 30, 0.72);
    display: grid;
    place-items: center;
    z-index: 50;
    padding: 24px;
  }

  .workbench-dialog {
    width: min(980px, 100%);
    max-height: min(760px, calc(100vh - 48px));
    overflow: auto;
    background: #eef3f6;
    color: #102332;
    border: 1px solid rgba(28, 53, 70, 0.18);
    border-radius: 8px;
    box-shadow: 0 20px 48px rgba(8, 16, 24, 0.24);
  }

  .workbench-header {
    display: flex;
    align-items: start;
    justify-content: space-between;
    gap: 16px;
    padding: 20px 24px 16px;
    border-bottom: 1px solid rgba(28, 53, 70, 0.12);
  }

  .workbench-header h3 {
    margin: 0 0 6px;
    font-size: 1.15rem;
  }

  .workbench-header p {
    margin: 0;
    color: #3c5564;
    max-width: 64ch;
  }

  .close-btn,
  .secondary,
  .primary {
    border-radius: 6px;
    border: 1px solid rgba(28, 53, 70, 0.18);
    padding: 10px 14px;
    font: inherit;
    cursor: pointer;
  }

  .close-btn,
  .secondary {
    background: #f8fbfd;
    color: #173447;
  }

  .primary {
    background: #2f8f63;
    border-color: #2f8f63;
    color: #f4fffa;
  }

  .close-btn:disabled,
  .secondary:disabled,
  .primary:disabled {
    opacity: 0.55;
    cursor: not-allowed;
  }

  .workbench-layout {
    display: grid;
    grid-template-columns: minmax(0, 1.5fr) minmax(280px, 0.95fr);
    gap: 18px;
    padding: 20px 24px 24px;
  }

  .workbench-panel {
    display: grid;
    gap: 16px;
    align-content: start;
  }

  .field-grid {
    display: grid;
    grid-template-columns: repeat(2, minmax(0, 1fr));
    gap: 14px;
  }

  .field {
    display: grid;
    gap: 6px;
  }

  .field span {
    font-size: 0.84rem;
    color: #4a6271;
  }

  .field input,
  .field select {
    width: 100%;
    min-width: 0;
    border-radius: 6px;
    border: 1px solid rgba(28, 53, 70, 0.16);
    background: #fcfeff;
    color: #102332;
    padding: 10px 12px;
    font: inherit;
  }

  .workbench-sidebar {
    border-left: 1px solid rgba(28, 53, 70, 0.12);
    padding-left: 18px;
  }

  .sidebar-block {
    display: grid;
    gap: 10px;
  }

  .sidebar-block h4 {
    margin: 0;
    font-size: 0.92rem;
  }

  .summary-row {
    display: flex;
    justify-content: space-between;
    gap: 12px;
    align-items: start;
  }

  .summary-row span {
    color: #516b7b;
  }

  .summary-row strong,
  .summary-row code {
    text-align: right;
    overflow-wrap: anywhere;
  }

  .conversion-note {
    display: grid;
    gap: 6px;
    padding: 14px 16px;
    background: #f8fbfd;
    border: 1px solid rgba(28, 53, 70, 0.1);
    border-radius: 6px;
  }

  .conversion-note strong {
    font-size: 0.9rem;
  }

  .conversion-note p,
  .sidebar-block p {
    margin: 0;
    color: #3f5a69;
  }

  .status {
    margin: 0;
    padding: 10px 12px;
    border-radius: 6px;
    background: rgba(33, 54, 69, 0.08);
    color: #244154;
  }

  .status.error {
    background: rgba(188, 58, 58, 0.12);
    color: #7c2626;
  }

  .action-row {
    display: flex;
    justify-content: flex-end;
    gap: 10px;
    margin-top: 8px;
  }

  @media (max-width: 880px) {
    .workbench-layout {
      grid-template-columns: 1fr;
    }

    .workbench-sidebar {
      border-left: 0;
      border-top: 1px solid rgba(28, 53, 70, 0.12);
      padding-left: 0;
      padding-top: 18px;
    }

    .field-grid {
      grid-template-columns: 1fr;
    }
  }
</style>
