<svelte:options runes={true} />

<script lang="ts">
  import { onMount } from "svelte";
  import HorizonImportDialog from "./lib/components/HorizonImportDialog.svelte";
  import ProjectSettingsDialog from "./lib/components/ProjectSettingsDialog.svelte";
  import WorkflowSidebar from "./lib/components/WorkflowSidebar.svelte";
  import ViewerMain from "./lib/components/ViewerMain.svelte";
  import { isTauriEnvironment } from "./lib/bridge";
  import {
    pickHorizonFiles,
    pickImportSeismicFile,
    pickProjectFolder,
    pickRuntimeStoreFile,
    pickVelocityFunctionsFile,
    pickWellTimeDepthJsonFile
  } from "./lib/file-dialog";
  import { ProcessingModel, setProcessingModelContext } from "./lib/processing-model.svelte";
  import { setViewerModelContext, ViewerModel } from "./lib/viewer-model.svelte";

  let showSidebar = $state(true);
  let settingsOpen = $state(false);
  let horizonImportDialogOpen = $state(false);
  let pendingHorizonImportPaths = $state<string[]>([]);
  let viewerChart = $state.raw<{ fitToData?: () => void } | null>(null);

  const viewerModel = setViewerModelContext(new ViewerModel({ tauriRuntime: isTauriEnvironment() }));
  const processingModel = setProcessingModelContext(new ProcessingModel({ viewerModel }));

  function hideSidebar(): void {
    showSidebar = false;
  }

  function showSidebarPanel(): void {
    showSidebar = true;
  }

  function openSettings(): void {
    settingsOpen = true;
  }

  function closeSettings(): void {
    settingsOpen = false;
  }

  function closeHorizonImportDialog(): void {
    if (viewerModel.horizonImporting) {
      return;
    }
    horizonImportDialogOpen = false;
    pendingHorizonImportPaths = [];
  }

  function openDepthConversionWorkbench(): void {
    showSidebarPanel();
    const blocker = viewerModel.depthConversionBlocker;
    if (blocker) {
      viewerModel.note(blocker, "ui", "warn");
      return;
    }
    viewerModel.openDepthConversionWorkbench();
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
    await viewerModel.openVolumePath(path);
  }

  async function handleRequestHorizonImport(): Promise<void> {
    const horizonImportBlocker = viewerModel.horizonImportBlocker;
    if (horizonImportBlocker) {
      viewerModel.note(horizonImportBlocker, "ui", "warn");
      return;
    }

    const inputPaths = await pickHorizonFiles();
    if (inputPaths.length === 0) {
      viewerModel.note("Horizon import selection did not produce usable files.", "ui", "warn");
      return;
    }
    if (viewerModel.horizonImportProjectAdvisory) {
      viewerModel.note(viewerModel.horizonImportProjectAdvisory, "ui", "warn");
    }
    pendingHorizonImportPaths = inputPaths;
    horizonImportDialogOpen = true;
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

    try {
      await viewerModel.importProjectWellTimeDepthAsset({
        projectRoot,
        jsonPath,
        binding,
        assetKind
      });
      await viewerModel.refreshProjectWellOverlayInventory(
        projectRoot,
        viewerModel.displayCoordinateReferenceId
      );
      openSettings();
    } catch (error) {
      viewerModel.note(
        "Failed to import project well time-depth asset.",
        "backend",
        "warn",
        error instanceof Error ? error.message : String(error)
      );
    }
  }

  onMount(() => {
    let disposed = false;
    let disposeNativeMenu = () => {};

    if (viewerModel.tauriRuntime) {
      void (async () => {
        const { listen } = await import("@tauri-apps/api/event");
        const unlistenSettings = await listen("menu:app-settings", () => {
          openSettings();
        });
        const unlistenVelocityModel = await listen("menu:app-velocity-model", () => {
          showSidebarPanel();
          viewerModel.openVelocityModelWorkbench();
        });
        const unlistenDepthConversion = await listen("menu:app-depth-conversion", () => {
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
          unlistenDepthConversion();
          unlistenWellTie();
          unlistenOpenVolume();
          unlistenImportSeismic();
          unlistenImportHorizons();
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
          unlistenDepthConversion();
          unlistenWellTie();
          unlistenOpenVolume();
          unlistenImportSeismic();
          unlistenImportHorizons();
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

<div class:sidebar-hidden={!showSidebar} class="shell">
  <WorkflowSidebar {showSidebar} {hideSidebar} />
  <ViewerMain
    {showSidebar}
    {showSidebarPanel}
    {openSettings}
    requestHorizonImport={handleRequestHorizonImport}
    bind:chartRef={viewerChart}
  />
</div>

<ProjectSettingsDialog open={settingsOpen} close={closeSettings} />
{#if horizonImportDialogOpen}
  <HorizonImportDialog inputPaths={pendingHorizonImportPaths} close={closeHorizonImportDialog} />
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
</style>
