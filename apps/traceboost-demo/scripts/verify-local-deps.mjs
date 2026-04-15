import { access } from "node:fs/promises";
import path from "node:path";
import { fileURLToPath } from "node:url";

const scriptDir = path.dirname(fileURLToPath(import.meta.url));
const frontendRoot = path.resolve(scriptDir, "..");

const requiredPaths = [
  {
    label: "@traceboost/seis-contracts",
    path: path.resolve(frontendRoot, "../../traceboost/contracts/ts/seis-contracts/package.json"),
    fix: "Restore the in-repo TraceBoost demo contracts package under traceboost/contracts/ts/seis-contracts."
  },
  {
    label: "@ophiolite/charts",
    path: path.resolve(frontendRoot, "../../charts/packages/svelte/package.json"),
    fix: "Restore the in-repo Ophiolite charts package under charts/packages/svelte."
  },
  {
    label: "@ophiolite/charts-data-models",
    path: path.resolve(frontendRoot, "../../charts/packages/data-models/package.json"),
    fix: "Restore the in-repo Ophiolite charts package under charts/packages/data-models."
  },
  {
    label: "@ophiolite/charts-core",
    path: path.resolve(frontendRoot, "../../charts/packages/chart-core/package.json"),
    fix: "Restore the in-repo Ophiolite charts package under charts/packages/chart-core."
  },
  {
    label: "@ophiolite/charts-domain",
    path: path.resolve(frontendRoot, "../../charts/packages/domain-geoscience/package.json"),
    fix: "Restore the in-repo Ophiolite charts package under charts/packages/domain-geoscience."
  },
  {
    label: "@ophiolite/charts-renderer",
    path: path.resolve(frontendRoot, "../../charts/packages/renderer/package.json"),
    fix: "Restore the in-repo Ophiolite charts package under charts/packages/renderer."
  },
  {
    label: "@ophiolite/contracts",
    path: path.resolve(frontendRoot, "../../contracts/ts/ophiolite-contracts/package.json"),
    fix: "Restore the in-repo Ophiolite contracts package under contracts/ts/ophiolite-contracts."
  }
];

const missing = [];

for (const entry of requiredPaths) {
  try {
    await access(entry.path);
  } catch {
    missing.push(entry);
  }
}

if (missing.length > 0) {
  console.error("Missing local development dependencies for traceboost-demo:");
  for (const entry of missing) {
    console.error(`- ${entry.label}: ${entry.path}`);
    console.error(`  ${entry.fix}`);
  }
  process.exit(1);
}

console.log("Local development dependencies are present for traceboost-demo.");
