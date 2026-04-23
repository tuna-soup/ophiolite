import {
  isTauriEnvironment,
  pickDesktopOutputPath,
  pickDesktopProjectRoot,
  pickDesktopRuntimeStore
} from "./bridge";

function normalizeDialogPath(result: string | null): string | null {
  if (typeof result !== "string") {
    return null;
  }

  const normalized = result.trim();
  return normalized.length > 0 ? normalized : null;
}

function normalizeDialogPaths(result: string[] | string | null): string[] {
  if (Array.isArray(result)) {
    return result
      .map((value) => normalizeDialogPath(value))
      .filter((value): value is string => value !== null);
  }

  const single = normalizeDialogPath(result);
  return single ? [single] : [];
}

/**
 * Opens a native file picker for TraceBoost runtime stores.
 * Returns the selected file path, or null if cancelled.
 */
export async function pickRuntimeStoreFile(): Promise<string | null> {
  if (!isTauriEnvironment()) {
    return normalizeDialogPath(prompt("Enter runtime store path (.tbvol):"));
  }

  return normalizeDialogPath(await pickDesktopRuntimeStore());
}

export async function pickImportSeismicFile(): Promise<string | null> {
  if (!isTauriEnvironment()) {
    return normalizeDialogPath(prompt("Enter import path (.segy, .sgy, .zarr, .mdio):"));
  }

  const { open } = await import("@tauri-apps/plugin-dialog");
  const result = await open({
    title: "Import Seismic Volume",
    filters: [
      { name: "SEG-Y / Zarr / MDIO", extensions: ["sgy", "segy", "zarr", "mdio"] },
      { name: "SEG-Y Files", extensions: ["sgy", "segy"] },
      { name: "Zarr Stores", extensions: ["zarr"] },
      { name: "MDIO Stores", extensions: ["mdio"] },
      { name: "All Files", extensions: ["*"] }
    ],
    multiple: false,
    directory: false
  });

  return normalizeDialogPath(result);
}

export const pickVolumeFile = pickRuntimeStoreFile;
export const pickSegyFile = pickImportSeismicFile;

export async function pickHorizonFiles(): Promise<string[]> {
  if (!isTauriEnvironment()) {
    const result = prompt("Enter horizon xyz paths separated by commas:");
    return normalizeDialogPaths(
      result
        ?.split(",")
        .map((value) => value.trim())
        .filter((value) => value.length > 0) ?? null
    );
  }

  const { open } = await import("@tauri-apps/plugin-dialog");
  const result = await open({
    title: "Import Horizons",
    filters: [
      { name: "Horizon XYZ", extensions: ["xyz"] },
      { name: "All Files", extensions: ["*"] }
    ],
    multiple: true,
    directory: false
  });

  return normalizeDialogPaths(result);
}

export async function pickVelocityFunctionsFile(): Promise<string | null> {
  if (!isTauriEnvironment()) {
    return normalizeDialogPath(prompt("Enter velocity functions path (.txt, .csv):"));
  }

  const { open } = await import("@tauri-apps/plugin-dialog");
  const result = await open({
    title: "Import Velocity Functions",
    filters: [
      { name: "Velocity Functions", extensions: ["txt", "csv"] },
      { name: "All Files", extensions: ["*"] }
    ],
    multiple: false,
    directory: false
  });

  return normalizeDialogPath(result);
}

export async function pickProjectFolder(): Promise<string | null> {
  if (!isTauriEnvironment()) {
    return normalizeDialogPath(prompt("Enter Ophiolite project root:"));
  }

  return normalizeDialogPath(await pickDesktopProjectRoot("Select Ophiolite Project Root"));
}

export async function pickVendorProjectFolder(title = "Select Vendor Project Root"): Promise<string | null> {
  if (!isTauriEnvironment()) {
    return normalizeDialogPath(prompt("Enter vendor project root:"));
  }

  return normalizeDialogPath(await pickDesktopProjectRoot(title));
}

export async function pickWellFolder(): Promise<string | null> {
  if (!isTauriEnvironment()) {
    return normalizeDialogPath(prompt("Enter well folder path:"));
  }

  const { open } = await import("@tauri-apps/plugin-dialog");
  const result = await open({
    title: "Select Well Folder",
    multiple: false,
    directory: true
  });

  return normalizeDialogPath(result);
}

export async function pickWellImportFiles(): Promise<string[]> {
  if (!isTauriEnvironment()) {
    const result = prompt("Enter well import source paths separated by commas:");
    return normalizeDialogPaths(
      result
        ?.split(",")
        .map((value) => value.trim())
        .filter((value) => value.length > 0) ?? null
    );
  }

  const { open } = await import("@tauri-apps/plugin-dialog");
  const result = await open({
    title: "Import Well Files",
    filters: [
      { name: "Well Import Sources", extensions: ["las", "asc", "txt", "csv", "dlis"] },
      { name: "All Files", extensions: ["*"] }
    ],
    multiple: true,
    directory: false
  });

  return normalizeDialogPaths(result);
}

export async function pickWellTimeDepthJsonFile(title = "Import Well Time-Depth JSON"): Promise<string | null> {
  if (!isTauriEnvironment()) {
    return normalizeDialogPath(prompt("Enter well time-depth JSON path:"));
  }

  const { open } = await import("@tauri-apps/plugin-dialog");
  const result = await open({
    title,
    filters: [
      { name: "JSON", extensions: ["json"] },
      { name: "All Files", extensions: ["*"] }
    ],
    multiple: false,
    directory: false
  });

  return normalizeDialogPath(result);
}

/**
 * Opens a native folder/save picker for the runtime store output.
 * Returns the selected path, or null if cancelled.
 */
export async function pickOutputStorePath(defaultPath = "survey.tbvol"): Promise<string | null> {
  if (!isTauriEnvironment()) {
    return normalizeDialogPath(prompt("Enter output store path:"));
  }

  return normalizeDialogPath(await pickDesktopOutputPath(defaultPath, "runtime_store_output"));
}

export const pickOutputFolder = pickOutputStorePath;

export async function pickSegyExportPath(defaultPath = "survey.export.sgy"): Promise<string | null> {
  if (!isTauriEnvironment()) {
    return normalizeDialogPath(prompt("Enter SEG-Y export path:"));
  }

  return normalizeDialogPath(await pickDesktopOutputPath(defaultPath, "segy_export"));
}

export async function pickZarrExportPath(defaultPath = "survey.export.zarr"): Promise<string | null> {
  if (!isTauriEnvironment()) {
    return normalizeDialogPath(prompt("Enter Zarr export path:"));
  }

  return normalizeDialogPath(await pickDesktopOutputPath(defaultPath, "zarr_export"));
}

export async function confirmOverwriteStore(outputStorePath: string): Promise<boolean> {
  const message = [
    "A runtime store already exists at this location.",
    "",
    outputStorePath,
    "",
    "Overwrite it and replace the existing .tbvol store?"
  ].join("\n");

  if (!isTauriEnvironment()) {
    return window.confirm(message);
  }

  const { confirm } = await import("@tauri-apps/plugin-dialog");
  return confirm(message, {
    title: "Overwrite Existing Runtime Store?",
    kind: "warning",
    okLabel: "Overwrite",
    cancelLabel: "Cancel"
  });
}

export async function confirmOverwriteSegy(outputPath: string): Promise<boolean> {
  const message = [
    "A SEG-Y file already exists at this location.",
    "",
    outputPath,
    "",
    "Overwrite it and replace the existing SEG-Y export?"
  ].join("\n");

  if (!isTauriEnvironment()) {
    return window.confirm(message);
  }

  const { confirm } = await import("@tauri-apps/plugin-dialog");
  return confirm(message, {
    title: "Overwrite Existing SEG-Y File?",
    kind: "warning",
    okLabel: "Overwrite",
    cancelLabel: "Cancel"
  });
}

export async function confirmOverwriteZarr(outputPath: string): Promise<boolean> {
  const message = [
    "A Zarr store already exists at this location.",
    "",
    outputPath,
    "",
    "Overwrite it and replace the existing Zarr export?"
  ].join("\n");

  if (!isTauriEnvironment()) {
    return window.confirm(message);
  }

  const { confirm } = await import("@tauri-apps/plugin-dialog");
  return confirm(message, {
    title: "Overwrite Existing Zarr Store?",
    kind: "warning",
    okLabel: "Overwrite",
    cancelLabel: "Cancel"
  });
}
