<svelte:options runes={true} />

<script lang="ts">
  import ToolbarIconButton from "./ToolbarIconButton.svelte";
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
        <ToolbarIconButton
          label={item.label}
          icon={item.icon}
          showLabel={!iconOnly}
          disabled={item.disabled}
          active={item.active}
          pressed={item.active}
          ariaLabel={toolAriaLabel(item)}
          onclick={() => onToolSelect?.(item.id)}
        />
      {/each}
    </div>
  {/if}

  {#if actions.length > 0}
    <div class="toolbar-group" role="toolbar" aria-label="Actions">
      {#each actions as item (item.id)}
        <ToolbarIconButton
          label={item.label}
          icon={item.icon}
          showLabel={!iconOnly}
          disabled={item.disabled}
          ariaLabel={actionAriaLabel(item)}
          onclick={() => onActionSelect?.(item.id)}
        />
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
    overflow: visible;
    padding: 4px;
    border: 1px solid var(--ophiolite-toolbar-border, rgba(176, 212, 238, 0.72));
    border-radius: 8px;
    background: var(--ophiolite-toolbar-bg, rgba(255, 255, 255, 0.94));
    box-shadow:
      inset 0 0 0 1px rgba(255, 255, 255, 0.35),
      0 10px 24px rgba(42, 64, 84, 0.12);
    backdrop-filter: blur(12px);
  }

  @media (max-width: 900px) {
    .toolbar-shell {
      gap: 8px;
    }
  }
</style>
