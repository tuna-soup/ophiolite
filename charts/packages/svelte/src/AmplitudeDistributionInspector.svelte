<svelte:options runes={true} />

<script lang="ts">
  import AnalysisInspectorShell from "./AnalysisInspectorShell.svelte";
  import AmplitudeDistributionChart from "./AmplitudeDistributionChart.svelte";
  import type { AmplitudeDistributionMarker, AmplitudeDistributionResult } from "./types";

  let {
    floating = false,
    distributionBusy,
    distributionStale,
    distributionError,
    distributionSelectionSummary,
    distributionResult,
    onRefresh,
    onClose
  }: {
    floating?: boolean;
    distributionBusy: boolean;
    distributionStale: boolean;
    distributionError: string | null;
    distributionSelectionSummary: string;
    distributionResult: AmplitudeDistributionResult | null;
    onRefresh: () => void | Promise<void>;
    onClose?: (() => void) | undefined;
  } = $props();

  const markerDefinitions = $derived.by<AmplitudeDistributionMarker[]>(() => {
    if (!distributionResult || distributionResult.count === 0) {
      return [];
    }

    const offset = distributionResult.standardDeviation * 2;
    return [
      {
        id: "minus-two-sigma",
        value: distributionResult.mean - offset,
        label: formatSigned(distributionResult.mean - offset),
        color: "#52c857"
      },
      {
        id: "plus-two-sigma",
        value: distributionResult.mean + offset,
        label: formatSigned(distributionResult.mean + offset),
        color: "#52c857"
      }
    ];
  });

  function formatSigned(value: number): string {
    const rounded = Math.round(value);
    return rounded > 0 ? `+${rounded}` : `${rounded}`;
  }

  function formatValue(value: number): string {
    const magnitude = Math.abs(value);
    if (magnitude >= 1000) {
      return value.toFixed(0);
    }
    if (magnitude >= 100) {
      return value.toFixed(1);
    }
    if (magnitude >= 10) {
      return value.toFixed(2);
    }
    return value.toFixed(4);
  }
</script>

<AnalysisInspectorShell
  {floating}
  ariaLabel="Amplitude distribution inspector"
  eyebrow="Amplitude Distribution"
  title="Amplitude Value Distribution"
  summary={distributionSelectionSummary}
  primaryActionLabel="Refresh"
  primaryActionBusyLabel="Computing..."
  primaryActionBusy={distributionBusy}
  primaryActionDisabled={distributionBusy}
  onPrimaryAction={onRefresh}
  {onClose}
>
  {#snippet notices()}
    {#if distributionStale}
      <div class="ophiolite-charts-analysis-status-bar">
        The displayed section changed after the last computation. The plot below is preserved from the previous section until you press <strong>Refresh</strong> again.
      </div>
    {/if}
  {/snippet}

  {#if distributionResult && distributionResult.count > 0}
    <div class="ophiolite-charts-analysis-chart-shell">
      <AmplitudeDistributionChart
        title="Amplitude Distribution"
        xLabel="Value"
        yLabel="Count"
        bins={distributionResult.bins}
        markers={markerDefinitions}
      />
    </div>

    <div class="distribution-summary-grid">
      <div class="ophiolite-charts-analysis-summary-card wide">
        <span>Value Range</span>
        <strong>{formatValue(distributionResult.min)} to {formatValue(distributionResult.max)}</strong>
      </div>
      <div class="ophiolite-charts-analysis-summary-card">
        <span>Mean</span>
        <strong>{formatValue(distributionResult.mean)}</strong>
      </div>
      <div class="ophiolite-charts-analysis-summary-card">
        <span>Std Deviation</span>
        <strong>{formatValue(distributionResult.standardDeviation)}</strong>
      </div>
      <div class="ophiolite-charts-analysis-summary-card">
        <span>Median</span>
        <strong>{formatValue(distributionResult.median)}</strong>
      </div>
      <div class="ophiolite-charts-analysis-summary-card">
        <span>RMS</span>
        <strong>{formatValue(distributionResult.rms)}</strong>
      </div>
      <div class="ophiolite-charts-analysis-summary-card wide">
        <span>Number of Values</span>
        <strong>{distributionResult.count.toLocaleString()}</strong>
      </div>
    </div>
  {:else}
    <div class="ophiolite-charts-analysis-empty-state">
      <strong>No distribution computed yet</strong>
      <p>Compute a histogram and summary statistics from the currently displayed section amplitudes.</p>
    </div>
  {/if}

  {#if distributionError}
    <div class="ophiolite-charts-analysis-error-bar">{distributionError}</div>
  {/if}
</AnalysisInspectorShell>

<style>
  .distribution-summary-grid {
    display: grid;
    grid-template-columns: repeat(2, minmax(0, 1fr));
    gap: 8px;
  }

  @media (max-width: 900px) {
    .distribution-summary-grid {
      grid-template-columns: 1fr;
    }
  }
</style>
