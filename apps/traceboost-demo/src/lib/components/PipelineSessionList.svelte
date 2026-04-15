<svelte:options runes={true} />

<script lang="ts">
  import type { WorkspacePipelineEntry } from "@traceboost/seis-contracts";

  let {
    pipelines,
    activePipelineId,
    onSelect,
    onCreate,
    onDuplicate,
    onCopy,
    onPaste,
    onRemove,
    onRemoveItem,
    getLabel,
    canRemove
  }: {
    pipelines: WorkspacePipelineEntry[];
    activePipelineId: string | null;
    onSelect: (pipelineId: string) => void;
    onCreate: () => void;
    onDuplicate: () => void;
    onCopy: () => void;
    onPaste: () => void;
    onRemove: () => void;
    onRemoveItem: (pipelineId: string) => void;
    getLabel: (entry: WorkspacePipelineEntry, index: number) => string;
    canRemove: boolean;
  } = $props();

  function handleKeyDown(event: KeyboardEvent): void {
    if (!(event.ctrlKey || event.metaKey)) {
      return;
    }

    const key = event.key.toLowerCase();
    if (key === "c" && activePipelineId) {
      event.preventDefault();
      onCopy();
    }

    if (key === "v") {
      event.preventDefault();
      onPaste();
    }
  }
</script>

<section class="session-panel">
  <header class="panel-header">
    <div>
      <h3>Session Pipelines</h3>
      <p>{pipelines.length} pipeline{pipelines.length === 1 ? "" : "s"} in this dataset session</p>
    </div>
    <div class="action-row">
      <button class="chip" onclick={onCreate}>+ New</button>
      <button class="chip" onclick={onDuplicate} disabled={!activePipelineId}>Duplicate</button>
    </div>
  </header>

  <div class="pipeline-list" role="listbox" tabindex="0" onkeydown={handleKeyDown} aria-label="Session pipelines">
    {#each pipelines as entry, index (entry.pipeline_id)}
      {@const selected = entry.pipeline_id === activePipelineId}
      {@const label = getLabel(entry, index)}
      <div class="pipeline-row-shell">
        <button
          class:selected={selected}
          class="pipeline-row"
            onclick={() => onSelect(entry.pipeline_id)}
        >
          <span class="pipeline-index">{index + 1}</span>
          <span class="pipeline-copy">
            <strong>{label}</strong>
            <small>{entry.pipeline.steps.length} step{entry.pipeline.steps.length === 1 ? "" : "s"}</small>
          </span>
        </button>
        <button
          class="pipeline-remove"
          onclick={(event) => {
            event.stopPropagation();
            onRemoveItem(entry.pipeline_id);
          }}
          disabled={!canRemove}
          aria-label={`Remove ${label}`}
          title={`Remove ${label}`}
        >
          X
        </button>
      </div>
    {/each}
  </div>

  <div class="panel-footer">
    <button class="chip danger" onclick={onRemove} disabled={!canRemove}>
      Remove Active
    </button>
  </div>
</section>

<style>
  .session-panel {
    display: flex;
    flex-direction: column;
    min-height: 0;
    background: var(--panel-bg);
    border: 1px solid var(--app-border);
    border-radius: var(--ui-radius-lg);
    overflow: hidden;
  }

  .panel-header,
  .panel-footer {
    padding: var(--ui-space-3) var(--ui-space-4);
  }

  .panel-header {
    display: flex;
    flex-direction: column;
    gap: var(--ui-space-2);
    border-bottom: 1px solid var(--app-border);
  }

  h3 {
    margin: 0;
    font-size: 12px;
    font-weight: 600;
    color: var(--text-primary);
  }

  .panel-header p {
    margin: 0;
    font-size: 11px;
    color: var(--text-muted);
  }

  .action-row {
    display: flex;
    gap: var(--ui-space-1);
    flex-wrap: wrap;
  }

  .pipeline-list {
    padding: var(--ui-space-2);
    display: flex;
    flex-direction: column;
    gap: var(--ui-space-1);
    overflow: auto;
    min-height: 0;
    flex: 1;
    outline: none;
  }

  .pipeline-row-shell {
    display: grid;
    grid-template-columns: minmax(0, 1fr) auto;
    gap: var(--ui-space-2);
  }

  .pipeline-row {
    width: 100%;
    display: grid;
    grid-template-columns: 22px 1fr;
    gap: var(--ui-space-2);
    align-items: center;
    text-align: left;
    border: 1px solid var(--app-border);
    background: #fff;
    color: inherit;
    min-height: var(--ui-list-row-min-height);
    padding: var(--ui-list-row-padding-y) var(--ui-list-row-padding-x);
    border-radius: var(--ui-radius-md);
    cursor: pointer;
  }

  .pipeline-row:hover {
    background: var(--surface-bg);
  }

  .pipeline-row.selected {
    border-color: #b0d4ee;
    background: #e8f3fb;
  }

  .pipeline-remove {
    width: var(--ui-icon-button-size);
    border-radius: var(--ui-radius-md);
    border: 1px solid var(--app-border);
    background: var(--surface-subtle);
    color: var(--text-muted);
    cursor: pointer;
    opacity: 0;
    pointer-events: none;
    transition:
      opacity 120ms ease,
      border-color 120ms ease,
      background 120ms ease,
      color 120ms ease;
  }

  .pipeline-row-shell:hover .pipeline-remove,
  .pipeline-row:focus-visible + .pipeline-remove,
  .pipeline-remove:focus-visible {
    opacity: 1;
    pointer-events: auto;
  }

  .pipeline-remove:hover:not(:disabled) {
    border-color: #e0b7b7;
    background: #f9ecec;
    color: #a74646;
  }

  .pipeline-remove:disabled {
    cursor: not-allowed;
    opacity: 0.28;
  }

  .pipeline-index {
    width: 20px;
    height: 20px;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    border-radius: var(--ui-radius-md);
    background: var(--surface-subtle);
    font-size: 10px;
    color: var(--text-dim);
    flex-shrink: 0;
  }

  .pipeline-copy {
    min-width: 0;
    display: flex;
    flex-direction: column;
    gap: 1px;
  }

  .pipeline-copy strong,
  .pipeline-copy small {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .pipeline-copy strong {
    font-size: 12px;
    color: var(--text-primary);
  }

  .pipeline-copy small {
    font-size: 11px;
    color: var(--text-muted);
  }

  .panel-footer {
    border-top: 1px solid var(--app-border);
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

  .chip.danger {
    border-color: #e0b7b7;
    color: #a74646;
  }

  .chip:disabled {
    opacity: 0.38;
    cursor: not-allowed;
  }
</style>
