import { createContext } from "svelte";
import type {
  BeginImportSessionRequest,
  ImportProviderDescriptor,
  ImportProviderId,
  ImportSessionEnvelope
} from "./bridge";
import { beginImportSession, listImportProviders } from "./bridge";

export type ImportManagerPendingAction = "open" | "import" | "deep_link";

export interface OpenImportManagerOptions {
  providerId?: ImportProviderId | null;
  sourceRefs?: string[];
  pendingAction?: ImportManagerPendingAction;
}

function normalizeSourceRefs(sourceRefs: string[] | null | undefined): string[] {
  const normalized: string[] = [];
  for (const sourceRef of sourceRefs ?? []) {
    const trimmed = sourceRef.trim();
    if (trimmed && !normalized.includes(trimmed)) {
      normalized.push(trimmed);
    }
  }
  return normalized;
}

export function commonWellImportRoot(inputPaths: string[]): string | null {
  const normalized = normalizeSourceRefs(inputPaths).map((value) => value.replace(/\\/g, "/"));
  if (normalized.length === 0) {
    return null;
  }

  const splitParent = (value: string): string[] => {
    const parts = value.split("/");
    parts.pop();
    return parts.filter((part, index) => part.length > 0 || index === 0);
  };

  let shared = splitParent(normalized[0]!);
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
    const fallbackPath = normalized[0]!;
    const separatorIndex = fallbackPath.lastIndexOf("/");
    return separatorIndex > 0 ? fallbackPath.slice(0, separatorIndex) : null;
  }
  return shared.join("/") || null;
}

export class ImportManagerModel {
  open = $state(false);
  loading = $state(false);
  error = $state<string | null>(null);
  providers = $state.raw<ImportProviderDescriptor[]>([]);
  activeProviderId = $state<ImportProviderId | null>(null);
  pendingAction = $state<ImportManagerPendingAction>("import");
  session = $state.raw<ImportSessionEnvelope | null>(null);
  sourceRefs = $state.raw<string[]>([]);

  get currentProvider(): ImportProviderDescriptor | null {
    if (!this.activeProviderId) {
      return null;
    }
    return this.providers.find((provider) => provider.providerId === this.activeProviderId) ?? null;
  }

  get currentSourceRefs(): string[] {
    return this.session?.sourceRefs ?? this.sourceRefs;
  }

  get sourceCount(): number {
    return this.currentSourceRefs.length;
  }

  get currentSourceRef(): string | null {
    return this.currentSourceRefs[0] ?? null;
  }

  async ensureProviders(): Promise<void> {
    if (this.providers.length > 0) {
      return;
    }
    this.providers = await listImportProviders();
  }

  async openManager(options: OpenImportManagerOptions = {}): Promise<void> {
    this.open = true;
    this.error = null;
    this.pendingAction = options.pendingAction ?? "import";
    this.sourceRefs = normalizeSourceRefs(options.sourceRefs);
    await this.ensureProviders();

    const nextProviderId =
      options.providerId ??
      this.activeProviderId ??
      (this.providers.length === 1 ? this.providers[0]!.providerId : null);
    this.activeProviderId = nextProviderId ?? null;

    if (this.activeProviderId) {
      await this.startSession({
        providerId: this.activeProviderId,
        sourceRefs: this.sourceRefs
      });
      return;
    }

    this.session = null;
  }

  closeManager(): void {
    this.open = false;
    this.error = null;
    this.session = null;
    this.sourceRefs = [];
    this.pendingAction = "import";
  }

  async selectProvider(providerId: ImportProviderId): Promise<void> {
    if (providerId === this.activeProviderId && this.session) {
      return;
    }
    const nextSourceRefs = providerId === this.activeProviderId ? this.sourceRefs : [];
    this.activeProviderId = providerId;
    this.sourceRefs = nextSourceRefs;
    this.error = null;
    await this.startSession({
      providerId,
      sourceRefs: nextSourceRefs
    });
  }

  async replaceSourceRefs(sourceRefs: string[]): Promise<void> {
    this.sourceRefs = normalizeSourceRefs(sourceRefs);
    if (!this.activeProviderId) {
      this.session = null;
      return;
    }
    await this.startSession({
      providerId: this.activeProviderId,
      sourceRefs: this.sourceRefs
    });
  }

  async clearSourceRefs(): Promise<void> {
    await this.replaceSourceRefs([]);
  }

  private async startSession(request: BeginImportSessionRequest): Promise<void> {
    this.loading = true;
    this.error = null;
    try {
      this.session = await beginImportSession(request);
    } catch (error) {
      this.session = null;
      this.error = error instanceof Error ? error.message : String(error);
    } finally {
      this.loading = false;
    }
  }
}

const [internalGetImportManagerContext, internalSetImportManagerContext] =
  createContext<ImportManagerModel>();

export function getImportManagerContext(): ImportManagerModel {
  const importManager = internalGetImportManagerContext();
  if (!importManager) {
    throw new Error("Import manager context not found");
  }
  return importManager;
}

export function setImportManagerContext(importManager: ImportManagerModel): ImportManagerModel {
  internalSetImportManagerContext(importManager);
  return importManager;
}
