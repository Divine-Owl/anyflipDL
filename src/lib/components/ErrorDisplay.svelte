<script lang="ts">
  import Button from './Button.svelte';
  import Icon from './icons/Icon.svelte';

  interface Props {
    message: string;
    code?: string;
    retryable?: boolean;
    onretry?: () => void;
    compact?: boolean;
    variant?: 'inline' | 'block';
  }

  let {
    message,
    code,
    retryable = false,
    onretry,
    compact = false,
    variant = 'inline',
  }: Props = $props();
</script>

{#if variant === 'block'}
  <div class="block">
    <div class="icon" aria-hidden="true">
      <Icon name="warning" size={20} />
    </div>
    <div class="body">
      <h4>{message}</h4>
      {#if code}<p class="code">{code}</p>{/if}
      {#if retryable && onretry}
        <Button variant="ghost" size="sm" onclick={onretry}>Try again</Button>
      {/if}
    </div>
  </div>
{:else}
  <div class="inline {compact ? 'inline--compact' : ''}" role="alert">
    <div class="bar" aria-hidden="true"></div>
    <div class="body">
      <div class="msg">{message}</div>
      {#if code}<div class="code">{code}</div>{/if}
    </div>
    {#if retryable && onretry}
      <Button variant="ghost" size="sm" onclick={onretry}>Try again</Button>
    {/if}
  </div>
{/if}

<style>
  .inline {
    display: flex;
    align-items: center;
    gap: var(--space-3);
    width: 100%;
    padding: var(--space-3) var(--space-4);
    background: var(--status-error-bg);
    border: 1px solid var(--color-border);
    border-left: 4px solid var(--status-error-text);
    border-radius: var(--radius-control);
  }

  .inline--compact {
    padding: var(--space-2) var(--space-3);
  }

  .bar { display: none; }

  .body {
    flex: 1;
    min-width: 0;
  }

  .msg {
    font-size: var(--font-size-sm);
    line-height: var(--line-height-sm);
    font-weight: var(--font-weight-semibold);
    color: var(--status-error-text);
  }

  .code {
    margin-top: 2px;
    font-family: var(--font-family-mono);
    font-size: var(--font-size-2xs);
    color: var(--color-text-muted);
  }

  .block {
    display: flex;
    flex-direction: column;
    align-items: center;
    text-align: center;
    gap: var(--space-3);
    padding: var(--space-6) var(--space-5);
    max-width: 400px;
    margin: 0 auto;
  }

  .icon {
    color: var(--status-error-text);
  }

  h4 {
    font-size: var(--font-size-base);
    line-height: var(--line-height-base);
    font-weight: var(--font-weight-semibold);
    color: var(--color-text-primary);
  }
</style>
