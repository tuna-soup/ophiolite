<svelte:options runes={true} />

<script lang="ts">
  import {
    AvoResponseChart,
    type AvoResponseChartHandle
  } from "@ophiolite/charts";
  import {
    adaptOphioliteAvoResponseToChart,
    validateOphioliteAvoResponseSource,
    type OphioliteResolvedAvoResponseSource,
    OphioliteAvoValidationError
  } from "@ophiolite/charts-data-models";

  let {
    source,
    chartId = "ophiolite-jupyter-avo-response"
  }: {
    source: OphioliteResolvedAvoResponseSource;
    chartId?: string;
  } = $props();

  let chart = $state.raw<AvoResponseChartHandle | null>(null);
  let currentSource = $state.raw<OphioliteResolvedAvoResponseSource | null>(null);
  let model = $state.raw<ReturnType<typeof adaptOphioliteAvoResponseToChart> | null>(null);
  let errorMessage = $state<string | null>(null);
  let resetToken = $state(0);

  $effect(() => {
    if (currentSource === null) {
      setSource(source);
    }
  });

  export function setSource(nextSource: OphioliteResolvedAvoResponseSource): void {
    currentSource = nextSource;
    model = adaptSource(currentSource);
    errorMessage = null;
    resetToken += 1;
  }

  export function fitToData(): void {
    chart?.fitToData();
  }

  function adaptSource(nextSource: OphioliteResolvedAvoResponseSource) {
    const issues = validateOphioliteAvoResponseSource(nextSource);
    if (issues.length > 0) {
      errorMessage = issues.map((issue) => issue.message).join(" ");
      throw new OphioliteAvoValidationError(issues);
    }
    return adaptOphioliteAvoResponseToChart(nextSource);
  }
</script>

<div class="ophiolite-jupyter-chart-host">
  <AvoResponseChart bind:this={chart} {chartId} {model} {errorMessage} {resetToken} />
</div>

<style>
  .ophiolite-jupyter-chart-host {
    width: 100%;
    height: 100%;
    min-height: 320px;
  }
</style>
