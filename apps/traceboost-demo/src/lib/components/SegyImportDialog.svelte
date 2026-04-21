<svelte:options runes={true} />

<script lang="ts">
  import type {
    ImportConfirmationStage,
    ImportFlowStep
  } from "./import-review";
  import ImportFlowStepper from "./ImportFlowStepper.svelte";
  import type { ViewerModel } from "../viewer-model.svelte";
  import {
    deleteSegyImportRecipe,
    listSegyImportRecipes,
    saveSegyImportRecipe,
    scanSegyImport,
    validateSegyImportPlan
  } from "../bridge";
  import type {
    SegyHeaderField,
    SegyHeaderValueType,
    SegyImportIssue,
    SegyImportPlan,
    SegyImportRecipe,
    SegyImportScanResponse,
    SegyImportValidationResponse
  } from "@traceboost/seis-contracts";

  interface Props {
    open: boolean;
    inputPath: string | null;
    viewerModel: ViewerModel;
    onClose: () => void;
  }

  type WizardStage = "scan" | "structure" | "spatial" | "review" | "import" | "raw_inspect";
  type EditableFieldTarget =
    | "inline_3d"
    | "crossline_3d"
    | "third_axis"
    | "x_field"
    | "y_field"
    | "coordinate_scalar_field";

  let { open, inputPath, viewerModel, onClose }: Props = $props();

  const baseSteps: ImportFlowStep[] = [
    { key: "scan", label: "1. Scan", description: "Read the SEG-Y structure and suggested mappings." },
    {
      key: "structure",
      label: "2. Structure",
      description: "Set the header mapping and sparse-grid import policy."
    },
    {
      key: "spatial",
      label: "3. Spatial",
      description: "Optional spatial header and CRS metadata for this source."
    },
    { key: "review", label: "4. Review", description: "Validate the plan and inspect the risks." },
    { key: "import", label: "5. Import", description: "Run the validated plan into a runtime store." }
  ];

  let stage = $state<WizardStage>("scan");
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
  const sourceRecipes = $derived(
    recipes.filter((recipe) => recipe.scope === "source_fingerprint")
  );
  const globalRecipes = $derived(recipes.filter((recipe) => recipe.scope === "global"));
  const denseGridWarning = $derived(
    currentIssues.find(
      (issue) =>
        issue.code === "sparse_policy_required" || issue.code === "sparse_regularization"
    ) ?? null
  );
  const canImport = $derived(
    !!validation && validation.can_import && !validationStale && !importing && !viewerModel.loading
  );
  const flowSteps = $derived.by((): ImportFlowStep[] => {
    const scanReady = !!scanResponse;
    const currentIndex = stageIndex(stage);

    return baseSteps.map((step, index) => {
      const severity = highestSeverityForStep(step.key);
      let status: ImportFlowStep["status"] = "pending";
      if (step.key === "scan") {
        if (scanLoading || !scanReady) {
          status = "active";
        } else if (severity) {
          status = severity;
        } else {
          status = currentIndex > index ? "completed" : "active";
        }
      } else if (!scanReady) {
        status = "pending";
      } else if (step.key === "import") {
        if (stage === "import") {
          status = canImport ? "active" : highestSeverityForStep("review") ?? "warning";
        } else {
          status = canImport ? "completed" : "pending";
        }
      } else if (severity) {
        status = severity;
      } else if (currentIndex > index) {
        status = "completed";
      } else if (currentIndex === index) {
        status = "active";
      }

      return {
        ...step,
        disabled: !scanReady && step.key !== "scan",
        status,
        detail: stepDetail(step.key, status)
      };
    });
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

  function stageIndex(currentStage: WizardStage): number {
    switch (currentStage) {
      case "scan":
        return 0;
      case "structure":
        return 1;
      case "spatial":
        return 2;
      case "review":
        return 3;
      case "import":
        return 4;
      default:
        return 0;
    }
  }

  function sectionsForStep(stepKey: ImportConfirmationStage): Array<SegyImportIssue["section"]> {
    switch (stepKey) {
      case "scan":
        return ["scan"];
      case "structure":
        return ["scan", "structure"];
      case "spatial":
        return ["spatial"];
      case "review":
      case "import":
        return ["scan", "structure", "spatial", "review", "import"];
      default:
        return [];
    }
  }

  function highestSeverityForStep(
    stepKey: ImportConfirmationStage
  ): "warning" | "blocking" | null {
    const matchingIssues = currentIssues.filter((issue) =>
      sectionsForStep(stepKey).includes(issue.section)
    );
    if (matchingIssues.some((issue) => issue.severity === "blocking")) {
      return "blocking";
    }
    if (matchingIssues.some((issue) => issue.severity === "warning")) {
      return "warning";
    }
    return null;
  }

  function stepDetail(
    stepKey: ImportConfirmationStage,
    status: ImportFlowStep["status"]
  ): string | undefined {
    if (status === "completed") {
      return stepKey === "import" ? "Ready" : "Passed";
    }
    if (status === "warning") {
      return "Needs review";
    }
    if (status === "blocking") {
      return "Fix required";
    }
    if (status === "active") {
      return stepKey === "scan" && scanLoading ? "Scanning..." : "Current";
    }
    return "Pending";
  }

  function clearValidationTimer(): void {
    if (validationTimer) {
      clearTimeout(validationTimer);
      validationTimer = null;
    }
  }

  function resetState(): void {
    clearValidationTimer();
    stage = "scan";
    scanResponse = null;
    plan = null;
    validation = null;
    recipes = [];
    dialogError = null;
    recipeName = "";
    lastValidatedPlanSignature = "";
  }

  async function loadScan(path: string): Promise<void> {
    resetState();
    scanLoading = true;
    try {
      const response = await scanSegyImport(path);
      const recipeResponse = await listSegyImportRecipes(response.source_fingerprint);
      const availableRecipes = recipeResponse.recipes;
      recipes = availableRecipes;
      scanResponse = response;
      const rememberedPlan = selectRememberedPlan(availableRecipes, response.source_fingerprint);
      plan = rememberedPlan ?? structuredClone(response.default_plan);
      stage = rememberedPlan ? "review" : wizardStageFromScan(response.recommended_next_stage);
      await validateCurrentPlan(false);
      if (rememberedPlan) {
        dialogError = null;
      }
    } catch (error) {
      dialogError = errorMessage(error);
    } finally {
      scanLoading = false;
    }
  }

  function selectRememberedPlan(
    availableRecipes: SegyImportRecipe[],
    sourceFingerprint: string
  ): SegyImportPlan | null {
    const remembered = [...availableRecipes]
      .filter(
        (recipe) =>
          recipe.scope === "source_fingerprint" && recipe.source_fingerprint === sourceFingerprint
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
    return materializeRecipePlan(remembered);
  }

  function materializeRecipePlan(recipe: SegyImportRecipe): SegyImportPlan {
    if (!scanResponse) {
      return structuredClone(recipe.plan);
    }
    const outputStorePath =
      plan?.policy.output_store_path?.trim() || scanResponse.default_plan.policy.output_store_path;
    return {
      ...structuredClone(recipe.plan),
      input_path: scanResponse.input_path,
      source_fingerprint: scanResponse.source_fingerprint,
      policy: {
        ...structuredClone(recipe.plan.policy),
        output_store_path: outputStorePath,
        overwrite_existing: false
      },
      provenance: {
        ...structuredClone(recipe.plan.provenance),
        plan_source: recipe.scope === "source_fingerprint" ? "source_memory" : "saved_recipe",
        recipe_id: recipe.recipe_id,
        recipe_name: recipe.name
      }
    };
  }

  function wizardStageFromScan(nextStage: string): WizardStage {
    switch (nextStage) {
      case "structure":
        return "structure";
      case "spatial":
        return "spatial";
      case "review":
        return "review";
      case "import":
        return "import";
      case "raw_inspect":
        return "raw_inspect";
      default:
        return "scan";
    }
  }

  function wizardStageFromValidation(nextStage: string): WizardStage {
    switch (nextStage) {
      case "structure":
        return "structure";
      case "spatial":
        return "spatial";
      case "import":
        return "import";
      default:
        return "review";
    }
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
      void validateCurrentPlan(false);
    }, 250);
  }

  async function validateCurrentPlan(advanceStage: boolean): Promise<void> {
    if (!plan) {
      return;
    }
    clearValidationTimer();
    validating = true;
    dialogError = null;
    try {
      const response = await validateSegyImportPlan(plan);
      validation = response;
      plan = structuredClone(response.validated_plan);
      lastValidatedPlanSignature = planSignature(response.validated_plan);
      if (advanceStage) {
        stage = wizardStageFromValidation(response.recommended_next_stage);
      }
    } catch (error) {
      dialogError = errorMessage(error);
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
    target: "inline_3d" | "crossline_3d" | "third_axis",
    kind: "byte" | "type",
    value: string
  ): void {
    if (!plan) {
      return;
    }
    const current = plan.header_mapping[target];
    const nextField = nextHeaderField(current, kind, value);
    updatePlan({
      ...plan,
      provenance: {
        ...plan.provenance,
        plan_source: "manual",
        selected_candidate_id: null
      },
      header_mapping: {
        ...plan.header_mapping,
        [target]: nextField
      }
    });
  }

  function updateSpatialField(
    target: EditableFieldTarget,
    kind: "byte" | "type",
    value: string
  ): void {
    if (!plan || !isSpatialFieldTarget(target)) {
      return;
    }
    const current = plan.spatial[target];
    updatePlan({
      ...plan,
      provenance: {
        ...plan.provenance,
        plan_source: "manual",
        selected_candidate_id: null
      },
      spatial: {
        ...plan.spatial,
        [target]: nextHeaderField(current, kind, value)
      }
    });
  }

  function isSpatialFieldTarget(
    target: EditableFieldTarget
  ): target is "x_field" | "y_field" | "coordinate_scalar_field" {
    return target === "x_field" || target === "y_field" || target === "coordinate_scalar_field";
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
    field:
      | "coordinate_units"
      | "coordinate_reference_id"
      | "coordinate_reference_name",
    value: string
  ): void {
    if (!plan) {
      return;
    }
    updatePlan({
      ...plan,
      spatial: {
        ...plan.spatial,
        [field]: value
      }
    });
  }

  function applyCandidate(candidateId: string): void {
    if (!scanResponse) {
      return;
    }
    const candidate = scanResponse.candidate_plans.find((entry) => entry.candidate_id === candidateId);
    if (!candidate) {
      return;
    }
    const outputStorePath =
      plan?.policy.output_store_path?.trim() || scanResponse.default_plan.policy.output_store_path;
    updatePlan({
      ...structuredClone(candidate.plan_patch),
      input_path: scanResponse.input_path,
      source_fingerprint: scanResponse.source_fingerprint,
      policy: {
        ...structuredClone(candidate.plan_patch.policy),
        output_store_path: outputStorePath,
        acknowledge_warnings: plan?.policy.acknowledge_warnings ?? false,
        overwrite_existing: false
      }
    });
  }

  function applyRecipe(recipe: SegyImportRecipe): void {
    updatePlan(materializeRecipePlan(recipe));
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
    }
  }

  async function confirmImport(): Promise<void> {
    if (!plan || !validation || validationStale) {
      dialogError = "The import plan changed. Validate again before importing.";
      return;
    }
    if (!validation.can_import) {
      dialogError = "Resolve the remaining blocking issues before importing.";
      return;
    }

    importing = true;
    dialogError = null;
    try {
      const activePlan = plan;
      const matchingEntry =
        viewerModel.workspaceEntries.find(
          (entry) => (entry.source_path ?? "").trim() === activePlan.input_path
        ) ??
        null;
      await viewerModel.importSegySurveyPlan(validation.validated_plan, validation.validation_fingerprint, {
        entryId: matchingEntry?.entry_id ?? null,
        sourcePath: activePlan.input_path,
        sessionPipelines: matchingEntry?.session_pipelines
          ? structuredClone(matchingEntry.session_pipelines)
          : null,
        activeSessionPipelineId: matchingEntry?.active_session_pipeline_id ?? null,
        makeActive: true,
        loadSection: true,
        reuseExistingStore: true
      });
      await saveRecipe("source_fingerprint");
      handleClose();
    } catch (error) {
      dialogError = errorMessage(error);
    } finally {
      importing = false;
    }
  }

  function goToStage(nextStage: ImportConfirmationStage): void {
    if (nextStage === "scan" || nextStage === "structure" || nextStage === "spatial" || nextStage === "review" || nextStage === "import") {
      stage = nextStage;
    }
  }

  function previousStage(currentStage: WizardStage): WizardStage {
    switch (currentStage) {
      case "structure":
        return "scan";
      case "spatial":
        return "structure";
      case "review":
        return "spatial";
      case "import":
        return "review";
      default:
        return currentStage;
    }
  }

  function issuesForStep(stepKey: ImportConfirmationStage): SegyImportIssue[] {
    return currentIssues.filter((issue) => sectionsForStep(stepKey).includes(issue.section));
  }

  function currentScanNextStage(): WizardStage {
    if (!scanResponse) {
      return "scan";
    }
    const nextStage = wizardStageFromScan(scanResponse.recommended_next_stage);
    return nextStage === "scan" ? "structure" : nextStage;
  }

  function statusCardTone(): "info" | "warning" | "blocking" | "success" {
    if (dialogError) {
      return "blocking";
    }
    if (scanLoading || validating || importing) {
      return "info";
    }
    if (!scanResponse) {
      return "warning";
    }
    if (stage === "import" && canImport) {
      return "success";
    }
    const severity =
      highestSeverityForStep(stage === "import" ? "review" : stage === "raw_inspect" ? "scan" : stage);
    if (severity === "blocking") {
      return "blocking";
    }
    if (severity === "warning" || validationStale) {
      return "warning";
    }
    return "success";
  }

  function statusCardTitle(): string {
    if (dialogError) {
      return "SEG-Y scan failed";
    }
    if (scanLoading) {
      return "Scanning SEG-Y structure";
    }
    if (validating) {
      return "Validating import plan";
    }
    if (importing) {
      return "Importing survey";
    }
    if (!scanResponse) {
      return "Scan did not populate";
    }
    if (stage === "scan") {
      return currentScanNextStage() === "import"
        ? "Scan passed"
        : currentScanNextStage() === "structure"
          ? "Structure review required"
          : "Review the scanned survey";
    }
    if (stage === "structure") {
      return highestSeverityForStep("structure") === "blocking"
        ? "Confirm the geometry mapping"
        : "Structure fields are ready to review";
    }
    if (stage === "spatial") {
      return "Spatial metadata is optional";
    }
    if (stage === "review") {
      if (validationStale) {
        return "Validate the updated plan";
      }
      return canImport ? "Plan validated" : "Review remaining issues";
    }
    if (stage === "import") {
      return canImport ? "Ready to import" : "Import is blocked";
    }
    return "Inspecting raw headers";
  }

  function statusCardMessage(): string {
    if (dialogError) {
      return dialogError;
    }
    if (scanLoading) {
      return "Reading trace headers, geometry, and candidate mappings. For clean surveys this should advance automatically.";
    }
    if (validating) {
      return "Rechecking the current mapping, sparse-grid risk, and output store plan.";
    }
    if (importing) {
      return "Writing the validated SEG-Y plan into a runtime store.";
    }
    if (!scanResponse) {
      return "Retry the scan. A clean survey like F3 should populate the next stages without manual guessing.";
    }
    if (stage === "scan" && currentScanNextStage() === "import") {
      return "This survey looks like a clean dense SEG-Y. The mapped fields are populated below, and you can jump straight to import.";
    }
    if (stage === "scan" && currentScanNextStage() === "structure") {
      return (
        issuesForStep("structure")[0]?.message ??
        "The scan found a geometry issue. Review the inline and crossline mapping before importing."
      );
    }
    if (stage === "structure") {
      return (
        issuesForStep("structure")[0]?.message ??
        "The current inline, crossline, and sparse-grid policy are populated. Adjust them only if the scan looks wrong."
      );
    }
    if (stage === "spatial") {
      return (
        issuesForStep("spatial")[0]?.message ??
        "Add X/Y/scalar or CRS metadata if you need it. Otherwise continue to review."
      );
    }
    if (stage === "review") {
      return validationStale
        ? "The import plan changed. Validate again before importing."
        : (issuesForStep("review")[0]?.message ??
          "Review the resolved layout, footprint estimate, and any warnings before importing.");
    }
    if (stage === "import") {
      return canImport
        ? "Validation passed. You can import immediately or go back and inspect the populated fields."
        : (issuesForStep("review")[0]?.message ??
          "Resolve the remaining review issues before importing.");
    }
    return "This mode only inspects raw trace header observations. It does not create a runtime store.";
  }

  function primaryStatusActionLabel(): string | null {
    if (scanLoading || validating) {
      return null;
    }
    if (dialogError || !scanResponse) {
      return "Retry Scan";
    }
    switch (stage) {
      case "scan":
        return currentScanNextStage() === "import" ? "Go To Import" : "Continue";
      case "structure":
        return "Continue";
      case "spatial":
        return "Continue";
      case "review":
        return validationStale || !validation ? "Validate Plan" : "Continue";
      case "import":
        return importing ? "Importing…" : "Import";
      case "raw_inspect":
        return "Back To Structure";
      default:
        return null;
    }
  }

  function secondaryStatusActionLabel(): string | null {
    if (!scanResponse || scanLoading || validating || dialogError) {
      return null;
    }
    if (stage === "scan") {
      return currentScanNextStage() === "import" ? "Review Structure" : "Inspect Raw";
    }
    if (stage === "review" && !validationStale) {
      return "Inspect Raw";
    }
    return null;
  }

  async function handlePrimaryStatusAction(): Promise<void> {
    if (!inputPath) {
      return;
    }
    if (dialogError || !scanResponse) {
      await loadScan(inputPath);
      return;
    }
    switch (stage) {
      case "scan":
        stage = currentScanNextStage();
        return;
      case "structure":
        stage = "spatial";
        return;
      case "spatial":
        stage = "review";
        return;
      case "review":
        if (validationStale || !validation) {
          await validateCurrentPlan(true);
        } else {
          stage = "import";
        }
        return;
      case "import":
        await confirmImport();
        return;
      case "raw_inspect":
        stage = "structure";
        return;
    }
  }

  function handleSecondaryStatusAction(): void {
    if (!scanResponse) {
      return;
    }
    if (stage === "scan") {
      stage = currentScanNextStage() === "import" ? "structure" : "raw_inspect";
      return;
    }
    if (stage === "review" && !validationStale) {
      stage = "raw_inspect";
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

  function headerByte(field: SegyHeaderField | null | undefined): string {
    return field?.start_byte ? String(field.start_byte) : "";
  }

  function headerType(field: SegyHeaderField | null | undefined): SegyHeaderValueType {
    return field?.value_type ?? "i32";
  }

  function describeField(field: SegyHeaderField | null | undefined): string {
    if (!field) {
      return "unset";
    }
    return `${field.start_byte} (${field.value_type.toUpperCase()})`;
  }

  function basename(path: string): string {
    const normalized = path.replace(/\\/g, "/");
    return normalized.split("/").pop() || normalized;
  }

  function formatBytes(bytes: bigint | number): string {
    const units = ["B", "KiB", "MiB", "GiB", "TiB"];
    let value = typeof bytes === "bigint" ? Number(bytes) : bytes;
    let unitIndex = 0;
    while (value >= 1024 && unitIndex < units.length - 1) {
      value /= 1024;
      unitIndex += 1;
    }
    return unitIndex === 0 ? `${Math.round(value)} ${units[unitIndex]}` : `${value.toFixed(1)} ${units[unitIndex]}`;
  }

  function formatPercent(value: number): string {
    return `${(value * 100).toFixed(value < 0.01 ? 4 : 2)}%`;
  }

  function severityClass(issue: SegyImportIssue): string {
    return issue.severity;
  }

  function slugify(value: string): string {
    return value
      .toLowerCase()
      .replace(/[^a-z0-9]+/g, "-")
      .replace(/^-+|-+$/g, "")
      .slice(0, 64);
  }

  function errorMessage(error: unknown): string {
    return error instanceof Error ? error.message : "The SEG-Y import flow failed.";
  }
</script>

{#if open}
  <div class="segy-import-backdrop" role="presentation" onclick={handleClose}>
    <div
      class="segy-import-dialog"
      role="dialog"
      aria-modal="true"
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

      <ImportFlowStepper stage={stage} steps={flowSteps} onSelect={goToStage} />

      <section class={`segy-import-status-card ${statusCardTone()}`}>
        <div>
          <h3>{statusCardTitle()}</h3>
          <p>{statusCardMessage()}</p>
        </div>
        <div class="segy-import-actions-row">
          {#if secondaryStatusActionLabel()}
            <button class="settings-btn secondary" type="button" onclick={handleSecondaryStatusAction}>
              {secondaryStatusActionLabel()}
            </button>
          {/if}
          {#if primaryStatusActionLabel()}
            <button
              class="settings-btn primary"
              type="button"
              onclick={() => void handlePrimaryStatusAction()}
              disabled={importing || viewerModel.loading}
            >
              {primaryStatusActionLabel()}
            </button>
          {/if}
        </div>
      </section>

      {#if scanLoading}
        <p class="segy-import-status">Scanning SEG-Y layout and candidate mappings…</p>
      {:else if dialogError}
        <p class="segy-import-error">{dialogError}</p>
      {/if}

      {#if scanResponse && plan}
        <section class="segy-import-summary">
          <div>
            <span>Source</span>
            <strong>{scanResponse.trace_count.toLocaleString()} traces</strong>
          </div>
          <div>
            <span>Samples</span>
            <strong>{scanResponse.samples_per_trace.toLocaleString()} per trace</strong>
          </div>
          <div>
            <span>Format</span>
            <strong>code {scanResponse.sample_format_code} • {scanResponse.endianness}</strong>
          </div>
          <div>
            <span>Output</span>
            <strong>{plan.policy.output_store_path || "unset"}</strong>
          </div>
        </section>

        {#if stage === "scan"}
          <section class="segy-import-panel">
            <h3>Scan</h3>
            <p>This mode inspects trace headers and layout only. It does not create a runtime store.</p>
            <div class="segy-import-actions-row">
              <button class="settings-btn secondary" type="button" onclick={() => (stage = "raw_inspect")}>
                Inspect Raw
              </button>
              <button class="settings-btn primary" type="button" onclick={() => (stage = "structure")}>
                Continue
              </button>
            </div>
          </section>
        {/if}

        {#if stage === "structure"}
          <section class="segy-import-panel">
            <h3>Structure</h3>
            {#if denseGridWarning}
              <p class="segy-import-banner">{denseGridWarning.message}</p>
            {/if}

            {#if scanResponse.candidate_plans.length > 0}
              <div class="segy-import-subsection">
                <h4>Suggested mappings</h4>
                <div class="segy-import-candidate-list">
                  {#each scanResponse.candidate_plans as candidate (candidate.candidate_id)}
                    <label class="segy-import-candidate">
                      <input
                        type="radio"
                        name="segy-candidate"
                        checked={plan.provenance.selected_candidate_id === candidate.candidate_id}
                        onchange={() => applyCandidate(candidate.candidate_id)}
                      />
                      <div>
                        <strong>{candidate.label}</strong>
                        <span>
                          {candidate.resolved_dataset.classification} • {candidate.risk_summary.observed_trace_count.toLocaleString()}
                          / {candidate.risk_summary.expected_trace_count.toLocaleString()}
                        </span>
                      </div>
                    </label>
                  {/each}
                </div>
              </div>
            {/if}

            <div class="segy-import-grid">
              <label>
                <span>Inline byte</span>
                <input
                  type="number"
                  min="1"
                  value={headerByte(plan.header_mapping.inline_3d)}
                  oninput={(event) =>
                    updateHeaderMappingField("inline_3d", "byte", (event.currentTarget as HTMLInputElement).value)}
                />
              </label>
              <label>
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
              <label>
                <span>Crossline byte</span>
                <input
                  type="number"
                  min="1"
                  value={headerByte(plan.header_mapping.crossline_3d)}
                  oninput={(event) =>
                    updateHeaderMappingField("crossline_3d", "byte", (event.currentTarget as HTMLInputElement).value)}
                />
              </label>
              <label>
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
              <label>
                <span>Third-axis byte</span>
                <input
                  type="number"
                  min="1"
                  value={headerByte(plan.header_mapping.third_axis)}
                  oninput={(event) =>
                    updateHeaderMappingField("third_axis", "byte", (event.currentTarget as HTMLInputElement).value)}
                />
              </label>
              <label>
                <span>Third-axis type</span>
                <select
                  value={headerType(plan.header_mapping.third_axis)}
                  onchange={(event) =>
                    updateHeaderMappingField("third_axis", "type", (event.currentTarget as HTMLSelectElement).value)}
                >
                  <option value="i32">I32</option>
                  <option value="i16">I16</option>
                </select>
              </label>
            </div>

            <div class="segy-import-grid">
              <label>
                <span>Sparse handling</span>
                <select
                  value={plan.policy.sparse_handling}
                  onchange={(event) =>
                    updateSparseHandling((event.currentTarget as HTMLSelectElement).value)}
                >
                  <option value="block_import">Review before import</option>
                  <option value="regularize_to_dense">Regularize to dense grid</option>
                </select>
              </label>
              <label class="wide">
                <span>Runtime store path</span>
                <input
                  type="text"
                  value={plan.policy.output_store_path}
                  oninput={(event) => updateOutputPath((event.currentTarget as HTMLInputElement).value)}
                />
              </label>
            </div>
          </section>
        {/if}

        {#if stage === "spatial"}
          <section class="segy-import-panel">
            <h3>Spatial</h3>
            <div class="segy-import-grid">
              <label>
                <span>X byte</span>
                <input
                  type="number"
                  min="1"
                  value={headerByte(plan.spatial.x_field)}
                  oninput={(event) => updateSpatialField("x_field", "byte", (event.currentTarget as HTMLInputElement).value)}
                />
              </label>
              <label>
                <span>X type</span>
                <select
                  value={headerType(plan.spatial.x_field)}
                  onchange={(event) => updateSpatialField("x_field", "type", (event.currentTarget as HTMLSelectElement).value)}
                >
                  <option value="i32">I32</option>
                  <option value="i16">I16</option>
                </select>
              </label>
              <label>
                <span>Y byte</span>
                <input
                  type="number"
                  min="1"
                  value={headerByte(plan.spatial.y_field)}
                  oninput={(event) => updateSpatialField("y_field", "byte", (event.currentTarget as HTMLInputElement).value)}
                />
              </label>
              <label>
                <span>Y type</span>
                <select
                  value={headerType(plan.spatial.y_field)}
                  onchange={(event) => updateSpatialField("y_field", "type", (event.currentTarget as HTMLSelectElement).value)}
                >
                  <option value="i32">I32</option>
                  <option value="i16">I16</option>
                </select>
              </label>
              <label>
                <span>Scalar byte</span>
                <input
                  type="number"
                  min="1"
                  value={headerByte(plan.spatial.coordinate_scalar_field)}
                  oninput={(event) =>
                    updateSpatialField("coordinate_scalar_field", "byte", (event.currentTarget as HTMLInputElement).value)}
                />
              </label>
              <label>
                <span>Scalar type</span>
                <select
                  value={headerType(plan.spatial.coordinate_scalar_field)}
                  onchange={(event) =>
                    updateSpatialField("coordinate_scalar_field", "type", (event.currentTarget as HTMLSelectElement).value)}
                >
                  <option value="i32">I32</option>
                  <option value="i16">I16</option>
                </select>
              </label>
              <label>
                <span>Coordinate units</span>
                <input
                  type="text"
                  value={plan.spatial.coordinate_units ?? ""}
                  oninput={(event) =>
                    updateSpatialText("coordinate_units", (event.currentTarget as HTMLInputElement).value)}
                />
              </label>
              <label>
                <span>CRS ID</span>
                <input
                  type="text"
                  value={plan.spatial.coordinate_reference_id ?? ""}
                  oninput={(event) =>
                    updateSpatialText("coordinate_reference_id", (event.currentTarget as HTMLInputElement).value)}
                />
              </label>
              <label class="wide">
                <span>CRS name</span>
                <input
                  type="text"
                  value={plan.spatial.coordinate_reference_name ?? ""}
                  oninput={(event) =>
                    updateSpatialText("coordinate_reference_name", (event.currentTarget as HTMLInputElement).value)}
                />
              </label>
            </div>
          </section>
        {/if}

        {#if stage === "review" || stage === "import"}
          <section class="segy-import-panel">
            <div class="segy-import-panel-header">
              <h3>Review</h3>
              <button class="settings-btn secondary" type="button" onclick={() => void validateCurrentPlan(true)} disabled={validating}>
                {validating ? "Validating…" : "Validate Plan"}
              </button>
            </div>

            {#if validationStale}
              <p class="segy-import-banner">The import plan changed. Validate again before importing.</p>
            {/if}

            {#if validation}
              <div class="segy-import-review-grid">
                <div>
                  <span>Resolved mapping</span>
                  <strong>{describeField(validation.validated_plan.header_mapping.inline_3d)} / {describeField(validation.validated_plan.header_mapping.crossline_3d)}</strong>
                </div>
                <div>
                  <span>Layout</span>
                  <strong>{validation.resolved_dataset.layout}</strong>
                </div>
                <div>
                  <span>Grid</span>
                  <strong>{validation.risk_summary.observed_trace_count.toLocaleString()} / {validation.risk_summary.expected_trace_count.toLocaleString()}</strong>
                </div>
                <div>
                  <span>Completeness</span>
                  <strong>{formatPercent(validation.risk_summary.completeness_ratio)}</strong>
                </div>
                <div>
                  <span>Blow-up</span>
                  <strong>{validation.risk_summary.blowup_ratio.toFixed(2)}x</strong>
                </div>
                <div>
                  <span>Estimated store</span>
                  <strong>{formatBytes(validation.risk_summary.estimated_total_bytes)}</strong>
                </div>
              </div>
            {/if}

            {#if currentIssues.length > 0}
              <div class="segy-import-issues">
                {#each currentIssues as issue (`${issue.code}-${issue.message}`)}
                  <div class={`segy-import-issue ${severityClass(issue)}`}>
                    <strong>{issue.severity}</strong>
                    <span>{issue.message}</span>
                    {#if issue.suggested_fix}
                      <small>{issue.suggested_fix}</small>
                    {/if}
                  </div>
                {/each}
              </div>
            {/if}

            {#if validation?.requires_acknowledgement}
              <label class="segy-import-checkbox">
                <input
                  type="checkbox"
                  checked={plan.policy.acknowledge_warnings}
                  onchange={(event) =>
                    updateWarningAcknowledgement((event.currentTarget as HTMLInputElement).checked)}
                />
                <span>I understand the remaining warnings and want to continue with this import plan.</span>
              </label>
            {/if}

            <div class="segy-import-recipes">
              <div class="segy-import-panel-header">
                <h4>Recipes</h4>
              </div>
              <div class="segy-import-recipe-actions">
                <input
                  type="text"
                  value={recipeName}
                  placeholder="Recipe name"
                  oninput={(event) => (recipeName = (event.currentTarget as HTMLInputElement).value)}
                />
                <button class="settings-btn secondary" type="button" onclick={() => void saveRecipe("global")} disabled={savingRecipe}>
                  Save Recipe
                </button>
                <button class="settings-btn secondary" type="button" onclick={() => void saveRecipe("source_fingerprint")} disabled={savingRecipe || !scanResponse}>
                  Remember For This Source
                </button>
              </div>

              {#if sourceRecipes.length > 0 || globalRecipes.length > 0}
                <div class="segy-import-recipe-list">
                  {#each [...sourceRecipes, ...globalRecipes] as recipe (recipe.recipe_id)}
                    <div class="segy-import-recipe-row">
                      <div>
                        <strong>{recipe.name}</strong>
                        <span>{recipe.scope === "source_fingerprint" ? "source memory" : "saved recipe"}</span>
                      </div>
                      <div class="segy-import-actions-row">
                        <button class="settings-btn secondary" type="button" onclick={() => applyRecipe(recipe)}>
                          Apply Recipe
                        </button>
                        <button class="settings-btn secondary" type="button" onclick={() => void removeRecipe(recipe.recipe_id)}>
                          Delete Recipe
                        </button>
                      </div>
                    </div>
                  {/each}
                </div>
              {/if}
            </div>
          </section>
        {/if}

        {#if stage === "raw_inspect"}
          <section class="segy-import-panel">
            <h3>Inspect Raw</h3>
            <p>This mode inspects trace headers and layout only. It does not create a runtime store.</p>
            <div class="segy-import-review-grid">
              {#each scanResponse.field_observations as field (`${field.label}-${field.field.start_byte}`)}
                <div>
                  <span>{field.label}</span>
                  <strong>{describeField(field.field)} • {field.unique_count.toLocaleString()} unique</strong>
                </div>
              {/each}
            </div>
            <div class="segy-import-actions-row">
              <button class="settings-btn secondary" type="button" onclick={() => (stage = "structure")}>
                Back To Structure
              </button>
            </div>
          </section>
        {/if}

        <footer class="segy-import-footer">
          <div class="segy-import-actions-row">
            {#if stage !== "scan" && stage !== "raw_inspect"}
              <button class="settings-btn secondary" type="button" onclick={() => (stage = previousStage(stage))}>
                Back
              </button>
            {/if}
            {#if stage === "structure"}
              <button class="settings-btn primary" type="button" onclick={() => (stage = "spatial")}>
                Continue
              </button>
            {:else if stage === "spatial"}
              <button class="settings-btn primary" type="button" onclick={() => (stage = "review")}>
                Continue
              </button>
            {:else if stage === "review"}
              <button class="settings-btn primary" type="button" onclick={() => void handlePrimaryStatusAction()}>
                {validationStale || !validation ? "Validate Plan" : "Continue"}
              </button>
            {:else if stage === "import"}
              <button class="settings-btn primary" type="button" onclick={() => void confirmImport()} disabled={!canImport}>
                {importing ? "Importing…" : "Import"}
              </button>
            {/if}
          </div>
        </footer>
      {:else if !scanLoading && !dialogError}
        <section class="segy-import-panel">
          <h3>Scan</h3>
          <p>Run the SEG-Y scan again. This dialog should populate the stage details automatically before you choose anything.</p>
        </section>
      {/if}
    </div>
  </div>
{/if}

<style>
  .segy-import-backdrop {
    position: fixed;
    inset: 0;
    z-index: 60;
    display: grid;
    place-items: center;
    padding: 24px;
    background: rgba(6, 10, 18, 0.68);
    backdrop-filter: blur(6px);
  }

  .segy-import-dialog {
    width: min(1080px, 100%);
    max-height: min(92vh, 980px);
    overflow: auto;
    padding: 20px;
    border: 1px solid var(--app-border-strong);
    border-radius: 8px;
    background: var(--panel-bg);
    display: grid;
    gap: 16px;
  }

  .segy-import-header,
  .segy-import-panel-header,
  .segy-import-recipe-row,
  .segy-import-actions-row {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 12px;
  }

  .segy-import-header h2,
  .segy-import-panel h3,
  .segy-import-panel h4 {
    margin: 0;
  }

  .segy-import-header p,
  .segy-import-panel p {
    margin: 4px 0 0;
    color: var(--text-muted);
  }

  .segy-import-status,
  .segy-import-error,
  .segy-import-banner,
  .segy-import-status-card {
    margin: 0;
    padding: 12px 14px;
    border-radius: 6px;
  }

  .segy-import-status {
    background: color-mix(in srgb, var(--accent-solid, #4f8cff) 12%, transparent);
  }

  .segy-import-error {
    color: #ffd0d0;
    background: rgba(146, 27, 27, 0.28);
  }

  .segy-import-banner {
    color: #ffe7b8;
    background: rgba(150, 94, 0, 0.22);
  }

  .segy-import-status-card {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 16px;
    border: 1px solid var(--app-border);
    background: var(--surface-subtle);
  }

  .segy-import-status-card h3 {
    margin: 0;
  }

  .segy-import-status-card p {
    margin: 4px 0 0;
    color: var(--text-muted);
  }

  .segy-import-status-card.success {
    border-color: color-mix(in srgb, #2eae6b 38%, var(--app-border));
    background: color-mix(in srgb, rgba(46, 174, 107, 0.12) 75%, transparent);
  }

  .segy-import-status-card.warning {
    border-color: rgba(214, 154, 42, 0.5);
    background: rgba(150, 94, 0, 0.16);
  }

  .segy-import-status-card.blocking {
    border-color: rgba(220, 76, 76, 0.5);
    background: rgba(146, 27, 27, 0.18);
  }

  .segy-import-status-card.info {
    border-color: color-mix(in srgb, var(--accent-solid, #4f8cff) 35%, var(--app-border));
    background: color-mix(in srgb, var(--accent-solid, #4f8cff) 10%, transparent);
  }

  .segy-import-summary,
  .segy-import-review-grid {
    display: grid;
    gap: 12px;
    grid-template-columns: repeat(4, minmax(0, 1fr));
  }

  .segy-import-summary > div,
  .segy-import-review-grid > div {
    display: grid;
    gap: 4px;
    padding: 12px;
    border: 1px solid var(--app-border);
    border-radius: 6px;
    background: var(--surface-subtle);
  }

  .segy-import-summary span,
  .segy-import-review-grid span,
  .segy-import-candidate span,
  .segy-import-recipe-row span {
    color: var(--text-muted);
    font-size: 0.9rem;
  }

  .segy-import-panel {
    display: grid;
    gap: 14px;
  }

  .segy-import-subsection,
  .segy-import-recipes,
  .segy-import-issues {
    display: grid;
    gap: 10px;
  }

  .segy-import-grid {
    display: grid;
    gap: 12px;
    grid-template-columns: repeat(2, minmax(0, 1fr));
  }

  .segy-import-grid label,
  .segy-import-candidate,
  .segy-import-checkbox {
    display: grid;
    gap: 6px;
  }

  .segy-import-grid label.wide {
    grid-column: 1 / -1;
  }

  .segy-import-grid input,
  .segy-import-grid select,
  .segy-import-recipe-actions input {
    min-width: 0;
    padding: 10px 12px;
    border: 1px solid var(--app-border);
    border-radius: 6px;
    background: var(--surface-subtle);
    color: inherit;
    font: inherit;
  }

  .segy-import-candidate-list,
  .segy-import-recipe-list {
    display: grid;
    gap: 10px;
  }

  .segy-import-candidate,
  .segy-import-recipe-row,
  .segy-import-issue {
    padding: 12px;
    border: 1px solid var(--app-border);
    border-radius: 6px;
    background: var(--surface-subtle);
  }

  .segy-import-candidate {
    grid-template-columns: auto minmax(0, 1fr);
    align-items: start;
  }

  .segy-import-issue {
    display: grid;
    gap: 4px;
  }

  .segy-import-issue.blocking {
    border-color: rgba(220, 76, 76, 0.5);
  }

  .segy-import-issue.warning {
    border-color: rgba(214, 154, 42, 0.5);
  }

  .segy-import-recipe-actions {
    display: grid;
    gap: 10px;
    grid-template-columns: minmax(0, 1fr) auto auto;
  }

  .segy-import-checkbox {
    grid-template-columns: auto minmax(0, 1fr);
    align-items: start;
  }

  .segy-import-footer {
    display: flex;
    justify-content: flex-end;
  }

  @media (max-width: 900px) {
    .segy-import-summary,
    .segy-import-review-grid,
    .segy-import-grid,
    .segy-import-recipe-actions {
      grid-template-columns: 1fr;
    }

    .segy-import-recipe-row,
    .segy-import-header,
    .segy-import-panel-header,
    .segy-import-actions-row,
    .segy-import-status-card {
      align-items: stretch;
      flex-direction: column;
    }
  }
</style>
