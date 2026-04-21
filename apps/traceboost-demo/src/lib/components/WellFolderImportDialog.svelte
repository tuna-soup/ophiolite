<svelte:options runes={true} />

<script lang="ts">
  import { onMount } from "svelte";
  import ImportFlowStepper from "./ImportFlowStepper.svelte";
  import ImportReviewChecklist from "./ImportReviewChecklist.svelte";
  import ImportReviewFieldSection from "./ImportReviewFieldSection.svelte";
  import CoordinateReferencePicker from "./CoordinateReferencePicker.svelte";
  import {
    commitProjectWellSourceImport,
    emitFrontendDiagnosticsEvent,
    previewProjectWellSourceImport,
    type CoordinateReferenceSelection,
    type ProjectWellSourceImportCanonicalDraft,
    type ProjectWellSourceImportCommitResponse,
    type ProjectWellFolderImportPreview,
    type ProjectWellMetadata,
    type ProjectWellSourceTopDraftRow,
    type ProjectWellboreMetadata,
    type WellSourceCoordinateReferenceCandidate,
    type WellSourceCoordinateReferenceSelection,
    type WellSourceCoordinateReferenceSelectionMode
  } from "../bridge";
  import { pickProjectFolder } from "../file-dialog";
  import {
    createWellSourceImportSession,
    emptyWellSourceImportDraft,
    emptyTrajectoryRow,
    selectedAsciiImportDrafts,
    selectedTrajectoryDraftRows,
    summarizeTrajectoryDraftRows,
    type WellImportMemory,
    type WellSourceImportDraft as ImportDraft,
    type WellSourceImportEditableAsciiLogImport as EditableAsciiLogImport,
    type WellSourceImportEditableTopRow as EditableTopRow,
    type WellSourceImportEditableTrajectoryRow as EditableTrajectoryRow,
    type WellSourceImportSessionState
  } from "../well-source-import-session";
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
    sourceRootPath: string;
    sourcePaths?: string[];
    close: () => void;
  }

  interface ReviewAsciiLogMappingRow {
    fileName: string;
    depthColumn: string;
    nullValue: string;
    sourceColumn: string;
    mnemonic: string;
    unit: string;
  }

  let { sourceRootPath, sourcePaths = [], close }: Props = $props();
  const REVIEW_ROW_LIMIT = 12;
  const REVIEW_MAPPING_LIMIT = 24;

  const viewerModel = getViewerModelContext();
  let importSession = $state.raw<WellSourceImportSessionState | null>(null);
  let preview = $state.raw<ProjectWellFolderImportPreview | null>(null);
  let commitResponse = $state.raw<ProjectWellSourceImportCommitResponse | null>(null);
  let loading = $state(true);
  let committing = $state(false);
  let errorMessage = $state<string | null>(null);
  let stage = $state<ImportConfirmationStage>("configure");
  let showRawCanonicalPayload = $state(false);
  let manualSourceCrsPickerOpen = $state(false);
  let draft = $state<ImportDraft>(emptyWellSourceImportDraft());
  let topsRows = $state<EditableTopRow[]>([]);
  let selectedLogSourcePaths = $state<string[]>([]);
  let asciiLogDrafts = $state<EditableAsciiLogImport[]>([]);
  let trajectoryRows = $state<EditableTrajectoryRow[]>([]);
  let projectRoot = $derived(viewerModel.projectRoot.trim());
  let selectedSourcePaths = $derived(sourcePaths.filter((value) => value.trim().length > 0));
  let sourceRootLabel = $derived(sourceRootPath.trim() || "Not resolved from selection");
  let importSelectionLabel = $derived.by(() => {
    if (selectedSourcePaths.length > 0) {
      return `${selectedSourcePaths.length} selected file${selectedSourcePaths.length === 1 ? "" : "s"}`;
    }
    return sourceRootPath;
  });
  let activeSurveyCoordinateReferenceId = $derived(
    viewerModel.activeEffectiveNativeCoordinateReferenceId ?? ""
  );
  let activeSurveyCoordinateReferenceName = $derived(
    viewerModel.activeEffectiveNativeCoordinateReferenceName ?? ""
  );
  let activeSurveyCoordinateReferenceAvailable = $derived(
    activeSurveyCoordinateReferenceId.trim().length > 0 ||
      activeSurveyCoordinateReferenceName.trim().length > 0
  );
  let logsImportAvailable = $derived(
    !!preview && (preview.logs.commitEnabled || preview.asciiLogs.commitEnabled)
  );
  let detectedCandidates = $derived(preview?.sourceCoordinateReference.candidates ?? []);
  let selectedDetectedCandidate = $derived.by(() =>
    resolveDetectedCandidate(preview, draft.detectedCandidateId)
  );
  let resolvedSourceCoordinateReference = $derived.by(() =>
    resolveSelectedSourceCoordinateReference(
      preview,
      draft,
      activeSurveyCoordinateReferenceId,
      activeSurveyCoordinateReferenceName
    )
  );
  let geometryCommitAllowed = $derived(resolvedSourceCoordinateReference !== null);
  let surfaceLocationPresent = $derived(
    draft.surfaceX.trim().length > 0 && draft.surfaceY.trim().length > 0
  );
  let surfaceLocationWillBeCommitted = $derived(
    surfaceLocationPresent && geometryCommitAllowed
  );
  let resolvedSourceCoordinateReferenceLabel = $derived(
    formatCoordinateReference(resolvedSourceCoordinateReference)
  );
  let selectedLogFiles = $derived(
    preview && draft.importLogs && preview.logs.commitEnabled
      ? preview.logs.files.filter((file) => selectedLogSourcePaths.includes(file.sourcePath))
      : []
  );
  let selectedAsciiLogImports = $derived.by(() =>
    selectedAsciiImportDrafts(draft, asciiLogDrafts, parseOptionalNumber, blankToNull)
  );
  let selectedAsciiLogFiles = $derived.by(() => {
    if (!preview) {
      return [];
    }
    const selectedSourcePaths = new Set(selectedAsciiLogImports.map((entry) => entry.sourcePath));
    return preview.asciiLogs.files.filter((file) => selectedSourcePaths.has(file.sourcePath));
  });
  let editedTopRowsForRequest = $derived(topsRowsForRequest(topsRows));
  let committableEditedTopRows = $derived(
    editedTopRowsForRequest.filter((row) => row.name && row.topDepth !== null)
  );
  let editedTrajectoryRowsForRequest = $derived(
    selectedTrajectoryDraftRows(trajectoryRows, parseOptionalNumber)
  );
  let trajectoryDraftSupport = $derived(
    summarizeTrajectoryDraftRows(editedTrajectoryRowsForRequest)
  );
  let omittedEditedTopRowCount = $derived(
    editedTopRowsForRequest.length - committableEditedTopRows.length
  );
  let topsImportSelected = $derived(!!preview && draft.importTopsMarkers && preview.topsMarkers.commitEnabled);
  let trajectorySourcePresent = $derived(!!preview?.trajectory.sourcePath);
  let trajectoryNeedsSupplement = $derived(
    !!preview?.trajectory.sourcePath && !preview.trajectory.commitEnabled
  );
  let trajectoryReadyForCommit = $derived(
    !!preview &&
      !!preview.trajectory.sourcePath &&
      (preview.trajectory.commitEnabled || trajectoryDraftSupport.commitEnabled)
  );
  let trajectoryCommitAllowed = $derived(trajectoryReadyForCommit && geometryCommitAllowed);
  let trajectoryCommittedStationCount = $derived(
    preview?.trajectory.commitEnabled
      ? preview.trajectory.committableRowCount
      : trajectoryDraftSupport.committableRowCount
  );
  let trajectoryImportSelected = $derived(draft.importTrajectory && trajectoryCommitAllowed);
  let selectedCanonicalImportCount = $derived.by(() => {
    const logCount = selectedLogFiles.length;
    const asciiLogCount = selectedAsciiLogImports.length;
    const topsCount = topsImportSelected ? 1 : 0;
    const trajectoryCount = trajectoryImportSelected ? 1 : 0;
    return logCount + asciiLogCount + topsCount + trajectoryCount;
  });
  let unsupportedPreservationMode = $derived.by(() => {
    if (!preview || preview.unsupportedSources.length === 0) {
      return null;
    }
    return selectedCanonicalImportCount > 0 ? "source_artifacts" : "raw_source_bundle";
  });
  let commitSelection = $derived.by(() =>
    buildSourceCoordinateReferenceSelection(
      draft,
      selectedDetectedCandidate,
      resolvedSourceCoordinateReference
    )
  );
  let canonicalWellMetadata = $derived.by(() =>
    preview ? buildWellMetadata(preview, draft, resolvedSourceCoordinateReference) : null
  );
  let canonicalWellboreMetadata = $derived.by(() =>
    preview ? buildWellboreMetadata(preview, draft) : null
  );
  let canonicalImportDraft = $derived.by((): ProjectWellSourceImportCanonicalDraft | null => {
    if (!preview) {
      return null;
    }
    return {
      binding: {
        well_name: draft.wellName.trim(),
        wellbore_name: draft.wellboreName.trim(),
        uwi: blankToNull(draft.uwi),
        api: blankToNull(draft.api),
        operator_aliases: []
      },
      sourceCoordinateReference: commitSelection,
      wellMetadata: canonicalWellMetadata,
      wellboreMetadata: canonicalWellboreMetadata,
      importPlan: {
        selectedLogSourcePaths: draft.importLogs ? selectedLogSourcePaths : null,
        asciiLogImports: draft.importLogs ? selectedAsciiLogImports : null,
        topsMarkers:
          draft.importTopsMarkers && preview.topsMarkers.commitEnabled
            ? {
                depthReference: blankToNull(draft.topsDepthReference),
                rows: committableEditedTopRows
              }
            : null,
        trajectory:
          draft.importTrajectory && trajectoryCommitAllowed
            ? {
                enabled: true,
                rows: editedTrajectoryRowsForRequest
              }
            : null,
      }
    };
  });
  let canonicalPreviewText = $derived(
    canonicalImportDraft
      ? JSON.stringify(
          {
            projectRoot: projectRoot || "<unset>",
            sourceRootPath,
            sourcePaths: selectedSourcePaths.length > 0 ? selectedSourcePaths : null,
            session: importSession
              ? {
                  suggestedDraft: importSession.suggestedDraft,
                  editable: {
                    draft,
                    topsRows,
                    selectedLogSourcePaths,
                    asciiLogDrafts,
                    trajectoryRows
                  }
                }
              : null,
            draft: canonicalImportDraft
          },
          null,
          2
        )
      : ""
  );
  let reviewSelectionFields = $derived.by(() =>
    compactImportReviewFields([
      [
        "Selection Mode",
        selectedSourcePaths.length > 0 ? "Selected files only" : "Source root scan"
      ],
      [
        "Selected Sources",
        selectedSourcePaths.length > 0 ? importSelectionLabel : "Selection root only"
      ],
      ["Source Root", sourceRootLabel]
    ])
  );
  let reviewBindingFields = $derived.by(() =>
    compactImportReviewFields([
      ["Well Name", draft.wellName.trim()],
      ["Wellbore Name", draft.wellboreName.trim()],
      ["UWI", draft.uwi.trim()],
      ["API", draft.api.trim()]
    ])
  );
  let reviewWellMetadataFields = $derived.by(() => {
    if (!canonicalWellMetadata) {
      return [];
    }
    return compactImportReviewFields([
      ["Field", canonicalWellMetadata.field_name],
      ["Block", canonicalWellMetadata.block_name],
      ["Country", canonicalWellMetadata.country],
      ["Province", canonicalWellMetadata.province_state],
      ["Location", canonicalWellMetadata.location_text],
      ["Interest Type", canonicalWellMetadata.interest_type]
    ]);
  });
  let reviewWellboreFields = $derived.by(() => {
    if (!canonicalWellboreMetadata) {
      return [];
    }
    return compactImportReviewFields([
      ["Status", canonicalWellboreMetadata.status],
      ["Purpose", canonicalWellboreMetadata.purpose],
      ["Trajectory Type", canonicalWellboreMetadata.trajectory_type],
      ["Parent Wellbore", canonicalWellboreMetadata.parent_wellbore_id],
      ["Service Company", canonicalWellboreMetadata.service_company_name],
      ["Location", canonicalWellboreMetadata.location_text]
    ]);
  });
  let reviewGeometryFields = $derived.by(() =>
    compactImportReviewFields([
      ["Resolved Source CRS", resolvedSourceCoordinateReferenceLabel],
      [
        "Surface Location",
        surfaceLocationPresent ? `${draft.surfaceX.trim()}, ${draft.surfaceY.trim()}` : "Not provided"
      ],
      [
        "Surface Location Status",
        surfaceLocationPresent
          ? surfaceLocationWillBeCommitted
            ? "Committed"
            : "Withheld until CRS is confirmed"
          : "Omitted"
      ],
      [
        "Trajectory",
        draft.importTrajectory && trajectoryCommitAllowed
          ? `${trajectoryCommittedStationCount} stations`
          : draft.importTrajectory && !geometryCommitAllowed
            ? "Withheld until CRS is confirmed"
            : draft.importTrajectory
              ? "Waiting on supplemented stations"
            : "Omitted"
      ]
    ])
  );
  let reviewCanonicalAssets = $derived.by(() => {
    const items: ReviewField[] = [];
    for (const file of selectedLogFiles) {
      items.push({
        label: "LAS Log",
        value: `${file.fileName} (${file.curveCount} curves, ${file.rowCount} rows)`
      });
    }
    for (const file of selectedAsciiLogImports) {
      const previewFile = preview?.asciiLogs.files.find(
        (candidate) => candidate.sourcePath === file.sourcePath
      );
      items.push({
        label: "NLOG ASCII Log",
        value: `${previewFile?.fileName ?? file.sourcePath} (${file.valueColumns.length} mapped curves from ${file.depthColumn})`
      });
    }
    if (topsImportSelected) {
      items.push({
        label: "Top Set",
        value: `${committableEditedTopRows.length} rows (${draft.topsDepthReference.trim() || "md"})`
      });
    }
    if (trajectoryImportSelected) {
      items.push({
        label: "Trajectory",
        value: `${trajectoryCommittedStationCount} stations`
      });
    }
    if (unsupportedPreservationMode === "raw_source_bundle") {
      items.push({
        label: "Raw Source Bundle",
        value: `${preview?.unsupportedSources.length ?? 0} unsupported file${preview?.unsupportedSources.length === 1 ? "" : "s"}`
      });
    }
    return items;
  });
  let reviewSourcePreservationFields = $derived.by(() => {
    const fields: ReviewField[] = [];
    if (preview?.unsupportedSources.length) {
      fields.push({
        label: "Unsupported Files",
        value:
          unsupportedPreservationMode === "source_artifacts"
            ? "Preserved as source artifacts on canonical assets"
            : "Preserved as a raw source bundle"
      });
    }
    if (preview?.logs.files.length) {
      const omittedLasCount = preview.logs.files.length - selectedLogFiles.length;
      if (omittedLasCount > 0) {
        fields.push({
          label: "Unselected LAS",
          value: `${omittedLasCount} preserved as source-only`
        });
      }
    }
    if (preview?.asciiLogs.files.length) {
      const omittedAsciiCount = preview.asciiLogs.files.length - selectedAsciiLogImports.length;
      if (omittedAsciiCount > 0) {
        fields.push({
          label: "Unmapped NLOG ASCII",
          value: `${omittedAsciiCount} preserved as source-only`
        });
      }
    }
    if (draft.importTopsMarkers && omittedEditedTopRowCount > 0) {
      fields.push({
        label: "Incomplete Tops Rows",
        value: `${omittedEditedTopRowCount} omitted from the canonical top set`
      });
    }
    return fields;
  });
  let projectedImportedAssetCount = $derived(
    selectedCanonicalImportCount + (unsupportedPreservationMode === "raw_source_bundle" ? 1 : 0)
  );
  let projectedImportedAssetKinds = $derived.by(() => {
    const assetKinds: string[] = [];
    for (const _ of selectedLogFiles) {
      assetKinds.push("log");
    }
    for (const _ of selectedAsciiLogImports) {
      assetKinds.push("log");
    }
    if (topsImportSelected) {
      assetKinds.push("top_set");
    }
    if (trajectoryImportSelected) {
      assetKinds.push("trajectory");
    }
    if (unsupportedPreservationMode === "raw_source_bundle") {
      assetKinds.push("raw_source_bundle");
    }
    return assetKinds;
  });
  let reviewTopRows = $derived(
    topsImportSelected ? committableEditedTopRows.slice(0, REVIEW_ROW_LIMIT) : []
  );
  let reviewLasLogRows = $derived(selectedLogFiles.slice(0, REVIEW_ROW_LIMIT));
  let reviewLasLogRowsHiddenCount = $derived(
    Math.max(selectedLogFiles.length - reviewLasLogRows.length, 0)
  );
  let reviewAsciiLogMappingRows = $derived.by((): ReviewAsciiLogMappingRow[] =>
    selectedAsciiLogImports.flatMap((entry) => {
      const previewFile = preview?.asciiLogs.files.find(
        (candidate) => candidate.sourcePath === entry.sourcePath
      );
      const fileName = previewFile?.fileName ?? entry.sourcePath;
      const depthColumn = entry.depthColumn.trim() || "—";
      const nullValue =
        entry.nullValue === null || entry.nullValue === undefined ? "—" : String(entry.nullValue);
      return entry.valueColumns.map((valueColumn) => ({
        fileName,
        depthColumn,
        nullValue,
        sourceColumn: valueColumn.sourceColumn,
        mnemonic: valueColumn.mnemonic,
        unit: valueColumn.unit?.trim() || "—"
      }));
    })
  );
  let reviewAsciiLogMappingPreviewRows = $derived(
    reviewAsciiLogMappingRows.slice(0, REVIEW_MAPPING_LIMIT)
  );
  let reviewAsciiLogMappingHiddenCount = $derived(
    Math.max(reviewAsciiLogMappingRows.length - reviewAsciiLogMappingPreviewRows.length, 0)
  );
  let reviewTopRowsHiddenCount = $derived(
    topsImportSelected ? Math.max(committableEditedTopRows.length - reviewTopRows.length, 0) : 0
  );
  let reviewTrajectoryRows = $derived(
    draft.importTrajectory && trajectoryCommitAllowed
      ? editedTrajectoryRowsForRequest.slice(0, REVIEW_ROW_LIMIT)
      : []
  );
  let reviewTrajectoryRowsHiddenCount = $derived(
    draft.importTrajectory && trajectoryCommitAllowed
      ? Math.max(editedTrajectoryRowsForRequest.length - reviewTrajectoryRows.length, 0)
      : 0
  );
  let reviewItems = $derived.by(() => {
    const items: ReviewItem[] = [];
    if (!projectRoot) {
      items.push({
        severity: "blocking",
        title: "Project storage location required",
        message: "Preview works without it, but commit needs an Ophiolite project location where the imported wells will be stored."
      });
    }
    if (draft.wellName.trim().length === 0 || draft.wellboreName.trim().length === 0) {
      items.push({
        severity: "blocking",
        title: "Binding incomplete",
        message: "Well name and wellbore name must be confirmed before import."
      });
    }
    if (draft.importTrajectory && !geometryCommitAllowed && preview?.trajectory.sourcePath) {
      items.push({
        severity: "blocking",
        title: "Trajectory needs CRS confirmation",
        message: "The trajectory file is present, but geometry cannot be committed until the source CRS path is confirmed."
      });
    }
    if (draft.importTrajectory && geometryCommitAllowed && !trajectoryReadyForCommit && preview?.trajectory.sourcePath) {
      items.push({
        severity: "blocking",
        title: "Trajectory stations still need supplementation",
        message:
          "The selected trajectory draft still needs at least two stations with measured depth plus inclination/azimuth or measured depth plus TVD and XY offsets."
      });
    }
    if (
      preview &&
      draft.sourceCrsMode === "unresolved" &&
      preview.sourceCoordinateReference.candidates.length > 0 &&
      (surfaceLocationPresent || !!preview.trajectory.sourcePath)
    ) {
      items.push({
        severity: "warning",
        title: "Geometry is waiting on an explicit CRS decision",
        message:
          "A source CRS candidate was detected, but surface and trajectory geometry stay out until you explicitly choose how to interpret the coordinates."
      });
    }
    if (surfaceLocationPresent && !surfaceLocationWillBeCommitted) {
      items.push({
        severity: "warning",
        title: "Surface location will be omitted",
        message: "Surface X/Y values are present, but they will stay out of the project until the source CRS is resolved."
      });
    }
    if (topsRows.length > 0 && omittedEditedTopRowCount > 0) {
      items.push({
        severity: "warning",
        title: "Some tops rows will be omitted",
        message: `${omittedEditedTopRowCount} tops rows are incomplete and will not be translated into canonical tops.`
      });
    }
    if (preview?.asciiLogs.files.length) {
      const unmappedAsciiFiles = preview.asciiLogs.files.length - selectedAsciiLogImports.length;
      if (unmappedAsciiFiles > 0) {
        items.push({
          severity: "info",
          title: "Some NLOG ASCII tables are source-only",
          message: `${unmappedAsciiFiles} ASCII table${unmappedAsciiFiles === 1 ? "" : "s"} will stay preserved as source until you confirm a mapping.`
        });
      }
    }
    if (preview && preview.unsupportedSources.length > 0) {
      items.push({
        severity: "info",
        title: "Unsupported files will still be preserved",
        message:
          unsupportedPreservationMode === "source_artifacts"
            ? "Unsupported files will be attached to the imported canonical assets as preserved source artifacts."
            : "Unsupported files will be imported as a raw source bundle because no canonical asset owns them."
      });
    }
    if (preview?.issues.some((issue) => issue.severity === "blocking")) {
      items.push({
        severity: "warning",
        title: "Preview contains blocking parser issues",
        message: "Some detected slices are incomplete or not viable. Review the issues section before confirming."
      });
    }
    if (items.length === 0) {
      items.push({
        severity: "info",
        title: "Ready to confirm",
        message: "The current draft resolves all required inputs for the selected import plan."
      });
    }
    return items;
  });
  let reviewSections = $derived.by((): ImportReviewSection[] => [
    {
      title: "Source Selection",
      fields: reviewSelectionFields
    },
    {
      title: "Binding",
      fields: reviewBindingFields,
      emptyMessage: "No binding fields have been confirmed yet."
    },
    {
      title: "Well Metadata",
      fields: reviewWellMetadataFields,
      emptyMessage: "No well metadata fields will be updated."
    },
    {
      title: "Wellbore Metadata",
      fields: reviewWellboreFields,
      emptyMessage: "No wellbore metadata fields will be updated."
    },
    {
      title: "Geometry Decisions",
      fields: reviewGeometryFields
    },
    {
      title: "Canonical Assets",
      fields: reviewCanonicalAssets,
      emptyMessage:
        "No canonical assets will be created. This confirmation will only update metadata or preserve source files.",
      wide: true
    },
    {
      title: "Source Preservation And Omissions",
      fields: reviewSourcePreservationFields,
      emptyMessage:
        "No source-only preservation outcomes are expected for the current selection.",
      wide: true
    }
  ]);
  let flowSteps = $derived.by((): ImportFlowStep[] => [
    {
      key: "configure",
      label: "1. Configure Translation",
      description:
        "Adjust the parsed fields and choose what should translate into canonical assets.",
      disabled: stage === "result"
    },
    {
      key: "review",
      label: "2. Review Canonical Draft",
      description:
        "This is the final draft that will be committed. Go back if any field or mapping still needs work.",
      disabled: stage === "result"
    },
    {
      key: "result",
      label: "3. Import Result",
      description:
        "Review the imported assets, omissions, and preservation outcomes before closing the dialog.",
      disabled: stage !== "result"
    }
  ]);
  let confirmDisabledReason = $derived.by(() => {
    if (committing) {
      return "Import in progress.";
    }
    if (!projectRoot) {
      return "Choose an Ophiolite project location for the imported well assets.";
    }
    if (draft.wellName.trim().length === 0) {
      return "Enter a well name.";
    }
    if (draft.wellboreName.trim().length === 0) {
      return "Enter a wellbore name.";
    }
    if (draft.importTrajectory && !geometryCommitAllowed) {
      return "Confirm the source CRS before importing trajectory geometry.";
    }
    if (draft.importTrajectory && !trajectoryReadyForCommit) {
      return "Supplement the trajectory draft with at least two viable stations before importing it.";
    }
    return null;
  });
  let confirmButtonLabel = $derived.by(() => {
    if (committing) {
      return "Importing...";
    }
    if (projectedImportedAssetKinds.length === 0) {
      return "Confirm Metadata Update";
    }
    return `Confirm Import (${projectedImportedAssetKinds.length})`;
  });
  let resultImportedAssetKinds = $derived(
    commitResponse?.importedAssets.map((asset) => asset.assetKind) ?? []
  );

  onMount(() => {
    void loadPreview();
  });

  function recordWellImportDiagnostics(
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
        "Failed to record well import diagnostics.",
        "backend",
        "warn",
        error instanceof Error ? error.message : String(error)
      );
    });
  }

  function applyDefaultSourceCrsMode(): void {
    if (
      draft.sourceCrsMode === "unresolved" &&
      activeSurveyCoordinateReferenceAvailable
    ) {
      draft.sourceCrsMode = "assume_same_as_survey";
    }
  }

  async function loadPreview(): Promise<void> {
    loading = true;
    errorMessage = null;
    commitResponse = null;
    showRawCanonicalPayload = false;
    try {
      const nextPreview = await previewProjectWellSourceImport({
        sourceRootPath,
        sourcePaths: selectedSourcePaths.length > 0 ? selectedSourcePaths : null
      });
      importSession = createWellSourceImportSession(nextPreview);
      preview = importSession.preview;
      stage = "configure";
      draft = importSession.draft;
      topsRows = importSession.topsRows;
      selectedLogSourcePaths = importSession.selectedLogSourcePaths;
      asciiLogDrafts = importSession.asciiLogDrafts;
      trajectoryRows = importSession.trajectoryRows;
      applyDefaultSourceCrsMode();
      applyImportMemory();
      syncGeometrySelections(importSession.preview);
      recordWellImportDiagnostics("preview_well_sources", "info", "Frontend well source preview ready", {
        sourceRootPath,
        selectedSourcePathCount: selectedSourcePaths.length,
        issueCount: importSession.preview.issues.length,
        logFileCount: importSession.preview.logs.files.length,
        asciiLogFileCount: importSession.preview.asciiLogs.files.length,
        topsRowCount: importSession.preview.topsMarkers.rowCount,
        trajectoryRowCount: importSession.preview.trajectory.rowCount,
        unsupportedSourceCount: importSession.preview.unsupportedSources.length,
        sourceCrsCandidateCount: importSession.preview.sourceCoordinateReference.candidates.length
      });
    } catch (error) {
      errorMessage = error instanceof Error ? error.message : String(error);
      viewerModel.note("Failed to preview well import sources.", "backend", "warn", errorMessage);
      recordWellImportDiagnostics("preview_well_sources", "error", "Frontend well source preview failed", {
        sourceRootPath,
        selectedSourcePathCount: selectedSourcePaths.length,
        error: errorMessage
      });
    } finally {
      loading = false;
    }
  }

  function goToReview(): void {
    recordWellImportDiagnostics("review_well_sources", "info", "Opened well source review draft", {
      projectRoot: projectRoot || null,
      selectedSourcePathCount: selectedSourcePaths.length,
      selectedLasLogCount: selectedLogFiles.length,
      selectedAsciiLogCount: selectedAsciiLogImports.length,
      topsImportSelected,
      trajectoryImportSelected,
      geometryCommitAllowed,
      trajectoryReadyForCommit
    });
    stage = "review";
  }

  function goToConfigure(): void {
    stage = "configure";
  }

  async function chooseProjectRoot(): Promise<void> {
    const pickedProjectRoot = await pickProjectFolder();
    if (pickedProjectRoot) {
      await viewerModel.setProjectRoot(pickedProjectRoot);
    }
  }

  async function confirmImport(): Promise<void> {
    if (!preview) {
      return;
    }
    if (!canonicalImportDraft) {
      viewerModel.note("The canonical well import draft could not be prepared.", "ui", "warn");
      recordWellImportDiagnostics("commit_well_sources", "warn", "Well source commit blocked because the canonical draft is unavailable", {
        projectRoot: projectRoot || null,
        selectedSourcePathCount: selectedSourcePaths.length
      });
      return;
    }
    if (!projectRoot) {
      viewerModel.note(
        "Choose where the imported wells should be stored. Preview works without it, but commit needs an Ophiolite project location.",
        "ui",
        "warn"
      );
      recordWellImportDiagnostics("commit_well_sources", "warn", "Well source commit blocked because no project storage location is set", {
        selectedSourcePathCount: selectedSourcePaths.length,
        selectedLasLogCount: selectedLogFiles.length,
        selectedAsciiLogCount: selectedAsciiLogImports.length,
        topsImportSelected,
        trajectoryImportSelected
      });
      return;
    }
    if (draft.importTrajectory && !geometryCommitAllowed) {
      viewerModel.note(
        "Confirm a source CRS before importing trajectory geometry.",
        "ui",
        "warn"
      );
      recordWellImportDiagnostics("commit_well_sources", "warn", "Well source commit blocked because geometry CRS is unresolved", {
        projectRoot,
        selectedSourcePathCount: selectedSourcePaths.length
      });
      return;
    }
    if (draft.importTrajectory && !trajectoryReadyForCommit) {
      viewerModel.note(
        "Add at least two viable trajectory stations before importing trajectory geometry.",
        "ui",
        "warn"
      );
      recordWellImportDiagnostics("commit_well_sources", "warn", "Well source commit blocked because trajectory stations are incomplete", {
        projectRoot,
        selectedSourcePathCount: selectedSourcePaths.length,
        trajectoryRowCount: editedTrajectoryRowsForRequest.length
      });
      return;
    }

    committing = true;
    errorMessage = null;
    recordWellImportDiagnostics("commit_well_sources", "info", "Started well source commit from the confirmation dialog", {
      projectRoot,
      sourceRootPath,
      selectedSourcePathCount: selectedSourcePaths.length,
      selectedLasLogCount: selectedLogFiles.length,
      selectedAsciiLogCount: selectedAsciiLogImports.length,
      topsRowCount: committableEditedTopRows.length,
      trajectoryRowCount: editedTrajectoryRowsForRequest.length,
      sourceCrsMode: draft.sourceCrsMode
    });
    try {
      const response = await commitProjectWellSourceImport({
        projectRoot,
        sourceRootPath,
        sourcePaths: selectedSourcePaths.length > 0 ? selectedSourcePaths : null,
        draft: canonicalImportDraft
      });
      viewerModel.note(
        "Imported selected well sources.",
        "backend",
        "info",
        `${response.wellId}:${response.importedAssets.map((asset) => asset.assetKind).join(", ")}`
      );
      await viewerModel.refreshProjectWellOverlayInventory(
        projectRoot,
        viewerModel.displayCoordinateReferenceId
      );
      saveImportMemory();
      recordWellImportDiagnostics("commit_well_sources", "info", "Well source commit completed", {
        projectRoot,
        wellId: response.wellId,
        wellboreId: response.wellboreId,
        createdWell: response.createdWell,
        createdWellbore: response.createdWellbore,
        importedAssetCount: response.importedAssets.length,
        omissionCount: response.omissions.length,
        issueCount: response.issues.length
      });
      commitResponse = response;
      stage = "result";
    } catch (error) {
      errorMessage = error instanceof Error ? error.message : String(error);
      viewerModel.note("Failed to commit selected well sources.", "backend", "warn", errorMessage);
      recordWellImportDiagnostics("commit_well_sources", "error", "Well source commit failed", {
        projectRoot,
        sourceRootPath,
        selectedSourcePathCount: selectedSourcePaths.length,
        error: errorMessage
      });
    } finally {
      committing = false;
    }
  }

  function topsRowsForRequest(rows: EditableTopRow[]): ProjectWellSourceTopDraftRow[] {
    return rows.map((row) => ({
      name: blankToNull(row.name),
      topDepth: parseOptionalNumber(row.topDepth),
      baseDepth: parseOptionalNumber(row.baseDepth),
      anomaly: blankToNull(row.anomaly),
      quality: blankToNull(row.quality),
      note: blankToNull(row.note)
    }));
  }

  function findAsciiLogDraft(sourcePath: string): EditableAsciiLogImport | undefined {
    return asciiLogDrafts.find((entry) => entry.sourcePath === sourcePath);
  }

  function toggleAsciiCurve(
    sourcePath: string,
    sourceColumn: string,
    enabled: boolean
  ): void {
    const entry = findAsciiLogDraft(sourcePath);
    const curve = entry?.curves.find((item) => item.sourceColumn === sourceColumn);
    if (curve) {
      curve.enabled = enabled;
    }
  }

  function resolveDetectedCandidate(
    nextPreview: ProjectWellFolderImportPreview | null,
    candidateId: string
  ): WellSourceCoordinateReferenceCandidate | null {
    if (!nextPreview) {
      return null;
    }
    return (
      nextPreview.sourceCoordinateReference.candidates.find(
        (candidate) =>
          normalizeComparableId(candidate.coordinateReference.id) ===
          normalizeComparableId(candidateId)
      ) ??
      nextPreview.sourceCoordinateReference.candidates.find(
        (candidate) =>
          normalizeComparableId(candidate.coordinateReference.id) ===
          normalizeComparableId(nextPreview.sourceCoordinateReference.recommendedCandidateId ?? "")
      ) ??
      nextPreview.sourceCoordinateReference.candidates[0] ??
      null
    );
  }

  function resolveSelectedSourceCoordinateReference(
    nextPreview: ProjectWellFolderImportPreview | null,
    nextDraft: ImportDraft,
    surveyCoordinateReferenceId: string,
    surveyCoordinateReferenceName: string
  ) {
    switch (nextDraft.sourceCrsMode) {
      case "detected":
        return resolveDetectedCandidate(nextPreview, nextDraft.detectedCandidateId)?.coordinateReference ?? null;
      case "assume_same_as_survey":
        if (
          surveyCoordinateReferenceId.trim().length === 0 &&
          surveyCoordinateReferenceName.trim().length === 0
        ) {
          return null;
        }
        return {
          id: blankToNull(surveyCoordinateReferenceId),
          name: blankToNull(surveyCoordinateReferenceName),
          geodetic_datum: null,
          unit: null
        };
      case "manual":
        if (
          nextDraft.manualSourceCrsId.trim().length === 0 &&
          nextDraft.manualSourceCrsName.trim().length === 0
        ) {
          return null;
        }
        return {
          id: blankToNull(nextDraft.manualSourceCrsId),
          name: blankToNull(nextDraft.manualSourceCrsName),
          geodetic_datum: null,
          unit: null
        };
      default:
        return null;
    }
  }

  function buildSourceCoordinateReferenceSelection(
    nextDraft: ImportDraft,
    detectedCandidate: WellSourceCoordinateReferenceCandidate | null,
    selectedCoordinateReference: ReturnType<typeof resolveSelectedSourceCoordinateReference>
  ): WellSourceCoordinateReferenceSelection {
    return {
      mode: nextDraft.sourceCrsMode,
      candidateId:
        nextDraft.sourceCrsMode === "detected"
          ? detectedCandidate?.coordinateReference.id ?? null
          : null,
      coordinateReference: selectedCoordinateReference
    };
  }

  function buildWellMetadata(
    nextPreview: ProjectWellFolderImportPreview,
    nextDraft: ImportDraft,
    selectedCoordinateReference: ReturnType<typeof resolveSelectedSourceCoordinateReference>
  ): ProjectWellMetadata {
    const base = nextPreview.metadata.wellMetadata ?? {};
    const surfaceLocation =
      nextDraft.surfaceX.trim() && nextDraft.surfaceY.trim()
        ? {
            coordinate_reference: selectedCoordinateReference,
            point: {
              x: Number(nextDraft.surfaceX),
              y: Number(nextDraft.surfaceY)
            },
            recorded_at: base.surface_location?.recorded_at ?? null,
            source: base.surface_location?.source ?? "well_folder_import",
            note: base.surface_location?.note ?? null
          }
        : null;
    return {
      ...base,
      field_name: blankToNull(nextDraft.fieldName),
      block_name: blankToNull(nextDraft.blockName),
      country: blankToNull(nextDraft.country),
      province_state: blankToNull(nextDraft.provinceState),
      location_text: blankToNull(nextDraft.locationText),
      interest_type: blankToNull(nextDraft.interestType),
      surface_location: surfaceLocation
    };
  }

  function buildWellboreMetadata(
    nextPreview: ProjectWellFolderImportPreview,
    nextDraft: ImportDraft
  ): ProjectWellboreMetadata {
    const base = nextPreview.metadata.wellboreMetadata ?? {};
    return {
      ...base,
      status: blankToNull(nextDraft.wellboreStatus),
      purpose: blankToNull(nextDraft.wellborePurpose),
      trajectory_type: blankToNull(nextDraft.trajectoryType),
      parent_wellbore_id: blankToNull(nextDraft.parentWellboreId),
      service_company_name: blankToNull(nextDraft.serviceCompanyName),
      location_text: blankToNull(nextDraft.wellboreLocationText)
    };
  }

  function chooseSourceCrsMode(mode: WellSourceCoordinateReferenceSelectionMode): void {
    draft.sourceCrsMode = mode;
    syncGeometrySelections(preview);
  }

  function openManualSourceCrsPicker(): void {
    draft.sourceCrsMode = "manual";
    manualSourceCrsPickerOpen = true;
  }

  function closeManualSourceCrsPicker(): void {
    manualSourceCrsPickerOpen = false;
  }

  function handleManualSourceCrsSelection(selection: CoordinateReferenceSelection): void {
    draft.sourceCrsMode = "manual";
    if (selection.kind === "authority_code") {
      draft.manualSourceCrsId = selection.authId;
      draft.manualSourceCrsName = selection.name?.trim() ?? "";
    } else if (selection.kind === "local_engineering") {
      draft.manualSourceCrsId = "";
      draft.manualSourceCrsName = selection.label.trim();
    }
    manualSourceCrsPickerOpen = false;
    syncGeometrySelections(preview);
  }

  function clearManualSourceCrsSelection(): void {
    draft.manualSourceCrsId = "";
    draft.manualSourceCrsName = "";
    syncGeometrySelections(preview);
  }

  function addTrajectoryRow(): void {
    trajectoryRows = [...trajectoryRows, emptyTrajectoryRow()];
  }

  function removeTrajectoryRow(index: number): void {
    trajectoryRows = trajectoryRows.filter((_, currentIndex) => currentIndex !== index);
  }

  function syncGeometrySelections(nextPreview: ProjectWellFolderImportPreview | null): void {
    const nextCoordinateReference = resolveSelectedSourceCoordinateReference(
      nextPreview,
      draft,
      activeSurveyCoordinateReferenceId,
      activeSurveyCoordinateReferenceName
    );
    if (!nextCoordinateReference) {
      draft.importTrajectory = false;
    } else if (
      nextPreview?.trajectory.sourcePath &&
      (nextPreview.trajectory.commitEnabled || trajectoryDraftSupport.commitEnabled)
    ) {
      draft.importTrajectory = true;
    }
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
    return "Unresolved";
  }

  function normalizeComparableId(value: string | null | undefined): string {
    return value?.trim().toLowerCase() ?? "";
  }

  function blankToNull(value: string): string | null {
    const trimmed = value.trim();
    return trimmed.length > 0 ? trimmed : null;
  }

  function parseOptionalNumber(value: string): number | null {
    const trimmed = value.trim();
    if (trimmed.length === 0) {
      return null;
    }
    const parsed = Number(trimmed);
    return Number.isFinite(parsed) ? parsed : null;
  }

  function formatReviewNumber(value: number | null | undefined): string {
    if (value === null || value === undefined || !Number.isFinite(value)) {
      return "—";
    }
    return String(value);
  }

  function formatCurvePreview(curveNames: string[]): string {
    if (curveNames.length === 0) {
      return "—";
    }
    const previewCurveNames = curveNames.slice(0, 6);
    const remainingCount = curveNames.length - previewCurveNames.length;
    if (remainingCount > 0) {
      return `${previewCurveNames.join(", ")} +${remainingCount} more`;
    }
    return previewCurveNames.join(", ");
  }

  function topRowReferenceLabel(row: EditableTopRow): string {
    return [row.anomaly.trim(), row.quality.trim(), row.note.trim()]
      .filter((value) => value.length > 0)
      .join(" | ");
  }

  function importMemoryStorageKey(currentProjectRoot: string): string | null {
    const normalizedProjectRoot = currentProjectRoot.trim();
    if (normalizedProjectRoot.length === 0) {
      return null;
    }
    return `traceboost.well-import-memory:${normalizedProjectRoot}`;
  }

  function loadImportMemory(): WellImportMemory | null {
    const storageKey = importMemoryStorageKey(projectRoot);
    if (!storageKey || typeof window === "undefined") {
      return null;
    }
    try {
      const raw = window.localStorage.getItem(storageKey);
      if (!raw) {
        return null;
      }
      return JSON.parse(raw) as WellImportMemory;
    } catch {
      return null;
    }
  }

  function applyImportMemory(): void {
    const memory = loadImportMemory();
    if (!memory) {
      return;
    }

    if (!memory.asciiLogs) {
      return;
    }
    for (const entry of asciiLogDrafts) {
      const logMemory = memory.asciiLogs[entry.sourcePath];
      if (!logMemory) {
        continue;
      }
      if (typeof logMemory.enabled === "boolean") {
        entry.enabled = logMemory.enabled;
      }
      if (typeof logMemory.depthColumn === "string" && logMemory.depthColumn.trim().length > 0) {
        entry.depthColumn = logMemory.depthColumn;
      }
      if (typeof logMemory.nullValue === "string" && logMemory.nullValue.trim().length > 0) {
        entry.nullValue = logMemory.nullValue;
      }
      if (!logMemory.curves) {
        continue;
      }
      for (const curve of entry.curves) {
        const curveMemory = logMemory.curves[curve.sourceColumn];
        if (!curveMemory) {
          continue;
        }
        if (typeof curveMemory.enabled === "boolean") {
          curve.enabled = curveMemory.enabled;
        }
        if (typeof curveMemory.mnemonic === "string" && curveMemory.mnemonic.trim().length > 0) {
          curve.mnemonic = curveMemory.mnemonic;
        }
        if (typeof curveMemory.unit === "string") {
          curve.unit = curveMemory.unit;
        }
      }
    }
  }

  function saveImportMemory(): void {
    const storageKey = importMemoryStorageKey(projectRoot);
    if (!storageKey || typeof window === "undefined") {
      return;
    }

    const memory: WellImportMemory = {
      asciiLogs: Object.fromEntries(
        asciiLogDrafts.map((entry) => [
          entry.sourcePath,
          {
            enabled: entry.enabled,
            depthColumn: entry.depthColumn,
            nullValue: entry.nullValue,
            curves: Object.fromEntries(
              entry.curves.map((curve) => [
                curve.sourceColumn,
                {
                  enabled: curve.enabled,
                  mnemonic: curve.mnemonic,
                  unit: curve.unit
                }
              ])
            )
          }
        ])
      )
    };

    try {
      window.localStorage.setItem(storageKey, JSON.stringify(memory));
    } catch {
      // Ignore local storage failures; import confirmation still succeeds.
    }
  }
</script>

<div class="well-folder-import-backdrop" role="presentation">
  <div
    class="well-folder-import-dialog"
    role="dialog"
    aria-modal="true"
    aria-labelledby="well-folder-import-title"
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
        <h2 id="well-folder-import-title">Import Well Sources</h2>
        <p>{importSelectionLabel}</p>
      </div>
      <button class="ghost-button" type="button" onclick={() => !committing && close()}>Close</button>
    </header>

    {#if loading}
      <div class="dialog-body">
        <p>Parsing selected well sources...</p>
      </div>
    {:else if errorMessage}
      <div class="dialog-body">
        <p class="error-text">{errorMessage}</p>
        <button class="primary-button" type="button" onclick={loadPreview}>Retry</button>
      </div>
    {:else if preview}
      <div class="dialog-body">
        <section class="summary-grid">
          <div>
            <span class="summary-label">Selected Sources</span>
            <strong>{selectedSourcePaths.length > 0 ? importSelectionLabel : "Selection root only"}</strong>
          </div>
          <div>
            <span class="summary-label">Source Root</span>
            <strong>{sourceRootLabel}</strong>
          </div>
          <div>
            <span class="summary-label">Project Storage</span>
            <div class="project-root-row">
              <code>{projectRoot || "Not set"}</code>
              <button class="ghost-button" type="button" onclick={chooseProjectRoot}>Change</button>
            </div>
          </div>
          <div>
            <span class="summary-label">Source CRS</span>
            <strong>{resolvedSourceCoordinateReferenceLabel}</strong>
          </div>
          <div>
            <span class="summary-label">Logs</span>
            <strong>{preview.logs.files.length}</strong>
          </div>
          <div>
            <span class="summary-label">NLOG ASCII</span>
            <strong>{preview.asciiLogs.files.length}</strong>
          </div>
          <div>
            <span class="summary-label">Tops</span>
            <strong>{preview.topsMarkers.committableRowCount} / {preview.topsMarkers.rowCount}</strong>
          </div>
          <div>
            <span class="summary-label">Unsupported</span>
            <strong>{preview.unsupportedSources.length}</strong>
          </div>
        </section>

        <div class="detail-block">
          <strong>Project storage</strong>
          <p>
            The selected files are parsed immediately in this dialog. Final import writes the translated
            well assets into an Ophiolite project location so they persist in the workspace and can be
            reused as project wells.
          </p>
        </div>

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

        {#if stage === "configure"}
          {#if selectedSourcePaths.length > 0}
            <section class="section-block">
              <div class="detail-block">
                <strong>Selected-file import</strong>
                <p>
                  Only the selected files are parsed and considered for canonical translation. The common
                  source root is kept only as context for source preservation and related import notes.
                </p>
              </div>
            </section>
          {/if}

          <section class="section-block">
            <h3>Target Binding</h3>
            <div class="field-grid">
              <label>
                <span>Well Name</span>
                <input bind:value={draft.wellName} />
              </label>
              <label>
                <span>Wellbore Name</span>
                <input bind:value={draft.wellboreName} />
              </label>
              <label>
                <span>UWI</span>
                <input bind:value={draft.uwi} />
              </label>
              <label>
                <span>API</span>
                <input bind:value={draft.api} />
              </label>
            </div>
          </section>

          <section class="section-block">
            <h3>Source CRS</h3>
            <div class="detail-block">
              <strong>Default import assumption</strong>
              <p>
                When the active survey already has a native CRS, this import defaults to that CRS.
                You can keep it, switch to a detected or manual CRS, or leave geometry unresolved.
              </p>
            </div>
            {#if draft.sourceCrsMode === "unresolved" && detectedCandidates.length > 0}
              <div class="detail-block">
                <strong>Explicit confirmation required for geometry</strong>
                <p>
                  A likely source CRS was detected and prefilled below, but the import keeps geometry
                  unresolved until you explicitly choose detected, survey, or manual CRS handling.
                </p>
              </div>
            {/if}
            <div class="choice-stack">
              <label class="choice-row">
                <input
                  type="radio"
                  name="well-folder-source-crs"
                  checked={draft.sourceCrsMode === "detected"}
                  disabled={detectedCandidates.length === 0}
                  onchange={() => chooseSourceCrsMode("detected")}
                />
                <div>
                  <strong>Use detected CRS</strong>
                  <p>
                    {selectedDetectedCandidate
                      ? formatCoordinateReference(selectedDetectedCandidate.coordinateReference)
                      : "No detected CRS candidates"}
                  </p>
                </div>
              </label>

              {#if draft.sourceCrsMode === "detected" && detectedCandidates.length > 0}
                <label class="field">
                  <span>Detected Candidate</span>
                  <select
                    bind:value={draft.detectedCandidateId}
                    onchange={() => syncGeometrySelections(preview)}
                  >
                    {#each detectedCandidates as candidate (candidate.coordinateReference.id ?? candidate.evidence)}
                      <option value={candidate.coordinateReference.id ?? ""}>
                        {formatCoordinateReference(candidate.coordinateReference)}
                      </option>
                    {/each}
                  </select>
                </label>
              {/if}

              {#if selectedDetectedCandidate && draft.sourceCrsMode === "detected"}
                <div class="detail-block">
                  <strong>{selectedDetectedCandidate.confidence} confidence</strong>
                  <p>{selectedDetectedCandidate.evidence}</p>
                  <p>{selectedDetectedCandidate.rationale}</p>
                </div>
              {/if}

              <label class="choice-row">
                <input
                  type="radio"
                  name="well-folder-source-crs"
                  checked={draft.sourceCrsMode === "assume_same_as_survey"}
                  disabled={!activeSurveyCoordinateReferenceAvailable}
                  onchange={() => chooseSourceCrsMode("assume_same_as_survey")}
                />
                <div>
                  <strong>Use active survey CRS</strong>
                  <p>
                    {activeSurveyCoordinateReferenceAvailable
                      ? formatCoordinateReference({
                          id: activeSurveyCoordinateReferenceId,
                          name: activeSurveyCoordinateReferenceName
                        })
                      : "No active survey native CRS"}
                  </p>
                </div>
              </label>

              <label class="choice-row">
                <input
                  type="radio"
                  name="well-folder-source-crs"
                  checked={draft.sourceCrsMode === "manual"}
                  onchange={() => chooseSourceCrsMode("manual")}
                />
                <div>
                  <strong>Choose CRS</strong>
                  <p>Use this when the selected source CRS is known but not detected correctly.</p>
                </div>
              </label>

              {#if draft.sourceCrsMode === "manual"}
                <div class="detail-block">
                  <strong>
                    {formatCoordinateReference({
                      id: blankToNull(draft.manualSourceCrsId),
                      name: blankToNull(draft.manualSourceCrsName)
                    }) || "Choose a validated CRS or local engineering label"}
                  </strong>
                  <p>
                    Use the registry-backed picker for validated CRSs, or choose local engineering
                    coordinates when the file does not carry an authority id.
                  </p>
                  <div class="detail-actions">
                    <button type="button" class="ghost-button" onclick={openManualSourceCrsPicker}>
                      Choose Source CRS
                    </button>
                    {#if draft.manualSourceCrsId || draft.manualSourceCrsName}
                      <button
                        type="button"
                        class="ghost-button"
                        onclick={clearManualSourceCrsSelection}
                      >
                        Clear
                      </button>
                    {/if}
                  </div>
                </div>
              {/if}

              <label class="choice-row">
                <input
                  type="radio"
                  name="well-folder-source-crs"
                  checked={draft.sourceCrsMode === "unresolved"}
                  onchange={() => chooseSourceCrsMode("unresolved")}
                />
                <div>
                  <strong>Leave unresolved</strong>
                  <p>Metadata and logs can still be committed. Surface geometry and trajectory stay out.</p>
                </div>
              </label>
            </div>

            <div class="detail-block">
              <strong>Commit impact</strong>
              <p>Surface location: {surfaceLocationWillBeCommitted ? "committed" : "omitted until CRS is confirmed"}</p>
              <p>
                Trajectory:
                {!geometryCommitAllowed
                  ? "disabled until CRS is confirmed"
                  : trajectoryReadyForCommit
                    ? "available for import"
                    : "waiting on supplemented stations"}
              </p>
            </div>

            {#if preview.sourceCoordinateReference.notes.length}
              <ul class="issue-list">
                {#each preview.sourceCoordinateReference.notes as note, index (`crs-note:${index}`)}
                  <li>
                    <strong>note</strong>
                    <span>{note}</span>
                  </li>
                {/each}
              </ul>
            {/if}
          </section>

          <section class="section-block">
            <h3>Metadata</h3>
            <div class="field-grid">
              <label>
                <span>Field</span>
                <input bind:value={draft.fieldName} />
              </label>
              <label>
                <span>Block</span>
                <input bind:value={draft.blockName} />
              </label>
              <label>
                <span>Country</span>
                <input bind:value={draft.country} />
              </label>
              <label>
                <span>Province</span>
                <input bind:value={draft.provinceState} />
              </label>
              <label>
                <span>Location</span>
                <input bind:value={draft.locationText} />
              </label>
              <label>
                <span>Interest Type</span>
                <input bind:value={draft.interestType} />
              </label>
              <label>
                <span>Surface X</span>
                <input bind:value={draft.surfaceX} />
              </label>
              <label>
                <span>Surface Y</span>
                <input bind:value={draft.surfaceY} />
              </label>
              <label>
                <span>Wellbore Status</span>
                <input bind:value={draft.wellboreStatus} />
              </label>
              <label>
                <span>Wellbore Purpose</span>
                <input bind:value={draft.wellborePurpose} />
              </label>
              <label>
                <span>Trajectory Type</span>
                <input bind:value={draft.trajectoryType} />
              </label>
              <label>
                <span>Parent Wellbore</span>
                <input bind:value={draft.parentWellboreId} />
              </label>
              <label>
                <span>Service Company</span>
                <input bind:value={draft.serviceCompanyName} />
              </label>
              <label>
                <span>Wellbore Location</span>
                <input bind:value={draft.wellboreLocationText} />
              </label>
            </div>
          </section>

          <section class="section-block">
            <h3>Asset Families</h3>
            <div class="toggle-grid">
              <label class="toggle-row">
                <input type="checkbox" bind:checked={draft.importLogs} disabled={!logsImportAvailable} />
                <span>Logs</span>
                <small>
                  {selectedLogFiles.length + selectedAsciiLogImports.length} selected of {preview.logs.files.length + preview.asciiLogs.files.length}
                </small>
              </label>
              <label class="toggle-row">
                <input
                  type="checkbox"
                  bind:checked={draft.importTopsMarkers}
                  disabled={!preview.topsMarkers.commitEnabled}
                />
                <span>Tops / Markers</span>
                <small>{committableEditedTopRows.length} committable rows</small>
              </label>
              <label class="toggle-row">
                <input
                  type="checkbox"
                  bind:checked={draft.importTrajectory}
                  disabled={!trajectorySourcePresent || !geometryCommitAllowed}
                />
                <span>Trajectory</span>
                <small>
                  {!trajectorySourcePresent
                    ? "not detected"
                    : !geometryCommitAllowed
                      ? "requires confirmed CRS"
                      : trajectoryReadyForCommit
                        ? `${trajectoryCommittedStationCount} stations ready`
                        : "needs supplemented stations"}
                </small>
              </label>
              <label>
                <span>Tops Depth Reference</span>
                <input bind:value={draft.topsDepthReference} />
              </label>
            </div>

            {#if preview.logs.files.length > 0}
              <div class="detail-block">
                <strong>LAS selection</strong>
                <p>Keep the best files checked. Duplicates can stay as preserved source-only files.</p>
              </div>
              <ul class="source-list">
                {#each preview.logs.files as file (file.sourcePath)}
                  <li>
                    <label class="toggle-row">
                      <input
                        type="checkbox"
                        checked={selectedLogSourcePaths.includes(file.sourcePath)}
                        disabled={!draft.importLogs}
                        onchange={(event) => {
                          if ((event.currentTarget as HTMLInputElement).checked) {
                            selectedLogSourcePaths = [...selectedLogSourcePaths, file.sourcePath];
                          } else {
                            selectedLogSourcePaths = selectedLogSourcePaths.filter(
                              (value) => value !== file.sourcePath
                            );
                          }
                        }}
                      />
                      <span>{file.fileName}</span>
                      <small>{file.curveCount} curves, {file.rowCount} rows</small>
                    </label>
                    {#if file.selectionReason}
                      <span>{file.selectionReason}</span>
                    {/if}
                  </li>
                {/each}
              </ul>
            {/if}

            {#if preview.asciiLogs.files.length > 0}
              <div class="detail-block">
                <strong>NLOG ASCII mapping</strong>
                <p>These tables can import even when some curves or fields stay unresolved. Unchecked tables remain preserved as source.</p>
              </div>
              {#each preview.asciiLogs.files as file (file.sourcePath)}
                {@const draftEntry = findAsciiLogDraft(file.sourcePath)}
                {#if draftEntry}
                  <div class="detail-block">
                    <label class="toggle-row">
                      <input
                        type="checkbox"
                        bind:checked={draftEntry.enabled}
                        disabled={!draft.importLogs}
                      />
                      <span>{file.fileName}</span>
                      <small>{file.rowCount} rows, {file.columnCount} columns</small>
                    </label>
                    <div class="field-grid">
                      <label>
                        <span>Depth Column</span>
                        <select bind:value={draftEntry.depthColumn} disabled={!draft.importLogs || !draftEntry.enabled}>
                          {#each file.columns as column (column.name)}
                            <option value={column.name}>{column.name}</option>
                          {/each}
                        </select>
                      </label>
                      <label>
                        <span>Null Value</span>
                        <input
                          bind:value={draftEntry.nullValue}
                          disabled={!draft.importLogs || !draftEntry.enabled}
                          placeholder="-999.25"
                        />
                      </label>
                    </div>
                    <div class="tops-grid">
                      <div class="tops-grid-header">Use</div>
                      <div class="tops-grid-header">Source Column</div>
                      <div class="tops-grid-header">Mnemonic</div>
                      <div class="tops-grid-header">Sample</div>
                      {#each draftEntry.curves as curve (curve.sourceColumn)}
                        <input
                          type="checkbox"
                          checked={curve.enabled}
                          disabled={!draft.importLogs || !draftEntry.enabled || curve.sourceColumn === draftEntry.depthColumn}
                          onchange={(event) =>
                            toggleAsciiCurve(
                              file.sourcePath,
                              curve.sourceColumn,
                              (event.currentTarget as HTMLInputElement).checked
                            )}
                        />
                        <div class="tops-row-reference">{curve.sourceColumn}</div>
                        <input
                          bind:value={curve.mnemonic}
                          disabled={!draft.importLogs || !draftEntry.enabled || !curve.enabled}
                        />
                        <div class="tops-row-reference">
                          {file.columns.find((column) => column.name === curve.sourceColumn)?.sampleValues
                            ?.slice(0, 2)
                            .join(", ") || "No sample"}
                        </div>
                      {/each}
                    </div>
                  </div>
                {/if}
              {/each}
            {/if}

            {#if preview.trajectory.sourcePath}
              <div class="detail-block">
                <strong>Trajectory translation</strong>
                <p>
                  Review the parsed stations below before import. You can correct, supplement, or trim rows here; the confirmed draft is what gets committed.
                </p>
                <p>
                  Parsed columns:
                  MD {preview.trajectory.nonEmptyColumnCount.measured_depth ?? 0},
                  Inc {preview.trajectory.nonEmptyColumnCount.inclination_deg ?? 0},
                  Azi {preview.trajectory.nonEmptyColumnCount.azimuth_deg ?? 0},
                  TVD {preview.trajectory.nonEmptyColumnCount.true_vertical_depth ?? 0},
                  X {preview.trajectory.nonEmptyColumnCount.x_offset ?? 0},
                  Y {preview.trajectory.nonEmptyColumnCount.y_offset ?? 0}
                </p>
                {#if trajectoryNeedsSupplement}
                  <p>
                    Draft coverage:
                    MD {trajectoryDraftSupport.measuredDepthCount},
                    Inc {trajectoryDraftSupport.inclinationDegCount},
                    Azi {trajectoryDraftSupport.azimuthDegCount},
                    TVD {trajectoryDraftSupport.trueVerticalDepthCount},
                    X {trajectoryDraftSupport.xOffsetCount},
                    Y {trajectoryDraftSupport.yOffsetCount}
                  </p>
                {/if}
              </div>
              <div class="tops-grid trajectory-grid">
                <div class="tops-grid-header">MD</div>
                <div class="tops-grid-header">Inc</div>
                <div class="tops-grid-header">Azi</div>
                <div class="tops-grid-header">TVD</div>
                <div class="tops-grid-header">X</div>
                <div class="tops-grid-header">Y</div>
                <div class="tops-grid-header">Row</div>
                {#each trajectoryRows as row, index (`trajectory-row:${index}`)}
                  <input bind:value={row.measuredDepth} inputmode="decimal" placeholder="MD" />
                  <input bind:value={row.inclinationDeg} inputmode="decimal" placeholder="Inc" />
                  <input bind:value={row.azimuthDeg} inputmode="decimal" placeholder="Azi" />
                  <input bind:value={row.trueVerticalDepth} inputmode="decimal" placeholder="TVD" />
                  <input bind:value={row.xOffset} inputmode="decimal" placeholder="X" />
                  <input bind:value={row.yOffset} inputmode="decimal" placeholder="Y" />
                  <button
                    type="button"
                    class="secondary-button"
                    onclick={() => removeTrajectoryRow(index)}
                    disabled={trajectoryRows.length <= 2}
                  >
                    Remove
                  </button>
                {/each}
              </div>
              <div class="dialog-actions">
                <button type="button" class="secondary-button" onclick={addTrajectoryRow}>
                  Add Station
                </button>
              </div>
            {/if}
          </section>

          {#if preview.topsMarkers.rows.length > 0}
            <section class="section-block">
              <h3>Tops Translation</h3>
              <div class="detail-block">
                <strong>Editable canonical tops draft</strong>
                <p>
                  Adjust names and depths before import. Rows without both a name and a top depth stay out of the canonical tops asset.
                </p>
              </div>

              <div class="tops-grid">
                <div class="tops-grid-header">Name</div>
                <div class="tops-grid-header">Top Depth</div>
                <div class="tops-grid-header">Base Depth</div>
                <div class="tops-grid-header">Reference</div>
                {#each topsRows as row, index (`top-row:${index}`)}
                  <input bind:value={row.name} placeholder="Formation name" />
                  <input bind:value={row.topDepth} inputmode="decimal" placeholder="Top depth" />
                  <input bind:value={row.baseDepth} inputmode="decimal" placeholder="Base depth" />
                  <div class="tops-row-reference">
                    {topRowReferenceLabel(row) || "No parser note"}
                  </div>
                {/each}
              </div>
            </section>
          {/if}
        {:else if stage === "review"}
          <section class="section-block">
            <h3>Review Checklist</h3>
            <ImportReviewChecklist items={reviewItems} />
          </section>

          <section class="section-block">
            <h3>Commit Preview</h3>
            <ul class="source-list">
              <li>
                <strong>Expected project assets</strong>
                <span>{projectedImportedAssetCount}</span>
              </li>
              <li>
                <strong>Asset kinds</strong>
                <span>{projectedImportedAssetKinds.length > 0 ? projectedImportedAssetKinds.join(", ") : "metadata only"}</span>
              </li>
              <li>
                <strong>Logs</strong>
                <span>
                  {selectedLogFiles.length + selectedAsciiLogImports.length > 0
                    ? `${selectedLogFiles.length + selectedAsciiLogImports.length} log asset${selectedLogFiles.length + selectedAsciiLogImports.length === 1 ? "" : "s"}`
                    : "omitted"}
                </span>
              </li>
              <li>
                <strong>Tops / Markers</strong>
                <span>
                  {draft.importTopsMarkers && preview.topsMarkers.commitEnabled
                    ? `${committableEditedTopRows.length} rows`
                    : "omitted"}
                </span>
              </li>
              <li>
                <strong>Trajectory</strong>
                <span>
                  {draft.importTrajectory && trajectoryCommitAllowed
                    ? `${trajectoryCommittedStationCount} stations`
                    : draft.importTrajectory && !geometryCommitAllowed
                      ? "omitted until CRS is confirmed"
                      : draft.importTrajectory
                        ? "omitted until stations are supplemented"
                      : "omitted"}
                </span>
              </li>
              <li>
                <strong>Surface location</strong>
                <span>{surfaceLocationWillBeCommitted ? "committed" : "omitted"}</span>
              </li>
              {#if unsupportedPreservationMode}
                <li>
                  <strong>Unsupported sources</strong>
                  <span>
                    {unsupportedPreservationMode === "source_artifacts"
                      ? "preserved on the imported canonical assets"
                      : "preserved as a raw source bundle"}
                  </span>
                </li>
              {/if}
            </ul>

            <div class="detail-block">
              <strong>Canonical draft</strong>
              <p>
                This is the bounded canonical translation that will be committed after confirmation.
                Use Back To Edit if any field, mapping, or import choice still needs adjustment.
              </p>
            </div>

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

            {#if reviewLasLogRows.length > 0}
              <div class="detail-block">
                <strong>Canonical LAS Logs</strong>
                <p>
                  These LAS files will be committed as canonical log assets with their parsed curve sets.
                  {#if reviewLasLogRowsHiddenCount > 0}
                    Showing the first {reviewLasLogRows.length} of {selectedLogFiles.length}.
                  {/if}
                </p>
              </div>
              <div class="review-row-grid review-las-grid">
                <div class="tops-grid-header">File</div>
                <div class="tops-grid-header">Index</div>
                <div class="tops-grid-header">Curves</div>
                <div class="tops-grid-header">Rows</div>
                {#each reviewLasLogRows as file (file.sourcePath)}
                  <div class="review-row-value">{file.fileName}</div>
                  <div class="review-row-value">{file.indexCurveName || "—"}</div>
                  <div class="review-row-value">
                    {file.curveCount} curves: {formatCurvePreview(file.curveNames)}
                  </div>
                  <div class="review-row-value">{file.rowCount}</div>
                {/each}
              </div>
            {/if}

            {#if reviewAsciiLogMappingPreviewRows.length > 0}
              <div class="detail-block">
                <strong>Canonical ASCII Log Mappings</strong>
                <p>
                  These NLOG column mappings will be committed as canonical log curves.
                  {#if reviewAsciiLogMappingHiddenCount > 0}
                    Showing the first {reviewAsciiLogMappingPreviewRows.length} of {reviewAsciiLogMappingRows.length}.
                  {/if}
                </p>
              </div>
              <div class="review-row-grid review-ascii-grid">
                <div class="tops-grid-header">File</div>
                <div class="tops-grid-header">Depth</div>
                <div class="tops-grid-header">Null</div>
                <div class="tops-grid-header">Source Column</div>
                <div class="tops-grid-header">Mnemonic</div>
                <div class="tops-grid-header">Unit</div>
                {#each reviewAsciiLogMappingPreviewRows as row, index (`review-ascii-row:${row.fileName}:${row.sourceColumn}:${index}`)}
                  <div class="review-row-value">{row.fileName}</div>
                  <div class="review-row-value">{row.depthColumn}</div>
                  <div class="review-row-value">{row.nullValue}</div>
                  <div class="review-row-value">{row.sourceColumn}</div>
                  <div class="review-row-value">{row.mnemonic}</div>
                  <div class="review-row-value">{row.unit}</div>
                {/each}
              </div>
            {/if}

            {#if reviewTopRows.length > 0}
              <div class="detail-block">
                <strong>Canonical Top Rows</strong>
                <p>
                  These rows will be committed to the canonical top set.
                  {#if reviewTopRowsHiddenCount > 0}
                    Showing the first {reviewTopRows.length} of {committableEditedTopRows.length}.
                  {/if}
                </p>
              </div>
              <div class="review-row-grid review-top-grid">
                <div class="tops-grid-header">Name</div>
                <div class="tops-grid-header">Top Depth</div>
                <div class="tops-grid-header">Base Depth</div>
                {#each reviewTopRows as row, index (`review-top-row:${index}`)}
                  <div class="review-row-value">{row.name ?? "—"}</div>
                  <div class="review-row-value">{formatReviewNumber(row.topDepth)}</div>
                  <div class="review-row-value">{formatReviewNumber(row.baseDepth)}</div>
                {/each}
              </div>
            {/if}

            {#if reviewTrajectoryRows.length > 0}
              <div class="detail-block">
                <strong>Canonical Trajectory Rows</strong>
                <p>
                  These rows will be committed to the canonical trajectory asset.
                  {#if reviewTrajectoryRowsHiddenCount > 0}
                    Showing the first {reviewTrajectoryRows.length} of {editedTrajectoryRowsForRequest.length}.
                  {/if}
                </p>
              </div>
              <div class="review-row-grid review-trajectory-grid">
                <div class="tops-grid-header">MD</div>
                <div class="tops-grid-header">Inc</div>
                <div class="tops-grid-header">Azi</div>
                <div class="tops-grid-header">TVD</div>
                <div class="tops-grid-header">X</div>
                <div class="tops-grid-header">Y</div>
                {#each reviewTrajectoryRows as row, index (`review-trajectory-row:${index}`)}
                  <div class="review-row-value">{formatReviewNumber(row.measuredDepth)}</div>
                  <div class="review-row-value">{formatReviewNumber(row.inclinationDeg)}</div>
                  <div class="review-row-value">{formatReviewNumber(row.azimuthDeg)}</div>
                  <div class="review-row-value">{formatReviewNumber(row.trueVerticalDepth)}</div>
                  <div class="review-row-value">{formatReviewNumber(row.xOffset)}</div>
                  <div class="review-row-value">{formatReviewNumber(row.yOffset)}</div>
                {/each}
              </div>
            {/if}

            <label class="toggle-row raw-payload-toggle">
              <input type="checkbox" bind:checked={showRawCanonicalPayload} />
              <span>Show raw payload</span>
            </label>
            {#if showRawCanonicalPayload}
              <pre class="json-preview">{canonicalPreviewText}</pre>
            {/if}
          </section>

          <section class="section-block">
            <h3>Detected Sources</h3>
            <ul class="source-list">
              {#each preview.logs.files as file (file.sourcePath)}
                <li>
                  <strong>{file.fileName}</strong>
                  <span>{file.curveCount} curves, {file.rowCount} rows</span>
                </li>
              {/each}
              {#each preview.asciiLogs.files as file (file.sourcePath)}
                <li class={selectedAsciiLogFiles.some((candidate) => candidate.sourcePath === file.sourcePath) ? "" : "muted-item"}>
                  <strong>{file.fileName}</strong>
                  <span>
                    {file.rowCount} rows, {file.columnCount} columns
                    {#if selectedAsciiLogFiles.some((candidate) => candidate.sourcePath === file.sourcePath)}
                      mapped to canonical log curves
                    {:else}
                      preserved as source until mapped
                    {/if}
                  </span>
                </li>
              {/each}
              {#if preview.topsMarkers.sourcePath}
                <li>
                  <strong>{preview.topsMarkers.sourceName ?? "lithostratigrafie.txt"}</strong>
                  <span>{preview.topsMarkers.committableRowCount} committable tops</span>
                </li>
              {/if}
              {#if preview.trajectory.sourcePath}
                <li>
                  <strong>deviatie.txt</strong>
                  <span>{preview.trajectory.rowCount} rows</span>
                </li>
              {/if}
              {#each preview.unsupportedSources as source (source.sourcePath)}
                <li class="muted-item">
                  <strong>{source.fileName}</strong>
                  <span>
                    {source.reason}
                    {#if unsupportedPreservationMode === "source_artifacts"}
                      Preserved on the imported canonical assets.
                    {:else if unsupportedPreservationMode === "raw_source_bundle"}
                      Preserved as a raw source bundle.
                    {/if}
                  </span>
                </li>
              {/each}
            </ul>
          </section>

          {#if preview.issues.length}
            <section class="section-block">
              <h3>Issues</h3>
              <ul class="issue-list">
                {#each preview.issues as issue, index (`${issue.code}:${index}`)}
                  <li>
                    <strong>{issue.severity}</strong>
                    <span>
                      {issue.message}
                      {#if issue.sourcePath}
                        ({issue.sourcePath})
                      {/if}
                    </span>
                  </li>
                {/each}
              </ul>
            </section>
          {/if}
        {:else if commitResponse}
          <section class="section-block">
            <h3>Import Result</h3>
            <ul class="source-list">
              <li>
                <strong>Well</strong>
                <span>{commitResponse.wellId}</span>
              </li>
              <li>
                <strong>Wellbore</strong>
                <span>{commitResponse.wellboreId}</span>
              </li>
              <li>
                <strong>Resolved source CRS</strong>
                <span>{formatCoordinateReference(commitResponse.sourceCoordinateReference)}</span>
              </li>
              <li>
                <strong>Imported assets</strong>
                <span>
                  {commitResponse.importedAssets.length > 0
                    ? `${commitResponse.importedAssets.length} (${resultImportedAssetKinds.join(", ")})`
                    : "0 canonical assets"}
                </span>
              </li>
              <li>
                <strong>Omissions</strong>
                <span>{commitResponse.omissions.length}</span>
              </li>
            </ul>
          </section>

          <section class="section-block">
            <h3>Imported Assets</h3>
            {#if commitResponse.importedAssets.length > 0}
              <ul class="source-list">
                {#each commitResponse.importedAssets as asset, index (`result-asset:${asset.assetId}:${index}`)}
                  <li>
                    <strong>{asset.assetKind}</strong>
                    <span>{asset.collectionName}</span>
                  </li>
                {/each}
              </ul>
            {:else}
              <div class="detail-block">
                <strong>No canonical assets were imported.</strong>
                <p>The confirmed update only changed well or wellbore metadata, or preserved sources without translating them into canonical assets.</p>
              </div>
            {/if}
          </section>

          <section class="section-block">
            <h3>Omissions</h3>
            {#if commitResponse.omissions.length > 0}
              <ul class="issue-list">
                {#each commitResponse.omissions as omission, index (`result-omission:${omission.reasonCode}:${index}`)}
                  <li>
                    <strong>{omission.reasonCode}</strong>
                    <span>
                      {omission.message}
                      {#if omission.rowCount !== null && omission.rowCount !== undefined}
                        ({omission.rowCount} row{omission.rowCount === 1 ? "" : "s"})
                      {/if}
                      {#if omission.sourcePath}
                        [{omission.sourcePath}]
                      {/if}
                    </span>
                  </li>
                {/each}
              </ul>
            {:else}
              <div class="detail-block">
                <strong>No omissions were reported.</strong>
                <p>Every selected slice in the confirmed plan was committed as expected.</p>
              </div>
            {/if}
          </section>

          {#if commitResponse.issues.length > 0}
            <section class="section-block">
              <h3>Commit Issues</h3>
              <ul class="issue-list">
                {#each commitResponse.issues as issue, index (`result-issue:${issue.code}:${index}`)}
                  <li>
                    <strong>{issue.severity}</strong>
                    <span>
                      {issue.message}
                      {#if issue.sourcePath}
                        ({issue.sourcePath})
                      {/if}
                    </span>
                  </li>
                {/each}
              </ul>
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
          <button class="primary-button" type="button" onclick={goToReview} disabled={committing}>
            Review Canonical Draft
          </button>
        {/if}
      </footer>
      {#if stage === "review" && confirmDisabledReason}
        <div class="footer-note">{confirmDisabledReason}</div>
      {/if}
    {/if}
  </div>
</div>

{#if manualSourceCrsPickerOpen}
  <CoordinateReferencePicker
    close={closeManualSourceCrsPicker}
    confirm={handleManualSourceCrsSelection}
    title="Well Source CRS"
    description="Choose the CRS used by the imported well geometry, or record it as local engineering coordinates."
    allowLocalEngineering={true}
    localEngineeringLabel="Local engineering coordinates"
    selectedAuthId={draft.manualSourceCrsId}
    projectRoot={viewerModel.projectRoot}
    projectedOnly={false}
    includeGeographic={true}
    includeVertical={false}
  />
{/if}

<style>
  .well-folder-import-backdrop {
    position: fixed;
    inset: 0;
    z-index: 80;
    display: flex;
    align-items: center;
    justify-content: center;
    padding: 20px;
    background: rgb(0 0 0 / 0.45);
  }

  .well-folder-import-dialog {
    width: min(960px, 100%);
    max-height: calc(100vh - 40px);
    display: grid;
    grid-template-rows: auto 1fr auto;
    background: #12161d;
    border: 1px solid rgb(255 255 255 / 0.12);
    border-radius: 8px;
    overflow: hidden;
  }

  .dialog-header,
  .dialog-footer {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 16px;
    padding: 16px 20px;
    border-bottom: 1px solid rgb(255 255 255 / 0.08);
  }

  .dialog-footer {
    border-top: 1px solid rgb(255 255 255 / 0.08);
    border-bottom: none;
    justify-content: flex-end;
  }

  .dialog-header h2,
  .section-block h3 {
    margin: 0;
  }

  .dialog-header p {
    margin: 4px 0 0;
    color: rgb(255 255 255 / 0.72);
    word-break: break-all;
  }

  .dialog-body {
    overflow: auto;
    padding: 20px;
    display: grid;
    gap: 20px;
  }

  .summary-grid,
  .field-grid,
  .toggle-grid,
  .choice-stack,
  .tops-grid,
  .review-grid,
  .review-row-grid {
    display: grid;
    gap: 12px;
  }

  .summary-grid {
    grid-template-columns: repeat(auto-fit, minmax(180px, 1fr));
  }

  .field-grid {
    grid-template-columns: repeat(auto-fit, minmax(220px, 1fr));
  }

  .toggle-grid {
    grid-template-columns: repeat(auto-fit, minmax(220px, 1fr));
  }

  .tops-grid {
    grid-template-columns: minmax(180px, 2fr) minmax(120px, 1fr) minmax(120px, 1fr) minmax(220px, 2fr);
    align-items: center;
  }

  .review-grid {
    grid-template-columns: repeat(auto-fit, minmax(220px, 1fr));
  }

  .review-top-grid {
    grid-template-columns: minmax(180px, 2fr) minmax(120px, 1fr) minmax(120px, 1fr);
  }

  .review-las-grid {
    grid-template-columns:
      minmax(180px, 2fr)
      minmax(120px, 1fr)
      minmax(280px, 3fr)
      minmax(96px, 1fr);
  }

  .review-ascii-grid {
    grid-template-columns:
      minmax(180px, 2fr)
      minmax(120px, 1fr)
      minmax(92px, 1fr)
      minmax(140px, 1.2fr)
      minmax(140px, 1.2fr)
      minmax(92px, 0.8fr);
  }

  .review-trajectory-grid {
    grid-template-columns: repeat(6, minmax(92px, 1fr));
  }

  .section-block {
    display: grid;
    gap: 12px;
    padding-top: 16px;
    border-top: 1px solid rgb(255 255 255 / 0.08);
  }

  .summary-label,
  label span,
  .tops-grid-header {
    display: block;
    margin-bottom: 6px;
    color: rgb(255 255 255 / 0.72);
    font-size: 0.9rem;
  }

  .tops-grid-header {
    margin-bottom: 0;
    font-weight: 600;
  }

  label {
    display: grid;
  }

  input,
  select {
    width: 100%;
    min-width: 0;
    padding: 10px 12px;
    border: 1px solid rgb(255 255 255 / 0.12);
    border-radius: 6px;
    background: #0d1117;
    color: #f5f7fa;
    font: inherit;
  }

  .project-root-row,
  .toggle-row,
  .source-list li,
  .issue-list li,
  .choice-row {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 12px;
  }

  .project-root-row {
    align-items: flex-start;
  }

  .project-root-row code {
    flex: 1 1 auto;
    min-width: 0;
    white-space: pre-wrap;
    word-break: break-all;
  }

  .toggle-row,
  .choice-row {
    justify-content: flex-start;
  }

  .choice-row {
    padding: 10px 12px;
    border: 1px solid rgb(255 255 255 / 0.08);
    border-radius: 6px;
    background: rgb(255 255 255 / 0.02);
  }

  .choice-row div {
    min-width: 0;
  }

  .choice-row strong,
  .detail-block strong {
    display: block;
    margin: 0;
  }

  .choice-row p,
  .detail-block p {
    margin: 4px 0 0;
    color: rgb(255 255 255 / 0.72);
  }

  .detail-block {
    padding: 10px 12px;
    border: 1px solid rgb(255 255 255 / 0.08);
    border-radius: 6px;
    background: rgb(255 255 255 / 0.02);
  }

  .detail-actions {
    display: flex;
    flex-wrap: wrap;
    gap: 10px;
    margin-top: 12px;
  }

  .toggle-row small,
  .source-list span,
  .issue-list span,
  .muted-item span,
  .tops-row-reference {
    color: rgb(255 255 255 / 0.72);
  }

  .tops-row-reference {
    min-width: 0;
    word-break: break-word;
  }

  .review-row-value {
    min-width: 0;
    padding: 10px 12px;
    border: 1px solid rgb(255 255 255 / 0.08);
    border-radius: 6px;
    background: rgb(255 255 255 / 0.02);
    color: rgb(255 255 255 / 0.88);
    word-break: break-word;
  }

  .source-list,
  .issue-list {
    list-style: none;
    margin: 0;
    padding: 0;
    display: grid;
    gap: 10px;
  }

  .json-preview {
    margin: 0;
    padding: 12px;
    overflow: auto;
    border: 1px solid rgb(255 255 255 / 0.08);
    border-radius: 6px;
    background: #0d1117;
    color: #d8e1ea;
    font: 0.85rem/1.5 ui-monospace, SFMono-Regular, SFMono-Regular, Consolas, "Liberation Mono",
      Menlo, monospace;
    white-space: pre-wrap;
    word-break: break-word;
  }

  .raw-payload-toggle {
    padding: 10px 12px;
    border: 1px solid rgb(255 255 255 / 0.08);
    border-radius: 6px;
    background: rgb(255 255 255 / 0.02);
  }

  .ghost-button,
  .secondary-button,
  .primary-button {
    padding: 10px 14px;
    border-radius: 6px;
    border: 1px solid rgb(255 255 255 / 0.12);
    background: transparent;
    color: #f5f7fa;
  }

  .dialog-actions {
    display: flex;
    justify-content: flex-start;
  }

  .trajectory-grid {
    grid-template-columns:
      minmax(92px, 1fr)
      minmax(92px, 1fr)
      minmax(92px, 1fr)
      minmax(92px, 1fr)
      minmax(92px, 1fr)
      minmax(92px, 1fr)
      auto;
  }

  .footer-note {
    padding: 0 20px 16px;
    color: rgb(255 255 255 / 0.72);
    font-size: 0.9rem;
  }

  @media (max-width: 900px) {
    .tops-grid {
      grid-template-columns: 1fr;
    }

    .review-las-grid,
    .review-ascii-grid,
    .review-top-grid,
    .review-trajectory-grid {
      grid-template-columns: 1fr;
    }

    .tops-grid-header {
      display: none;
    }
  }

  .primary-button {
    background: #1b8f52;
    border-color: #1b8f52;
  }

  .error-text {
    color: #ff8f8f;
  }

  .muted-item {
    opacity: 0.82;
  }

</style>
