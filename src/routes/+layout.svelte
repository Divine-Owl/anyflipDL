<script lang="ts">
  import '../app.css';
  import { onMount } from 'svelte';
  import AppLayout from '$lib/layout/AppLayout.svelte';
  import { loadSettings } from '$lib/stores/settings';
  import { setupDownloadListeners } from '$lib/ipc/events';

  let { children } = $props();

  onMount(() => {
    loadSettings();
    const unlisten = setupDownloadListeners();
    return () => {
      unlisten.then((fn: () => void) => fn());
    };
  });
</script>

<AppLayout>
  {@render children()}
</AppLayout>
