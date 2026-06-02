<script lang="ts">
  import { isOffline } from '$lib/stores/network';
</script>

{#if $isOffline}
  <div class="bar" role="status" aria-live="polite">
    <span class="dot" aria-hidden="true"></span>
    <span>No connection</span>
  </div>
{/if}

<style>
  .bar {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    margin: var(--space-3) auto 0;
    padding: 4px 10px;
    background: var(--color-surface-secondary);
    color: var(--color-text-muted);
    border-radius: var(--radius-pill);
    font-size: var(--font-size-2xs);
    line-height: var(--line-height-2xs);
    font-weight: var(--font-weight-medium);
    user-select: none;
    width: fit-content;
  }
  .dot {
    width: 6px;
    height: 6px;
    background: currentColor;
    border-radius: 50%;
  }
</style>
