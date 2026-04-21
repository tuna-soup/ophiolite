<svelte:options runes={true} />

<script lang="ts">
  import {
    AvoInterceptGradientCrossplotChart,
    type AvoInterceptGradientCrossplotChartHandle
  } from "@ophiolite/charts";
  import {
    adaptOphioliteAvoCrossplotToChart,
    validateOphioliteAvoCrossplotSource,
    type OphioliteResolvedAvoCrossplotSource,
    OphioliteAvoValidationError
  } from "@ophiolite/charts-data-models";

  let {
    source,
    chartId = "ophiolite-jupyter-avo-crossplot"
  }: {
    source: OphioliteResolvedAvoCrossplotSource;
    chartId?: string;
  } = $props();

  let chart = $state.raw<AvoInterceptGradientCrossplotChartHandle | null>(null);
  let currentSource = $state.raw<OphioliteResolvedAvoCrossplotSource | null>(null);
  let model = $state.raw<ReturnType<typeof adaptOphioliteAvoCrossplotToChart> | null>(null);
  let errorMessage = $state<string | null>(null);
  let resetToken = $state(0);

  $effect(() => {
    if (currentSource === null) {
      setSource(source);
    }
  });

  export function setSource(nextSource: OphioliteResolvedAvoCrossplotSource): void {
    currentSource = nextSource;
    model = adaptSource(currentSource);
    errorMessage = null;
    resetToken += 1;
  }

  export function fitToData(): void {
    chart?.fitToData();
  }

  function adaptSource(nextSource: OphioliteResolvedAvoCrossplotSource) {
    const issues = validateOphioliteAvoCrossplotSource(nextSource);
    if (issues.length > 0) {
      errorMessage = issues.map((issue) => issue.message).join(" ");
      throw new OphioliteAvoValidationError(issues);
    }
    return adaptOphioliteAvoCrossplotToChart(nextSource);
  }
</script>

<div class="ophiolite-jupyter-chart-host">
  <AvoInterceptGradientCrossplotChart bind:this={chart} {chartId} {model} {errorMessage} {resetToken} />
</div>

<style>
  .ophiolite-jupyter-chart-host {
    width: 100%;
    height: 100%;
    min-height: 320px;
  }
</style>
