<svelte:options runes={true} />

<script lang="ts">
  import type { RecentProcessingEntry } from "../processing-model.svelte";

  let {
    entries,
    activeJobId,
    activeBatchId,
    onSelectJob,
    onSelectBatch,
    onOpenArtifact,
    canClearFinished,
    onClearFinished
  }: {
    entries: RecentProcessingEntry[];
    activeJobId: string | null;
    activeBatchId: string | null;
    onSelectJob: (jobId: string) => void;
    onSelectBatch: (batchId: string) => void;
    onOpenArtifact: (storePath: string) => void | Promise<void>;
    canClearFinished: boolean;
    onClearFinished: () => void;
  } = $props();

  function summarizeStorePath(path: string | null | undefined): string | null {
    const normalized = path?.trim();
    if (!normalized) {
      return null;
    }
    const parts = normalized.replace(/\\/g, "/").split("/").filter((part) => part.length > 0);
    return parts.at(-1) ?? normalized;
  }

  function isActive(entry: RecentProcessingEntry): boolean {
    return entry.kind === "job"
      ? activeJobId === entry.job.job_id
      : activeBatchId === entry.batch.batch_id;
  }

  function subtitle(entry: RecentProcessingEntry): string {
    if (entry.kind === "job") {
      return summarizeStorePath(entry.job.output_store_path) ?? summarizeStorePath(entry.job.input_store_path) ?? entry.job.job_id;
    }
    const failedCount = entry.batch.items.filter((item) => item.state === "failed").length;
    return failedCount > 0
      ? `${entry.batch.progress.completed_jobs}/${entry.batch.progress.total_jobs} complete · ${failedCount} failed`
      : `${entry.batch.progress.completed_jobs}/${entry.batch.progress.total_jobs} complete`;
  }

  function detail(entry: RecentProcessingEntry): string | null {
    if (entry.kind === "job") {
      return entry.job.error_message ?? entry.job.current_stage_label ?? null;
    }
    const firstError = entry.batch.items.find((item) => item.error_message)?.error_message;
    return firstError ?? null;
  }

  function finalOutputPath(entry: RecentProcessingEntry): string | null {
    if (entry.kind !== "job") {
      return null;
    }
    return (
      entry.job.output_store_path ??
      entry.job.artifacts.find((artifact) => artifact.kind === "final_output")?.store_path ??
      null
    );
  }

  function handleSelect(entry: RecentProcessingEntry): void {
    if (entry.kind === "job") {
      onSelectJob(entry.job.job_id);
      return;
    }
    onSelectBatch(entry.batch.batch_id);
  }

  function handleOpenArtifact(event: MouseEvent, entry: RecentProcessingEntry): void {
    event.stopPropagation();
    const outputPath = finalOutputPath(entry);
    if (!outputPath) {
      return;
    }
    void onOpenArtifact(outputPath);
  }

  function handleCardKeydown(event: KeyboardEvent, entry: RecentProcessingEntry): void {
    if (event.key !== "Enter" && event.key !== " ") {
      return;
    }
    event.preventDefault();
    handleSelect(entry);
  }
</script>

{#if entries.length > 0}
  <section class="activity-strip">
    <div class="activity-header">
      <div class="activity-header-copy">
        <h3>Recent Activity</h3>
        <span>{entries.length} session item{entries.length === 1 ? "" : "s"}</span>
      </div>
      {#if canClearFinished}
        <button class="activity-header-action" onclick={onClearFinished}>Clear Finished</button>
      {/if}
    </div>

    <div class="activity-list">
      {#each entries as entry (entry.kind === "job" ? entry.job.job_id : entry.batch.batch_id)}
        <div
          class:active={isActive(entry)}
          class="activity-card"
          role="button"
          tabindex="0"
          data-state={entry.kind === "job" ? entry.job.state : entry.batch.state}
          onclick={() => handleSelect(entry)}
          onkeydown={(event) => handleCardKeydown(event, entry)}
        >
          <div class="activity-card-header">
            <strong>{entry.title}</strong>
            <span>{entry.kind === "job" ? entry.job.state : entry.batch.state}</span>
          </div>
          <div class="activity-subtitle">{subtitle(entry)}</div>
          <div class="activity-meta">
            <span>{entry.familyLabel}</span>
            <span>{entry.kind === "job" ? "job" : "batch"}</span>
          </div>
          {#if finalOutputPath(entry)}
            <div class="activity-actions">
              <button class="activity-action" onclick={(event) => handleOpenArtifact(event, entry)}>
                Open Output
              </button>
            </div>
          {/if}
          {#if detail(entry)}
            <div class="activity-detail">{detail(entry)}</div>
          {/if}
        </div>
      {/each}
    </div>
  </section>
{/if}

<style>
  .activity-strip {
    display: flex;
    flex-direction: column;
    gap: var(--ui-space-2);
    padding: var(--ui-space-3);
    border: 1px solid var(--app-border);
    border-radius: var(--ui-radius-lg);
    background: var(--panel-bg);
  }

  .activity-header {
    display: flex;
    justify-content: space-between;
    gap: var(--ui-space-3);
    align-items: center;
  }

  .activity-header-copy {
    display: flex;
    flex-direction: column;
    gap: 2px;
  }

  .activity-header h3 {
    margin: 0;
    font-size: 12px;
    color: var(--text-primary);
  }

  .activity-header span {
    font-size: 10px;
    color: var(--text-muted);
  }

  .activity-header-action {
    border: 1px solid var(--app-border-strong);
    background: var(--surface-subtle);
    color: var(--text-primary);
    border-radius: var(--ui-radius-md);
    min-height: 26px;
    padding: 0 var(--ui-space-2);
    font-size: 10px;
    cursor: pointer;
    white-space: nowrap;
  }

  .activity-list {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(220px, 1fr));
    gap: var(--ui-space-2);
  }

  .activity-card {
    display: flex;
    flex-direction: column;
    gap: 4px;
    text-align: left;
    border: 1px solid var(--app-border);
    border-radius: var(--ui-radius-md);
    background: #fff;
    color: var(--text-primary);
    padding: var(--ui-space-2);
    cursor: pointer;
  }

  .activity-card.active {
    border-color: #b0d4ee;
    background: #f5fbff;
  }

  .activity-card[data-state="failed"] {
    border-color: #e0b7b7;
    background: #fff6f6;
  }

  .activity-card[data-state="running"] {
    border-color: #b0d4ee;
  }

  .activity-card-header {
    display: flex;
    justify-content: space-between;
    gap: var(--ui-space-2);
    align-items: center;
    font-size: 11px;
  }

  .activity-card-header strong {
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .activity-card-header span,
  .activity-subtitle,
  .activity-meta,
  .activity-detail {
    font-size: 10px;
  }

  .activity-subtitle,
  .activity-meta {
    color: var(--text-muted);
  }

  .activity-meta {
    display: flex;
    gap: var(--ui-space-2);
    flex-wrap: wrap;
  }

  .activity-detail {
    color: #a74646;
    word-break: break-word;
  }

  .activity-actions {
    display: flex;
    justify-content: flex-start;
  }

  .activity-action {
    border: 1px solid var(--app-border-strong);
    background: var(--surface-subtle);
    color: var(--text-primary);
    border-radius: var(--ui-radius-md);
    min-height: 24px;
    padding: 0 var(--ui-space-2);
    font-size: 10px;
    cursor: pointer;
  }
</style>
