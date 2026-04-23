<svelte:options runes={true} />

<script lang="ts">
  import type { ProcessingWorkspaceFamily } from "../processing-model.svelte";
  import type {
    ProcessingBatchItemStatus,
    ProcessingBatchStatus,
    PostStackNeighborhoodProcessingPipeline,
    TraceLocalProcessingPipeline as ProcessingPipeline,
    ProcessingPreset as ProcessingPresetContract
  } from "@traceboost/seis-contracts";
  type PipelineLike = ProcessingPipeline | PostStackNeighborhoodProcessingPipeline;
  interface BatchCandidate {
    storePath: string;
    displayName: string;
    isActive: boolean;
  }

  let {
    processingFamily,
    pipeline,
    previewState,
    previewLabel,
    presets,
    loadingPresets,
    canPreview,
    canRun,
    previewBusy,
    runBusy,
    batchBusy,
    activeBatch,
    batchCandidates,
    selectedBatchStorePaths,
    batchExecutionMode,
    batchMaxActiveJobs,
    runOutputSettingsOpen,
    runOutputPathMode,
    runOutputPath,
    resolvingRunOutputPath,
    overwriteExistingRunOutput,
    onSetPipelineName,
    onPreview,
    onShowRaw,
    onRun,
    onRunBatch,
    onCancelBatch,
    onToggleRunOutputSettings,
    onSetRunOutputPathMode,
    onSetCustomRunOutputPath,
    onBrowseRunOutputPath,
    onResetRunOutputPath,
    onSetOverwriteExistingRunOutput,
    onToggleBatchStorePath,
    onSelectAllBatchCandidates,
    onClearBatchSelection,
    onSetBatchExecutionMode,
    onSetBatchMaxActiveJobs,
    onLoadPreset,
    onSavePreset,
    onDeletePreset
  }: {
    processingFamily: ProcessingWorkspaceFamily;
    pipeline: PipelineLike;
    previewState: "raw" | "preview" | "stale";
    previewLabel: string | null;
    presets: ProcessingPresetContract[];
    loadingPresets: boolean;
    canPreview: boolean;
    canRun: boolean;
    previewBusy: boolean;
    runBusy: boolean;
    batchBusy: boolean;
    activeBatch: ProcessingBatchStatus | null;
    batchCandidates: BatchCandidate[];
    selectedBatchStorePaths: string[];
    batchExecutionMode: "auto" | "conservative" | "throughput";
    batchMaxActiveJobs: string;
    runOutputSettingsOpen: boolean;
    runOutputPathMode: "default" | "custom";
    runOutputPath: string | null;
    resolvingRunOutputPath: boolean;
    overwriteExistingRunOutput: boolean;
    onSetPipelineName: (value: string) => void;
    onPreview: () => void | Promise<void>;
    onShowRaw: () => void;
    onRun: () => void | Promise<void>;
    onRunBatch: () => void | Promise<void>;
    onCancelBatch: () => void | Promise<void>;
    onToggleRunOutputSettings: () => void;
    onSetRunOutputPathMode: (mode: "default" | "custom") => void;
    onSetCustomRunOutputPath: (value: string) => void;
    onBrowseRunOutputPath: () => void | Promise<void>;
    onResetRunOutputPath: () => void;
    onSetOverwriteExistingRunOutput: (value: boolean) => void;
    onToggleBatchStorePath: (storePath: string) => void;
    onSelectAllBatchCandidates: () => void;
    onClearBatchSelection: () => void;
    onSetBatchExecutionMode: (mode: "auto" | "conservative" | "throughput") => void;
    onSetBatchMaxActiveJobs: (value: string) => void;
    onLoadPreset: (preset: ProcessingPresetContract) => void;
    onSavePreset: () => void | Promise<void>;
    onDeletePreset: (presetId: string) => void | Promise<void>;
  } = $props();

  let selectedPresetId = $state("");

  function normalizeTemplateId(value: string): string {
    return value
      .trim()
      .toLowerCase()
      .replace(/[^a-z0-9]+/g, "-")
      .replace(/^-+|-+$/g, "");
  }

  const currentLibraryTemplateId = $derived(
    normalizeTemplateId(pipeline.preset_id ?? pipeline.name ?? "")
  );
  const currentLibraryTemplateExists = $derived(
    !!currentLibraryTemplateId &&
      presets.some((preset) => preset.preset_id === currentLibraryTemplateId)
  );
  const saveLibraryButtonLabel = $derived(
    currentLibraryTemplateExists ? "Update Library Template" : "Save As Library Template"
  );
  const activeBatchItems = $derived(activeBatch?.items ?? []);
  const activeBatchFamily = $derived(
    activeBatch ? describeBatchFamily(activeBatch) : null
  );
  const batchQueuedCount = $derived(countBatchItems(activeBatchItems, "queued"));
  const batchRunningCount = $derived(countBatchItems(activeBatchItems, "running"));
  const batchCompletedCount = $derived(countBatchItems(activeBatchItems, "completed"));
  const batchFailedCount = $derived(countBatchItems(activeBatchItems, "failed"));
  const batchCancelledCount = $derived(countBatchItems(activeBatchItems, "cancelled"));

  function describeBatchFamily(batch: ProcessingBatchStatus): string {
    if ("trace_local" in batch.pipeline) {
      return "trace-local";
    }
    if ("subvolume" in batch.pipeline) {
      return "subvolume";
    }
    if ("post_stack_neighborhood" in batch.pipeline) {
      return "post-stack neighborhood";
    }
    return "gather";
  }

  function countBatchItems(
    items: ProcessingBatchItemStatus[],
    state: ProcessingBatchItemStatus["state"]
  ): number {
    return items.filter((item) => item.state === state).length;
  }

  function summarizeBatchItemPath(path: string): string {
    const normalized = path.replace(/\\/g, "/");
    const parts = normalized.split("/").filter((part) => part.length > 0);
    return parts.at(-1) ?? path;
  }

  function batchExecutionModeLabel(mode: string): string {
    switch (mode) {
      case "conservative":
        return "Conservative";
      case "throughput":
        return "Throughput";
      case "custom":
        return "Custom";
      default:
        return "Auto";
    }
  }

  function batchSchedulerReasonLabel(reason: string): string {
    switch (reason) {
      case "interactive_preview_policy":
        return "Preview-priority policy";
      case "foreground_materialize_policy":
        return "Foreground materialization policy";
      case "auto_low_cost_batch":
        return "Auto policy: low-cost workload";
      case "auto_medium_cost_batch":
        return "Auto policy: medium-cost workload";
      case "auto_high_cost_batch":
        return "Auto policy: high-cost workload";
      case "auto_full_volume_batch":
        return "Auto policy: full-volume workload";
      case "conservative_mode":
        return "Conservative policy";
      case "throughput_mode":
        return "Throughput policy";
      case "user_requested":
        return "Manual concurrency override";
      default:
        return reason.replaceAll("_", " ");
    }
  }

  function presetLabel(preset: ProcessingPresetContract): string {
    const spec = preset.pipeline;
    if ("trace_local" in spec) {
      return spec.trace_local.pipeline.name ?? preset.preset_id;
    }
    if ("subvolume" in spec) {
      return spec.subvolume.pipeline.name ?? preset.preset_id;
    }
    if ("post_stack_neighborhood" in spec) {
      return spec.post_stack_neighborhood.pipeline.name ?? preset.preset_id;
    }
    return spec.gather.pipeline.name ?? preset.preset_id;
  }
</script>

<section class="control-panel">
  <div class="control-header">
    <div>
      <h3>Pipeline Controls</h3>
      <p>
        {previewState === "preview"
          ? `Preview active: ${previewLabel ?? "processed"}`
          : previewState === "stale"
            ? "Preview stale"
            : "Showing raw section"}
      </p>
    </div>

    <div class="action-row">
      <button class="chip" onclick={onSavePreset}>{saveLibraryButtonLabel}</button>
      <button class="chip" onclick={onPreview} disabled={!canPreview || previewBusy}>
        {previewBusy ? "Previewing..." : "Preview"}
      </button>
      <button class="chip" onclick={onToggleRunOutputSettings} disabled={!canRun || runBusy}>
        {runOutputSettingsOpen ? "Hide Output" : "Output Settings"}
      </button>
      <button class="chip primary" onclick={onRun} disabled={!canRun || runBusy}>
        {runBusy ? "Running..." : "Run Volume"}
      </button>
    </div>
  </div>

  <div class="control-grid">
    <label class="field">
      <span>{processingFamily === "trace_local" ? "Pipeline Name" : "Neighborhood Name"}</span>
      <input
        type="text"
        value={pipeline.name ?? ""}
        placeholder="Untitled pipeline"
        oninput={(event) => onSetPipelineName((event.currentTarget as HTMLInputElement).value)}
      />
    </label>

    <div class="library-row">
      <select bind:value={selectedPresetId} disabled={loadingPresets || !presets.length}>
        <option value="">Apply library template...</option>
        {#each presets as preset (preset.preset_id)}
          <option value={preset.preset_id}>{presetLabel(preset)}</option>
        {/each}
      </select>
      <button
        class="chip"
        disabled={!selectedPresetId}
        onclick={() => {
          const preset = presets.find((candidate) => candidate.preset_id === selectedPresetId);
          if (preset) onLoadPreset(preset);
        }}
      >
        Apply
      </button>
      <button class="chip danger" disabled={!selectedPresetId} onclick={() => onDeletePreset(selectedPresetId)}>
        Delete Template
      </button>
      <button class="chip" disabled={previewState === "raw"} onclick={onShowRaw}>Show Raw</button>
    </div>
  </div>

  {#if runOutputSettingsOpen}
    <section class="output-settings">
      <div class="output-settings-header">
        <strong>Volume Output</strong>
        <span>{runOutputPathMode === "default" ? "Managed default" : "Custom path"}</span>
      </div>

      <div class="mode-row">
        <button
          class:active={runOutputPathMode === "default"}
          class="chip"
          onclick={() => onSetRunOutputPathMode("default")}
          disabled={runBusy}
        >
          Default
        </button>
        <button
          class:active={runOutputPathMode === "custom"}
          class="chip"
          onclick={() => onSetRunOutputPathMode("custom")}
          disabled={runBusy}
        >
          Custom
        </button>
      </div>

      <label class="field">
        <span>Output Store Path</span>
        <div class="path-row">
          <input
            type="text"
            value={runOutputPath ?? ""}
            placeholder={resolvingRunOutputPath ? "Resolving managed output path..." : "No output path selected"}
            readonly={runOutputPathMode === "default"}
            oninput={(event) => onSetCustomRunOutputPath((event.currentTarget as HTMLInputElement).value)}
          />
          <button class="chip" onclick={onBrowseRunOutputPath} disabled={runBusy}>
            Browse
          </button>
          <button class="chip" onclick={onResetRunOutputPath} disabled={runBusy || runOutputPathMode === "default"}>
            Reset
          </button>
        </div>
        <small>
          {#if runOutputPathMode === "default"}
            TraceBoost writes a unique derived `.tbvol` into its managed output library.
          {:else}
            Use a custom `.tbvol` path when you need to control naming or overwrite an existing store.
          {/if}
        </small>
      </label>

      <label class="checkbox-row">
        <input
          type="checkbox"
          checked={overwriteExistingRunOutput}
          onchange={(event) => onSetOverwriteExistingRunOutput((event.currentTarget as HTMLInputElement).checked)}
        />
        <span>Allow overwrite if the output store already exists</span>
      </label>

      {#if batchCandidates.length > 0}
        <section class="batch-settings">
          <div class="output-settings-header">
            <strong>Batch Run</strong>
            <span>{selectedBatchStorePaths.length} / {batchCandidates.length} selected</span>
          </div>

          <small>
            Apply the current workspace pipeline across compatible workspace datasets. Batch runs use
            managed derived output paths for the active family.
          </small>

          <div class="action-row">
            <button class="chip" onclick={onSelectAllBatchCandidates} disabled={batchBusy}>
              Select All
            </button>
            <button class="chip" onclick={onClearBatchSelection} disabled={batchBusy}>
              Clear
            </button>
          </div>

          <div class="batch-candidate-list">
            {#each batchCandidates as candidate (candidate.storePath)}
              <label class="batch-candidate">
                <input
                  type="checkbox"
                  checked={selectedBatchStorePaths.includes(candidate.storePath)}
                  onchange={() => onToggleBatchStorePath(candidate.storePath)}
                  disabled={batchBusy}
                />
                <span class="batch-candidate-label">{candidate.displayName}</span>
                {#if candidate.isActive}
                  <span class="batch-candidate-badge">Active</span>
                {/if}
              </label>
            {/each}
          </div>

          <div class="batch-policy-row">
            <label class="field">
              <span>Scheduler Mode</span>
              <select
                value={batchExecutionMode}
                onchange={(event) =>
                  onSetBatchExecutionMode(
                    (event.currentTarget as HTMLSelectElement).value as
                      | "auto"
                      | "conservative"
                      | "throughput"
                  )}
                disabled={batchBusy}
              >
                <option value="auto">Auto</option>
                <option value="conservative">Conservative</option>
                <option value="throughput">Throughput</option>
              </select>
            </label>

            <label class="field batch-concurrency">
              <span>Max Active Jobs</span>
              <input
                type="text"
                inputmode="numeric"
                value={batchMaxActiveJobs}
                placeholder="Auto"
                oninput={(event) => onSetBatchMaxActiveJobs((event.currentTarget as HTMLInputElement).value)}
                disabled={batchBusy}
              />
            </label>
          </div>

          <div class="action-row">
            <button
              class="chip primary"
              onclick={onRunBatch}
              disabled={batchBusy || !selectedBatchStorePaths.length}
            >
              {batchBusy ? "Batch Running..." : "Run Batch"}
            </button>
            {#if activeBatch}
              <button
                class="chip"
                onclick={onCancelBatch}
                disabled={activeBatch.state !== "queued" && activeBatch.state !== "running"}
              >
                Cancel Batch
              </button>
            {/if}
          </div>

          {#if activeBatch}
            <div class="batch-status-card">
              <div class="batch-status">
                <strong>{activeBatch.state}</strong>
                <span>{activeBatch.progress.completed_jobs} / {activeBatch.progress.total_jobs} jobs</span>
              </div>
              <div class="batch-status-metadata">
                <span>{activeBatchFamily} family</span>
                <span>{batchExecutionModeLabel(activeBatch.execution_mode)} mode</span>
                {#if activeBatch.execution_mode === "custom" && activeBatch.requested_max_active_jobs !== null}
                  {#if activeBatch.requested_max_active_jobs !== activeBatch.effective_max_active_jobs}
                    <span>
                      requested {activeBatch.requested_max_active_jobs}, using {activeBatch.effective_max_active_jobs}
                    </span>
                  {:else}
                    <span>max {activeBatch.effective_max_active_jobs} active jobs</span>
                  {/if}
                {:else}
                  <span>max {activeBatch.effective_max_active_jobs} active jobs</span>
                {/if}
                <span>{batchSchedulerReasonLabel(activeBatch.scheduler_reason)}</span>
                {#if batchQueuedCount > 0}
                  <span>{batchQueuedCount} queued</span>
                {/if}
                {#if batchRunningCount > 0}
                  <span>{batchRunningCount} running</span>
                {/if}
                {#if batchCompletedCount > 0}
                  <span>{batchCompletedCount} completed</span>
                {/if}
                {#if batchFailedCount > 0}
                  <span class="status-failed">{batchFailedCount} failed</span>
                {/if}
                {#if batchCancelledCount > 0}
                  <span>{batchCancelledCount} cancelled</span>
                {/if}
              </div>
              <div class="batch-item-list">
                {#each activeBatchItems as item (item.job_id)}
                  <div class="batch-item" data-state={item.state}>
                    <div class="batch-item-header">
                      <strong>{summarizeBatchItemPath(item.store_path)}</strong>
                      <span>{item.state}</span>
                    </div>
                    {#if item.output_store_path}
                      <div class="batch-item-detail">
                        Output: {summarizeBatchItemPath(item.output_store_path)}
                      </div>
                    {/if}
                    {#if item.error_message}
                      <div class="batch-item-detail error">{item.error_message}</div>
                    {/if}
                  </div>
                {/each}
              </div>
            </div>
          {/if}
        </section>
      {/if}
    </section>
  {/if}
</section>

<style>
  .control-panel {
    display: flex;
    flex-direction: column;
    gap: var(--ui-panel-gap);
    background: var(--panel-bg);
    border: 1px solid var(--app-border);
    border-radius: var(--ui-radius-lg);
    padding: var(--ui-panel-padding);
  }

  .control-header {
    display: flex;
    justify-content: space-between;
    gap: var(--ui-space-5);
    align-items: flex-start;
  }

  h3 {
    margin: 0;
    font-size: 12px;
    font-weight: 600;
    color: var(--text-primary);
  }

  .control-header p {
    margin: 2px 0 0;
    font-size: 11px;
    color: var(--text-muted);
  }

  .control-grid {
    display: grid;
    grid-template-columns: minmax(220px, 320px) minmax(0, 1fr);
    gap: var(--ui-space-4);
    align-items: end;
  }

  .action-row,
  .library-row,
  .mode-row {
    display: flex;
    gap: var(--ui-space-1);
    flex-wrap: wrap;
    align-items: center;
  }

  .field {
    display: flex;
    flex-direction: column;
    gap: var(--ui-field-gap);
  }

  .field span {
    font-size: 11px;
    color: var(--text-dim);
  }

  .field input,
  .library-row select {
    background: #fff;
    border: 1px solid var(--app-border-strong);
    border-radius: var(--ui-radius-md);
    color: var(--text-primary);
    min-height: var(--ui-input-height);
    padding: 0 var(--ui-space-3);
    font: inherit;
    font-size: 12px;
  }

  .library-row select {
    min-width: 220px;
    flex: 1;
  }

  .chip {
    border: 1px solid var(--app-border-strong);
    background: var(--surface-subtle);
    color: var(--text-primary);
    border-radius: var(--ui-radius-md);
    min-height: var(--ui-button-height);
    padding: 0 var(--ui-button-padding-x);
    font-size: 11px;
    cursor: pointer;
  }

  .chip:hover:not(:disabled) {
    background: var(--surface-bg);
    color: var(--text-primary);
  }

  .chip.primary {
    border-color: var(--accent-border);
    background: var(--accent-bg);
    color: var(--accent-text);
  }

  .chip.danger {
    border-color: #e0b7b7;
    color: #a74646;
  }

  .chip.active {
    background: #e8f3fb;
    border-color: #b0d4ee;
    color: #1f5577;
  }

  .output-settings {
    border: 1px solid var(--app-border);
    background: var(--surface-bg);
    border-radius: var(--ui-radius-lg);
    padding: var(--ui-space-3);
    display: flex;
    flex-direction: column;
    gap: var(--ui-space-3);
  }

  .output-settings-header {
    display: flex;
    justify-content: space-between;
    gap: var(--ui-space-3);
    align-items: center;
    color: var(--text-muted);
    font-size: 11px;
  }

  .path-row {
    display: grid;
    grid-template-columns: minmax(0, 1fr) auto auto;
    gap: var(--ui-space-2);
  }

  .batch-settings {
    border-top: 1px solid var(--app-border);
    padding-top: var(--ui-space-3);
    display: flex;
    flex-direction: column;
    gap: var(--ui-space-3);
  }

  .batch-candidate-list {
    display: grid;
    gap: var(--ui-space-2);
    max-height: 180px;
    overflow: auto;
  }

  .batch-candidate {
    display: grid;
    grid-template-columns: auto minmax(0, 1fr) auto;
    gap: var(--ui-space-2);
    align-items: center;
    font-size: 11px;
    color: var(--text-primary);
  }

  .batch-candidate-label {
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .batch-candidate-badge {
    border: 1px solid #b0d4ee;
    background: #e8f3fb;
    color: #1f5577;
    border-radius: 999px;
    padding: 2px 6px;
    font-size: 10px;
  }

  .batch-policy-row {
    display: grid;
    grid-template-columns: minmax(0, 1fr) minmax(0, 1fr);
    gap: var(--ui-space-2);
  }

  .batch-concurrency input {
    max-width: 140px;
  }

  .batch-status-card {
    display: flex;
    flex-direction: column;
    gap: var(--ui-space-2);
    padding: var(--ui-space-2);
    border: 1px solid var(--app-border);
    border-radius: var(--ui-radius-md);
    background: var(--surface-subtle);
  }

  .batch-status {
    display: flex;
    justify-content: space-between;
    gap: var(--ui-space-3);
    align-items: center;
    font-size: 11px;
    color: var(--text-muted);
  }

  .batch-status-metadata {
    display: flex;
    flex-wrap: wrap;
    gap: var(--ui-space-2);
    font-size: 10px;
    color: var(--text-muted);
  }

  .status-failed {
    color: #a74646;
  }

  .batch-item-list {
    display: grid;
    gap: var(--ui-space-2);
    max-height: 180px;
    overflow: auto;
  }

  .batch-item {
    display: flex;
    flex-direction: column;
    gap: 2px;
    padding: var(--ui-space-2);
    border-radius: var(--ui-radius-md);
    background: #fff;
    border: 1px solid var(--app-border);
    font-size: 10px;
  }

  .batch-item[data-state="failed"] {
    border-color: #e0b7b7;
    background: #fff6f6;
  }

  .batch-item[data-state="running"] {
    border-color: #b0d4ee;
    background: #f5fbff;
  }

  .batch-item-header {
    display: flex;
    justify-content: space-between;
    gap: var(--ui-space-2);
    align-items: center;
    color: var(--text-primary);
  }

  .batch-item-header strong {
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .batch-item-detail {
    color: var(--text-muted);
    word-break: break-word;
  }

  .batch-item-detail.error {
    color: #a74646;
  }

  .path-row input {
    min-width: 0;
  }

  small {
    color: var(--text-muted);
    font-size: 10px;
    line-height: 1.4;
  }

  .checkbox-row {
    display: flex;
    gap: var(--ui-space-3);
    align-items: center;
    color: var(--text-muted);
    font-size: 11px;
  }

  @media (max-width: 1100px) {
    .control-header,
    .control-grid {
      grid-template-columns: 1fr;
      flex-direction: column;
      align-items: stretch;
    }

    .library-row select {
      min-width: 0;
    }
  }
</style>
