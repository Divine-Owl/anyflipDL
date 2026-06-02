<script lang="ts">
  import Icon, { type IconName } from './icons/Icon.svelte';
  import Button from './Button.svelte';

  interface Props {
    icon: IconName;
    heading: string;
    description?: string;
    actionLabel?: string;
    onAction?: () => void;
  }

  let { icon, heading, description, actionLabel, onAction }: Props = $props();
</script>

<div class="empty">
  <div class="icon" aria-hidden="true">
    <Icon name={icon} size={32} />
  </div>
  <h3>{heading}</h3>
  {#if description}<p>{description}</p>{/if}
  {#if actionLabel && onAction}
    <div class="action">
      <Button variant="primary" size="md" onclick={onAction}>
        {actionLabel}
      </Button>
    </div>
  {/if}
</div>

<style>
  .empty {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    text-align: center;
    max-width: 360px;
    margin: 0 auto;
    padding: var(--space-7) var(--space-5);
    gap: var(--space-3);
  }

  .icon {
    color: var(--color-text-muted);
    opacity: 0.6;
    margin-bottom: var(--space-2);
  }

  h3 {
    font-size: var(--font-size-lg);
    line-height: var(--line-height-lg);
    font-weight: var(--font-weight-semibold);
    color: var(--color-text-primary);
    letter-spacing: var(--letter-tracking-tight);
  }

  p {
    font-size: var(--font-size-sm);
    line-height: var(--line-height-sm);
    color: var(--color-text-secondary);
  }

  .action {
    margin-top: var(--space-4);
  }
</style>
