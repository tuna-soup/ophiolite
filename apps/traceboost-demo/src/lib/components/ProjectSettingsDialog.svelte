<svelte:options runes={true} />

<script lang="ts">
  import { onMount } from "svelte";
  import type { CoordinateReferenceCatalogEntry, CoordinateReferenceSelection } from "../bridge";
  import { emitFrontendDiagnosticsEvent, resolveCoordinateReference } from "../bridge";
  import { describeDatasetCoordinateMaterializationAvailability } from "../dataset-coordinate-materialization";
  import { pickProjectFolder } from "../file-dialog";
  import { buildStartupSetupBlockers } from "../startup-setup";
  import { getViewerModelContext } from "../viewer-model.svelte";
  import CoordinateReferencePicker from "./CoordinateReferencePicker.svelte";

  interface Props {
    close: () => void;
    dismissible?: boolean;
  }

  type SettingsSection = "coordinate_systems" | "project_wells" | "section_overlays";
  type CoordinateReferencePickerTarget = "project_display" | "native_override" | null;

  let { close, dismissible = true }: Props = $props();

  const viewerModel = getViewerModelContext();

  let activeSection = $state<SettingsSection>("coordinate_systems");
  let crsPickerTarget = $state<CoordinateReferencePickerTarget>(null);
  let displayMode = $state<"native_engineering" | "authority_code">(
    viewerModel.projectDisplayCoordinateReferenceMode
  );
  let displayCoordinateReferenceId = $state(viewerModel.projectDisplayCoordinateReferenceIdDraft);
  let displayCoordinateReferenceName = $state("");
  let nativeOverrideId = $state(viewerModel.nativeCoordinateReferenceOverrideIdDraft);
  let nativeOverrideName = $state(viewerModel.nativeCoordinateReferenceOverrideNameDraft);
  let projectRoot = $state(viewerModel.projectRoot);
  let surveyAssetId = $state(viewerModel.projectSurveyAssetId);
  let wellboreId = $state(viewerModel.projectWellboreId);
  let toleranceM = $state(String(viewerModel.projectSectionToleranceM));
  let suggestedProjectDisplayEntry = $state.raw<CoordinateReferenceCatalogEntry | null>(null);
  let nativeOverrideFeedback = $state<{ level: "error" | "info"; message: string } | null>(null);

  const displayCoordinateReferenceSummary = $derived.by(() =>
    displayMode === "native_engineering"
      ? "Native engineering coordinates"
      : formatCoordinateReferenceLabel(displayCoordinateReferenceId, displayCoordinateReferenceName) ||
        (projectRoot.trim() ? "Choose a validated project CRS" : "Choose a validated session CRS")
  );
  const applyProjectDisplayLabel = $derived.by(() =>
    projectRoot.trim() ? "Apply Project CRS" : "Apply Session CRS"
  );
  const nativeOverrideSummary = $derived.by(
    () =>
      formatCoordinateReferenceLabel(nativeOverrideId, nativeOverrideName) ||
      "No survey CRS assigned"
  );
  const datasetCoordinateMaterializationAvailability = $derived.by(() =>
    describeDatasetCoordinateMaterializationAvailability({
      hasActiveDataset: !!viewerModel.comparePrimaryDataset,
      displayCoordinateReferenceId: viewerModel.displayCoordinateReferenceId,
      activeEffectiveNativeCoordinateReferenceId:
        viewerModel.activeEffectiveNativeCoordinateReferenceId,
      activeSurveyMapTransformStatus: viewerModel.activeSurveyMapSurvey?.transform_status ?? null
    })
  );
  const projectDisplayRecommendedEntries = $derived.by<CoordinateReferenceCatalogEntry[]>(() =>
    suggestedProjectDisplayEntry ? [suggestedProjectDisplayEntry] : []
  );
  const remainingStartupSetupBlockers = $derived.by(() =>
    buildStartupSetupBlockers({
      workspaceReady: viewerModel.workspaceReady,
      hasProjectRoot: viewerModel.projectRoot.trim().length > 0,
      projectGeospatialSettingsResolved: viewerModel.projectGeospatialSettingsResolved,
      hasActiveStore: viewerModel.activeStorePath.trim().length > 0,
      activeEffectiveNativeCoordinateReferenceId:
        viewerModel.activeEffectiveNativeCoordinateReferenceId,
      activeEffectiveNativeCoordinateReferenceName:
        viewerModel.activeEffectiveNativeCoordinateReferenceName
    })
  );
  const projectWorkflowAdvisory = $derived.by(() =>
    viewerModel.projectRoot.trim()
      ? null
      : "Project root is optional for section viewing and processing. Set it before using project wells, overlays, or project map composition."
  );
  const projectDisplaySourceLabel = $derived.by(() => {
    switch (viewerModel.projectGeospatialSettingsSource) {
      case "temporary_workspace":
        return "Temporary workspace";
      case "legacy_session":
        return "Legacy session";
      case "default_native_engineering":
        return "Default native engineering";
      default:
        return viewerModel.projectGeospatialSettingsSource ?? "Unresolved";
    }
  });

  onMount(() => {
    emitSettingsDialogDiagnostics("Project settings dialog mounted.");
    void refreshCoordinateReferenceHints();
  });

  function formatCoordinateReferenceLabel(
    coordinateReferenceId: string | null | undefined,
    coordinateReferenceName: string | null | undefined
  ): string {
    const normalizedId = coordinateReferenceId?.trim() ?? "";
    const normalizedName = coordinateReferenceName?.trim() ?? "";
    if (normalizedId && normalizedName) {
      return `${normalizedId} (${normalizedName})`;
    }
    if (normalizedId) {
      return normalizedId;
    }
    if (normalizedName) {
      return normalizedName;
    }
    return "";
  }

  async function resolveCatalogEntry(
    authId: string | null | undefined
  ): Promise<CoordinateReferenceCatalogEntry | null> {
    const normalizedAuthId = authId?.trim() ?? "";
    if (!normalizedAuthId) {
      return null;
    }
    try {
      return await resolveCoordinateReference({ authId: normalizedAuthId });
    } catch {
      return null;
    }
  }

  async function refreshCoordinateReferenceHints(): Promise<void> {
    const [displayEntry, suggestedEntry] = await Promise.all([
      resolveCatalogEntry(displayCoordinateReferenceId),
      resolveCatalogEntry(viewerModel.suggestedProjectDisplayCoordinateReferenceId)
    ]);
    displayCoordinateReferenceName = displayEntry?.name ?? "";
    suggestedProjectDisplayEntry = suggestedEntry;
  }

  function settingsDialogDiagnosticsFields(
    fields: Record<string, unknown> | null = null
  ): Record<string, unknown> {
    return {
      activeSection,
      dismissible,
      blockers: remainingStartupSetupBlockers,
      blockerCount: remainingStartupSetupBlockers.length,
      hasProjectRoot: viewerModel.projectRoot.trim().length > 0,
      projectGeospatialSettingsResolved: viewerModel.projectGeospatialSettingsResolved,
      projectGeospatialSettingsSource: viewerModel.projectGeospatialSettingsSource,
      hasActiveStore: viewerModel.activeStorePath.trim().length > 0,
      activeEffectiveNativeCoordinateReferenceId:
        viewerModel.activeEffectiveNativeCoordinateReferenceId,
      activeEffectiveNativeCoordinateReferenceName:
        viewerModel.activeEffectiveNativeCoordinateReferenceName,
      ...fields
    };
  }

  function emitSettingsDialogDiagnostics(
    message: string,
    fields: Record<string, unknown> | null = null
  ): void {
    void emitFrontendDiagnosticsEvent({
      stage: "settings_dialog",
      level: "debug",
      message,
      fields: settingsDialogDiagnosticsFields(fields)
    }).catch(() => {});
  }

  function closeDialog(): void {
    if (!dismissible) {
      return;
    }
    close();
  }

  function handleBackdropClick(event: MouseEvent): void {
    if (dismissible && event.target === event.currentTarget) {
      closeDialog();
    }
  }

  function openCoordinateReferencePicker(target: Exclude<CoordinateReferencePickerTarget, null>): void {
    crsPickerTarget = target;
  }

  function closeCoordinateReferencePicker(): void {
    crsPickerTarget = null;
  }

  function handleCoordinateReferenceSelection(selection: CoordinateReferenceSelection): void {
    if (crsPickerTarget === "project_display") {
      if (selection.kind !== "authority_code") {
        return;
      }
      displayMode = "authority_code";
      displayCoordinateReferenceId = selection.authId;
      displayCoordinateReferenceName = selection.name?.trim() ?? "";
      closeCoordinateReferencePicker();
      return;
    }

    if (crsPickerTarget === "native_override") {
      if (selection.kind === "authority_code") {
        nativeOverrideId = selection.authId;
        nativeOverrideName = selection.name?.trim() ?? "";
      } else if (selection.kind === "local_engineering") {
        nativeOverrideId = "";
        nativeOverrideName = selection.label.trim();
      }
      closeCoordinateReferencePicker();
    }
  }

  async function handlePickProjectRoot(): Promise<void> {
    const nextProjectRoot = await pickProjectFolder();
    if (!nextProjectRoot) {
      return;
    }
    projectRoot = nextProjectRoot;
    await viewerModel.setProjectRoot(projectRoot);
    syncDraftsFromModel();
    emitSettingsDialogDiagnostics("Applied project root from folder picker.", {
      action: "apply_project_root",
      source: "browse"
    });
  }

  async function applyProjectRoot(): Promise<void> {
    await viewerModel.setProjectRoot(projectRoot);
    syncDraftsFromModel();
    emitSettingsDialogDiagnostics("Applied project root from text input.", {
      action: "apply_project_root",
      source: "text_input"
    });
  }

  async function applyProjectDisplaySettings(): Promise<void> {
    viewerModel.setProjectDisplayCoordinateReferenceMode(displayMode);

    if (displayMode === "native_engineering") {
      await viewerModel.saveProjectDisplaySettings("user_selected", {
        kind: "native_engineering"
      });
      syncDraftsFromModel();
      emitSettingsDialogDiagnostics("Applied native-engineering display mode.", {
        action: "apply_display_crs",
        displayMode
      });
      return;
    }

    const resolvedEntry = await resolveCatalogEntry(displayCoordinateReferenceId);
    if (!resolvedEntry) {
      viewerModel.note("Choose a valid project CRS before applying it.", "ui", "warn");
      return;
    }

    displayCoordinateReferenceId = resolvedEntry.authId;
    displayCoordinateReferenceName = resolvedEntry.name;
    await viewerModel.saveProjectDisplaySettings("user_selected", {
      kind: "authority_code",
      authority: resolvedEntry.authority,
      code: resolvedEntry.code,
      authId: resolvedEntry.authId,
      name: resolvedEntry.name
    });
    syncDraftsFromModel();
    emitSettingsDialogDiagnostics("Applied authority display CRS.", {
      action: "apply_display_crs",
      displayMode,
      displayCoordinateReferenceId: resolvedEntry.authId
    });
  }

  async function applySuggestedProjectDisplaySettings(): Promise<void> {
    const suggestedEntry =
      suggestedProjectDisplayEntry ??
      (await resolveCatalogEntry(viewerModel.suggestedProjectDisplayCoordinateReferenceId));
    if (!suggestedEntry) {
      return;
    }
    await viewerModel.saveProjectDisplaySettings("user_selected", {
      kind: "authority_code",
      authority: suggestedEntry.authority,
      code: suggestedEntry.code,
      authId: suggestedEntry.authId,
      name: suggestedEntry.name
    });
    syncDraftsFromModel();
    emitSettingsDialogDiagnostics("Applied suggested survey CRS as display CRS.", {
      action: "apply_suggested_display_crs",
      displayCoordinateReferenceId: suggestedEntry.authId
    });
  }

  async function applyNativeOverride(): Promise<void> {
    const normalizedId = nativeOverrideId.trim();
    const normalizedName = nativeOverrideName.trim();
    if (!normalizedId && !normalizedName) {
      viewerModel.note("Choose a survey CRS before assigning it.", "ui", "warn");
      return;
    }
    const result = await viewerModel.setActiveDatasetNativeCoordinateReference(
      normalizedId || null,
      normalizedName || null
    );
    syncDraftsFromModel();
    nativeOverrideFeedback = result.exactMatch
      ? {
          level: "info",
          message:
            result.effectiveCoordinateReferenceId ??
            result.effectiveCoordinateReferenceName ??
            "Survey CRS assigned."
        }
      : {
          level: "error",
          message: result.error
            ? `TraceBoost could not assign the requested survey CRS. ${result.error}`
            : `TraceBoost applied ${
                result.effectiveCoordinateReferenceId ??
                result.effectiveCoordinateReferenceName ??
                "an unknown CRS"
              } instead of ${normalizedId || normalizedName}.`
        };
    emitSettingsDialogDiagnostics("Applied active dataset survey CRS assignment.", {
      action: "apply_native_override",
      nativeOverrideId: normalizedId || null,
      nativeOverrideName: normalizedName || null
    });
  }

  async function clearNativeOverride(): Promise<void> {
    await viewerModel.setActiveDatasetNativeCoordinateReference(null, null);
    syncDraftsFromModel();
    nativeOverrideFeedback = null;
    emitSettingsDialogDiagnostics("Cleared active dataset survey CRS assignment.", {
      action: "clear_native_override"
    });
  }

  async function applyDisplayCrsAsNativeOverride(): Promise<void> {
    const normalizedDisplayCoordinateReferenceId = displayCoordinateReferenceId.trim();
    if (!normalizedDisplayCoordinateReferenceId) {
      return;
    }
    const resolvedEntry = await resolveCatalogEntry(normalizedDisplayCoordinateReferenceId);
    nativeOverrideId = normalizedDisplayCoordinateReferenceId;
    nativeOverrideName =
      resolvedEntry?.name?.trim() ??
      displayCoordinateReferenceName.trim() ??
      nativeOverrideName;
    await applyNativeOverride();
  }

  function applySurveySelection(): void {
    viewerModel.setProjectSurveyAssetId(surveyAssetId);
    syncDraftsFromModel();
  }

  function applyWellboreSelection(): void {
    viewerModel.setProjectWellboreId(wellboreId);
    syncDraftsFromModel();
  }

  function applyTolerance(): void {
    const parsedTolerance = Number(toleranceM);
    viewerModel.setProjectSectionToleranceM(parsedTolerance);
    syncDraftsFromModel();
  }

  async function handleResolveProjectWellOverlays(): Promise<void> {
    try {
      await viewerModel.resolveConfiguredProjectSectionWellOverlays();
    } catch (error) {
      viewerModel.note(
        "Failed to resolve configured project well overlays.",
        "backend",
        "warn",
        error instanceof Error ? error.message : String(error)
      );
    }
  }

  function syncDraftsFromModel(): void {
    displayMode = viewerModel.projectDisplayCoordinateReferenceMode;
    displayCoordinateReferenceId = viewerModel.projectDisplayCoordinateReferenceIdDraft;
    nativeOverrideId = viewerModel.nativeCoordinateReferenceOverrideIdDraft;
    nativeOverrideName = viewerModel.nativeCoordinateReferenceOverrideNameDraft;
    projectRoot = viewerModel.projectRoot;
    surveyAssetId = viewerModel.projectSurveyAssetId;
    wellboreId = viewerModel.projectWellboreId;
    toleranceM = String(viewerModel.projectSectionToleranceM);
    void refreshCoordinateReferenceHints();
  }

  const settingsSections: Array<{
    key: SettingsSection;
    label: string;
    description: string;
  }> = [
    {
      key: "coordinate_systems",
      label: "Coordinate Systems",
      description: "Display CRS and active survey CRS assignment."
    },
    {
      key: "project_wells",
      label: "Project Wells",
      description: "Project root, survey selection, and well inventory."
    },
    {
      key: "section_overlays",
      label: "Section Overlays",
      description: "Configured section-well alignment and overlay resolution."
    }
  ];
</script>

<svelte:window
  onkeydown={(event) => {
    if (dismissible && event.key === "Escape") {
      closeDialog();
    }
  }}
/>

<div class="settings-backdrop" role="presentation" onclick={handleBackdropClick}>
  <div
    class="settings-dialog"
    role="dialog"
    aria-modal="true"
    aria-label="Project settings"
    tabindex="0"
    onclick={(event) => event.stopPropagation()}
    onkeydown={(event) => event.stopPropagation()}
  >
    <header class="settings-header">
      <div>
        <h2>Project Settings</h2>
        <p>Coordinate systems, project wells, and section overlays.</p>
      </div>
      {#if dismissible}
        <button class="secondary" type="button" onclick={closeDialog}>Close</button>
      {/if}
    </header>

    <div class="settings-shell">
      <nav class="settings-nav" aria-label="Project settings sections">
        {#each settingsSections as section (section.key)}
          <button
            type="button"
            class={[
              "nav-item",
              activeSection === section.key && "nav-item-active"
            ]}
            onclick={() => {
              activeSection = section.key;
            }}
          >
            <strong>{section.label}</strong>
            <span>{section.description}</span>
          </button>
        {/each}
      </nav>

      <div class="settings-content">
        {#if projectWorkflowAdvisory || remainingStartupSetupBlockers.length}
          <div class="warning-list info-note">
            {#if projectWorkflowAdvisory}
              <p>{projectWorkflowAdvisory}</p>
            {/if}
            {#each remainingStartupSetupBlockers as blocker (blocker)}
              <p>{blocker}</p>
            {/each}
          </div>
        {/if}

        {#if activeSection === "coordinate_systems"}
          <section class="settings-section">
            <div class="section-heading">
              <h3>Coordinate Systems</h3>
              <p>Separate the project composition CRS from the active survey's raw source CRS.</p>
            </div>

            <div class="warning-list info-note">
              <p>
                <strong>Display CRS</strong> is the common map/project frame used for overlays and
                composition. <strong>Survey CRS assignment</strong> tells TraceBoost how to interpret
                the active survey's raw coordinates before any transform into the display CRS.
              </p>
              <p>
                Assigning a survey CRS changes metadata only. A reprojected copy would write a new
                dataset in the display CRS.
              </p>
            </div>

            <label class="field">
              <span>Display Mode</span>
              <select bind:value={displayMode}>
                <option value="authority_code">Specific CRS from registry</option>
                <option value="native_engineering">Native engineering coordinates</option>
              </select>
            </label>

            <div class="field">
              <span>Display CRS</span>
              <div class="selection-card">
                <div>
                  <strong>{displayCoordinateReferenceSummary}</strong>
                  <p>
                    {#if displayMode === "authority_code"}
                      {#if viewerModel.projectRoot.trim()}
                        Used for project maps, cross-survey overlays, and project well composition.
                      {:else}
                        Used for session map alignment now, and reused for project maps if you later
                        set a project root.
                      {/if}
                    {:else}
                      Keep project display in local engineering coordinates without an authority CRS.
                    {/if}
                  </p>
                </div>
                <div class="selection-actions">
                  <button
                    type="button"
                    class="secondary"
                    disabled={displayMode !== "authority_code"}
                    onclick={() => openCoordinateReferencePicker("project_display")}
                  >
                    Choose CRS
                  </button>
                  {#if displayMode === "authority_code" && displayCoordinateReferenceId}
                    <button
                      type="button"
                      class="secondary"
                      onclick={() => {
                        displayCoordinateReferenceId = "";
                        displayCoordinateReferenceName = "";
                      }}
                    >
                      Clear
                    </button>
                  {/if}
                </div>
              </div>
            </div>

            <div class="action-row">
              <button
                type="button"
                disabled={viewerModel.projectGeospatialSettingsSaving}
                onclick={() => void applyProjectDisplaySettings()}
              >
                {viewerModel.projectGeospatialSettingsSaving ? "Saving..." : applyProjectDisplayLabel}
              </button>
              {#if suggestedProjectDisplayEntry}
                <button
                  type="button"
                  class="secondary"
                  disabled={viewerModel.projectGeospatialSettingsSaving}
                  onclick={() => void applySuggestedProjectDisplaySettings()}
                >
                  Use Survey CRS
                </button>
              {/if}
            </div>

            <div class="meta-grid">
              <div class="meta-card">
                <span>Detected Native CRS</span>
                <strong>
                  {viewerModel.activeDetectedNativeCoordinateReferenceId ??
                    viewerModel.activeDetectedNativeCoordinateReferenceName ??
                    "Unknown"}
                </strong>
              </div>
              <div class="meta-card">
                <span>Effective Native CRS</span>
                <strong>
                  {viewerModel.activeEffectiveNativeCoordinateReferenceId ??
                    viewerModel.activeEffectiveNativeCoordinateReferenceName ??
                    "Unknown"}
                </strong>
              </div>
              <div class="meta-card">
                <span>Project CRS Source</span>
                <strong>{projectDisplaySourceLabel}</strong>
              </div>
              <div class="meta-card">
                <span>Suggested CRS</span>
                <strong>{suggestedProjectDisplayEntry?.authId ?? "None"}</strong>
              </div>
            </div>

            <div class="field">
              <span>Assign Survey CRS</span>
              <div class="selection-card">
                <div>
                  <strong>{nativeOverrideSummary}</strong>
                  <p>
                    Used to interpret the active survey's raw X/Y coordinates when the dataset
                    itself does not declare a trustworthy source CRS.
                  </p>
                </div>
                <div class="selection-actions">
                  <button
                    type="button"
                    class="secondary"
                    disabled={!viewerModel.comparePrimaryStorePath}
                    onclick={() => openCoordinateReferencePicker("native_override")}
                  >
                    Choose Survey CRS
                  </button>
                  {#if nativeOverrideId || nativeOverrideName}
                    <button
                      type="button"
                      class="secondary"
                      disabled={!viewerModel.comparePrimaryStorePath}
                      onclick={() => {
                        nativeOverrideId = "";
                        nativeOverrideName = "";
                      }}
                    >
                      Clear Draft
                    </button>
                  {/if}
                </div>
              </div>
            </div>

            <div class="action-row">
              <button
                type="button"
                class="secondary"
                disabled={
                  !viewerModel.comparePrimaryStorePath ||
                  displayMode !== "authority_code" ||
                  !displayCoordinateReferenceId.trim()
                }
                onclick={() => void applyDisplayCrsAsNativeOverride()}
              >
                Use Display CRS as Survey CRS
              </button>
              <button
                type="button"
                disabled={
                  !viewerModel.comparePrimaryStorePath ||
                  (!nativeOverrideId.trim() && !nativeOverrideName.trim())
                }
                onclick={() => void applyNativeOverride()}
              >
                Assign Survey CRS
              </button>
              <button
                type="button"
                class="secondary"
                disabled={!viewerModel.comparePrimaryStorePath}
                onclick={() => void clearNativeOverride()}
              >
                Clear Survey CRS
              </button>
            </div>

            {#if nativeOverrideFeedback}
              <p class={["status", nativeOverrideFeedback.level === "error" && "error"]}>
                {nativeOverrideFeedback.message}
              </p>
            {/if}

            <div class="field">
              <span>Reprojected Copy</span>
              <div class="selection-card">
                <div>
                  <strong>{datasetCoordinateMaterializationAvailability.title}</strong>
                  <p>{datasetCoordinateMaterializationAvailability.message}</p>
                </div>
                <div class="selection-actions">
                  <button type="button" class="secondary" disabled>
                    Materialize Reprojected Copy
                  </button>
                </div>
              </div>
            </div>

            {#if viewerModel.workspaceCoordinateReferenceWarnings.length}
              <div class="warning-list">
                {#each viewerModel.workspaceCoordinateReferenceWarnings as warning (warning)}
                  <p>{warning}</p>
                {/each}
              </div>
            {/if}
          </section>
        {/if}

        {#if activeSection === "project_wells"}
          <section class="settings-section">
            <div class="section-heading">
              <h3>Project Wells</h3>
              <p>Project root, active survey selection, and project well inventory for project-only workflows.</p>
            </div>

            <label class="field">
              <span>Project Root</span>
              <div class="field-row">
                <input
                  bind:value={projectRoot}
                  type="text"
                  placeholder="/path/to/ophiolite-project"
                  onkeydown={(event) => {
                    if (event.key === "Enter") {
                      void applyProjectRoot();
                    }
                  }}
                  onblur={() => void applyProjectRoot()}
                />
                <button type="button" class="secondary" onclick={() => void handlePickProjectRoot()}>
                  Browse...
                </button>
              </div>
            </label>

            <label class="field">
              <span>Survey Asset</span>
              <select bind:value={surveyAssetId} onchange={applySurveySelection}>
                <option value="">Select a survey asset</option>
                {#each viewerModel.projectSurveyAssets as survey (survey.assetId)}
                  <option value={survey.assetId}>{survey.name} | {survey.wellboreName}</option>
                {/each}
              </select>
            </label>

            <label class="field">
              <span>Wellbore</span>
              <select bind:value={wellboreId} onchange={applyWellboreSelection}>
                <option value="">Select a wellbore</option>
                {#each viewerModel.projectWellboreInventory as wellbore (wellbore.wellboreId)}
                  <option value={wellbore.wellboreId}>
                    {wellbore.wellName} | {wellbore.wellboreName}
                  </option>
                {/each}
              </select>
            </label>

            <div class="meta-grid">
              <div class="meta-card">
                <span>Survey Assets</span>
                <strong>{viewerModel.projectSurveyAssets.length}</strong>
              </div>
              <div class="meta-card">
                <span>Wellbores</span>
                <strong>{viewerModel.projectWellboreInventory.length}</strong>
              </div>
              <div class="meta-card">
                <span>Observation Sets</span>
                <strong>{viewerModel.projectWellTimeDepthObservationSets.length}</strong>
              </div>
              <div class="meta-card">
                <span>Compiled Models</span>
                <strong>{viewerModel.projectWellTimeDepthModels.length}</strong>
              </div>
            </div>

            {#if viewerModel.projectWellOverlayInventoryError}
              <p class="status error">{viewerModel.projectWellOverlayInventoryError}</p>
            {:else if viewerModel.projectWellOverlayInventoryLoading}
              <p class="status">Loading project inventory...</p>
            {/if}
          </section>
        {/if}

        {#if activeSection === "section_overlays"}
          <section class="settings-section">
            <div class="section-heading">
              <h3>Section Overlays</h3>
              <p>Choose the reference wellbore and resolve project section overlays.</p>
            </div>

            <label class="field">
              <span>Tolerance (m)</span>
              <input
                bind:value={toleranceM}
                type="number"
                min="0.1"
                step="0.1"
                onkeydown={(event) => {
                  if (event.key === "Enter") {
                    applyTolerance();
                  }
                }}
                onblur={applyTolerance}
              />
            </label>

            <div class="action-row">
              <button
                type="button"
                class="secondary"
                disabled={viewerModel.loading || !viewerModel.projectRoot}
                onclick={() =>
                  void viewerModel.refreshProjectWellOverlayInventory(viewerModel.projectRoot)}
              >
                Refresh Inventory
              </button>
              <button
                type="button"
                disabled={
                  !viewerModel.canResolveConfiguredProjectSectionWellOverlays ||
                  viewerModel.projectSectionWellOverlaysLoading ||
                  viewerModel.loading
                }
                onclick={() => void handleResolveProjectWellOverlays()}
              >
                {viewerModel.projectSectionWellOverlaysLoading ? "Resolving..." : "Resolve Overlays"}
              </button>
              <button
                type="button"
                class="secondary"
                disabled={!viewerModel.sectionWellOverlays.length && !viewerModel.projectSectionWellOverlays}
                onclick={() => viewerModel.clearProjectSectionWellOverlays()}
              >
                Clear
              </button>
            </div>

            <div class="meta-grid">
              <div class="meta-card">
                <span>Overlay Count</span>
                <strong>{viewerModel.sectionWellOverlays.length}</strong>
              </div>
              <div class="meta-card">
                <span>Project Well</span>
                <strong>{viewerModel.projectWellboreId || "Not selected"}</strong>
              </div>
              <div class="meta-card">
                <span>Display CRS</span>
                <strong>{viewerModel.displayCoordinateReferenceId ?? "Unresolved"}</strong>
              </div>
              <div class="meta-card">
                <span>Tolerance</span>
                <strong>{Number.isFinite(viewerModel.projectSectionToleranceM) ? `${viewerModel.projectSectionToleranceM} m` : "Unknown"}</strong>
              </div>
            </div>

            {#if viewerModel.projectSectionWellOverlayResolveBlocker}
              <div class="warning-list">
                <p>{viewerModel.projectSectionWellOverlayResolveBlocker}</p>
              </div>
            {/if}

            {#if viewerModel.projectDisplayCompatibilityBlockingMessages.length}
              <div class="warning-list">
                {#each viewerModel.projectDisplayCompatibilityBlockingMessages as message (message)}
                  <p>{message}</p>
                {/each}
              </div>
            {/if}
          </section>
        {/if}
      </div>
    </div>
  </div>
</div>

{#if crsPickerTarget}
  <CoordinateReferencePicker
    close={closeCoordinateReferencePicker}
    confirm={handleCoordinateReferenceSelection}
    title={crsPickerTarget === "project_display"
      ? "Project Coordinate Reference System"
      : "Active Survey Coordinate Reference System"}
    description={crsPickerTarget === "project_display"
      ? "Choose the validated CRS used for project display and cross-survey alignment."
      : "Choose the CRS that should be used to interpret the active survey dataset's raw coordinates."}
    allowLocalEngineering={crsPickerTarget === "native_override"}
    localEngineeringLabel="Survey local coordinates"
    selectedAuthId={crsPickerTarget === "project_display"
      ? displayCoordinateReferenceId
      : nativeOverrideId}
    {projectRoot}
    projectedOnly={false}
    includeGeographic={true}
    includeVertical={false}
    recommendedEntries={crsPickerTarget === "project_display"
      ? projectDisplayRecommendedEntries
      : []}
  />
{/if}

<style>
  .settings-backdrop {
    position: fixed;
    inset: 0;
    z-index: 90;
    display: flex;
    align-items: center;
    justify-content: center;
    padding: 24px;
    background: rgba(38, 55, 71, 0.2);
    backdrop-filter: blur(4px);
  }

  .settings-dialog {
    width: min(1080px, calc(100vw - 48px));
    max-height: min(860px, calc(100vh - 48px));
    overflow: auto;
    display: grid;
    gap: 16px;
    padding: 22px;
    border: 1px solid var(--app-border);
    border-radius: 8px;
    background: var(--panel-bg);
    color: var(--text-primary);
    box-shadow: 0 20px 60px rgba(42, 64, 84, 0.18);
  }

  .settings-header,
  .action-row,
  .field-row,
  .selection-actions {
    display: flex;
    align-items: flex-start;
    justify-content: space-between;
    gap: 12px;
  }

  .settings-header h2,
  .section-heading h3 {
    margin: 0;
    font-size: 16px;
    font-weight: 650;
  }

  .settings-header p,
  .section-heading p,
  .selection-card p,
  .nav-item span {
    margin: 4px 0 0;
    color: var(--text-muted);
  }

  .settings-shell {
    display: grid;
    grid-template-columns: minmax(220px, 250px) minmax(0, 1fr);
    gap: 18px;
    min-height: 0;
  }

  .settings-nav,
  .settings-content,
  .settings-section,
  .meta-grid,
  .warning-list,
  .field {
    display: grid;
    gap: 12px;
  }

  .settings-nav {
    align-content: start;
  }

  .nav-item {
    display: grid;
    gap: 4px;
    padding: 12px;
    text-align: left;
    border: 1px solid var(--app-border);
    border-radius: 8px;
    background: var(--surface-bg);
    color: var(--text-primary);
  }

  .nav-item-active {
    border-color: var(--accent-border);
    background: rgba(69, 120, 165, 0.08);
  }

  .settings-section {
    align-content: start;
    padding: 16px;
    border: 1px solid var(--app-border);
    border-radius: 8px;
    background: var(--surface-bg);
  }

  .field span,
  .meta-card span {
    font-size: 11px;
    text-transform: uppercase;
    letter-spacing: 0.04em;
    color: var(--text-dim);
  }

  .field input,
  .field select {
    width: 100%;
    min-width: 0;
    padding: 10px 11px;
    border: 1px solid var(--app-border-strong);
    border-radius: 6px;
    background: #fff;
    color: var(--text-primary);
    font: inherit;
  }

  .field-row input {
    flex: 1 1 auto;
  }

  .selection-card,
  .meta-card {
    display: grid;
    gap: 8px;
    padding: 12px;
    border: 1px solid var(--app-border);
    border-radius: 8px;
    background: #fff;
  }

  .selection-card strong,
  .meta-card strong {
    font-size: 14px;
    word-break: break-word;
  }

  .meta-grid {
    grid-template-columns: repeat(auto-fit, minmax(180px, 1fr));
  }

  .warning-list {
    padding: 12px;
    border: 1px solid rgba(196, 140, 26, 0.32);
    border-radius: 8px;
    background: rgba(245, 197, 66, 0.12);
    color: var(--text-primary);
  }

  .info-note {
    border-color: var(--app-border);
    background: rgba(69, 120, 165, 0.08);
  }

  .warning-list p,
  .status {
    margin: 0;
  }

  .status.error {
    color: #b34b35;
  }

  @media (max-width: 900px) {
    .settings-dialog {
      width: min(100vw - 24px, 1000px);
      max-height: calc(100vh - 24px);
      padding: 16px;
    }

    .settings-shell {
      grid-template-columns: minmax(0, 1fr);
    }

    .settings-nav {
      grid-template-columns: repeat(auto-fit, minmax(180px, 1fr));
    }
  }

  @media (max-width: 640px) {
    .settings-backdrop {
      padding: 12px;
    }

    .settings-header,
    .action-row,
    .field-row,
    .selection-actions {
      flex-direction: column;
      align-items: stretch;
    }
  }
</style>
