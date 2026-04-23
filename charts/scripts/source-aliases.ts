import path from "node:path";

export function resolveChartSourceAliases(configDir: string): Record<string, string> {
  return {
    "@ophiolite/contracts": path.resolve(configDir, "../../../contracts/ts/ophiolite-contracts/src/index.ts"),
    "@ophiolite/charts-data-models": path.resolve(configDir, "../../packages/data-models/src/index.ts"),
    "@ophiolite/charts-core": path.resolve(configDir, "../../packages/chart-core/src/index.ts"),
    "@ophiolite/charts-renderer": path.resolve(configDir, "../../packages/renderer/src/index.ts"),
    "@ophiolite/charts-domain": path.resolve(configDir, "../../packages/domain-geoscience/src/index.ts")
  };
}

export function resolveSvelteChartSourceAliases(configDir: string): Record<string, string> {
  return {
    ...resolveChartSourceAliases(configDir),
    "@ophiolite/charts": path.resolve(configDir, "../../packages/svelte/src/index.ts"),
    "@ophiolite/charts-toolbar": path.resolve(configDir, "../../packages/svelte-toolbar/src/index.ts")
  };
}
