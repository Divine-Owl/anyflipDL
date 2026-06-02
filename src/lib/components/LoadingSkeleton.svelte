<script lang="ts">
  import { onMount } from 'svelte';

  interface Props {
    width?: string;
    height?: string;
    count?: number;
    gap?: string;
    rounded?: 'sm' | 'md' | 'card';
    shimmer?: boolean;
  }

  let {
    width = '100%',
    height = '16px',
    count = 1,
    gap = 'var(--space-3)',
    rounded = 'md',
    shimmer = false,
  }: Props = $props();

  let animated = $state(false);

  onMount(() => {
    if (!shimmer) return;
    animated = true;
    const t = setTimeout(() => { animated = false; }, 3000);
    return () => clearTimeout(t);
  });

  const radiusMap = {
    sm: 'var(--radius-sm)',
    md: 'var(--radius-chip)',
    card: 'var(--radius-card)',
  };
</script>

<div class="skel" style:gap={gap} aria-hidden="true">
  {#each Array.from({ length: count }) as _, i (i)}
    <div
      class="block {animated ? 'block--shimmer' : ''}"
      style:width
      style:height
      style:border-radius={radiusMap[rounded]}
    ></div>
  {/each}
</div>

<style>
  .skel { display: flex; flex-direction: column; }

  .block {
    background: var(--color-neutral-100);
    position: relative;
    overflow: hidden;
  }

  .block--shimmer::after {
    content: '';
    position: absolute;
    inset: 0;
    background: linear-gradient(
      90deg,
      transparent 0%,
      rgba(255, 255, 255, 0.4) 50%,
      transparent 100%
    );
    animation: shimmer 1.6s linear infinite;
  }

  @keyframes shimmer {
    0%   { transform: translateX(-100%); }
    100% { transform: translateX(100%); }
  }

  @media (prefers-reduced-motion: reduce) {
    .block--shimmer::after { animation: none; }
  }
</style>
