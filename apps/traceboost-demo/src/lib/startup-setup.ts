export interface StartupSetupBlockersInput {
  workspaceReady: boolean;
  hasProjectRoot: boolean;
  projectGeospatialSettingsResolved: boolean;
  hasActiveStore: boolean;
  activeEffectiveNativeCoordinateReferenceId: string | null | undefined;
  activeEffectiveNativeCoordinateReferenceName: string | null | undefined;
}

export function buildStartupSetupBlockers(input: StartupSetupBlockersInput): string[] {
  if (!input.workspaceReady) {
    return [];
  }

  return [];
}
