import type { SectionAxis } from "@traceboost/seis-contracts";

export type ViewerDisplayDomain = "time" | "depth";
export type ViewerResetReason = "store_switch" | "domain_change" | "geometry_change";

interface ViewerIdentity {
  storePath: string;
  geometryFingerprint: string | null;
  domain: ViewerDisplayDomain;
}

interface ViewerViewportIdentity extends ViewerIdentity {
  axis: SectionAxis;
}

const VIEWER_KEY_VERSION = "v1";

function normalizeStorePath(storePath: string): string {
  const normalized = storePath.trim();
  return normalized.length > 0 ? normalized : "no-store";
}

function normalizeFingerprint(fingerprint: string | null): string {
  const normalized = fingerprint?.trim() ?? "";
  return normalized.length > 0 ? normalized : "none";
}

export function buildViewerSessionKey(identity: ViewerIdentity): string {
  return [
    "viewer",
    VIEWER_KEY_VERSION,
    normalizeStorePath(identity.storePath),
    normalizeFingerprint(identity.geometryFingerprint),
    identity.domain
  ].join(":");
}

export function buildViewportMemoryKey(identity: ViewerViewportIdentity): string {
  return [
    "viewport",
    VIEWER_KEY_VERSION,
    normalizeStorePath(identity.storePath),
    normalizeFingerprint(identity.geometryFingerprint),
    identity.axis,
    identity.domain
  ].join(":");
}

export function resolveViewerResetReason(
  current: ViewerIdentity | null,
  next: ViewerIdentity
): ViewerResetReason | null {
  if (!current || !current.storePath.trim() || !next.storePath.trim()) {
    return null;
  }
  if (current.storePath !== next.storePath) {
    return "store_switch";
  }
  if (current.domain !== next.domain) {
    return "domain_change";
  }
  if (normalizeFingerprint(current.geometryFingerprint) !== normalizeFingerprint(next.geometryFingerprint)) {
    return "geometry_change";
  }
  return null;
}
