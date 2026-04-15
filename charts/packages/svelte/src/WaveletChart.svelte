<svelte:options runes={true} />

<script lang="ts">
  interface WaveletChartSeries {
    id: string;
    label: string;
    color: string;
    timesMs: number[];
    amplitudes: number[];
  }

  interface WaveletPoint {
    x: number;
    y: number;
  }

  interface ResolvedSeries extends WaveletChartSeries {
    points: WaveletPoint[];
  }

  const chartWidth = 640;
  const chartHeight = 260;
  const plotLeft = 58;
  const plotRight = 18;
  const plotTop = 18;
  const plotBottom = 36;
  const plotWidth = chartWidth - plotLeft - plotRight;
  const plotHeight = chartHeight - plotTop - plotBottom;

  let {
    title = "Wavelet",
    xLabel = "Time (ms)",
    yLabel = "Normalized amplitude",
    ariaLabel = "Derived wavelet chart",
    series = []
  }: {
    title?: string;
    xLabel?: string;
    yLabel?: string;
    ariaLabel?: string;
    series?: WaveletChartSeries[];
  } = $props();

  const timeDomain = $derived.by(() => {
    const allTimes = series.flatMap((entry) => entry.timesMs).filter((value) => Number.isFinite(value));
    if (allTimes.length === 0) {
      return { min: -50, max: 50 };
    }

    return {
      min: Math.min(...allTimes),
      max: Math.max(...allTimes)
    };
  });
  const timeTicks = $derived(buildTicks(timeDomain.min, timeDomain.max, 5));
  const amplitudeTicks = [-1, -0.5, 0, 0.5, 1];
  const resolvedSeries = $derived.by<ResolvedSeries[]>(() =>
    series.map((entry) => ({
      ...entry,
      points: entry.timesMs.map((timeMs, index) => ({
        x: scaleX(timeMs, timeDomain.min, timeDomain.max),
        y: scaleY(entry.amplitudes[index] ?? 0)
      }))
    }))
  );

  function buildTicks(min: number, max: number, steps: number): number[] {
    if (!Number.isFinite(min) || !Number.isFinite(max) || max <= min || steps <= 1) {
      return [0];
    }

    return Array.from({ length: steps }, (_, index) => min + ((max - min) * index) / (steps - 1));
  }

  function scaleX(value: number, min: number, max: number): number {
    if (!Number.isFinite(value) || max <= min) {
      return plotLeft + plotWidth / 2;
    }

    return plotLeft + ((value - min) / (max - min)) * plotWidth;
  }

  function scaleY(value: number): number {
    const clamped = Math.max(-1, Math.min(1, Number.isFinite(value) ? value : 0));
    return plotTop + (1 - (clamped + 1) / 2) * plotHeight;
  }

  function linePath(points: WaveletPoint[]): string {
    if (points.length === 0) {
      return "";
    }

    return points
      .map((point, index) => `${index === 0 ? "M" : "L"}${point.x.toFixed(2)},${point.y.toFixed(2)}`)
      .join(" ");
  }
</script>

<div class="ophiolite-charts-wavelet-chart">
  <svg viewBox={`0 0 ${chartWidth} ${chartHeight}`} class="chart" aria-label={ariaLabel} role="img">
    <text class="chart-title" x={plotLeft} y="12">{title}</text>

    <g class="grid">
      {#each timeTicks as tick (tick)}
        <line x1={scaleX(tick, timeDomain.min, timeDomain.max)} y1={plotTop} x2={scaleX(tick, timeDomain.min, timeDomain.max)} y2={plotTop + plotHeight} />
        <text class="tick x" x={scaleX(tick, timeDomain.min, timeDomain.max)} y={chartHeight - 10}>{tick.toFixed(0)}</text>
      {/each}

      {#each amplitudeTicks as tick (tick)}
        <line x1={plotLeft} y1={scaleY(tick)} x2={plotLeft + plotWidth} y2={scaleY(tick)} />
        <text class="tick y" x={plotLeft - 10} y={scaleY(tick) + 4}>{tick.toFixed(1)}</text>
      {/each}
    </g>

    <rect class="plot-frame" x={plotLeft} y={plotTop} width={plotWidth} height={plotHeight} rx="10" />
    <line
      class="zero-line"
      x1={scaleX(0, timeDomain.min, timeDomain.max)}
      y1={plotTop}
      x2={scaleX(0, timeDomain.min, timeDomain.max)}
      y2={plotTop + plotHeight}
    />

    {#each resolvedSeries as entry (entry.id)}
      <path d={linePath(entry.points)} class="series-line" stroke={entry.color} />
    {/each}

    <text class="axis-label x" x={plotLeft + plotWidth} y={chartHeight - 2}>{xLabel}</text>
    <text class="axis-label y" x="18" y={plotTop + plotHeight / 2} transform={`rotate(-90 18 ${plotTop + plotHeight / 2})`}>
      {yLabel}
    </text>
  </svg>
</div>

<style>
  .ophiolite-charts-wavelet-chart {
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

  .zero-line {
    stroke: rgba(111, 144, 165, 0.42);
    stroke-width: 1.2;
    stroke-dasharray: 5 4;
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
