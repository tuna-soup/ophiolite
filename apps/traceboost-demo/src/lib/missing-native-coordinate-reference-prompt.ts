export interface MissingNativeCoordinateReferencePromptInput {
  makeActive: boolean;
  promptRequested: boolean;
  restoringWorkspace: boolean;
  storePath: string | null | undefined;
  effectiveCoordinateReferenceId: string | null | undefined;
  effectiveCoordinateReferenceName: string | null | undefined;
  acceptedNativeEngineeringStorePaths: ReadonlySet<string>;
}

function normalizeValue(value: string | null | undefined): string {
  return value?.trim() ?? "";
}

export function shouldPromptForMissingNativeCoordinateReference(
  input: MissingNativeCoordinateReferencePromptInput
): boolean {
  if (!input.makeActive || !input.promptRequested || input.restoringWorkspace) {
    return false;
  }

  const normalizedStorePath = normalizeValue(input.storePath);
  if (
    !normalizedStorePath ||
    input.acceptedNativeEngineeringStorePaths.has(normalizedStorePath)
  ) {
    return false;
  }

  if (
    normalizeValue(input.effectiveCoordinateReferenceId) ||
    normalizeValue(input.effectiveCoordinateReferenceName)
  ) {
    return false;
  }

  return true;
}
