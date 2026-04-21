<svelte:options runes={true} />

<script lang="ts">
  import { onMount } from "svelte";
  import { emitFrontendDiagnosticsEvent } from "../bridge";
  import { getViewerModelContext } from "../viewer-model.svelte";

  const viewerModel = getViewerModelContext();

  let selectedHorizonId = $state("");
  let selectedTransformId = $state("");
  let outputIdDraft = $state("");
  let outputNameDraft = $state("");
  let outputIdManual = $state(false);
  let outputNameManual = $state(false);

  const availableHorizons = $derived(viewerModel.depthConversionHorizonAssets);
  const availableTransforms = $derived(viewerModel.availableVelocityModels);
  const selectedHorizon = $derived(
    availableHorizons.find((horizon) => horizon.id === selectedHorizonId) ?? null
  );
  const selectedTransform = $derived(
    availableTransforms.find((transform) => transform.id === selectedTransformId) ?? null
  );
  const sourceDomain = $derived(selectedHorizon?.vertical_domain ?? null);
  const targetDomain = $derived(
    sourceDomain === "time" ? "depth" : sourceDomain === "depth" ? "time" : null
  );
  const targetUnit = $derived(targetDomain === "depth" ? "m" : targetDomain === "time" ? "ms" : "");
  const recommendedOutputId = $derived(
    selectedHorizon && targetDomain
      ? `${selectedHorizon.id}-derived_${targetDomain === "depth" ? "depth_m" : "twt_ms"}`
      : ""
  );
  const recommendedOutputName = $derived(
    selectedHorizon && targetDomain
      ? `${selectedHorizon.name} Derived ${targetDomain === "depth" ? "Depth" : "TWT"}`
      : ""
  );
  const conversionBlocker = $derived.by(() => {
    if (viewerModel.depthConversionBlocker) {
      return viewerModel.depthConversionBlocker;
    }
    if (!selectedHorizon) {
      return "Choose a source horizon.";
    }
    if (!selectedTransform) {
      return "Choose a survey velocity model.";
    }
    if (!targetDomain) {
      return "Selected horizon does not declare a convertible vertical domain.";
    }
    return null;
  });
  const canConvert = $derived(!viewerModel.depthConversionWorkbenchWorking && !conversionBlocker);

  onMount(() => {
    void emitFrontendDiagnosticsEvent({
      stage: "depth_conversion_dialog",
      level: "debug",
      message: "Depth conversion dialog mounted."
    }).catch(() => {});
  });

  $effect(() => {
    if (!availableHorizons.some((horizon) => horizon.id === selectedHorizonId)) {
      selectedHorizonId = availableHorizons[0]?.id ?? "";
    }

    const preferredTransformId =
      viewerModel.activeVelocityModelDescriptor?.id ?? availableTransforms[0]?.id ?? "";
    if (!availableTransforms.some((transform) => transform.id === selectedTransformId)) {
      selectedTransformId = preferredTransformId;
    }

    if (!outputIdManual) {
      outputIdDraft = recommendedOutputId;
    }
    if (!outputNameManual) {
      outputNameDraft = recommendedOutputName;
    }
  });

  function resetDrafts(): void {
    selectedHorizonId = "";
    selectedTransformId = "";
    outputIdDraft = "";
    outputNameDraft = "";
    outputIdManual = false;
    outputNameManual = false;
  }

  function closeWorkbench(): void {
    resetDrafts();
    viewerModel.closeDepthConversionWorkbench();
  }

  async function handleConvert(): Promise<void> {
    if (!selectedHorizon || !selectedTransform || !targetDomain) {
      return;
    }

    try {
      await viewerModel.convertSurveyHorizonDomain({
        sourceHorizonId: selectedHorizon.id,
        transformId: selectedTransform.id,
        targetDomain,
        outputId: outputIdDraft.trim() || null,
        outputName: outputNameDraft.trim() || null
      });
      closeWorkbench();
    } catch {
      // ViewerModel owns the user-facing error state.
    }
  }

  function horizonDomainLabel(domain: string, unit: string): string {
    return `${domain === "time" ? "TWT" : domain === "depth" ? "Depth" : domain} | ${unit}`;
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
</script>

<div class="workbench-backdrop" role="presentation" onclick={closeWorkbench}>
  <div
    class="workbench-dialog"
    role="dialog"
    aria-modal="true"
    aria-label="Depth conversion"
    tabindex="0"
    onclick={(event) => event.stopPropagation()}
    onkeydown={(event) => event.stopPropagation()}
  >
    <div class="workbench-header">
      <div>
        <h3>Depth Conversion</h3>
        <p>
          Convert a stored survey horizon between TWT and depth using a selected survey velocity model.
          The output is written back into the active store as another horizon asset.
        </p>
      </div>
      <button class="close-btn" type="button" onclick={closeWorkbench}>Close</button>
    </div>

    <div class="workbench-layout">
      <section class="workbench-panel">
        <div class="field-grid">
          <label class="field">
            <span>Source Horizon</span>
            <select bind:value={selectedHorizonId}>
              {#each availableHorizons as horizon (horizon.id)}
                <option value={horizon.id}>
                  {horizon.name} | {horizonDomainLabel(horizon.vertical_domain, horizon.vertical_unit)}
                </option>
              {/each}
            </select>
          </label>

            <label class="field">
              <span>Velocity Model</span>
              <select bind:value={selectedTransformId}>
                {#each availableTransforms as transform (transform.id)}
                  <option value={transform.id}>
                    {transform.name} | {velocitySourceKindLabel(transform.source_kind)}
                  </option>
                {/each}
              </select>
            </label>

            <label class="field">
              <span>Source Domain</span>
              <input
                value={selectedHorizon ? horizonDomainLabel(selectedHorizon.vertical_domain, selectedHorizon.vertical_unit) : ""}
                disabled
              />
            </label>

            <label class="field">
              <span>Output Domain</span>
              <input
                value={targetDomain ? `${targetDomain === "depth" ? "Depth" : "TWT"} | ${targetUnit}` : ""}
                disabled
              />
            </label>

            <label class="field">
              <span>Output Name</span>
              <input
                bind:value={outputNameDraft}
                type="text"
                placeholder={recommendedOutputName || "Auto-generated if empty"}
                oninput={() => {
                  outputNameManual = true;
                }}
              />
            </label>

            <label class="field">
              <span>Output Id</span>
              <input
                bind:value={outputIdDraft}
                type="text"
                placeholder={recommendedOutputId || "Auto-generated if empty"}
                oninput={() => {
                  outputIdManual = true;
                }}
              />
            </label>
          </div>

          <div class="conversion-note">
            <strong>Write target</strong>
            <p>
              Converted horizons are stored in the active seismic volume store and will appear alongside the imported
              horizons.
            </p>
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
                <span>Source Id</span>
                <code>{selectedHorizon.id}</code>
              </div>
            {/if}
            {#if selectedTransform}
              <div class="summary-row">
                <span>Transform</span>
                <strong>{selectedTransform.name}</strong>
              </div>
              <div class="summary-row">
                <span>Coverage</span>
                <code>{selectedTransform.coverage.relationship}</code>
              </div>
            {/if}
          </div>

          <div class="sidebar-block">
            <h4>Behavior</h4>
            <p class="sidebar-copy">
              Source domain is taken from the stored horizon metadata. Output domain is the opposite vertical domain,
              and the chosen velocity model supplies the time-depth transform used for the conversion.
            </p>
          </div>
      </aside>
    </div>

    {#if conversionBlocker}
      <p class="build-error">{conversionBlocker}</p>
    {/if}

    {#if viewerModel.depthConversionWorkbenchError}
      <p class="build-error">{viewerModel.depthConversionWorkbenchError}</p>
    {/if}

    <div class="workbench-actions">
      <button class="secondary" type="button" onclick={closeWorkbench}>Cancel</button>
      <button type="button" disabled={!canConvert} onclick={() => void handleConvert()}>
        {viewerModel.depthConversionWorkbenchWorking ? "Converting..." : "Convert Horizon"}
      </button>
    </div>
  </div>
</div>

<style>
  .workbench-backdrop {
    position: fixed;
    inset: 0;
    z-index: 45;
    display: grid;
    place-items: center;
    background: rgb(38 55 71 / 0.2);
    backdrop-filter: blur(4px);
  }

  .workbench-dialog {
    width: min(1040px, calc(100vw - 40px));
    max-height: calc(100vh - 40px);
    overflow: auto;
    border: 1px solid var(--app-border);
    border-radius: 8px;
    background: var(--panel-bg);
    box-shadow: 0 24px 60px rgba(42, 64, 84, 0.22);
  }

  .workbench-header,
  .workbench-actions {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 16px;
    padding: 18px 20px;
    border-bottom: 1px solid var(--app-border);
  }

  .workbench-actions {
    border-top: 1px solid var(--app-border);
    border-bottom: 0;
  }

  .workbench-header h3,
  .workbench-sidebar h4 {
    margin: 0;
    color: var(--text-primary);
  }

  .workbench-header p,
  .sidebar-copy,
  .conversion-note p {
    margin: 6px 0 0;
    color: var(--text-muted);
    line-height: 1.45;
  }

  .workbench-layout {
    display: grid;
    grid-template-columns: minmax(0, 1.5fr) minmax(280px, 0.9fr);
    gap: 16px;
    padding: 20px;
  }

  .workbench-panel {
    border: 1px solid var(--app-border);
    border-radius: 8px;
    background: var(--surface-bg);
    padding: 16px;
  }

  .field-grid {
    display: grid;
    grid-template-columns: repeat(2, minmax(0, 1fr));
    gap: 14px;
  }

  .field {
    display: grid;
    gap: 8px;
  }

  .field span,
  .summary-row span {
    font-size: 0.82rem;
    font-weight: 600;
    color: var(--text-muted);
  }

  .field input,
  .field select {
    width: 100%;
    min-width: 0;
    border: 1px solid var(--app-border);
    border-radius: 8px;
    background: var(--panel-bg);
    color: var(--text-primary);
    padding: 10px 12px;
  }

  .field input:disabled {
    color: var(--text-muted);
  }

  .conversion-note,
  .sidebar-block {
    margin-top: 16px;
    padding-top: 16px;
    border-top: 1px solid var(--app-border);
  }

  .conversion-note strong {
    color: var(--text-primary);
  }

  .summary-row {
    display: grid;
    gap: 6px;
    margin-top: 10px;
  }

  .summary-row strong,
  .summary-row code {
    color: var(--text-primary);
    word-break: break-word;
  }

  .build-error {
    margin: 0 20px;
    padding: 12px 14px;
    border-radius: 8px;
    background: rgba(188, 67, 67, 0.12);
    color: #8f2b2b;
  }

  .close-btn,
  .workbench-actions button {
    border: 1px solid var(--app-border);
    border-radius: 8px;
    background: var(--surface-bg);
    color: var(--text-primary);
    padding: 10px 14px;
    cursor: pointer;
  }

  .workbench-actions button:not(.secondary) {
    background: var(--accent-bg);
    color: var(--accent-fg);
    border-color: transparent;
  }

  .workbench-actions button:disabled {
    opacity: 0.55;
    cursor: default;
  }

  @media (max-width: 900px) {
    .workbench-layout,
    .field-grid {
      grid-template-columns: 1fr;
    }
  }
</style>
