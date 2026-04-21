<svelte:options runes={true} />

<script lang="ts">
  import { onMount } from "svelte";
  import type {
    CoordinateReferenceCatalogEntry,
    CoordinateReferenceSelection
  } from "../bridge";
  import { emitFrontendDiagnosticsEvent, searchCoordinateReferences } from "../bridge";

  interface Props {
    close: () => void;
    confirm: (selection: CoordinateReferenceSelection) => void;
    title?: string;
    description?: string;
    allowLocalEngineering?: boolean;
    localEngineeringLabel?: string;
    selectedAuthId?: string | null;
    projectRoot?: string | null;
    projectedOnly?: boolean;
    includeGeographic?: boolean;
    includeVertical?: boolean;
    recommendedEntries?: CoordinateReferenceCatalogEntry[];
  }

  const APP_RECENT_KEY = "traceboost.crs-picker.recent";
  const PROJECT_RECENT_KEY_PREFIX = "traceboost.crs-picker.project:";
  const MAX_RECENT = 8;

  let {
    close,
    confirm,
    title = "Select Coordinate Reference System",
    description = "Choose a validated CRS from the local registry.",
    allowLocalEngineering = false,
    localEngineeringLabel = "Survey local coordinates",
    selectedAuthId = null,
    projectRoot = null,
    projectedOnly = true,
    includeGeographic = true,
    includeVertical = false,
    recommendedEntries = []
  }: Props = $props();

  let query = $state("");
  let activeFilter = $state<"projected" | "geographic" | "all">("all");
  let loading = $state(false);
  let error = $state<string | null>(null);
  let entries = $state.raw<CoordinateReferenceCatalogEntry[]>([]);
  let recentEntries = $state.raw<CoordinateReferenceCatalogEntry[]>([]);
  let selectedEntry = $state.raw<CoordinateReferenceCatalogEntry | null>(null);

  function storageAvailable(): boolean {
    return typeof window !== "undefined" && typeof window.localStorage !== "undefined";
  }

  function uniqueEntriesInOrder(values: CoordinateReferenceCatalogEntry[]): CoordinateReferenceCatalogEntry[] {
    const results: CoordinateReferenceCatalogEntry[] = [];
    for (const value of values) {
      if (!results.some((entry) => entry.authId === value.authId)) {
        results.push(value);
      }
    }
    return results;
  }

  function normalizeRecommendedEntries(
    values: CoordinateReferenceCatalogEntry[]
  ): CoordinateReferenceCatalogEntry[] {
    return uniqueEntriesInOrder(values.filter((value) => value.authId.trim().length > 0));
  }

  function loadStoredEntries(key: string): CoordinateReferenceCatalogEntry[] {
    if (!storageAvailable()) {
      return [];
    }
    try {
      const stored = window.localStorage.getItem(key);
      if (!stored) {
        return [];
      }
      const parsed = JSON.parse(stored) as CoordinateReferenceCatalogEntry[];
      return normalizeRecommendedEntries(Array.isArray(parsed) ? parsed : []);
    } catch {
      return [];
    }
  }

  function saveStoredEntries(key: string, values: CoordinateReferenceCatalogEntry[]): void {
    if (!storageAvailable()) {
      return;
    }
    window.localStorage.setItem(key, JSON.stringify(values.slice(0, MAX_RECENT)));
  }

  function recentProjectKey(root: string | null): string | null {
    const normalized = root?.trim() ?? "";
    return normalized ? `${PROJECT_RECENT_KEY_PREFIX}${normalized}` : null;
  }

  function rememberEntry(entry: CoordinateReferenceCatalogEntry): void {
    const appEntries = uniqueEntriesInOrder([entry, ...loadStoredEntries(APP_RECENT_KEY)]);
    saveStoredEntries(APP_RECENT_KEY, appEntries);
    const projectKey = recentProjectKey(projectRoot);
    if (projectKey) {
      const projectEntries = uniqueEntriesInOrder([entry, ...loadStoredEntries(projectKey)]);
      saveStoredEntries(projectKey, projectEntries);
    }
  }

  function syncSelectedEntry(): void {
    const normalizedSelectedAuthId = selectedAuthId?.trim().toUpperCase() ?? "";
    selectedEntry =
      recentEntries.find((entry) => entry.authId.toUpperCase() === normalizedSelectedAuthId) ??
      recommendedEntries.find((entry) => entry.authId.toUpperCase() === normalizedSelectedAuthId) ??
      entries.find((entry) => entry.authId.toUpperCase() === normalizedSelectedAuthId) ??
      selectedEntry;
  }

  async function refreshEntries(): Promise<void> {
    loading = true;
    error = null;
    try {
      entries = await searchCoordinateReferences({
        query,
        limit: 24,
        projectedOnly: activeFilter === "projected",
        includeGeographic: activeFilter !== "projected" && (activeFilter === "all" || includeGeographic),
        includeVertical: activeFilter === "all" && includeVertical
      });
      if (!selectedEntry && entries[0]) {
        selectedEntry = entries[0];
      } else {
        syncSelectedEntry();
      }
    } catch (nextError) {
      error = nextError instanceof Error ? nextError.message : String(nextError);
    } finally {
      loading = false;
    }
  }

  function chooseEntry(entry: CoordinateReferenceCatalogEntry): void {
    selectedEntry = entry;
  }

  function confirmSelectedEntry(): void {
    if (!selectedEntry) {
      return;
    }
    rememberEntry(selectedEntry);
    void emitFrontendDiagnosticsEvent({
      stage: "crs_picker",
      level: "debug",
      message: "Confirmed coordinate reference selection.",
      fields: {
        authId: selectedEntry.authId,
        name: selectedEntry.name,
        source: "picker"
      }
    }).catch(() => {});
    confirm({
      kind: "authority_code",
      authority: selectedEntry.authority,
      code: selectedEntry.code,
      authId: selectedEntry.authId,
      name: selectedEntry.name
    });
  }

  function confirmLocalEngineering(): void {
    void emitFrontendDiagnosticsEvent({
      stage: "crs_picker",
      level: "debug",
      message: "Confirmed local engineering coordinate selection.",
      fields: {
        label: localEngineeringLabel
      }
    }).catch(() => {});
    confirm({
      kind: "local_engineering",
      label: localEngineeringLabel
    });
  }

  onMount(() => {
    activeFilter = projectedOnly ? "projected" : "all";
    recentEntries = normalizeRecommendedEntries([
      ...loadStoredEntries(APP_RECENT_KEY),
      ...(recentProjectKey(projectRoot) ? loadStoredEntries(recentProjectKey(projectRoot)!) : [])
    ]);
    syncSelectedEntry();
    void refreshEntries();
  });
</script>

<svelte:window
  onkeydown={(event) => {
    if (event.key === "Escape") {
      close();
    }
  }}
/>

<div class="picker-backdrop" role="presentation" onclick={(event) => event.target === event.currentTarget && close()}>
  <div
    class="picker-dialog"
    role="dialog"
    aria-modal="true"
    aria-label={title}
    tabindex="0"
    onclick={(event) => event.stopPropagation()}
    onkeydown={(event) => event.stopPropagation()}
  >
    <header class="picker-header">
      <div>
        <h3>{title}</h3>
        <p>{description}</p>
      </div>
      <button type="button" class="secondary" onclick={close}>Close</button>
    </header>

    <div class="picker-toolbar">
      <label class="picker-search">
        <span>Filter</span>
        <input
          bind:value={query}
          type="search"
          placeholder="EPSG code, identifier, or CRS name"
          oninput={() => void refreshEntries()}
        />
      </label>
      <div class="filter-strip" role="tablist" aria-label="CRS type filter">
        <button
          type="button"
          class={[activeFilter === "projected" && "active"]}
          onclick={() => {
            activeFilter = "projected";
            void refreshEntries();
          }}
        >
          Projected
        </button>
        <button
          type="button"
          class={[activeFilter === "geographic" && "active"]}
          onclick={() => {
            activeFilter = "geographic";
            void refreshEntries();
          }}
        >
          Geographic
        </button>
        <button
          type="button"
          class={[activeFilter === "all" && "active"]}
          onclick={() => {
            activeFilter = "all";
            void refreshEntries();
          }}
        >
          All
        </button>
      </div>
    </div>

    <div class="picker-layout">
      <div class="picker-column">
        {#if allowLocalEngineering}
          <section class="list-section">
            <h4>Quick Actions</h4>
            <button type="button" class="quick-action" onclick={confirmLocalEngineering}>
              <span>{localEngineeringLabel}</span>
              <small>Use local engineering coordinates without a resolved authority id.</small>
            </button>
          </section>
        {/if}

        {#if recommendedEntries.length}
          <section class="list-section">
            <h4>Recommended</h4>
            <div class="entry-list">
              {#each normalizeRecommendedEntries(recommendedEntries) as entry (entry.authId)}
                <button
                  type="button"
                  class={["entry-card", selectedEntry?.authId === entry.authId && "selected"]}
                  onclick={() => chooseEntry(entry)}
                >
                  <strong>{entry.name}</strong>
                  <span>{entry.authId}</span>
                </button>
              {/each}
            </div>
          </section>
        {/if}

        {#if recentEntries.length}
          <section class="list-section">
            <h4>Recent</h4>
            <div class="entry-list">
              {#each recentEntries as entry (entry.authId)}
                <button
                  type="button"
                  class={["entry-card", selectedEntry?.authId === entry.authId && "selected"]}
                  onclick={() => chooseEntry(entry)}
                >
                  <strong>{entry.name}</strong>
                  <span>{entry.authId}</span>
                </button>
              {/each}
            </div>
          </section>
        {/if}

        <section class="list-section grow">
          <h4>Registry Results</h4>
          {#if loading}
            <p class="status">Loading coordinate references...</p>
          {:else if error}
            <p class="status error">{error}</p>
          {:else if entries.length === 0}
            <p class="status">No coordinate references matched the current filter.</p>
          {:else}
            <div class="entry-list">
              {#each entries as entry (entry.authId)}
                <button
                  type="button"
                  class={["entry-card", selectedEntry?.authId === entry.authId && "selected"]}
                  onclick={() => chooseEntry(entry)}
                >
                  <strong>{entry.name}</strong>
                  <span>{entry.authId}</span>
                  <small>{entry.coordinateReferenceType.replaceAll("_", " ")}</small>
                </button>
              {/each}
            </div>
          {/if}
        </section>
      </div>

      <aside class="details-panel">
        <h4>Details</h4>
        {#if selectedEntry}
          <dl class="details-grid">
            <div>
              <dt>Name</dt>
              <dd>{selectedEntry.name}</dd>
            </div>
            <div>
              <dt>Authority ID</dt>
              <dd>{selectedEntry.authId}</dd>
            </div>
            <div>
              <dt>Type</dt>
              <dd>{selectedEntry.coordinateReferenceType.replaceAll("_", " ")}</dd>
            </div>
            <div>
              <dt>Area</dt>
              <dd>{selectedEntry.areaName ?? "Unknown"}</dd>
            </div>
          </dl>
          <div class="detail-actions">
            <button type="button" onclick={confirmSelectedEntry}>Use This CRS</button>
          </div>
        {:else}
          <p class="status">Choose a coordinate reference to inspect and confirm it.</p>
        {/if}
      </aside>
    </div>
  </div>
</div>

<style>
  .picker-backdrop {
    position: fixed;
    inset: 0;
    z-index: 110;
    display: flex;
    align-items: center;
    justify-content: center;
    padding: 24px;
    background: rgba(13, 19, 26, 0.22);
    backdrop-filter: blur(4px);
  }

  .picker-dialog {
    width: min(1040px, calc(100vw - 48px));
    max-height: min(840px, calc(100vh - 48px));
    display: grid;
    gap: 16px;
    padding: 20px;
    overflow: auto;
    border: 1px solid var(--app-border);
    border-radius: 8px;
    background: var(--panel-bg);
    color: var(--text-primary);
    box-shadow: 0 20px 60px rgba(42, 64, 84, 0.18);
  }

  .picker-header,
  .picker-toolbar {
    display: flex;
    align-items: flex-start;
    justify-content: space-between;
    gap: 12px;
  }

  .picker-header h3,
  .list-section h4,
  .details-panel h4 {
    margin: 0;
    font-size: 16px;
    font-weight: 650;
  }

  .picker-header p {
    margin: 4px 0 0;
    color: var(--text-muted);
  }

  .picker-search {
    display: grid;
    gap: 6px;
    flex: 1 1 280px;
  }

  .picker-search span,
  dt {
    font-size: 11px;
    text-transform: uppercase;
    color: var(--text-dim);
    letter-spacing: 0.04em;
  }

  .picker-search input {
    min-width: 0;
    padding: 9px 10px;
    border: 1px solid var(--app-border-strong);
    border-radius: 6px;
    background: #fff;
    color: var(--text-primary);
    font: inherit;
  }

  .filter-strip {
    display: flex;
    flex-wrap: wrap;
    gap: 8px;
  }

  .picker-layout {
    display: grid;
    grid-template-columns: minmax(0, 1.35fr) minmax(280px, 0.8fr);
    gap: 16px;
  }

  .picker-column,
  .details-panel,
  .list-section {
    display: grid;
    gap: 12px;
  }

  .picker-column {
    min-width: 0;
  }

  .grow {
    min-height: 240px;
  }

  .entry-list {
    display: grid;
    gap: 8px;
  }

  .entry-card,
  .quick-action {
    display: grid;
    gap: 4px;
    padding: 10px 12px;
    text-align: left;
    border: 1px solid var(--app-border);
    border-radius: 6px;
    background: #fff;
    color: var(--text-primary);
    cursor: pointer;
  }

  .entry-card.selected,
  .filter-strip button.active {
    border-color: var(--accent-border);
    background: rgba(69, 120, 165, 0.08);
  }

  .entry-card span,
  .entry-card small,
  .quick-action small,
  dd,
  .status {
    color: var(--text-muted);
  }

  .details-panel {
    align-content: start;
    padding: 14px;
    border: 1px solid var(--app-border);
    border-radius: 8px;
    background: var(--surface-bg);
  }

  .details-grid {
    display: grid;
    gap: 10px;
    margin: 0;
  }

  .details-grid div {
    display: grid;
    gap: 4px;
  }

  dd {
    margin: 0;
  }

  .detail-actions,
  .filter-strip {
    align-items: center;
  }

  button {
    padding: 9px 12px;
    border: 1px solid var(--accent-border);
    border-radius: 6px;
    background: var(--accent-bg);
    color: var(--accent-text);
    font: inherit;
    cursor: pointer;
  }

  button.secondary {
    border-color: var(--app-border-strong);
    background: var(--surface-subtle);
    color: var(--text-primary);
  }

  .status.error {
    color: #a53030;
  }

  @media (max-width: 960px) {
    .picker-dialog {
      width: calc(100vw - 20px);
      max-height: calc(100vh - 20px);
      padding: 16px;
    }

    .picker-layout {
      grid-template-columns: 1fr;
    }
  }
</style>
