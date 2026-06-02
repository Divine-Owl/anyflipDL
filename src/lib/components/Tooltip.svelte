<script lang="ts">
  import type { Snippet } from 'svelte';
  import { untrack } from 'svelte';

  type Placement = 'top' | 'bottom' | 'left' | 'right';

  interface Props {
    label: string;
    placement?: Placement;
    children: Snippet;
  }

  let { label, placement = 'top', children }: Props = $props();

  let open = $state(false);
  let anchor: HTMLElement | null = $state(null);
  let pos = $state({ x: 0, y: 0, actual: untrack(() => placement) });
  let showTimer: ReturnType<typeof setTimeout> | null = null;

  function measure() {
    if (!anchor) return;
    const r = anchor.getBoundingClientRect();
    const tw = 200, th = 32;
    let x = 0, y = 0, actual = placement;

    if (placement === 'top') {
      x = r.left + r.width / 2 - tw / 2;
      y = r.top - th - 8;
      if (y < 4) { actual = 'bottom'; y = r.bottom + 8; }
    } else if (placement === 'bottom') {
      x = r.left + r.width / 2 - tw / 2;
      y = r.bottom + 8;
      if (y + th > window.innerHeight - 4) { actual = 'top'; y = r.top - th - 8; }
    } else if (placement === 'left') {
      x = r.left - tw - 8;
      y = r.top + r.height / 2 - th / 2;
      if (x < 4) { actual = 'right'; x = r.right + 8; }
    } else {
      x = r.right + 8;
      y = r.top + r.height / 2 - th / 2;
      if (x + tw > window.innerWidth - 4) { actual = 'left'; x = r.left - tw - 8; }
    }

    x = Math.max(4, Math.min(window.innerWidth - tw - 4, x));
    y = Math.max(4, y);
    pos = { x, y, actual };
  }

  function scheduleOpen() {
    showTimer = setTimeout(() => { open = true; measure(); }, 120);
  }
  function cancelOpen() {
    if (showTimer) { clearTimeout(showTimer); showTimer = null; }
    open = false;
  }

  function onKey(e: KeyboardEvent) {
    if (e.key === 'Escape') cancelOpen();
  }
</script>

<svelte:window onkeydown={onKey} />

<span
  class="anchor"
  bind:this={anchor}
  onmouseenter={scheduleOpen}
  onmouseleave={cancelOpen}
  onfocusin={scheduleOpen}
  onfocusout={cancelOpen}
  role="presentation"
>
  {@render children()}
</span>

{#if open}
  <span
    class="tip tip--{pos.actual}"
    style:left="{pos.x}px"
    style:top="{pos.y}px"
    role="tooltip"
  >
    {label}
  </span>
{/if}

<style>
  .anchor { display: inline-flex; }

  .tip {
    position: fixed;
    background: rgba(15, 23, 42, 0.92);
    color: #ffffff;
    font-size: var(--font-size-2xs);
    line-height: var(--line-height-2xs);
    padding: 6px 10px;
    border-radius: 6px;
    pointer-events: none;
    z-index: 1000;
    max-width: 240px;
    text-align: center;
    animation: tipIn 80ms var(--ease-standard);
  }

  @keyframes tipIn {
    from { opacity: 0; transform: translateY(2px); }
    to   { opacity: 1; transform: translateY(0); }
  }

  @media (prefers-reduced-motion: reduce) {
    .tip { animation: none; }
  }
</style>
