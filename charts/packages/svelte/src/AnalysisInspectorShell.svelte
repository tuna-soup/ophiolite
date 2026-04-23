<svelte:options runes={true} />

<script lang="ts">
  import type { Snippet } from "svelte";

  let {
    floating = false,
    ariaLabel,
    eyebrow,
    title,
    summary,
    primaryActionLabel,
    primaryActionBusyLabel = primaryActionLabel,
    primaryActionBusy = false,
    primaryActionDisabled = false,
    selectionLabel = "Displayed section",
    onPrimaryAction,
    onClose,
    controls,
    notices,
    children
  }: {
    floating?: boolean;
    ariaLabel: string;
    eyebrow: string;
    title: string;
    summary: string;
    primaryActionLabel: string;
    primaryActionBusyLabel?: string;
    primaryActionBusy?: boolean;
    primaryActionDisabled?: boolean;
    selectionLabel?: string;
    onPrimaryAction: () => void | Promise<void>;
    onClose?: (() => void) | undefined;
    controls?: Snippet | undefined;
    notices?: Snippet | undefined;
    children: Snippet;
  } = $props();
</script>

<section
  class:floating
  class="ophiolite-charts-analysis-inspector"
  role={floating ? "dialog" : undefined}
  aria-modal="false"
  aria-label={ariaLabel}
>
  <header class="analysis-header">
    <div class="analysis-header-copy">
      <span class="analysis-eyebrow">{eyebrow}</span>
      <h3>{title}</h3>
      <p>{summary}</p>
    </div>

    <div class="analysis-header-actions">
      <button
        class="analysis-chip analysis-chip-primary"
        onclick={onPrimaryAction}
        disabled={primaryActionDisabled}
      >
        {primaryActionBusy ? primaryActionBusyLabel : primaryActionLabel}
      </button>

      {#if onClose}
        <button class="analysis-icon-btn" onclick={onClose} aria-label={`Close ${title}`}>
          <svg viewBox="0 0 24 24" width="14" height="14" fill="none" stroke="currentColor" stroke-width="1.9">
            <path d="M6 6l12 12" />
            <path d="M18 6l-12 12" />
          </svg>
        </button>
      {/if}
    </div>
  </header>

  <div class="analysis-controls-row">
    <div class="ophiolite-charts-analysis-selection-badge">{selectionLabel}</div>
    {@render controls?.()}
  </div>

  {@render notices?.()}
  {@render children()}
</section>

<style>
  .ophiolite-charts-analysis-inspector {
    pointer-events: auto;
    display: flex;
    flex-direction: column;
    gap: 16px;
    min-height: 0;
    padding: 24px;
    border: 1px solid rgba(196, 206, 214, 0.88);
    border-radius: 14px;
    background: rgba(248, 251, 253, 0.98);
    box-shadow: 0 18px 40px rgba(36, 56, 72, 0.18);
    color: #203544;
  }

  .ophiolite-charts-analysis-inspector.floating {
    width: min(480px, calc(100vw - 64px));
    max-height: min(420px, calc(100vh - 120px));
    overflow: auto;
    backdrop-filter: blur(8px);
  }

  .analysis-header,
  .analysis-controls-row {
    display: flex;
    gap: 16px;
    align-items: center;
    justify-content: space-between;
    flex-wrap: wrap;
  }

  .analysis-header-copy {
    min-width: 0;
  }

  .analysis-eyebrow {
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
    color: #203544;
  }

  .analysis-header-copy p {
    margin: 4px 0 0;
    color: #566c7a;
    font-size: 12px;
    line-height: 1.45;
  }

  .analysis-header-actions {
    display: flex;
    gap: 8px;
    align-items: center;
  }

  .analysis-chip,
  .analysis-icon-btn {
    border: 1px solid rgba(196, 206, 214, 0.88);
    background: rgba(234, 240, 244, 0.78);
    color: #203544;
    border-radius: 6px;
    padding: 6px 10px;
    font-size: 11px;
    font-weight: 600;
    cursor: pointer;
  }

  .analysis-icon-btn {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    width: 30px;
    height: 30px;
    padding: 0;
  }

  .analysis-chip:hover:not(:disabled),
  .analysis-icon-btn:hover:not(:disabled) {
    background: rgba(241, 245, 248, 0.96);
    border-color: rgba(165, 181, 192, 0.92);
  }

  .analysis-chip:disabled,
  .analysis-icon-btn:disabled {
    cursor: default;
    opacity: 0.55;
  }

  .analysis-chip-primary {
    border-color: #9bc7e3;
    background: #eef6fb;
    color: #274b61;
  }

  :global(.ophiolite-charts-analysis-selection-badge) {
    border: 1px solid rgba(196, 206, 214, 0.88);
    border-radius: 6px;
    padding: 5px 10px;
    background: rgba(234, 240, 244, 0.78);
    color: #566c7a;
    font-size: 11px;
    letter-spacing: 0.04em;
    text-transform: uppercase;
  }

  :global(.ophiolite-charts-analysis-chip-group) {
    display: flex;
    gap: 8px;
    align-items: center;
  }

  :global(.ophiolite-charts-analysis-chip-toggle) {
    border: 1px solid rgba(196, 206, 214, 0.88);
    background: rgba(234, 240, 244, 0.78);
    color: #203544;
    border-radius: 6px;
    padding: 6px 10px;
    font-size: 11px;
    font-weight: 600;
    cursor: pointer;
  }

  :global(.ophiolite-charts-analysis-chip-toggle:hover:not(:disabled)) {
    background: rgba(241, 245, 248, 0.96);
    border-color: rgba(165, 181, 192, 0.92);
  }

  :global(.ophiolite-charts-analysis-chip-toggle:disabled) {
    cursor: default;
    opacity: 0.55;
  }

  :global(.ophiolite-charts-analysis-chip-toggle.active) {
    background: #e8f3fb;
    border-color: #b0d4ee;
    color: #274b61;
  }

  :global(.ophiolite-charts-analysis-legend-row) {
    display: flex;
    gap: 14px;
    align-items: center;
    justify-content: flex-start;
    flex-wrap: wrap;
  }

  :global(.ophiolite-charts-analysis-legend-item) {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    color: #566c7a;
    font-size: 11px;
  }

  :global(.ophiolite-charts-analysis-legend-swatch) {
    width: 14px;
    height: 3px;
    border-radius: 6px;
    display: inline-block;
  }

  :global(.ophiolite-charts-analysis-chart-shell),
  :global(.ophiolite-charts-analysis-status-bar),
  :global(.ophiolite-charts-analysis-summary-card),
  :global(.ophiolite-charts-analysis-empty-state),
  :global(.ophiolite-charts-analysis-error-bar) {
    border: 1px solid rgba(196, 206, 214, 0.88);
    background: #fff;
    border-radius: 8px;
    padding: 10px 12px;
  }

  :global(.ophiolite-charts-analysis-chart-shell) {
    padding: 10px;
  }

  :global(.ophiolite-charts-analysis-status-bar) {
    color: #315b75;
    border-color: #c9dcec;
    background: #edf6fc;
    font-size: 12px;
    line-height: 1.5;
  }

  :global(.ophiolite-charts-analysis-empty-state strong) {
    display: block;
    margin-bottom: 4px;
    color: #203544;
    font-size: 13px;
  }

  :global(.ophiolite-charts-analysis-empty-state p),
  :global(.ophiolite-charts-analysis-error-bar) {
    margin: 0;
    font-size: 12px;
    line-height: 1.5;
    color: #566c7a;
  }

  :global(.ophiolite-charts-analysis-error-bar) {
    color: #8f3c3c;
    border-color: #e0b7b7;
    background: #f9ecec;
  }

  :global(.ophiolite-charts-analysis-summary-card span) {
    display: block;
    font-size: 10px;
    letter-spacing: 0.08em;
    text-transform: uppercase;
    color: #6b808d;
  }

  :global(.ophiolite-charts-analysis-summary-card strong) {
    display: block;
    margin-top: 5px;
    font-size: 13px;
    color: #203544;
    line-height: 1.45;
  }

  :global(.ophiolite-charts-analysis-summary-card.wide) {
    grid-column: 1 / -1;
  }

  @media (max-width: 900px) {
    .ophiolite-charts-analysis-inspector.floating {
      width: min(100%, calc(100vw - 28px));
      max-height: min(440px, calc(100vh - 96px));
    }
  }
</style>
