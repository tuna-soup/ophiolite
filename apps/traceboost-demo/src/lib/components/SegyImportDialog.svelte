<svelte:options runes={true} />

<script lang="ts">
  import { emitFrontendDiagnosticsEvent, type CoordinateReferenceSelection } from "../bridge";
  import {
    deleteSegyImportRecipe,
    listSegyImportRecipes,
    saveSegyImportRecipe,
    scanSegyImport,
    validateSegyImportPlan
  } from "../bridge";
  import CoordinateReferencePicker from "./CoordinateReferencePicker.svelte";
  import { pickOutputStorePath } from "../file-dialog";
  import type { ImportManagerNormalizedResult } from "../import-manager-types";
  import type { ViewerModel } from "../viewer-model.svelte";
  import type {
    SegyHeaderField,
    SegyHeaderValueType,
    SegyImportCandidatePlan,
    SegyImportIssue,
    SegyImportPlan,
    SegyImportRecipe,
    SegyImportRiskSummary,
    SegyImportScanResponse,
    SegyImportValidationResponse
  } from "@traceboost/seis-contracts";

  interface Props {
    open: boolean;
    inputPath: string | null;
    viewerModel: ViewerModel;
    onClose: () => void;
    embedded?: boolean;
    onCommitResult?: ((result: ImportManagerNormalizedResult) => void) | undefined;
  }

  type GeometryFieldTarget = "inline_3d" | "crossline_3d" | "third_axis";
  type SpatialFieldTarget = "x_field" | "y_field" | "coordinate_scalar_field";

  let { open, inputPath, viewerModel, onClose, embedded = false, onCommitResult }: Props = $props();

  let scanResponse = $state<SegyImportScanResponse | null>(null);
  let plan = $state.raw<SegyImportPlan | null>(null);
  let validation = $state<SegyImportValidationResponse | null>(null);
  let recipes = $state.raw<SegyImportRecipe[]>([]);
  let dialogError = $state<string | null>(null);
  let scanLoading = $state(false);
  let validating = $state(false);
  let savingRecipe = $state(false);
  let importing = $state(false);
  let recipeName = $state("");
  let showAdvancedSpatial = $state(false);
  let showInspection = $state(false);
  let showRecipes = $state(false);
  let coordinateReferencePickerOpen = $state(false);
  let lastLoadedInputPath = "";
  let lastValidatedPlanSignature = "";
  let validationTimer: ReturnType<typeof setTimeout> | null = null;

  const currentIssues = $derived(validation?.issues ?? scanResponse?.issues ?? []);
  const blockingIssues = $derived(currentIssues.filter((issue) => issue.severity === "blocking"));
  const warningIssues = $derived(currentIssues.filter((issue) => issue.severity === "warning"));
  const infoIssues = $derived(currentIssues.filter((issue) => issue.severity === "info"));
  const validationStale = $derived.by(() => {
    if (!plan || !lastValidatedPlanSignature) {
      return false;
    }
    return planSignature(plan) !== lastValidatedPlanSignature;
  });
  const canImport = $derived(
    !!validation &&
      validation.can_import &&
      !validationStale &&
      !validating &&
      !importing &&
      !viewerModel.loading
  );
  const sourceRecipes = $derived(
    recipes.filter((recipe) => recipe.scope === "source_fingerprint")
  );
  const globalRecipes = $derived(recipes.filter((recipe) => recipe.scope === "global"));
  const displayedRecipes = $derived([...sourceRecipes, ...globalRecipes]);
  const resolvedRiskSummary = $derived(
    (validation?.risk_summary ?? scanResponse?.risk_summary ?? null) as SegyImportRiskSummary | null
  );
  const showThirdAxis = $derived.by(() => {
    if (plan?.header_mapping.third_axis) {
      return true;
    }
    if ((validation?.resolved_dataset.third_axis_count ?? 0) > 1) {
      return true;
    }
    return (
      scanResponse?.field_observations.some((field) =>
        field.label.toLowerCase().includes("third axis")
      ) ?? false
    );
  });

  $effect(() => {
    if (!open) {
      clearValidationTimer();
      return;
    }
    const normalizedPath = inputPath?.trim() ?? "";
    if (!normalizedPath || normalizedPath === lastLoadedInputPath) {
      return;
    }
    lastLoadedInputPath = normalizedPath;
    void loadScan(normalizedPath);
  });

  function clearValidationTimer(): void {
    if (validationTimer) {
      clearTimeout(validationTimer);
      validationTimer = null;
    }
  }

  function resetState(): void {
    clearValidationTimer();
    scanResponse = null;
    plan = null;
    validation = null;
    recipes = [];
    dialogError = null;
    recipeName = "";
    showAdvancedSpatial = false;
    showInspection = false;
    showRecipes = false;
    coordinateReferencePickerOpen = false;
    lastValidatedPlanSignature = "";
  }

  function logSegyDiagnostics(
    level: "debug" | "info" | "warn" | "error",
    message: string,
    fields: Record<string, unknown> | null = null
  ): void {
    void emitFrontendDiagnosticsEvent({
      stage: "segy_import_dialog",
      level,
      message,
      fields
    }).catch(() => {});
  }

  function normalizeIssue(issue: SegyImportIssue): SegyImportIssue {
    return {
      ...issue,
      field_path: issue.field_path ?? null,
      source_path: issue.source_path ?? null,
      suggested_fix: issue.suggested_fix ?? null
    };
  }

  function normalizeCandidate(candidate: SegyImportCandidatePlan): SegyImportCandidatePlan {
    return {
      ...candidate,
      issues: Array.isArray(candidate.issues) ? candidate.issues.map(normalizeIssue) : []
    };
  }

  function normalizeScanResponse(response: SegyImportScanResponse): SegyImportScanResponse {
    return {
      ...response,
      candidate_plans: Array.isArray(response.candidate_plans)
        ? response.candidate_plans.map(normalizeCandidate)
        : [],
      field_observations: Array.isArray(response.field_observations) ? response.field_observations : [],
      issues: Array.isArray(response.issues) ? response.issues.map(normalizeIssue) : []
    };
  }

  function normalizeValidationResponse(
    response: SegyImportValidationResponse
  ): SegyImportValidationResponse {
    return {
      ...response,
      issues: Array.isArray(response.issues) ? response.issues.map(normalizeIssue) : [],
      resolved_spatial: {
        ...response.resolved_spatial,
        notes: Array.isArray(response.resolved_spatial?.notes)
          ? response.resolved_spatial.notes
          : []
      }
    };
  }

  async function loadScan(path: string): Promise<void> {
    resetState();
    scanLoading = true;
    logSegyDiagnostics("info", "Started SEG-Y dialog scan request.", {
      inputPath: path
    });
    try {
      const response = normalizeScanResponse(await scanSegyImport(path));
      const recipeResponse = await listSegyImportRecipes(response.source_fingerprint);
      const availableRecipes = recipeResponse.recipes;
      const rememberedPlan = selectRememberedPlan(availableRecipes, response);
      recipes = availableRecipes;
      scanResponse = response;
      plan = rememberedPlan ?? preferredPlanFromScan(response);
      logSegyDiagnostics("info", "SEG-Y dialog scan payload normalized.", {
        inputPath: response.input_path,
        traceCount: Number(response.trace_count),
        candidateCount: response.candidate_plans.length,
        fieldObservationCount: response.field_observations.length,
        issueCount: response.issues.length,
        recommendedNextStage: response.recommended_next_stage,
        selectedPlanSource: rememberedPlan ? "source_memory" : plan?.provenance.plan_source ?? "scan_default"
      });
      await validateCurrentPlan();
      if (rememberedPlan) {
        showRecipes = true;
      }
    } catch (error) {
      dialogError = errorMessage(error);
      logSegyDiagnostics("error", "SEG-Y dialog scan failed.", {
        inputPath: path,
        error: dialogError
      });
    } finally {
      scanLoading = false;
    }
  }

  function selectRememberedPlan(
    availableRecipes: SegyImportRecipe[],
    response: SegyImportScanResponse
  ): SegyImportPlan | null {
    const remembered = [...availableRecipes]
      .filter(
        (recipe) =>
          recipe.scope === "source_fingerprint" &&
          recipe.source_fingerprint === response.source_fingerprint
      )
      .sort((left, right) => {
        if (left.updated_at_unix_s === right.updated_at_unix_s) {
          return 0;
        }
        return left.updated_at_unix_s > right.updated_at_unix_s ? -1 : 1;
      })[0];

    if (!remembered) {
      return null;
    }
    return materializeRecipePlan(remembered, response);
  }

  function preferredPlanFromScan(response: SegyImportScanResponse): SegyImportPlan {
    const autoCandidate =
      response.candidate_plans.find(
        (candidate) =>
          candidate.auto_selectable &&
          !candidate.issues.some((issue) => issue.severity === "blocking")
      ) ?? null;
    if (autoCandidate) {
      return materializeCandidatePlan(autoCandidate, response, null);
    }
    return materializeDefaultPlan(response, null);
  }

  function materializeDefaultPlan(
    response: SegyImportScanResponse,
    currentPlan: SegyImportPlan | null
  ): SegyImportPlan {
    const outputStorePath =
      currentPlan?.policy.output_store_path?.trim() ||
      response.default_plan.policy.output_store_path;
    return {
      ...structuredClone(response.default_plan),
      input_path: response.input_path,
      source_fingerprint: response.source_fingerprint,
      policy: {
        ...structuredClone(response.default_plan.policy),
        output_store_path: outputStorePath,
        overwrite_existing: false,
        acknowledge_warnings: currentPlan?.policy.acknowledge_warnings ?? false
      },
      provenance: {
        ...structuredClone(response.default_plan.provenance),
        selected_candidate_id: null,
        recipe_id: null,
        recipe_name: null
      }
    };
  }

  function materializeCandidatePlan(
    candidate: SegyImportCandidatePlan,
    response: SegyImportScanResponse,
    currentPlan: SegyImportPlan | null
  ): SegyImportPlan {
    const outputStorePath =
      currentPlan?.policy.output_store_path?.trim() ||
      response.default_plan.policy.output_store_path;
    return {
      ...structuredClone(candidate.plan_patch),
      input_path: response.input_path,
      source_fingerprint: response.source_fingerprint,
      policy: {
        ...structuredClone(candidate.plan_patch.policy),
        output_store_path: outputStorePath,
        overwrite_existing: false,
        acknowledge_warnings: currentPlan?.policy.acknowledge_warnings ?? false
      }
    };
  }

  function materializeRecipePlan(
    recipe: SegyImportRecipe,
    response: SegyImportScanResponse
  ): SegyImportPlan {
    const outputStorePath =
      plan?.policy.output_store_path?.trim() || response.default_plan.policy.output_store_path;
    return {
      ...structuredClone(recipe.plan),
      input_path: response.input_path,
      source_fingerprint: response.source_fingerprint,
      policy: {
        ...structuredClone(recipe.plan.policy),
        output_store_path: outputStorePath,
        overwrite_existing: false,
        acknowledge_warnings: plan?.policy.acknowledge_warnings ?? false
      },
      provenance: {
        ...structuredClone(recipe.plan.provenance),
        plan_source: recipe.scope === "source_fingerprint" ? "source_memory" : "saved_recipe",
        recipe_id: recipe.recipe_id,
        recipe_name: recipe.name
      }
    };
  }

  function planSignature(value: SegyImportPlan): string {
    return JSON.stringify(value);
  }

  function scheduleValidation(): void {
    clearValidationTimer();
    if (!plan) {
      return;
    }
    validationTimer = setTimeout(() => {
      void validateCurrentPlan();
    }, 250);
  }

  async function validateCurrentPlan(): Promise<void> {
    if (!plan) {
      return;
    }
    clearValidationTimer();
    validating = true;
    dialogError = null;
    logSegyDiagnostics("debug", "Started SEG-Y dialog validation.", {
      inputPath: plan.input_path,
      outputStorePath: plan.policy.output_store_path,
      planSource: plan.provenance.plan_source,
      selectedCandidateId: plan.provenance.selected_candidate_id ?? null,
      acknowledgeWarnings: plan.policy.acknowledge_warnings
    });
    try {
      const response = normalizeValidationResponse(await validateSegyImportPlan(plan));
      validation = response;
      plan = structuredClone(response.validated_plan);
      lastValidatedPlanSignature = planSignature(response.validated_plan);
      logSegyDiagnostics("debug", "SEG-Y dialog validation completed.", {
        canImport: response.can_import,
        requiresAcknowledgement: response.requires_acknowledgement,
        issueCount: response.issues.length,
        recommendedNextStage: response.recommended_next_stage,
        outputStorePath: response.validated_plan.policy.output_store_path
      });
    } catch (error) {
      dialogError = errorMessage(error);
      logSegyDiagnostics("error", "SEG-Y dialog validation failed.", {
        inputPath: plan.input_path,
        outputStorePath: plan.policy.output_store_path,
        error: dialogError
      });
    } finally {
      validating = false;
    }
  }

  function updatePlan(nextPlan: SegyImportPlan): void {
    plan = structuredClone(nextPlan);
    dialogError = null;
    scheduleValidation();
  }

  function updateHeaderMappingField(
    target: GeometryFieldTarget,
    kind: "byte" | "type",
    value: string
  ): void {
    if (!plan) {
      return;
    }
    const current = plan.header_mapping[target];
    updatePlan({
      ...plan,
      provenance: {
        ...plan.provenance,
        plan_source: "manual",
        selected_candidate_id: null
      },
      header_mapping: {
        ...plan.header_mapping,
        [target]: nextHeaderField(current, kind, value)
      }
    });
  }

  function updateSpatialField(
    target: SpatialFieldTarget,
    kind: "byte" | "type",
    value: string
  ): void {
    if (!plan) {
      return;
    }
    updatePlan({
      ...plan,
      spatial: {
        ...plan.spatial,
        [target]: nextHeaderField(plan.spatial[target], kind, value)
      }
    });
  }

  function nextHeaderField(
    field: SegyHeaderField | null | undefined,
    kind: "byte" | "type",
    value: string
  ): SegyHeaderField | null {
    const currentByte = field?.start_byte ? String(field.start_byte) : "";
    const currentType = field?.value_type ?? "i32";
    const nextByte = kind === "byte" ? value.trim() : currentByte;
    const nextType = kind === "type" ? (value as SegyHeaderValueType) : currentType;
    if (!nextByte) {
      return null;
    }
    const parsed = Number(nextByte);
    if (!Number.isFinite(parsed) || parsed < 1) {
      return field ?? null;
    }
    return {
      start_byte: parsed,
      value_type: nextType
    };
  }

  function updateSparseHandling(value: string): void {
    if (!plan) {
      return;
    }
    updatePlan({
      ...plan,
      policy: {
        ...plan.policy,
        sparse_handling: value as SegyImportPlan["policy"]["sparse_handling"]
      }
    });
  }

  function updateOutputPath(value: string): void {
    if (!plan) {
      return;
    }
    updatePlan({
      ...plan,
      policy: {
        ...plan.policy,
        output_store_path: value
      }
    });
  }

  function updateWarningAcknowledgement(checked: boolean): void {
    if (!plan) {
      return;
    }
    updatePlan({
      ...plan,
      policy: {
        ...plan.policy,
        acknowledge_warnings: checked
      }
    });
  }

  function updateSpatialText(
    target: "coordinate_units" | "coordinate_reference_id" | "coordinate_reference_name",
    value: string
  ): void {
    if (!plan) {
      return;
    }
    updatePlan({
      ...plan,
      spatial: {
        ...plan.spatial,
        [target]: value
      }
    });
  }

  function applyScannedDefault(): void {
    if (!scanResponse) {
      return;
    }
    logSegyDiagnostics("debug", "Applied scanned default SEG-Y mapping.", {
      inputPath: scanResponse.input_path
    });
    updatePlan(materializeDefaultPlan(scanResponse, plan));
  }

  function applyCandidate(candidateId: string): void {
    if (!scanResponse) {
      return;
    }
    const candidate = scanResponse.candidate_plans.find((entry) => entry.candidate_id === candidateId);
    if (!candidate) {
      return;
    }
    logSegyDiagnostics("debug", "Applied SEG-Y candidate mapping.", {
      inputPath: scanResponse.input_path,
      candidateId,
      candidateLabel: candidate.label,
      candidateIssueCount: candidate.issues.length
    });
    updatePlan(materializeCandidatePlan(candidate, scanResponse, plan));
  }

  function applyRecipe(recipe: SegyImportRecipe): void {
    if (!scanResponse) {
      return;
    }
    updatePlan(materializeRecipePlan(recipe, scanResponse));
  }

  async function saveRecipe(scope: "global" | "source_fingerprint"): Promise<void> {
    if (!plan || !scanResponse) {
      return;
    }
    const normalizedName =
      scope === "source_fingerprint"
        ? `Remembered for ${basename(scanResponse.input_path)}`
        : recipeName.trim();
    if (!normalizedName) {
      dialogError = "Enter a recipe name before saving.";
      showRecipes = true;
      return;
    }

    savingRecipe = true;
    dialogError = null;
    try {
      const timestamp = Math.floor(Date.now() / 1000);
      const recipeId =
        scope === "source_fingerprint"
          ? `source-${slugify(scanResponse.source_fingerprint).slice(0, 48)}`
          : `${slugify(normalizedName)}-${timestamp}`;
      await saveSegyImportRecipe({
        recipe_id: recipeId,
        name: normalizedName,
        scope,
        source_fingerprint: scope === "source_fingerprint" ? scanResponse.source_fingerprint : null,
        plan: structuredClone(plan),
        created_at_unix_s: 0n,
        updated_at_unix_s: 0n
      });
      const updated = await listSegyImportRecipes(scanResponse.source_fingerprint);
      recipes = updated.recipes;
      if (scope === "global") {
        recipeName = "";
      }
    } catch (error) {
      dialogError = errorMessage(error);
      showRecipes = true;
    } finally {
      savingRecipe = false;
    }
  }

  async function removeRecipe(recipeId: string): Promise<void> {
    if (!scanResponse) {
      return;
    }
    try {
      await deleteSegyImportRecipe(recipeId);
      const updated = await listSegyImportRecipes(scanResponse.source_fingerprint);
      recipes = updated.recipes;
    } catch (error) {
      dialogError = errorMessage(error);
      showRecipes = true;
    }
  }

  async function browseOutputPath(): Promise<void> {
    if (!plan) {
      return;
    }
    const selectedPath = await pickOutputStorePath(suggestedOutputPath());
    if (selectedPath) {
      updateOutputPath(selectedPath);
    }
  }

  function suggestedOutputPath(): string {
    const currentOutputPath = plan?.policy.output_store_path?.trim();
    if (currentOutputPath) {
      return currentOutputPath;
    }
    const scannedDefault = scanResponse?.default_plan.policy.output_store_path?.trim();
    if (scannedDefault) {
      return scannedDefault;
    }
    const inputName = basename(inputPath ?? "survey.sgy");
    return inputName.replace(/\.(sgy|segy)$/i, ".tbvol");
  }

  function openCoordinateReferencePicker(): void {
    coordinateReferencePickerOpen = true;
  }

  function closeCoordinateReferencePicker(): void {
    coordinateReferencePickerOpen = false;
  }

  function clearCoordinateReference(): void {
    if (!plan) {
      return;
    }
    updatePlan({
      ...plan,
      spatial: {
        ...plan.spatial,
        coordinate_reference_id: null,
        coordinate_reference_name: null
      }
    });
  }

  function handleCoordinateReferenceSelection(selection: CoordinateReferenceSelection): void {
    if (!plan) {
      return;
    }
    if (selection.kind === "authority_code") {
      updatePlan({
        ...plan,
        spatial: {
          ...plan.spatial,
          coordinate_reference_id: selection.authId,
          coordinate_reference_name: selection.name?.trim() ?? ""
        }
      });
      coordinateReferencePickerOpen = false;
      return;
    }

    if (selection.kind === "local_engineering") {
      updatePlan({
        ...plan,
        spatial: {
          ...plan.spatial,
          coordinate_reference_id: null,
          coordinate_reference_name: selection.label.trim()
        }
      });
    }
    coordinateReferencePickerOpen = false;
  }

  async function retryScan(): Promise<void> {
    if (!inputPath) {
      return;
    }
    await loadScan(inputPath);
  }

  async function confirmImport(): Promise<void> {
    if (!plan || !validation || validationStale) {
      dialogError = "The import plan changed. Wait for validation to finish before importing.";
      logSegyDiagnostics("warn", "SEG-Y dialog import blocked because validation is stale.", {
        inputPath: plan?.input_path ?? null
      });
      return;
    }
    if (!validation.can_import) {
      dialogError = "Resolve the remaining blocking issues before importing.";
      logSegyDiagnostics("warn", "SEG-Y dialog import blocked by validation issues.", {
        inputPath: plan.input_path,
        issueCount: validation.issues.length
      });
      return;
    }

    importing = true;
    dialogError = null;
    logSegyDiagnostics("info", "Started SEG-Y dialog import.", {
      inputPath: validation.validated_plan.input_path,
      outputStorePath: validation.validated_plan.policy.output_store_path,
      issueCount: validation.issues.length,
      warningCount: warningIssues.length
    });
    try {
      const activePlan = plan;
      const matchingEntry =
        viewerModel.workspaceEntries.find(
          (entry) => (entry.source_path ?? "").trim() === activePlan.input_path
        ) ?? null;

      await viewerModel.importSegySurveyPlan(
        validation.validated_plan,
        validation.validation_fingerprint,
        {
          entryId: matchingEntry?.entry_id ?? null,
          sourcePath: activePlan.input_path,
          sessionPipelines: matchingEntry?.session_pipelines
            ? structuredClone(matchingEntry.session_pipelines)
            : null,
          activeSessionPipelineId: matchingEntry?.active_session_pipeline_id ?? null,
          makeActive: true,
          loadSection: true,
          reuseExistingStore: true
        }
      );

      await saveRecipe("source_fingerprint");

      onCommitResult?.({
        providerId: "seismic_volume",
        status: "commit_succeeded",
        outcome: "canonical_commit",
        canonicalAssets: [
          {
            kind: "runtime_store",
            id: validation.validated_plan.policy.output_store_path,
            label: basename(validation.validated_plan.policy.output_store_path),
            detail: activePlan.input_path
          }
        ],
        preservedSources: [],
        droppedItems: [],
        warnings: warningIssues.map((issue) => issue.message),
        blockers: [],
        diagnostics: currentIssues.map((issue) => issue.message),
        refreshScopes: [],
        activationEffects: [],
        requestActions: ["close_after_success"],
        providerDetail: {
          outputStorePath: validation.validated_plan.policy.output_store_path
        }
      });
      logSegyDiagnostics("info", "SEG-Y dialog import completed.", {
        inputPath: validation.validated_plan.input_path,
        outputStorePath: validation.validated_plan.policy.output_store_path
      });
      resetState();
      lastLoadedInputPath = "";
      onClose();
    } catch (error) {
      const message = errorMessage(error);
      dialogError = message;
      logSegyDiagnostics("error", "SEG-Y dialog import failed.", {
        inputPath: validation.validated_plan.input_path,
        outputStorePath: validation.validated_plan.policy.output_store_path,
        error: message
      });
      onCommitResult?.({
        providerId: "seismic_volume",
        status: "commit_failed",
        outcome: "commit_failed",
        canonicalAssets: [],
        preservedSources: [],
        droppedItems: [],
        warnings: [],
        blockers: [message],
        diagnostics: [message],
        refreshScopes: [],
        activationEffects: [],
        providerDetail: null
      });
    } finally {
      importing = false;
    }
  }

  function handleClose(): void {
    if (importing) {
      return;
    }
    resetState();
    lastLoadedInputPath = "";
    onClose();
  }

  function issueMatchesField(issue: SegyImportIssue, path: string): boolean {
    return issue.field_path === path || issue.field_path?.startsWith(`${path}.`) === true;
  }

  function outputPathIssues(): SegyImportIssue[] {
    return currentIssues.filter(
      (issue) =>
        issue.code === "output_store_path_required" ||
        issueMatchesField(issue, "policy.output_store_path") ||
        issue.section === "import"
    );
  }

  function geometrySectionIssues(): SegyImportIssue[] {
    return currentIssues.filter(
      (issue) =>
        issue.section === "structure" &&
        !issueMatchesField(issue, "policy.sparse_handling")
    );
  }

  function sparsePolicyIssues(): SegyImportIssue[] {
    return currentIssues.filter((issue) => issueMatchesField(issue, "policy.sparse_handling"));
  }

  function spatialSectionIssues(): SegyImportIssue[] {
    return currentIssues.filter((issue) => issue.section === "spatial");
  }

  function scanSectionIssues(): SegyImportIssue[] {
    return currentIssues.filter((issue) => issue.section === "scan");
  }

  function leadingIssue(issues: SegyImportIssue[]): SegyImportIssue | null {
    return (
      issues.find((issue) => issue.severity === "blocking") ??
      issues.find((issue) => issue.severity === "warning") ??
      issues[0] ??
      null
    );
  }

  function issueTone(issue: SegyImportIssue | null): "blocking" | "warning" | "info" | null {
    if (!issue) {
      return null;
    }
    if (issue.severity === "blocking") {
      return "blocking";
    }
    if (issue.severity === "warning") {
      return "warning";
    }
    return "info";
  }

  function headerByte(field: SegyHeaderField | null | undefined): string {
    return field?.start_byte ? String(field.start_byte) : "";
  }

  function headerType(field: SegyHeaderField | null | undefined): SegyHeaderValueType {
    return field?.value_type ?? "i32";
  }

  function describeField(field: SegyHeaderField | null | undefined): string {
    if (!field) {
      return "Unset";
    }
    return `${field.start_byte} (${field.value_type.toUpperCase()})`;
  }

  function formatBytes(bytes: bigint | number): string {
    const units = ["B", "KiB", "MiB", "GiB", "TiB"];
    let value = typeof bytes === "bigint" ? Number(bytes) : bytes;
    let unitIndex = 0;
    while (value >= 1024 && unitIndex < units.length - 1) {
      value /= 1024;
      unitIndex += 1;
    }
    return unitIndex === 0
      ? `${Math.round(value)} ${units[unitIndex]}`
      : `${value.toFixed(1)} ${units[unitIndex]}`;
  }

  function formatPercent(value: number): string {
    return `${(value * 100).toFixed(value < 0.01 ? 4 : 2)}%`;
  }

  function formatCount(value: bigint | number | null | undefined): string {
    if (value === null || value === undefined) {
      return "0";
    }
    return value.toLocaleString();
  }

  function basename(path: string): string {
    const normalized = path.replace(/\\/g, "/");
    return normalized.split("/").pop() || normalized;
  }

  function slugify(value: string): string {
    return value
      .toLowerCase()
      .replace(/[^a-z0-9]+/g, "-")
      .replace(/^-+|-+$/g, "")
      .slice(0, 64);
  }

  function candidateSummary(candidate: SegyImportCandidatePlan): string {
    const issue = leadingIssue(candidate.issues);
    if (issue) {
      return issue.message;
    }
    return `${candidate.resolved_dataset.layout} with ${formatCount(candidate.risk_summary.observed_trace_count)} observed traces.`;
  }

  function formatCoordinateReference(): string {
    const identifier = plan?.spatial.coordinate_reference_id?.trim() ?? "";
    const name = plan?.spatial.coordinate_reference_name?.trim() ?? "";
    if (identifier && name) {
      return `${identifier} - ${name}`;
    }
    if (identifier) {
      return identifier;
    }
    if (name) {
      return name;
    }
    return "Unset";
  }

  function statusTone(): "info" | "blocking" | "warning" | "success" {
    if (dialogError) {
      return "blocking";
    }
    if (scanLoading || validating || importing) {
      return "info";
    }
    if (blockingIssues.length > 0) {
      return "blocking";
    }
    if (validation?.requires_acknowledgement && !plan?.policy.acknowledge_warnings) {
      return "warning";
    }
    if (validationStale) {
      return "warning";
    }
    if (canImport) {
      return "success";
    }
    return "info";
  }

  function statusTitle(): string {
    if (dialogError) {
      return "SEG-Y import requires attention";
    }
    if (scanLoading) {
      return "Scanning SEG-Y structure";
    }
    if (validating) {
      return "Revalidating import plan";
    }
    if (importing) {
      return "Importing survey";
    }
    if (blockingIssues.length > 0) {
      return `${blockingIssues.length} blocking issue${blockingIssues.length === 1 ? "" : "s"} to resolve`;
    }
    if (validation?.requires_acknowledgement && !plan?.policy.acknowledge_warnings) {
      return `${warningIssues.length === 1 ? "1 warning requires" : `${warningIssues.length} warnings require`} acknowledgement`;
    }
    if (validationStale) {
      return "Waiting for the updated validation result";
    }
    if (canImport) {
      return "Ready to import";
    }
    if (scanResponse) {
      return "Review the detected mapping";
    }
    return "Choose a SEG-Y file to begin";
  }

  function statusMessage(): string {
    if (dialogError) {
      return dialogError;
    }
    if (scanLoading) {
      return "Reading trace headers, geometry, and candidate mappings.";
    }
    if (validating) {
      return "Validation updates automatically after each edit.";
    }
    if (importing) {
      return "Writing the validated SEG-Y plan into a runtime store.";
    }
    if (blockingIssues.length > 0) {
      return "Fix the highlighted mapping or import-option issues in this dialog. There is no separate review page.";
    }
    if (validation?.requires_acknowledgement && !plan?.policy.acknowledge_warnings) {
      return "The remaining warnings do not block import, but they do require explicit acknowledgement.";
    }
    if (validationStale) {
      return "The latest edit is still being validated.";
    }
    if (canImport) {
      return "The current scan, mapping, and output plan are valid.";
    }
    return "Scan results and import readiness update inline as you edit the form.";
  }

  function errorMessage(error: unknown): string {
    return error instanceof Error ? error.message : "The SEG-Y import flow failed.";
  }
</script>

{#if open}
  <div
    class={["segy-import-backdrop", embedded && "embedded"]}
    role="presentation"
    onclick={() => {
      if (!embedded) {
        handleClose();
      }
    }}
  >
    <div
      class={["segy-import-dialog", embedded && "embedded"]}
      role="dialog"
      aria-modal={!embedded}
      aria-label="Import SEG-Y survey"
      tabindex="0"
      onclick={(event) => event.stopPropagation()}
      onkeydown={(event) => event.stopPropagation()}
    >
      <header class="segy-import-header">
        <div>
          <h2>Import SEG-Y Survey</h2>
          <p>{inputPath ? basename(inputPath) : "Choose a SEG-Y file to begin."}</p>
        </div>
        <button class="settings-btn secondary" type="button" onclick={handleClose} disabled={importing}>
          Cancel
        </button>
      </header>

      <section class={`status-strip ${statusTone()}`}>
        <div>
          <h3>{statusTitle()}</h3>
          <p>{statusMessage()}</p>
        </div>
        {#if dialogError && inputPath}
          <button class="settings-btn secondary" type="button" onclick={() => void retryScan()} disabled={scanLoading}>
            Retry Scan
          </button>
        {/if}
      </section>

      {#if scanLoading && !scanResponse}
        <section class="section-card empty-state">
          <h3>Scanning SEG-Y structure</h3>
          <p>The dialog will populate automatically when the header scan finishes.</p>
        </section>
      {:else if !scanResponse && !scanLoading}
        <section class="section-card empty-state">
          <h3>No SEG-Y scan available</h3>
          <p>Retry the scan to populate detected geometry, candidate mappings, and import readiness.</p>
        </section>
      {:else if scanResponse && plan}
        <fieldset class="fieldset">
          <legend>Scan Summary</legend>
          <table class="summary-table">
            <tbody>
              <tr><td>File</td><td>{basename(scanResponse.input_path)}</td></tr>
              <tr><td>Traces</td><td>{formatCount(scanResponse.trace_count)}</td></tr>
              <tr><td>Samples / trace</td><td>{formatCount(scanResponse.samples_per_trace)}</td></tr>
              <tr><td>Sample interval</td><td>{formatCount(scanResponse.sample_interval_us)} us</td></tr>
              <tr><td>Format</td><td>Code {scanResponse.sample_format_code} / {scanResponse.endianness}</td></tr>
              <tr><td>Layout</td><td>{validation?.resolved_dataset.layout ?? "Pending"}</td></tr>
              <tr><td>Classification</td><td>{validation?.resolved_dataset.classification ?? resolvedRiskSummary?.classification ?? "Detected survey"}</td></tr>
              <tr><td>Completeness</td><td>{resolvedRiskSummary ? formatPercent(resolvedRiskSummary.completeness_ratio) : "Pending"}</td></tr>
              <tr>
                <td>Observed / expected</td>
                <td>
                  {resolvedRiskSummary
                    ? `${formatCount(resolvedRiskSummary.observed_trace_count)} / ${formatCount(resolvedRiskSummary.expected_trace_count)}`
                    : "Pending"}
                </td>
              </tr>
              <tr><td>Blow-up</td><td>{resolvedRiskSummary ? `${resolvedRiskSummary.blowup_ratio.toFixed(2)}x` : "Pending"}</td></tr>
              <tr><td>Est. store</td><td>{resolvedRiskSummary ? formatBytes(resolvedRiskSummary.estimated_total_bytes) : "Pending"}</td></tr>
              <tr>
                <td>Mapping</td>
                <td>
                  {plan.provenance.plan_source === "candidate"
                    ? "Suggested candidate"
                    : plan.provenance.plan_source === "source_memory"
                      ? "Remembered for this source"
                      : plan.provenance.plan_source === "saved_recipe"
                        ? "Saved recipe"
                        : plan.provenance.plan_source === "manual"
                          ? "Manual"
                          : "Scanned default"}
                </td>
              </tr>
            </tbody>
          </table>

          {#if scanSectionIssues().length > 0}
            <div class="section-issues info">
              {#each scanSectionIssues() as issue (`scan-${issue.code}-${issue.message}`)}
                <div class="issue-row">
                  <strong>{issue.severity}</strong>
                  <span>{issue.message}</span>
                </div>
              {/each}
            </div>
          {/if}
        </fieldset>

        <fieldset class="fieldset">
          <legend>Geometry Mapping</legend>

          {#if leadingIssue(geometrySectionIssues())}
            <div class={`section-banner ${issueTone(leadingIssue(geometrySectionIssues()))}`}>
              <strong>{leadingIssue(geometrySectionIssues())?.message}</strong>
              {#if leadingIssue(geometrySectionIssues())?.suggested_fix}
                <span>{leadingIssue(geometrySectionIssues())?.suggested_fix}</span>
              {/if}
            </div>
          {/if}

          <div class="candidate-list">
            <label class={["candidate-card", !plan.provenance.selected_candidate_id && "selected"]}>
              <input
                type="radio"
                name="segy-candidate"
                checked={!plan.provenance.selected_candidate_id}
                onchange={applyScannedDefault}
              />
              <div>
                <strong>Scanned default</strong>
                <span>Use the mapping inferred directly from the SEG-Y scan.</span>
              </div>
            </label>

            {#each scanResponse.candidate_plans as candidate (candidate.candidate_id)}
              <label class={["candidate-card", plan.provenance.selected_candidate_id === candidate.candidate_id && "selected"]}>
                <input
                  type="radio"
                  name="segy-candidate"
                  checked={plan.provenance.selected_candidate_id === candidate.candidate_id}
                  onchange={() => applyCandidate(candidate.candidate_id)}
                />
                <div>
                  <div class="candidate-title-row">
                    <strong>{candidate.label}</strong>
                    {#if candidate.auto_selectable}
                      <small class="auto-tag">auto</small>
                    {/if}
                  </div>
                  <span>{candidate.resolved_dataset.classification} / {candidate.resolved_dataset.layout} &mdash; {formatCount(candidate.risk_summary.observed_trace_count)} / {formatCount(candidate.risk_summary.expected_trace_count)} traces</span>
                </div>
              </label>
            {/each}
          </div>

          <div class="form-grid">
            <label class="inline-field">
              <span>Inline byte</span>
              <input
                type="number"
                min="1"
                value={headerByte(plan.header_mapping.inline_3d)}
                oninput={(event) =>
                  updateHeaderMappingField("inline_3d", "byte", (event.currentTarget as HTMLInputElement).value)}
              />
            </label>
            <label class="inline-field">
              <span>Inline type</span>
              <select
                value={headerType(plan.header_mapping.inline_3d)}
                onchange={(event) =>
                  updateHeaderMappingField("inline_3d", "type", (event.currentTarget as HTMLSelectElement).value)}
              >
                <option value="i32">I32</option>
                <option value="i16">I16</option>
              </select>
            </label>
            <label class="inline-field">
              <span>Crossline byte</span>
              <input
                type="number"
                min="1"
                value={headerByte(plan.header_mapping.crossline_3d)}
                oninput={(event) =>
                  updateHeaderMappingField("crossline_3d", "byte", (event.currentTarget as HTMLInputElement).value)}
              />
            </label>
            <label class="inline-field">
              <span>Crossline type</span>
              <select
                value={headerType(plan.header_mapping.crossline_3d)}
                onchange={(event) =>
                  updateHeaderMappingField("crossline_3d", "type", (event.currentTarget as HTMLSelectElement).value)}
              >
                <option value="i32">I32</option>
                <option value="i16">I16</option>
              </select>
            </label>
            {#if showThirdAxis}
              <label class="inline-field">
                <span>Third axis byte</span>
                <input
                  type="number"
                  min="1"
                  value={headerByte(plan.header_mapping.third_axis)}
                  oninput={(event) =>
                    updateHeaderMappingField("third_axis", "byte", (event.currentTarget as HTMLInputElement).value)}
                />
              </label>
              <label class="inline-field">
                <span>Third axis type</span>
                <select
                  value={headerType(plan.header_mapping.third_axis)}
                  onchange={(event) =>
                    updateHeaderMappingField("third_axis", "type", (event.currentTarget as HTMLSelectElement).value)}
                >
                  <option value="i32">I32</option>
                  <option value="i16">I16</option>
                </select>
              </label>
            {/if}
          </div>
        </fieldset>

        <fieldset class="fieldset">
          <legend>Import Options</legend>

          {#if leadingIssue(sparsePolicyIssues())}
            <div class={`section-banner ${issueTone(leadingIssue(sparsePolicyIssues()))}`}>
              <strong>{leadingIssue(sparsePolicyIssues())?.message}</strong>
              {#if leadingIssue(sparsePolicyIssues())?.suggested_fix}
                <span>{leadingIssue(sparsePolicyIssues())?.suggested_fix}</span>
              {/if}
            </div>
          {/if}

          <div class="form-grid">
            <label class="inline-field wide">
              <span>Sparse handling</span>
              <select
                value={plan.policy.sparse_handling}
                onchange={(event) => updateSparseHandling((event.currentTarget as HTMLSelectElement).value)}
              >
                <option value="block_import">Stop and review</option>
                <option value="regularize_to_dense">Regularize to dense grid</option>
              </select>
            </label>
          </div>

          {#if resolvedRiskSummary}
            <p class="field-help">
              {formatCount(resolvedRiskSummary.observed_trace_count)} observed,
              {formatCount(resolvedRiskSummary.expected_trace_count)} bins,
              {formatBytes(resolvedRiskSummary.estimated_total_bytes)} store.
            </p>
          {/if}

          <label class="inline-field wide">
            <span>Output path</span>
            <div class="path-row">
              <input
                type="text"
                value={plan.policy.output_store_path}
                oninput={(event) => updateOutputPath((event.currentTarget as HTMLInputElement).value)}
              />
              <button class="settings-btn secondary" type="button" onclick={() => void browseOutputPath()}>
                Browse...
              </button>
            </div>
          </label>

          {#if outputPathIssues().length > 0}
            <div class="field-issues">
              {#each outputPathIssues() as issue (`output-${issue.code}-${issue.message}`)}
                <div class={`inline-issue ${issue.severity}`}>
                  <strong>{issue.message}</strong>
                  {#if issue.suggested_fix}
                    <span>{issue.suggested_fix}</span>
                  {/if}
                </div>
              {/each}
            </div>
          {/if}

          {#if validation?.requires_acknowledgement}
            <label class="checkbox-row">
              <input
                type="checkbox"
                checked={plan.policy.acknowledge_warnings}
                onchange={(event) =>
                  updateWarningAcknowledgement((event.currentTarget as HTMLInputElement).checked)}
              />
              <span>I understand the remaining warnings and want to continue.</span>
            </label>
          {/if}
        </fieldset>

        <fieldset class="fieldset collapsible">
          <legend>
            <button type="button" class="fieldset-toggle" onclick={() => (showAdvancedSpatial = !showAdvancedSpatial)}>
              <span class="toggle-indicator">{showAdvancedSpatial ? "\u25BC" : "\u25B6"}</span>
              Advanced Spatial Metadata
            </button>
          </legend>

          {#if showAdvancedSpatial}
            {#if leadingIssue(spatialSectionIssues())}
              <div class={`section-banner ${issueTone(leadingIssue(spatialSectionIssues()))}`}>
                <strong>{leadingIssue(spatialSectionIssues())?.message}</strong>
                {#if leadingIssue(spatialSectionIssues())?.suggested_fix}
                  <span>{leadingIssue(spatialSectionIssues())?.suggested_fix}</span>
                {/if}
              </div>
            {/if}

            <div class="form-grid">
              <label class="inline-field">
                <span>X byte</span>
                <input
                  type="number"
                  min="1"
                  value={headerByte(plan.spatial.x_field)}
                  oninput={(event) =>
                    updateSpatialField("x_field", "byte", (event.currentTarget as HTMLInputElement).value)}
                />
              </label>
              <label class="inline-field">
                <span>X type</span>
                <select
                  value={headerType(plan.spatial.x_field)}
                  onchange={(event) =>
                    updateSpatialField("x_field", "type", (event.currentTarget as HTMLSelectElement).value)}
                >
                  <option value="i32">I32</option>
                  <option value="i16">I16</option>
                </select>
              </label>
              <label class="inline-field">
                <span>Y byte</span>
                <input
                  type="number"
                  min="1"
                  value={headerByte(plan.spatial.y_field)}
                  oninput={(event) =>
                    updateSpatialField("y_field", "byte", (event.currentTarget as HTMLInputElement).value)}
                />
              </label>
              <label class="inline-field">
                <span>Y type</span>
                <select
                  value={headerType(plan.spatial.y_field)}
                  onchange={(event) =>
                    updateSpatialField("y_field", "type", (event.currentTarget as HTMLSelectElement).value)}
                >
                  <option value="i32">I32</option>
                  <option value="i16">I16</option>
                </select>
              </label>
              <label class="inline-field">
                <span>Scalar byte</span>
                <input
                  type="number"
                  min="1"
                  value={headerByte(plan.spatial.coordinate_scalar_field)}
                  oninput={(event) =>
                    updateSpatialField(
                      "coordinate_scalar_field",
                      "byte",
                      (event.currentTarget as HTMLInputElement).value
                    )}
                />
              </label>
              <label class="inline-field">
                <span>Scalar type</span>
                <select
                  value={headerType(plan.spatial.coordinate_scalar_field)}
                  onchange={(event) =>
                    updateSpatialField(
                      "coordinate_scalar_field",
                      "type",
                      (event.currentTarget as HTMLSelectElement).value
                    )}
                >
                  <option value="i32">I32</option>
                  <option value="i16">I16</option>
                </select>
              </label>
              <label class="inline-field wide">
                <span>Coord. units</span>
                <input
                  type="text"
                  value={plan.spatial.coordinate_units ?? ""}
                  oninput={(event) =>
                    updateSpatialText("coordinate_units", (event.currentTarget as HTMLInputElement).value)}
                />
              </label>
              <label class="inline-field wide">
                <span>Source CRS</span>
                <div class="crs-inline">
                  <strong class="crs-value">{formatCoordinateReference()}</strong>
                  <button class="settings-btn secondary" type="button" onclick={openCoordinateReferencePicker}>
                    Choose
                  </button>
                  <button class="settings-btn secondary" type="button" onclick={clearCoordinateReference}>
                    Clear
                  </button>
                </div>
              </label>
            </div>

            {#if validation?.resolved_spatial.notes.length}
              <div class="field-help-list">
                {#each validation.resolved_spatial.notes as note (`spatial-note-${note}`)}
                  <span>{note}</span>
                {/each}
              </div>
            {/if}
          {/if}
        </fieldset>

        <fieldset class="fieldset collapsible">
          <legend>
            <button type="button" class="fieldset-toggle" onclick={() => (showInspection = !showInspection)}>
              <span class="toggle-indicator">{showInspection ? "\u25BC" : "\u25B6"}</span>
              Inspection Details
            </button>
          </legend>

          {#if showInspection}
            <table class="summary-table inspection">
              <thead>
                <tr><th>Field</th><th>Mapping</th><th>Unique</th></tr>
              </thead>
              <tbody>
                {#each scanResponse.field_observations as field (`${field.label}-${field.field.start_byte}`)}
                  <tr>
                    <td>{field.label}</td>
                    <td>{describeField(field.field)}</td>
                    <td>{formatCount(field.unique_count)}</td>
                  </tr>
                {/each}
              </tbody>
            </table>

            {#if infoIssues.length > 0}
              <div class="section-issues info">
                {#each infoIssues as issue (`info-${issue.code}-${issue.message}`)}
                  <div class="issue-row">
                    <strong>{issue.severity}</strong>
                    <span>{issue.message}</span>
                  </div>
                {/each}
              </div>
            {/if}
          {/if}
        </fieldset>

        <fieldset class="fieldset collapsible">
          <legend>
            <button type="button" class="fieldset-toggle" onclick={() => (showRecipes = !showRecipes)}>
              <span class="toggle-indicator">{showRecipes ? "\u25BC" : "\u25B6"}</span>
              Saved Settings
            </button>
          </legend>

          {#if showRecipes}
            <div class="recipe-actions">
              <input
                type="text"
                value={recipeName}
                placeholder="Recipe name"
                oninput={(event) => (recipeName = (event.currentTarget as HTMLInputElement).value)}
              />
              <button class="settings-btn secondary" type="button" onclick={() => void saveRecipe("global")} disabled={savingRecipe}>
                Save
              </button>
              <button
                class="settings-btn secondary"
                type="button"
                onclick={() => void saveRecipe("source_fingerprint")}
                disabled={savingRecipe || !scanResponse}
              >
                Remember
              </button>
            </div>

            {#if displayedRecipes.length > 0}
              <div class="recipe-list">
                {#each displayedRecipes as recipe (recipe.recipe_id)}
                  <div class="recipe-row">
                    <div>
                      <strong>{recipe.name}</strong>
                      <span>{recipe.scope === "source_fingerprint" ? "source memory" : "saved recipe"}</span>
                    </div>
                    <div class="button-row">
                      <button class="settings-btn secondary" type="button" onclick={() => applyRecipe(recipe)}>
                        Apply
                      </button>
                      <button class="settings-btn secondary" type="button" onclick={() => void removeRecipe(recipe.recipe_id)}>
                        Delete
                      </button>
                    </div>
                  </div>
                {/each}
              </div>
            {/if}
          {/if}
        </fieldset>

        <footer class="dialog-footer">
          <div class="footer-status">
            <strong>{statusTitle()}</strong>
            {#if validationStale}
              <span>Validation is still catching up with the latest edit.</span>
            {:else if validating}
              <span>Validating updated plan...</span>
            {:else if canImport && validation}
              <span>Import will use {basename(validation.validated_plan.policy.output_store_path)}.</span>
            {:else if blockingIssues.length > 0}
              <span>Resolve the highlighted blocking issues to enable import.</span>
            {:else if validation?.requires_acknowledgement && !plan.policy.acknowledge_warnings}
              <span>Acknowledge the remaining warnings to continue.</span>
            {/if}
          </div>
          <div class="button-row">
            <button class="settings-btn secondary" type="button" onclick={handleClose} disabled={importing}>
              Cancel
            </button>
            <button
              class="settings-btn primary"
              type="button"
              onclick={() => void confirmImport()}
              disabled={!canImport}
            >
              {importing ? "Importing..." : "Import Survey"}
            </button>
          </div>
        </footer>
      {/if}
    </div>
  </div>

  {#if coordinateReferencePickerOpen}
    <CoordinateReferencePicker
      close={closeCoordinateReferencePicker}
      confirm={handleCoordinateReferenceSelection}
      title="SEG-Y Source CRS"
      description="Choose the CRS used by the imported SEG-Y coordinates, or record them as local engineering coordinates."
      allowLocalEngineering={true}
      localEngineeringLabel="Local engineering coordinates"
      selectedAuthId={plan?.spatial.coordinate_reference_id ?? null}
      projectRoot={viewerModel.projectRoot}
      projectedOnly={false}
      includeGeographic={true}
      includeVertical={false}
    />
  {/if}
{/if}

<style>
  /* ── backdrop / shell ── */

  .segy-import-backdrop {
    position: fixed;
    inset: 0;
    z-index: 60;
    display: grid;
    place-items: center;
    padding: 24px;
    background: rgba(38, 55, 71, 0.2);
    backdrop-filter: blur(4px);
  }

  .segy-import-backdrop.embedded {
    position: static;
    padding: 0;
    background: transparent;
    backdrop-filter: none;
  }

  .segy-import-dialog {
    width: min(820px, 100%);
    max-height: min(92vh, 980px);
    overflow: auto;
    padding: 14px 16px;
    border: 1px solid var(--app-border-strong);
    border-radius: 6px;
    background: var(--panel-bg);
    display: grid;
    gap: 10px;
  }

  .segy-import-dialog.embedded {
    width: 100%;
    max-height: none;
    border-radius: 6px;
    box-shadow: none;
  }

  /* ── header ── */

  .segy-import-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 10px;
  }

  .segy-import-header h2 {
    margin: 0;
    font-size: 13px;
    font-weight: 650;
  }

  .segy-import-header p {
    margin: 2px 0 0;
    color: var(--text-muted);
    font-size: 11px;
  }

  /* ── status strip ── */

  .status-strip {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 10px;
    padding: 8px 10px;
    border: 1px solid var(--app-border);
    border-radius: 4px;
    background: var(--surface-subtle);
  }

  .status-strip h3 {
    margin: 0;
    font-size: 11px;
    font-weight: 650;
  }

  .status-strip p {
    margin: 2px 0 0;
    color: var(--text-muted);
    font-size: 11px;
  }

  .status-strip.info {
    border-color: var(--info-border);
    background: var(--info-bg);
  }

  .status-strip.warning {
    border-color: var(--warn-border);
    background: var(--warn-bg);
  }

  .status-strip.blocking {
    border-color: var(--danger-border);
    background: var(--danger-bg);
  }

  .status-strip.success {
    border-color: rgba(46, 174, 107, 0.35);
    background: rgba(46, 174, 107, 0.08);
  }

  /* ── fieldset sections (QGIS-style) ── */

  .fieldset {
    margin: 0;
    padding: 8px 10px 10px;
    border: 1px solid var(--app-border);
    border-radius: 4px;
    background: var(--surface-bg);
    display: grid;
    gap: 8px;
  }

  .fieldset > legend {
    padding: 0 4px;
    font-size: 11px;
    font-weight: 650;
    color: var(--text-primary);
  }

  .fieldset.collapsible > legend {
    padding: 0;
  }

  .fieldset-toggle {
    display: inline-flex;
    align-items: center;
    gap: 4px;
    padding: 0 4px;
    border: none;
    background: transparent;
    font: inherit;
    font-size: 11px;
    font-weight: 650;
    color: var(--text-primary);
    cursor: pointer;
  }

  .toggle-indicator {
    font-size: 8px;
    width: 10px;
    text-align: center;
    color: var(--text-dim);
  }

  /* ── section-card (empty states) ── */

  .section-card {
    padding: 10px 12px;
    border: 1px solid var(--app-border);
    border-radius: 4px;
    background: var(--surface-bg);
    display: grid;
    gap: 6px;
  }

  .section-card h3 {
    margin: 0;
    font-size: 11px;
    font-weight: 650;
  }

  .empty-state {
    min-height: 80px;
    align-content: center;
  }

  .empty-state p,
  .field-help {
    margin: 2px 0 0;
    color: var(--text-muted);
    font-size: 11px;
  }

  /* ── summary table ── */

  .summary-table {
    width: 100%;
    border-collapse: collapse;
    font-size: 11px;
  }

  .summary-table td,
  .summary-table th {
    padding: 3px 8px;
    border: 1px solid var(--app-border);
    text-align: left;
    vertical-align: top;
  }

  .summary-table td:first-child,
  .summary-table th:first-child {
    color: var(--text-muted);
    white-space: nowrap;
    width: 130px;
  }

  .summary-table th {
    background: var(--surface-subtle);
    font-weight: 600;
  }

  .summary-table.inspection td:first-child {
    width: auto;
  }

  /* ── inline form fields (QGIS-style label | input on same row) ── */

  .form-grid {
    display: grid;
    gap: 6px;
    grid-template-columns: repeat(2, minmax(0, 1fr));
  }

  .inline-field {
    display: grid;
    grid-template-columns: auto minmax(0, 1fr);
    align-items: center;
    gap: 8px;
  }

  .inline-field > span {
    font-size: 11px;
    color: var(--text-muted);
    white-space: nowrap;
  }

  .inline-field.wide {
    grid-column: 1 / -1;
  }

  /* ── inputs ── */

  input,
  select {
    min-width: 0;
    height: 26px;
    padding: 0 6px;
    border: 1px solid var(--app-border-strong);
    border-radius: 3px;
    background: #fff;
    color: var(--text-primary);
    font: inherit;
    font-size: 11px;
  }

  select {
    padding-right: 18px;
  }

  input[type="checkbox"] {
    height: auto;
    width: 14px;
    padding: 0;
  }

  /* ── path row ── */

  .path-row {
    display: flex;
    align-items: center;
    gap: 6px;
  }

  .path-row input {
    flex: 1 1 auto;
  }

  /* ── CRS inline ── */

  .crs-inline {
    display: flex;
    align-items: center;
    gap: 6px;
    min-width: 0;
  }

  .crs-value {
    flex: 1 1 auto;
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    font-size: 11px;
    font-weight: 600;
  }

  /* ── buttons ── */

  .settings-btn {
    height: 26px;
    padding: 0 10px;
    border: 1px solid var(--app-border-strong);
    border-radius: 3px;
    background: var(--surface-subtle);
    color: var(--text-primary);
    font: inherit;
    font-size: 11px;
    white-space: nowrap;
    cursor: pointer;
  }

  .settings-btn.primary {
    border-color: var(--accent-text);
    background: var(--accent-text);
    color: #fff;
    font-weight: 600;
  }

  .settings-btn:disabled {
    opacity: 0.5;
    cursor: default;
  }

  /* ── candidate cards ── */

  .candidate-list {
    display: grid;
    gap: 4px;
  }

  .candidate-card {
    display: grid;
    grid-template-columns: auto minmax(0, 1fr);
    align-items: start;
    gap: 6px;
    padding: 6px 8px;
    border: 1px solid var(--app-border);
    border-radius: 3px;
    background: #fff;
    cursor: pointer;
  }

  .candidate-card.selected {
    border-color: var(--accent-border);
    background: var(--accent-bg);
  }

  .candidate-card input[type="radio"] {
    margin-top: 2px;
    height: auto;
  }

  .candidate-card strong {
    font-size: 11px;
  }

  .candidate-card span,
  .candidate-card small {
    font-size: 11px;
    color: var(--text-muted);
  }

  .candidate-title-row {
    display: flex;
    align-items: center;
    gap: 6px;
  }

  .auto-tag {
    padding: 0 4px;
    border-radius: 2px;
    background: var(--accent-bg);
    color: var(--accent-text);
    font-size: 9px;
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 0.04em;
  }

  /* ── banners / issues ── */

  .section-banner,
  .inline-issue {
    padding: 6px 8px;
    border: 1px solid var(--app-border);
    border-radius: 3px;
    background: var(--surface-subtle);
    display: grid;
    gap: 2px;
    font-size: 11px;
  }

  .section-banner.info,
  .inline-issue.info,
  .section-issues.info {
    border-color: var(--info-border);
    background: var(--info-bg);
  }

  .section-banner.warning,
  .inline-issue.warning {
    border-color: var(--warn-border);
    background: var(--warn-bg);
  }

  .section-banner.blocking,
  .inline-issue.blocking {
    border-color: var(--danger-border);
    background: var(--danger-bg);
  }

  .section-banner span,
  .inline-issue span {
    color: var(--text-muted);
  }

  .section-issues {
    padding: 6px 8px;
    border: 1px solid transparent;
    border-radius: 3px;
  }

  .issue-row {
    display: grid;
    gap: 2px;
    font-size: 11px;
  }

  .field-issues {
    display: grid;
    gap: 4px;
  }

  /* ── checkbox row ── */

  .checkbox-row {
    display: grid;
    grid-template-columns: auto minmax(0, 1fr);
    align-items: start;
    gap: 6px;
    font-size: 11px;
  }

  /* ── field help ── */

  .field-help-list {
    display: grid;
    gap: 2px;
  }

  .field-help-list span,
  .footer-status span {
    color: var(--text-muted);
    font-size: 11px;
  }

  /* ── recipe section ── */

  .recipe-actions {
    display: grid;
    gap: 6px;
    grid-template-columns: minmax(0, 1fr) auto auto;
    align-items: center;
  }

  .recipe-list {
    display: grid;
    gap: 4px;
  }

  .recipe-row {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 8px;
    padding: 5px 8px;
    border: 1px solid var(--app-border);
    border-radius: 3px;
    background: #fff;
    font-size: 11px;
  }

  .recipe-row span {
    color: var(--text-muted);
  }

  .button-row {
    display: flex;
    align-items: center;
    gap: 6px;
  }

  /* ── footer ── */

  .dialog-footer {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 10px;
    border-top: 1px solid var(--app-border);
    padding-top: 10px;
  }

  .footer-status {
    display: grid;
    gap: 2px;
  }

  .footer-status strong {
    font-size: 11px;
  }

  /* ── responsive ── */

  @media (max-width: 760px) {
    .form-grid {
      grid-template-columns: 1fr;
    }

    .inline-field {
      grid-template-columns: 1fr;
    }

    .recipe-actions {
      grid-template-columns: 1fr;
    }

    .dialog-footer {
      flex-direction: column;
      align-items: stretch;
    }
  }
</style>
