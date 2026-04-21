<svelte:options runes={true} />

<script lang="ts">
  import type { CoordinateReferenceSelection } from "../bridge";
  import { getViewerModelContext } from "../viewer-model.svelte";
  import CoordinateReferencePicker from "./CoordinateReferencePicker.svelte";

  interface Props {
    openSettings?: (source?: string) => void;
  }

  let { openSettings = () => {} }: Props = $props();

  const viewerModel = getViewerModelContext();

  let pickerOpen = $state(false);
  let feedback = $state<{ level: "error" | "info"; message: string } | null>(null);

  const prompt = $derived(viewerModel.missingNativeCoordinateReferencePrompt);
  const triggerLabel = $derived(prompt?.triggeredBy === "import" ? "imported" : "opened");
  const displayCoordinateReferenceLabel = $derived(
    prompt?.displayCoordinateReferenceId?.trim() || null
  );

  function dismissPrompt(): void {
    pickerOpen = false;
    feedback = null;
    viewerModel.dismissMissingNativeCoordinateReferencePrompt();
  }

  async function applySelection(
    coordinateReferenceId: string | null,
    coordinateReferenceName: string | null = null
  ): Promise<void> {
    const result = await viewerModel.applyMissingNativeCoordinateReferencePromptSelection(
      coordinateReferenceId,
      coordinateReferenceName
    );
    if (result.exactMatch) {
      feedback = null;
      pickerOpen = false;
      return;
    }
    feedback = {
      level: "error",
      message: result.error
        ? `TraceBoost could not assign the requested survey CRS. ${result.error}`
        : `TraceBoost applied ${
            result.effectiveCoordinateReferenceId ??
            result.effectiveCoordinateReferenceName ??
            "an unknown CRS"
          } instead of ${coordinateReferenceId ?? coordinateReferenceName ?? "the requested CRS"}.`
    };
  }

  async function handleCoordinateReferenceSelection(
    selection: CoordinateReferenceSelection
  ): Promise<void> {
    if (selection.kind !== "authority_code") {
      return;
    }
    await applySelection(selection.authId, selection.name?.trim() ?? null);
  }

  async function applyDisplayCoordinateReference(): Promise<void> {
    if (!prompt?.displayCoordinateReferenceId) {
      return;
    }
    await applySelection(
      prompt.displayCoordinateReferenceId,
      prompt.displayCoordinateReferenceName ?? null
    );
  }
</script>

<svelte:window
  onkeydown={(event) => {
    if (event.key === "Escape" && !pickerOpen && prompt) {
      dismissPrompt();
    }
  }}
/>

{#if prompt}
  <div
    class="prompt-backdrop"
    role="presentation"
    onclick={(event) => event.target === event.currentTarget && dismissPrompt()}
  >
    <div
      class="prompt-dialog"
      role="dialog"
      aria-modal="true"
      aria-label="Survey CRS not declared"
      tabindex="0"
      onclick={(event) => event.stopPropagation()}
      onkeydown={(event) => event.stopPropagation()}
    >
      <header class="prompt-header">
        <div>
          <h3>Survey CRS Not Declared</h3>
          <p>
            <strong>{prompt.datasetDisplayName}</strong> was {triggerLabel} without a trustworthy
            source CRS.
          </p>
        </div>
      </header>

      <div class="prompt-body">
        <p>
          Continue in native engineering coordinates for section viewing and processing, or assign a
          native CRS now for maps, overlays, and cross-survey alignment.
        </p>
        <p>This choice is remembered for this workspace session.</p>

        {#if prompt.sourcePath}
          <p class="source-path">{prompt.sourcePath}</p>
        {/if}

        {#if feedback}
          <p class={["status", feedback.level === "error" && "error"]}>{feedback.message}</p>
        {/if}
      </div>

      <div class="action-row">
        <button type="button" onclick={dismissPrompt}>Keep Native Engineering</button>
        {#if displayCoordinateReferenceLabel}
          <button type="button" class="secondary" onclick={() => void applyDisplayCoordinateReference()}>
            Use Display CRS {displayCoordinateReferenceLabel}
          </button>
        {/if}
        <button type="button" class="secondary" onclick={() => (pickerOpen = true)}>
          Choose Survey CRS
        </button>
        <button
          type="button"
          class="secondary"
          onclick={() => openSettings("missing_native_crs_prompt")}
        >
          Project Settings
        </button>
      </div>
    </div>
  </div>

  {#if pickerOpen}
    <CoordinateReferencePicker
      close={() => {
        pickerOpen = false;
      }}
      confirm={(selection) => void handleCoordinateReferenceSelection(selection)}
      title="Choose Survey Native CRS"
      description="Select the source CRS used to interpret this survey's raw X/Y coordinates."
      projectRoot={viewerModel.projectRoot}
      projectedOnly={false}
      includeGeographic={true}
      includeVertical={false}
      selectedAuthId={prompt.displayCoordinateReferenceId}
    />
  {/if}
{/if}

<style>
  .prompt-backdrop {
    position: fixed;
    inset: 0;
    z-index: 2500;
    display: grid;
    place-items: center;
    background: rgba(16, 23, 32, 0.34);
    backdrop-filter: blur(4px);
    padding: 24px;
  }

  .prompt-dialog {
    width: min(720px, 100%);
    border-radius: 8px;
    background: #f8fbff;
    border: 1px solid rgba(120, 145, 170, 0.28);
    box-shadow: 0 28px 60px rgba(15, 23, 42, 0.16);
    display: grid;
    gap: 18px;
    padding: 22px;
  }

  .prompt-header,
  .prompt-body {
    display: grid;
    gap: 10px;
  }

  .prompt-header h3,
  .prompt-header p,
  .prompt-body p {
    margin: 0;
  }

  .prompt-header h3 {
    font-size: 1.3rem;
    font-weight: 700;
    color: #1f2937;
  }

  .prompt-header p,
  .prompt-body p {
    color: #52657a;
    line-height: 1.55;
  }

  .source-path {
    font-family: "SFMono-Regular", ui-monospace, "Menlo", "Monaco", "Roboto Mono", monospace;
    font-size: 0.85rem;
    color: #475569;
    background: rgba(226, 232, 240, 0.65);
    border: 1px solid rgba(148, 163, 184, 0.3);
    border-radius: 6px;
    padding: 10px 12px;
    overflow-wrap: anywhere;
  }

  .action-row {
    display: flex;
    flex-wrap: wrap;
    gap: 10px;
  }

  .action-row button {
    border-radius: 6px;
    border: 1px solid rgba(37, 99, 235, 0.22);
    background: #2563eb;
    color: #fff;
    padding: 10px 14px;
    font: inherit;
    cursor: pointer;
  }

  .action-row button.secondary {
    background: #fff;
    color: #334155;
  }

  .status {
    border-radius: 6px;
    padding: 10px 12px;
    background: rgba(59, 130, 246, 0.08);
    border: 1px solid rgba(59, 130, 246, 0.2);
  }

  .status.error {
    background: rgba(239, 68, 68, 0.08);
    border-color: rgba(239, 68, 68, 0.24);
    color: #991b1b;
  }

  @media (max-width: 720px) {
    .prompt-backdrop {
      padding: 14px;
    }

    .prompt-dialog {
      padding: 18px;
    }

    .action-row {
      flex-direction: column;
    }

    .action-row button {
      width: 100%;
    }
  }
</style>
