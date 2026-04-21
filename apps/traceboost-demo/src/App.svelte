<svelte:options runes={true} />

<script lang="ts">
  import { onMount } from "svelte";
  import HorizonImportDialog from "./lib/components/HorizonImportDialog.svelte";
  import MissingNativeCoordinateReferencePrompt from "./lib/components/MissingNativeCoordinateReferencePrompt.svelte";
  import ProjectSettingsDialog from "./lib/components/ProjectSettingsDialog.svelte";
  import SegyImportDialog from "./lib/components/SegyImportDialog.svelte";
  import VendorProjectImportDialog from "./lib/components/VendorProjectImportDialog.svelte";
  import type { ProjectAssetBindingInput } from "./lib/bridge";
  import WellSourceImportDialog from "./lib/components/WellFolderImportDialog.svelte";
  import WellTimeDepthImportDialog from "./lib/components/WellTimeDepthImportDialog.svelte";
  import WorkflowSidebar from "./lib/components/WorkflowSidebar.svelte";
  import ViewerMain from "./lib/components/ViewerMain.svelte";
  import { emitFrontendDiagnosticsEvent, isTauriEnvironment } from "./lib/bridge";
  import {
    pickHorizonFiles,
    pickImportSeismicFile,
    pickProjectFolder,
    pickRuntimeStoreFile,
    pickVelocityFunctionsFile,
    pickWellImportFiles,
    pickWellTimeDepthJsonFile
  } from "./lib/file-dialog";
  import { ProcessingModel, setProcessingModelContext } from "./lib/processing-model.svelte";
  import { buildStartupSetupBlockers } from "./lib/startup-setup";
  import { setViewerModelContext, ViewerModel } from "./lib/viewer-model.svelte";

  let showSidebar = $state(true);
  let horizonImportDialogOpen = $state(false);
  let pendingHorizonImportPaths = $state<string[]>([]);
  let wellSourceImportDialogOpen = $state(false);
  let pendingWellImportSourceRoot = $state<string | null>(null);
  let pendingWellImportSourcePaths = $state<string[]>([]);
  let segyImportDialogOpen = $state(false);
  let pendingSegyImportPath = $state<string | null>(null);
  let wellTimeDepthImportDialogOpen = $state(false);
  let vendorProjectImportDialogOpen = $state(false);
  let pendingWellTimeDepthImport = $state.raw<{
    projectRoot: string;
    jsonPath: string;
    binding: ProjectAssetBindingInput;
    assetKind:
      | "checkshot_vsp_observation_set"
      | "manual_time_depth_pick_set"
      | "well_time_depth_authored_model"
      | "well_time_depth_model";
    dialogTitle: string;
  } | null>(null);
  let viewerChart = $state.raw<{ fitToData?: () => void } | null>(null);
  let lastStartupSetupDiagnosticsSignature = "";

  const viewerModel = setViewerModelContext(new ViewerModel({ tauriRuntime: isTauriEnvironment() }));
  const processingModel = setProcessingModelContext(new ProcessingModel({ viewerModel }));
  let startupSetupBlockers = $derived.by(() =>
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
  let startupSetupRequired = $derived(startupSetupBlockers.length > 0);

  function hideSidebar(): void {
    showSidebar = false;
  }

  function showSidebarPanel(): void {
    showSidebar = true;
  }

  function logSettingsDialogEvent(
    message: string,
    fields: Record<string, unknown> | null = null
  ): void {
    void emitFrontendDiagnosticsEvent({
      stage: "settings_dialog",
      level: "debug",
      message,
      fields
    }).catch((error) => {
      console.warn("Failed to emit settings dialog diagnostics event.", error);
    });
  }

  $effect(() => {
    const fields = {
      blockers: startupSetupBlockers,
      blockerCount: startupSetupBlockers.length,
      dismissible: !startupSetupRequired,
      startupSetupRequired,
      workspaceReady: viewerModel.workspaceReady,
      projectSettingsOpen: viewerModel.projectSettingsOpen,
      hasProjectRoot: viewerModel.projectRoot.trim().length > 0,
      projectGeospatialSettingsResolved: viewerModel.projectGeospatialSettingsResolved,
      projectGeospatialSettingsSource: viewerModel.projectGeospatialSettingsSource,
      hasActiveStore: viewerModel.activeStorePath.trim().length > 0,
      activeEffectiveNativeCoordinateReferenceId:
        viewerModel.activeEffectiveNativeCoordinateReferenceId,
      activeEffectiveNativeCoordinateReferenceName:
        viewerModel.activeEffectiveNativeCoordinateReferenceName
    };
    const signature = JSON.stringify(fields);
    if (signature === lastStartupSetupDiagnosticsSignature) {
      return;
    }
    lastStartupSetupDiagnosticsSignature = signature;
    logSettingsDialogEvent("Startup setup state evaluated.", fields);
  });

  function openSettings(source = "unknown"): void {
    logSettingsDialogEvent("Project settings dialog open requested.", {
      source,
      openBeforeRequest: viewerModel.projectSettingsOpen
    });
    viewerModel.openProjectSettings();
    logSettingsDialogEvent("Project settings dialog open state applied.", {
      source,
      open: viewerModel.projectSettingsOpen
    });
  }

  function closeSettings(reason = "unknown"): void {
    viewerModel.closeProjectSettings();
    logSettingsDialogEvent("Project settings dialog closed.", {
      reason,
      open: viewerModel.projectSettingsOpen
    });
  }

  function closeHorizonImportDialog(): void {
    if (viewerModel.horizonImporting) {
      return;
    }
    horizonImportDialogOpen = false;
    pendingHorizonImportPaths = [];
  }

  function closeWellSourceImportDialog(): void {
    wellSourceImportDialogOpen = false;
    pendingWellImportSourceRoot = null;
    pendingWellImportSourcePaths = [];
  }

  function closeSegyImportDialog(): void {
    segyImportDialogOpen = false;
    pendingSegyImportPath = null;
  }

  function closeWellTimeDepthImportDialog(): void {
    wellTimeDepthImportDialogOpen = false;
    pendingWellTimeDepthImport = null;
  }

  function closeVendorProjectImportDialog(): void {
    vendorProjectImportDialogOpen = false;
  }

  function commonWellImportRoot(inputPaths: string[]): string | null {
    const normalized = inputPaths
      .map((value) => value.trim())
      .filter((value) => value.length > 0)
      .map((value) => value.replace(/\\/g, "/"));
    if (normalized.length === 0) {
      return null;
    }

    const splitParent = (value: string): string[] => {
      const parts = value.split("/");
      parts.pop();
      return parts.filter((part, index) => part.length > 0 || index === 0);
    };

    let shared = splitParent(normalized[0]);
    for (const path of normalized.slice(1)) {
      const next = splitParent(path);
      const sharedLength = Math.min(shared.length, next.length);
      let index = 0;
      while (index < sharedLength && shared[index] === next[index]) {
        index += 1;
      }
      shared = shared.slice(0, index);
    }

    if (shared.length === 0) {
      const fallbackPath = normalized[0];
      const separatorIndex = fallbackPath.lastIndexOf("/");
      return separatorIndex > 0 ? fallbackPath.slice(0, separatorIndex) : null;
    }
    return shared.join("/") || null;
  }

  function openDepthConversionWorkbench(): void {
    showSidebarPanel();
    logSettingsDialogEvent("Depth conversion dialog open requested.", {
      source: "app_menu_or_ui",
      openBeforeRequest: viewerModel.depthConversionWorkbenchOpen
    });
    viewerModel.openDepthConversionWorkbench();
    logSettingsDialogEvent("Depth conversion dialog open state applied.", {
      open: viewerModel.depthConversionWorkbenchOpen
    });
  }

  function openResidualWorkbench(): void {
    showSidebarPanel();
    viewerModel.openResidualWorkbench();
  }

  function selectedProjectWellBinding() {
    const selectedWellbore = viewerModel.selectedProjectWellboreInventoryItem;
    if (!selectedWellbore) {
      return null;
    }

    return {
      well_name: selectedWellbore.wellName,
      wellbore_name: selectedWellbore.wellboreName,
      operator_aliases: []
    };
  }

  async function handleOpenVolumeMenu(): Promise<void> {
    showSidebarPanel();
    const path = await pickRuntimeStoreFile();

    if (path) {
      await viewerModel.openVolumePath(path);
      return;
    }

    viewerModel.note("Volume selection did not produce a usable path.", "ui", "warn");
  }

  async function handleImportSeismicMenu(): Promise<void> {
    showSidebarPanel();
    const path = await pickImportSeismicFile();
    if (!path) {
      viewerModel.note("Import selection did not produce a usable seismic path.", "ui", "warn");
      return;
    }
    if (/\.(sgy|segy)$/i.test(path.trim())) {
      pendingSegyImportPath = path;
      segyImportDialogOpen = true;
      return;
    }
    await viewerModel.openVolumePath(path);
  }

  async function handleRequestHorizonImport(): Promise<void> {
    const inputPaths = await pickHorizonFiles();
    if (inputPaths.length === 0) {
      viewerModel.note("Horizon import selection did not produce usable files.", "ui", "warn");
      return;
    }
    if (!viewerModel.activeStorePath.trim()) {
      viewerModel.note(
        "No active survey store is open. The selected horizons will be parsed for review, but final import stays disabled until a seismic volume is open.",
        "ui",
        "warn"
      );
    }
    if (viewerModel.horizonImportProjectAdvisory) {
      viewerModel.note(viewerModel.horizonImportProjectAdvisory, "ui", "warn");
    }
    pendingHorizonImportPaths = inputPaths;
    horizonImportDialogOpen = true;
  }

  async function handleRequestPetrelImport(): Promise<void> {
    showSidebarPanel();
    if (!viewerModel.tauriRuntime) {
      viewerModel.note("Petrel import is only available in the desktop runtime.", "ui", "warn");
      return;
    }
    vendorProjectImportDialogOpen = true;
  }

  async function handleImportVelocityFunctionsMenu(): Promise<void> {
    showSidebarPanel();
    if (!viewerModel.activeStorePath) {
      viewerModel.note("Open a seismic volume before importing velocity functions.", "ui", "warn");
      return;
    }
    const inputPath = await pickVelocityFunctionsFile();
    if (!inputPath) {
      return;
    }
    await viewerModel.importVelocityFunctionsFile(inputPath, "interval");
  }

  async function handleRequestWellSourceImport(): Promise<void> {
    showSidebarPanel();
    if (!viewerModel.tauriRuntime) {
      viewerModel.note("Well import is only available in the desktop runtime.", "ui", "warn");
      return;
    }

    const sourcePaths = await pickWellImportFiles();
    if (sourcePaths.length === 0) {
      return;
    }
    if (!viewerModel.projectRoot.trim()) {
      viewerModel.note(
        "No Ophiolite project location is set. The selected well files will still be parsed for review, but final import stays disabled until a destination project is chosen.",
        "ui",
        "warn"
      );
    }

    pendingWellImportSourceRoot = commonWellImportRoot(sourcePaths);
    pendingWellImportSourcePaths = sourcePaths;
    wellSourceImportDialogOpen = true;
  }

  async function ensureProjectSettingsReady(): Promise<boolean> {
    if (viewerModel.canImportProjectWellAssets && selectedProjectWellBinding()) {
      return true;
    }

    openSettings();

    if (!viewerModel.projectRoot.trim()) {
      const pickedProjectRoot = await pickProjectFolder();
      if (pickedProjectRoot) {
        await viewerModel.setProjectRoot(pickedProjectRoot);
      }
    }

    const importBlocker = viewerModel.projectWellAssetImportBlocker;
    if (importBlocker || !selectedProjectWellBinding()) {
      viewerModel.note(
        importBlocker ??
          "Set the project root and select a project wellbore in Settings before importing well objects.",
        "ui",
        "warn"
      );
      return false;
    }

    const importAdvisory = viewerModel.projectWellAssetImportAdvisory;
    if (importAdvisory) {
      viewerModel.note(
        importAdvisory,
        "ui",
        "warn"
      );
    }

    return true;
  }

  async function handleImportProjectWellTimeDepthAsset(
    assetKind:
      | "checkshot_vsp_observation_set"
      | "manual_time_depth_pick_set"
      | "well_time_depth_authored_model"
      | "well_time_depth_model",
    dialogTitle: string
  ): Promise<void> {
    showSidebarPanel();
    const settingsReady = await ensureProjectSettingsReady();
    const projectRoot = viewerModel.projectRoot.trim();
    const binding = selectedProjectWellBinding();
    if (!settingsReady || !projectRoot || !binding) {
      return;
    }

    const jsonPath = await pickWellTimeDepthJsonFile(dialogTitle);
    if (!jsonPath) {
      return;
    }

    pendingWellTimeDepthImport = {
      projectRoot,
      jsonPath,
      binding,
      assetKind,
      dialogTitle
    };
    wellTimeDepthImportDialogOpen = true;
  }

  onMount(() => {
    let disposed = false;
    let disposeNativeMenu = () => {};

    if (viewerModel.tauriRuntime) {
      void (async () => {
        const { listen } = await import("@tauri-apps/api/event");
        const unlistenSettings = await listen("menu:app-settings", () => {
          logSettingsDialogEvent("Received native settings menu event.", {
            event: "menu:app-settings"
          });
          openSettings("native_menu");
        });
        const unlistenVelocityModel = await listen("menu:app-velocity-model", () => {
          showSidebarPanel();
          viewerModel.openVelocityModelWorkbench();
        });
        const unlistenResiduals = await listen("menu:app-residuals", () => {
          openResidualWorkbench();
        });
        const unlistenDepthConversion = await listen("menu:app-depth-conversion", () => {
          logSettingsDialogEvent("Received native depth-conversion menu event.", {
            event: "menu:app-depth-conversion"
          });
          openDepthConversionWorkbench();
        });
        const unlistenWellTie = await listen("menu:app-well-tie", () => {
          showSidebarPanel();
          viewerModel.openWellTieWorkbench();
        });
        const unlistenOpenVolume = await listen("menu:file-open-volume", () => {
          void handleOpenVolumeMenu();
        });
        const unlistenImportSeismic = await listen("menu:file-import-seismic", () => {
          void handleImportSeismicMenu();
        });
        const unlistenImportHorizons = await listen("menu:file-import-horizons", () => {
          void handleRequestHorizonImport();
        });
        const unlistenImportWellSources = await listen("menu:file-import-well-sources", () => {
          void handleRequestWellSourceImport();
        });
        const unlistenImportVelocityFunctions = await listen("menu:file-import-velocity-functions", () => {
          void handleImportVelocityFunctionsMenu();
        });
        const unlistenImportCheckshot = await listen("menu:file-import-checkshot", () => {
          void handleImportProjectWellTimeDepthAsset(
            "checkshot_vsp_observation_set",
            "Import Checkshot/VSP Observation Set"
          );
        });
        const unlistenImportManualPicks = await listen("menu:file-import-manual-picks", () => {
          void handleImportProjectWellTimeDepthAsset(
            "manual_time_depth_pick_set",
            "Import Manual Time-Depth Picks"
          );
        });
        const unlistenImportAuthoredModel = await listen("menu:file-import-authored-well-model", () => {
          void handleImportProjectWellTimeDepthAsset(
            "well_time_depth_authored_model",
            "Import Well Time-Depth Authored Model"
          );
        });
        const unlistenImportCompiledModel = await listen("menu:file-import-compiled-well-model", () => {
          void handleImportProjectWellTimeDepthAsset(
            "well_time_depth_model",
            "Import Compiled Well Time-Depth Model"
          );
        });

        if (disposed) {
          unlistenSettings();
          unlistenVelocityModel();
          unlistenResiduals();
          unlistenDepthConversion();
          unlistenWellTie();
          unlistenOpenVolume();
          unlistenImportSeismic();
          unlistenImportHorizons();
          unlistenImportWellSources();
          unlistenImportVelocityFunctions();
          unlistenImportCheckshot();
          unlistenImportManualPicks();
          unlistenImportAuthoredModel();
          unlistenImportCompiledModel();
          return;
        }

        disposeNativeMenu = () => {
          unlistenSettings();
          unlistenVelocityModel();
          unlistenResiduals();
          unlistenDepthConversion();
          unlistenWellTie();
          unlistenOpenVolume();
          unlistenImportSeismic();
          unlistenImportHorizons();
          unlistenImportWellSources();
          unlistenImportVelocityFunctions();
          unlistenImportCheckshot();
          unlistenImportManualPicks();
          unlistenImportAuthoredModel();
          unlistenImportCompiledModel();
        };
      })();
    }

    const disposeViewer = viewerModel.mountShell();
    const disposeProcessing = processingModel.mount();
    return () => {
      disposed = true;
      disposeNativeMenu();
      disposeProcessing();
      disposeViewer();
    };
  });

</script>

<svelte:head>
  <title>TraceBoost</title>
</svelte:head>

<div class={["shell", !showSidebar && "sidebar-hidden", startupSetupRequired && "shell-blocked"]}>
  <WorkflowSidebar {showSidebar} {hideSidebar} />
  <ViewerMain
    {showSidebar}
    {showSidebarPanel}
    {openSettings}
    requestHorizonImport={handleRequestHorizonImport}
    requestPetrelImport={handleRequestPetrelImport}
    bind:chartRef={viewerChart}
  />
</div>

{#if startupSetupRequired}
  <div class="startup-setup-backdrop" role="presentation">
    <div class="startup-setup-panel" role="dialog" aria-modal="true" aria-label="Initial project setup required">
      <h2>Initial setup required</h2>
      <p>
        Resolve the remaining startup setup choices before continuing into the workspace.
      </p>
      <ul class="startup-setup-list">
        {#each startupSetupBlockers as blocker (blocker)}
          <li>{blocker}</li>
        {/each}
      </ul>
      <button type="button" class="startup-setup-action" onclick={() => openSettings("startup_gate_panel")}>
        Open Project Settings
      </button>
    </div>
  </div>
{/if}

{#if startupSetupRequired || viewerModel.projectSettingsOpen}
  <ProjectSettingsDialog
    close={() => closeSettings("dialog")}
    dismissible={!startupSetupRequired}
  />
{/if}

{#if viewerModel.missingNativeCoordinateReferencePrompt}
  <MissingNativeCoordinateReferencePrompt {openSettings} />
{/if}

{#if segyImportDialogOpen && pendingSegyImportPath}
  <SegyImportDialog
    open={segyImportDialogOpen}
    inputPath={pendingSegyImportPath}
    {viewerModel}
    onClose={closeSegyImportDialog}
  />
{/if}
{#if horizonImportDialogOpen}
  <HorizonImportDialog inputPaths={pendingHorizonImportPaths} close={closeHorizonImportDialog} />
{/if}
{#if wellSourceImportDialogOpen && pendingWellImportSourceRoot}
  <WellSourceImportDialog
    sourceRootPath={pendingWellImportSourceRoot}
    sourcePaths={pendingWellImportSourcePaths}
    close={closeWellSourceImportDialog}
  />
{/if}
{#if wellTimeDepthImportDialogOpen && pendingWellTimeDepthImport}
  <WellTimeDepthImportDialog
    projectRoot={pendingWellTimeDepthImport.projectRoot}
    jsonPath={pendingWellTimeDepthImport.jsonPath}
    binding={pendingWellTimeDepthImport.binding}
    assetKind={pendingWellTimeDepthImport.assetKind}
    dialogTitle={pendingWellTimeDepthImport.dialogTitle}
    openSettings={openSettings}
    close={closeWellTimeDepthImportDialog}
  />
{/if}
{#if vendorProjectImportDialogOpen}
  <VendorProjectImportDialog openSettings={openSettings} close={closeVendorProjectImportDialog} />
{/if}

<style>
  .shell {
    display: grid;
    grid-template-columns: var(--sidebar-width) 1fr;
    min-height: 100vh;
  }

  .shell.sidebar-hidden {
    grid-template-columns: 1fr;
  }

  .shell.shell-blocked {
    filter: blur(8px);
    pointer-events: none;
    user-select: none;
  }

  .startup-setup-backdrop {
    position: fixed;
    inset: 0;
    z-index: 70;
    display: flex;
    align-items: center;
    justify-content: center;
    padding: 24px;
    background: rgba(6, 10, 16, 0.74);
  }

  .startup-setup-panel {
    width: min(560px, calc(100vw - 32px));
    display: grid;
    gap: 14px;
    padding: 20px;
    border: 1px solid var(--app-border);
    border-radius: 8px;
    background: var(--panel-bg);
    color: var(--text-primary);
    box-shadow: 0 20px 60px rgba(0, 0, 0, 0.28);
  }

  .startup-setup-panel h2,
  .startup-setup-panel p {
    margin: 0;
  }

  .startup-setup-panel p {
    color: var(--text-muted);
  }

  .startup-setup-list {
    margin: 0;
    padding-left: 18px;
    display: grid;
    gap: 8px;
    color: var(--text-primary);
  }

  .startup-setup-action {
    justify-self: start;
    padding: 10px 14px;
    border: 1px solid var(--accent, #4b8cff);
    border-radius: 6px;
    background: var(--accent, #4b8cff);
    color: #fff;
    font: inherit;
  }
</style>
