import fs from "node:fs";
import path from "node:path";

type SupportTier = "public-launch" | "public-adapter" | "preview" | "internal";
type Layer = "models" | "core" | "renderer" | "domain" | "wrapper" | "toolbar";
type DependencyRole = "runtime" | "adapter-runtime" | "peer" | "preview-runtime" | "test-only";

type ManifestEntrypoint = {
  subpath: string;
  tier: SupportTier;
  description: string;
};

type ManifestDependency = {
  id: string;
  role: DependencyRole;
  optional?: boolean;
};

type ModuleManifest = {
  $schema: string;
  kind: "ophioliteChartsModule";
  id: string;
  displayName: string;
  layer: Layer;
  supportTier: SupportTier;
  description: string;
  entrypoints: ManifestEntrypoint[];
  dependsOn: ManifestDependency[];
  testPolicy: {
    requiredSuites: string[];
    releaseBlocking: boolean;
  };
  consumerGuarantees: {
    traceboostApproved: boolean;
    externalConsumerApproved: boolean;
  };
  notes?: string[];
};

type PackageJson = {
  name: string;
  exports?: string | Record<string, unknown>;
  dependencies?: Record<string, string>;
  peerDependencies?: Record<string, string>;
  devDependencies?: Record<string, string>;
};

type LoadedManifest = {
  manifestPath: string;
  packageDir: string;
  manifest: ModuleManifest;
};

type GeneratedPackageCatalogEntry = {
  packageName: string;
  displayName: string;
  manifestPath: string;
  packageDir: string;
  layer: Layer;
  tier: SupportTier;
  description: string;
  entrypoints: ManifestEntrypoint[];
  dependencies: ManifestDependency[];
  releaseBlocking: boolean;
  requiredSuites: string[];
  traceboostApproved: boolean;
  externalConsumerApproved: boolean;
  notes: string[];
};

type GeneratedSurfaceCatalogEntry = {
  surface: string;
  packageName: string;
  manifestPath: string;
  layer: Layer;
  tier: SupportTier;
  role: string;
};

type GeneratedSupportTierEntry = {
  tier: SupportTier;
  packageCount: number;
  entrypointCount: number;
  packages: string[];
};

type GeneratedCatalog = {
  supportTiers: GeneratedSupportTierEntry[];
  packageCatalog: GeneratedPackageCatalogEntry[];
  surfaceCatalog: GeneratedSurfaceCatalogEntry[];
};

const supportTierOrder: SupportTier[] = ["public-launch", "public-adapter", "preview", "internal"];
const supportTiers = new Set<SupportTier>(supportTierOrder);
const layers = new Set<Layer>(["models", "core", "renderer", "domain", "wrapper", "toolbar"]);
const dependencyRoles = new Set<DependencyRole>([
  "runtime",
  "adapter-runtime",
  "peer",
  "preview-runtime",
  "test-only"
]);

const repoRoot = path.resolve(import.meta.dir, "..");
const expectedPackages = [
  "packages/data-models",
  "packages/chart-core",
  "packages/renderer",
  "packages/domain-geoscience",
  "packages/svelte",
  "packages/svelte-toolbar"
] as const;
const schemaPath = path.join(repoRoot, "manifests/schemas/ophiolite-charts-module.schema.json");
const generatedJsonPath = path.join(repoRoot, "manifests/generated/module-catalog.json");
const generatedTsPath = path.join(repoRoot, "apps/public-docs/src/lib/generated/manifest-catalog.ts");

function readJson<T>(filePath: string): T {
  return JSON.parse(fs.readFileSync(filePath, "utf8")) as T;
}

function normalizeExports(exportsField: PackageJson["exports"]): string[] {
  if (!exportsField) {
    return [];
  }

  if (typeof exportsField === "string") {
    return ["."];
  }

  return Object.keys(exportsField).sort();
}

function asRecord(value: unknown): Record<string, unknown> | null {
  if (!value || typeof value !== "object" || Array.isArray(value)) {
    return null;
  }

  return value as Record<string, unknown>;
}

function pushError(errors: string[], manifestPath: string, message: string) {
  errors.push(`${path.relative(repoRoot, manifestPath)}: ${message}`);
}

function validateManifestShape(manifest: Record<string, unknown>, manifestPath: string, errors: string[]) {
  const requiredKeys = [
    "$schema",
    "kind",
    "id",
    "displayName",
    "layer",
    "supportTier",
    "description",
    "entrypoints",
    "dependsOn",
    "testPolicy",
    "consumerGuarantees"
  ];

  for (const key of requiredKeys) {
    if (!(key in manifest)) {
      pushError(errors, manifestPath, `missing required field "${key}"`);
    }
  }
}

function validateManifest(packageDir: string, errors: string[], summaries: string[], loadedManifests: LoadedManifest[]) {
  const manifestPath = path.join(repoRoot, packageDir, "ophiolite.module.json");
  const packageJsonPath = path.join(repoRoot, packageDir, "package.json");

  if (!fs.existsSync(manifestPath)) {
    pushError(errors, manifestPath, "manifest file is missing");
    return;
  }

  const rawManifest = readJson<Record<string, unknown>>(manifestPath);
  validateManifestShape(rawManifest, manifestPath, errors);

  const manifest = rawManifest as ModuleManifest;
  const pkg = readJson<PackageJson>(packageJsonPath);

  if (manifest.$schema !== "../../manifests/schemas/ophiolite-charts-module.schema.json") {
    pushError(errors, manifestPath, 'expected "$schema" to point at ../../manifests/schemas/ophiolite-charts-module.schema.json');
  }

  if (manifest.kind !== "ophioliteChartsModule") {
    pushError(errors, manifestPath, 'expected kind "ophioliteChartsModule"');
  }

  if (manifest.id !== pkg.name) {
    pushError(errors, manifestPath, `manifest id "${manifest.id}" does not match package name "${pkg.name}"`);
  }

  if (!layers.has(manifest.layer)) {
    pushError(errors, manifestPath, `unsupported layer "${String(manifest.layer)}"`);
  }

  if (!supportTiers.has(manifest.supportTier)) {
    pushError(errors, manifestPath, `unsupported supportTier "${String(manifest.supportTier)}"`);
  }

  if (typeof manifest.displayName !== "string" || manifest.displayName.trim().length === 0) {
    pushError(errors, manifestPath, "displayName must be a non-empty string");
  }

  if (typeof manifest.description !== "string" || manifest.description.trim().length === 0) {
    pushError(errors, manifestPath, "description must be a non-empty string");
  }

  if (!Array.isArray(manifest.entrypoints) || manifest.entrypoints.length === 0) {
    pushError(errors, manifestPath, "entrypoints must be a non-empty array");
  }

  const packageExports = normalizeExports(pkg.exports);
  const manifestSubpaths = Array.isArray(manifest.entrypoints)
    ? [...new Set(manifest.entrypoints.map((entry) => entry.subpath))].sort()
    : [];

  if (packageExports.length !== manifestSubpaths.length || packageExports.some((entry, index) => entry !== manifestSubpaths[index])) {
    pushError(
      errors,
      manifestPath,
      `entrypoints ${JSON.stringify(manifestSubpaths)} do not match package exports ${JSON.stringify(packageExports)}`
    );
  }

  for (const entry of manifest.entrypoints ?? []) {
    if (typeof entry.subpath !== "string" || entry.subpath.length === 0) {
      pushError(errors, manifestPath, "every entrypoint must define a non-empty subpath");
    }
    if (!supportTiers.has(entry.tier)) {
      pushError(errors, manifestPath, `entrypoint "${entry.subpath}" uses unsupported tier "${String(entry.tier)}"`);
    }
    if (typeof entry.description !== "string" || entry.description.trim().length === 0) {
      pushError(errors, manifestPath, `entrypoint "${entry.subpath}" must include a description`);
    }
  }

  const dependencyKeys = new Set([
    ...Object.keys(pkg.dependencies ?? {}),
    ...Object.keys(pkg.peerDependencies ?? {}),
    ...Object.keys(pkg.devDependencies ?? {})
  ]);

  for (const dependency of manifest.dependsOn ?? []) {
    if (typeof dependency.id !== "string" || dependency.id.length === 0) {
      pushError(errors, manifestPath, "every dependency record must include a non-empty id");
    }
    if (!dependencyRoles.has(dependency.role)) {
      pushError(errors, manifestPath, `dependency "${dependency.id}" uses unsupported role "${String(dependency.role)}"`);
    }
    if (!dependencyKeys.has(dependency.id)) {
      pushError(errors, manifestPath, `dependency "${dependency.id}" is not declared in package.json dependencies or peerDependencies`);
    }
    if ("optional" in dependency && typeof dependency.optional !== "boolean") {
      pushError(errors, manifestPath, `dependency "${dependency.id}" optional flag must be boolean`);
    }
  }

  if (!Array.isArray(manifest.testPolicy?.requiredSuites) || manifest.testPolicy.requiredSuites.length === 0) {
    pushError(errors, manifestPath, "testPolicy.requiredSuites must be a non-empty array");
  }

  if (typeof manifest.testPolicy?.releaseBlocking !== "boolean") {
    pushError(errors, manifestPath, "testPolicy.releaseBlocking must be boolean");
  }

  if (typeof manifest.consumerGuarantees?.traceboostApproved !== "boolean") {
    pushError(errors, manifestPath, "consumerGuarantees.traceboostApproved must be boolean");
  }

  if (typeof manifest.consumerGuarantees?.externalConsumerApproved !== "boolean") {
    pushError(errors, manifestPath, "consumerGuarantees.externalConsumerApproved must be boolean");
  }

  if (manifest.supportTier === "internal" && manifest.consumerGuarantees?.externalConsumerApproved) {
    pushError(errors, manifestPath, "internal packages cannot claim externalConsumerApproved");
  }

  const publicEntrypoints = (manifest.entrypoints ?? []).filter(
    (entry) => entry.tier === "public-launch" || entry.tier === "public-adapter"
  );
  if (publicEntrypoints.length > 0 && !manifest.consumerGuarantees?.externalConsumerApproved) {
    pushError(errors, manifestPath, "packages with public entrypoints must claim externalConsumerApproved");
  }

  for (const entry of manifest.entrypoints ?? []) {
    if (entry.subpath === "./adapters/ophiolite" && entry.tier !== "public-adapter") {
      pushError(errors, manifestPath, "./adapters/ophiolite must use tier public-adapter");
    }
    if ((entry.subpath === "./preview" || entry.subpath === "./extras") && entry.tier === "public-launch") {
      pushError(errors, manifestPath, `${entry.subpath} cannot use tier public-launch`);
    }
  }

  const testPolicyRecord = asRecord(manifest.testPolicy);
  const consumerGuaranteesRecord = asRecord(manifest.consumerGuarantees);
  if (!testPolicyRecord) {
    pushError(errors, manifestPath, "testPolicy must be an object");
  }
  if (!consumerGuaranteesRecord) {
    pushError(errors, manifestPath, "consumerGuarantees must be an object");
  }

  summaries.push(
    `${manifest.id} [${manifest.supportTier}] -> ${manifest.entrypoints.map((entry) => `${entry.subpath}:${entry.tier}`).join(", ")}`
  );

  loadedManifests.push({
    manifestPath,
    packageDir,
    manifest
  });
}

function formatSurfaceName(packageName: string, subpath: string) {
  return subpath === "." ? packageName : `${packageName}/${subpath.slice(2)}`;
}

function buildGeneratedCatalog(loadedManifests: LoadedManifest[]): GeneratedCatalog {
  const packageCatalog = loadedManifests
    .map(({ manifest, manifestPath, packageDir }) => ({
      packageName: manifest.id,
      displayName: manifest.displayName,
      manifestPath: path.relative(repoRoot, manifestPath),
      packageDir,
      layer: manifest.layer,
      tier: manifest.supportTier,
      description: manifest.description,
      entrypoints: manifest.entrypoints.map((entrypoint) => ({ ...entrypoint })),
      dependencies: manifest.dependsOn.map((dependency) => ({ ...dependency })),
      releaseBlocking: manifest.testPolicy.releaseBlocking,
      requiredSuites: [...manifest.testPolicy.requiredSuites],
      traceboostApproved: manifest.consumerGuarantees.traceboostApproved,
      externalConsumerApproved: manifest.consumerGuarantees.externalConsumerApproved,
      notes: [...(manifest.notes ?? [])]
    }))
    .sort((left, right) => left.packageName.localeCompare(right.packageName));

  const surfaceCatalog = packageCatalog
    .flatMap((pkg) =>
      pkg.entrypoints.map((entrypoint) => ({
        surface: formatSurfaceName(pkg.packageName, entrypoint.subpath),
        packageName: pkg.packageName,
        manifestPath: pkg.manifestPath,
        layer: pkg.layer,
        tier: entrypoint.tier,
        role: entrypoint.description
      }))
    )
    .sort((left, right) => left.surface.localeCompare(right.surface));

  const supportTierCatalog = supportTierOrder.map((tier) => {
    const packages = packageCatalog.filter((pkg) => pkg.tier === tier).map((pkg) => pkg.packageName);
    const entrypointCount = surfaceCatalog.filter((entry) => entry.tier === tier).length;

    return {
      tier,
      packageCount: packages.length,
      entrypointCount,
      packages
    };
  });

  return {
    supportTiers: supportTierCatalog,
    packageCatalog,
    surfaceCatalog
  };
}

function ensureDirectory(filePath: string) {
  fs.mkdirSync(path.dirname(filePath), { recursive: true });
}

function writeGeneratedOutputs(catalog: GeneratedCatalog) {
  ensureDirectory(generatedJsonPath);
  ensureDirectory(generatedTsPath);

  const jsonBody = `${JSON.stringify(catalog, null, 2)}\n`;
  const tsBody = `export type SupportTier = "public-launch" | "public-adapter" | "preview" | "internal";
export type Layer = "models" | "core" | "renderer" | "domain" | "wrapper" | "toolbar";

export type ManifestCatalogEntrypoint = {
  subpath: string;
  tier: SupportTier;
  description: string;
};

export type ManifestCatalogDependency = {
  id: string;
  role: "runtime" | "adapter-runtime" | "peer" | "preview-runtime" | "test-only";
  optional?: boolean;
};

export type SupportTierCatalogEntry = {
  tier: SupportTier;
  packageCount: number;
  entrypointCount: number;
  packages: string[];
};

export type PackageCatalogEntry = {
  packageName: string;
  displayName: string;
  manifestPath: string;
  packageDir: string;
  layer: Layer;
  tier: SupportTier;
  description: string;
  entrypoints: ManifestCatalogEntrypoint[];
  dependencies: ManifestCatalogDependency[];
  releaseBlocking: boolean;
  requiredSuites: string[];
  traceboostApproved: boolean;
  externalConsumerApproved: boolean;
  notes: string[];
};

export type SurfaceCatalogEntry = {
  surface: string;
  packageName: string;
  manifestPath: string;
  layer: Layer;
  tier: SupportTier;
  role: string;
};

export type ManifestCatalog = {
  supportTiers: SupportTierCatalogEntry[];
  packageCatalog: PackageCatalogEntry[];
  surfaceCatalog: SurfaceCatalogEntry[];
};

export const manifestCatalog: ManifestCatalog = ${JSON.stringify(catalog, null, 2)};\n`;

  fs.writeFileSync(generatedJsonPath, jsonBody);
  fs.writeFileSync(generatedTsPath, tsBody);
}

if (!fs.existsSync(schemaPath)) {
  console.error(`Missing schema: ${path.relative(repoRoot, schemaPath)}`);
  process.exit(1);
}

const errors: string[] = [];
const summaries: string[] = [];
const loadedManifests: LoadedManifest[] = [];

for (const packageDir of expectedPackages) {
  validateManifest(packageDir, errors, summaries, loadedManifests);
}

if (errors.length > 0) {
  console.error("Module manifest validation failed:\n");
  for (const error of errors) {
    console.error(`- ${error}`);
  }
  process.exit(1);
}

const generatedCatalog = buildGeneratedCatalog(loadedManifests);
writeGeneratedOutputs(generatedCatalog);

console.log("Validated Ophiolite Charts module manifests:");
for (const summary of summaries) {
  console.log(`- ${summary}`);
}
console.log(`Wrote ${path.relative(repoRoot, generatedJsonPath)}`);
console.log(`Wrote ${path.relative(repoRoot, generatedTsPath)}`);
