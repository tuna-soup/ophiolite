<svelte:options runes={true} />

<script lang="ts">
  import { WellTieChart } from "@ophiolite/charts/extras";
  import { createMockWellTieChartModel } from "@ophiolite/charts-data-models";
  import type { WellTieChartModel } from "@ophiolite/charts-data-models";
  import type { WellTieAnalysis1D, WellTieObservationSet1D } from "@ophiolite/contracts";
  import type { ProjectWellTieAnalysisResponse } from "../bridge";
  import { getViewerModelContext } from "../viewer-model.svelte";

  let { open = $bindable(false) }: { open?: boolean } = $props();

  const viewerModel = getViewerModelContext();

  let tieName = $state("Well Tie");
  let tieStartMsDraft = $state("1100");
  let tieEndMsDraft = $state("2200");
  let searchRadiusMDraft = $state("200");
  let activateAfterAccept = $state(true);
  let prepared = $state(false);
  let preparationMessage = $state<string | null>(null);
  let analysis = $state.raw<ProjectWellTieAnalysisResponse | null>(null);
  let analyzing = $state(false);
  let accepting = $state(false);
  let lastAppliedDraftSeedNonce = 0;

  const selectedWellbore = $derived(viewerModel.selectedProjectWellboreInventoryItem);
  const selectedTimeDepthModel = $derived(viewerModel.selectedProjectWellTimeDepthModel);
  const selectedObservation = $derived(viewerModel.selectedProjectWellTieObservationSet);
  const selectedSurvey = $derived(viewerModel.selectedProjectSurveyAsset);
  const selectedSurveyCompatibility = $derived(viewerModel.selectedProjectSurveyDisplayCompatibility);
  const selectedWellboreCompatibility = $derived(viewerModel.selectedProjectWellboreDisplayCompatibility);
  const draftSeed = $derived(viewerModel.projectWellTieDraftSeed);
  const draftSeedNonce = $derived(viewerModel.projectWellTieDraftSeedNonce);
  const prepareBlocker = $derived(viewerModel.projectWellTiePreparationBlocker);
  const analyzeBlocker = $derived(viewerModel.projectWellTieAnalysisBlocker);
  const acceptBlocker = $derived(viewerModel.projectWellTieAcceptBlocker);
  const compatibilityAdvisory = $derived(viewerModel.projectWellTieCompatibilityAdvisory);
  const gateMessage = $derived(prepareBlocker ?? analyzeBlocker ?? acceptBlocker);
  const tieWindow = $derived.by(() => {
    const parsedStart = Number(tieStartMsDraft);
    const parsedEnd = Number(tieEndMsDraft);
    const start = Number.isFinite(parsedStart) ? parsedStart : 1100;
    const end = Number.isFinite(parsedEnd) ? parsedEnd : 2200;
    return end > start + 40 ? { start, end } : { start, end: start + 400 };
  });
  const searchRadiusM = $derived.by(() => {
    const parsed = Number(searchRadiusMDraft);
    return Number.isFinite(parsed) && parsed > 0 ? parsed : 200;
  });
  const canPrepare = $derived(viewerModel.canPrepareProjectWellTie && !analyzing && !accepting);
  const canAnalyze = $derived(viewerModel.canAnalyzeProjectWellTie && !analyzing && !accepting);
  const canAccept = $derived(viewerModel.canAcceptProjectWellTie && !accepting);
  const previewModel = $derived.by<WellTieChartModel>(() => {
    if (analysis) {
      return createWellTieChartModel(analysis);
    }

    return createMockWellTieChartModel({
      id: "traceboost-well-tie-preview",
      name:
        (tieName.trim() || "Well Tie") +
        (selectedWellbore ? ` - ${selectedWellbore.wellboreName}` : ""),
      wellName: selectedWellbore?.wellName,
      timeStartMs: tieWindow.start,
      timeEndMs: tieWindow.end
    });
  });

  $effect(() => {
    if (!open || draftSeedNonce === lastAppliedDraftSeedNonce) {
      return;
    }
    lastAppliedDraftSeedNonce = draftSeedNonce;
    if (!draftSeed) {
      return;
    }
    tieName = draftSeed.tieName;
    tieStartMsDraft = draftSeed.tieStartMs;
    tieEndMsDraft = draftSeed.tieEndMs;
    searchRadiusMDraft = draftSeed.searchRadiusM;
    analysis = null;
    prepared = false;
    preparationMessage = draftSeed.summary;
  });

  function createWellTieChartModel(result: ProjectWellTieAnalysisResponse): WellTieChartModel {
    const { analysis } = result;
    const observationSet = result.draftObservationSet;
    const curveTimesMs = Float32Array.from(analysis.acoustic_impedance_curve.times_ms);
    const curveValues = Float32Array.from(analysis.acoustic_impedance_curve.values);
    const depthRangeM = resolveDepthRange(observationSet);
    const notes = [
      `Source model: ${result.sourceModelName}`,
      `Density: ${analysis.log_selection.density_curve.asset_name} / ${analysis.log_selection.density_curve.curve_name}`,
      `Velocity: ${analysis.log_selection.velocity_curve.asset_name} / ${analysis.log_selection.velocity_curve.curve_name}`,
      ...analysis.notes,
      ...observationSet.notes
    ];

    return {
      id: `traceboost-well-tie-${result.sourceModelAssetId}`,
      name:
        (tieName.trim() || "Well Tie") +
        (selectedWellbore ? ` - ${selectedWellbore.wellboreName}` : ""),
      timeRangeMs: {
        start: tieWindow.start,
        end: tieWindow.end,
        unit: "ms"
      },
      depthRangeM,
      tracks: [
        {
          kind: "curve",
          id: analysis.acoustic_impedance_curve.id,
          label: analysis.acoustic_impedance_curve.label,
          unit: analysis.acoustic_impedance_curve.unit ?? undefined,
          color: "#30455c",
          fillColor: "rgba(123, 172, 210, 0.16)",
          timesMs: curveTimesMs,
          values: curveValues,
          valueRange: resolveValueRange(curveValues)
        },
        createWiggleTrack(analysis.best_match_trace, "#213140"),
        createWiggleTrack(analysis.synthetic_trace, "#213140"),
        createWiggleTrack(analysis.well_trace, "#213140")
      ],
      metrics: buildMetrics(observationSet, analysis),
      section: {
        id: analysis.section_window.id,
        label: analysis.section_window.label,
        timesMs: Float32Array.from(analysis.section_window.times_ms),
        traceOffsetsM: Float32Array.from(analysis.section_window.trace_offsets_m),
        amplitudes: Float32Array.from(analysis.section_window.amplitudes),
        traceCount: analysis.section_window.trace_count,
        sampleCount: analysis.section_window.sample_count,
        wellTraceIndex: analysis.section_window.well_trace_index,
        matchTraceIndex: resolveMatchTraceIndex(
          analysis.section_window.trace_offsets_m,
          observationSet.trace_search_offset_m
        ),
        matchOffsetM: observationSet.trace_search_offset_m ?? undefined,
        wellLabel: "Well",
        matchLabel: "Best Match"
      },
      wavelet: {
        id: analysis.wavelet.id,
        label: analysis.wavelet.label,
        timesMs: Float32Array.from(analysis.wavelet.times_ms),
        amplitudes: Float32Array.from(analysis.wavelet.amplitudes),
        amplitudeRange: resolveValueRange(Float32Array.from(analysis.wavelet.amplitudes)),
        state: resolveWaveletState(analysis),
        detail: resolveWaveletDetail(analysis)
      },
      notes
    };
  }

  function createWiggleTrack(
    trace: WellTieAnalysis1D["best_match_trace"],
    lineColor: string
  ): WellTieChartModel["tracks"][number] {
    return {
      kind: "wiggle",
      id: trace.id,
      label: trace.label,
      timesMs: Float32Array.from(trace.times_ms),
      amplitudes: Float32Array.from(trace.amplitudes),
      lineColor,
      positiveFill: "rgba(221, 70, 61, 0.84)",
      negativeFill: "rgba(52, 89, 178, 0.82)",
      amplitudeScale: 1
    };
  }

  function resolveDepthRange(
    observationSet: WellTieObservationSet1D
  ): WellTieChartModel["depthRangeM"] {
    const first = observationSet.samples[0];
    const last = observationSet.samples[observationSet.samples.length - 1];
    if (!first || !last) {
      return undefined;
    }
    return { start: first.depth_m, end: last.depth_m };
  }

  function resolveValueRange(values: Float32Array): { min: number; max: number } | undefined {
    if (!values.length) {
      return undefined;
    }

    let min = Number.POSITIVE_INFINITY;
    let max = Number.NEGATIVE_INFINITY;
    for (const value of values) {
      if (value < min) {
        min = value;
      }
      if (value > max) {
        max = value;
      }
    }

    return Number.isFinite(min) && Number.isFinite(max) ? { min, max } : undefined;
  }

  function buildMetrics(
    observationSet: WellTieObservationSet1D,
    analysis: WellTieAnalysis1D
  ): WellTieChartModel["metrics"] {
    const stretch = observationSet.stretch_factor ?? null;
    const bulkShift = observationSet.bulk_shift_ms ?? null;
    const waveletState = resolveWaveletState(analysis);
    return [
      {
        id: "corr",
        label: "Corr",
        value: observationSet.correlation?.toFixed(2) ?? "n/a",
        emphasis: (observationSet.correlation ?? 0) >= 0.8 ? "good" : "neutral"
      },
      {
        id: "shift",
        label: "Bulk Shift",
        value:
          bulkShift !== null
            ? `${bulkShift.toFixed(0)} ms`
            : "n/a",
        emphasis: bulkShift !== null && Math.abs(bulkShift) >= 8 ? "warn" : "neutral"
      },
      {
        id: "stretch",
        label: "Stretch",
        value:
          stretch !== null
            ? `${stretch.toFixed(3)}x`
            : "n/a",
        emphasis: stretch !== null && Math.abs(stretch - 1) >= 0.015 ? "warn" : "neutral"
      },
      {
        id: "search",
        label: "Best Match",
        value:
          observationSet.trace_search_offset_m !== undefined &&
          observationSet.trace_search_offset_m !== null
            ? `${observationSet.trace_search_offset_m.toFixed(0)} m`
            : "0 m",
        emphasis: "neutral"
      },
      {
        id: "wavelet",
        label: "Wavelet",
        value: waveletState === "extracted" ? "Extracted" : "Provisional",
        emphasis: waveletState === "extracted" ? "good" : "warn"
      }
    ];
  }

  function resolveWaveletState(analysis: WellTieAnalysis1D): "provisional" | "extracted" {
    return analysis.wavelet.id === "extracted-wavelet" ? "extracted" : "provisional";
  }

  function resolveWaveletDetail(analysis: WellTieAnalysis1D): string {
    return resolveWaveletState(analysis) === "extracted"
      ? "Least-squares estimate from the matched seismic trace"
      : "Provisional seed wavelet pending stable extraction";
  }

  function resolveMatchTraceIndex(
    traceOffsetsM: readonly number[],
    matchOffsetM: number | null | undefined
  ): number | undefined {
    if (!traceOffsetsM.length || matchOffsetM === null || matchOffsetM === undefined) {
      return undefined;
    }
    let bestIndex = 0;
    let bestDistance = Number.POSITIVE_INFINITY;
    for (let index = 0; index < traceOffsetsM.length; index += 1) {
      const distance = Math.abs((traceOffsetsM[index] ?? 0) - matchOffsetM);
      if (distance < bestDistance) {
        bestDistance = distance;
        bestIndex = index;
      }
    }
    return bestIndex;
  }

  function closeWorkbench(): void {
    open = false;
    prepared = false;
    preparationMessage = null;
    analysis = null;
    analyzing = false;
    accepting = false;
    viewerModel.closeWellTieWorkbench();
  }

  function statusLabel(value: string | null | undefined, fallback = "Not set"): string {
    return value && value.trim() ? value : fallback;
  }

  function transformStatusLabel(status: string | null | undefined, fallback = "Not ready"): string {
    switch (status) {
      case "display_equivalent":
        return "Ready - native matches project CRS";
      case "display_transformed":
        return "Ready - reprojection available";
      case "display_degraded":
        return "Degraded - partial geometry";
      case "display_unavailable":
        return "Unavailable";
      case "native_only":
        return "Project CRS unresolved";
      default:
        return fallback;
    }
  }

  function handlePrepare(): void {
    if (prepareBlocker) {
      viewerModel.wellTieWorkbenchError =
        prepareBlocker;
      prepared = false;
      preparationMessage = null;
      return;
    }

    viewerModel.wellTieWorkbenchError = null;
    prepared = true;
    analysis = null;
    preparationMessage = `Prepared ${tieWindow.start.toFixed(0)}-${tieWindow.end.toFixed(0)} ms with a +/-${searchRadiusM.toFixed(0)} m trace search window.`;
    viewerModel.note(
      "Prepared a well-tie session from the active volume and selected project wellbore.",
      "ui",
      "info",
      selectedWellbore
        ? `${selectedWellbore.wellName} / ${selectedWellbore.wellboreName}`
        : "Selected project wellbore"
    );
  }

  async function handleAnalyze(): Promise<void> {
    if (analyzeBlocker) {
      viewerModel.wellTieWorkbenchError = analyzeBlocker;
      return;
    }

    const sourceModel = selectedTimeDepthModel;
    const survey = selectedSurvey;
    const storePath = viewerModel.activeStorePath;
    const displayCoordinateReferenceId = viewerModel.displayCoordinateReferenceId;
    if (!sourceModel || !survey || !storePath || !displayCoordinateReferenceId) {
      viewerModel.wellTieWorkbenchError = "Well-tie inputs are incomplete.";
      return;
    }

    analyzing = true;
    viewerModel.wellTieWorkbenchError = null;
    try {
      analysis = await viewerModel.analyzeProjectWellTie({
        projectRoot: viewerModel.projectRoot,
        sourceModelAssetId: sourceModel.assetId,
        tieName,
        tieStartMs: tieWindow.start,
        tieEndMs: tieWindow.end,
        searchRadiusM: searchRadiusM,
        storePath,
        surveyAssetId: survey.assetId,
        displayCoordinateReferenceId
      });
      prepared = true;
      preparationMessage = `Analyzed ${analysis.analysis.synthetic_trace.amplitudes.length} synthetic samples from '${analysis.sourceModelName}'.`;
    } catch (error) {
      viewerModel.wellTieWorkbenchError =
        error instanceof Error ? error.message : String(error);
    } finally {
      analyzing = false;
    }
  }

  async function handleAccept(): Promise<void> {
    if (acceptBlocker) {
      viewerModel.wellTieWorkbenchError = acceptBlocker;
      return;
    }

    const sourceModel = selectedTimeDepthModel;
    const survey = selectedSurvey;
    const wellbore = selectedWellbore;
    const storePath = viewerModel.activeStorePath;
    const displayCoordinateReferenceId = viewerModel.displayCoordinateReferenceId;
    if (
      !sourceModel ||
      !survey ||
      !wellbore ||
      !storePath ||
      !displayCoordinateReferenceId
    ) {
      viewerModel.wellTieWorkbenchError = "Well-tie inputs are incomplete.";
      return;
    }

    accepting = true;
    viewerModel.wellTieWorkbenchError = null;
    try {
      await viewerModel.acceptProjectWellTie({
        projectRoot: viewerModel.projectRoot,
        binding: {
          well_name: wellbore.wellName,
          wellbore_name: wellbore.wellboreName,
          operator_aliases: []
        },
        sourceModelAssetId: sourceModel.assetId,
        tieName,
        tieStartMs: tieWindow.start,
        tieEndMs: tieWindow.end,
        searchRadiusM: searchRadiusM,
        storePath,
        surveyAssetId: survey.assetId,
        displayCoordinateReferenceId,
        outputCollectionName: tieName.trim() || null,
        setActive: activateAfterAccept
      });
      closeWorkbench();
    } catch (error) {
      viewerModel.wellTieWorkbenchError =
        error instanceof Error ? error.message : String(error);
    } finally {
      accepting = false;
    }
  }
</script>

{#if open}
  <div class="workbench-backdrop" role="presentation" onclick={closeWorkbench}>
    <div
      class="workbench-dialog"
      role="dialog"
      aria-modal="true"
      aria-label="Well tie definition"
      tabindex="0"
      onclick={(event) => event.stopPropagation()}
      onkeydown={(event) => event.stopPropagation()}
    >
	      <div class="workbench-header">
	        <div>
	          <h3>Well Tie</h3>
	          <p>Single-well post-stack tie preparation with integrated AI, synthetic, survey-backed seismic, and wavelet preview.</p>
	        </div>
	        <button class="close-btn" type="button" onclick={closeWorkbench}>Close</button>
	      </div>

      <div class="workbench-layout">
        <section class="workbench-panel">
          <div class="field-grid">
            <label class="field">
              <span>Tie Name</span>
              <input bind:value={tieName} type="text" placeholder="Well Tie" />
            </label>

            <label class="field">
              <span>Tie Start (ms)</span>
              <input bind:value={tieStartMsDraft} type="number" step="10" />
            </label>

            <label class="field">
              <span>Tie End (ms)</span>
              <input bind:value={tieEndMsDraft} type="number" step="10" />
            </label>

            <label class="field">
              <span>Trace Search Radius (m)</span>
              <input bind:value={searchRadiusMDraft} type="number" min="0" step="10" />
            </label>

            <div class="field checkbox-field">
              <span>Activate Accepted Model</span>
              <label class="checkbox-toggle">
                <input bind:checked={activateAfterAccept} type="checkbox" />
                <strong>{activateAfterAccept ? "Yes" : "No"}</strong>
              </label>
            </div>
          </div>

          <div class="status-grid">
            <div class="status-row">
              <span>Active Volume</span>
              <strong>{statusLabel(viewerModel.activeStorePath)}</strong>
            </div>
            <div class="status-row">
              <span>Project Root</span>
              <strong>{statusLabel(viewerModel.projectRoot)}</strong>
            </div>
            <div class="status-row">
              <span>Project Display CRS</span>
              <strong>{statusLabel(viewerModel.displayCoordinateReferenceId, "Unresolved")}</strong>
            </div>
            <div class="status-row">
              <span>Selected Wellbore</span>
              <strong>
                {#if selectedWellbore}
                  {selectedWellbore.wellName} / {selectedWellbore.wellboreName}
                {:else}
                  Not selected
                {/if}
              </strong>
            </div>
            <div class="status-row">
              <span>Selected Well Time-Depth Model</span>
              <strong>{selectedTimeDepthModel ? selectedTimeDepthModel.name : "None selected"}</strong>
            </div>
            <div class="status-row">
              <span>Selected Survey</span>
              <strong>{selectedSurvey ? selectedSurvey.name : "None selected"}</strong>
            </div>
            <div class="status-row">
              <span>Survey Readiness</span>
              <strong>{transformStatusLabel(selectedSurveyCompatibility?.transformStatus)}</strong>
            </div>
            <div class="status-row">
              <span>Wellbore Readiness</span>
              <strong>{transformStatusLabel(selectedWellboreCompatibility?.transformStatus)}</strong>
            </div>
            <div class="status-row">
              <span>Required Logs</span>
              <strong>Density plus sonic or Vp</strong>
            </div>
          </div>

          <div class="workbench-chart">
            <WellTieChart model={previewModel} />
          </div>
        </section>

        <aside class="workbench-panel workbench-sidebar">
          <div class="sidebar-block">
            <h4>Current Scope</h4>
            <p class="sidebar-copy">Prepare a single-well tie window, derive impedance and synthetic traces from density plus sonic or Vp logs, and stage the accepted tie override.</p>
          </div>

          <div class="sidebar-block">
            <h4>Accepted Output</h4>
            <p class="sidebar-copy">Accept writes a well-tie observation set, an authored model that prefers that tie in-range, and a compiled model for this wellbore.</p>
          </div>

	          <div class="sidebar-block">
	            <h4>Current Matching</h4>
	            <p class="sidebar-copy">The tie now solves a local best-match trace, applies a conservative bulk shift plus affine stretch, and updates the synthetic with a least-squares extracted wavelet when the estimate is stable.</p>
	          </div>

          {#if compatibilityAdvisory}
            <div class="sidebar-block advisory-block">
              <h4>CRS Advisory</h4>
              <p class="sidebar-copy">{compatibilityAdvisory}</p>
            </div>
          {/if}

          {#if preparationMessage}
            <div class="sidebar-block prepared-block">
              <h4>Prepared Session</h4>
              <p class="sidebar-copy">{preparationMessage}</p>
            </div>
          {/if}

          {#if analysis}
            <div class="sidebar-block prepared-block">
              <h4>Draft Observation</h4>
              <p class="sidebar-copy">
                {analysis.draftObservationSet.samples.length} samples, source model
                <strong>{analysis.sourceModelName}</strong>.
              </p>
            </div>
          {/if}

          {#if selectedObservation}
            <div class="sidebar-block">
              <h4>Resumed Input</h4>
              <p class="sidebar-copy">
                {selectedObservation.name}
                {#if selectedObservation.tieWindowStartMs != null && selectedObservation.tieWindowEndMs != null}
                  <strong>
                    | {selectedObservation.tieWindowStartMs.toFixed(0)}-{selectedObservation.tieWindowEndMs.toFixed(0)} ms
                  </strong>
                {/if}
              </p>
            </div>
          {/if}

          {#if !selectedTimeDepthModel}
            <div class="sidebar-block advisory-block">
              <h4>Initial Model</h4>
              <p class="sidebar-copy">No compiled well time-depth model is selected yet. This backend slice currently seeds the tie from a selected compiled model before writing the accepted override.</p>
            </div>
          {/if}
        </aside>
      </div>

      {#if viewerModel.wellTieWorkbenchError}
        <p class="build-error">{viewerModel.wellTieWorkbenchError}</p>
      {:else if gateMessage}
        <p class="build-advisory">{gateMessage}</p>
      {/if}

      <div class="workbench-actions">
        <button class="secondary" type="button" onclick={closeWorkbench}>Cancel</button>
        <div class="action-group">
          <button type="button" class="secondary" onclick={handlePrepare} disabled={!canPrepare}>
            Prepare Session
          </button>
          <button type="button" onclick={() => void handleAnalyze()} disabled={!canAnalyze}>
            {analyzing ? "Analyzing..." : "Analyze Tie"}
          </button>
          <button
            type="button"
            onclick={() => void handleAccept()}
            disabled={!canAccept}
          >
            {accepting ? "Accepting..." : activateAfterAccept ? "Accept & Activate" : "Accept Tie"}
          </button>
        </div>
      </div>
    </div>
  </div>
{/if}

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
    width: min(1380px, calc(100vw - 40px));
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
  .sidebar-copy {
    margin: 6px 0 0;
    color: var(--text-muted);
    line-height: 1.45;
  }

  .workbench-layout {
    display: grid;
    grid-template-columns: minmax(0, 1.7fr) minmax(260px, 0.72fr);
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
    grid-template-columns: repeat(5, minmax(0, 1fr));
    gap: 12px;
    margin-bottom: 16px;
  }

  .field {
    display: grid;
    gap: 6px;
  }

  .field span {
    color: var(--text-muted);
    font-size: 12px;
    font-weight: 600;
    text-transform: uppercase;
  }

  .field input {
    min-width: 0;
    padding: 10px 12px;
    border: 1px solid var(--app-border);
    border-radius: 8px;
    background: var(--panel-bg);
    color: var(--text-primary);
    font: inherit;
  }

  .checkbox-field {
    align-content: start;
  }

  .checkbox-toggle {
    display: inline-flex;
    align-items: center;
    gap: 8px;
    color: var(--text-primary);
  }

  .status-grid {
    display: grid;
    grid-template-columns: repeat(2, minmax(0, 1fr));
    gap: 10px 14px;
    margin-bottom: 16px;
  }

  .status-row {
    display: grid;
    gap: 3px;
    padding: 10px 12px;
    border: 1px solid var(--app-border);
    border-radius: 8px;
    background: var(--panel-bg);
  }

  .status-row span {
    color: var(--text-muted);
    font-size: 11px;
    font-weight: 600;
    text-transform: uppercase;
  }

  .status-row strong {
    color: var(--text-primary);
    font-size: 13px;
    word-break: break-word;
  }

  .workbench-chart {
    min-width: 0;
  }

  .workbench-sidebar {
    display: grid;
    align-content: start;
    gap: 12px;
  }

  .sidebar-block {
    border: 1px solid var(--app-border);
    border-radius: 8px;
    background: var(--panel-bg);
    padding: 14px;
  }

  .prepared-block {
    background: rgba(222, 246, 234, 0.55);
  }

  .advisory-block {
    background: rgba(252, 244, 236, 0.7);
  }

  .build-error {
    margin: 0 20px;
    padding: 12px 14px;
    border-radius: 8px;
    background: rgba(212, 89, 72, 0.12);
    color: #a13f34;
  }

  .build-advisory {
    margin: 0 20px;
    padding: 12px 14px;
    border-radius: 8px;
    background: rgba(252, 244, 236, 0.88);
    color: #7a5634;
  }

  .action-group {
    display: flex;
    gap: 10px;
    flex-wrap: wrap;
  }

  .close-btn,
  button {
    border: 1px solid var(--app-border-strong);
    border-radius: 8px;
    background: var(--panel-bg);
    color: var(--text-primary);
    padding: 10px 14px;
    font: inherit;
    cursor: pointer;
  }

  button.secondary,
  .close-btn {
    background: var(--surface-bg);
  }

  button:disabled {
    opacity: 0.58;
    cursor: default;
  }

  @media (max-width: 1260px) {
    .field-grid {
      grid-template-columns: repeat(2, minmax(0, 1fr));
    }

    .workbench-layout {
      grid-template-columns: 1fr;
    }
  }

  @media (max-width: 760px) {
    .status-grid {
      grid-template-columns: 1fr;
    }

    .field-grid {
      grid-template-columns: 1fr;
    }

    .workbench-actions {
      align-items: stretch;
      flex-direction: column;
    }

    .action-group {
      width: 100%;
      justify-content: stretch;
    }

    .action-group button,
    .workbench-actions > button {
      width: 100%;
    }
  }
</style>
