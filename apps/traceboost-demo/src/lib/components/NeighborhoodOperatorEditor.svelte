<svelte:options runes={true} />

<script lang="ts">
  import type {
    InspectableProcessingPlan,
    LocalVolumeStatistic,
    NeighborhoodDipOutput,
    ProcessingJobRuntimeState,
    ProcessingJobStatus
  } from "@traceboost/seis-contracts";
  import type { ProcessingRuntimeEvent } from "@traceboost/seis-contracts";
  import type { NeighborhoodOperation } from "../processing-model.svelte";
  import {
    isNeighborhoodDip,
    isNeighborhoodLocalVolumeStats,
    isNeighborhoodSimilarity
  } from "../processing-model.svelte";
  import ProcessingDebugPanel from "./ProcessingDebugPanel.svelte";

  let {
    selectedOperation,
    activeJob,
    activeDebugPlan = null,
    activeRuntimeState = null,
    activeRuntimeEvents = [],
    processingError,
    onSetWindow,
    onSetStatistic,
    onSetDipOutput,
    onSetOperationKind,
    onCancelJob,
    onOpenArtifact
  }: {
    selectedOperation: NeighborhoodOperation | null;
    activeJob: ProcessingJobStatus | null;
    activeDebugPlan?: InspectableProcessingPlan | null;
    activeRuntimeState?: ProcessingJobRuntimeState | null;
    activeRuntimeEvents?: ProcessingRuntimeEvent[];
    processingError: string | null;
    onSetWindow: (field: "gate_ms" | "inline_stepout" | "xline_stepout", value: number) => void;
    onSetStatistic: (statistic: LocalVolumeStatistic) => void;
    onSetDipOutput: (output: NeighborhoodDipOutput) => void;
    onSetOperationKind: (kind: "similarity" | "local_volume_stats" | "dip") => void;
    onCancelJob: () => void | Promise<void>;
    onOpenArtifact: (storePath: string) => void | Promise<void>;
  } = $props();

</script>

<section class="editor-panel">
  <header class="editor-header">
    <h3>Neighborhood Editor</h3>
    <p>Choose the neighborhood operator and adjust the shared window and stepout.</p>
  </header>

  {#if selectedOperation}
    <label class="field operator-type-field">
      <span>Operator</span>
      <select
        value={
          isNeighborhoodSimilarity(selectedOperation)
            ? "similarity"
            : isNeighborhoodLocalVolumeStats(selectedOperation)
              ? "local_volume_stats"
              : "dip"
        }
        onchange={(event) =>
          onSetOperationKind(
            (event.currentTarget as HTMLSelectElement).value as "similarity" | "local_volume_stats" | "dip"
          )}
      >
        <option value="similarity">Similarity</option>
        <option value="local_volume_stats">Local Volume Stats</option>
        <option value="dip">Dip</option>
      </select>
    </label>

    <div class="field-grid">
      <label class="field">
        <span>Gate (ms)</span>
        <input
          type="number"
          min="1"
          step="1"
          value={
            isNeighborhoodSimilarity(selectedOperation)
              ? selectedOperation.similarity.window.gate_ms
              : isNeighborhoodLocalVolumeStats(selectedOperation)
                ? selectedOperation.local_volume_stats.window.gate_ms
                : selectedOperation.dip.window.gate_ms
          }
          oninput={(event) => onSetWindow("gate_ms", Number((event.currentTarget as HTMLInputElement).value))}
        />
      </label>
      <label class="field">
        <span>Inline Stepout</span>
        <input
          type="number"
          min="0"
          step="1"
          value={
            isNeighborhoodSimilarity(selectedOperation)
              ? selectedOperation.similarity.window.inline_stepout
              : isNeighborhoodLocalVolumeStats(selectedOperation)
                ? selectedOperation.local_volume_stats.window.inline_stepout
                : selectedOperation.dip.window.inline_stepout
          }
          oninput={(event) =>
            onSetWindow("inline_stepout", Number((event.currentTarget as HTMLInputElement).value))}
        />
      </label>
      <label class="field">
        <span>Xline Stepout</span>
        <input
          type="number"
          min="0"
          step="1"
          value={
            isNeighborhoodSimilarity(selectedOperation)
              ? selectedOperation.similarity.window.xline_stepout
              : isNeighborhoodLocalVolumeStats(selectedOperation)
                ? selectedOperation.local_volume_stats.window.xline_stepout
                : selectedOperation.dip.window.xline_stepout
          }
          oninput={(event) =>
            onSetWindow("xline_stepout", Number((event.currentTarget as HTMLInputElement).value))}
        />
      </label>
    </div>

    {#if isNeighborhoodLocalVolumeStats(selectedOperation)}
      <label class="field operator-type-field">
        <span>Statistic</span>
        <select
          value={selectedOperation.local_volume_stats.statistic}
          onchange={(event) =>
            onSetStatistic((event.currentTarget as HTMLSelectElement).value as LocalVolumeStatistic)}
        >
          <option value="mean">Mean</option>
          <option value="rms">RMS</option>
          <option value="variance">Variance</option>
          <option value="minimum">Minimum</option>
          <option value="maximum">Maximum</option>
        </select>
      </label>
    {:else if isNeighborhoodDip(selectedOperation)}
      <label class="field operator-type-field">
        <span>Output</span>
        <select
          value={selectedOperation.dip.output}
          onchange={(event) =>
            onSetDipOutput((event.currentTarget as HTMLSelectElement).value as NeighborhoodDipOutput)}
        >
          <option value="inline">Inline</option>
          <option value="xline">Xline</option>
          <option value="azimuth">Azimuth</option>
          <option value="abs_dip">Absolute Dip</option>
        </select>
      </label>
    {/if}

    <div class="info-block">
      {#if isNeighborhoodSimilarity(selectedOperation)}
        <strong>Similarity</strong>
        <p>
          Computes a bounded continuity score from neighboring traces using a symmetric vertical gate and
          rectangular inline/xline stepout.
        </p>
        <p>
          Higher values indicate waveform continuity. Lower values highlight discontinuities such as faults,
          edges, and reflector breaks.
        </p>
      {:else if isNeighborhoodLocalVolumeStats(selectedOperation)}
        <strong>Local Volume Stats</strong>
        <p>
          Computes the selected amplitude statistic over the full valid 3D neighborhood window, including
          the center trace samples.
        </p>
        <p>
          Use it to map local mean level, RMS energy, variance, or bounded extrema before moving on to dip.
        </p>
      {:else if isNeighborhoodDip(selectedOperation)}
        <strong>Dip</strong>
        <p>
          Estimates local time-shift slope from neighboring traces over the shared vertical gate and
          rectangular inline/xline stepout.
        </p>
        <p>
          Inline and xline outputs are returned as milliseconds per trace. Azimuth is derived from those
          fitted slopes, and absolute dip is their combined magnitude.
        </p>
      {/if}
    </div>
  {/if}

  {#if processingError}
    <div class="error-block">{processingError}</div>
  {/if}

  {#if activeJob}
    <div class="job-block">
      <div class="job-header">
        <strong>Active Job</strong>
        <span>{activeJob.state}</span>
      </div>
      {#if activeJob.progress}
        <p>{activeJob.progress.completed} / {activeJob.progress.total} units</p>
      {/if}
      <ProcessingDebugPanel
        {activeJob}
        debugPlan={activeDebugPlan}
        runtimeState={activeRuntimeState}
        runtimeEvents={activeRuntimeEvents}
        {onCancelJob}
        {onOpenArtifact}
      />
    </div>
  {/if}
</section>

<style>
  .editor-panel {
    display: flex;
    flex-direction: column;
    gap: var(--ui-panel-gap);
    background: var(--panel-bg);
    border: 1px solid var(--app-border);
    border-radius: var(--ui-radius-lg);
    padding: var(--ui-panel-padding);
    min-height: 0;
  }

  .editor-header h3 {
    margin: 0;
    font-size: 12px;
    font-weight: 600;
    color: var(--text-primary);
  }

  .editor-header p,
  .info-block p,
  .job-block p {
    margin: 4px 0 0;
    font-size: 11px;
    color: var(--text-muted);
  }

  .field-grid {
    display: grid;
    grid-template-columns: repeat(3, minmax(0, 1fr));
    gap: var(--ui-space-3);
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

  .field input {
    background: #fff;
    border: 1px solid var(--app-border-strong);
    border-radius: var(--ui-radius-md);
    color: var(--text-primary);
    min-height: var(--ui-input-height);
    padding: 0 var(--ui-space-3);
    font: inherit;
    font-size: 12px;
  }

  .field select {
    background: #fff;
    border: 1px solid var(--app-border-strong);
    border-radius: var(--ui-radius-md);
    color: var(--text-primary);
    min-height: var(--ui-input-height);
    padding: 0 var(--ui-space-3);
    font: inherit;
    font-size: 12px;
  }

  .operator-type-field {
    max-width: 240px;
  }

  .info-block,
  .job-block,
  .error-block {
    border: 1px solid var(--app-border);
    border-radius: var(--ui-radius-md);
    background: var(--surface-subtle);
    padding: var(--ui-space-3);
  }

  .error-block {
    color: #a74646;
    background: #f9ecec;
    border-color: #e0b7b7;
    font-size: 11px;
  }

  .job-header {
    display: flex;
    gap: var(--ui-space-2);
    align-items: center;
    flex-wrap: wrap;
    justify-content: space-between;
  }

  @media (max-width: 900px) {
    .field-grid {
      grid-template-columns: 1fr;
    }
  }
</style>
