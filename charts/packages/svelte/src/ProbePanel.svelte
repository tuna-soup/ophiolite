<svelte:options runes={true} />

<script lang="ts">
  import {
    formatProbePanelCssFont,
    resolveProbePanelPresentation,
    type ProbePanelSizeId,
    type ProbePanelThemeId
  } from "@ophiolite/charts-core";

  interface ProbePanelRow {
    label: string;
    value: string;
    valueTitle?: string;
  }

  let {
    rows,
    theme = "light",
    size = "standard",
    top = undefined,
    right = undefined,
    bottom = undefined,
    left = undefined,
    zIndex = 3
  }: {
    rows: ProbePanelRow[];
    theme?: ProbePanelThemeId;
    size?: ProbePanelSizeId;
    top?: string;
    right?: string;
    bottom?: string;
    left?: string;
    zIndex?: number;
  } = $props();

  const presentation = $derived(resolveProbePanelPresentation(theme, size));
  const rowFont = $derived(formatProbePanelCssFont(presentation.typography.row));
</script>

<div
  class="ophiolite-charts-probe-panel"
  style:top={top}
  style:right={right}
  style:bottom={bottom}
  style:left={left}
  style:z-index={zIndex}
  style:--ophiolite-chart-probe-width={`${presentation.frame.widthPx}px`}
  style:--ophiolite-chart-probe-label-width={`${presentation.frame.labelWidthPx}px`}
  style:--ophiolite-chart-probe-padding-x={`${presentation.frame.paddingXPx}px`}
  style:--ophiolite-chart-probe-padding-y={`${presentation.frame.paddingYPx}px`}
  style:--ophiolite-chart-probe-row-gap={`${presentation.frame.rowGapPx}px`}
  style:--ophiolite-chart-probe-column-gap={`${presentation.frame.columnGapPx}px`}
  style:--ophiolite-chart-probe-radius={`${presentation.frame.borderRadiusPx}px`}
  style:--ophiolite-chart-probe-font={rowFont}
  style:--ophiolite-chart-probe-border={presentation.colors.border}
  style:--ophiolite-chart-probe-bg={presentation.colors.background}
  style:--ophiolite-chart-probe-shadow={presentation.colors.shadow}
  style:--ophiolite-chart-probe-text={presentation.colors.text}
  style:--ophiolite-chart-probe-label={presentation.colors.label}
>
  {#each rows as row, index (`${index}:${row.label}`)}
    <div class="ophiolite-charts-probe-panel-row">
      <span class="ophiolite-charts-probe-panel-label" title={row.label}>{row.label}</span>
      <span class="ophiolite-charts-probe-panel-value" title={row.valueTitle ?? row.value}>{row.value}</span>
    </div>
  {/each}
</div>

<style>
  .ophiolite-charts-probe-panel {
    position: absolute;
    display: grid;
    gap: var(--ophiolite-chart-probe-row-gap);
    width: var(--ophiolite-chart-probe-width);
    padding: var(--ophiolite-chart-probe-padding-y) var(--ophiolite-chart-probe-padding-x);
    border: 1px solid var(--ophiolite-chart-probe-border);
    border-radius: var(--ophiolite-chart-probe-radius);
    background: var(--ophiolite-chart-probe-bg);
    box-shadow: var(--ophiolite-chart-probe-shadow);
    color: var(--ophiolite-chart-probe-text);
    pointer-events: none;
    box-sizing: border-box;
  }

  .ophiolite-charts-probe-panel-row {
    display: grid;
    grid-template-columns: var(--ophiolite-chart-probe-label-width) minmax(0, 1fr);
    column-gap: var(--ophiolite-chart-probe-column-gap);
    align-items: baseline;
    font: var(--ophiolite-chart-probe-font);
    white-space: nowrap;
  }

  .ophiolite-charts-probe-panel-label {
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    color: var(--ophiolite-chart-probe-label);
    text-transform: lowercase;
  }

  .ophiolite-charts-probe-panel-value {
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    text-align: right;
    color: var(--ophiolite-chart-probe-text);
    font-variant-numeric: tabular-nums lining-nums;
  }
</style>
