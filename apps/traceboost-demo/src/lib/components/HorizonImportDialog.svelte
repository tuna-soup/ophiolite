<svelte:options runes={true} />

<script lang="ts">
  import { getViewerModelContext } from "../viewer-model.svelte";

  interface Props {
    inputPaths: string[];
    close: () => void;
  }

  let { inputPaths, close }: Props = $props();

  const viewerModel = getViewerModelContext();
  let sourceMode = $state<"survey" | "custom">(
    viewerModel.horizonImportSurveyModeBlocker ? "custom" : "survey"
  );
  let sourceCoordinateReferenceIdDraft = $state("");
  let sourceCoordinateReferenceNameDraft = $state("");

  async function confirmImport(): Promise<void> {
    if (inputPaths.length === 0) {
      close();
      return;
    }

    if (sourceMode === "survey" && viewerModel.horizonImportSurveyModeBlocker) {
      viewerModel.note(viewerModel.horizonImportSurveyModeBlocker, "ui", "warn");
      return;
    }

    if (sourceMode === "custom" && sourceCoordinateReferenceIdDraft.trim().length === 0) {
      viewerModel.note(
        "Enter a source CRS identifier before importing horizons with a custom CRS.",
        "ui",
        "warn"
      );
      return;
    }

    await viewerModel.importHorizonFiles(inputPaths, {
      assumeSameAsSurvey: sourceMode === "survey",
      sourceCoordinateReferenceId: sourceMode === "custom" ? sourceCoordinateReferenceIdDraft.trim() : null,
      sourceCoordinateReferenceName:
        sourceMode === "custom" ? sourceCoordinateReferenceNameDraft.trim() || null : null
    });

    if (!viewerModel.error) {
      close();
    }
  }

  function handleBackdropClick(event: MouseEvent): void {
    if (event.target === event.currentTarget && !viewerModel.horizonImporting) {
      close();
    }
  }
</script>

<svelte:window
  onkeydown={(event) => {
    if (event.key === "Escape" && !viewerModel.horizonImporting) {
      close();
    }
  }}
/>

<div class="dialog-backdrop" role="presentation" onclick={handleBackdropClick}>
  <div class="dialog" role="dialog" aria-modal="true" aria-label="Import horizons">
      <header>
        <h3>Import Horizons</h3>
        <p>Assign the source CRS for the selected horizon XYZ files.</p>
      </header>

      <div class="summary">
        <span>Files</span>
        <strong>{inputPaths.length}</strong>
      </div>

      {#if viewerModel.horizonImportProjectAdvisory}
        <div class="advisory">
          <strong>Project CRS advisory</strong>
          <p>{viewerModel.horizonImportProjectAdvisory}</p>
        </div>
      {/if}

      <label class="choice">
        <input
          type="radio"
          name="horizon-crs-mode"
          checked={sourceMode === "survey"}
          disabled={viewerModel.horizonImporting || !!viewerModel.horizonImportSurveyModeBlocker}
          onchange={() => {
            sourceMode = "survey";
          }}
        />
        <div>
          <strong>Use active survey CRS</strong>
          <p>
            {viewerModel.activeEffectiveNativeCoordinateReferenceId ?? "Unknown survey CRS"}
          </p>
          {#if viewerModel.horizonImportSurveyModeBlocker}
            <p>{viewerModel.horizonImportSurveyModeBlocker}</p>
          {/if}
        </div>
      </label>

      <label class="choice">
        <input
          type="radio"
          name="horizon-crs-mode"
          checked={sourceMode === "custom"}
          disabled={viewerModel.horizonImporting}
          onchange={() => {
            sourceMode = "custom";
          }}
        />
        <div>
          <strong>Specify source CRS</strong>
          <p>Use this when the horizon XYZ coordinates are not already in the survey CRS.</p>
        </div>
      </label>

      <label class="field">
        <span>Source CRS Identifier</span>
        <input
          bind:value={sourceCoordinateReferenceIdDraft}
          type="text"
          placeholder="EPSG:23031"
          disabled={sourceMode !== "custom" || viewerModel.horizonImporting}
        />
      </label>

      <label class="field">
        <span>Source CRS Label</span>
        <input
          bind:value={sourceCoordinateReferenceNameDraft}
          type="text"
          placeholder="ED50 / UTM zone 31N"
          disabled={sourceMode !== "custom" || viewerModel.horizonImporting}
        />
      </label>

      <div class="actions">
        <button type="button" class="secondary" onclick={close} disabled={viewerModel.horizonImporting}>
          Cancel
        </button>
        <button type="button" onclick={() => void confirmImport()}>
          {viewerModel.horizonImporting ? "Importing..." : "Import Horizons"}
        </button>
      </div>
  </div>
</div>

<style>
  .dialog-backdrop {
    position: fixed;
    inset: 0;
    z-index: 45;
    display: flex;
    align-items: center;
    justify-content: center;
    padding: 24px;
    background: rgba(38, 55, 71, 0.2);
    backdrop-filter: blur(4px);
  }

  .dialog {
    width: min(520px, calc(100vw - 48px));
    display: grid;
    gap: 14px;
    padding: 18px;
    border: 1px solid var(--app-border);
    border-radius: 8px;
    background: var(--panel-bg);
    color: var(--text-primary);
    box-shadow: 0 20px 60px rgba(42, 64, 84, 0.18);
  }

  header h3,
  .choice strong {
    margin: 0;
    font-size: 16px;
    font-weight: 650;
  }

  header p,
  .choice p {
    margin: 4px 0 0;
    color: var(--text-muted);
  }

  .summary,
  .advisory,
  .choice {
    display: grid;
    gap: 6px;
    padding: 10px 12px;
    border: 1px solid var(--app-border);
    border-radius: 6px;
    background: var(--surface-bg);
  }

  .summary span,
  .field span {
    font-size: 11px;
    text-transform: uppercase;
    color: var(--text-dim);
    letter-spacing: 0.04em;
  }

  .summary strong {
    font-size: 18px;
  }

  .advisory {
    background: rgba(252, 244, 236, 0.88);
    color: #7a5634;
  }

  .advisory strong {
    font-size: 13px;
  }

  .advisory p {
    margin: 0;
  }

  .choice {
    grid-template-columns: auto minmax(0, 1fr);
    align-items: start;
    gap: 10px;
    cursor: pointer;
  }

  .field {
    display: grid;
    gap: 6px;
  }

  .field input {
    min-width: 0;
    padding: 9px 10px;
    border: 1px solid var(--app-border-strong);
    border-radius: 6px;
    background: #fff;
    color: var(--text-primary);
    font: inherit;
  }

  .field input:disabled {
    background: var(--surface-subtle);
    color: var(--text-muted);
  }

  .actions {
    display: flex;
    justify-content: flex-end;
    gap: 8px;
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

  button:disabled {
    opacity: 0.55;
    cursor: not-allowed;
  }
</style>
