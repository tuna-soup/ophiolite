<svelte:options runes={true} />

<script lang="ts">
  import {
    buildSeismicTickIndices,
    buildSeismicTopAxisRows,
    formatSeismicAxisValue,
    formatSeismicCssFont,
    resolveSeismicPresentationProfile,
    resolveSeismicSampleAxisTitle,
    resolveSeismicSectionTitle
  } from "@ophiolite/charts-core";
  import type { RenderMode, SectionPayload } from "@ophiolite/charts-data-models";
  import { getPlotRect } from "@ophiolite/charts-renderer";

  interface SeismicOverlayViewport {
    traceStart: number;
    traceEnd: number;
    sampleStart: number;
    sampleEnd: number;
  }

  let {
    section,
    viewport,
    renderMode,
    stageWidth,
    stageHeight,
    zIndex = 2
  }: {
    section: SectionPayload | null;
    viewport: SeismicOverlayViewport | null;
    renderMode: RenderMode;
    stageWidth: number;
    stageHeight: number;
    zIndex?: number;
  } = $props();

  const presentation = resolveSeismicPresentationProfile("standard");
  const tickFont = formatSeismicCssFont(presentation.typography.tick);
  const axisLabelFont = formatSeismicCssFont(presentation.typography.axisLabel);
  const titleFont = formatSeismicCssFont(presentation.typography.title);

  const plotRect = $derived(getPlotRect(stageWidth, stageHeight));
  const topAxisRows = $derived(section ? buildSeismicTopAxisRows(section) : []);
  const topTicks = $derived(
    section && viewport ? buildSeismicTickIndices(viewport.traceStart, viewport.traceEnd, 12) : []
  );
  const leftTicks = $derived(
    section && viewport ? buildSeismicTickIndices(viewport.sampleStart, viewport.sampleEnd, 14) : []
  );
  const horizontalAxisRange = $derived.by(() => {
    if (!section || !viewport) {
      return null;
    }

    let min = Number.POSITIVE_INFINITY;
    let max = Number.NEGATIVE_INFINITY;
    for (let index = viewport.traceStart; index < viewport.traceEnd; index += 1) {
      const value = section.horizontalAxis[index]!;
      min = Math.min(min, value);
      max = Math.max(max, value);
    }

    if (!Number.isFinite(min) || !Number.isFinite(max)) {
      return null;
    }

    return { min, max };
  });

  function traceIndexToPlotX(traceIndex: number): number {
    if (!section || !viewport) {
      return plotRect.x;
    }

    if (renderMode === "wiggle" && horizontalAxisRange) {
      const span = Math.max(1e-6, horizontalAxisRange.max - horizontalAxisRange.min);
      return plotRect.x + ((section.horizontalAxis[traceIndex]! - horizontalAxisRange.min) / span) * plotRect.width;
    }

    return plotRect.x + ((traceIndex - viewport.traceStart) / Math.max(1, viewport.traceEnd - viewport.traceStart - 1)) * plotRect.width;
  }

  function sampleIndexToPlotY(sampleIndex: number): number {
    if (!viewport) {
      return plotRect.y;
    }

    return plotRect.y + ((sampleIndex - viewport.sampleStart) / Math.max(1, viewport.sampleEnd - viewport.sampleStart - 1)) * plotRect.height;
  }
</script>

{#if section && viewport}
  <svg
    class="ophiolite-charts-seismic-axis-overlay"
    width={stageWidth}
    height={stageHeight}
    viewBox={`0 0 ${stageWidth} ${stageHeight}`}
    style:z-index={zIndex}
    aria-hidden="true"
  >
    <text
      x={plotRect.x + plotRect.width / 2}
      y={presentation.frame.titleY}
      text-anchor="middle"
      dominant-baseline="hanging"
      style:font={titleFont}
      style:fill={presentation.palette.title}
    >
      {resolveSeismicSectionTitle(section)}
    </text>

    {#each topTicks as traceIndex (`top-tick:${traceIndex}`)}
      {@const x = traceIndexToPlotX(traceIndex)}
      <line
        x1={x}
        y1={plotRect.y}
        x2={x}
        y2={plotRect.y - presentation.frame.topTickLength}
        stroke={presentation.palette.axisStroke}
        stroke-width="1"
      />
      {#each topAxisRows as row, rowIndex (`top-row:${rowIndex}:${row.label}`)}
        <text
          x={x}
          y={plotRect.y - presentation.frame.topTickOffset - rowIndex * presentation.frame.topAxisRowSpacing}
          text-anchor="middle"
          dominant-baseline="ideographic"
          style:font={tickFont}
          style:fill={presentation.palette.axisLabel}
        >
          {formatSeismicAxisValue(row.values[traceIndex]!)}
        </text>
      {/each}
    {/each}

    {#each leftTicks as sampleIndex (`left-tick:${sampleIndex}`)}
      {@const y = sampleIndexToPlotY(sampleIndex)}
      <line
        x1={plotRect.x}
        y1={y}
        x2={plotRect.x - presentation.frame.leftTickLength}
        y2={y}
        stroke={presentation.palette.axisStroke}
        stroke-width="1"
      />
      <text
        x={plotRect.x - presentation.frame.leftTickOffset}
        y={y}
        text-anchor="end"
        dominant-baseline="middle"
        style:font={tickFont}
        style:fill={presentation.palette.axisLabel}
      >
        {formatSeismicAxisValue(section.sampleAxis[sampleIndex]!)}
      </text>
    {/each}

    {#each topAxisRows as row, rowIndex (`row-label:${rowIndex}:${row.label}`)}
      <text
        x={presentation.frame.topAxisLabelX}
        y={plotRect.y - presentation.frame.topAxisRowLabelOffset - rowIndex * presentation.frame.topAxisRowSpacing}
        text-anchor="start"
        dominant-baseline="middle"
        style:font={axisLabelFont}
        style:fill={presentation.palette.title}
      >
        {row.label}
      </text>
    {/each}

    <text
      x={presentation.frame.yAxisLabelX}
      y={plotRect.y + plotRect.height / 2}
      text-anchor="middle"
      dominant-baseline="middle"
      transform={`rotate(-90 ${presentation.frame.yAxisLabelX} ${plotRect.y + plotRect.height / 2})`}
      style:font={axisLabelFont}
      style:fill={presentation.palette.title}
    >
      {resolveSeismicSampleAxisTitle(section)}
    </text>
  </svg>
{/if}

<style>
  .ophiolite-charts-seismic-axis-overlay {
    position: absolute;
    inset: 0;
    overflow: visible;
    pointer-events: none;
  }
  .ophiolite-charts-seismic-axis-overlay text {
    font-variant-numeric: tabular-nums lining-nums;
  }
</style>
