<svelte:options runes={true} />

<script lang="ts">
  import type { ChartToolbarActionItem, ChartToolbarToolItem } from "./types";

  interface Props {
    label?: string;
    tools?: ChartToolbarToolItem[];
    actions?: ChartToolbarActionItem[];
    onToolSelect?: (toolId: string) => void;
    onActionSelect?: (actionId: string) => void;
    variant?: "standard" | "overlay";
    iconOnly?: boolean;
  }

  let {
    label = "Chart interactions",
    tools = [],
    actions = [],
    onToolSelect,
    onActionSelect,
    variant = "standard",
    iconOnly = false
  }: Props = $props();

  function toolAriaLabel(item: ChartToolbarToolItem): string {
    return item.active ? `${item.label} tool active` : `Activate ${item.label} tool`;
  }

  function actionAriaLabel(item: ChartToolbarActionItem): string {
    return item.label;
  }
</script>

<div class={["toolbar-shell", variant === "overlay" && "overlay"]} role="group" aria-label={label}>
  {#if tools.length > 0}
    <div class="toolbar-group" role="toolbar" aria-label="Tools">
      {#each tools as item (item.id)}
        <button
          type="button"
          class={["toolbar-button", item.active && "active"]}
          disabled={item.disabled}
          aria-pressed={item.active}
          aria-label={toolAriaLabel(item)}
          title={item.label}
          onclick={() => onToolSelect?.(item.id)}
        >
          <span class="icon" aria-hidden="true">
            {#if item.icon === "pointer"}
              <svg viewBox="0 0 24 24" width="18" height="18" fill="none" stroke="currentColor" stroke-width="1.8">
                <path d="M6 4L15 14H11L13 20L10.8 21L8.8 15.8L5.4 18.2Z" stroke-linejoin="round" />
              </svg>
            {:else if item.icon === "crosshair"}
              <svg viewBox="0 0 24 24" width="18" height="18" fill="none" stroke="currentColor" stroke-width="1.8">
                <circle cx="12" cy="12" r="4.5" />
                <path d="M12 2V6M12 18V22M2 12H6M18 12H22" stroke-linecap="round" />
              </svg>
            {:else if item.icon === "pan"}
              <svg viewBox="0 0 24 24" width="18" height="18" fill="none" stroke="currentColor" stroke-width="1.8">
                <path d="M12 3V21M3 12H21" stroke-linecap="round" />
                <path d="M8 6L12 2L16 6M18 8L22 12L18 16M16 18L12 22L8 18M6 16L2 12L6 8" stroke-linecap="round" stroke-linejoin="round" />
              </svg>
            {/if}
          </span>
          {#if !iconOnly}
            <span class="label">{item.label}</span>
          {/if}
        </button>
      {/each}
    </div>
  {/if}

  {#if actions.length > 0}
    <div class="toolbar-group" role="toolbar" aria-label="Actions">
      {#each actions as item (item.id)}
        <button
          type="button"
          class={["toolbar-button", "action"]}
          disabled={item.disabled}
          aria-label={actionAriaLabel(item)}
          title={item.label}
          onclick={() => onActionSelect?.(item.id)}
        >
          <span class="icon" aria-hidden="true">
            {#if item.icon === "fitToData"}
              <svg viewBox="0 0 24 24" width="18" height="18" fill="none" stroke="currentColor" stroke-width="1.8">
                <path d="M4 9V4H9M15 4H20V9M20 15V20H15M9 20H4V15" stroke-linecap="round" stroke-linejoin="round" />
                <path d="M8 8L4 4M16 8L20 4M16 16L20 20M8 16L4 20" stroke-linecap="round" />
              </svg>
            {:else if item.icon === "settings"}
              <svg viewBox="0 0 24 24" width="18" height="18" fill="none" stroke="currentColor" stroke-width="1.8">
                <circle cx="12" cy="12" r="3.2" />
                <path d="M19.4 15a1 1 0 0 0 .2 1.1l.1.1a1.2 1.2 0 0 1 0 1.7l-1.2 1.2a1.2 1.2 0 0 1-1.7 0l-.1-.1a1 1 0 0 0-1.1-.2 1 1 0 0 0-.6.9V20a1.2 1.2 0 0 1-1.2 1.2h-1.7A1.2 1.2 0 0 1 10 20v-.2a1 1 0 0 0-.6-.9 1 1 0 0 0-1.1.2l-.1.1a1.2 1.2 0 0 1-1.7 0l-1.2-1.2a1.2 1.2 0 0 1 0-1.7l.1-.1a1 1 0 0 0 .2-1.1 1 1 0 0 0-.9-.6H4a1.2 1.2 0 0 1-1.2-1.2v-1.7A1.2 1.2 0 0 1 4 10h.2a1 1 0 0 0 .9-.6 1 1 0 0 0-.2-1.1l-.1-.1a1.2 1.2 0 0 1 0-1.7l1.2-1.2a1.2 1.2 0 0 1 1.7 0l.1.1a1 1 0 0 0 1.1.2 1 1 0 0 0 .6-.9V4A1.2 1.2 0 0 1 10.7 2.8h1.7A1.2 1.2 0 0 1 13.6 4v.2a1 1 0 0 0 .6.9 1 1 0 0 0 1.1-.2l.1-.1a1.2 1.2 0 0 1 1.7 0l1.2 1.2a1.2 1.2 0 0 1 0 1.7l-.1.1a1 1 0 0 0-.2 1.1 1 1 0 0 0 .9.6h.2A1.2 1.2 0 0 1 21.2 10v1.7A1.2 1.2 0 0 1 20 12.9h-.2a1 1 0 0 0-.9.6" stroke-linecap="round" stroke-linejoin="round" />
              </svg>
            {/if}
          </span>
          {#if !iconOnly}
            <span class="label">{item.label}</span>
          {/if}
        </button>
      {/each}
    </div>
  {/if}
</div>

<style>
  .toolbar-shell {
    display: flex;
    flex-wrap: wrap;
    gap: 10px;
    align-items: center;
  }

  .toolbar-shell.overlay {
    gap: 8px;
  }

  .toolbar-group {
    display: inline-flex;
    align-items: stretch;
    gap: 0;
    padding: 4px;
    border: 1px solid var(--ophiolite-toolbar-border, rgba(176, 212, 238, 0.72));
    border-radius: 8px;
    background: var(--ophiolite-toolbar-bg, rgba(255, 255, 255, 0.94));
    box-shadow:
      inset 0 0 0 1px rgba(255, 255, 255, 0.35),
      0 10px 24px rgba(42, 64, 84, 0.12);
    backdrop-filter: blur(12px);
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

  .toolbar-shell.overlay .toolbar-button {
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

  @media (max-width: 900px) {
    .toolbar-shell {
      gap: 8px;
    }

    .toolbar-button {
      padding: 8px 10px;
    }
  }
</style>
