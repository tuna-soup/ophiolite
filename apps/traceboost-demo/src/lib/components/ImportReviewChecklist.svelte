<svelte:options runes={true} />

<script lang="ts">
  import type { ImportReviewItem } from "./import-review";

  interface Props {
    items: ImportReviewItem[];
  }

  let { items }: Props = $props();
</script>

<ul class="review-list">
  {#each items as item, index (`review:${item.title}:${index}`)}
    <li class={["review-item", `review-${item.severity}`]}>
      <strong>{item.title}</strong>
      <span>{item.message}</span>
    </li>
  {/each}
</ul>

<style>
  .review-list {
    list-style: none;
    margin: 0;
    padding: 0;
    display: grid;
    gap: 10px;
  }

  .review-item {
    display: grid;
    gap: 4px;
    padding: 10px 12px;
    border: 1px solid var(--import-review-border, var(--app-border, rgba(255, 255, 255, 0.08)));
    border-radius: 6px;
    background: var(--import-review-bg, var(--surface-bg, rgba(255, 255, 255, 0.02)));
    color: var(--import-review-text, inherit);
  }

  .review-item span {
    color: var(--import-review-muted, var(--text-muted, rgba(255, 255, 255, 0.72)));
  }

  .review-blocking {
    border-color: var(--import-review-blocking-border, rgba(177, 71, 61, 0.28));
  }

  .review-warning {
    border-color: var(--import-review-warning-border, rgba(201, 145, 62, 0.3));
  }

  .review-info {
    border-color: var(--import-review-info-border, rgba(66, 140, 84, 0.24));
  }
</style>
