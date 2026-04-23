function normalizePath(id: string): string {
  return id.replace(/\\/g, "/");
}

export function chartsManualChunks(id: string): string | undefined {
  const normalized = normalizePath(id);

  if (normalized.includes("/node_modules/@kitware/vtk.js/")) {
    return "vendor-vtk";
  }
  if (normalized.includes("/node_modules/wslink/")) {
    return "vendor-wslink";
  }
  if (normalized.includes("/node_modules/xmlbuilder2/")) {
    return "vendor-xmlbuilder";
  }
  if (normalized.includes("/node_modules/gl-matrix/")) {
    return "vendor-gl-matrix";
  }
  if (normalized.includes("/node_modules/fflate/")) {
    return "vendor-fflate";
  }
  if (normalized.includes("/node_modules/utif/")) {
    return "vendor-utif";
  }

  if (normalized.includes("/node_modules/svelte/")) {
    return "vendor-svelte";
  }
  if (normalized.includes("/node_modules/")) {
    return "vendor";
  }

  if (normalized.includes("/packages/renderer/")) {
    return "charts-renderer";
  }
  if (normalized.includes("/packages/domain-geoscience/")) {
    return "charts-domain";
  }
  if (normalized.includes("/packages/chart-core/")) {
    return "charts-core";
  }
  if (normalized.includes("/packages/data-models/")) {
    return "charts-data-models";
  }
  if (normalized.includes("/packages/toolbar/")) {
    return "charts-toolbar";
  }
  if (normalized.includes("/packages/svelte/")) {
    return "charts-svelte";
  }

  return undefined;
}
