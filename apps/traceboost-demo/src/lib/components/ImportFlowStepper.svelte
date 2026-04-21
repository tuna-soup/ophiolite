<svelte:options runes={true} />

<script lang="ts">
  import type { ImportConfirmationStage, ImportFlowStep } from "./import-review";

  interface Props {
    stage: ImportConfirmationStage;
    steps: ImportFlowStep[];
    onSelect?: (stage: ImportConfirmationStage) => void;
  }

  let { stage, steps, onSelect }: Props = $props();

  const activeStepDescription = $derived.by(
    () => steps.find((step) => step.key === stage)?.description ?? ""
  );

  function statusLabel(status: ImportFlowStep["status"]): string {
    switch (status) {
      case "active":
        return "Current";
      case "completed":
        return "Passed";
      case "warning":
        return "Review";
      case "blocking":
        return "Fix";
      default:
        return "Pending";
    }
  }
</script>

<div class="stepper">
  {#each steps as step (step.key)}
    <button
      type="button"
      class={[
        "step-chip",
        step.key === stage && "active-step",
        step.status && `step-${step.status}`
      ]}
      onclick={() => {
        if (!step.disabled) {
          onSelect?.(step.key);
        }
      }}
      disabled={step.disabled ?? false}
    >
      <span class="step-label-row">
        <span>{step.label}</span>
        <span class="step-status">{statusLabel(step.status)}</span>
      </span>
      {#if step.detail}
        <span class="step-detail">{step.detail}</span>
      {/if}
    </button>
  {/each}
</div>
{#if activeStepDescription}
  <p class="step-copy">{activeStepDescription}</p>
{/if}

<style>
  .stepper {
    display: grid;
    gap: 12px;
    grid-template-columns: repeat(var(--import-stepper-columns, 3), minmax(0, 1fr));
  }

  .step-chip {
    min-height: 56px;
    padding: 10px 12px;
    border: 1px solid var(--import-step-border, var(--app-border, rgba(255, 255, 255, 0.08)));
    border-radius: 6px;
    background: var(--import-step-bg, var(--surface-bg, rgba(255, 255, 255, 0.02)));
    color: var(--import-step-text, inherit);
    font: inherit;
    text-align: left;
    display: grid;
    gap: 6px;
  }

  .active-step {
    border-color: var(
      --import-step-active-border,
      color-mix(in srgb, var(--accent-solid, #5e92e0) 35%, white)
    );
    background: var(
      --import-step-active-bg,
      color-mix(in srgb, var(--accent-solid, #5e92e0) 12%, white)
    );
  }

  .step-pending {
    opacity: 0.78;
  }

  .step-completed {
    border-color: color-mix(in srgb, #2eae6b 38%, var(--app-border, rgba(255, 255, 255, 0.08)));
  }

  .step-warning {
    border-color: rgba(214, 154, 42, 0.55);
    background: color-mix(in srgb, rgba(214, 154, 42, 0.16) 70%, transparent);
  }

  .step-blocking {
    border-color: rgba(220, 76, 76, 0.55);
    background: color-mix(in srgb, rgba(220, 76, 76, 0.16) 70%, transparent);
  }

  .step-label-row {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 12px;
  }

  .step-status,
  .step-detail {
    color: var(--import-step-muted, var(--text-muted, rgba(255, 255, 255, 0.72)));
    font-size: 0.84rem;
  }

  .step-status {
    white-space: nowrap;
  }

  .step-copy {
    margin: 0;
    color: var(--import-step-muted, var(--text-muted, rgba(255, 255, 255, 0.72)));
  }

  @media (max-width: 820px) {
    .stepper {
      grid-template-columns: 1fr;
    }
  }
</style>
