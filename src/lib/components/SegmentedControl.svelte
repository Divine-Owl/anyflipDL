<script lang="ts">
  import Icon, { type IconName } from './icons/Icon.svelte';

  interface Option {
    value: string;
    label: string;
    icon?: IconName;
  }

  interface Props {
    value: string;
    options: Option[];
    ariaLabel?: string;
    disabled?: boolean;
    onChange?: (value: string) => void;
  }

  let { value = $bindable(''), options, ariaLabel, disabled = false, onChange }: Props = $props();

  function pick(v: string) {
    if (disabled || v === value) return;
    value = v;
    onChange?.(v);
  }
</script>

<div class="seg" role="radiogroup" aria-label={ariaLabel}>
  {#each options as opt (opt.value)}
    {@const isActive = opt.value === value}
    <button
      type="button"
      class="pill {isActive ? 'pill--active' : ''}"
      role="radio"
      aria-checked={isActive}
      {disabled}
      onclick={() => pick(opt.value)}
    >
      {#if opt.icon}<Icon name={opt.icon} size={16} />{/if}
      <span>{opt.label}</span>
    </button>
  {/each}
</div>

<style>
  .seg {
    display: inline-flex;
    align-items: center;
    background: var(--color-surface-secondary);
    border-radius: var(--radius-chip);
    padding: 3px;
    gap: 2px;
    border: 1px solid var(--color-border);
  }

  .pill {
    display: inline-flex;
    align-items: center;
    gap: var(--space-2);
    height: 32px;
    padding: 0 var(--space-4);
    border: none;
    background: transparent;
    color: var(--color-text-secondary);
    font-family: inherit;
    font-size: var(--font-size-sm);
    font-weight: var(--font-weight-medium);
    line-height: 1;
    border-radius: 6px;
    cursor: pointer;
    transition:
      background-color var(--duration-fast) var(--ease-standard),
      color var(--duration-fast) var(--ease-standard);
    white-space: nowrap;
  }

  .pill:hover:not(.pill--active):not(:disabled) {
    color: var(--color-text-primary);
  }

  .pill--active {
    background: var(--color-surface-elevated);
    color: var(--color-primary-700);
    font-weight: var(--font-weight-semibold);
    box-shadow: var(--shadow-whisper);
  }

  .pill:focus-visible {
    outline: var(--focus-ring);
    outline-offset: 2px;
  }

  .pill:disabled { opacity: 0.5; cursor: not-allowed; }
</style>
