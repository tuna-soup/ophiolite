<svelte:options runes={true} />

<script lang="ts">
  import type { AmplitudeDistributionBin, AmplitudeDistributionMarker } from "./types";

  const chartWidth = 640;
  const chartHeight = 280;
  const plotLeft = 58;
  const plotRight = 18;
  const plotTop = 20;
  const plotBottom = 42;
  const plotWidth = chartWidth - plotLeft - plotRight;
  const plotHeight = chartHeight - plotTop - plotBottom;

  let {
    title = "Amplitude Distribution",
    xLabel = "Value",
    yLabel = "Count",
    ariaLabel = "Amplitude distribution chart",
    bins = [],
    markers = []
  }: {
    title?: string;
    xLabel?: string;
    yLabel?: string;
    ariaLabel?: string;
    bins?: AmplitudeDistributionBin[];
    markers?: AmplitudeDistributionMarker[];
  } = $props();

  const resolvedBins = $derived.by(() =>
    bins
      .filter(
        (entry) =>
          Number.isFinite(entry.start) &&
          Number.isFinite(entry.end) &&
          Number.isFinite(entry.count) &&
          entry.end > entry.start &&
          entry.count >= 0
      )
      .slice()
      .sort((left, right) => left.start - right.start)
  );
  const xDomain = $derived.by(() => {
    const values = [
      ...resolvedBins.flatMap((entry) => [entry.start, entry.end]),
      ...markers.map((entry) => entry.value).filter((value) => Number.isFinite(value))
    ];
    if (values.length === 0) {
      return { min: -1, max: 1 };
    }

    const min = Math.min(...values);
    const max = Math.max(...values);
    if (min === max) {
      const radius = Math.max(1, Math.abs(min) * 0.05);
      return { min: min - radius, max: max + radius };
    }

    const padding = (max - min) * 0.04;
    return {
      min: min - padding,
      max: max + padding
    };
  });
  const yMax = $derived(Math.max(1, ...resolvedBins.map((entry) => entry.count)));
  const xTicks = $derived(buildTicks(xDomain.min, xDomain.max, 6));
  const yTicks = $derived(buildTicks(0, yMax, 5));
  const histogramBars = $derived.by(() =>
    resolvedBins.map((entry) => {
      const left = scaleX(entry.start, xDomain.min, xDomain.max);
      const right = scaleX(entry.end, xDomain.min, xDomain.max);
      const top = scaleY(entry.count, yMax);
      return {
        ...entry,
        x: left,
        y: top,
        width: Math.max(1, right - left - 1),
        height: plotTop + plotHeight - top
      };
    })
  );
  const resolvedMarkers = $derived.by(() =>
    markers
      .filter((entry) => Number.isFinite(entry.value))
      .map((entry) => ({
        ...entry,
        x: scaleX(entry.value, xDomain.min, xDomain.max),
        color: entry.color ?? "#52c857"
      }))
  );

  function buildTicks(min: number, max: number, steps: number): number[] {
    if (!Number.isFinite(min) || !Number.isFinite(max) || steps <= 1 || max <= min) {
      return [min];
    }

    const span = max - min;
    return Array.from({ length: steps }, (_, index) => min + (span * index) / (steps - 1));
  }

  function scaleX(value: number, min: number, max: number): number {
    if (!Number.isFinite(value) || max <= min) {
      return plotLeft;
    }

    const ratio = (value - min) / (max - min);
    return plotLeft + ratio * plotWidth;
  }

  function scaleY(value: number, max: number): number {
    if (!Number.isFinite(value) || max <= 0) {
      return plotTop + plotHeight;
    }

    const ratio = Math.min(1, Math.max(0, value / max));
    return plotTop + (1 - ratio) * plotHeight;
  }

  function formatTick(value: number): string {
    const magnitude = Math.abs(value);
    if (magnitude >= 10000) {
      return value.toFixed(0);
    }
    if (magnitude >= 1000) {
      return value.toFixed(1);
    }
    if (magnitude >= 10) {
      return value.toFixed(2);
    }
    return value.toFixed(3);
  }
</script>

<div class="ophiolite-charts-amplitude-distribution-chart">
  <svg viewBox={`0 0 ${chartWidth} ${chartHeight}`} class="chart" aria-label={ariaLabel} role="img">
    <text class="chart-title" x={plotLeft} y="13">{title}</text>

    <g class="grid">
      {#each xTicks as tick (tick)}
        <line x1={scaleX(tick, xDomain.min, xDomain.max)} y1={plotTop} x2={scaleX(tick, xDomain.min, xDomain.max)} y2={plotTop + plotHeight} />
        <text class="tick x" x={scaleX(tick, xDomain.min, xDomain.max)} y={chartHeight - 14}>{formatTick(tick)}</text>
      {/each}

      {#each yTicks as tick (tick)}
        <line x1={plotLeft} y1={scaleY(tick, yMax)} x2={plotLeft + plotWidth} y2={scaleY(tick, yMax)} />
        <text class="tick y" x={plotLeft - 10} y={scaleY(tick, yMax) + 4}>{tick.toFixed(0)}</text>
      {/each}
    </g>

    <rect class="plot-frame" x={plotLeft} y={plotTop} width={plotWidth} height={plotHeight} rx="10" />

    {#if histogramBars.length > 0}
      {#each histogramBars as bar (`${bar.start}:${bar.end}`)}
        <rect
          class="bar"
          x={bar.x}
          y={bar.y}
          width={bar.width}
          height={bar.height}
          rx="1.5"
        />
      {/each}

      {#each resolvedMarkers as marker (marker.id)}
        <line class="marker" x1={marker.x} y1={plotTop} x2={marker.x} y2={plotTop + plotHeight} stroke={marker.color} />
        {#if marker.label}
          <text class="marker-label" x={marker.x} y={plotTop + 16} fill={marker.color}>{marker.label}</text>
        {/if}
      {/each}
    {:else}
      <text class="empty-state" x={plotLeft + plotWidth / 2} y={plotTop + plotHeight / 2}>No distribution data</text>
    {/if}

    <text class="axis-label x" x={plotLeft + plotWidth} y={chartHeight - 2}>{xLabel}</text>
    <text class="axis-label y" x="18" y={plotTop + plotHeight / 2} transform={`rotate(-90 18 ${plotTop + plotHeight / 2})`}>
      {yLabel}
    </text>
  </svg>
</div>

<style>
  .ophiolite-charts-amplitude-distribution-chart {
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

  .bar {
    fill: rgba(180, 150, 128, 0.68);
    stroke: rgba(71, 58, 50, 0.88);
    stroke-width: 0.8;
    vector-effect: non-scaling-stroke;
  }

  .marker {
    stroke-width: 1.8;
    vector-effect: non-scaling-stroke;
  }

  .marker-label {
    font-size: 10px;
    font-weight: 600;
    text-anchor: middle;
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

  .empty-state {
    fill: #637b8b;
    font-size: 12px;
    font-weight: 600;
    text-anchor: middle;
  }
</style>
