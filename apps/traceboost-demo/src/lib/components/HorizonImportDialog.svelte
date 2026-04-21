<svelte:options runes={true} />

<script lang="ts">
  import { onMount } from "svelte";
  import type { ImportedHorizonDescriptor } from "@traceboost/seis-contracts";
  import type { CoordinateReferenceSelection } from "../bridge";
  import ImportFlowStepper from "./ImportFlowStepper.svelte";
  import ImportReviewChecklist from "./ImportReviewChecklist.svelte";
  import ImportReviewFieldSection from "./ImportReviewFieldSection.svelte";
  import CoordinateReferencePicker from "./CoordinateReferencePicker.svelte";
  import {
    emitFrontendDiagnosticsEvent,
    inspectHorizonXyzFiles,
    previewHorizonSourceImport,
    type HorizonSourceImportCanonicalDraft,
    type HorizonSourceImportPreview,
    type HorizonXyzFilePreview
  } from "../bridge";
  import { pickRuntimeStoreFile } from "../file-dialog";
  import { getViewerModelContext } from "../viewer-model.svelte";
  import {
    compactImportReviewFields,
    type ImportConfirmationStage,
    type ImportFlowStep,
    type ImportReviewField as ReviewField,
    type ImportReviewItem as ReviewItem,
    type ImportReviewSection
  } from "./import-review";

  interface Props {
    inputPaths: string[];
    close: () => void;
  }

  interface ReviewFileSummary {
    sourcePath: string;
    name: string;
    parsedPointCount: number;
    invalidRowCount: number;
    mappedPointCount: number | null;
    missingCellCount: number | null;
    xRangeLabel: string;
    yRangeLabel: string;
    zRangeLabel: string;
    canCommit: boolean | null;
    issues: string[];
  }

  let { inputPaths, close }: Props = $props();

  const viewerModel = getViewerModelContext();
  let sourceMode = $state<"survey" | "custom" | "unresolved">("unresolved");
  let sourceCoordinateReferenceIdDraft = $state("");
  let sourceCoordinateReferenceNameDraft = $state("");
  let verticalDomainDraft = $state<ImportedHorizonDescriptor["vertical_domain"]>("time");
  let verticalUnitDraft = $state("ms");
  let previewResponse = $state.raw<HorizonSourceImportPreview | null>(null);
  let importResult = $state.raw<ImportedHorizonDescriptor[] | null>(null);
  let parseOnlyFiles = $state<HorizonXyzFilePreview[]>([]);
  let previewLoading = $state(false);
  let previewError = $state<string | null>(null);
  let stage = $state<ImportConfirmationStage>("configure");
  let showRawReviewPayload = $state(false);
  let sourceCrsPickerOpen = $state(false);
  let sourceModeSeeded = false;
  let previewRequestVersion = 0;
  const activeStorePath = $derived(viewerModel.activeStorePath.trim());
  const preview = $derived(previewResponse?.parsed ?? null);
  const canonicalDraft = $derived.by<HorizonSourceImportCanonicalDraft>(() => ({
    selectedSourcePaths: inputPaths.map((value) => value.trim()).filter((value) => value.length > 0),
    verticalDomain: verticalDomainDraft,
    verticalUnit: normalizedVerticalUnit(),
    sourceCoordinateReference:
      sourceMode === "custom"
        ? {
            id: sourceCoordinateReferenceIdDraft.trim() || null,
            name: sourceCoordinateReferenceNameDraft.trim() || null,
            geodetic_datum: null,
            unit: null
          }
        : null,
    assumeSameAsSurvey: sourceMode === "survey"
  }));

  const surveyCoordinateReferenceLabel = $derived.by(() => {
    const id = viewerModel.activeEffectiveNativeCoordinateReferenceId?.trim() ?? "";
    const name = viewerModel.activeEffectiveNativeCoordinateReferenceName?.trim() ?? "";
    if (id && name) {
      return `${id} (${name})`;
    }
    if (id) {
      return id;
    }
    if (name) {
      return name;
    }
    return "Survey local coordinates without a resolved CRS id";
  });
  const sourceModeLabel = $derived.by(() => {
    if (sourceMode === "survey") {
      return "Use active survey native CRS";
    }
    if (sourceMode === "custom") {
      return "Specify source CRS";
    }
    return "Leave unresolved";
  });
  const selectedSourceCoordinateReferenceLabel = $derived.by(() => {
    if (preview?.source_coordinate_reference) {
      return formatCoordinateReference(preview.source_coordinate_reference);
    }
    if (sourceMode === "survey") {
      return surveyCoordinateReferenceLabel;
    }
    if (sourceMode === "custom") {
      return formatCoordinateReference(canonicalDraft.sourceCoordinateReference) || "Unresolved";
    }
    return "Unresolved";
  });
  const alignedCoordinateReferenceLabel = $derived.by(() =>
    preview?.aligned_coordinate_reference
      ? formatCoordinateReference(preview.aligned_coordinate_reference)
      : activeStorePath
        ? surveyCoordinateReferenceLabel
        : "Unavailable until a survey store is active"
  );
  const reviewFiles = $derived.by<ReviewFileSummary[]>(() =>
    preview
      ? preview.files.map((file) => ({
          sourcePath: file.source_path,
          name: file.name,
          parsedPointCount: file.parsed_point_count,
          invalidRowCount: file.invalid_row_count,
          mappedPointCount: file.estimated_mapped_point_count,
          missingCellCount: file.estimated_missing_cell_count,
          xRangeLabel: previewRangeLabel(file.x_min, file.x_max, 2),
          yRangeLabel: previewRangeLabel(file.y_min, file.y_max, 2),
          zRangeLabel: previewRangeLabel(file.z_min, file.z_max, 2),
          canCommit: file.can_commit,
          issues: file.issues
        }))
      : parseOnlyFiles.map((file) => ({
          sourcePath: file.source_path,
          name: file.name,
          parsedPointCount: file.parsed_point_count,
          invalidRowCount: file.invalid_row_count,
          mappedPointCount: null,
          missingCellCount: null,
          xRangeLabel: previewRangeLabel(file.x_min, file.x_max, 2),
          yRangeLabel: previewRangeLabel(file.y_min, file.y_max, 2),
          zRangeLabel: previewRangeLabel(file.z_min, file.z_max, 2),
          canCommit: null,
          issues: file.issues
        }))
  );
  const reviewSummaryFields = $derived.by(() =>
    compactImportReviewFields([
      ["Selected files", String(canonicalDraft.selectedSourcePaths.length)],
      ["Target survey store", activeStorePath],
      ["Vertical Domain", labelForVerticalDomain(canonicalDraft.verticalDomain)],
      ["Vertical Unit", displayVerticalUnit()],
      ["Source CRS mode", sourceModeLabel],
      ["Source CRS", selectedSourceCoordinateReferenceLabel],
      ["Aligned CRS", alignedCoordinateReferenceLabel],
      ["Transform required", preview ? (preview.transformed ? "Yes" : "No") : "Unknown"],
      ["Import status", preview ? (preview.can_commit ? "Ready to import" : "Preview only") : "Parse only"]
    ])
  );
  const reviewOutcomeFields = $derived.by(() =>
    compactImportReviewFields([
      [
        "Files ready to commit",
        preview ? String(preview.files.filter((file) => file.can_commit).length) : null
      ],
      [
        "Files blocked",
        preview ? String(preview.files.filter((file) => !file.can_commit).length) : null
      ],
      [
        "Projected mapped cells",
        preview
          ? numberLabel(
              preview.files.reduce((sum, file) => sum + (file.estimated_mapped_point_count ?? 0), 0),
              0
            )
          : null
      ],
      [
        "Projected missing cells",
        preview
          ? numberLabel(
              preview.files.reduce((sum, file) => sum + (file.estimated_missing_cell_count ?? 0), 0),
              0
            )
          : null
      ]
    ])
  );
  const reviewItems = $derived.by(() => {
    const items: ReviewItem[] = [];
    if (previewLoading) {
      items.push({
        severity: "info",
        title: "Preview is still running",
        message: "Wait for the parse and CRS validation to finish before confirming the import."
      });
      return items;
    }
    if (previewError) {
      items.push({
        severity: "blocking",
        title: "Preview failed",
        message: previewError
      });
      return items;
    }
    if (inputPaths.length === 0) {
      items.push({
        severity: "blocking",
        title: "No files selected",
        message: "Choose at least one horizon XYZ file before continuing."
      });
    }
    if (!activeStorePath) {
      items.push({
        severity: "blocking",
        title: "Target survey store required",
        message: "The files can be parsed, but import stays disabled until a survey store is active."
      });
    }
    if (sourceMode === "custom" && sourceCoordinateReferenceIdDraft.trim().length === 0) {
      items.push({
        severity: "blocking",
        title: "Source CRS selection required",
        message: "Choose a source CRS before importing with a custom CRS."
      });
    }
    if (preview && !preview.can_commit) {
      items.push({
        severity: "warning",
        title: "CRS path is not committable yet",
        message: "The files parsed successfully, but the chosen CRS path still blocks final import."
      });
    }
    if (preview?.transformed) {
      items.push({
        severity: "info",
        title: "Reprojection will be applied",
        message: "The selected source CRS will be transformed into the active survey coordinate frame during import."
      });
    }
    if (preview?.issues.length) {
      items.push({
        severity: "warning",
        title: "Preview reported import issues",
        message: `${preview.issues.length} issue${preview.issues.length === 1 ? "" : "s"} should be reviewed before confirmation.`
      });
    }
    if (reviewFiles.some((file) => file.invalidRowCount > 0)) {
      const invalidRows = reviewFiles.reduce((sum, file) => sum + file.invalidRowCount, 0);
      items.push({
        severity: "info",
        title: "Some rows were skipped during parse",
        message: `${invalidRows} invalid row${invalidRows === 1 ? "" : "s"} will stay out of the imported horizons.`
      });
    }
    if (items.length === 0) {
      items.push({
        severity: "info",
        title: "Ready to confirm",
        message: "The parsed horizon draft is aligned with the current survey import path."
      });
    }
    return items;
  });
  const reviewSections = $derived.by((): ImportReviewSection[] => [
    {
      title: "Summary",
      fields: reviewSummaryFields
    },
    {
      title: "Projected Outcome",
      fields: reviewOutcomeFields
    }
  ]);
  const flowSteps = $derived.by((): ImportFlowStep[] => [
    {
      key: "configure",
      label: "1. Configure",
      description: "Choose the horizon source CRS path and inspect the parsed file draft.",
      disabled: stage === "result"
    },
    {
      key: "review",
      label: "2. Review",
      description: "Confirm the bounded import draft before any horizons are written into the survey.",
      disabled: stage === "result"
    },
    {
      key: "result",
      label: "3. Result",
      description: "Review the imported horizon descriptors and finish the flow.",
      disabled: stage !== "result"
    }
  ]);
  const reviewPayload = $derived.by(() => ({
    targetStorePath: activeStorePath || null,
    sourceCrsMode: sourceMode,
    draft: canonicalDraft,
    suggestedDraft: previewResponse?.suggestedDraft ?? null,
    parsed: preview,
    files: reviewFiles.map((file) => ({
      name: file.name,
      sourcePath: file.sourcePath,
      parsedPointCount: file.parsedPointCount,
      invalidRowCount: file.invalidRowCount,
      mappedPointCount: file.mappedPointCount,
      missingCellCount: file.missingCellCount,
      canCommit: file.canCommit
    }))
  }));
  const reviewPayloadText = $derived(JSON.stringify(reviewPayload, null, 2));
  const confirmDisabledReason = $derived.by(() => {
    if (previewLoading) {
      return "Preparing import preview.";
    }
    if (previewError) {
      return "Preview failed.";
    }
    if (inputPaths.length === 0) {
      return "No horizon files selected.";
    }
    if (!activeStorePath) {
      return "Choose an active survey store to complete the import.";
    }
    if (sourceMode === "custom" && sourceCoordinateReferenceIdDraft.trim().length === 0) {
      return "Choose a source CRS.";
    }
    if (!preview?.can_commit) {
      return "Resolve the CRS path before importing.";
    }
    return null;
  });

  function numberLabel(value: number | null | undefined, digits = 2): string {
    if (typeof value !== "number" || !Number.isFinite(value)) {
      return "Unknown";
    }
    return value.toLocaleString(undefined, {
      minimumFractionDigits: 0,
      maximumFractionDigits: digits
    });
  }

  function previewRangeLabel(minimum: number | null, maximum: number | null, digits = 2): string {
    if (minimum === null || maximum === null) {
      return "Unknown";
    }
    return `${numberLabel(minimum, digits)} to ${numberLabel(maximum, digits)}`;
  }

  function formatCoordinateReference(
    coordinateReference:
      | {
          id?: string | null;
          name?: string | null;
        }
      | null
      | undefined
  ): string {
    const id = coordinateReference?.id?.trim();
    const name = coordinateReference?.name?.trim();
    if (id && name) {
      return `${id} (${name})`;
    }
    if (id) {
      return id;
    }
    if (name) {
      return name;
    }
    return "";
  }

  function defaultVerticalUnit(
    verticalDomain: ImportedHorizonDescriptor["vertical_domain"]
  ): string {
    return verticalDomain === "depth" ? "m" : "ms";
  }

  function normalizedVerticalUnit(): string | null {
    const trimmed = verticalUnitDraft.trim();
    return trimmed.length > 0 ? trimmed : null;
  }

  function displayVerticalUnit(): string {
    return normalizedVerticalUnit() ?? defaultVerticalUnit(verticalDomainDraft);
  }

  function labelForVerticalDomain(
    verticalDomain: ImportedHorizonDescriptor["vertical_domain"]
  ): string {
    return verticalDomain === "depth" ? "Depth" : "Time";
  }

  function chooseVerticalDomain(
    nextDomain: ImportedHorizonDescriptor["vertical_domain"]
  ): void {
    const currentUnit = verticalUnitDraft.trim();
    const currentDefault = defaultVerticalUnit(verticalDomainDraft);
    verticalDomainDraft = nextDomain;
    if (!currentUnit || currentUnit === currentDefault) {
      verticalUnitDraft = defaultVerticalUnit(nextDomain);
    }
    void refreshPreview();
  }

  function seedSourceModeFromSurveyNative(): void {
    if (sourceModeSeeded) {
      return;
    }
    if (!activeStorePath) {
      return;
    }
    const hasSurveyNativeCoordinateReference =
      (viewerModel.activeEffectiveNativeCoordinateReferenceId?.trim().length ?? 0) > 0 ||
      (viewerModel.activeEffectiveNativeCoordinateReferenceName?.trim().length ?? 0) > 0;
    if (hasSurveyNativeCoordinateReference) {
      sourceMode = "survey";
      sourceModeSeeded = true;
    }
  }

  async function refreshPreview(): Promise<void> {
    if (inputPaths.length === 0) {
      previewResponse = null;
      importResult = null;
      parseOnlyFiles = [];
      previewError = null;
      previewLoading = false;
      stage = "configure";
      return;
    }

    const currentVersion = ++previewRequestVersion;
    previewLoading = true;
    previewError = null;
    importResult = null;
    showRawReviewPayload = false;
    stage = "configure";
    try {
      if (!activeStorePath) {
        const inspectedFiles = await inspectHorizonXyzFiles(inputPaths);
        if (currentVersion !== previewRequestVersion) {
          return;
        }
        previewResponse = null;
        parseOnlyFiles = inspectedFiles;
        recordHorizonImportDiagnostics(
          "preview_horizon_sources",
          "info",
          "Horizon source files parsed without an active survey store.",
          {
            inputPathCount: inputPaths.length,
            parsedFileCount: inspectedFiles.length,
            sourceCrsMode: sourceMode
          }
        );
      } else {
        const nextPreview = await previewHorizonSourceImport({
          storePath: activeStorePath,
          inputPaths,
          draft: canonicalDraft
        });
        if (currentVersion !== previewRequestVersion) {
          return;
        }
        previewResponse = nextPreview;
        parseOnlyFiles = [];
        recordHorizonImportDiagnostics(
          "preview_horizon_sources",
          "info",
          "Horizon source preview completed.",
          {
            storePath: activeStorePath,
            inputPathCount: inputPaths.length,
            canCommit: nextPreview.parsed.can_commit,
            transformed: nextPreview.parsed.transformed,
            issueCount: nextPreview.parsed.issues.length,
            noteCount: nextPreview.parsed.notes.length,
            sourceCrsMode: sourceMode,
            sourceCrsId: canonicalDraft.sourceCoordinateReference?.id ?? null
          }
        );
      }
    } catch (error) {
      if (currentVersion !== previewRequestVersion) {
        return;
      }
      previewResponse = null;
      parseOnlyFiles = [];
      previewError = error instanceof Error ? error.message : "Unknown horizon import preview error";
      recordHorizonImportDiagnostics("preview_horizon_sources", "error", "Horizon source preview failed.", {
        storePath: activeStorePath || null,
        inputPathCount: inputPaths.length,
        sourceCrsMode: sourceMode,
        error: previewError
      });
    } finally {
      if (currentVersion === previewRequestVersion) {
        previewLoading = false;
      }
    }
  }

  async function confirmImport(): Promise<void> {
    if (inputPaths.length === 0) {
      close();
      return;
    }

    if (confirmDisabledReason) {
      viewerModel.note(confirmDisabledReason, "ui", "warn");
      recordHorizonImportDiagnostics("commit_horizon_sources", "warn", "Horizon import blocked.", {
        reason: confirmDisabledReason,
        storePath: activeStorePath || null,
        inputPathCount: inputPaths.length
      });
      return;
    }

    recordHorizonImportDiagnostics("commit_horizon_sources", "info", "Started horizon source import.", {
      storePath: activeStorePath,
      inputPathCount: inputPaths.length,
      sourceCrsMode: sourceMode,
      sourceCrsId: canonicalDraft.sourceCoordinateReference?.id ?? null
    });
    const imported = await viewerModel.importHorizonDraft(canonicalDraft);
    if (!viewerModel.error && imported) {
      importResult = imported;
      stage = "result";
      recordHorizonImportDiagnostics("commit_horizon_sources", "info", "Horizon source import completed.", {
        storePath: activeStorePath,
        inputPathCount: inputPaths.length,
        importedCount: imported.length
      });
    }
  }

  function goToReview(): void {
    recordHorizonImportDiagnostics("review_horizon_sources", "info", "Opened horizon import review draft.", {
      storePath: activeStorePath || null,
      inputPathCount: inputPaths.length,
      sourceCrsMode: sourceMode,
      canCommit: preview?.can_commit ?? false
    });
    stage = "review";
  }

  function goToConfigure(): void {
    stage = "configure";
  }

  function chooseSourceMode(nextMode: "survey" | "custom" | "unresolved"): void {
    sourceMode = nextMode;
    void refreshPreview();
  }

  function recordHorizonImportDiagnostics(
    stage: string,
    level: "info" | "warn" | "error",
    message: string,
    fields: Record<string, unknown>
  ): void {
    void emitFrontendDiagnosticsEvent({
      stage,
      level,
      message,
      fields
    }).catch((error) => {
      viewerModel.note(
        "Failed to record horizon import diagnostics.",
        "backend",
        "warn",
        error instanceof Error ? error.message : String(error)
      );
    });
  }

  function openSourceCrsPicker(): void {
    sourceMode = "custom";
    sourceCrsPickerOpen = true;
  }

  function closeSourceCrsPicker(): void {
    sourceCrsPickerOpen = false;
  }

  function handleSourceCoordinateReferenceSelection(selection: CoordinateReferenceSelection): void {
    if (selection.kind !== "authority_code") {
      return;
    }
    sourceMode = "custom";
    sourceCoordinateReferenceIdDraft = selection.authId;
    sourceCoordinateReferenceNameDraft = selection.name?.trim() ?? "";
    sourceCrsPickerOpen = false;
    void refreshPreview();
  }

  function clearCustomSourceCoordinateReference(): void {
    sourceCoordinateReferenceIdDraft = "";
    sourceCoordinateReferenceNameDraft = "";
    void refreshPreview();
  }

  async function chooseActiveStore(): Promise<void> {
    const nextStorePath = await pickRuntimeStoreFile();
    if (!nextStorePath) {
      return;
    }
    await viewerModel.openVolumePath(nextStorePath);
    seedSourceModeFromSurveyNative();
    await refreshPreview();
  }

  function handleBackdropClick(event: MouseEvent): void {
    if (event.target === event.currentTarget && !viewerModel.horizonImporting) {
      close();
    }
  }

  onMount(() => {
    seedSourceModeFromSurveyNative();
    void refreshPreview();
  });
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
      <p>Parse the selected horizon XYZ files, confirm the CRS path, then import.</p>
    </header>

    <div class="summary-grid">
      <div class="summary">
        <span>Files</span>
        <strong>{inputPaths.length}</strong>
      </div>
      <div class="summary">
        <span>Survey Coordinates</span>
        <strong>{activeStorePath ? surveyCoordinateReferenceLabel : "No active survey store"}</strong>
      </div>
    </div>

    <div class="section">
      <h4>Import Flow</h4>
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
    </div>

    {#if !activeStorePath}
      <div class="advisory">
        <strong>Parse-only preview</strong>
        <p>The selected files are being inspected without a survey store. Import stays disabled until a target survey is active.</p>
        <button
          type="button"
          class="secondary"
          onclick={() => {
            void chooseActiveStore();
          }}
          disabled={viewerModel.horizonImporting || previewLoading}
        >
          Choose Survey Store
        </button>
      </div>
    {/if}

    {#if viewerModel.horizonImportProjectAdvisory}
      <div class="advisory">
        <strong>Project CRS advisory</strong>
        <p>{viewerModel.horizonImportProjectAdvisory}</p>
      </div>
    {/if}

    {#if stage === "configure"}
      <div class="section">
        <h4>Vertical Axis</h4>

        <label class="choice">
          <input
            type="radio"
            name="horizon-vertical-domain"
            checked={verticalDomainDraft === "time"}
            disabled={viewerModel.horizonImporting || previewLoading}
            onchange={() => chooseVerticalDomain("time")}
          />
          <div>
            <strong>Time</strong>
            <p>Use this when the XYZ Z values represent horizon time picks.</p>
          </div>
        </label>

        <label class="choice">
          <input
            type="radio"
            name="horizon-vertical-domain"
            checked={verticalDomainDraft === "depth"}
            disabled={viewerModel.horizonImporting || previewLoading}
            onchange={() => chooseVerticalDomain("depth")}
          />
          <div>
            <strong>Depth</strong>
            <p>Use this when the XYZ Z values represent horizon depth values.</p>
          </div>
        </label>

        <div class="field-grid">
          <label class="field">
            <span>Vertical Unit</span>
            <input
              bind:value={verticalUnitDraft}
              type="text"
              placeholder={defaultVerticalUnit(verticalDomainDraft)}
              disabled={viewerModel.horizonImporting || previewLoading}
              oninput={() => {
                void refreshPreview();
              }}
            />
          </label>
        </div>
      </div>

      <div class="section">
        <h4>Source CRS</h4>

        <div class="detail-note">
          <strong>Default import assumption</strong>
          <p>
            When the active survey already has a native CRS, the import defaults to using that CRS.
            You can keep it, switch to a different source CRS, or leave geometry unresolved.
          </p>
        </div>

        {#if sourceMode === "unresolved" && activeStorePath}
          <div class="status">
            <strong>Explicit confirmation required for import</strong>
            <p>
              Horizon files parse immediately, but final import stays disabled until you explicitly
              choose survey coordinates or enter a source CRS for reprojection.
            </p>
          </div>
        {/if}

        <label class="choice">
          <input
            type="radio"
            name="horizon-crs-mode"
            checked={sourceMode === "survey"}
            disabled={viewerModel.horizonImporting || previewLoading}
            onchange={() => chooseSourceMode("survey")}
          />
          <div>
            <strong>Use survey coordinates</strong>
            <p>
              Treat the XYZ coordinates as already aligned with the active survey native CRS. This
              is the default when the active survey already has a confirmed native CRS.
            </p>
          </div>
        </label>

        <label class="choice">
          <input
            type="radio"
            name="horizon-crs-mode"
            checked={sourceMode === "custom"}
            disabled={viewerModel.horizonImporting || previewLoading}
            onchange={() => chooseSourceMode("custom")}
          />
          <div>
            <strong>Specify source CRS</strong>
            <p>Use this when the XYZ coordinates need reprojection into the survey coordinate frame.</p>
          </div>
        </label>

        <label class="choice">
          <input
            type="radio"
            name="horizon-crs-mode"
            checked={sourceMode === "unresolved"}
            disabled={viewerModel.horizonImporting || previewLoading}
            onchange={() => chooseSourceMode("unresolved")}
          />
          <div>
            <strong>Leave unresolved</strong>
            <p>Preview the parsed files without enabling final import.</p>
          </div>
        </label>

        {#if sourceMode === "custom"}
          <div class="field">
            <span>Source CRS</span>
            <div class="selection-card">
              <div>
                <strong>{selectedSourceCoordinateReferenceLabel}</strong>
                <p>Choose a validated CRS from the local registry for reprojection into the survey frame.</p>
              </div>
              <div class="selection-actions">
                <button
                  type="button"
                  class="secondary"
                  disabled={viewerModel.horizonImporting || previewLoading}
                  onclick={openSourceCrsPicker}
                >
                  Choose Source CRS
                </button>
                {#if sourceCoordinateReferenceIdDraft}
                  <button
                    type="button"
                    class="secondary"
                    disabled={viewerModel.horizonImporting || previewLoading}
                    onclick={clearCustomSourceCoordinateReference}
                  >
                    Clear
                  </button>
                {/if}
              </div>
            </div>
          </div>
        {/if}
      </div>

      <div class="section">
        <h4>Preview</h4>

        {#if previewLoading}
          <div class="status">
            <p>Parsing horizon files and validating the selected CRS path.</p>
          </div>
        {:else if previewError}
          <div class="status error">
            <strong>Preview failed</strong>
            <p>{previewError}</p>
          </div>
        {:else if preview || reviewFiles.length > 0}
          <div
            class={[
              "status",
              preview?.can_commit && "ready",
              preview && !preview.can_commit && "error"
            ]}
          >
            <strong>
              {#if preview}
                {preview.can_commit ? "Ready to import" : "Preview only"}
              {:else}
                Parse only
              {/if}
            </strong>
            <p>
              {#if preview}
                {#if preview.can_commit}
                  The selected CRS path is viable for import.
                {:else}
                  The files parsed, but import is blocked until the CRS path is resolved.
                {/if}
              {:else}
                The selected files were parsed without an active survey store.
              {/if}
            </p>
            {#if preview?.notes.length}
              <ul class="issue-list">
                {#each preview.notes as note (`preview-note:${note}`)}
                  <li>{note}</li>
                {/each}
              </ul>
            {/if}
            {#if preview?.issues.length}
              <ul class="issue-list">
                {#each preview.issues as issue (`preview-issue:${issue}`)}
                  <li>{issue}</li>
                {/each}
              </ul>
            {/if}
          </div>

          <div class="file-list">
            {#each reviewFiles as file (file.sourcePath)}
              <div class={["file-card", file.canCommit === false && "blocked"]}>
                <div class="file-header">
                  <div>
                    <strong>{file.name}</strong>
                    <p>{file.sourcePath}</p>
                  </div>
                  <span
                    class={[
                      "badge",
                      file.canCommit === true ? "ok" : "warn"
                    ]}
                  >
                    {file.canCommit === null ? "Parsed only" : file.canCommit ? "Committable" : "Preview only"}
                  </span>
                </div>

                <div class="metric-grid">
                  <div>
                    <span>Parsed Points</span>
                    <strong>{numberLabel(file.parsedPointCount, 0)}</strong>
                  </div>
                  <div>
                    <span>Invalid Rows</span>
                    <strong>{numberLabel(file.invalidRowCount, 0)}</strong>
                  </div>
                  <div>
                    <span>Mapped Grid Cells</span>
                    <strong>{numberLabel(file.mappedPointCount, 0)}</strong>
                  </div>
                  <div>
                    <span>Missing Grid Cells</span>
                    <strong>{numberLabel(file.missingCellCount, 0)}</strong>
                  </div>
                </div>

                <div class="metric-grid">
                  <div>
                    <span>X Range</span>
                    <strong>{file.xRangeLabel}</strong>
                  </div>
                  <div>
                    <span>Y Range</span>
                    <strong>{file.yRangeLabel}</strong>
                  </div>
                  <div>
                    <span>Z Range</span>
                    <strong>{file.zRangeLabel}</strong>
                  </div>
                </div>

                {#if file.issues.length}
                  <ul class="issue-list">
                    {#each file.issues as issue (`file-issue:${file.sourcePath}:${issue}`)}
                      <li>{issue}</li>
                    {/each}
                  </ul>
                {/if}
              </div>
            {/each}
          </div>
        {:else}
          <div class="status error">
            <strong>No preview available</strong>
            <p>The selected files could not be prepared for preview.</p>
          </div>
        {/if}
      </div>
    {:else if stage === "review"}
      <div class="section">
        <h4>Review Checklist</h4>
        <ImportReviewChecklist items={reviewItems} />
      </div>

      <div class="section">
        <h4>Import Draft</h4>
        <div class="review-grid">
          {#each reviewSections as section (`section:${section.title}`)}
            <ImportReviewFieldSection
              title={section.title}
              fields={section.fields}
              emptyMessage={section.emptyMessage}
              wide={section.wide ?? false}
            />
          {/each}
        </div>
      </div>

      <div class="section">
        <h4>Files</h4>
        <div class="file-list">
          {#each reviewFiles as file (file.sourcePath)}
            <div class={["file-card", file.canCommit === false && "blocked"]}>
              <div class="file-header">
                <div>
                  <strong>{file.name}</strong>
                  <p>{file.sourcePath}</p>
                </div>
                <span
                  class={[
                    "badge",
                    file.canCommit === true ? "ok" : "warn"
                  ]}
                >
                  {file.canCommit === null ? "Parsed only" : file.canCommit ? "Committable" : "Preview only"}
                </span>
              </div>
              <div class="metric-grid">
                <div>
                  <span>Parsed Points</span>
                  <strong>{numberLabel(file.parsedPointCount, 0)}</strong>
                </div>
                <div>
                  <span>Invalid Rows</span>
                  <strong>{numberLabel(file.invalidRowCount, 0)}</strong>
                </div>
                <div>
                  <span>Mapped Grid Cells</span>
                  <strong>{numberLabel(file.mappedPointCount, 0)}</strong>
                </div>
                <div>
                  <span>Missing Grid Cells</span>
                  <strong>{numberLabel(file.missingCellCount, 0)}</strong>
                </div>
              </div>
              {#if file.issues.length}
                <ul class="issue-list">
                  {#each file.issues as issue (`review-file-issue:${file.sourcePath}:${issue}`)}
                    <li>{issue}</li>
                  {/each}
                </ul>
              {/if}
            </div>
          {/each}
        </div>
      </div>

      {#if preview?.notes.length || preview?.issues.length}
        <div class="section">
          <h4>Preview Notes</h4>
          <ul class="issue-list">
            {#each preview?.notes ?? [] as note (`review-note:${note}`)}
              <li>{note}</li>
            {/each}
            {#each preview?.issues ?? [] as issue (`review-issue:${issue}`)}
              <li>{issue}</li>
            {/each}
          </ul>
        </div>
      {/if}

      <div class="section">
        <h4>Raw Payload</h4>
        <label class="choice raw-toggle">
          <input type="checkbox" bind:checked={showRawReviewPayload} />
          <div>
            <strong>Show raw review payload</strong>
            <p>Use this only when you want to inspect the low-level import request shape.</p>
          </div>
        </label>
        {#if showRawReviewPayload}
          <pre class="json-preview">{reviewPayloadText}</pre>
        {/if}
      </div>
    {:else if importResult}
      <div class="section">
        <h4>Import Result</h4>
        <div class="status ready">
          <strong>Horizons imported</strong>
          <p>{importResult.length} horizon{importResult.length === 1 ? "" : "s"} were written into the active survey store.</p>
        </div>
        <div class="file-list">
          {#each importResult as horizon (horizon.id)}
            <div class="file-card">
              <div class="file-header">
                <div>
                  <strong>{horizon.name}</strong>
                  <p>{horizon.source_path}</p>
                </div>
                <span class="badge ok">{horizon.vertical_domain} {horizon.vertical_unit}</span>
              </div>
              <div class="metric-grid">
                <div>
                  <span>Points</span>
                  <strong>{numberLabel(horizon.point_count, 0)}</strong>
                </div>
                <div>
                  <span>Mapped Cells</span>
                  <strong>{numberLabel(horizon.mapped_point_count, 0)}</strong>
                </div>
                <div>
                  <span>Missing Cells</span>
                  <strong>{numberLabel(horizon.missing_cell_count, 0)}</strong>
                </div>
                <div>
                  <span>Transformed</span>
                  <strong>{horizon.transformed ? "Yes" : "No"}</strong>
                </div>
              </div>
              <div class="review-grid">
                <div class="review-subsection">
                  <strong>Source CRS</strong>
                  <p class="muted-copy">
                    {formatCoordinateReference(horizon.source_coordinate_reference) || "Unresolved"}
                  </p>
                </div>
                <div class="review-subsection">
                  <strong>Aligned CRS</strong>
                  <p class="muted-copy">
                    {formatCoordinateReference(horizon.aligned_coordinate_reference) || "Survey native frame"}
                  </p>
                </div>
              </div>
              {#if horizon.notes.length}
                <ul class="issue-list">
                  {#each horizon.notes as note (`result-note:${horizon.id}:${note}`)}
                    <li>{note}</li>
                  {/each}
                </ul>
              {/if}
            </div>
          {/each}
        </div>
      </div>
    {/if}

    <div class="actions">
      <button
        type="button"
        class="secondary"
        onclick={stage === "result" ? close : close}
        disabled={viewerModel.horizonImporting}
      >
        {stage === "result" ? "Done" : "Cancel"}
      </button>
      {#if stage === "configure"}
        <button
          type="button"
          onclick={goToReview}
          disabled={viewerModel.horizonImporting || previewLoading || previewError !== null || reviewFiles.length === 0}
        >
          Review Import Draft
        </button>
      {:else if stage === "review"}
        <button
          type="button"
          class="secondary"
          onclick={goToConfigure}
          disabled={viewerModel.horizonImporting}
        >
          Back To Edit
        </button>
        <button
          type="button"
          onclick={() => void confirmImport()}
          disabled={viewerModel.horizonImporting || confirmDisabledReason !== null}
        >
          {viewerModel.horizonImporting ? "Importing..." : "Confirm Horizon Import"}
        </button>
      {/if}
    </div>
    {#if stage === "review" && confirmDisabledReason}
      <div class="footer-note">{confirmDisabledReason}</div>
    {/if}
  </div>
</div>

{#if sourceCrsPickerOpen}
  <CoordinateReferencePicker
    close={closeSourceCrsPicker}
    confirm={handleSourceCoordinateReferenceSelection}
    title="Horizon Source CRS"
    description="Choose the CRS used by the selected horizon XYZ coordinates."
    selectedAuthId={sourceCoordinateReferenceIdDraft}
    projectRoot={viewerModel.projectRoot}
    projectedOnly={false}
    includeGeographic={true}
    includeVertical={false}
  />
{/if}

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
    width: min(880px, calc(100vw - 32px));
    max-height: min(90vh, 980px);
    overflow: auto;
    display: grid;
    gap: 16px;
    padding: 18px;
    border: 1px solid var(--app-border);
    border-radius: 8px;
    background: var(--panel-bg);
    color: var(--text-primary);
    box-shadow: 0 20px 60px rgba(42, 64, 84, 0.18);
  }

  header h3,
  .section h4,
  .choice strong,
  .file-header strong {
    margin: 0;
    font-size: 16px;
    font-weight: 650;
  }

  header p,
  .choice p,
  .file-header p,
  .status p,
  .advisory p {
    margin: 4px 0 0;
    color: var(--text-muted);
  }

  .summary-grid,
  .field-grid,
  .metric-grid,
  .review-grid {
    display: grid;
    gap: 12px;
  }

  .summary-grid,
  .field-grid {
    grid-template-columns: repeat(2, minmax(0, 1fr));
  }

  .metric-grid {
    grid-template-columns: repeat(4, minmax(0, 1fr));
  }

  .review-grid {
    grid-template-columns: repeat(auto-fit, minmax(220px, 1fr));
  }

  .summary,
  .advisory,
  .section,
  .choice,
  .status,
  .file-card,
  .detail-note,
  .selection-card {
    border: 1px solid var(--app-border);
    border-radius: 6px;
    background: var(--surface-bg);
  }

  .summary,
  .advisory,
  .section,
  .status,
  .file-card,
  .detail-note,
  .selection-card {
    padding: 12px;
  }

  .summary span,
  .field span,
  .metric-grid span {
    font-size: 11px;
    text-transform: uppercase;
    color: var(--text-dim);
    letter-spacing: 0.04em;
  }

  .summary strong,
  .metric-grid strong {
    font-size: 15px;
  }

  .advisory {
    background: rgba(252, 244, 236, 0.88);
    color: #7a5634;
  }

  .detail-note {
    background: rgba(69, 120, 165, 0.08);
  }

  .detail-note p {
    margin: 4px 0 0;
    color: var(--text-muted);
  }

  .section {
    display: grid;
    gap: 12px;
  }

  .muted-copy {
    margin: 0;
    color: var(--text-muted);
  }

  .choice {
    display: grid;
    grid-template-columns: auto minmax(0, 1fr);
    align-items: start;
    gap: 10px;
    padding: 10px 12px;
    cursor: pointer;
  }

  .field {
    display: grid;
    gap: 6px;
  }

  .selection-card,
  .selection-actions {
    display: flex;
    align-items: flex-start;
    justify-content: space-between;
    gap: 12px;
  }

  .selection-card strong {
    display: block;
  }

  .field input {
    width: 100%;
    min-width: 0;
    padding: 9px 10px;
    border: 1px solid var(--app-border);
    border-radius: 6px;
    background: var(--panel-bg);
    color: var(--text-primary);
  }

  .status.ready {
    border-color: rgba(60, 130, 78, 0.28);
    background: rgba(236, 248, 239, 0.85);
  }

  .status.error {
    border-color: rgba(177, 71, 61, 0.28);
    background: rgba(250, 238, 236, 0.92);
  }

  .file-list {
    display: grid;
    gap: 10px;
  }

  .file-card {
    display: grid;
    gap: 10px;
  }

  .review-subsection {
    display: grid;
    gap: 10px;
    padding: 12px;
    border: 1px solid var(--app-border);
    border-radius: 6px;
    background: var(--surface-bg);
  }

  .file-card.blocked {
    border-color: rgba(177, 71, 61, 0.28);
  }

  .file-header {
    display: flex;
    justify-content: space-between;
    gap: 12px;
    align-items: start;
  }

  .badge {
    display: inline-flex;
    align-items: center;
    min-height: 28px;
    padding: 0 10px;
    border-radius: 6px;
    font-size: 12px;
    font-weight: 600;
    white-space: nowrap;
  }

  .badge.ok {
    background: rgba(66, 140, 84, 0.14);
    color: #2d6b3c;
  }

  .badge.warn {
    background: rgba(177, 71, 61, 0.14);
    color: #8e4338;
  }

  .issue-list {
    margin: 0;
    padding-left: 18px;
    display: grid;
    gap: 4px;
  }

  .raw-toggle {
    align-items: start;
  }

  .json-preview {
    margin: 0;
    padding: 12px;
    overflow: auto;
    border: 1px solid var(--app-border);
    border-radius: 6px;
    background: var(--panel-bg);
    color: var(--text-primary);
    font: 12px/1.5 ui-monospace, SFMono-Regular, SFMono-Regular, Consolas, "Liberation Mono",
      Menlo, monospace;
    white-space: pre-wrap;
    word-break: break-word;
  }

  .actions {
    display: flex;
    justify-content: flex-end;
    gap: 10px;
  }

  .footer-note {
    color: var(--text-muted);
    font-size: 12px;
  }

  button {
    min-height: 38px;
    padding: 0 14px;
    border: 1px solid transparent;
    border-radius: 6px;
    background: var(--accent-solid);
    color: white;
    font: inherit;
    cursor: pointer;
  }

  button.secondary {
    border-color: var(--app-border);
    background: var(--panel-bg);
    color: var(--text-primary);
  }

  button:disabled {
    cursor: default;
    opacity: 0.6;
  }

  @media (max-width: 820px) {
    .summary-grid,
    .field-grid,
    .metric-grid {
      grid-template-columns: 1fr;
    }

    .dialog {
      width: min(100vw - 16px, 100vw);
      padding: 14px;
    }

    .file-header {
      flex-direction: column;
    }

    .selection-card,
    .selection-actions {
      flex-direction: column;
      align-items: stretch;
    }
  }
</style>
