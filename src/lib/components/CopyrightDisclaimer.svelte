<script lang="ts" module>
  // Static disclaimer text — kept verbatim. Do not edit without legal review.
  const COPYRIGHT_TEXT_FULL = `AnyflipDL is an unofficial third-party tool. It is not affiliated with, endorsed by, sponsored by, or in any way associated with Anyflip.com or its owners.

This application is intended solely for downloading documents that the user already has the legal right to access offline. Users are responsible for ensuring their use of downloaded content complies with applicable copyright laws, the terms of service of Anyflip.com, and any licenses attached to the documents they download.

The developers of AnyflipDL do not host, mirror, or claim ownership of any content downloaded through this tool. All trademarks, document titles, and copyrighted materials remain the property of their respective owners.

By using this application, you acknowledge that you have the right to download the content you target, and you agree that the developers of AnyflipDL bear no responsibility for misuse.`;
</script>

<script lang="ts">
  import RaisedCard from './RaisedCard.svelte';

  interface Props {
    variant?: 'compact' | 'full';
  }

  let { variant = 'compact' }: Props = $props();
</script>

{#if variant === 'full'}
  <RaisedCard padding="md">
    <h4>Copyright notice</h4>
    <p class="body">{COPYRIGHT_TEXT_FULL}</p>
  </RaisedCard>
{:else}
  <p class="compact">
    AnyflipDL is a third-party tool. Users are responsible for respecting copyright and the Anyflip.com terms of service.
  </p>
{/if}

<style>
  .compact {
    font-size: var(--font-size-2xs);
    line-height: var(--line-height-2xs);
    color: var(--color-text-muted);
  }
  h4 {
    font-size: var(--font-size-sm);
    line-height: var(--line-height-sm);
    font-weight: var(--font-weight-semibold);
    color: var(--color-text-primary);
    margin-bottom: var(--space-2);
  }
  .body {
    font-size: var(--font-size-xs);
    line-height: 1.55;
    color: var(--color-text-secondary);
    white-space: pre-wrap;
  }
</style>
