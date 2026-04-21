<svelte:options runes={true} />

<script lang="ts">
  import type { ImportReviewField } from "./import-review";

  interface Props {
    title: string;
    fields?: ImportReviewField[];
    emptyMessage?: string;
    wide?: boolean;
  }

  let { title, fields = [], emptyMessage = "", wide = false }: Props = $props();
</script>

<div class={["review-subsection", wide && "review-subsection-wide"]}>
  <strong>{title}</strong>
  {#if fields.length > 0}
    <ul class="review-field-list">
      {#each fields as field (`field:${title}:${field.label}`)}
        <li>
          <span>{field.label}</span>
          <strong>{field.value}</strong>
        </li>
      {/each}
    </ul>
  {:else if emptyMessage}
    <p class="muted-copy">{emptyMessage}</p>
  {/if}
</div>

<style>
  .review-subsection {
    display: grid;
    gap: 10px;
    padding: 12px;
    border: 1px solid var(--import-review-border, var(--app-border, rgba(255, 255, 255, 0.08)));
    border-radius: 6px;
    background: var(--import-review-bg, var(--surface-bg, rgba(255, 255, 255, 0.02)));
    color: var(--import-review-text, inherit);
  }

  .review-subsection-wide {
    grid-column: 1 / -1;
  }

  .review-field-list {
    list-style: none;
    margin: 0;
    padding: 0;
    display: grid;
    gap: 10px;
  }

  .review-field-list li {
    display: flex;
    align-items: flex-start;
    justify-content: space-between;
    gap: 12px;
    padding-bottom: 10px;
    border-bottom: 1px solid var(--import-review-divider, rgba(255, 255, 255, 0.06));
  }

  .review-field-list li:last-child {
    padding-bottom: 0;
    border-bottom: none;
  }

  .review-field-list span,
  .muted-copy {
    color: var(--import-review-muted, var(--text-muted, rgba(255, 255, 255, 0.72)));
  }

  .review-field-list strong {
    text-align: right;
    word-break: break-word;
  }

  .muted-copy {
    margin: 0;
  }
</style>
