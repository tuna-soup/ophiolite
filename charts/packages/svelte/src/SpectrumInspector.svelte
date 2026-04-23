<svelte:options runes={true} />

<script lang="ts">
  import AnalysisInspectorShell from "./AnalysisInspectorShell.svelte";
  import SpectrumChart from "./SpectrumChart.svelte";
  import WaveletChart from "./WaveletChart.svelte";
  import { deriveZeroPhaseWavelet } from "./spectrum-wavelet";
  import type { SpectrumAmplitudeScale, SpectrumResponseLike } from "./types";

  let {
    floating = false,
    canInspectSpectrum,
    spectrumBusy,
    spectrumStale,
    spectrumError,
    spectrumSelectionSummary,
    spectrumAmplitudeScale,
    rawSpectrum,
    processedSpectrum,
    onSetSpectrumAmplitudeScale,
    onRefreshSpectrum,
    onClose
  }: {
    floating?: boolean;
    canInspectSpectrum: boolean;
    spectrumBusy: boolean;
    spectrumStale: boolean;
    spectrumError: string | null;
    spectrumSelectionSummary: string;
    spectrumAmplitudeScale: SpectrumAmplitudeScale;
    rawSpectrum: SpectrumResponseLike | null;
    processedSpectrum: SpectrumResponseLike | null;
    onSetSpectrumAmplitudeScale: (scale: SpectrumAmplitudeScale) => void;
    onRefreshSpectrum: () => void | Promise<void>;
    onClose?: (() => void) | undefined;
  } = $props();

  const chartSeries = $derived.by(() => {
    const series = [];

    if (rawSpectrum) {
      series.push({
        id: "raw",
        label: "Raw",
        color: "#8ca2b3",
        fillColor: "rgba(140, 162, 179, 0.18)",
        frequenciesHz: rawSpectrum.curve.frequenciesHz,
        amplitudes: rawSpectrum.curve.amplitudes
      });
    }

    if (processedSpectrum) {
      series.push({
        id: "processed",
        label: processedSpectrum.processingLabel ? `Processed - ${processedSpectrum.processingLabel}` : "Processed",
        color: "#4dd48b",
        fillColor: "rgba(77, 212, 139, 0.18)",
        frequenciesHz: processedSpectrum.curve.frequenciesHz,
        amplitudes: processedSpectrum.curve.amplitudes
      });
    }

    return series;
  });
  const bins = $derived(rawSpectrum?.curve.frequenciesHz.length ?? processedSpectrum?.curve.frequenciesHz.length ?? 0);
  const nyquistHz = $derived(rawSpectrum?.curve.frequenciesHz.at(-1) ?? processedSpectrum?.curve.frequenciesHz.at(-1) ?? 0);
  const sampleIntervalMs = $derived(rawSpectrum?.sampleIntervalMs ?? processedSpectrum?.sampleIntervalMs ?? 0);
  const rawWavelet = $derived(deriveZeroPhaseWavelet(rawSpectrum));
  const processedWavelet = $derived(deriveZeroPhaseWavelet(processedSpectrum));
  const waveletSeries = $derived.by(() => {
    const series = [];

    if (rawWavelet) {
      series.push({
        id: "raw",
        label: "Raw",
        color: "#8ca2b3",
        timesMs: rawWavelet.timesMs,
        amplitudes: rawWavelet.amplitudes
      });
    }

    if (processedWavelet) {
      series.push({
        id: "processed",
        label: processedSpectrum?.processingLabel ? `Processed - ${processedSpectrum.processingLabel}` : "Processed",
        color: "#4dd48b",
        timesMs: processedWavelet.timesMs,
        amplitudes: processedWavelet.amplitudes
      });
    }

    return series;
  });
  const dominantFrequencyHz = $derived(
    processedWavelet?.dominantFrequencyHz ?? rawWavelet?.dominantFrequencyHz ?? null
  );
</script>

<AnalysisInspectorShell
  {floating}
  ariaLabel="Frequency spectrum inspector"
  eyebrow="Frequency Spectrum"
  title="Amplitude Spectrum"
  summary={spectrumSelectionSummary}
  primaryActionLabel="Derive Spectrum"
  primaryActionBusyLabel="Deriving..."
  primaryActionBusy={spectrumBusy}
  primaryActionDisabled={!canInspectSpectrum || spectrumBusy}
  onPrimaryAction={onRefreshSpectrum}
  {onClose}
>
  {#snippet controls()}
    <div class="ophiolite-charts-analysis-chip-group">
      <button
        class:active={spectrumAmplitudeScale === "db"}
        class="ophiolite-charts-analysis-chip-toggle"
        onclick={() => onSetSpectrumAmplitudeScale("db")}
      >
        dB
      </button>
      <button
        class:active={spectrumAmplitudeScale === "linear"}
        class="ophiolite-charts-analysis-chip-toggle"
        onclick={() => onSetSpectrumAmplitudeScale("linear")}
      >
        Linear
      </button>
    </div>
  {/snippet}

  {#snippet notices()}
    {#if spectrumStale}
      <div class="ophiolite-charts-analysis-status-bar">
        Pipeline settings changed after the last derivation. The charts below are preserved from the previous run until you press <strong>Derive Spectrum</strong> again.
      </div>
    {/if}
  {/snippet}

  {#if chartSeries.length > 0}
    <div class="ophiolite-charts-analysis-legend-row">
      {#each chartSeries as entry (entry.id)}
        <span class="ophiolite-charts-analysis-legend-item">
          <i class="ophiolite-charts-analysis-legend-swatch" style:background={entry.color}></i>
          {entry.label}
        </span>
      {/each}
    </div>

    <div class="spectrum-chart-grid">
      <div class="ophiolite-charts-analysis-chart-shell">
        <SpectrumChart
          title="Amplitude Spectrum"
          yLabel="Amplitude"
          amplitudeScale={spectrumAmplitudeScale}
          series={chartSeries}
        />
      </div>

      <div class="ophiolite-charts-analysis-chart-shell">
        <WaveletChart
          title="Derived Wavelet"
          yLabel="Normalized amplitude"
          series={waveletSeries}
        />
      </div>
    </div>

    <div class="spectrum-summary-grid">
      <div class="ophiolite-charts-analysis-summary-card">
        <span>Bins</span>
        <strong>{bins}</strong>
      </div>
      <div class="ophiolite-charts-analysis-summary-card">
        <span>Nyquist</span>
        <strong>{nyquistHz.toFixed(1)} Hz</strong>
      </div>
      <div class="ophiolite-charts-analysis-summary-card">
        <span>Sample Interval</span>
        <strong>{sampleIntervalMs.toFixed(2)} ms</strong>
      </div>
      <div class="ophiolite-charts-analysis-summary-card">
        <span>Wavelet Assumption</span>
        <strong>Zero phase</strong>
      </div>
      <div class="ophiolite-charts-analysis-summary-card">
        <span>Dominant Frequency</span>
        <strong>{dominantFrequencyHz ? `${dominantFrequencyHz.toFixed(1)} Hz` : "N/A"}</strong>
      </div>
      <div class="ophiolite-charts-analysis-summary-card wide">
        <span>Interpretation Note</span>
        <strong>Bandwidth controls wavelet compactness and tuning; phase controls symmetry. This wavelet is reconstructed from the spectrum assuming zero phase.</strong>
      </div>
    </div>
  {:else}
    <div class="ophiolite-charts-analysis-empty-state">
      <strong>No spectrum derived yet</strong>
      <p>Derive from the full displayed section now. Rectangle-based extraction will plug into this inspector next.</p>
    </div>
  {/if}

  {#if spectrumError}
    <div class="ophiolite-charts-analysis-error-bar">{spectrumError}</div>
  {/if}
</AnalysisInspectorShell>

<style>
  .spectrum-chart-grid {
    display: grid;
    grid-template-columns: repeat(2, minmax(0, 1fr));
    gap: 10px;
  }

  .spectrum-summary-grid {
    display: grid;
    grid-template-columns: repeat(3, minmax(0, 1fr));
    gap: 8px;
  }

  @media (max-width: 900px) {
    .spectrum-chart-grid,
    .spectrum-summary-grid {
      grid-template-columns: 1fr;
    }
  }
</style>
