<svelte:options runes={true} />

<script lang="ts">
  import { SpectrumChart, WaveletChart } from "@ophiolite/charts";
  import type { AmplitudeSpectrumResponse } from "@traceboost/seis-contracts";
  import { deriveZeroPhaseWavelet } from "../spectrum-wavelet";

  type SpectrumAmplitudeScale = "db" | "linear";

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
    rawSpectrum: AmplitudeSpectrumResponse | null;
    processedSpectrum: AmplitudeSpectrumResponse | null;
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
        frequenciesHz: rawSpectrum.curve.frequencies_hz,
        amplitudes: rawSpectrum.curve.amplitudes
      });
    }

    if (processedSpectrum) {
      series.push({
        id: "processed",
        label: processedSpectrum.processing_label ? `Processed · ${processedSpectrum.processing_label}` : "Processed",
        color: "#4dd48b",
        fillColor: "rgba(77, 212, 139, 0.18)",
        frequenciesHz: processedSpectrum.curve.frequencies_hz,
        amplitudes: processedSpectrum.curve.amplitudes
      });
    }

    return series;
  });
  const bins = $derived(rawSpectrum?.curve.frequencies_hz.length ?? processedSpectrum?.curve.frequencies_hz.length ?? 0);
  const nyquistHz = $derived(rawSpectrum?.curve.frequencies_hz.at(-1) ?? processedSpectrum?.curve.frequencies_hz.at(-1) ?? 0);
  const sampleIntervalMs = $derived(rawSpectrum?.sample_interval_ms ?? processedSpectrum?.sample_interval_ms ?? 0);
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
        label: processedSpectrum?.processing_label ? `Processed · ${processedSpectrum.processing_label}` : "Processed",
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

<section class:floating class="spectrum-inspector" role={floating ? "dialog" : undefined} aria-modal="false" aria-label="Frequency spectrum inspector">
  <header class="panel-header">
    <div class="header-copy">
      <span class="eyebrow">Frequency Spectrum</span>
      <h3>Amplitude Spectrum</h3>
      <p>{spectrumSelectionSummary}</p>
    </div>

    <div class="header-actions">
      <button
        class="chip action primary"
        onclick={onRefreshSpectrum}
        disabled={!canInspectSpectrum || spectrumBusy}
      >
        {spectrumBusy ? "Deriving..." : "Derive Spectrum"}
      </button>

      {#if onClose}
        <button class="icon-btn" onclick={onClose} aria-label="Close spectrum inspector">
          <svg viewBox="0 0 24 24" width="14" height="14" fill="none" stroke="currentColor" stroke-width="1.9">
            <path d="M6 6l12 12" />
            <path d="M18 6l-12 12" />
          </svg>
        </button>
      {/if}
    </div>
  </header>

  <div class="controls-row">
    <div class="selection-badge">Displayed section</div>

    <div class="scale-toggle">
      <button
        class:active={spectrumAmplitudeScale === "db"}
        class="chip"
        onclick={() => onSetSpectrumAmplitudeScale("db")}
      >
        dB
      </button>
      <button
        class:active={spectrumAmplitudeScale === "linear"}
        class="chip"
        onclick={() => onSetSpectrumAmplitudeScale("linear")}
      >
        Linear
      </button>
    </div>
  </div>

  {#if spectrumStale}
    <div class="status-bar">
      Pipeline settings changed after the last derivation. The charts below are preserved from the previous run until you press <strong>Derive Spectrum</strong> again.
    </div>
  {/if}

  {#if chartSeries.length > 0}
    <div class="legend-row">
      {#each chartSeries as entry (entry.id)}
        <span class="legend-item">
          <i class="legend-swatch" style:background={entry.color}></i>
          {entry.label}
        </span>
      {/each}
    </div>

    <div class="chart-grid">
      <div class="chart-shell">
        <SpectrumChart
          title="Amplitude Spectrum"
          yLabel="Amplitude"
          amplitudeScale={spectrumAmplitudeScale}
          series={chartSeries}
        />
      </div>

      <div class="chart-shell">
        <WaveletChart
          title="Derived Wavelet"
          yLabel="Normalized amplitude"
          series={waveletSeries}
        />
      </div>
    </div>

    <div class="summary-grid">
      <div class="summary-card">
        <span>Bins</span>
        <strong>{bins}</strong>
      </div>
      <div class="summary-card">
        <span>Nyquist</span>
        <strong>{nyquistHz.toFixed(1)} Hz</strong>
      </div>
      <div class="summary-card">
        <span>Sample Interval</span>
        <strong>{sampleIntervalMs.toFixed(2)} ms</strong>
      </div>
      <div class="summary-card">
        <span>Wavelet Assumption</span>
        <strong>Zero phase</strong>
      </div>
      <div class="summary-card">
        <span>Dominant Frequency</span>
        <strong>{dominantFrequencyHz ? `${dominantFrequencyHz.toFixed(1)} Hz` : "N/A"}</strong>
      </div>
      <div class="summary-card wide">
        <span>Interpretation Note</span>
        <strong>Bandwidth controls wavelet compactness and tuning; phase controls symmetry. This wavelet is reconstructed from the spectrum assuming zero phase.</strong>
      </div>
    </div>
  {:else}
    <div class="empty-state">
      <strong>No spectrum derived yet</strong>
      <p>Derive from the full displayed section now. Rectangle-based extraction will plug into this inspector next.</p>
    </div>
  {/if}

  {#if spectrumError}
    <div class="error-bar">{spectrumError}</div>
  {/if}
</section>

<style>
  .spectrum-inspector {
    pointer-events: auto;
    display: flex;
    flex-direction: column;
    gap: var(--ui-space-4);
    min-height: 0;
    padding: var(--ui-space-6);
    border: 1px solid var(--app-border);
    border-radius: var(--ui-radius-lg);
    background: var(--panel-bg);
    box-shadow: var(--ui-shadow-popover);
    color: var(--text-primary);
  }

  .spectrum-inspector.floating {
    width: min(480px, calc(100vw - 64px));
    max-height: min(420px, calc(100vh - 120px));
    overflow: auto;
    backdrop-filter: blur(8px);
  }

  .panel-header,
  .controls-row,
  .legend-row {
    display: flex;
    gap: var(--ui-space-4);
    align-items: center;
    justify-content: space-between;
    flex-wrap: wrap;
  }

  .header-copy {
    min-width: 0;
  }

  .eyebrow {
    display: inline-block;
    margin-bottom: 2px;
    font-size: 10px;
    letter-spacing: 0.12em;
    text-transform: uppercase;
    color: #447196;
  }

  h3 {
    margin: 0;
    font-size: 17px;
    font-weight: 600;
    color: var(--text-primary);
  }

  .header-copy p {
    margin: 4px 0 0;
    color: var(--text-muted);
    font-size: 12px;
    line-height: 1.45;
  }

  .header-actions,
  .scale-toggle {
    display: flex;
    gap: 8px;
    align-items: center;
  }

  .selection-badge {
    border: 1px solid var(--app-border);
    border-radius: 6px;
    padding: 5px 10px;
    background: var(--surface-subtle);
    color: var(--text-muted);
    font-size: 11px;
    letter-spacing: 0.04em;
    text-transform: uppercase;
  }

  .chip,
  .icon-btn {
    border: 1px solid var(--app-border);
    background: var(--surface-subtle);
    color: var(--text-primary);
    border-radius: 6px;
    padding: 6px 10px;
    font-size: 11px;
    font-weight: 600;
    cursor: pointer;
    transition:
      background-color 120ms ease,
      border-color 120ms ease,
      color 120ms ease;
  }

  .icon-btn {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    width: 30px;
    height: 30px;
    padding: 0;
  }

  .chip:hover:not(:disabled),
  .icon-btn:hover:not(:disabled) {
    background: var(--surface-bg);
    border-color: var(--app-border-strong);
    color: var(--text-primary);
  }

  .chip:disabled,
  .icon-btn:disabled {
    cursor: default;
    opacity: 0.55;
  }

  .chip.primary {
    border-color: #9bc7e3;
    background: #eef6fb;
    color: #274b61;
  }

  .chip.active {
    background: #e8f3fb;
    border-color: #b0d4ee;
    color: #274b61;
  }

  .legend-row {
    justify-content: flex-start;
    gap: 14px;
  }

  .legend-item {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    color: var(--text-muted);
    font-size: 11px;
  }

  .legend-swatch {
    width: 14px;
    height: 3px;
    border-radius: 6px;
    display: inline-block;
  }

  .chart-shell,
  .status-bar,
  .summary-card,
  .empty-state,
  .error-bar {
    border: 1px solid var(--app-border);
    background: #fff;
  }

  .chart-shell {
    padding: 10px;
    border-radius: 8px;
  }

  .chart-grid {
    display: grid;
    grid-template-columns: repeat(2, minmax(0, 1fr));
    gap: 10px;
  }

  .summary-grid {
    display: grid;
    grid-template-columns: repeat(3, minmax(0, 1fr));
    gap: 8px;
  }

  .summary-card,
  .status-bar,
  .empty-state,
  .error-bar {
    border-radius: 8px;
    padding: 10px 12px;
  }

  .summary-card span {
    display: block;
    font-size: 10px;
    letter-spacing: 0.08em;
    text-transform: uppercase;
    color: var(--text-dim);
  }

  .summary-card strong {
    display: block;
    margin-top: 5px;
    font-size: 13px;
    color: var(--text-primary);
    line-height: 1.45;
  }

  .summary-card.wide {
    grid-column: 1 / -1;
  }

  .status-bar {
    color: #315b75;
    border-color: #c9dcec;
    background: #edf6fc;
    font-size: 12px;
    line-height: 1.5;
  }

  .empty-state strong {
    display: block;
    margin-bottom: 4px;
    color: var(--text-primary);
    font-size: 13px;
  }

  .empty-state p,
  .error-bar {
    margin: 0;
    font-size: 12px;
    line-height: 1.5;
    color: var(--text-muted);
  }

  .error-bar {
    color: #8f3c3c;
    border-color: #e0b7b7;
    background: #f9ecec;
  }

  @media (max-width: 900px) {
    .spectrum-inspector.floating {
      width: min(100%, calc(100vw - 28px));
      max-height: min(440px, calc(100vh - 96px));
    }

    .chart-grid,
    .summary-grid {
      grid-template-columns: 1fr;
    }
  }
</style>
