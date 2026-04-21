<svelte:options runes={true} />

<script lang="ts">
  import type { ToolbarIconName } from "./types";

  interface Props {
    label: string;
    icon: ToolbarIconName;
    showLabel?: boolean;
    disabled?: boolean;
    active?: boolean;
    pressed?: boolean | undefined;
    ariaLabel: string;
    onclick?: () => void;
  }

  let {
    label,
    icon,
    showLabel = false,
    disabled = false,
    active = false,
    pressed = undefined,
    ariaLabel,
    onclick
  }: Props = $props();
</script>

<div class="toolbar-button-shell">
  <button
    type="button"
    class={["toolbar-button", active && "active"]}
    {disabled}
    aria-pressed={pressed}
    aria-label={ariaLabel}
    title={label}
    {onclick}
  >
    <span class="icon" aria-hidden="true">
      {#if icon === "pointer"}
        <svg viewBox="0 0 24 24" width="18" height="18" fill="none" stroke="currentColor" stroke-width="1.8">
          <path d="M6 4L15 14H11L13 20L10.8 21L8.8 15.8L5.4 18.2Z" stroke-linejoin="round" />
        </svg>
      {:else if icon === "crosshair"}
        <svg viewBox="0 0 24 24" width="18" height="18" fill="none" stroke="currentColor" stroke-width="1.8">
          <circle cx="12" cy="12" r="4.5" />
          <path d="M12 2V6M12 18V22M2 12H6M18 12H22" stroke-linecap="round" />
        </svg>
      {:else if icon === "pan"}
        <svg viewBox="0 0 24 24" width="18" height="18" fill="none" stroke="currentColor" stroke-width="1.8">
          <path d="M12 3V21M3 12H21" stroke-linecap="round" />
          <path d="M8 6L12 2L16 6M18 8L22 12L18 16M16 18L12 22L8 18M6 16L2 12L6 8" stroke-linecap="round" stroke-linejoin="round" />
        </svg>
      {:else if icon === "orbit"}
        <svg viewBox="0 0 24 24" width="18" height="18" fill="none" stroke="currentColor" stroke-width="1.8">
          <ellipse cx="12" cy="12" rx="7.5" ry="3.8" />
          <path d="M12 4.5C15.8 4.5 19 7.8 19 12s-3.2 7.5-7 7.5S5 16.2 5 12" stroke-linecap="round" />
          <path d="M18.8 8.5L20.8 7.8L20.1 5.8" stroke-linecap="round" stroke-linejoin="round" />
        </svg>
      {:else if icon === "sliceDrag"}
        <svg viewBox="0 0 24 24" width="18" height="18" fill="none" stroke="currentColor" stroke-width="1.8">
          <path d="M5 7H19M5 17H19" stroke-linecap="round" />
          <path d="M12 4V20" opacity="0.45" stroke-linecap="round" />
          <path d="M9 10L12 7L15 10M15 14L12 17L9 14" stroke-linecap="round" stroke-linejoin="round" />
        </svg>
      {:else if icon === "fitToData"}
        <svg viewBox="0 0 24 24" width="18" height="18" fill="none" stroke="currentColor" stroke-width="1.8">
          <path d="M4 9V4H9M15 4H20V9M20 15V20H15M9 20H4V15" stroke-linecap="round" stroke-linejoin="round" />
          <path d="M8 8L4 4M16 8L20 4M16 16L20 20M8 16L4 20" stroke-linecap="round" />
        </svg>
      {:else if icon === "topView"}
        <svg viewBox="0 0 24 24" width="18" height="18" fill="none" stroke="currentColor" stroke-width="1.8">
          <path d="M5 8L12 4L19 8L12 12Z" stroke-linejoin="round" />
          <path d="M5 8V16L12 20L19 16V8" opacity="0.5" stroke-linejoin="round" />
          <path d="M12 2V6M10 4H14" stroke-linecap="round" />
        </svg>
      {:else if icon === "sideView"}
        <svg viewBox="0 0 24 24" width="18" height="18" fill="none" stroke="currentColor" stroke-width="1.8">
          <path d="M5 6H13V18H5Z" stroke-linejoin="round" />
          <path d="M13 8L18.5 5V17L13 18" stroke-linejoin="round" opacity="0.6" />
          <path d="M20 12H16" stroke-linecap="round" />
          <path d="M18 10L20 12L18 14" stroke-linecap="round" stroke-linejoin="round" />
        </svg>
      {:else if icon === "resetView"}
        <svg viewBox="0 0 24 24" width="18" height="18" fill="none" stroke="currentColor" stroke-width="1.8">
          <path d="M6 8V4H10" stroke-linecap="round" stroke-linejoin="round" />
          <path d="M6 4C7.7 2.8 9.8 2 12 2C17.5 2 22 6.5 22 12S17.5 22 12 22 2 17.5 2 12c0-2 .6-3.8 1.7-5.4" stroke-linecap="round" />
          <path d="M12 7V12L15.4 14.1" stroke-linecap="round" stroke-linejoin="round" />
        </svg>
      {:else if icon === "centerSelection"}
        <svg viewBox="0 0 24 24" width="18" height="18" fill="none" stroke="currentColor" stroke-width="1.8">
          <circle cx="12" cy="12" r="3.5" />
          <path d="M12 2V6M12 18V22M2 12H6M18 12H22" stroke-linecap="round" />
          <path d="M7 7L9.4 9.4M14.6 14.6L17 17M17 7L14.6 9.4M9.4 14.6L7 17" stroke-linecap="round" opacity="0.55" />
        </svg>
      {:else if icon === "settings"}
        <svg viewBox="0 0 24 24" width="18" height="18" fill="none" stroke="currentColor" stroke-width="1.8">
          <circle cx="12" cy="12" r="3.2" />
          <path d="M19.4 15a1 1 0 0 0 .2 1.1l.1.1a1.2 1.2 0 0 1 0 1.7l-1.2 1.2a1.2 1.2 0 0 1-1.7 0l-.1-.1a1 1 0 0 0-1.1-.2 1 1 0 0 0-.6.9V20a1.2 1.2 0 0 1-1.2 1.2h-1.7A1.2 1.2 0 0 1 10 20v-.2a1 1 0 0 0-.6-.9 1 1 0 0 0-1.1.2l-.1.1a1.2 1.2 0 0 1-1.7 0l-1.2-1.2a1.2 1.2 0 0 1 0-1.7l.1-.1a1 1 0 0 0 .2-1.1 1 1 0 0 0-.9-.6H4a1.2 1.2 0 0 1-1.2-1.2v-1.7A1.2 1.2 0 0 1 4 10h.2a1 1 0 0 0 .9-.6 1 1 0 0 0-.2-1.1l-.1-.1a1.2 1.2 0 0 1 0-1.7l1.2-1.2a1.2 1.2 0 0 1 1.7 0l.1.1a1 1 0 0 0 1.1.2 1 1 0 0 0 .6-.9V4A1.2 1.2 0 0 1 10.7 2.8h1.7A1.2 1.2 0 0 1 13.6 4v.2a1 1 0 0 0 .6.9 1 1 0 0 0 1.1-.2l.1-.1a1.2 1.2 0 0 1 1.7 0l1.2 1.2a1.2 1.2 0 0 1 0 1.7l-.1.1a1 1 0 0 0-.2 1.1 1 1 0 0 0 .9.6h.2A1.2 1.2 0 0 1 21.2 10v1.7A1.2 1.2 0 0 1 20 12.9h-.2a1 1 0 0 0-.9.6" stroke-linecap="round" stroke-linejoin="round" />
        </svg>
      {/if}
    </span>
    {#if showLabel}
      <span class="label">{label}</span>
    {/if}
  </button>
  <span class="toolbar-tooltip" role="tooltip">{label}</span>
</div>

<style>
  .toolbar-button-shell {
    position: relative;
    display: inline-flex;
    overflow: visible;
  }

  .toolbar-button {
    display: inline-flex;
    align-items: center;
    gap: 8px;
    padding: 9px 12px;
    border: 0;
    border-radius: 6px;
    background: transparent;
    color: var(--ophiolite-toolbar-text, #35505f);
    font: 600 12px/1 "Segoe UI", sans-serif;
    letter-spacing: 0.01em;
    cursor: pointer;
    transition:
      background-color 140ms ease,
      color 140ms ease,
      transform 140ms ease;
  }

  :global(.toolbar-shell.overlay) .toolbar-button {
    padding: 7px;
    border-radius: 6px;
  }

  .toolbar-button:hover:not(:disabled) {
    background: var(--ophiolite-toolbar-hover-bg, #eef6fb);
    color: var(--ophiolite-toolbar-hover-text, #274b61);
  }

  .toolbar-button.active {
    background: var(--ophiolite-toolbar-active-bg, #e8f3fb);
    color: var(--ophiolite-toolbar-active-text, #274b61);
    box-shadow:
      inset 0 0 0 1px rgba(155, 199, 227, 0.72),
      0 4px 12px rgba(42, 64, 84, 0.1);
  }

  .toolbar-button:disabled {
    opacity: 0.4;
    cursor: not-allowed;
  }

  .icon {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    width: 18px;
    height: 18px;
  }

  .label {
    white-space: nowrap;
  }

  .toolbar-tooltip {
    position: absolute;
    left: 50%;
    top: calc(100% + 8px);
    z-index: 20;
    max-width: 180px;
    padding: 6px 8px;
    border: 1px solid rgba(123, 142, 161, 0.3);
    border-radius: 6px;
    background: rgba(35, 52, 69, 0.96);
    box-shadow: 0 8px 20px rgba(16, 28, 39, 0.24);
    color: #f8fbfe;
    font: 600 11px/1.2 "Segoe UI", sans-serif;
    text-align: center;
    white-space: nowrap;
    pointer-events: none;
    opacity: 0;
    transform: translate(-50%, -4px);
    transition:
      opacity 120ms ease,
      transform 120ms ease;
  }

  .toolbar-tooltip::before {
    content: "";
    position: absolute;
    left: 50%;
    bottom: 100%;
    width: 8px;
    height: 8px;
    border-left: 1px solid rgba(123, 142, 161, 0.3);
    border-top: 1px solid rgba(123, 142, 161, 0.3);
    background: rgba(35, 52, 69, 0.96);
    transform: translate(-50%, 50%) rotate(45deg);
  }

  .toolbar-button-shell:hover .toolbar-tooltip,
  .toolbar-button:focus-visible + .toolbar-tooltip {
    opacity: 1;
    transform: translate(-50%, 0);
  }

  @media (hover: none) {
    .toolbar-button-shell:hover .toolbar-tooltip {
      opacity: 0;
      transform: translate(-50%, -4px);
    }
  }

  @media (max-width: 900px) {
    .toolbar-button {
      padding: 8px 10px;
    }

    :global(.toolbar-shell.overlay) .toolbar-button {
      padding: 7px;
    }
  }
</style>
