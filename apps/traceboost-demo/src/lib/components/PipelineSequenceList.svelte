<svelte:options runes={true} />

<script lang="ts">
  import type { OperatorCatalogId, WorkspaceOperation } from "../processing-model.svelte";
  import { describeOperation, operatorCatalogItems } from "../processing-model.svelte";

  let {
    operations,
    traceLocalOperationCount,
    hasSubvolumeCrop,
    selectedIndex,
    checkpointAfterOperationIndexes,
    checkpointWarning,
    onSelect,
    onInsertOperator,
    onCopy,
    onPaste,
    onRemove,
    onToggleCheckpoint
  }: {
    operations: WorkspaceOperation[];
    traceLocalOperationCount: number;
    hasSubvolumeCrop: boolean;
    selectedIndex: number;
    checkpointAfterOperationIndexes: number[];
    checkpointWarning: string | null;
    onSelect: (index: number) => void;
    onInsertOperator: (operatorId: OperatorCatalogId) => void;
    onCopy: () => void;
    onPaste: () => void;
    onRemove: (index: number) => void;
    onToggleCheckpoint: (index: number) => void;
  } = $props();

  let query = $state("");
  let searchFocused = $state(false);
  let activeResultIndex = $state(0);
  let searchInput: HTMLInputElement | null = null;
  let hoveredStepIndex = $state<number | null>(null);

  const normalizedQuery = $derived(query.trim().toLowerCase());
  const filteredCatalog = $derived(
    operatorCatalogItems.filter((item) => {
      if (!normalizedQuery) {
        return true;
      }
      const haystack = [item.label, item.description, ...item.keywords, item.shortcut].join(" ").toLowerCase();
      return haystack.includes(normalizedQuery);
    })
  );
  const showCatalog = $derived(searchFocused || normalizedQuery.length > 0);
  const checkpointIndexSet = $derived(new Set(checkpointAfterOperationIndexes));
  const traceLocalOperations = $derived(operations.slice(0, traceLocalOperationCount));
  const runOnlyOperation = $derived(
    hasSubvolumeCrop ? operations[traceLocalOperationCount] ?? null : null
  );

  function summary(operation: WorkspaceOperation): string {
    return describeOperation(operation);
  }

  function focusSearch(): void {
    searchInput?.focus();
    searchInput?.select();
  }

  function resetSearch(): void {
    query = "";
    activeResultIndex = 0;
  }

  function insertOperator(operatorId: OperatorCatalogId): void {
    onInsertOperator(operatorId);
    resetSearch();
    focusSearch();
  }

  function handleSearchKeydown(event: KeyboardEvent): void {
    if (event.key === "ArrowDown") {
      event.preventDefault();
      if (filteredCatalog.length) {
        activeResultIndex = Math.min(activeResultIndex + 1, filteredCatalog.length - 1);
      }
      return;
    }

    if (event.key === "ArrowUp") {
      event.preventDefault();
      activeResultIndex = Math.max(activeResultIndex - 1, 0);
      return;
    }

    if (event.key === "Enter") {
      event.preventDefault();
      const target = filteredCatalog[activeResultIndex] ?? filteredCatalog[0];
      if (target) {
        insertOperator(target.id);
      }
      return;
    }

    if (event.key === "Escape") {
      event.preventDefault();
      if (query) {
        resetSearch();
      } else {
        searchInput?.blur();
      }
    }
  }

  function handleSequenceKeydown(event: KeyboardEvent): void {
    if (!(event.ctrlKey || event.metaKey)) {
      return;
    }

    const key = event.key.toLowerCase();
    if (key === "c" && operations.length) {
      event.preventDefault();
      onCopy();
    }

    if (key === "v") {
      event.preventDefault();
      onPaste();
    }
  }

  function handleWindowKeydown(event: KeyboardEvent): void {
    const target = event.target as HTMLElement | null;
    const tagName = target?.tagName?.toLowerCase();
    const editingText = Boolean(
      target?.isContentEditable ||
        tagName === "input" ||
        tagName === "textarea" ||
        tagName === "select"
    );

    if ((event.ctrlKey || event.metaKey) && event.key.toLowerCase() === "k") {
      event.preventDefault();
      focusSearch();
      return;
    }

    if (editingText || event.ctrlKey || event.metaKey || event.altKey) {
      return;
    }

    if (event.key === "/") {
      event.preventDefault();
      focusSearch();
    }
  }
</script>

<svelte:window onkeydown={handleWindowKeydown} />

<section class="sequence-panel">
  <header class="panel-header">
    <div>
      <h3>Pipeline</h3>
      <p>{operations.length} step{operations.length === 1 ? "" : "s"}</p>
    </div>
    <div class="header-meta">
      <span>{checkpointAfterOperationIndexes.length} checkpoint{checkpointAfterOperationIndexes.length === 1 ? "" : "s"}</span>
    </div>
  </header>

  <div class="search-shell">
    <label class="search-label" for="pipeline-operator-search">Add Operator</label>
    <div class="search-input-shell">
      <span class="search-prompt">&gt;</span>
      <input
        bind:this={searchInput}
        id="pipeline-operator-search"
        type="text"
        placeholder="Search operators..."
        bind:value={query}
        onfocus={() => {
          searchFocused = true;
          activeResultIndex = 0;
        }}
        onblur={() => {
          searchFocused = false;
        }}
        oninput={() => {
          activeResultIndex = 0;
        }}
        onkeydown={handleSearchKeydown}
      />
    </div>
    <div class="search-meta">
      <span><code>/</code> or <code>Ctrl/Cmd+K</code> focus</span>
      <span><code>Enter</code> insert</span>
    </div>

    {#if showCatalog}
      <div class="catalog-list">
        {#if filteredCatalog.length}
          {#each filteredCatalog as item, index (item.id)}
            <button
              class:active={index === activeResultIndex}
              class="catalog-row"
              onmousedown={(event) => event.preventDefault()}
              onclick={() => insertOperator(item.id)}
              onmouseenter={() => {
                activeResultIndex = index;
              }}
            >
              <span class="catalog-copy">
                <strong>{item.label}</strong>
                <span>{item.description}</span>
              </span>
              {#if item.shortcut}
                <span class="catalog-meta">
                  <kbd>{item.shortcut}</kbd>
                </span>
              {/if}
            </button>
          {/each}
        {:else}
          <div class="catalog-empty">No operators match "{query.trim()}".</div>
        {/if}
      </div>
    {/if}
  </div>

  {#if checkpointWarning}
    <div class="checkpoint-warning">{checkpointWarning}</div>
  {/if}

  {#if hasSubvolumeCrop}
    <div class="tail-warning">Preview shows only the processing steps. Crop Subvolume applies only on Run Volume.</div>
  {/if}

  {#if operations.length}
    <div
      class="sequence-list"
      role="listbox"
      tabindex="0"
      onkeydown={handleSequenceKeydown}
      aria-label="Pipeline steps"
    >
      {#if traceLocalOperations.length}
        <div class="sequence-phase-label">Preview + Run</div>
      {/if}

      {#each traceLocalOperations as operation, index (`trace:${index}:${summary(operation)}`)}
        {@const label = summary(operation)}
        {@const checkpointArmed = checkpointIndexSet.has(index)}
        {@const canToggleCheckpoint = index < traceLocalOperationCount - 1 || hasSubvolumeCrop}
        <div
          class="sequence-row-shell"
          role="presentation"
          onmouseenter={() => {
            hoveredStepIndex = index;
          }}
          onmouseleave={() => {
            if (hoveredStepIndex === index) {
              hoveredStepIndex = null;
            }
          }}
        >
          <button
            class:armed={checkpointArmed}
            class:visible={checkpointArmed || (canToggleCheckpoint && hoveredStepIndex === index)}
            class="checkpoint-gutter"
            disabled={!canToggleCheckpoint}
            onclick={(event) => {
              event.stopPropagation();
              onToggleCheckpoint(index);
            }}
            aria-label={
              checkpointArmed
                ? `Remove checkpoint after ${label}`
                : `Add checkpoint after ${label}`
            }
            title={
              canToggleCheckpoint
                ? checkpointArmed
                  ? `Remove checkpoint after ${label}`
                  : `Add checkpoint after ${label}`
                : "Final output is emitted automatically"
            }
          >
            <span></span>
          </button>
          <button
            class:selected={index === selectedIndex}
            class="sequence-row"
            onclick={() => onSelect(index)}
          >
            <span class="step-index">{index + 1}</span>
            <span class="step-copy">
              <strong>{label}</strong>
            </span>
          </button>
          <button
            class="step-remove"
            onclick={(event) => {
              event.stopPropagation();
              onRemove(index);
            }}
            aria-label={`Remove ${label}`}
            title={`Remove ${label}`}
          >
            X
          </button>
        </div>
      {/each}

      {#if runOnlyOperation}
        <div class="tail-divider" role="presentation">
          <span>Run Volume Only</span>
          <small>Not shown in preview</small>
        </div>

        {@const label = summary(runOnlyOperation)}
        {@const cropIndex = traceLocalOperationCount}
        <div class="sequence-row-shell tail-shell" role="presentation">
          <span class="tail-spacer" aria-hidden="true"></span>
          <button
            class:selected={cropIndex === selectedIndex}
            class="sequence-row tail-row"
            onclick={() => onSelect(cropIndex)}
          >
            <span class="step-index">{cropIndex + 1}</span>
            <span class="step-copy">
              <strong>{label}</strong>
              <span class="step-note">Run Only</span>
            </span>
          </button>
          <button
            class="step-remove"
            onclick={(event) => {
              event.stopPropagation();
              onRemove(cropIndex);
            }}
            aria-label={`Remove ${label}`}
            title={`Remove ${label}`}
          >
            X
          </button>
        </div>
      {/if}
    </div>
  {:else}
    <div class="empty-state">
      <p>No operators in the pipeline.</p>
      <p class="hint">Use the search above to add processing steps. Crop Subvolume appears as a run-only tail step.</p>
    </div>
  {/if}
</section>

<style>
  .sequence-panel {
    display: flex;
    flex-direction: column;
    min-height: 0;
    background: var(--panel-bg);
    border: 1px solid var(--app-border);
    border-radius: var(--ui-radius-lg);
    overflow: visible;
    position: relative;
    isolation: isolate;
    z-index: 3;
  }

  .panel-header {
    display: flex;
    justify-content: space-between;
    gap: var(--ui-space-3);
    padding: var(--ui-space-3) var(--ui-space-4);
    border-bottom: 1px solid var(--app-border);
    align-items: center;
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

  .header-meta {
    font-size: 10px;
    color: var(--text-dim);
    text-transform: uppercase;
    letter-spacing: 0.08em;
  }

  .search-shell {
    display: flex;
    flex-direction: column;
    gap: var(--ui-space-2);
    padding: var(--ui-panel-padding);
    border-bottom: 1px solid var(--app-border);
    background: var(--surface-bg);
    position: relative;
    overflow: visible;
    z-index: 2;
  }

  .search-label {
    font-size: 11px;
    color: var(--text-dim);
  }

  .search-input-shell {
    display: grid;
    grid-template-columns: 18px minmax(0, 1fr);
    align-items: center;
    gap: var(--ui-space-3);
    border: 1px solid var(--app-border-strong);
    border-radius: var(--ui-radius-md);
    background: #fff;
    min-height: var(--ui-input-height);
    padding: 0 var(--ui-space-4);
  }

  .search-prompt {
    color: var(--text-dim);
    font-family: "Cascadia Mono", "Consolas", monospace;
    font-size: 15px;
    font-weight: 700;
  }

  .search-input-shell input {
    min-width: 0;
    border: none;
    outline: none;
    background: transparent;
    color: var(--text-primary);
    font: inherit;
    font-size: 13px;
  }

  .search-meta {
    display: flex;
    justify-content: space-between;
    gap: 8px;
    color: var(--text-muted);
    font-size: 10px;
  }

  .search-meta code,
  .catalog-meta kbd {
    font-family: "Cascadia Mono", "Consolas", monospace;
  }

  .catalog-list {
    position: absolute;
    top: calc(100% + 4px);
    left: var(--ui-panel-padding);
    right: var(--ui-panel-padding);
    border: 1px solid var(--app-border);
    border-radius: var(--ui-radius-lg);
    background: var(--panel-bg);
    max-height: min(420px, calc(100vh - 180px));
    overflow: auto;
    box-shadow: var(--ui-shadow-soft);
    z-index: 20;
  }

  .catalog-row {
    width: 100%;
    display: grid;
    grid-template-columns: minmax(0, 1fr) auto;
    gap: var(--ui-space-4);
    align-items: center;
    min-height: var(--ui-list-row-min-height);
    padding: var(--ui-list-row-padding-y) var(--ui-list-row-padding-x);
    border: none;
    border-bottom: 1px solid var(--app-border);
    background: transparent;
    color: inherit;
    text-align: left;
    cursor: pointer;
  }

  .catalog-row:hover,
  .catalog-row.active {
    background: #e8f3fb;
  }

  .catalog-copy {
    display: flex;
    flex-direction: column;
    gap: 1px;
    min-width: 0;
  }

  .catalog-copy strong {
    color: var(--text-primary);
    font-size: 12px;
    font-weight: 600;
  }

  .catalog-copy span {
    color: var(--text-muted);
    font-size: 11px;
  }

  .catalog-meta kbd {
    border: 1px solid var(--app-border-strong);
    border-radius: var(--ui-radius-sm);
    padding: 2px var(--ui-space-2);
    background: var(--surface-subtle);
    color: var(--text-dim);
    font-size: 10px;
  }

  .catalog-empty {
    padding: var(--ui-panel-padding);
    color: var(--text-muted);
    font-size: 11px;
  }

  .checkpoint-warning {
    padding: var(--ui-space-3) var(--ui-space-4);
    border-bottom: 1px solid var(--warn-border);
    background: var(--warn-bg);
    color: var(--warn-text);
    font-size: 11px;
  }

  .tail-warning {
    padding: var(--ui-space-3) var(--ui-space-4);
    border-bottom: 1px solid var(--info-border);
    background: var(--info-bg);
    color: var(--info-text);
    font-size: 11px;
  }

  .sequence-list {
    margin: 0;
    padding: var(--ui-space-2);
    display: flex;
    flex-direction: column;
    gap: var(--ui-space-1);
    overflow: auto;
    min-height: 0;
    outline: none;
  }

  .sequence-phase-label {
    padding: 6px 2px 4px;
    font-size: 10px;
    letter-spacing: 0.08em;
    text-transform: uppercase;
    color: var(--text-dim);
  }

  .tail-divider {
    display: flex;
    justify-content: space-between;
    gap: var(--ui-space-3);
    align-items: center;
    margin-top: var(--ui-space-4);
    padding: var(--ui-space-4) 2px var(--ui-space-1);
    border-top: 1px solid var(--app-border);
    font-size: 10px;
    letter-spacing: 0.08em;
    text-transform: uppercase;
    color: #4d7088;
  }

  .tail-divider small {
    font: inherit;
    color: var(--text-muted);
  }

  .sequence-row-shell {
    margin: 0;
    display: grid;
    grid-template-columns: 16px minmax(0, 1fr) auto;
    gap: var(--ui-space-2);
    align-items: stretch;
  }

  .tail-shell {
    margin-top: 2px;
  }

  .checkpoint-gutter {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    border: none;
    background: transparent;
    padding: 0;
    cursor: pointer;
  }

  .checkpoint-gutter span {
    width: 10px;
    height: 10px;
    border-radius: 999px;
    border: 1px solid transparent;
    background: transparent;
    opacity: 0;
    transition:
      opacity 120ms ease,
      background 120ms ease,
      border-color 120ms ease,
      transform 120ms ease;
  }

  .checkpoint-gutter.visible span {
    opacity: 1;
    border-color: #cf8787;
  }

  .checkpoint-gutter.armed span {
    opacity: 1;
    background: #c86666;
    border-color: #c86666;
    box-shadow: 0 0 0 2px rgba(200, 102, 102, 0.12);
  }

  .checkpoint-gutter:hover:not(:disabled) span {
    transform: scale(1.08);
  }

  .checkpoint-gutter:disabled {
    cursor: default;
  }

  .tail-spacer {
    width: 16px;
  }

  .sequence-row {
    width: 100%;
    display: grid;
    grid-template-columns: 22px 1fr;
    gap: var(--ui-space-2);
    align-items: center;
    border: 1px solid var(--app-border);
    background: #fff;
    color: inherit;
    text-align: left;
    min-height: var(--ui-list-row-min-height);
    padding: var(--ui-list-row-padding-y) var(--ui-list-row-padding-x);
    border-radius: var(--ui-radius-md);
    cursor: pointer;
  }

  .sequence-row:hover {
    background: var(--surface-bg);
  }

  .sequence-row.selected {
    border-color: #b0d4ee;
    background: #e8f3fb;
  }

  .tail-row {
    border-color: var(--info-border);
    background: #eef6fb;
  }

  .tail-row:hover {
    background: #e6f2fa;
  }

  .step-remove {
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

  .sequence-row-shell:hover .step-remove,
  .sequence-row:focus-visible ~ .step-remove,
  .step-remove:focus-visible {
    opacity: 1;
    pointer-events: auto;
  }

  .step-remove:hover {
    border-color: #e0b7b7;
    background: #f9ecec;
    color: #a74646;
  }

  .step-index {
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

  .step-copy {
    display: flex;
    flex-direction: column;
    gap: 1px;
    min-width: 0;
  }

  .step-copy strong {
    display: block;
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    font-size: 12px;
    font-weight: 500;
    color: var(--text-primary);
  }

  .step-note {
    font-size: 10px;
    letter-spacing: 0.04em;
    text-transform: uppercase;
    color: var(--info-text);
  }

  .empty-state {
    padding: var(--ui-space-6) var(--ui-space-4);
    color: var(--text-muted);
    font-size: 12px;
  }

  .empty-state p {
    margin: 0 0 5px;
  }
</style>
