<script lang="ts">
  import type { Snippet } from 'svelte';
  import type { HTMLAttributes } from 'svelte/elements';

  type As = 'div' | 'section' | 'article';

  interface Props extends HTMLAttributes<HTMLElement> {
    as?: As;
    interactive?: boolean;
    padding?: 'sm' | 'md' | 'lg' | 'none';
    class?: string;
    children?: Snippet;
  }

  let {
    as = 'div',
    interactive = false,
    padding = 'md',
    class: className = '',
    children,
    ...rest
  }: Props = $props();

  const padMap: Record<NonNullable<Props['padding']>, string> = {
    none: '0',
    sm: 'var(--space-4) var(--space-5)',
    md: 'var(--space-5) var(--space-6)',
    lg: 'var(--space-7) var(--space-8)',
  };
</script>

<svelte:element
  this={as}
  class="raised-card {interactive ? 'raised-card--interactive' : ''} {className}"
  style:padding={padMap[padding]}
  {...rest}
>
  {#if children}{@render children()}{/if}
</svelte:element>

<style>
  .raised-card {
    display: block;
    background: var(--color-surface-raised);
    border: 1px solid var(--color-border);
    border-radius: var(--radius-card);
    box-shadow: var(--shadow-whisper);
    color: var(--color-text-primary);
    box-sizing: border-box;
  }

  .raised-card--interactive {
    transition: border-color var(--duration-fast) var(--ease-standard),
                background-color var(--duration-fast) var(--ease-standard);
  }

  .raised-card--interactive:hover,
  .raised-card--interactive:focus-visible {
    border-color: var(--color-primary-200);
  }

  .raised-card--interactive:focus-visible {
    outline: var(--focus-ring);
    outline-offset: 2px;
  }
</style>
