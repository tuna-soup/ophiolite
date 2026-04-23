<svelte:options runes={true} />

<script lang="ts">
  import type { NeighborhoodOperation } from "../processing-model.svelte";
  import { describeNeighborhoodOperation } from "../processing-model.svelte";

  let {
    operations,
    selectedIndex,
    onSelect
  }: {
    operations: NeighborhoodOperation[];
    selectedIndex: number;
    onSelect: (index: number) => void;
  } = $props();
</script>

<section class="sequence-panel">
  <header class="panel-header">
    <div>
      <h3>Neighborhood</h3>
      <p>{operations.length} step{operations.length === 1 ? "" : "s"}</p>
    </div>
    <div class="header-meta">Fixed v1 operator surface</div>
  </header>

  <div class="intro-note">
    Neighborhood operators preview on the current inline or xline and run as full derived volumes.
  </div>

  <div class="sequence-list" role="listbox" tabindex="0" aria-label="Neighborhood steps">
    {#each operations as operation, index (`neighborhood:${index}:${describeNeighborhoodOperation(operation)}`)}
      <button
        class:selected={index === selectedIndex}
        class="sequence-row"
        onclick={() => onSelect(index)}
      >
        <span class="step-index">{index + 1}</span>
        <span class="step-copy">
          <strong>{describeNeighborhoodOperation(operation)}</strong>
          <small>Post-stack neighborhood</small>
        </span>
      </button>
    {/each}
  </div>
</section>

<style>
  .sequence-panel {
    display: flex;
    flex-direction: column;
    min-height: 0;
    background: var(--panel-bg);
    border: 1px solid var(--app-border);
    border-radius: var(--ui-radius-lg);
    overflow: hidden;
  }

  .panel-header {
    display: flex;
    justify-content: space-between;
    gap: var(--ui-space-3);
    align-items: flex-start;
    padding: var(--ui-panel-padding);
    border-bottom: 1px solid var(--app-border);
  }

  h3 {
    margin: 0;
    font-size: 12px;
    font-weight: 600;
    color: var(--text-primary);
  }

  .panel-header p,
  .header-meta,
  .intro-note,
  .step-copy small {
    color: var(--text-muted);
    font-size: 11px;
  }

  .intro-note {
    padding: 0 var(--ui-panel-padding) var(--ui-space-3);
  }

  .sequence-list {
    display: flex;
    flex-direction: column;
    gap: var(--ui-space-2);
    padding: 0 var(--ui-panel-padding) var(--ui-panel-padding);
    min-height: 0;
    overflow: auto;
  }

  .sequence-row {
    width: 100%;
    display: grid;
    grid-template-columns: 24px 1fr;
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

  .sequence-row.selected {
    border-color: #b0d4ee;
    background: #e8f3fb;
  }

  .step-index {
    color: var(--text-dim);
    font-size: 11px;
  }

  .step-copy {
    display: flex;
    flex-direction: column;
    gap: 2px;
  }
</style>
