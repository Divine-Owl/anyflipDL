<script lang="ts">
  import type { Snippet } from 'svelte';
  import ErrorDisplay from './ErrorDisplay.svelte';
  import RaisedCard from './RaisedCard.svelte';

  interface Props {
    children: Snippet;
  }

  let { children }: Props = $props();

  let error: Error | null = $state(null);
  let resetKey = $state(0);

  function handleError(e: Error | unknown) {
    if (e instanceof Error) error = e;
    else error = new Error(String(e));
  }

  export function reset() {
    error = null;
    resetKey += 1;
  }

  $effect(() => {
    resetKey;
  });
</script>

{#if error}
  <RaisedCard padding="lg">
    <ErrorDisplay
      message={error.message || 'Something went wrong'}
      code={error.name}
      retryable={true}
      variant="block"
      onretry={reset}
    />
  </RaisedCard>
{:else}
  {#key resetKey}
    {@render children()}
  {/key}
{/if}
