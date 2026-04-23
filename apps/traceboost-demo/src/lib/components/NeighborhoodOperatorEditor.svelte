<svelte:options runes={true} />

<script lang="ts">
  import type {
    LocalVolumeStatistic,
    NeighborhoodDipOutput,
    ProcessingJobStatus
  } from "@traceboost/seis-contracts";
  import type { NeighborhoodOperation } from "../processing-model.svelte";
  import {
    isNeighborhoodDip,
    isNeighborhoodLocalVolumeStats,
    isNeighborhoodSimilarity,
    summarizeProcessingExecution,
    summarizeProcessingPlan
  } from "../processing-model.svelte";

  let {
    selectedOperation,
    activeJob,
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
    processingError: string | null;
    onSetWindow: (field: "gate_ms" | "inline_stepout" | "xline_stepout", value: number) => void;
    onSetStatistic: (statistic: LocalVolumeStatistic) => void;
    onSetDipOutput: (output: NeighborhoodDipOutput) => void;
    onSetOperationKind: (kind: "similarity" | "local_volume_stats" | "dip") => void;
    onCancelJob: () => void | Promise<void>;
    onOpenArtifact: (storePath: string) => void | Promise<void>;
  } = $props();

  let planSummary = $derived(summarizeProcessingPlan(activeJob?.plan_summary));
  let executionSummary = $derived(summarizeProcessingExecution(activeJob?.execution_summary));
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
      {#if planSummary}
        <div class="job-plan">
          <strong>Planned Execution</strong>
          <p>{planSummary.overview}</p>
          {#if planSummary.detail}
            <p>{planSummary.detail}</p>
          {/if}
          {#if planSummary.stages.length}
            <div class="job-plan-stages">
              {#each planSummary.stages as stageSummary (`${activeJob.job_id}:${stageSummary}`)}
                <span>{stageSummary}</span>
              {/each}
            </div>
          {/if}
        </div>
      {/if}
      {#if executionSummary}
        <div class="job-plan">
          <strong>Actual Execution</strong>
          <p>{executionSummary.overview}</p>
          {#if executionSummary.detail}
            <p>{executionSummary.detail}</p>
          {/if}
          {#if executionSummary.stages.length}
            <div class="job-plan-stages">
              {#each executionSummary.stages as stageSummary (`${activeJob.job_id}:actual:${stageSummary}`)}
                <span>{stageSummary}</span>
              {/each}
            </div>
          {/if}
        </div>
      {/if}
      <div class="job-actions">
        <button class="chip" onclick={onCancelJob} disabled={activeJob.state !== "queued" && activeJob.state !== "running"}>
          Cancel Job
        </button>
        {#each activeJob.artifacts as artifact (artifact.store_path)}
          <button class="chip" onclick={() => onOpenArtifact(artifact.store_path)}>
            Open {artifact.kind === "final_output" ? "Output" : "Artifact"}
          </button>
        {/each}
      </div>
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

  .job-header,
  .job-actions {
    display: flex;
    gap: var(--ui-space-2);
    align-items: center;
    flex-wrap: wrap;
    justify-content: space-between;
  }

  .job-plan {
    display: flex;
    flex-direction: column;
    gap: 4px;
    margin-top: var(--ui-space-2);
    padding-top: var(--ui-space-2);
    border-top: 1px solid var(--app-border);
  }

  .job-plan strong {
    color: var(--text-primary);
    font-size: 11px;
  }

  .job-plan p {
    margin: 0;
    font-size: 11px;
    color: var(--text-muted);
    line-height: 1.45;
  }

  .job-plan-stages {
    display: flex;
    flex-direction: column;
    gap: 3px;
  }

  .job-plan-stages span {
    color: var(--text-dim);
    font-size: 10px;
    line-height: 1.4;
  }

  .chip {
    border: 1px solid var(--app-border-strong);
    background: #fff;
    color: var(--text-primary);
    border-radius: var(--ui-radius-md);
    min-height: var(--ui-button-height);
    padding: 0 var(--ui-button-padding-x);
    font-size: 11px;
    cursor: pointer;
  }

  @media (max-width: 900px) {
    .field-grid {
      grid-template-columns: 1fr;
    }
  }
</style>
