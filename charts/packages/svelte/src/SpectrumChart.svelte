<svelte:options runes={true} />

<script lang="ts">
  type AmplitudeScale = "db" | "linear";

  interface SpectrumChartSeries {
    id: string;
    label: string;
    color: string;
    fillColor?: string;
    frequenciesHz: number[];
    amplitudes: number[];
  }

  interface SpectrumPoint {
    x: number;
    y: number;
    value: number;
  }

  interface ResolvedSeries extends SpectrumChartSeries {
    points: SpectrumPoint[];
  }

  const chartWidth = 640;
  const chartHeight = 260;
  const plotLeft = 60;
  const plotRight = 18;
  const plotTop = 18;
  const plotBottom = 36;
  const plotWidth = chartWidth - plotLeft - plotRight;
  const plotHeight = chartHeight - plotTop - plotBottom;
  const dbFloorEpsilon = 1.0e-12;

  let {
    title = "Amplitude Spectrum",
    xLabel = "Frequency (Hz)",
    yLabel = "Amplitude",
    ariaLabel = "Amplitude spectrum chart",
    amplitudeScale = "db",
    minDb = -100,
    series = []
  }: {
    title?: string;
    xLabel?: string;
    yLabel?: string;
    ariaLabel?: string;
    amplitudeScale?: AmplitudeScale;
    minDb?: number;
    series?: SpectrumChartSeries[];
  } = $props();

  const referenceAmplitude = $derived.by(() => {
    const peak = Math.max(
      dbFloorEpsilon,
      ...series.flatMap((entry) => entry.amplitudes).map((value) => (Number.isFinite(value) ? Math.abs(value) : 0))
    );
    return peak;
  });
  const frequencyMax = $derived(
    Math.max(1, ...series.map((entry) => entry.frequenciesHz.at(-1) ?? 0).map((value) => (Number.isFinite(value) ? value : 0)))
  );
  const linearMax = $derived(
    Math.max(
      1,
      ...series.flatMap((entry) => entry.amplitudes).map((value) => (Number.isFinite(value) ? Math.max(0, value) : 0))
    )
  );
  const yDomain = $derived.by(() => {
    if (amplitudeScale === "db") {
      return { min: minDb, max: 0 };
    }

    return {
      min: 0,
      max: linearMax <= 0 ? 1 : linearMax * 1.05
    };
  });
  const xTicks = $derived(buildTicks(0, frequencyMax, 6));
  const yTicks = $derived.by(() => {
    if (amplitudeScale === "db") {
      return [0, -20, -40, -60, -80, -100].filter((value) => value >= minDb);
    }

    return buildTicks(0, yDomain.max, 5);
  });
  const resolvedSeries = $derived.by<ResolvedSeries[]>(() =>
    series.map((entry) => ({
      ...entry,
      points: entry.frequenciesHz.map((frequency, index) => {
        const amplitude = entry.amplitudes[index] ?? 0;
        const scaledValue =
          amplitudeScale === "db"
            ? 20 * Math.log10(Math.max(Math.abs(amplitude), dbFloorEpsilon) / referenceAmplitude)
            : Math.max(0, amplitude);
        return {
          x: plotLeft + (frequency / frequencyMax) * plotWidth,
          y: scaleY(scaledValue, yDomain.min, yDomain.max),
          value: scaledValue
        };
      })
    }))
  );

  function buildTicks(min: number, max: number, steps: number): number[] {
    if (!Number.isFinite(min) || !Number.isFinite(max) || steps <= 1 || max <= min) {
      return [min];
    }

    const span = max - min;
    return Array.from({ length: steps }, (_, index) => min + (span * index) / (steps - 1));
  }

  function scaleY(value: number, min: number, max: number): number {
    if (!Number.isFinite(value) || max <= min) {
      return plotTop + plotHeight;
    }

    const clamped = Math.min(max, Math.max(min, value));
    const ratio = (clamped - min) / (max - min);
    return plotTop + (1 - ratio) * plotHeight;
  }

  function linePath(points: SpectrumPoint[]): string {
    if (points.length === 0) {
      return "";
    }

    return points
      .map((point, index) => `${index === 0 ? "M" : "L"}${point.x.toFixed(2)},${point.y.toFixed(2)}`)
      .join(" ");
  }

  function areaPath(points: SpectrumPoint[]): string {
    if (points.length === 0) {
      return "";
    }

    const baseline = plotTop + plotHeight;
    const line = linePath(points);
    const first = points[0];
    const last = points.at(-1);
    if (!first || !last) {
      return "";
    }
    return `${line} L${last.x.toFixed(2)},${baseline.toFixed(2)} L${first.x.toFixed(2)},${baseline.toFixed(2)} Z`;
  }

  function formatTick(value: number): string {
    if (amplitudeScale === "db") {
      return `${Math.round(value)}`;
    }

    if (value >= 1000) {
      return value.toFixed(0);
    }
    if (value >= 100) {
      return value.toFixed(1);
    }
    if (value >= 10) {
      return value.toFixed(2);
    }
    return value.toFixed(3);
  }

  function gridX(value: number): number {
    return plotLeft + (value / frequencyMax) * plotWidth;
  }
</script>

<div class="ophiolite-charts-spectrum-chart">
  <svg viewBox={`0 0 ${chartWidth} ${chartHeight}`} class="chart" aria-label={ariaLabel} role="img">
    <text class="chart-title" x={plotLeft} y="12">{title}</text>

    <g class="grid">
      {#each xTicks as tick (tick)}
        <line x1={gridX(tick)} y1={plotTop} x2={gridX(tick)} y2={plotTop + plotHeight} />
        <text class="tick x" x={gridX(tick)} y={chartHeight - 10}>{tick.toFixed(0)}</text>
      {/each}

      {#each yTicks as tick (tick)}
        <line x1={plotLeft} y1={scaleY(tick, yDomain.min, yDomain.max)} x2={plotLeft + plotWidth} y2={scaleY(tick, yDomain.min, yDomain.max)} />
        <text class="tick y" x={plotLeft - 10} y={scaleY(tick, yDomain.min, yDomain.max) + 4}>{formatTick(tick)}</text>
      {/each}
    </g>

    <rect class="plot-frame" x={plotLeft} y={plotTop} width={plotWidth} height={plotHeight} rx="10" />

    {#each resolvedSeries as entry (entry.id)}
      <path d={areaPath(entry.points)} class="series-area" fill={entry.fillColor ?? `${entry.color}22`} />
      <path d={linePath(entry.points)} class="series-line" stroke={entry.color} />
    {/each}

    <text class="axis-label x" x={plotLeft + plotWidth} y={chartHeight - 2}>{xLabel}</text>
    <text class="axis-label y" x="18" y={plotTop + plotHeight / 2} transform={`rotate(-90 18 ${plotTop + plotHeight / 2})`}>
      {amplitudeScale === "db" ? `${yLabel} (dB)` : yLabel}
    </text>
  </svg>
</div>

<style>
  .ophiolite-charts-spectrum-chart {
    width: 100%;
    min-height: 0;
  }

  .chart {
    width: 100%;
    height: auto;
    display: block;
  }

  .chart-title {
    fill: #274052;
    font-size: 12px;
    font-weight: 600;
    letter-spacing: 0.02em;
  }

  .grid line {
    stroke: rgba(111, 129, 143, 0.24);
    stroke-width: 1;
    shape-rendering: crispEdges;
  }

  .tick {
    fill: #637b8b;
    font-size: 10px;
    font-weight: 500;
  }

  .tick.x {
    text-anchor: middle;
  }

  .tick.y {
    text-anchor: end;
  }

  .plot-frame {
    fill: #fff;
    stroke: rgba(176, 212, 238, 0.88);
    stroke-width: 1.2;
  }

  .series-area {
    opacity: 0.72;
  }

  .series-line {
    fill: none;
    stroke-width: 2.2;
    stroke-linejoin: round;
    stroke-linecap: round;
    vector-effect: non-scaling-stroke;
  }

  .axis-label {
    fill: #35505f;
    font-size: 10px;
    font-weight: 600;
    letter-spacing: 0.04em;
    text-transform: uppercase;
  }

  .axis-label.x {
    text-anchor: end;
  }

  .axis-label.y {
    text-anchor: middle;
  }
</style>
