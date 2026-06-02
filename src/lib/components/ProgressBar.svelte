<script lang="ts">
  import { transitionDuration } from '$lib/utils/motion';

  interface Props {
    progress: number;
    currentPage?: number;
    totalPages?: number;
    size?: 'xs' | 'sm';
  }

  let { progress, currentPage, totalPages, size = 'sm' }: Props = $props();

  const clamped = $derived(Math.max(0, Math.min(100, progress)));
  const height = $derived(size === 'xs' ? '2px' : '4px');
  const dur = $derived(transitionDuration(160));
</script>

<div class="wrap" role="progressbar" aria-valuemin="0" aria-valuemax="100" aria-valuenow={clamped} style:height>
  <div class="fill" style:width="{clamped}%" style:transition="width {dur}ms linear"></div>
</div>

<style>
  .wrap {
    width: 100%;
    background: var(--color-neutral-100);
    border-radius: var(--radius-control);
    overflow: hidden;
  }

  .fill {
    height: 100%;
    background: var(--color-primary-400);
    border-radius: var(--radius-control);
  }
</style>
