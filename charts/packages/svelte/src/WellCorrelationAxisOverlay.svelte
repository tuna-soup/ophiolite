<svelte:options runes={true} />

<script lang="ts">
  import type { WellCorrelationChromeModel } from "@ophiolite/charts-core";
  const TRACK_ROW_HEIGHT = 20;

  let {
    model,
    zIndex = 1,
    stageHeight
  }: {
    model: WellCorrelationChromeModel | null;
    stageHeight: number;
    zIndex?: number;
  } = $props();
</script>

{#if model}
  <svg
    class="ophiolite-charts-correlation-axis-overlay"
    width={model.layout.contentWidth}
    height={stageHeight}
    viewBox={`0 0 ${model.layout.contentWidth} ${stageHeight}`}
    style:z-index={zIndex}
    aria-hidden="true"
  >
    <g>
      {#each model.columns as column (`well:${column.wellId}`)}
          <rect
            x={column.headerRect.x}
            y={column.headerRect.y}
            width={column.headerRect.width}
            height={column.headerRect.height}
            fill="#ffffff"
            stroke="#b8b8b8"
            stroke-width="1"
          />
          <text
            x={column.headerRect.x + column.headerRect.width / 2}
            y={column.headerRect.y + 16}
            text-anchor="middle"
            dominant-baseline="alphabetic"
            class="ophiolite-charts-correlation-axis-overlay-title"
            fill="#1a1a1a"
          >
            {column.wellName}
          </text>

          {#each column.tracks as track (`track:${column.wellId}:${track.trackId}`)}
            <rect
              x={track.headerRect.x}
              y={track.headerRect.y}
              width={track.headerRect.width}
              height={track.headerRect.height}
              fill="#f3f1ed"
              stroke="#c8c1b8"
              stroke-width="1"
            />

            {#each track.headerRows as row, index (`row:${track.trackId}:${index}:${row.label}`)}
              {@const rowY = track.headerRect.y + 4 + index * TRACK_ROW_HEIGHT}
              <text
                x={track.headerRect.x + track.headerRect.width / 2}
                y={rowY + 8}
                text-anchor="middle"
                dominant-baseline="middle"
                class="ophiolite-charts-correlation-axis-overlay-row-label"
                fill={row.color}
              >
                {row.label}
              </text>
              {#if row.axisLabels}
                <text
                  x={track.headerRect.x + 4}
                  y={rowY + 17}
                  text-anchor="start"
                  dominant-baseline="middle"
                  class="ophiolite-charts-correlation-axis-overlay-row-value"
                  fill="#6b6b6b"
                >
                  {row.axisLabels.min}
                </text>
                <text
                  x={track.headerRect.x + track.headerRect.width - 4}
                  y={rowY + 17}
                  text-anchor="end"
                  dominant-baseline="middle"
                  class="ophiolite-charts-correlation-axis-overlay-row-value"
                  fill="#6b6b6b"
                >
                  {row.axisLabels.max}
                </text>
              {/if}
            {/each}

            {#if track.kind === "reference"}
              {#each track.depthTicks as tick (`depth:${track.trackId}:${tick.depth}`)}
                <line
                  x1={track.bodyRect.x + track.bodyRect.width - 12}
                  y1={tick.y}
                  x2={track.bodyRect.x + track.bodyRect.width}
                  y2={tick.y}
                  stroke="#767676"
                  stroke-width="1"
                />
                <text
                  x={track.bodyRect.x + track.bodyRect.width - 16}
                  y={tick.y + 3}
                  text-anchor="end"
                  dominant-baseline="middle"
                  class="ophiolite-charts-correlation-axis-overlay-depth"
                  fill="#404040"
                >
                  {tick.label}
                </text>
              {/each}
            {/if}
          {/each}
      {/each}
    </g>
  </svg>
{/if}

<style>
  .ophiolite-charts-correlation-axis-overlay {
    position: absolute;
    inset: 0;
    overflow: visible;
    pointer-events: none;
  }

  .ophiolite-charts-correlation-axis-overlay text {
    font-variant-numeric: tabular-nums lining-nums;
  }

  .ophiolite-charts-correlation-axis-overlay-title {
    font: 600 12px "Segoe UI", sans-serif;
  }

  .ophiolite-charts-correlation-axis-overlay-row-label {
    font: 600 10px "Segoe UI", sans-serif;
  }

  .ophiolite-charts-correlation-axis-overlay-row-value {
    font: 10px "Segoe UI", sans-serif;
  }

  .ophiolite-charts-correlation-axis-overlay-depth {
    font: 11px "Segoe UI", sans-serif;
  }
</style>
