<script lang="ts">
  import type { Snippet } from 'svelte';
  import type { HTMLButtonAttributes } from 'svelte/elements';
  import Icon, { type IconName } from './icons/Icon.svelte';

  type Variant = 'primary' | 'secondary' | 'ghost' | 'danger';
  type Size = 'sm' | 'md' | 'lg';

  interface Props extends Omit<HTMLButtonAttributes, 'children'> {
    variant?: Variant;
    size?: Size;
    full?: boolean;
    leadingIcon?: IconName;
    trailingIcon?: IconName;
    loading?: boolean;
    children?: Snippet;
  }

  let {
    variant = 'primary',
    size = 'md',
    full = false,
    leadingIcon,
    trailingIcon,
    loading = false,
    disabled,
    type = 'button',
    class: className = '',
    children,
    ...rest
  }: Props = $props();

  const heightMap: Record<Size, string> = {
    sm: 'var(--btn-height-sm)',
    md: 'var(--btn-height-md)',
    lg: 'var(--btn-height-lg)',
  };

  const fontMap: Record<Size, string> = {
    sm: 'var(--font-size-xs)',
    md: 'var(--font-size-sm)',
    lg: 'var(--font-size-base)',
  };
</script>

<button
  {type}
  class="btn btn--{variant} btn--{size} {full ? 'btn--full' : ''} {className}"
  style:height={heightMap[size]}
  style:font-size={fontMap[size]}
  disabled={disabled || loading}
  {...rest}
>
  {#if loading}
    <span class="spinner" aria-hidden="true"></span>
  {:else if leadingIcon}
    <Icon name={leadingIcon} size={size === 'lg' ? 20 : 16} />
  {/if}
  {#if children}<span class="label">{@render children()}</span>{/if}
  {#if trailingIcon && !loading}
    <Icon name={trailingIcon} size={size === 'lg' ? 20 : 16} />
  {/if}
</button>

<style>
  .btn {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    gap: var(--space-2);
    padding: 0 var(--space-4);
    border-radius: var(--btn-radius);
    font-family: inherit;
    font-weight: var(--font-weight-semibold);
    line-height: 1;
    border: 1px solid transparent;
    cursor: pointer;
    user-select: none;
    text-decoration: none;
    transition:
      background-color var(--duration-fast) var(--ease-standard),
      border-color var(--duration-fast) var(--ease-standard),
      color var(--duration-fast) var(--ease-standard);
  }

  .btn--full { width: 100%; }
  .btn--sm { padding: 0 var(--space-3); }
  .btn--md { padding: 0 var(--space-4); }
  .btn--lg { padding: 0 var(--space-5); }

  .btn:focus-visible {
    outline: var(--focus-ring);
    outline-offset: 2px;
  }

  .btn:disabled {
    cursor: not-allowed;
    opacity: 0.6;
  }

  .btn--primary {
    background: var(--btn-primary-bg);
    color: var(--btn-primary-text);
  }
  .btn--primary:hover:not(:disabled) { background: var(--btn-primary-bg-hover); }
  .btn--primary:active:not(:disabled) { background: var(--btn-primary-bg-active); }

  .btn--secondary {
    background: var(--btn-secondary-bg);
    color: var(--btn-secondary-text);
    border-color: var(--btn-secondary-border);
  }
  .btn--secondary:hover:not(:disabled) {
    background: var(--btn-secondary-bg-hover);
    border-color: transparent;
  }
  .btn--secondary:active:not(:disabled) { background: var(--btn-secondary-bg-active); }

  .btn--ghost {
    background: var(--btn-ghost-bg);
    color: var(--btn-ghost-text);
  }
  .btn--ghost:hover:not(:disabled) {
    background: var(--btn-ghost-bg-hover);
    color: var(--btn-ghost-text-hover);
  }
  .btn--ghost:active:not(:disabled) { background: var(--btn-ghost-bg-active); }

  .btn--danger {
    background: var(--btn-danger-bg);
    color: var(--btn-danger-text);
  }
  .btn--danger:hover:not(:disabled) { background: var(--btn-danger-bg-hover); }
  .btn--danger:active:not(:disabled) { background: var(--btn-danger-bg-active); }

  .label {
    display: inline-block;
  }

  .spinner {
    width: 14px;
    height: 14px;
    border: 2px solid currentColor;
    border-top-color: transparent;
    border-radius: 50%;
    animation: spin 0.8s linear infinite;
  }

  @keyframes spin {
    to { transform: rotate(360deg); }
  }

  @media (prefers-reduced-motion: reduce) {
    .spinner { animation: none; }
  }
</style>
