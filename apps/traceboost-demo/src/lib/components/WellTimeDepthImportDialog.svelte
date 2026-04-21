<svelte:options runes={true} />

<script lang="ts">
  import { onMount } from "svelte";
  import type {
    ImportProjectWellTimeDepthModelResponse,
    ProjectWellTimeDepthImportCanonicalDraft,
    ProjectWellTimeDepthImportPreview,
    PreviewProjectWellTimeDepthAssetRequest,
    ProjectAssetBindingInput
  } from "../bridge";
  import {
    previewProjectWellTimeDepthImport
  } from "../bridge";
  import { getViewerModelContext } from "../viewer-model.svelte";
  import ImportFlowStepper from "./ImportFlowStepper.svelte";
  import ImportReviewChecklist from "./ImportReviewChecklist.svelte";
  import ImportReviewFieldSection from "./ImportReviewFieldSection.svelte";
  import {
    compactImportReviewFields,
    type ImportConfirmationStage,
    type ImportFlowStep,
    type ImportReviewItem,
    type ImportReviewSection
  } from "./import-review";

  interface Props {
    projectRoot: string;
    jsonPath: string;
    binding: ProjectAssetBindingInput;
    assetKind: PreviewProjectWellTimeDepthAssetRequest["assetKind"];
    dialogTitle: string;
    openSettings: () => void;
    close: () => void;
  }

  let { projectRoot, jsonPath, binding, assetKind, dialogTitle, openSettings, close }: Props =
    $props();

  const viewerModel = getViewerModelContext();

  let importPreview = $state.raw<ProjectWellTimeDepthImportPreview | null>(null);
  let importResult = $state.raw<ImportProjectWellTimeDepthModelResponse | null>(null);
  let previewLoading = $state(true);
  let committing = $state(false);
  let previewErrorMessage = $state<string | null>(null);
  let commitErrorMessage = $state<string | null>(null);
  let collectionNameDraft = $state("");
  let payloadDraft = $state("");
  let filePayloadJson = $state("");
  let usingEditedPayload = $state(false);
  let payloadDirty = $state(false);
  let stage = $state<ImportConfirmationStage>("configure");
  let showRawJson = $state(false);

  type EditablePayload = Record<string, unknown>;

  const DEPTH_REFERENCE_OPTIONS = [
    { value: "measured_depth", label: "Measured Depth" },
    { value: "true_vertical_depth", label: "True Vertical Depth" },
    { value: "true_vertical_depth_subsea", label: "True Vertical Depth Subsea" }
  ] as const;
  const TRAVEL_TIME_REFERENCE_OPTIONS = [
    { value: "one_way", label: "One-Way" },
    { value: "two_way", label: "Two-Way" }
  ] as const;
  const SOURCE_KIND_OPTIONS = [
    { value: "constant_velocity", label: "Constant Velocity" },
    { value: "velocity_function1_d", label: "Velocity Function 1D" },
    { value: "velocity_grid3_d", label: "Velocity Grid 3D" },
    { value: "checkshot_model1_d", label: "Checkshot Model 1D" },
    { value: "sonic_log1_d", label: "Sonic Log 1D" },
    { value: "vp_log1_d", label: "Vp Log 1D" },
    { value: "horizon_layer_model", label: "Horizon Layer Model" },
    { value: "well_tie_observation_set1_d", label: "Well Tie Observation Set 1D" }
  ] as const;

  const assetKindLabel = $derived(labelForAssetKind(assetKind));
  const preview = $derived(importPreview?.parsed ?? null);
  const canonicalDraft = $derived.by((): ProjectWellTimeDepthImportCanonicalDraft => ({
    assetKind,
    jsonPayload: payloadDraft.trim().length > 0 ? payloadDraft : filePayloadJson,
    collectionName: blankToNull(collectionNameDraft)
  }));
  const importStatusLabel = $derived.by(() => {
    if (previewLoading) {
      return "Parsing";
    }
    if (previewErrorMessage) {
      return "Preview failed";
    }
    if (!preview) {
      return "Waiting for preview";
    }
    return preview.canImport ? "Ready to import" : "Preview only";
  });
  const payloadSourceLabel = $derived(usingEditedPayload ? "Edited payload" : "File payload");
  const editablePayloadState = $derived.by(() => parseEditablePayload(payloadDraft));
  const previewBlockingIssueCount = $derived.by(
    () => preview?.issues.filter((issue) => issue.severity === "blocking").length ?? 0
  );
  const previewWarningIssueCount = $derived.by(
    () => preview?.issues.filter((issue) => issue.severity === "warning").length ?? 0
  );
  const targetFields = $derived.by(() =>
    compactImportReviewFields([
      ["Project Root", projectRoot],
      ["Target Well", binding.well_name],
      ["Target Wellbore", binding.wellbore_name],
      ["Collection Name", collectionNameDraft.trim() || "Use parsed asset name"],
      ["Asset Kind", assetKindLabel],
      ["Source File", jsonPath]
    ])
  );
  const parsedFields = $derived.by(() =>
    compactImportReviewFields([
      ["Asset Name", preview?.name],
      ["Asset Id", preview?.id],
      ["File Wellbore Id", preview?.wellboreId],
      ["Depth Reference", preview?.depthReference],
      ["Travel Time Reference", preview?.travelTimeReference],
      ["Sample Count", countLabel(preview?.sampleCount)],
      ["Note Count", countLabel(preview?.noteCount)]
    ])
  );
  const modelFields = $derived.by(() =>
    compactImportReviewFields([
      ["Source Kind", preview?.sourceKind],
      ["Source Binding Count", countLabel(preview?.sourceBindingCount)],
      ["Assumption Intervals", countLabel(preview?.assumptionIntervalCount)],
      ["Sampling Step (m)", numberLabel(preview?.samplingStepM)],
      ["Trajectory Fingerprint", preview?.resolvedTrajectoryFingerprint],
      ["Source Model Asset", preview?.sourceWellTimeDepthModelAssetId],
      ["Tie Window Start (ms)", numberLabel(preview?.tieWindowStartMs)],
      ["Tie Window End (ms)", numberLabel(preview?.tieWindowEndMs)],
      ["Trace Search Radius (m)", numberLabel(preview?.traceSearchRadiusM)],
      ["Bulk Shift (ms)", numberLabel(preview?.bulkShiftMs)],
      ["Stretch Factor", numberLabel(preview?.stretchFactor)],
      ["Trace Search Offset (m)", numberLabel(preview?.traceSearchOffsetM)],
      ["Correlation", numberLabel(preview?.correlation)]
    ])
  );
  const outcomeFields = $derived.by(() =>
    compactImportReviewFields([
      ["Import Status", importStatusLabel],
      ["Blocking Issues", countLabel(previewBlockingIssueCount)],
      ["Warnings", countLabel(previewWarningIssueCount)],
      ["Can Commit", preview ? (preview.canImport ? "Yes" : "No") : null]
    ])
  );
  const resultFields = $derived.by(() =>
    compactImportReviewFields([
      ["Imported Asset Id", importResult?.assetId],
      ["Well Id", importResult?.wellId],
      ["Wellbore Id", importResult?.wellboreId],
      ["Created Well", importResult ? yesNoLabel(importResult.createdWell) : null],
      ["Created Wellbore", importResult ? yesNoLabel(importResult.createdWellbore) : null]
    ])
  );
  const reviewSections = $derived.by((): ImportReviewSection[] => [
    {
      title: "Import Target",
      fields: targetFields
    },
    {
      title: "Parsed Asset",
      fields: parsedFields
    },
    {
      title: "Model Details",
      fields: modelFields,
      emptyMessage: "No additional model-specific fields were detected."
    },
    {
      title: "Validation",
      fields: outcomeFields
    }
  ]);
  const reviewItems = $derived.by(() => {
    const items: ImportReviewItem[] = [];
    if (previewLoading) {
      items.push({
        severity: "info",
        title: "Preview is still running",
        message: "Wait for the parse and validation pass to finish before confirming the import."
      });
      return items;
    }
    if (previewErrorMessage) {
      items.push({
        severity: "blocking",
        title: "Preview failed",
        message: previewErrorMessage
      });
      return items;
    }
    if (!preview) {
      items.push({
        severity: "blocking",
        title: "Preview is unavailable",
        message: "Retry the preview before attempting to import this asset."
      });
      return items;
    }
    if (!preview.canImport) {
      items.push({
        severity: "blocking",
        title: "Import is blocked",
        message: "The file was parsed for review, but the backend validation path still rejects the payload."
      });
    }
    if (!collectionNameDraft.trim()) {
      items.push({
        severity: "info",
        title: "Collection name will be inferred",
        message: "The project collection will default to the parsed asset name when available."
      });
    }
    if (previewWarningIssueCount > 0) {
      items.push({
        severity: "warning",
        title: "Preview reported warnings",
        message: `${previewWarningIssueCount} warning${previewWarningIssueCount === 1 ? "" : "s"} should be reviewed before confirmation.`
      });
    }
    if (preview.issues.length === 0) {
      items.push({
        severity: "info",
        title: "Ready to confirm",
        message: "The parsed asset is aligned with the current import target."
      });
    }
    return items;
  });
  const flowSteps = $derived.by((): ImportFlowStep[] => [
    {
      key: "configure",
      label: "1. Configure",
      description: "Inspect the parsed asset and set the optional collection name."
    },
    {
      key: "review",
      label: "2. Review",
      description: "Check the translated asset summary and confirm the import.",
      disabled: previewLoading || (!!previewErrorMessage && !preview)
    },
    {
      key: "result",
      label: "3. Result",
      description: "Review the imported project asset identifiers.",
      disabled: importResult === null
    }
  ]);
  const confirmDisabledReason = $derived.by(() => {
    if (previewLoading) {
      return "Wait for the preview to finish first.";
    }
    if (previewErrorMessage) {
      return "The preview must succeed before this asset can be imported.";
    }
    if (!preview) {
      return "No preview is available yet.";
    }
    if (payloadDirty) {
      return "Re-run preview after editing the payload.";
    }
    if (!preview.canImport) {
      return "The parsed payload still has blocking validation issues.";
    }
    if (committing) {
      return "Import is already running.";
    }
    return null;
  });
  const confirmButtonLabel = $derived(committing ? "Importing..." : "Confirm Import");

  onMount(() => {
    void loadPreview();
  });

  function goToConfigure(): void {
    if (committing) {
      return;
    }
    stage = "configure";
  }

  function goToReview(): void {
    if (previewLoading || committing || previewErrorMessage || !preview) {
      return;
    }
    stage = "review";
  }

  async function loadPreview(): Promise<void> {
    previewLoading = true;
    previewErrorMessage = null;
    commitErrorMessage = null;
    importResult = null;
    importPreview = null;
    stage = "configure";
    try {
      const nextPreview = await previewProjectWellTimeDepthImport({
        jsonPath,
        draft:
          usingEditedPayload || collectionNameDraft.trim().length > 0 || filePayloadJson.length > 0
            ? canonicalDraft
            : null,
        assetKind
      });
      importPreview = nextPreview;
      if (!usingEditedPayload) {
        filePayloadJson = nextPreview.suggestedDraft.jsonPayload;
      }
      payloadDraft = nextPreview.suggestedDraft.jsonPayload;
      if (!collectionNameDraft.trim() && nextPreview.suggestedDraft.collectionName) {
        collectionNameDraft = nextPreview.suggestedDraft.collectionName;
      }
      payloadDirty = false;
    } catch (error) {
      previewErrorMessage = error instanceof Error ? error.message : String(error);
      viewerModel.note(
        "Failed to preview project well time-depth asset.",
        "backend",
        "warn",
        previewErrorMessage
      );
    } finally {
      previewLoading = false;
    }
  }

  async function confirmImport(): Promise<void> {
    if (confirmDisabledReason) {
      return;
    }
    committing = true;
    commitErrorMessage = null;
    try {
      const response = await viewerModel.importProjectWellTimeDepthDraft({
        projectRoot,
        jsonPath,
        binding,
        draft: canonicalDraft
      });
      await viewerModel.refreshProjectWellOverlayInventory(
        projectRoot,
        viewerModel.displayCoordinateReferenceId
      );
      await viewerModel.refreshProjectWellTimeDepthModels(projectRoot, response.wellboreId);
      openSettings();
      importResult = response;
      stage = "result";
    } catch (error) {
      commitErrorMessage = error instanceof Error ? error.message : String(error);
      viewerModel.note(
        "Failed to import project well time-depth asset.",
        "backend",
        "warn",
        commitErrorMessage
      );
    } finally {
      committing = false;
    }
  }

  function blankToNull(value: string): string | null {
    const trimmed = value.trim();
    return trimmed.length > 0 ? trimmed : null;
  }

  function labelForAssetKind(value: PreviewProjectWellTimeDepthAssetRequest["assetKind"]): string {
    switch (value) {
      case "checkshot_vsp_observation_set":
        return "Checkshot / VSP Observation Set";
      case "manual_time_depth_pick_set":
        return "Manual Time-Depth Pick Set";
      case "well_tie_observation_set":
        return "Well Tie Observation Set";
      case "well_time_depth_authored_model":
        return "Well Time-Depth Authored Model";
      case "well_time_depth_model":
        return "Compiled Well Time-Depth Model";
    }
  }

  function countLabel(value: number | null | undefined): string | null {
    return typeof value === "number" ? String(value) : null;
  }

  function numberLabel(value: number | null | undefined): string | null {
    return typeof value === "number" && Number.isFinite(value) ? String(value) : null;
  }

  function yesNoLabel(value: boolean): string {
    return value ? "Yes" : "No";
  }

  function handlePayloadInput(event: Event): void {
    payloadDraft = (event.currentTarget as HTMLTextAreaElement).value;
    usingEditedPayload = true;
    payloadDirty = true;
  }

  function resetPayloadToFile(): void {
    payloadDraft = filePayloadJson;
    usingEditedPayload = false;
    payloadDirty = false;
    void loadPreview();
  }

  function parseEditablePayload(value: string): {
    payload: EditablePayload | null;
    error: string | null;
  } {
    if (!value.trim()) {
      return { payload: null, error: "The payload is empty." };
    }
    try {
      const parsed = JSON.parse(value) as unknown;
      if (!parsed || typeof parsed !== "object" || Array.isArray(parsed)) {
        return { payload: null, error: "The top-level payload must be a JSON object." };
      }
      return { payload: parsed as EditablePayload, error: null };
    } catch (error) {
      return {
        payload: null,
        error: error instanceof Error ? error.message : String(error)
      };
    }
  }

  function payloadStringField(fieldName: string): string {
    const value = editablePayloadState.payload?.[fieldName];
    return typeof value === "string" ? value : "";
  }

  function payloadOptionalNumberField(fieldName: string): string {
    const value = editablePayloadState.payload?.[fieldName];
    if (typeof value === "number" && Number.isFinite(value)) {
      return String(value);
    }
    return typeof value === "string" ? value : "";
  }

  function payloadNotesText(): string {
    const value = editablePayloadState.payload?.notes;
    if (!Array.isArray(value)) {
      return "";
    }
    return value.filter((entry): entry is string => typeof entry === "string").join("\n");
  }

  function patchPayloadObject(mutator: (payload: EditablePayload) => void): void {
    const existing = editablePayloadState.payload;
    if (!existing) {
      return;
    }
    const nextPayload = JSON.parse(JSON.stringify(existing)) as EditablePayload;
    mutator(nextPayload);
    payloadDraft = JSON.stringify(nextPayload, null, 2);
    usingEditedPayload = true;
    payloadDirty = true;
  }

  function patchPayloadField(fieldName: string, value: string | null): void {
    patchPayloadObject((payload) => {
      const trimmed = value?.trim() ?? "";
      if (trimmed.length === 0) {
        delete payload[fieldName];
        return;
      }
      payload[fieldName] = trimmed;
    });
  }

  function patchPayloadNumberField(fieldName: string, value: string): void {
    patchPayloadObject((payload) => {
      const trimmed = value.trim();
      if (!trimmed.length) {
        delete payload[fieldName];
        return;
      }
      const parsed = Number(trimmed);
      payload[fieldName] = Number.isFinite(parsed) ? parsed : trimmed;
    });
  }

  function patchPayloadNotes(value: string): void {
    patchPayloadObject((payload) => {
      const notes = value
        .split(/\r?\n/u)
        .map((entry) => entry.trim())
        .filter((entry) => entry.length > 0);
      if (notes.length === 0) {
        delete payload.notes;
        return;
      }
      payload.notes = notes;
    });
  }
</script>

<div class="well-time-depth-import-backdrop" role="presentation">
  <div
    class="well-time-depth-import-dialog"
    role="dialog"
    aria-modal="true"
    aria-labelledby="well-time-depth-import-title"
    tabindex="-1"
    onclick={(event) => event.stopPropagation()}
    onkeydown={(event) => {
      if (event.key === "Escape" && !committing) {
        close();
      }
    }}
  >
    <header class="dialog-header">
      <div>
        <h2 id="well-time-depth-import-title">{dialogTitle}</h2>
        <p>{jsonPath}</p>
      </div>
      <button class="ghost-button" type="button" onclick={() => !committing && close()}>
        Close
      </button>
    </header>

    <div class="dialog-body">
      <section class="summary-grid">
        <div>
          <span class="summary-label">Asset Kind</span>
          <strong>{assetKindLabel}</strong>
        </div>
        <div>
          <span class="summary-label">Project Root</span>
          <strong>{projectRoot}</strong>
        </div>
        <div>
          <span class="summary-label">Target Wellbore</span>
          <strong>{binding.well_name} / {binding.wellbore_name}</strong>
        </div>
        <div>
          <span class="summary-label">Status</span>
          <strong>{importStatusLabel}</strong>
        </div>
      </section>

      <section class="section-block">
        <h3>Import Flow</h3>
        <ImportFlowStepper
          {stage}
          steps={flowSteps}
          onSelect={(nextStage) => {
            if (nextStage === "configure") {
              goToConfigure();
            } else if (nextStage === "review") {
              goToReview();
            }
          }}
        />
      </section>

      {#if previewLoading}
        <section class="section-block">
          <div class="status-block">
            <strong>Parsing asset</strong>
            <p>Loading the JSON payload and checking whether it is ready for import.</p>
          </div>
        </section>
      {:else if previewErrorMessage}
        <section class="section-block">
          <div class={["status-block", "status-error"]}>
            <strong>Preview failed</strong>
            <p>{previewErrorMessage}</p>
          </div>
          <button class="primary-button" type="button" onclick={loadPreview}>
            Retry Preview
          </button>
        </section>
      {:else if preview}
        {#if stage === "configure"}
          <section class="section-block">
            <h3>Import Target</h3>
            <div class="field-grid">
              <label>
                <span>Collection Name</span>
                <input
                  bind:value={collectionNameDraft}
                  placeholder={preview.name ?? "Use parsed asset name"}
                />
              </label>
            </div>
            <p class="section-copy">
              Leave the collection name blank to keep the parsed asset name as the project-facing label.
            </p>
          </section>

          <section class="section-block">
            <div>
              <h3>Parsed Fields</h3>
              <p class="section-copy">
                Adjust common canonical fields here. Keep using the raw JSON editor for sample arrays,
                source bindings, and other nested structures.
              </p>
            </div>
            {#if editablePayloadState.error}
              <div class={["status-block", "status-warning"]}>
                <strong>Structured editing unavailable</strong>
                <p>{editablePayloadState.error}</p>
              </div>
            {:else}
              <div class="field-grid">
                <label>
                  <span>Asset Id</span>
                  <input
                    value={payloadStringField("id")}
                    oninput={(event) =>
                      patchPayloadField("id", (event.currentTarget as HTMLInputElement).value)}
                  />
                </label>
                <label>
                  <span>Asset Name</span>
                  <input
                    value={payloadStringField("name")}
                    oninput={(event) =>
                      patchPayloadField("name", (event.currentTarget as HTMLInputElement).value)}
                  />
                </label>
                <label>
                  <span>Wellbore Id</span>
                  <input
                    value={payloadStringField("wellbore_id")}
                    oninput={(event) =>
                      patchPayloadField(
                        "wellbore_id",
                        (event.currentTarget as HTMLInputElement).value
                      )}
                  />
                </label>
                <label>
                  <span>Depth Reference</span>
                  <select
                    value={payloadStringField("depth_reference")}
                    onchange={(event) =>
                      patchPayloadField(
                        "depth_reference",
                        (event.currentTarget as HTMLSelectElement).value
                      )}
                  >
                    <option value="">Unset</option>
                    {#each DEPTH_REFERENCE_OPTIONS as option (option.value)}
                      <option value={option.value}>{option.label}</option>
                    {/each}
                  </select>
                </label>
                <label>
                  <span>Travel Time Reference</span>
                  <select
                    value={payloadStringField("travel_time_reference")}
                    onchange={(event) =>
                      patchPayloadField(
                        "travel_time_reference",
                        (event.currentTarget as HTMLSelectElement).value
                      )}
                  >
                    <option value="">Unset</option>
                    {#each TRAVEL_TIME_REFERENCE_OPTIONS as option (option.value)}
                      <option value={option.value}>{option.label}</option>
                    {/each}
                  </select>
                </label>
                {#if assetKind === "well_time_depth_model"}
                  <label>
                    <span>Source Kind</span>
                    <select
                      value={payloadStringField("source_kind")}
                      onchange={(event) =>
                        patchPayloadField(
                          "source_kind",
                          (event.currentTarget as HTMLSelectElement).value
                        )}
                    >
                      <option value="">Unset</option>
                      {#each SOURCE_KIND_OPTIONS as option (option.value)}
                        <option value={option.value}>{option.label}</option>
                      {/each}
                    </select>
                  </label>
                {/if}
                {#if assetKind === "well_time_depth_authored_model"}
                  <label>
                    <span>Resolved Trajectory Fingerprint</span>
                    <input
                      value={payloadStringField("resolved_trajectory_fingerprint")}
                      oninput={(event) =>
                        patchPayloadField(
                          "resolved_trajectory_fingerprint",
                          (event.currentTarget as HTMLInputElement).value
                        )}
                    />
                  </label>
                  <label>
                    <span>Sampling Step (m)</span>
                    <input
                      inputmode="decimal"
                      value={payloadOptionalNumberField("sampling_step_m")}
                      oninput={(event) =>
                        patchPayloadNumberField(
                          "sampling_step_m",
                          (event.currentTarget as HTMLInputElement).value
                        )}
                    />
                  </label>
                {/if}
                {#if assetKind === "well_tie_observation_set"}
                  <label>
                    <span>Source Model Asset Id</span>
                    <input
                      value={payloadStringField("source_well_time_depth_model_asset_id")}
                      oninput={(event) =>
                        patchPayloadField(
                          "source_well_time_depth_model_asset_id",
                          (event.currentTarget as HTMLInputElement).value
                        )}
                    />
                  </label>
                  <label>
                    <span>Tie Window Start (ms)</span>
                    <input
                      inputmode="decimal"
                      value={payloadOptionalNumberField("tie_window_start_ms")}
                      oninput={(event) =>
                        patchPayloadNumberField(
                          "tie_window_start_ms",
                          (event.currentTarget as HTMLInputElement).value
                        )}
                    />
                  </label>
                  <label>
                    <span>Tie Window End (ms)</span>
                    <input
                      inputmode="decimal"
                      value={payloadOptionalNumberField("tie_window_end_ms")}
                      oninput={(event) =>
                        patchPayloadNumberField(
                          "tie_window_end_ms",
                          (event.currentTarget as HTMLInputElement).value
                        )}
                    />
                  </label>
                  <label>
                    <span>Trace Search Radius (m)</span>
                    <input
                      inputmode="decimal"
                      value={payloadOptionalNumberField("trace_search_radius_m")}
                      oninput={(event) =>
                        patchPayloadNumberField(
                          "trace_search_radius_m",
                          (event.currentTarget as HTMLInputElement).value
                        )}
                    />
                  </label>
                  <label>
                    <span>Bulk Shift (ms)</span>
                    <input
                      inputmode="decimal"
                      value={payloadOptionalNumberField("bulk_shift_ms")}
                      oninput={(event) =>
                        patchPayloadNumberField(
                          "bulk_shift_ms",
                          (event.currentTarget as HTMLInputElement).value
                        )}
                    />
                  </label>
                  <label>
                    <span>Stretch Factor</span>
                    <input
                      inputmode="decimal"
                      value={payloadOptionalNumberField("stretch_factor")}
                      oninput={(event) =>
                        patchPayloadNumberField(
                          "stretch_factor",
                          (event.currentTarget as HTMLInputElement).value
                        )}
                    />
                  </label>
                  <label>
                    <span>Trace Search Offset (m)</span>
                    <input
                      inputmode="decimal"
                      value={payloadOptionalNumberField("trace_search_offset_m")}
                      oninput={(event) =>
                        patchPayloadNumberField(
                          "trace_search_offset_m",
                          (event.currentTarget as HTMLInputElement).value
                        )}
                    />
                  </label>
                  <label>
                    <span>Correlation</span>
                    <input
                      inputmode="decimal"
                      value={payloadOptionalNumberField("correlation")}
                      oninput={(event) =>
                        patchPayloadNumberField(
                          "correlation",
                          (event.currentTarget as HTMLInputElement).value
                        )}
                    />
                  </label>
                {/if}
                <label class="wide-field">
                  <span>Notes</span>
                  <textarea
                    class="field-textarea"
                    value={payloadNotesText()}
                    oninput={(event) =>
                      patchPayloadNotes((event.currentTarget as HTMLTextAreaElement).value)}
                  ></textarea>
                </label>
              </div>
            {/if}
          </section>

          <section class="section-block">
            <div class="payload-header">
              <div>
                <h3>Payload Editor</h3>
                <p class="section-copy">
                  {payloadSourceLabel}. Edit the JSON, then re-run preview before import.
                </p>
              </div>
              <div class="payload-actions">
                <button
                  class="ghost-button"
                  type="button"
                  onclick={() => void loadPreview()}
                  disabled={previewLoading || committing}
                >
                  Re-run Preview
                </button>
                <button
                  class="ghost-button"
                  type="button"
                  onclick={resetPayloadToFile}
                  disabled={previewLoading || committing || !filePayloadJson}
                >
                  Reset To File
                </button>
              </div>
            </div>
            <textarea
              class="payload-editor"
              value={payloadDraft}
              spellcheck="false"
              oninput={handlePayloadInput}
            ></textarea>
            {#if payloadDirty}
              <p class="section-copy">The payload has changed since the last preview.</p>
            {/if}
          </section>

          <section class="section-block">
            <h3>Parsed Preview</h3>
            <div class={["status-block", preview.canImport ? "status-ready" : "status-warning"]}>
              <strong>{preview.canImport ? "Ready to import" : "Preview only"}</strong>
              <p>
                {#if preview.canImport}
                  The payload passed the current backend parse and validation path.
                {:else}
                  The file was parsed for review, but the current backend validation path still blocks final import.
                {/if}
              </p>
            </div>

            <div class="review-grid">
              {#each reviewSections as section (section.title)}
                <ImportReviewFieldSection
                  title={section.title}
                  fields={section.fields}
                  emptyMessage={section.emptyMessage}
                  wide={section.wide ?? false}
                />
              {/each}
            </div>
          </section>

          {#if preview.issues.length > 0}
            <section class="section-block">
              <h3>Issues</h3>
              <ul class="issue-list">
                {#each preview.issues as issue, index (`preview-issue:${issue.code}:${index}`)}
                  <li class={["issue-item", `severity-${issue.severity}`]}>
                    <strong>{issue.severity}</strong>
                    <span>{issue.message}</span>
                  </li>
                {/each}
              </ul>
            </section>
          {/if}
        {:else if stage === "review"}
          <section class="section-block">
            <h3>Review Checklist</h3>
            <ImportReviewChecklist items={reviewItems} />
          </section>

          <section class="section-block">
            <h3>Import Draft</h3>
            <div class="review-grid">
              {#each reviewSections as section (section.title)}
                <ImportReviewFieldSection
                  title={section.title}
                  fields={section.fields}
                  emptyMessage={section.emptyMessage}
                  wide={section.wide ?? false}
                />
              {/each}
            </div>
          </section>

          {#if preview.issues.length > 0}
            <section class="section-block">
              <h3>Issues</h3>
              <ul class="issue-list">
                {#each preview.issues as issue, index (`review-issue:${issue.code}:${index}`)}
                  <li class={["issue-item", `severity-${issue.severity}`]}>
                    <strong>{issue.severity}</strong>
                    <span>{issue.message}</span>
                  </li>
                {/each}
              </ul>
            </section>
          {/if}

          <section class="section-block">
            <label class="toggle-row">
              <input type="checkbox" bind:checked={showRawJson} />
              <span>Show raw payload</span>
            </label>
            {#if showRawJson}
              <pre class="json-preview">{payloadDraft}</pre>
            {/if}
          </section>

          {#if commitErrorMessage}
            <section class="section-block">
              <div class={["status-block", "status-error"]}>
                <strong>Import failed</strong>
                <p>{commitErrorMessage}</p>
              </div>
            </section>
          {/if}
        {:else if importResult}
          <section class="section-block">
            <h3>Import Result</h3>
            <div class="review-grid">
              <ImportReviewFieldSection title="Result" fields={resultFields} />
              <ImportReviewFieldSection title="Imported Asset" fields={parsedFields} />
            </div>
          </section>
        {/if}
      {/if}
    </div>

    <footer class="dialog-footer">
      <button class="ghost-button" type="button" onclick={() => !committing && close()} disabled={committing}>
        Cancel
      </button>
      {#if stage === "review"}
        <button class="ghost-button" type="button" onclick={goToConfigure} disabled={committing}>
          Back To Edit
        </button>
        <button
          class="primary-button"
          type="button"
          onclick={confirmImport}
          disabled={confirmDisabledReason !== null}
        >
          {confirmButtonLabel}
        </button>
      {:else if stage === "result"}
        <button class="primary-button" type="button" onclick={close}>
          Done
        </button>
      {:else}
        <button
          class="primary-button"
          type="button"
          onclick={goToReview}
          disabled={previewLoading || !!previewErrorMessage || !preview}
        >
          Review Import Draft
        </button>
      {/if}
    </footer>

    {#if stage === "review" && confirmDisabledReason}
      <div class="footer-note">{confirmDisabledReason}</div>
    {/if}
  </div>
</div>

<style>
  .well-time-depth-import-backdrop {
    position: fixed;
    inset: 0;
    z-index: 80;
    display: flex;
    align-items: center;
    justify-content: center;
    padding: 20px;
    background: rgb(0 0 0 / 0.45);
  }

  .well-time-depth-import-dialog {
    width: min(920px, 100%);
    max-height: min(90vh, 960px);
    display: grid;
    grid-template-rows: auto 1fr auto auto;
    overflow: hidden;
    border: 1px solid rgba(255, 255, 255, 0.08);
    border-radius: 8px;
    background: rgba(14, 20, 29, 0.98);
    box-shadow: 0 28px 72px rgb(0 0 0 / 0.38);
  }

  .dialog-header,
  .dialog-footer {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 12px;
    padding: 16px 18px;
    border-bottom: 1px solid rgba(255, 255, 255, 0.06);
  }

  .dialog-footer {
    border-top: 1px solid rgba(255, 255, 255, 0.06);
    border-bottom: none;
  }

  .dialog-header h2,
  .dialog-header p,
  .section-block h3,
  .section-copy,
  .footer-note {
    margin: 0;
  }

  .dialog-header p,
  .section-copy,
  .summary-label,
  label span,
  .footer-note {
    color: rgba(255, 255, 255, 0.7);
  }

  .dialog-body {
    overflow: auto;
    padding: 18px;
    display: grid;
    gap: 16px;
  }

  .summary-grid,
  .review-grid {
    display: grid;
    gap: 12px;
    grid-template-columns: repeat(2, minmax(0, 1fr));
  }

  .summary-grid > div,
  .section-block {
    border: 1px solid rgba(255, 255, 255, 0.08);
    border-radius: 8px;
    background: rgba(255, 255, 255, 0.02);
  }

  .summary-grid > div {
    display: grid;
    gap: 8px;
    padding: 14px;
  }

  .summary-label {
    font-size: 0.9rem;
  }

  .section-block {
    display: grid;
    gap: 12px;
    padding: 14px;
  }

  .field-grid {
    display: grid;
    gap: 12px;
    grid-template-columns: repeat(2, minmax(0, 1fr));
  }

  .payload-header,
  .payload-actions {
    display: flex;
    align-items: flex-start;
    justify-content: space-between;
    gap: 12px;
  }

  .payload-actions {
    justify-content: flex-end;
    flex-wrap: wrap;
  }

  .wide-field {
    grid-column: 1 / -1;
  }

  label {
    display: grid;
    gap: 8px;
  }

  input,
  select,
  .field-textarea {
    min-height: 40px;
    padding: 0 12px;
    border: 1px solid rgba(255, 255, 255, 0.08);
    border-radius: 6px;
    background: rgba(11, 16, 24, 0.92);
    color: inherit;
    font: inherit;
  }

  .field-textarea {
    min-height: 96px;
    padding: 10px 12px;
    resize: vertical;
  }

  .payload-editor {
    min-height: 280px;
    padding: 12px;
    border: 1px solid rgba(255, 255, 255, 0.08);
    border-radius: 6px;
    background: rgb(8 12 18);
    color: inherit;
    font: inherit;
    resize: vertical;
    white-space: pre;
  }

  .status-block {
    display: grid;
    gap: 6px;
    padding: 12px;
    border: 1px solid rgba(255, 255, 255, 0.08);
    border-radius: 6px;
    background: rgba(255, 255, 255, 0.02);
  }

  .status-block p,
  .issue-item span {
    margin: 0;
    color: rgba(255, 255, 255, 0.72);
  }

  .status-ready {
    border-color: rgba(66, 140, 84, 0.24);
  }

  .status-warning {
    border-color: rgba(201, 145, 62, 0.3);
  }

  .status-error {
    border-color: rgba(177, 71, 61, 0.28);
  }

  .issue-list {
    list-style: none;
    margin: 0;
    padding: 0;
    display: grid;
    gap: 10px;
  }

  .issue-item {
    display: grid;
    gap: 4px;
    padding: 10px 12px;
    border: 1px solid rgba(255, 255, 255, 0.08);
    border-radius: 6px;
    background: rgba(255, 255, 255, 0.02);
  }

  .severity-blocking {
    border-color: rgba(177, 71, 61, 0.28);
  }

  .severity-warning {
    border-color: rgba(201, 145, 62, 0.3);
  }

  .severity-info {
    border-color: rgba(66, 140, 84, 0.24);
  }

  .toggle-row {
    display: flex;
    align-items: center;
    gap: 10px;
  }

  .toggle-row input {
    min-height: auto;
    width: 16px;
    height: 16px;
    padding: 0;
  }

  .json-preview {
    margin: 0;
    max-height: 280px;
    overflow: auto;
    padding: 12px;
    border-radius: 6px;
    background: rgb(8 12 18);
    font-size: 0.85rem;
  }

  .ghost-button,
  .primary-button {
    min-height: 38px;
    padding: 0 14px;
    border-radius: 6px;
    border: 1px solid rgba(255, 255, 255, 0.08);
    background: rgba(255, 255, 255, 0.02);
    color: inherit;
    font: inherit;
  }

  .primary-button {
    background: color-mix(in srgb, var(--accent-solid, #5e92e0) 24%, white);
    border-color: color-mix(in srgb, var(--accent-solid, #5e92e0) 34%, white);
  }

  .ghost-button:disabled,
  .primary-button:disabled {
    opacity: 0.6;
  }

  .footer-note {
    padding: 0 18px 16px;
  }

  @media (max-width: 820px) {
    .well-time-depth-import-dialog {
      width: 100%;
      max-height: 100vh;
      border-radius: 0;
    }

    .summary-grid,
    .review-grid,
    .field-grid {
      grid-template-columns: 1fr;
    }

    .payload-header {
      flex-direction: column;
    }
  }
</style>
