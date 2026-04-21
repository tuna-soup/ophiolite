<svelte:options runes={true} />

<script lang="ts">
  import type { ChartToolbarActionItem, ChartToolbarToolItem } from "@ophiolite/charts-toolbar";
  import { ChartInteractionToolbar } from "@ophiolite/charts-toolbar";
  import { SeismicSectionChart } from "@ophiolite/charts";
  import type { SegyGeometryCandidate, SegyHeaderField } from "@traceboost/seis-contracts";
  import PipelineControlBar from "./PipelineControlBar.svelte";
  import DepthConversionWorkbench from "./DepthConversionWorkbench.svelte";
  import PipelineOperatorEditor from "./PipelineOperatorEditor.svelte";
  import PipelineSequenceList from "./PipelineSequenceList.svelte";
  import PipelineSessionList from "./PipelineSessionList.svelte";
  import ResidualWorkbench from "./ResidualWorkbench.svelte";
  import SpectrumInspector from "./SpectrumInspector.svelte";
  import VelocityModelWorkbench from "./VelocityModelWorkbench.svelte";
  import WellTieWorkbench from "./WellTieWorkbench.svelte";
  import { getProcessingModelContext } from "../processing-model.svelte";
  import { getViewerModelContext } from "../viewer-model.svelte";

  let {
    showSidebar,
    showSidebarPanel,
    openSettings,
    requestHorizonImport,
    requestPetrelImport,
    chartRef = $bindable<{ fitToData?: () => void } | null>(null)
  }: {
    showSidebar: boolean;
    showSidebarPanel: () => void;
    openSettings: () => void;
    requestHorizonImport: () => Promise<void>;
    requestPetrelImport: () => Promise<void>;
    chartRef?: { fitToData?: () => void } | null;
  } = $props();

  const viewerModel = getViewerModelContext();
  const processingModel = getProcessingModelContext();
  let displaySettingsOpen = $state(false);
  let draftGain = $state(1);
  let draftClipMode = $state<"auto" | "manual">("auto");
  let draftClipMin = $state("");
  let draftClipMax = $state("");
  let draftColormap = $state<"grayscale" | "red-white-blue">("grayscale");
  let draftPolarity = $state<"normal" | "reversed">("normal");
  let sectionIndexDraft = $state<number | undefined>(undefined);
  let depthVelocityDraft = $state(String(viewerModel.depthVelocityMPerS));

  const compareViewport = $derived(viewerModel.displayedViewport);
  const chartSessionKey = $derived(`${viewerModel.displayedViewerSessionKey}:${processingModel.displaySectionMode}`);
  const displayedViewId = $derived(`${viewerModel.displayedViewId}:${processingModel.displaySectionMode}`);
  const geometryRecovery = $derived(viewerModel.importGeometryRecovery);
  const datasetExportDialog = $derived(viewerModel.datasetExportDialog);
  const tileStats = $derived(viewerModel.sectionTileStatsSnapshot);
  const splitReady = $derived(
    viewerModel.compareSplitEnabled &&
      !!processingModel.displaySection &&
      !!viewerModel.backgroundSection &&
      viewerModel.displayTransform.renderMode === "heatmap"
  );
  const shellCoordinateReferenceWarnings = $derived(
    viewerModel.workspaceCoordinateReferenceWarnings.slice(0, 3)
  );
  const shellCoordinateReferenceWarningOverflow = $derived(
    Math.max(0, viewerModel.workspaceCoordinateReferenceWarnings.length - shellCoordinateReferenceWarnings.length)
  );
  const sectionAxisLimit = $derived(
    viewerModel.dataset
      ? viewerModel.axis === "inline"
        ? Math.max(0, viewerModel.dataset.descriptor.shape[0] - 1)
        : Math.max(0, viewerModel.dataset.descriptor.shape[1] - 1)
      : 0
  );
  const tileWindow = $derived(resolveSectionTileWindow(viewerModel.section));
  const tileHitRate = $derived(
    tileStats.cacheHits + tileStats.fetches > 0 ? tileStats.cacheHits / (tileStats.cacheHits + tileStats.fetches) : null
  );
  const tileDiagnosticsStatus = $derived.by(() => {
    if (!viewerModel.activeStorePath.trim()) {
      return "No store";
    }
    if (!viewerModel.tauriRuntime) {
      return "Browser fallback";
    }
    if (!processingModel.displaySection || !viewerModel.section) {
      return "Waiting";
    }
    if (viewerModel.sectionDomain !== "time") {
      return "Depth mode";
    }
    if (viewerModel.compareSplitEnabled) {
      return "Compare split";
    }
    if (viewerModel.showVelocityOverlay) {
      return "Velocity overlay";
    }
    if (viewerModel.sectionScalarOverlays.length > 0) {
      return "Scalar overlays";
    }
    return "Active";
  });
  const tileDiagnosticsDetail = $derived.by(() => {
    switch (tileDiagnosticsStatus) {
      case "Browser fallback":
        return "Section tiles are not requested in browser mode.";
      case "Depth mode":
        return "Depth conversion still uses full-section payloads.";
      case "Compare split":
        return "Split compare currently pauses viewport tile orchestration.";
      case "Velocity overlay":
        return "Velocity overlays currently force full-section loads.";
      case "Scalar overlays":
        return "Scalar overlays currently force full-section loads.";
      case "Active":
        return "Viewport tiles with halo plus adjacent-slice prefetch.";
      case "Waiting":
        return "Load a section and move the viewport to start tile traffic.";
      default:
        return "Open a runtime store to enable section tiling diagnostics.";
    }
  });
  const toolbarTools = $derived<ChartToolbarToolItem[]>([
    {
      id: "pointer",
      label: "Pointer",
      icon: "pointer",
      active: viewerModel.chartTool === "pointer",
      disabled: !processingModel.displaySection
    },
    {
      id: "crosshair",
      label: "Crosshair",
      icon: "crosshair",
      active: viewerModel.chartTool === "crosshair",
      disabled: !processingModel.displaySection
    },
    {
      id: "pan",
      label: "Pan",
      icon: "pan",
      active: viewerModel.chartTool === "pan",
      disabled: !processingModel.displaySection
    }
  ]);
  const toolbarActions = $derived<ChartToolbarActionItem[]>([
    {
      id: "fitToData",
      label: "Fit To Data",
      icon: "fitToData",
      disabled: !processingModel.displaySection
    }
  ]);

  function describeSegyHeaderField(field: SegyHeaderField | null | undefined): string {
    if (!field) {
      return "unset";
    }
    return `${field.start_byte} (${field.value_type.toUpperCase()})`;
  }

  function describeSegyGeometryCandidate(candidate: SegyGeometryCandidate): string {
    return `inline ${describeSegyHeaderField(candidate.geometry.inline_3d)}, crossline ${describeSegyHeaderField(candidate.geometry.crossline_3d)}`;
  }

  function handleToolbarToolSelect(toolId: string): void {
    if (toolId === "pointer" || toolId === "crosshair" || toolId === "pan") {
      viewerModel.setChartTool(toolId);
    }
  }

  function handleToolbarActionSelect(actionId: string): void {
    if (actionId === "fitToData") {
      chartRef?.fitToData?.();
    }
  }

  function handleAxisChange(nextAxis: "inline" | "xline"): void {
    if (!viewerModel.activeStorePath || viewerModel.loading) {
      return;
    }

    const clampedIndex = Math.min(
      viewerModel.index,
      nextAxis === "inline"
        ? Math.max(0, (viewerModel.dataset?.descriptor.shape[0] ?? 1) - 1)
        : Math.max(0, (viewerModel.dataset?.descriptor.shape[1] ?? 1) - 1)
    );
    void viewerModel.load(nextAxis, clampedIndex);
  }

  function commitSectionIndex(): void {
    const sectionIndexInput = sectionIndexDraft ?? viewerModel.index;
    if (!viewerModel.activeStorePath || viewerModel.loading) {
      sectionIndexDraft = undefined;
      return;
    }

    if (!Number.isFinite(sectionIndexInput)) {
      sectionIndexDraft = undefined;
      return;
    }

    const clamped = Math.min(Math.max(Math.round(sectionIndexInput), 0), sectionAxisLimit);
    sectionIndexDraft = undefined;
    if (clamped !== viewerModel.index) {
      void viewerModel.load(viewerModel.axis, clamped);
    }
  }

  function toggleRenderMode(nextMode: "heatmap" | "wiggle"): void {
    viewerModel.setRenderMode(nextMode);
    if (viewerModel.compareSplitEnabled && nextMode !== "heatmap") {
      viewerModel.setCompareSplitEnabled(false);
    }
  }

  function toggleColormap(): void {
    viewerModel.setColormap(
      viewerModel.displayTransform.colormap === "grayscale" ? "red-white-blue" : "grayscale"
    );
  }

  function commitDepthVelocityModel(): void {
    const velocityMPerS = Number(depthVelocityDraft);
    if (!Number.isFinite(velocityMPerS) || velocityMPerS < 1) {
      depthVelocityDraft = String(viewerModel.depthVelocityMPerS);
      viewerModel.note("Depth conversion velocity must be a finite value >= 1 m/s.", "ui", "warn");
      return;
    }

    depthVelocityDraft = String(velocityMPerS);
    void viewerModel.setDepthVelocityMPerS(velocityMPerS);
  }

  function switchSectionDomain(domain: "time" | "depth"): void {
    if (domain === "depth" && !viewerModel.canDisplayDepthSection) {
      viewerModel.note(
        "Depth display currently requires the desktop runtime, an active volume, and a valid velocity model.",
        "ui",
        "warn"
      );
      return;
    }

    void viewerModel.setSectionDomain(domain);
  }

  function openDisplaySettings(): void {
    draftGain = viewerModel.displayTransform.gain;
    draftClipMode =
      typeof viewerModel.displayTransform.clipMin === "number" ||
      typeof viewerModel.displayTransform.clipMax === "number"
        ? "manual"
        : "auto";
    draftClipMin =
      typeof viewerModel.displayTransform.clipMin === "number"
        ? String(viewerModel.displayTransform.clipMin)
        : "";
    draftClipMax =
      typeof viewerModel.displayTransform.clipMax === "number"
        ? String(viewerModel.displayTransform.clipMax)
        : "";
    draftColormap = viewerModel.displayTransform.colormap;
    draftPolarity = viewerModel.displayTransform.polarity;
    displaySettingsOpen = true;
  }

  function closeDisplaySettings(): void {
    displaySettingsOpen = false;
  }

  function applyDisplaySettings(): void {
    const gain = Number(draftGain);
    if (Number.isFinite(gain) && gain > 0) {
      viewerModel.setGain(gain);
    }

    viewerModel.setColormap(draftColormap);
    viewerModel.setPolarity(draftPolarity);

    if (draftClipMode === "manual") {
      const clipMin = draftClipMin.trim() === "" ? undefined : Number(draftClipMin);
      const clipMax = draftClipMax.trim() === "" ? undefined : Number(draftClipMax);
      viewerModel.setClipRange(
        clipMin !== undefined && Number.isFinite(clipMin) ? clipMin : undefined,
        clipMax !== undefined && Number.isFinite(clipMax) ? clipMax : undefined
      );
    } else {
      viewerModel.setClipRange(undefined, undefined);
    }

    displaySettingsOpen = false;
  }

  function handleWindowKeyDown(event: KeyboardEvent): void {
    if (datasetExportDialog && event.key === "Escape") {
      viewerModel.closeDatasetExportDialog();
      return;
    }

    if (displaySettingsOpen && event.key === "Escape") {
      closeDisplaySettings();
      return;
    }

    if (processingModel.spectrumInspectorOpen && event.key === "Escape") {
      processingModel.closeSpectrumInspector();
      return;
    }

    if (displaySettingsOpen) {
      return;
    }

    void processingModel.handleKeydown(event);
  }

  function resolveSectionTileWindow(
    section: unknown
  ): { trace_start: number; trace_end: number; sample_start: number; sample_end: number; lod?: number } | null {
    if (!section || typeof section !== "object" || !("window" in section)) {
      return null;
    }
    const candidate = (section as { window?: unknown }).window;
    if (!candidate || typeof candidate !== "object") {
      return null;
    }
    if (
      "trace_start" in candidate &&
      "trace_end" in candidate &&
      "sample_start" in candidate &&
      "sample_end" in candidate
    ) {
      return candidate as {
        trace_start: number;
        trace_end: number;
        sample_start: number;
        sample_end: number;
        lod?: number;
      };
    }
    return null;
  }

  function formatIndexRange(start: number, end: number): string {
    return `[${start}, ${end})`;
  }

  function formatMiB(bytes: number): string {
    const mib = bytes / (1024 * 1024);
    return `${mib >= 10 ? mib.toFixed(1) : mib.toFixed(2)} MiB`;
  }

  function formatPercent(value: number | null): string {
    return value === null ? "n/a" : `${Math.round(value * 100)}%`;
  }
</script>

<svelte:window onkeydown={handleWindowKeyDown} />

{#snippet chartDisplayOverlay()}
  <div class="chart-display-overlay">
    <div class="display-chip-row">
      <label class="display-chip field">
        <span>{viewerModel.axis === "inline" ? "Inline" : "Xline"}</span>
        <select
          value={viewerModel.axis}
          disabled={!viewerModel.activeStorePath || viewerModel.loading}
          onchange={(event) => handleAxisChange((event.currentTarget as HTMLSelectElement).value as "inline" | "xline")}
        >
          <option value="inline">Inline</option>
          <option value="xline">Xline</option>
        </select>
      </label>

      <label class="display-chip field">
        <span>Index</span>
        <input
          bind:value={
            () => sectionIndexDraft ?? viewerModel.index,
            (value) => {
              sectionIndexDraft = value;
            }
          }
          type="number"
          min="0"
          max={sectionAxisLimit}
          disabled={!viewerModel.activeStorePath || viewerModel.loading}
          onblur={commitSectionIndex}
          onkeydown={(event) => {
            if (event.key === "Enter") {
              commitSectionIndex();
            }
          }}
        />
      </label>

      {#if viewerModel.datasetSampleDataFidelityLabel}
        <div
          class={[
            "display-chip field sample-fidelity-chip",
            viewerModel.datasetSampleDataFidelityNeedsWarning && "warn"
          ]}
          title={viewerModel.datasetSampleDataFidelityDetail ?? undefined}
        >
          <span>Samples</span>
          <strong>{viewerModel.datasetSampleDataFidelityLabel}</strong>
        </div>
      {/if}
    </div>

    <div class="display-chip-row">
      <button
        class:active={viewerModel.sectionHorizons.length > 0}
        class="display-chip action"
        onclick={() => void requestHorizonImport()}
        disabled={!viewerModel.activeStorePath || viewerModel.horizonImporting}
      >
        {viewerModel.horizonImporting ? "Importing…" : `Horizons ${viewerModel.sectionHorizons.length}`}
      </button>
      <button
        class="display-chip action"
        onclick={() => void requestPetrelImport()}
        disabled={!viewerModel.tauriRuntime}
      >
        Petrel...
      </button>
      {#if viewerModel.sectionWellOverlays.length > 0}
        <div class="display-chip field time-depth-status">
          <span>Wells</span>
          <strong>{viewerModel.sectionWellOverlays.length}</strong>
        </div>
      {/if}
      <button
        class="display-chip action"
        onclick={() => void viewerModel.openActiveDatasetExportDialog()}
        disabled={!viewerModel.canOpenExportDialog}
      >
        {viewerModel.datasetExporting ? "Exporting..." : "Export..."}
      </button>
      <button
        class:active={viewerModel.sectionDomain === "time"}
        class="display-chip action"
        onclick={() => switchSectionDomain("time")}
        disabled={!viewerModel.activeStorePath || viewerModel.loading}
      >
        TWT
      </button>
      <button
        class:active={viewerModel.sectionDomain === "depth"}
        class="display-chip action"
        onclick={() => switchSectionDomain("depth")}
        disabled={(!viewerModel.canDisplayDepthSection && viewerModel.sectionDomain !== "depth") || viewerModel.loading}
      >
        Depth
      </button>
      {#if viewerModel.timeDepthStatusLabel}
        <div
          class="display-chip field time-depth-status"
          title={viewerModel.timeDepthStatusDetail ?? undefined}
        >
          <span>Transform</span>
          <strong>{viewerModel.timeDepthStatusLabel}</strong>
        </div>
      {/if}
      {#if viewerModel.activeVelocityModelDescriptor}
        <div class="display-chip field time-depth-status" title={viewerModel.activeVelocityModelDescriptor.name}>
          <span>Velocity Model</span>
          <strong>{viewerModel.activeVelocityModelDescriptor.name}</strong>
        </div>
      {/if}
      <label class="display-chip field velocity-field">
        <span>{viewerModel.activeVelocityModelDescriptor ? "Fallback Vavg" : "Vavg"}</span>
        <input
          bind:value={depthVelocityDraft}
          type="number"
          min="1"
          step="50"
          disabled={!viewerModel.tauriRuntime || viewerModel.loading || !!viewerModel.activeVelocityModelDescriptor}
          onblur={commitDepthVelocityModel}
          onkeydown={(event) => {
            if (event.key === "Enter") {
              commitDepthVelocityModel();
            }
          }}
        />
        <small>m/s</small>
      </label>
      <button
        class:active={viewerModel.showVelocityOverlay}
        class="display-chip action"
        onclick={() => void viewerModel.setShowVelocityOverlay(!viewerModel.showVelocityOverlay)}
        disabled={!viewerModel.canDisplayVelocityOverlay || viewerModel.loading}
      >
        Velocity
      </button>
      <label class="display-chip field velocity-overlay-alpha">
        <span>Alpha</span>
        <input
          value={Math.round(viewerModel.velocityOverlayOpacity * 100)}
          type="range"
          min="0"
          max="100"
          step="1"
          disabled={!viewerModel.showVelocityOverlay || viewerModel.loading}
          oninput={(event) => {
            viewerModel.setVelocityOverlayOpacity(Number((event.currentTarget as HTMLInputElement).value) / 100);
          }}
        />
        <small>{Math.round(viewerModel.velocityOverlayOpacity * 100)}%</small>
      </label>
      <button
        class:active={viewerModel.displayTransform.renderMode === "heatmap"}
        class="display-chip action"
        onclick={() => toggleRenderMode("heatmap")}
        disabled={!processingModel.displaySection}
      >
        Heatmap
      </button>
      <button
        class:active={viewerModel.displayTransform.renderMode === "wiggle"}
        class="display-chip action"
        onclick={() => toggleRenderMode("wiggle")}
        disabled={!processingModel.displaySection}
      >
        Wiggle
      </button>
      <button
        class="display-chip action"
        onclick={toggleColormap}
        disabled={!processingModel.displaySection}
      >
        {viewerModel.displayTransform.colormap === "grayscale" ? "R/W/B" : "Gray"}
      </button>
      <button
        class:active={processingModel.spectrumInspectorOpen}
        class="display-chip icon"
        onclick={processingModel.openSpectrumInspector}
        aria-label="Open frequency spectrum inspector"
        disabled={!processingModel.canInspectSpectrum}
      >
        <svg viewBox="0 0 24 24" width="14" height="14" fill="none" stroke="currentColor" stroke-width="1.8">
          <path d="M4 18.5V14" />
          <path d="M9 18.5V8" />
          <path d="M14 18.5V11" />
          <path d="M19 18.5V5" />
          <path d="M3 18.5h18" />
        </svg>
      </button>
      <button
        class="display-chip icon"
        onclick={openDisplaySettings}
        aria-label="Open display settings"
        disabled={!processingModel.displaySection}
      >
        <svg viewBox="0 0 24 24" width="14" height="14" fill="none" stroke="currentColor" stroke-width="1.8">
          <path d="M10.3 2.5h3.4l.5 2.2a7.9 7.9 0 012 .8l1.9-1.2 2.4 2.4-1.2 1.9c.35.63.61 1.3.78 2l2.23.52v3.4l-2.23.52a7.9 7.9 0 01-.78 2l1.2 1.9-2.4 2.4-1.9-1.2a7.9 7.9 0 01-2 .78l-.52 2.23h-3.4l-.52-2.23a7.9 7.9 0 01-2-.78l-1.9 1.2-2.4-2.4 1.2-1.9a7.9 7.9 0 01-.78-2L2.5 13.7v-3.4l2.23-.52a7.9 7.9 0 01.78-2L4.26 5.9l2.4-2.4 1.9 1.2a7.9 7.9 0 012-.78z" />
          <circle cx="12" cy="12" r="3.1" />
        </svg>
      </button>
    </div>
  </div>
{/snippet}

{#snippet chartToolbarOverlay()}
  <div class="chart-toolbar-overlay">
    <ChartInteractionToolbar
      variant="overlay"
      iconOnly={true}
      tools={toolbarTools}
      actions={toolbarActions}
      onToolSelect={handleToolbarToolSelect}
      onActionSelect={handleToolbarActionSelect}
    />
  </div>
{/snippet}

{#snippet compareCycleOverlay()}
  <div class="compare-cycle-overlay">
    <button
      class="compare-arrow"
      onclick={() => void viewerModel.cycleForegroundCompareSurvey(-1)}
      aria-label="Show previous compatible survey"
      disabled={viewerModel.loading}
    >
      <svg viewBox="0 0 24 24" width="16" height="16" fill="none" stroke="currentColor" stroke-width="2">
        <path d="M12 5v14" />
        <path d="M7 10l5-5 5 5" />
      </svg>
    </button>
    <div class="compare-cycle-copy">
      <small>
        {viewerModel.compatibleCompareCandidates.findIndex(
          (candidate) => candidate.storePath === viewerModel.comparePrimaryStorePath
        ) + 1}
        / {viewerModel.compatibleCompareCandidates.length}
      </small>
    </div>
    <button
      class="compare-arrow"
      onclick={() => void viewerModel.cycleForegroundCompareSurvey(1)}
      aria-label="Show next compatible survey"
      disabled={viewerModel.loading}
    >
      <svg viewBox="0 0 24 24" width="16" height="16" fill="none" stroke="currentColor" stroke-width="2">
        <path d="M12 19V5" />
        <path d="M7 14l5 5 5-5" />
      </svg>
    </button>
  </div>
{/snippet}

{#snippet compareLabelOverlay()}
  <div class="compare-label-overlay">
    <div class="compare-label-line">
      <strong>{viewerModel.activeForegroundCompareCandidate?.displayName ?? viewerModel.activeDatasetDisplayName}</strong>
    </div>

    {#if viewerModel.activeBackgroundCompareCandidate}
      <div class="compare-label-line secondary">
        <strong>{viewerModel.activeBackgroundCompareCandidate.displayName}</strong>
      </div>
    {/if}
  </div>
{/snippet}

{#snippet tileDiagnosticsOverlay()}
  <div class="tile-diagnostics-overlay">
    <div class="tile-diagnostics-header">
      <span>Section Tiling</span>
      <strong class:active={tileDiagnosticsStatus === "Active"}>{tileDiagnosticsStatus}</strong>
    </div>
    <p>{tileDiagnosticsDetail}</p>
    {#if compareViewport}
      <dl class="tile-diagnostics-grid">
        <div>
          <dt>Viewport</dt>
          <dd>
            T {formatIndexRange(compareViewport.trace_start, compareViewport.trace_end)} ·
            S {formatIndexRange(compareViewport.sample_start, compareViewport.sample_end)}
          </dd>
        </div>
        <div>
          <dt>Loaded</dt>
          <dd>
            {#if tileWindow}
              T {formatIndexRange(tileWindow.trace_start, tileWindow.trace_end)} ·
              S {formatIndexRange(tileWindow.sample_start, tileWindow.sample_end)}
              {#if typeof tileWindow.lod === "number"}
                · LOD {tileWindow.lod}
              {/if}
            {:else}
              Full section
            {/if}
          </dd>
        </div>
        <div>
          <dt>Cache</dt>
          <dd>{formatMiB(tileStats.cachedBytes)} · {tileStats.evictions} evictions</dd>
        </div>
        <div>
          <dt>Reuse</dt>
          <dd>{tileStats.cacheHits} hits · {formatPercent(tileHitRate)}</dd>
        </div>
        <div>
          <dt>Fetch</dt>
          <dd>{tileStats.fetches} viewport · {tileStats.prefetchRequests} prefetch</dd>
        </div>
        <div>
          <dt>Errors</dt>
          <dd>{tileStats.fetchErrors + tileStats.prefetchErrors}</dd>
        </div>
      </dl>
    {/if}
  </div>
{/snippet}

{#if !showSidebar}
  <button class="sidebar-toggle" onclick={showSidebarPanel} aria-label="Show sidebar">
    <svg viewBox="0 0 24 24" width="20" height="20" fill="none" stroke="currentColor" stroke-width="2">
      <polyline points="9 18 15 12 9 6" />
    </svg>
  </button>
{/if}

<main class="viewer-shell">
  {#if shellCoordinateReferenceWarnings.length}
    <div class="crs-advisory-strip">
      <div class="crs-advisory-copy">
        <span>Coordinate Reference Alerts</span>
        {#each shellCoordinateReferenceWarnings as warning (warning)}
          <p>{warning}</p>
        {/each}
        {#if shellCoordinateReferenceWarningOverflow > 0}
          <p>
            {shellCoordinateReferenceWarningOverflow} more CRS alert{shellCoordinateReferenceWarningOverflow === 1 ? "" : "s"} in Project Settings.
          </p>
        {/if}
      </div>
      <button type="button" class="crs-advisory-action" onclick={() => openSettings()}>
        Project Settings
      </button>
    </div>
  {/if}

  <div class="workspace-columns">
    <aside class="session-column">
      <div class="session-column-header">
        <span class="eyebrow">Processing Workspace</span>
        <h2>{processingModel.pipelineTitle}</h2>
        <p>
          {viewerModel.dataset
            ? `Working on ${viewerModel.activeDatasetDisplayName} at ${viewerModel.axis}:${viewerModel.index}`
            : "Open a runtime store to preview processing on the current section."}
        </p>
      </div>

        <PipelineSessionList
          pipelines={processingModel.sessionPipelineItems}
          activePipelineId={processingModel.activeSessionPipelineId}
          onSelect={processingModel.activateSessionPipeline}
          onCreate={processingModel.createSessionPipeline}
          onDuplicate={processingModel.duplicateActiveSessionPipeline}
          onCopy={processingModel.copyActiveSessionPipeline}
          onPaste={processingModel.pasteCopiedSessionPipeline}
          onRemove={processingModel.removeActiveSessionPipeline}
          onRemoveItem={processingModel.removeSessionPipeline}
          getLabel={processingModel.sessionPipelineLabel}
          canRemove={processingModel.canRemoveSessionPipeline}
        />
    </aside>

    <div class="main-column">
      <div class="definition-pane">
        <PipelineControlBar
          pipeline={processingModel.pipeline}
          previewState={processingModel.previewState}
          previewLabel={processingModel.previewLabel}
          presets={processingModel.presets}
          loadingPresets={processingModel.loadingPresets}
          canPreview={processingModel.canPreview}
          canRun={processingModel.canRun}
          previewBusy={processingModel.previewBusy}
          runBusy={processingModel.runBusy}
          runOutputSettingsOpen={processingModel.runOutputSettingsOpen}
          runOutputPathMode={processingModel.runOutputPathMode}
          runOutputPath={processingModel.resolvedRunOutputPath}
          resolvingRunOutputPath={processingModel.resolvingRunOutputPath}
          overwriteExistingRunOutput={processingModel.overwriteExistingRunOutput}
          onSetPipelineName={processingModel.setPipelineName}
          onPreview={() => processingModel.previewCurrentSection()}
          onShowRaw={processingModel.showRawSection}
          onRun={() => processingModel.runOnVolume()}
          onToggleRunOutputSettings={() =>
            processingModel.setRunOutputSettingsOpen(!processingModel.runOutputSettingsOpen)}
          onSetRunOutputPathMode={processingModel.setRunOutputPathMode}
          onSetCustomRunOutputPath={processingModel.setCustomRunOutputPath}
          onBrowseRunOutputPath={() => processingModel.browseRunOutputPath()}
          onResetRunOutputPath={processingModel.resetRunOutputPath}
          onSetOverwriteExistingRunOutput={processingModel.setOverwriteExistingRunOutput}
          onLoadPreset={processingModel.loadPreset}
          onSavePreset={() => processingModel.savePreset()}
          onDeletePreset={(presetId) => processingModel.deletePreset(presetId)}
        />

        <div class="definition-grid">
          <PipelineSequenceList
            operations={processingModel.workspaceOperations}
            traceLocalOperationCount={processingModel.pipeline.steps.length}
            hasSubvolumeCrop={processingModel.hasSubvolumeCrop}
            selectedIndex={processingModel.selectedStepIndex}
            checkpointAfterOperationIndexes={processingModel.checkpointAfterOperationIndexes}
            checkpointWarning={processingModel.checkpointWarning}
            onSelect={processingModel.selectStep}
            onInsertOperator={processingModel.insertOperatorById}
            onCopy={processingModel.copySelectedOperation}
            onPaste={processingModel.pasteCopiedOperation}
            onRemove={processingModel.removeOperationAt}
            onToggleCheckpoint={processingModel.toggleCheckpointAfterOperation}
          />

          <PipelineOperatorEditor
            selectedOperation={processingModel.selectedOperation}
            activeJob={processingModel.activeJob}
            processingError={processingModel.error}
            primaryVolumeLabel={processingModel.activePrimaryVolumeLabel}
            sourceSubvolumeBounds={processingModel.sourceSubvolumeBounds}
            secondaryVolumeOptions={processingModel.volumeArithmeticSecondaryOptions}
            selectedStepCanCheckpoint={processingModel.canToggleSelectedCheckpoint}
            selectedStepCheckpoint={processingModel.selectedStepCheckpoint}
            onSetAmplitudeScalarFactor={processingModel.setSelectedAmplitudeScalarFactor}
            onSetAgcWindow={processingModel.setSelectedAgcWindow}
            onSetPhaseRotationAngle={processingModel.setSelectedPhaseRotationAngle}
            onSetLowpassCorner={processingModel.setSelectedLowpassCorner}
            onSetHighpassCorner={processingModel.setSelectedHighpassCorner}
            onSetBandpassCorner={processingModel.setSelectedBandpassCorner}
            onSetVolumeArithmeticOperator={processingModel.setSelectedVolumeArithmeticOperator}
            onSetVolumeArithmeticSecondaryStorePath={processingModel.setSelectedVolumeArithmeticSecondaryStorePath}
            onSetSubvolumeCropBound={processingModel.setSelectedSubvolumeCropBound}
            onSetSelectedCheckpoint={processingModel.setSelectedCheckpoint}
            canMoveUp={processingModel.canMoveSelectedUp}
            canMoveDown={processingModel.canMoveSelectedDown}
            onMoveUp={processingModel.moveSelectedUp}
            onMoveDown={processingModel.moveSelectedDown}
            onRemove={processingModel.removeSelected}
            onCancelJob={() => processingModel.cancelActiveJob()}
            onOpenArtifact={(storePath) => processingModel.openProcessingArtifact(storePath)}
          />
        </div>
      </div>

      <div class="viewer-pane">
      {#if processingModel.displaySection}
        <div class="chart-frame">
          {#key chartSessionKey}
          <SeismicSectionChart
            bind:this={chartRef}
            --ophiolite-chart-shell-bg="var(--panel-bg)"
            --ophiolite-chart-overlay-bg="rgba(244, 247, 249, 0.88)"
            --ophiolite-chart-overlay-text="#284052"
            --ophiolite-chart-probe-bg="rgba(255, 255, 255, 0.96)"
            --ophiolite-chart-probe-border="rgba(176, 212, 238, 0.78)"
            --ophiolite-chart-probe-text="#213746"
            --ophiolite-toolbar-bg="rgba(255, 255, 255, 0.94)"
            --ophiolite-toolbar-border="rgba(176, 212, 238, 0.72)"
            --ophiolite-toolbar-text="#35505f"
            --ophiolite-toolbar-hover-bg="#eef6fb"
            --ophiolite-toolbar-hover-text="#274b61"
            --ophiolite-toolbar-active-bg="#e8f3fb"
            --ophiolite-toolbar-active-text="#274b61"
            chartId={`traceboost-main:${chartSessionKey}`}
            viewId={displayedViewId}
            section={processingModel.displaySection}
            sectionScalarOverlays={viewerModel.sectionScalarOverlays}
            sectionHorizons={viewerModel.sectionHorizons}
            sectionWellOverlays={viewerModel.sectionWellOverlays}
            secondarySection={splitReady ? viewerModel.backgroundSection : null}
            compareMode={splitReady ? "split" : "single"}
            splitPosition={viewerModel.compareSplitPosition}
            viewport={compareViewport}
            displayTransform={viewerModel.displayTransform}
            interactions={{ tool: viewerModel.chartTool }}
            loading={viewerModel.loading || processingModel.previewBusy || (splitReady && viewerModel.backgroundLoading)}
            loadingMessage="Loading section..."
            errorMessage={viewerModel.error ?? (splitReady ? viewerModel.backgroundError : null)}
            resetToken={processingModel.displayResetToken}
            onProbeChange={viewerModel.setProbe}
            onViewportChange={viewerModel.setViewport}
            onInteractionChange={viewerModel.setInteraction}
            onInteractionStateChange={viewerModel.setInteractionState}
            onSplitPositionChange={(ratio) => viewerModel.setCompareSplitPosition(ratio)}
            stageScale={2}
            stageTopLeft={chartDisplayOverlay}
            plotTopCenter={chartToolbarOverlay}
            plotTopRight={viewerModel.canCycleForegroundCompareSurvey ? compareCycleOverlay : undefined}
            plotBottomRight={tileDiagnosticsOverlay}
            plotBottomLeft={compareLabelOverlay}
          />
          {/key}

          {#if processingModel.spectrumInspectorOpen}
            <div class="spectrum-inspector-layer">
              <SpectrumInspector
                floating={true}
                canInspectSpectrum={processingModel.canInspectSpectrum}
                spectrumBusy={processingModel.spectrumBusy}
                spectrumStale={processingModel.spectrumStale}
                spectrumError={processingModel.spectrumError}
                spectrumSelectionSummary={processingModel.spectrumSelectionSummary}
                spectrumAmplitudeScale={processingModel.spectrumAmplitudeScale}
                rawSpectrum={processingModel.rawSpectrum}
                processedSpectrum={processingModel.processedSpectrum}
                onSetSpectrumAmplitudeScale={processingModel.setSpectrumAmplitudeScale}
                onRefreshSpectrum={() => processingModel.refreshSpectrum()}
                onClose={processingModel.closeSpectrumInspector}
              />
            </div>
          {/if}
        </div>
      {:else}
        <div class="welcome-card">
          <svg
            class="welcome-icon"
            viewBox="0 0 24 24"
            width="64"
            height="64"
            fill="none"
            stroke="currentColor"
            stroke-width="1"
          >
            <path
              d="M3 20 L6 8 L9 14 L12 4 L15 16 L18 10 L21 20"
              stroke-linecap="round"
              stroke-linejoin="round"
            />
            <line x1="3" y1="20" x2="21" y2="20" />
          </svg>
          <h2>Open a Volume</h2>
          <p>
            Use <strong>File &gt; Open Volume…</strong> for runtime stores or <strong>File &gt; Import</strong>
            for seismic volumes, then start viewing and processing.
          </p>
          <span class="welcome-version">TraceBoost v0.1.0</span>
        </div>
      {/if}
      </div>
    </div>
  </div>
</main>

{#if displaySettingsOpen}
  <div
    class="display-settings-backdrop"
    role="presentation"
    onclick={closeDisplaySettings}
  >
    <div
      class="display-settings-dialog"
      role="dialog"
      aria-modal="true"
      aria-label="Display settings"
      tabindex="0"
      onclick={(event) => event.stopPropagation()}
      onkeydown={(event) => event.stopPropagation()}
    >
      <div class="display-settings-header">
        <h3>Display Settings</h3>
      </div>

      <div class="display-settings-grid">
        <label class="settings-field">
          <span>Gain</span>
          <input type="number" min="0.01" step="0.05" bind:value={draftGain} />
        </label>

        <label class="settings-field">
          <span>Color Scale</span>
          <select bind:value={draftColormap}>
            <option value="grayscale">Grayscale</option>
            <option value="red-white-blue">Red / White / Blue</option>
          </select>
        </label>

        <label class="settings-field">
          <span>Polarity</span>
          <select bind:value={draftPolarity}>
            <option value="normal">Normal</option>
            <option value="reversed">Reversed</option>
          </select>
        </label>

        <label class="settings-field">
          <span>Amplitude Range</span>
          <select bind:value={draftClipMode}>
            <option value="auto">Auto</option>
            <option value="manual">Manual</option>
          </select>
        </label>

        <label class="settings-field">
          <span>Minimum</span>
          <input type="number" step="0.01" bind:value={draftClipMin} disabled={draftClipMode !== "manual"} />
        </label>

        <label class="settings-field">
          <span>Maximum</span>
          <input type="number" step="0.01" bind:value={draftClipMax} disabled={draftClipMode !== "manual"} />
        </label>
      </div>

      <div class="display-settings-actions">
        <button class="settings-btn secondary" onclick={closeDisplaySettings}>Cancel</button>
        <button class="settings-btn primary" onclick={applyDisplaySettings}>Apply</button>
      </div>
    </div>
  </div>
{/if}

{#if datasetExportDialog}
  <div
    class="import-geometry-backdrop"
    role="presentation"
    onclick={() => viewerModel.closeDatasetExportDialog()}
  >
    <div
      class="import-geometry-dialog export-dialog"
      role="dialog"
      aria-modal="true"
      aria-label="Export dataset"
      tabindex="0"
      onclick={(event) => event.stopPropagation()}
      onkeydown={(event) => event.stopPropagation()}
    >
      <div class="import-geometry-header">
        <h3>Export Dataset</h3>
        <p>
          Choose one or more export formats, confirm the output path for each selection, then run the
          export for {datasetExportDialog.displayName}.
        </p>
      </div>

      <div class="import-geometry-summary">
        <div class="import-geometry-summary-row">
          <span>Selected dataset</span>
          <strong>{datasetExportDialog.displayName}</strong>
        </div>
        <div class="import-geometry-summary-row">
          <span>Runtime store</span>
          <strong>{datasetExportDialog.storePath}</strong>
        </div>
      </div>

      <div class="export-format-list">
        <label
          class:selected={datasetExportDialog.formats.segy.selected}
          class:disabled={!datasetExportDialog.formats.segy.available}
          class="import-geometry-candidate export-format-card"
        >
          <input
            type="checkbox"
            checked={datasetExportDialog.formats.segy.selected}
            disabled={!datasetExportDialog.formats.segy.available || datasetExportDialog.working}
            onchange={(event) =>
              viewerModel.setDatasetExportFormatSelected(
                "segy",
                (event.currentTarget as HTMLInputElement).checked
              )}
          />
          <div class="import-geometry-candidate-copy">
            <strong>SEG-Y</strong>
            <span>
              {#if datasetExportDialog.formats.segy.available}
                Export to SEG-Y using the captured survey provenance.
              {:else}
                {datasetExportDialog.formats.segy.reason ?? "SEG-Y export is unavailable for this dataset."}
              {/if}
            </span>
          </div>
        </label>

        <div class="import-geometry-manual export-path-card">
          <div class="import-geometry-manual-header">
            <span>SEG-Y Output Path</span>
            <small>Choose the `.sgy` or `.segy` file to write.</small>
          </div>
          <div class="export-path-row">
            <input
              type="text"
              value={datasetExportDialog.formats.segy.path}
              disabled={!datasetExportDialog.formats.segy.available || !datasetExportDialog.formats.segy.selected || datasetExportDialog.working}
              oninput={(event) =>
                viewerModel.setDatasetExportPath(
                  "segy",
                  (event.currentTarget as HTMLInputElement).value
                )}
            />
            <button
              class="settings-btn secondary"
              type="button"
              disabled={!datasetExportDialog.formats.segy.available || !datasetExportDialog.formats.segy.selected || datasetExportDialog.working}
              onclick={() => void viewerModel.browseDatasetExportPath("segy")}
            >
              Browse
            </button>
          </div>
        </div>

        <label
          class:selected={datasetExportDialog.formats.zarr.selected}
          class:disabled={!datasetExportDialog.formats.zarr.available}
          class="import-geometry-candidate export-format-card"
        >
          <input
            type="checkbox"
            checked={datasetExportDialog.formats.zarr.selected}
            disabled={!datasetExportDialog.formats.zarr.available || datasetExportDialog.working}
            onchange={(event) =>
              viewerModel.setDatasetExportFormatSelected(
                "zarr",
                (event.currentTarget as HTMLInputElement).checked
              )}
          />
          <div class="import-geometry-candidate-copy">
            <strong>Zarr</strong>
            <span>
              {#if datasetExportDialog.formats.zarr.available}
                Export to a chunked Zarr store with TraceBoost metadata.
              {:else}
                {datasetExportDialog.formats.zarr.reason ?? "Zarr export is unavailable for this dataset."}
              {/if}
            </span>
          </div>
        </label>

        <div class="import-geometry-manual export-path-card">
          <div class="import-geometry-manual-header">
            <span>Zarr Output Path</span>
            <small>Choose the `.zarr` store path to write.</small>
          </div>
          <div class="export-path-row">
            <input
              type="text"
              value={datasetExportDialog.formats.zarr.path}
              disabled={!datasetExportDialog.formats.zarr.available || !datasetExportDialog.formats.zarr.selected || datasetExportDialog.working}
              oninput={(event) =>
                viewerModel.setDatasetExportPath(
                  "zarr",
                  (event.currentTarget as HTMLInputElement).value
                )}
            />
            <button
              class="settings-btn secondary"
              type="button"
              disabled={!datasetExportDialog.formats.zarr.available || !datasetExportDialog.formats.zarr.selected || datasetExportDialog.working}
              onclick={() => void viewerModel.browseDatasetExportPath("zarr")}
            >
              Browse
            </button>
          </div>
        </div>
      </div>

      {#if datasetExportDialog.error}
        <p class="import-geometry-error">{datasetExportDialog.error}</p>
      {/if}

      <div class="import-geometry-actions">
        <button
          class="settings-btn secondary"
          type="button"
          onclick={() => viewerModel.closeDatasetExportDialog()}
          disabled={datasetExportDialog.working}
        >
          Cancel
        </button>
        <button
          class="settings-btn primary"
          type="button"
          onclick={() => void viewerModel.confirmDatasetExportDialog()}
          disabled={datasetExportDialog.working}
        >
          {datasetExportDialog.working ? "Exporting..." : "Export Selected Formats"}
        </button>
      </div>
    </div>
  </div>
{/if}

{#if geometryRecovery}
  <div class="import-geometry-backdrop" role="presentation" onclick={() => viewerModel.closeImportGeometryRecovery()}>
    <div
      class="import-geometry-dialog"
      role="dialog"
      aria-modal="true"
      aria-label="Review SEG-Y geometry mapping"
      tabindex="0"
      onclick={(event) => event.stopPropagation()}
      onkeydown={(event) => event.stopPropagation()}
    >
      <div class="import-geometry-header">
        <h3>Review SEG-Y Geometry Mapping</h3>
        <p>
          TraceBoost could not import this SEG-Y with the default inline/crossline mapping. Review a suggested
          mapping or enter the header bytes manually, then continue the import.
        </p>
      </div>

      <div class="import-geometry-summary">
        <div class="import-geometry-summary-row">
          <span>Resolved default mapping</span>
          <strong>
            inline {describeSegyHeaderField(geometryRecovery.preflight.resolved_geometry.inline_3d)} /
            crossline {describeSegyHeaderField(geometryRecovery.preflight.resolved_geometry.crossline_3d)}
          </strong>
        </div>
        <div class="import-geometry-summary-row">
          <span>Current result</span>
          <strong>{geometryRecovery.preflight.classification} • {geometryRecovery.preflight.layout}</strong>
        </div>
        <div class="import-geometry-summary-row">
          <span>Samples</span>
          <strong
            class={[
              viewerModel.preflightSampleDataFidelityNeedsWarning(geometryRecovery.preflight) && "warn"
            ]}
            title={viewerModel.preflightSampleDataFidelityDetail(geometryRecovery.preflight) ?? undefined}
          >
            {viewerModel.preflightSampleDataFidelityLabel(geometryRecovery.preflight) ?? "unknown"}
          </strong>
        </div>
      </div>

      {#if geometryRecovery.preflight.geometry_candidates.length}
        <div class="import-geometry-mode">
          <label class="import-geometry-mode-option">
            <input
              type="radio"
              name="geometry-mode"
              checked={geometryRecovery.mode === "candidate"}
              onchange={() => viewerModel.setImportGeometryRecoveryMode("candidate")}
            />
            <span>Use suggested mappings</span>
          </label>
          <label class="import-geometry-mode-option">
            <input
              type="radio"
              name="geometry-mode"
              checked={geometryRecovery.mode === "manual"}
              onchange={() => viewerModel.setImportGeometryRecoveryMode("manual")}
            />
            <span>Enter bytes manually</span>
          </label>
        </div>

        {#if geometryRecovery.mode === "candidate"}
          <div class="import-geometry-candidates">
            {#each geometryRecovery.preflight.geometry_candidates as candidate, candidateIndex (candidate.label)}
              <label
                class:selected={geometryRecovery.selectedCandidateIndex === candidateIndex}
                class="import-geometry-candidate"
              >
                <input
                  type="radio"
                  name="geometry-candidate"
                  checked={geometryRecovery.selectedCandidateIndex === candidateIndex}
                  onchange={() => viewerModel.selectImportGeometryCandidate(candidateIndex)}
                />
                <div class="import-geometry-candidate-copy">
                  <strong>{candidate.label}</strong>
                  <span>
                    {candidate.classification} • {candidate.inline_count} x {candidate.crossline_count}
                    {#if candidate.auto_selectable}
                      • recommended
                    {/if}
                  </span>
                  <code>{describeSegyGeometryCandidate(candidate)}</code>
                </div>
              </label>
            {/each}
          </div>
        {/if}
      {/if}

      <div class="import-geometry-manual">
        <div class="import-geometry-manual-header">
          <span>Manual Override</span>
          <small>Advanced: use this when the suggested list does not match the survey.</small>
        </div>

        <div class="import-geometry-grid">
          <label class="import-geometry-field">
            <span>Inline byte</span>
            <input
              type="number"
              min="1"
              value={geometryRecovery.draft.inlineByte}
              disabled={geometryRecovery.mode !== "manual" || geometryRecovery.working}
              oninput={(event) =>
                viewerModel.setImportGeometryRecoveryDraft(
                  "inlineByte",
                  (event.currentTarget as HTMLInputElement).value
                )}
            />
          </label>

          <label class="import-geometry-field">
            <span>Inline type</span>
            <select
              value={geometryRecovery.draft.inlineType}
              disabled={geometryRecovery.mode !== "manual" || geometryRecovery.working}
              onchange={(event) =>
                viewerModel.setImportGeometryRecoveryDraft(
                  "inlineType",
                  (event.currentTarget as HTMLSelectElement).value as "i16" | "i32"
                )}
            >
              <option value="i32">I32</option>
              <option value="i16">I16</option>
            </select>
          </label>

          <label class="import-geometry-field">
            <span>Crossline byte</span>
            <input
              type="number"
              min="1"
              value={geometryRecovery.draft.crosslineByte}
              disabled={geometryRecovery.mode !== "manual" || geometryRecovery.working}
              oninput={(event) =>
                viewerModel.setImportGeometryRecoveryDraft(
                  "crosslineByte",
                  (event.currentTarget as HTMLInputElement).value
                )}
            />
          </label>

          <label class="import-geometry-field">
            <span>Crossline type</span>
            <select
              value={geometryRecovery.draft.crosslineType}
              disabled={geometryRecovery.mode !== "manual" || geometryRecovery.working}
              onchange={(event) =>
                viewerModel.setImportGeometryRecoveryDraft(
                  "crosslineType",
                  (event.currentTarget as HTMLSelectElement).value as "i16" | "i32"
                )}
            >
              <option value="i32">I32</option>
              <option value="i16">I16</option>
            </select>
          </label>
        </div>
      </div>

      {#if geometryRecovery.error}
        <p class="import-geometry-error">{geometryRecovery.error}</p>
      {/if}

      <div class="import-geometry-actions">
        <button
          class="settings-btn secondary"
          onclick={() => viewerModel.closeImportGeometryRecovery()}
          disabled={geometryRecovery.working}
        >
          Cancel
        </button>
        <button
          class="settings-btn primary"
          onclick={() => viewerModel.confirmImportGeometryRecovery()}
          disabled={geometryRecovery.working}
        >
          {geometryRecovery.working ? "Validating Mapping..." : "Use Mapping And Import"}
        </button>
      </div>
    </div>
  </div>
{/if}

<VelocityModelWorkbench open={viewerModel.velocityModelWorkbenchOpen} />
<ResidualWorkbench open={viewerModel.residualWorkbenchOpen} />
{#if viewerModel.depthConversionWorkbenchOpen}
  <DepthConversionWorkbench />
{/if}
<WellTieWorkbench open={viewerModel.wellTieWorkbenchOpen} />

<style>
  .sidebar-toggle {
    position: fixed;
    left: 0;
    top: 50%;
    transform: translateY(-50%);
    z-index: 10;
    background: var(--panel-bg);
    border: 1px solid var(--app-border-strong);
    border-left: none;
    border-radius: 0 var(--ui-radius-md) var(--ui-radius-md) 0;
    padding: var(--ui-space-4) 5px;
    color: var(--text-muted);
    cursor: pointer;
  }

  .sidebar-toggle:hover {
    color: var(--text-primary);
    background: var(--surface-subtle);
  }

  .viewer-shell {
    min-height: 100vh;
    background: var(--app-bg);
    display: grid;
    grid-template-rows: auto minmax(0, 1fr);
  }

  .crs-advisory-strip {
    display: grid;
    grid-template-columns: minmax(0, 1fr) auto;
    gap: var(--ui-space-4);
    align-items: start;
    padding: var(--ui-space-4) var(--ui-space-5);
    border-bottom: 1px solid var(--app-border);
    background: rgba(252, 244, 236, 0.92);
  }

  .crs-advisory-copy {
    min-width: 0;
    display: grid;
    gap: 4px;
    color: #7a5634;
  }

  .crs-advisory-copy span {
    font-size: 11px;
    font-weight: 650;
    letter-spacing: 0.04em;
    text-transform: uppercase;
  }

  .crs-advisory-copy p {
    margin: 0;
    line-height: 1.35;
  }

  .crs-advisory-action {
    padding: 9px 12px;
    border: 1px solid var(--app-border-strong);
    border-radius: 8px;
    background: var(--panel-bg);
    color: var(--text-primary);
    font: inherit;
    cursor: pointer;
  }

  .workspace-columns {
    min-height: 100vh;
    display: grid;
    grid-template-columns: minmax(260px, 300px) minmax(0, 1fr);
  }

  .session-column {
    min-height: 0;
    display: grid;
    grid-template-rows: auto minmax(0, 1fr);
    gap: var(--ui-panel-gap);
    padding: var(--ui-panel-padding) var(--ui-panel-padding) var(--ui-space-5);
    border-right: 1px solid var(--app-border);
    background: var(--panel-bg);
  }

  .session-column-header {
    display: grid;
    gap: 2px;
  }

  .eyebrow {
    display: inline-block;
    font-size: 10px;
    letter-spacing: 0.1em;
    text-transform: uppercase;
    color: var(--text-dim);
  }

  .session-column-header h2 {
    margin: 0;
    font-size: 14px;
    font-weight: 600;
    color: var(--text-primary);
  }

  .session-column-header p {
    margin: 0;
    font-size: 11px;
    color: var(--text-muted);
    line-height: 1.45;
  }

  .main-column {
    min-height: 0;
    padding: var(--ui-panel-padding) var(--ui-space-6) var(--ui-space-6);
    display: grid;
    grid-template-rows: auto minmax(0, 1fr);
    gap: var(--ui-panel-gap);
  }

  .definition-pane {
    min-height: 0;
    display: grid;
    gap: var(--ui-panel-gap);
    position: relative;
    z-index: 8;
  }

  .definition-grid {
    min-height: 0;
    display: grid;
    grid-template-columns: minmax(320px, 0.95fr) minmax(420px, 1.25fr);
    gap: var(--ui-panel-gap);
    align-items: stretch;
  }

  .viewer-pane {
    min-height: 0;
    display: flex;
    flex-direction: column;
    gap: var(--ui-panel-gap);
    position: relative;
    z-index: 1;
  }

  .chart-frame {
    position: relative;
    flex: 1;
    min-height: 0;
    background: var(--panel-bg);
    border: 1px solid var(--app-border);
    border-radius: var(--ui-radius-lg);
    overflow: hidden;
  }

  .chart-display-overlay {
    display: grid;
    gap: var(--ui-space-2);
    justify-items: start;
  }

  .display-chip-row {
    display: flex;
    gap: var(--ui-space-2);
    flex-wrap: wrap;
  }

  .display-chip {
    display: inline-flex;
    align-items: center;
    gap: var(--ui-space-2);
    min-height: var(--ui-button-height);
    padding: 0 var(--ui-space-3);
    border: 1px solid rgba(176, 212, 238, 0.72);
    border-radius: var(--ui-radius-md);
    background: rgba(255, 255, 255, 0.92);
    color: var(--text-primary);
    box-shadow: var(--ui-shadow-soft);
    backdrop-filter: blur(6px);
  }

  .display-chip.field {
    padding-right: 6px;
  }

  .display-chip.field span {
    font-size: 10px;
    letter-spacing: 0.08em;
    text-transform: uppercase;
    color: var(--text-dim);
  }

  .display-chip.field select,
  .display-chip.field input {
    min-width: 56px;
    border: none;
    outline: none;
    background: transparent;
    color: var(--text-primary);
    font: inherit;
  }

  .display-chip.field input {
    width: 52px;
  }

  .velocity-field {
    gap: 4px;
  }

  .time-depth-status {
    gap: 4px;
  }

  .time-depth-status strong {
    font-size: 11px;
    font-weight: 600;
    letter-spacing: 0.03em;
    color: #274b61;
  }

  .sample-fidelity-chip strong {
    font-size: 11px;
    font-weight: 600;
    letter-spacing: 0.03em;
    color: #274b61;
  }

  .sample-fidelity-chip.warn strong {
    color: #705c1c;
  }

  .velocity-field small {
    font-size: 10px;
    letter-spacing: 0.04em;
    text-transform: uppercase;
    color: var(--text-dim);
  }

  .display-chip.action,
  .display-chip.icon {
    cursor: pointer;
  }

  .display-chip.action:hover:not(:disabled),
  .display-chip.icon:hover:not(:disabled) {
    background: #eef6fb;
    color: #274b61;
  }

  .display-chip.action.active {
    background: #e8f3fb;
    border-color: #9bc7e3;
    color: #274b61;
  }

  .display-chip.icon.active {
    background: #e8f3fb;
    border-color: #9bc7e3;
    color: #274b61;
  }

  .display-chip:disabled {
    opacity: 0.45;
    cursor: not-allowed;
  }

  .chart-toolbar-overlay {
    display: flex;
    justify-content: center;
  }

  .spectrum-inspector-layer {
    position: absolute;
    right: 14px;
    bottom: 16px;
    z-index: 6;
    pointer-events: none;
  }

  .chart-toolbar-overlay :global(.toolbar-group) {
    background: rgba(255, 255, 255, 0.94);
    border: 1px solid rgba(176, 212, 238, 0.72);
    box-shadow: var(--ui-shadow-soft);
    backdrop-filter: blur(10px);
  }

  .chart-toolbar-overlay :global(.toolbar-button) {
    color: var(--text-primary);
  }

  .chart-toolbar-overlay :global(.toolbar-button:hover:not(:disabled)) {
    background: #eef6fb;
    color: #274b61;
  }

  .chart-toolbar-overlay :global(.toolbar-button.active) {
    background: #e8f3fb;
    box-shadow: inset 0 0 0 1px rgba(155, 199, 227, 0.72);
    color: #274b61;
  }

  .viewer-shell :global(.ophiolite-charts-svelte-chart-shell) {
    height: 100%;
    width: 100%;
    border-radius: 0;
    overflow: hidden;
    border: 1px solid var(--app-border-strong);
    background: var(--panel-bg) !important;
  }

  .viewer-shell :global(.ophiolite-charts-svelte-chart-lane),
  .viewer-shell :global(.ophiolite-charts-svelte-chart-stage),
  .viewer-shell :global(.ophiolite-charts-svelte-chart-host) {
    background: var(--panel-bg) !important;
  }

  .viewer-shell :global(.ophiolite-charts-svelte-chart-host canvas) {
    background: transparent !important;
  }

  .compare-cycle-overlay {
    display: grid;
    grid-template-columns: auto minmax(0, 1fr) auto;
    gap: var(--ui-space-2);
    align-items: center;
    padding: 0;
  }

  .compare-arrow {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    width: var(--ui-icon-button-size);
    height: var(--ui-icon-button-size);
    border-radius: var(--ui-radius-md);
    border: 1px solid rgba(176, 212, 238, 0.72);
    background: rgba(255, 255, 255, 0.92);
    color: var(--text-primary);
    cursor: pointer;
  }

  .compare-arrow:hover:not(:disabled) {
    background: #eef6fb;
    color: #274b61;
  }

  .compare-arrow:disabled {
    opacity: 0.38;
    cursor: not-allowed;
  }

  .compare-cycle-copy {
    min-width: 0;
    display: grid;
    gap: 1px;
    text-align: center;
  }

  .compare-cycle-copy small {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .compare-cycle-copy small {
    font-size: 11px;
    color: var(--text-muted);
  }

  .compare-label-overlay {
    display: flex;
    gap: var(--ui-space-2);
    flex-wrap: wrap;
    pointer-events: none;
  }

  .tile-diagnostics-overlay {
    min-width: min(300px, calc(100vw - 48px));
    max-width: min(360px, calc(100vw - 48px));
    display: grid;
    gap: var(--ui-space-2);
    padding: 10px 12px;
    border: 1px solid rgba(176, 212, 238, 0.72);
    border-radius: var(--ui-radius-md);
    background: rgba(255, 255, 255, 0.94);
    box-shadow: var(--ui-shadow-soft);
    backdrop-filter: blur(6px);
  }

  .tile-diagnostics-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: var(--ui-space-3);
  }

  .tile-diagnostics-header span {
    font-size: 10px;
    letter-spacing: 0.08em;
    text-transform: uppercase;
    color: var(--text-dim);
  }

  .tile-diagnostics-header strong {
    font-size: 11px;
    font-weight: 650;
    color: #705c1c;
  }

  .tile-diagnostics-header strong.active {
    color: #274b61;
  }

  .tile-diagnostics-overlay p {
    margin: 0;
    font-size: 11px;
    line-height: 1.4;
    color: var(--text-muted);
  }

  .tile-diagnostics-grid {
    margin: 0;
    display: grid;
    grid-template-columns: repeat(2, minmax(0, 1fr));
    gap: var(--ui-space-2) var(--ui-space-4);
  }

  .tile-diagnostics-grid div {
    min-width: 0;
    display: grid;
    gap: 2px;
  }

  .tile-diagnostics-grid dt {
    font-size: 10px;
    letter-spacing: 0.06em;
    text-transform: uppercase;
    color: var(--text-dim);
  }

  .tile-diagnostics-grid dd {
    margin: 0;
    font-size: 11px;
    line-height: 1.35;
    color: var(--text-primary);
    word-break: break-word;
  }

  .compare-label-line {
    padding: var(--ui-space-1) var(--ui-space-3);
    border: 1px solid rgba(176, 212, 238, 0.72);
    border-radius: var(--ui-radius-md);
    background: rgba(255, 255, 255, 0.92);
    box-shadow: var(--ui-shadow-soft);
    backdrop-filter: blur(6px);
  }

  .compare-label-line strong {
    font-size: 12px;
    font-weight: 600;
    color: var(--text-primary);
  }

  .compare-label-line.secondary strong {
    color: #325472;
  }

  .welcome-card {
    text-align: center;
    max-width: 380px;
    padding: 32px 28px;
    background: var(--panel-bg);
    border: 1px solid var(--app-border);
    border-radius: var(--ui-radius-lg);
    margin: auto;
  }

  .welcome-icon {
    color: #7ea9c8;
    margin-bottom: 16px;
  }

  .welcome-card h2 {
    margin: 0 0 10px;
    font-size: 16px;
    font-weight: 600;
    color: var(--text-primary);
  }

  .welcome-card p {
    margin: 0 0 18px;
    font-size: 12px;
    line-height: 1.55;
    color: var(--text-muted);
  }

  .welcome-version {
    font-size: 11px;
    color: var(--text-dim);
  }

  .display-settings-backdrop {
    position: fixed;
    inset: 0;
    z-index: 30;
    display: flex;
    align-items: center;
    justify-content: center;
    background: rgba(38, 55, 71, 0.2);
    backdrop-filter: blur(6px);
  }

  .display-settings-dialog {
    width: min(520px, calc(100vw - 32px));
    padding: 16px;
    background: var(--panel-bg);
    border: 1px solid var(--app-border);
    border-radius: var(--ui-radius-lg);
    box-shadow: var(--ui-shadow-dialog);
  }

  .display-settings-header h3 {
    margin: 0 0 14px;
    font-size: 16px;
    color: var(--text-primary);
  }

  .display-settings-grid {
    display: grid;
    grid-template-columns: repeat(2, minmax(0, 1fr));
    gap: var(--ui-space-5);
  }

  .settings-field {
    display: grid;
    gap: var(--ui-space-2);
    font-size: 12px;
    color: var(--text-muted);
  }

  .settings-field input,
  .settings-field select {
    min-height: var(--ui-input-height);
    padding: 0 var(--ui-space-3);
    border: 1px solid var(--app-border-strong);
    background: #fff;
    color: var(--text-primary);
    border-radius: var(--ui-radius-md);
  }

  .settings-field input:disabled {
    opacity: 0.45;
  }

  .display-settings-actions {
    display: flex;
    justify-content: flex-end;
    gap: var(--ui-space-4);
    margin-top: 18px;
  }

  .settings-btn {
    min-width: 92px;
    min-height: var(--ui-input-height);
    padding: 0 14px;
    border: 1px solid var(--app-border-strong);
    border-radius: var(--ui-radius-md);
    cursor: pointer;
  }

  .settings-btn.secondary {
    background: var(--surface-subtle);
    color: var(--text-primary);
  }

  .settings-btn.primary {
    background: var(--accent-bg);
    color: var(--accent-text);
    border-color: var(--accent-border);
  }

  .import-geometry-backdrop {
    position: fixed;
    inset: 0;
    background: rgba(38, 55, 71, 0.2);
    backdrop-filter: blur(6px);
    display: flex;
    align-items: center;
    justify-content: center;
    padding: 24px;
    z-index: 30;
  }

  .import-geometry-dialog {
    width: min(720px, 100%);
    max-height: min(84vh, 820px);
    overflow: auto;
    background: var(--panel-bg);
    border: 1px solid var(--app-border);
    border-radius: var(--ui-radius-lg);
    box-shadow: var(--ui-shadow-dialog);
    padding: 22px;
    display: flex;
    flex-direction: column;
    gap: var(--ui-space-7);
  }

  .import-geometry-header h3 {
    margin: 0 0 8px;
    font-size: 18px;
    color: var(--text-primary);
  }

  .import-geometry-header p {
    margin: 0;
    color: var(--text-muted);
    line-height: 1.45;
  }

  .import-geometry-summary {
    display: grid;
    gap: var(--ui-space-4);
    padding: 12px 14px;
    border-radius: var(--ui-radius-lg);
    background: var(--surface-bg);
    border: 1px solid var(--app-border);
  }

  .import-geometry-summary-row {
    display: flex;
    justify-content: space-between;
    gap: 16px;
    align-items: baseline;
  }

  .import-geometry-summary-row span {
    color: var(--text-muted);
  }

  .import-geometry-summary-row strong {
    color: var(--text-primary);
    text-align: right;
  }

  .import-geometry-summary-row strong.warn {
    color: #ffd086;
  }

  .import-geometry-mode {
    display: flex;
    gap: 18px;
    flex-wrap: wrap;
  }

  .import-geometry-mode-option {
    display: inline-flex;
    align-items: center;
    gap: 8px;
    color: var(--text-primary);
  }

  .import-geometry-candidates {
    display: grid;
    gap: var(--ui-space-4);
  }

  .import-geometry-candidate {
    display: grid;
    grid-template-columns: auto 1fr;
    gap: var(--ui-space-5);
    align-items: start;
    padding: 12px 14px;
    border-radius: var(--ui-radius-lg);
    border: 1px solid var(--app-border);
    background: #fff;
    cursor: pointer;
  }

  .import-geometry-candidate.selected {
    border-color: #b0d4ee;
    background: #e8f3fb;
  }

  .import-geometry-candidate-copy {
    display: grid;
    gap: 4px;
  }

  .import-geometry-candidate-copy strong {
    color: var(--text-primary);
  }

  .import-geometry-candidate-copy span {
    color: var(--text-muted);
  }

  .import-geometry-candidate-copy code {
    color: #325472;
    font-size: 11px;
  }

  .import-geometry-manual {
    display: grid;
    gap: var(--ui-space-5);
    padding: 14px;
    border-radius: var(--ui-radius-lg);
    background: var(--surface-bg);
    border: 1px solid var(--app-border);
  }

  .import-geometry-manual-header {
    display: flex;
    justify-content: space-between;
    gap: 12px;
    align-items: baseline;
  }

  .import-geometry-manual-header span {
    color: var(--text-primary);
    font-weight: 600;
  }

  .import-geometry-manual-header small {
    color: var(--text-muted);
  }

  .import-geometry-grid {
    display: grid;
    grid-template-columns: repeat(2, minmax(0, 1fr));
    gap: var(--ui-space-5);
  }

  .import-geometry-field {
    display: grid;
    gap: var(--ui-space-2);
  }

  .import-geometry-field span {
    color: var(--text-dim);
  }

  .import-geometry-field input,
  .import-geometry-field select {
    width: 100%;
    min-height: var(--ui-input-height);
    border-radius: var(--ui-radius-md);
    border: 1px solid var(--app-border-strong);
    background: #fff;
    color: var(--text-primary);
    padding: 0 var(--ui-space-3);
  }

  .import-geometry-error {
    margin: 0;
    color: #ff9f9f;
  }

  .import-geometry-actions {
    display: flex;
    justify-content: flex-end;
    gap: var(--ui-space-4);
  }

  .export-dialog {
    border-color: rgba(107, 166, 206, 0.35);
  }

  .export-format-list {
    display: grid;
    gap: 12px;
  }

  .export-format-card.disabled {
    opacity: 0.62;
    cursor: not-allowed;
  }

  .export-path-card {
    gap: 10px;
  }

  .export-path-row {
    display: grid;
    grid-template-columns: minmax(0, 1fr) auto;
    gap: var(--ui-space-4);
    align-items: center;
  }

  .export-path-row input {
    width: 100%;
    min-height: var(--ui-input-height);
    border-radius: var(--ui-radius-md);
    border: 1px solid var(--app-border-strong);
    background: #fff;
    color: var(--text-primary);
    padding: 0 var(--ui-space-3);
  }

  @media (max-width: 900px) {
    .crs-advisory-strip {
      grid-template-columns: 1fr;
    }

    .workspace-columns {
      grid-template-columns: 1fr;
    }

    .session-column {
      grid-template-rows: auto minmax(220px, auto);
      border-right: none;
      border-bottom: 1px solid var(--app-border);
      padding-bottom: 10px;
    }

    .main-column {
      padding-inline: 10px;
      padding-bottom: 10px;
    }

    .definition-grid {
      grid-template-columns: 1fr;
    }

    .display-settings-grid {
      grid-template-columns: minmax(0, 1fr);
    }

    .tile-diagnostics-grid {
      grid-template-columns: 1fr;
    }

    .export-path-row {
      grid-template-columns: 1fr;
    }

    .import-geometry-grid {
      grid-template-columns: 1fr;
    }

    .import-geometry-summary-row,
    .import-geometry-manual-header {
      flex-direction: column;
      align-items: flex-start;
    }

    .import-geometry-summary-row strong {
      text-align: left;
    }
  }
</style>
