<script lang="ts">
  import { toasts, dismissToast, type ToastVariant } from '$lib/stores/toasts';
  import Icon from './icons/Icon.svelte';
  import type { ComponentProps } from 'svelte';
  type IconName = ComponentProps<typeof Icon>['name'];

  function iconFor(v: ToastVariant): IconName {
    if (v === 'success') return 'check';
    if (v === 'warning') return 'warning';
    if (v === 'error') return 'warning';
    return 'info';
  }

  function roleFor(v: ToastVariant): 'status' | 'alert' {
    return v === 'error' ? 'alert' : 'status';
  }
</script>

<div class="stack" aria-live="polite" aria-relevant="additions">
  {#each $toasts as t (t.id)}
    <div class="toast toast--{t.variant}" role={roleFor(t.variant)}>
      <div class="icon" aria-hidden="true">
        <Icon name={iconFor(t.variant)} size={16} />
      </div>
      <div class="body">
        <div class="title">{t.title}</div>
        {#if t.detail}<div class="detail">{t.detail}</div>{/if}
      </div>
      {#if t.action}
        <button type="button" class="action" onclick={() => { t.action!.onclick(); dismissToast(t.id); }}>
          {t.action.label}
        </button>
      {/if}
      <button type="button" class="close" aria-label="Dismiss" onclick={() => dismissToast(t.id)}>
        <Icon name="x" size={16} />
      </button>
    </div>
  {/each}
</div>

<style>
  .stack {
    position: fixed;
    bottom: var(--space-5);
    left: 50%;
    transform: translateX(-50%);
    display: flex;
    flex-direction: column;
    gap: var(--space-2);
    z-index: 2000;
    pointer-events: none;
  }

  .toast {
    pointer-events: auto;
    display: flex;
    align-items: flex-start;
    gap: var(--space-3);
    min-width: 320px;
    max-width: 480px;
    padding: var(--space-3) var(--space-4);
    background: var(--color-surface-raised);
    border: 1px solid var(--color-border);
    border-left: 4px solid var(--color-border);
    border-radius: var(--radius-control);
    box-shadow: var(--shadow-whisper);
    animation: slideIn var(--duration-base) var(--ease-standard);
  }

  @keyframes slideIn {
    from { opacity: 0; transform: translateY(8px); }
    to   { opacity: 1; transform: translateY(0); }
  }

  @media (prefers-reduced-motion: reduce) {
    .toast { animation: none; }
  }

  .toast--info    { border-left-color: var(--color-info); }
  .toast--success { border-left-color: var(--color-success); }
  .toast--warning { border-left-color: var(--color-warning); }
  .toast--error   { border-left-color: var(--color-error); }

  .icon {
    flex-shrink: 0;
    width: 18px;
    height: 18px;
    display: flex;
    align-items: center;
    justify-content: center;
    margin-top: 2px;
  }
  .toast--success .icon { color: var(--color-success); }
  .toast--warning .icon { color: var(--color-warning); }
  .toast--error   .icon { color: var(--color-error); }
  .toast--info    .icon { color: var(--color-info); }

  .body {
    flex: 1;
    min-width: 0;
  }

  .title {
    font-size: var(--font-size-sm);
    line-height: var(--line-height-sm);
    font-weight: var(--font-weight-semibold);
    color: var(--color-text-primary);
  }

  .detail {
    margin-top: 2px;
    font-size: var(--font-size-xs);
    line-height: var(--line-height-xs);
    color: var(--color-text-secondary);
  }

  .action {
    flex-shrink: 0;
    align-self: center;
    padding: 6px 10px;
    border: none;
    background: transparent;
    color: var(--color-primary-700);
    font-family: inherit;
    font-size: var(--font-size-xs);
    font-weight: var(--font-weight-semibold);
    border-radius: var(--radius-sm);
    cursor: pointer;
  }
  .action:hover { background: var(--color-primary-50); }

  .close {
    flex-shrink: 0;
    width: 24px;
    height: 24px;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    border: none;
    background: transparent;
    color: var(--color-text-muted);
    border-radius: var(--radius-sm);
    cursor: pointer;
    align-self: flex-start;
  }
  .close:hover {
    background: var(--color-neutral-100);
    color: var(--color-text-primary);
  }
</style>
