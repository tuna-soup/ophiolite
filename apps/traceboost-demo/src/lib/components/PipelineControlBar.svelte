<svelte:options runes={true} />

<script lang="ts">
  import type {
    TraceLocalProcessingPipeline as ProcessingPipeline,
    TraceLocalProcessingPreset as ProcessingPreset
  } from "@traceboost/seis-contracts";

  let {
    pipeline,
    previewState,
    previewLabel,
    presets,
    loadingPresets,
    canPreview,
    canRun,
    previewBusy,
    runBusy,
    runOutputSettingsOpen,
    runOutputPathMode,
    runOutputPath,
    resolvingRunOutputPath,
    overwriteExistingRunOutput,
    onSetPipelineName,
    onPreview,
    onShowRaw,
    onRun,
    onToggleRunOutputSettings,
    onSetRunOutputPathMode,
    onSetCustomRunOutputPath,
    onBrowseRunOutputPath,
    onResetRunOutputPath,
    onSetOverwriteExistingRunOutput,
    onLoadPreset,
    onSavePreset,
    onDeletePreset
  }: {
    pipeline: ProcessingPipeline;
    previewState: "raw" | "preview" | "stale";
    previewLabel: string | null;
    presets: ProcessingPreset[];
    loadingPresets: boolean;
    canPreview: boolean;
    canRun: boolean;
    previewBusy: boolean;
    runBusy: boolean;
    runOutputSettingsOpen: boolean;
    runOutputPathMode: "default" | "custom";
    runOutputPath: string | null;
    resolvingRunOutputPath: boolean;
    overwriteExistingRunOutput: boolean;
    onSetPipelineName: (value: string) => void;
    onPreview: () => void | Promise<void>;
    onShowRaw: () => void;
    onRun: () => void | Promise<void>;
    onToggleRunOutputSettings: () => void;
    onSetRunOutputPathMode: (mode: "default" | "custom") => void;
    onSetCustomRunOutputPath: (value: string) => void;
    onBrowseRunOutputPath: () => void | Promise<void>;
    onResetRunOutputPath: () => void;
    onSetOverwriteExistingRunOutput: (value: boolean) => void;
    onLoadPreset: (preset: ProcessingPreset) => void;
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
      <span>Pipeline Name</span>
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
          <option value={preset.preset_id}>{preset.pipeline.name ?? preset.preset_id}</option>
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
