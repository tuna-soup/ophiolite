import path from "node:path";
import type { Alias } from "vite";

export function resolveChartSourceAliases(configDir: string): Alias[] {
  return [
    {
      find: "@ophiolite/contracts",
      replacement: path.resolve(configDir, "../../../contracts/ts/ophiolite-contracts/src/index.ts")
    },
    {
      find: "@ophiolite/charts-data-models",
      replacement: path.resolve(configDir, "../../packages/data-models/src/index.ts")
    },
    {
      find: "@ophiolite/charts-core",
      replacement: path.resolve(configDir, "../../packages/chart-core/src/index.ts")
    },
    {
      find: "@ophiolite/charts-renderer/preview",
      replacement: path.resolve(configDir, "../../packages/renderer/src/preview.ts")
    },
    {
      find: "@ophiolite/charts-renderer",
      replacement: path.resolve(configDir, "../../packages/renderer/src/index.ts")
    },
    {
      find: "@ophiolite/charts-domain/preview",
      replacement: path.resolve(configDir, "../../packages/domain-geoscience/src/preview.ts")
    },
    {
      find: "@ophiolite/charts-domain",
      replacement: path.resolve(configDir, "../../packages/domain-geoscience/src/index.ts")
    }
  ];
}

export function resolveSvelteChartSourceAliases(configDir: string): Alias[] {
  return [
    ...resolveChartSourceAliases(configDir),
    {
      find: "@ophiolite/charts/preview",
      replacement: path.resolve(configDir, "../../packages/svelte/src/preview.ts")
    },
    {
      find: "@ophiolite/charts/extras",
      replacement: path.resolve(configDir, "../../packages/svelte/src/extras.ts")
    },
    {
      find: "@ophiolite/charts/adapters/ophiolite",
      replacement: path.resolve(configDir, "../../packages/svelte/src/adapters/ophiolite.ts")
    },
    {
      find: "@ophiolite/charts/contracts",
      replacement: path.resolve(configDir, "../../packages/svelte/src/contracts.ts")
    },
    {
      find: "@ophiolite/charts/types",
      replacement: path.resolve(configDir, "../../packages/svelte/src/types.ts")
    },
    {
      find: "@ophiolite/charts",
      replacement: path.resolve(configDir, "../../packages/svelte/src/index.ts")
    },
    {
      find: "@ophiolite/charts-toolbar",
      replacement: path.resolve(configDir, "../../packages/svelte-toolbar/src/index.ts")
    }
  ];
}
