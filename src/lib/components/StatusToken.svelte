<script lang="ts">
  import type { Snippet } from 'svelte';

  type Variant = 'neutral' | 'active' | 'paused' | 'success' | 'error' | 'offline';

  interface Props {
    variant?: Variant;
    label: string;
    dot?: boolean;
    compact?: boolean;
    title?: string;
    children?: Snippet;
  }

  let {
    variant = 'neutral',
    label,
    dot = false,
    compact = false,
    title,
    children,
  }: Props = $props();
</script>

<span
  class="status-token status-token--{variant} {compact ? 'status-token--compact' : ''}"
  {title}
>
  {#if dot}<span class="dot" aria-hidden="true"></span>{/if}
  <span class="label">{label}</span>
  {#if children}{@render children()}{/if}
</span>

<style>
  .status-token {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    padding: 4px 10px;
    border-radius: var(--radius-chip);
    font-size: var(--font-size-2xs);
    line-height: var(--line-height-2xs);
    font-weight: var(--font-weight-semibold);
    white-space: nowrap;
    user-select: none;
  }

  .status-token--compact {
    padding: 2px 6px;
  }

  .dot {
    width: 6px;
    height: 6px;
    border-radius: 50%;
    background: currentColor;
    flex-shrink: 0;
  }

  .status-token--neutral {
    background: var(--status-neutral-bg);
    color: var(--status-neutral-text);
  }

  .status-token--active {
    background: var(--status-active-bg);
    color: var(--status-active-text);
  }

  .status-token--paused {
    background: var(--status-paused-bg);
    color: var(--status-paused-text);
  }

  .status-token--success {
    background: var(--status-success-bg);
    color: var(--status-success-text);
  }

  .status-token--error {
    background: var(--status-error-bg);
    color: var(--status-error-text);
  }

  .status-token--offline {
    background: var(--status-offline-bg);
    color: var(--status-offline-text);
  }
</style>
