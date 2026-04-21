<svelte:options runes={true} />

<script lang="ts">
  import type {
    ProjectAssetBindingInput,
    VendorProjectCommitResponse,
    VendorProjectObjectPreview,
    VendorProjectPlanResponse,
    VendorProjectScanResponse
  } from "../bridge";
  import {
    commitVendorProjectImport,
    planVendorProjectImport,
    scanVendorProject
  } from "../bridge";
  import { pickVendorProjectFolder } from "../file-dialog";
  import { getViewerModelContext } from "../viewer-model.svelte";

  interface Props {
    openSettings: () => void;
    close: () => void;
  }

  let { openSettings, close }: Props = $props();

  const viewerModel = getViewerModelContext();
  let vendorProjectRoot = $state("");
  let selectedVendorObjectIds = $state<string[]>([]);
  let targetSurveyAssetId = $state(viewerModel.projectSurveyAssetId.trim());
  let scanResponse = $state.raw<VendorProjectScanResponse | null>(null);
  let planResponse = $state.raw<VendorProjectPlanResponse | null>(null);
  let commitResponse = $state.raw<VendorProjectCommitResponse | null>(null);
  let scanLoading = $state(false);
  let planLoading = $state(false);
  let commitLoading = $state(false);
  let error = $state<string | null>(null);

  const targetProjectRoot = $derived(viewerModel.projectRoot.trim());
  const selectedProjectSurvey = $derived(viewerModel.selectedProjectSurveyAsset);
  const selectedWellBinding = $derived.by<ProjectAssetBindingInput | null>(() => {
    const wellbore = viewerModel.selectedProjectWellboreInventoryItem;
    if (!wellbore) {
      return null;
    }
    return {
      well_name: wellbore.wellName,
      wellbore_name: wellbore.wellboreName,
      operator_aliases: []
    };
  });
  const selectedObjects = $derived.by<VendorProjectObjectPreview[]>(() => {
    return (
      scanResponse?.objects.filter((object) => selectedVendorObjectIds.includes(object.vendorObjectId)) ?? []
    );
  });
  const requiresTargetSurveyAsset = $derived.by(
    () =>
      planResponse?.targetSurveyAssetRequired ??
      selectedObjects.some(
        (object) =>
          object.canonicalTargetKind === "survey_store_horizon" &&
          object.disposition !== "raw_source_only"
      )
  );
  const requiresExplicitBinding = $derived.by(
    () =>
      planResponse?.plannedImports.some(
        (planned) =>
          planned.disposition !== "raw_source_only" &&
          planned.canonicalTargetKind !== "survey_store_horizon"
      ) ??
      selectedObjects.some(
        (object) =>
          object.disposition !== "raw_source_only" &&
          object.canonicalTargetKind !== "survey_store_horizon"
      )
  );
  const planWarnings = $derived(planResponse?.warnings ?? []);
  const planBlockingIssues = $derived(planResponse?.blockingIssues ?? []);
  const commitIssues = $derived(commitResponse?.issues ?? []);
  const commitBlockers = $derived.by(() => {
    const blockers: string[] = [];
    if (!scanResponse) {
      blockers.push("Scan a Petrel export root before planning or importing.");
      return blockers;
    }
    if (!targetProjectRoot) {
      blockers.push("Set the Ophiolite project root before committing imported objects.");
    }
    if (requiresTargetSurveyAsset && !targetSurveyAssetId.trim()) {
      blockers.push("Choose a target survey asset for Petrel horizon imports.");
    }
    if (requiresExplicitBinding && !selectedWellBinding) {
      blockers.push("Choose a target project wellbore before committing well-bound Petrel assets.");
    }
    if (!planResponse) {
      blockers.push("Run the import plan before committing.");
    } else if (planBlockingIssues.length > 0) {
      blockers.push("Resolve the blocking plan issues before committing.");
    }
    return blockers;
  });

  function resetPlanAndCommit(): void {
    planResponse = null;
    commitResponse = null;
    error = null;
  }

  function formatObjectKind(value: string): string {
    return value.replace(/_/g, " ");
  }

  function formatDisposition(value: string): string {
    return value.replace(/_/g, " ");
  }

  function toggleObjectSelection(vendorObjectId: string, checked: boolean): void {
    selectedVendorObjectIds = checked
      ? [...selectedVendorObjectIds.filter((value) => value !== vendorObjectId), vendorObjectId]
      : selectedVendorObjectIds.filter((value) => value !== vendorObjectId);
    resetPlanAndCommit();
  }

  async function browseVendorProjectRoot(): Promise<void> {
    const picked = await pickVendorProjectFolder("Select Petrel Export Root");
    if (!picked) {
      return;
    }
    vendorProjectRoot = picked;
    scanResponse = null;
    resetPlanAndCommit();
  }

  async function handleScan(): Promise<void> {
    const projectRoot = vendorProjectRoot.trim();
    if (!projectRoot) {
      error = "Choose a Petrel export root before scanning.";
      return;
    }

    scanLoading = true;
    error = null;
    planResponse = null;
    commitResponse = null;
    try {
      const response = await scanVendorProject({
        vendor: "petrel",
        projectRoot
      });
      scanResponse = response;
      selectedVendorObjectIds = response.objects
        .filter((object) => object.defaultSelected)
        .map((object) => object.vendorObjectId);
      if (!targetSurveyAssetId.trim()) {
        targetSurveyAssetId = viewerModel.projectSurveyAssetId.trim();
      }
    } catch (scanError) {
      scanResponse = null;
      selectedVendorObjectIds = [];
      error = scanError instanceof Error ? scanError.message : "Petrel scan failed.";
    } finally {
      scanLoading = false;
    }
  }

  async function handlePlan(): Promise<void> {
    if (!scanResponse) {
      error = "Scan a Petrel export root before planning.";
      return;
    }

    planLoading = true;
    error = null;
    commitResponse = null;
    try {
      const response = await planVendorProjectImport({
        vendor: "petrel",
        projectRoot: scanResponse.projectRoot,
        selectedVendorObjectIds,
        targetProjectRoot: targetProjectRoot || null,
        targetSurveyAssetId: targetSurveyAssetId.trim() || null,
        binding: selectedWellBinding ?? null,
        coordinateReference: scanResponse.surveyMetadata.coordinateReference ?? null
      });
      planResponse = response;
      if (!targetSurveyAssetId.trim()) {
        targetSurveyAssetId = response.selectedTargetSurveyAsset?.asset_id ?? targetSurveyAssetId;
      }
    } catch (planError) {
      planResponse = null;
      error = planError instanceof Error ? planError.message : "Petrel planning failed.";
    } finally {
      planLoading = false;
    }
  }

  async function handleCommit(): Promise<void> {
    if (commitBlockers.length > 0 || !planResponse) {
      error = commitBlockers[0] ?? "Run the plan before committing.";
      return;
    }

    commitLoading = true;
    error = null;
    try {
      const response = await commitVendorProjectImport({
        plan: planResponse,
        targetProjectRoot,
        targetSurveyAssetId: targetSurveyAssetId.trim() || null,
        binding: selectedWellBinding ?? null,
        coordinateReference: scanResponse?.surveyMetadata.coordinateReference ?? null,
        bridgeOutputs: [],
        dryRun: false
      });
      commitResponse = response;
      if (targetProjectRoot === viewerModel.projectRoot.trim()) {
        await viewerModel.refreshProjectWellOverlayInventory(
          targetProjectRoot,
          viewerModel.displayCoordinateReferenceId
        );
        if (targetSurveyAssetId.trim()) {
          await viewerModel.refreshProjectSurveyHorizons(targetProjectRoot, targetSurveyAssetId.trim());
        }
      }
      viewerModel.note(
        "Imported Petrel objects into the active project.",
        "backend",
        "info",
        `${response.importedAssets.length} canonical asset group(s)`
      );
    } catch (commitError) {
      commitResponse = null;
      error = commitError instanceof Error ? commitError.message : "Petrel import commit failed.";
    } finally {
      commitLoading = false;
    }
  }
</script>

<div
  class="vendor-import-backdrop"
  role="presentation"
  tabindex="-1"
  onclick={close}
  onkeydown={(event) => {
    if (event.key === "Escape") {
      close();
    }
  }}
>
  <div
    class="vendor-import-dialog"
    role="dialog"
    tabindex="0"
    aria-modal="true"
    aria-label="Import Petrel project exports"
    onclick={(event) => event.stopPropagation()}
    onkeydown={(event) => event.stopPropagation()}
  >
    <header class="vendor-import-header">
      <div>
        <h2>Petrel Import</h2>
        <p>Scan a Petrel export bundle, review the canonical plan, then commit into the active Ophiolite project.</p>
      </div>
      <button class="close-button" type="button" onclick={close} aria-label="Close Petrel import dialog">
        &times;
      </button>
    </header>

    <section class="vendor-import-section">
      <div class="field-row">
        <label class="field grow">
          <span>Petrel Export Root</span>
          <input
            bind:value={vendorProjectRoot}
            type="text"
            placeholder="/path/to/Petrel Data"
            disabled={scanLoading || planLoading || commitLoading}
          />
        </label>
        <button class="secondary" type="button" onclick={() => void browseVendorProjectRoot()}>
          Browse...
        </button>
        <button
          class="primary"
          type="button"
          onclick={() => void handleScan()}
          disabled={scanLoading || !vendorProjectRoot.trim()}
        >
          {scanLoading ? "Scanning..." : "Scan"}
        </button>
      </div>

      {#if scanResponse}
        <div class="summary-grid">
          <div class="summary-card">
            <strong>Vendor Project</strong>
            <span>{scanResponse.vendorProject ?? "Unnamed Petrel export"}</span>
          </div>
          <div class="summary-card">
            <strong>Objects</strong>
            <span>{scanResponse.objects.length}</span>
          </div>
          <div class="summary-card">
            <strong>Survey CRS</strong>
            <span>{scanResponse.surveyMetadata.coordinateReference?.id ?? "Unresolved"}</span>
          </div>
        </div>
      {/if}
    </section>

    <section class="vendor-import-section">
      <div class="section-heading">
        <h3>Selected Objects</h3>
        {#if scanResponse}
          <span>{selectedVendorObjectIds.length} selected</span>
        {/if}
      </div>

      {#if scanResponse}
        <div class="object-list">
          {#each scanResponse.objects as object (object.vendorObjectId)}
            <label class="object-row">
              <input
                type="checkbox"
                checked={selectedVendorObjectIds.includes(object.vendorObjectId)}
                onchange={(event) =>
                  toggleObjectSelection(object.vendorObjectId, (event.currentTarget as HTMLInputElement).checked)}
                disabled={planLoading || commitLoading}
              />
              <div class="object-copy">
                <strong>{object.displayName}</strong>
                <span>
                  {formatObjectKind(object.vendorKind)} | {formatObjectKind(object.canonicalTargetKind)} | {formatDisposition(object.disposition)}
                </span>
                {#if object.notes.length}
                  <small>{object.notes[0]}</small>
                {/if}
              </div>
            </label>
          {/each}
        </div>
      {:else}
        <p class="empty-state">Choose a Petrel export root and run a scan to inspect the available objects.</p>
      {/if}
    </section>

    <section class="vendor-import-section">
      <div class="section-heading">
        <h3>Target Project</h3>
        <button class="secondary" type="button" onclick={openSettings}>
          Project Settings...
        </button>
      </div>

      <div class="summary-grid">
        <div class="summary-card">
          <strong>Project Root</strong>
          <span>{targetProjectRoot || "Not configured"}</span>
        </div>
        <div class="summary-card">
          <strong>Selected Wellbore</strong>
          <span>
            {selectedWellBinding ? `${selectedWellBinding.well_name} | ${selectedWellBinding.wellbore_name}` : "Not selected"}
          </span>
        </div>
        <div class="summary-card">
          <strong>Current Survey</strong>
          <span>{selectedProjectSurvey ? selectedProjectSurvey.name : "Not selected"}</span>
        </div>
      </div>

      {#if requiresTargetSurveyAsset}
        <label class="field">
          <span>Target Survey Asset</span>
          <select bind:value={targetSurveyAssetId} disabled={planLoading || commitLoading}>
            <option value="">Choose a survey asset</option>
            {#each viewerModel.projectSurveyAssets as survey (survey.assetId)}
              <option value={survey.assetId}>{viewerModel.projectSurveyOptionLabel(survey)}</option>
            {/each}
          </select>
        </label>
      {/if}
    </section>

    <section class="vendor-import-section">
      <div class="section-heading">
        <h3>Plan</h3>
        <button
          class="primary"
          type="button"
          onclick={() => void handlePlan()}
          disabled={!scanResponse || planLoading || selectedVendorObjectIds.length === 0}
        >
          {planLoading ? "Planning..." : "Plan Import"}
        </button>
      </div>

      {#if planBlockingIssues.length}
        <div class="issue-list blocking">
          {#each planBlockingIssues as issue (`blocking-${issue.code}-${issue.message}`)}
            <p>{issue.message}</p>
          {/each}
        </div>
      {/if}

      {#if planWarnings.length}
        <div class="issue-list warning">
          {#each planWarnings as issue (`warning-${issue.code}-${issue.message}`)}
            <p>{issue.message}</p>
          {/each}
        </div>
      {/if}

      {#if scanResponse?.issues.length}
        <div class="issue-list info">
          {#each scanResponse.issues as issue (`scan-${issue.code}-${issue.message}`)}
            <p>{issue.message}</p>
          {/each}
        </div>
      {/if}

      {#if planResponse}
        <div class="plan-list">
          {#each planResponse.plannedImports as planned (planned.vendorObjectId)}
            <div class="plan-row">
              <strong>{planned.displayName}</strong>
              <span>
                {formatObjectKind(planned.canonicalTargetKind)} | {formatDisposition(planned.disposition)}
                {planned.requiresTargetSurveyAsset ? " | survey target required" : ""}
              </span>
            </div>
          {/each}
        </div>
      {/if}
    </section>

    <section class="vendor-import-section">
      <div class="section-heading">
        <h3>Commit</h3>
        <button
          class="primary"
          type="button"
          onclick={() => void handleCommit()}
          disabled={commitLoading || commitBlockers.length > 0}
        >
          {commitLoading ? "Importing..." : "Commit Import"}
        </button>
      </div>

      {#if commitBlockers.length}
        <div class="issue-list blocking">
          {#each commitBlockers as blocker (blocker)}
            <p>{blocker}</p>
          {/each}
        </div>
      {/if}

      {#if commitIssues.length}
        <div class="issue-list info">
          {#each commitIssues as issue (`commit-${issue.code}-${issue.message}`)}
            <p>{issue.message}</p>
          {/each}
        </div>
      {/if}

      {#if commitResponse}
        <div class="summary-grid">
          <div class="summary-card">
            <strong>Imported</strong>
            <span>{commitResponse.importedAssets.length}</span>
          </div>
          <div class="summary-card">
            <strong>Preserved Raw</strong>
            <span>{commitResponse.preservedRawSources.length}</span>
          </div>
          <div class="summary-card">
            <strong>Validation Reports</strong>
            <span>{commitResponse.validationReports.length}</span>
          </div>
        </div>

        <div class="plan-list">
          {#each commitResponse.validationReports as report (report.vendorObjectId)}
            <div class="plan-row">
              <strong>{report.displayName}</strong>
              <span>{report.checks.join(" | ")}</span>
            </div>
          {/each}
        </div>
      {/if}
    </section>

    {#if error}
      <p class="error-banner">{error}</p>
    {/if}
  </div>
</div>

<style>
  .vendor-import-backdrop {
    position: fixed;
    inset: 0;
    z-index: 30;
    display: flex;
    align-items: center;
    justify-content: center;
    padding: 24px;
    background: rgba(8, 12, 18, 0.7);
    backdrop-filter: blur(3px);
  }

  .vendor-import-dialog {
    width: min(1040px, 100%);
    max-height: calc(100vh - 48px);
    overflow: auto;
    border: 1px solid var(--app-border-strong);
    border-radius: 8px;
    background: var(--panel-bg);
    color: var(--text-primary);
    box-shadow: 0 28px 60px rgba(0, 0, 0, 0.35);
  }

  .vendor-import-header {
    display: flex;
    align-items: flex-start;
    justify-content: space-between;
    gap: 16px;
    padding: 20px 20px 16px;
    border-bottom: 1px solid var(--app-border);
  }

  .vendor-import-header h2,
  .section-heading h3 {
    margin: 0;
    font-size: 16px;
    line-height: 1.25;
  }

  .vendor-import-header p {
    margin: 6px 0 0;
    color: var(--text-muted);
    font-size: 13px;
    line-height: 1.45;
  }

  .close-button,
  .primary,
  .secondary {
    border-radius: 8px;
    border: 1px solid var(--app-border-strong);
    min-height: 36px;
    padding: 0 12px;
    font: inherit;
    cursor: pointer;
  }

  .close-button,
  .secondary {
    background: var(--surface-subtle);
    color: var(--text-primary);
  }

  .primary {
    background: #1d8f73;
    color: #f7fffb;
    border-color: #1d8f73;
  }

  .close-button {
    width: 36px;
    padding: 0;
    font-size: 20px;
    line-height: 1;
  }

  .vendor-import-section {
    display: flex;
    flex-direction: column;
    gap: 12px;
    padding: 16px 20px;
    border-bottom: 1px solid var(--app-border);
  }

  .section-heading {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 12px;
  }

  .section-heading span {
    color: var(--text-muted);
    font-size: 12px;
  }

  .field-row {
    display: grid;
    grid-template-columns: minmax(0, 1fr) auto auto;
    gap: 10px;
    align-items: end;
  }

  .field {
    display: flex;
    flex-direction: column;
    gap: 6px;
    min-width: 0;
  }

  .field.grow {
    min-width: 0;
  }

  .field span {
    font-size: 12px;
    color: var(--text-muted);
  }

  .field input,
  .field select {
    width: 100%;
    min-height: 38px;
    padding: 8px 10px;
    border-radius: 8px;
    border: 1px solid var(--app-border-strong);
    background: var(--surface-bg);
    color: var(--text-primary);
    font: inherit;
  }

  .summary-grid {
    display: grid;
    grid-template-columns: repeat(3, minmax(0, 1fr));
    gap: 10px;
  }

  .summary-card {
    display: flex;
    flex-direction: column;
    gap: 4px;
    padding: 12px;
    border: 1px solid var(--app-border);
    border-radius: 8px;
    background: var(--surface-bg);
    min-width: 0;
  }

  .summary-card strong {
    font-size: 12px;
  }

  .summary-card span {
    color: var(--text-muted);
    font-size: 12px;
    line-height: 1.4;
    overflow-wrap: anywhere;
  }

  .object-list,
  .plan-list {
    display: flex;
    flex-direction: column;
    gap: 8px;
    max-height: 220px;
    overflow: auto;
  }

  .object-row,
  .plan-row {
    display: grid;
    grid-template-columns: auto minmax(0, 1fr);
    gap: 10px;
    padding: 10px 12px;
    border: 1px solid var(--app-border);
    border-radius: 8px;
    background: var(--surface-bg);
    align-items: start;
  }

  .plan-row {
    grid-template-columns: minmax(0, 1fr);
  }

  .object-copy,
  .plan-row {
    display: flex;
    flex-direction: column;
    gap: 4px;
    min-width: 0;
  }

  .object-copy strong,
  .plan-row strong {
    font-size: 13px;
    overflow-wrap: anywhere;
  }

  .object-copy span,
  .object-copy small,
  .plan-row span {
    color: var(--text-muted);
    font-size: 12px;
    line-height: 1.4;
    overflow-wrap: anywhere;
  }

  .issue-list {
    display: flex;
    flex-direction: column;
    gap: 6px;
    padding: 10px 12px;
    border: 1px solid var(--app-border);
    border-radius: 8px;
    background: var(--surface-bg);
  }

  .issue-list p,
  .empty-state,
  .error-banner {
    margin: 0;
    font-size: 12px;
    line-height: 1.45;
    overflow-wrap: anywhere;
  }

  .issue-list.blocking {
    border-color: rgba(239, 68, 68, 0.45);
    background: rgba(127, 29, 29, 0.16);
  }

  .issue-list.warning {
    border-color: rgba(245, 158, 11, 0.45);
    background: rgba(120, 53, 15, 0.16);
  }

  .issue-list.info {
    border-color: rgba(59, 130, 246, 0.35);
  }

  .empty-state {
    color: var(--text-muted);
  }

  .error-banner {
    margin: 0 20px 20px;
    padding: 10px 12px;
    border: 1px solid rgba(239, 68, 68, 0.45);
    border-radius: 8px;
    background: rgba(127, 29, 29, 0.16);
  }

  @media (max-width: 900px) {
    .field-row,
    .summary-grid {
      grid-template-columns: 1fr;
    }
  }
</style>
