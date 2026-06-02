<script lang="ts">
  import { overlayOpen, closeOverlay, isEditableTarget } from '$lib/stores/keyboard';
  import { activeView, navigateTo, toggleSettings } from '$lib/stores/ui';
  import {
    downloadItems, activeDownload, pauseDownload, resumeDownload,
    cancelActiveDownload, retryItem, removeFromQueue,
  } from '$lib/stores/downloads';
  import RaisedCard from './RaisedCard.svelte';
  import Icon from './icons/Icon.svelte';

  interface Group {
    title: string;
    shortcuts: { keys: string[]; description: string }[];
  }

  const groups: Group[] = [
    {
      title: 'Navigation',
      shortcuts: [
        { keys: ['1'], description: 'Go to New download' },
        { keys: ['2'], description: 'Go to Queue' },
        { keys: ['3'], description: 'Go to History' },
        { keys: ['4'], description: 'Go to Settings' },
      ],
    },
    {
      title: 'Queue',
      shortcuts: [
        { keys: ['Space'], description: 'Pause or resume the active download' },
        { keys: ['R'], description: 'Retry the selected failed download' },
        { keys: ['Delete'], description: 'Remove the active item from the queue' },
      ],
    },
    {
      title: 'General',
      shortcuts: [
        { keys: ['?'], description: 'Show or hide this overlay' },
        { keys: ['Esc'], description: 'Close the overlay' },
      ],
    },
  ];

  function onKey(e: KeyboardEvent) {
    if (e.defaultPrevented) return;
    if (isEditableTarget(e.target)) return;
    if (e.metaKey || e.ctrlKey || e.altKey) return;

    if (e.key === '?' || e.key === '/') {
      e.preventDefault();
      overlayOpen.update((v) => !v);
      return;
    }

    if (e.key === '1') navigateTo('url');
    else if (e.key === '2') navigateTo('downloads');
    else if (e.key === '3') navigateTo('history');
    else if (e.key === '4') { e.preventDefault(); toggleSettings(); }
    else if (e.key === ' ' && $activeView === 'downloads' && $activeDownload) {
      e.preventDefault();
      if ($activeDownload.status === 'paused') resumeDownload($activeDownload.id);
      else pauseDownload($activeDownload.id);
    }
    else if ((e.key === 'r' || e.key === 'R') && $activeView === 'downloads' && $activeDownload?.status === 'error') {
      e.preventDefault();
      retryItem($activeDownload.id);
    }
    else if (e.key === 'Delete' && $activeView === 'downloads' && $activeDownload) {
      e.preventDefault();
      removeFromQueue($activeDownload.id);
    }
    else if (e.key === 'Escape' && $overlayOpen) {
      closeOverlay();
    }
  }
</script>

<svelte:window onkeydown={onKey} />

{#if $overlayOpen}
  <!-- svelte-ignore a11y_click_events_have_key_events -->
  <!-- svelte-ignore a11y_no_static_element_interactions -->
  <div class="backdrop" onclick={closeOverlay}>
    <div class="modal-wrap" onclick={(e) => e.stopPropagation()}>
      <RaisedCard padding="lg" class="overlay">
        <div class="head">
          <h2>Keyboard shortcuts</h2>
          <button class="close" type="button" aria-label="Close" onclick={closeOverlay}>
            <Icon name="x" size={18} />
          </button>
        </div>
        {#each groups as g (g.title)}
          <section class="group">
            <h3>{g.title}</h3>
            <dl>
              {#each g.shortcuts as s, i (i)}
                <div class="row">
                  <dt>
                    {#each s.keys as k, ki (ki)}
                      <kbd>{k}</kbd>{ki < s.keys.length - 1 ? ' ' : ''}
                    {/each}
                  </dt>
                  <dd>{s.description}</dd>
                </div>
              {/each}
            </dl>
          </section>
        {/each}
      </RaisedCard>
    </div>
  </div>
{/if}

<style>
  .backdrop {
    position: fixed;
    inset: 0;
    background: rgba(15, 23, 42, 0.45);
    z-index: 1500;
    display: flex;
    align-items: center;
    justify-content: center;
    padding: var(--space-6);
    animation: fadeIn var(--duration-base) var(--ease-standard);
  }

  @keyframes fadeIn {
    from { opacity: 0; }
    to   { opacity: 1; }
  }

  .modal-wrap {
    width: 100%;
    max-width: 480px;
  }

  :global(.overlay) {
    max-height: 80vh;
    overflow-y: auto;
  }

  .head {
    display: flex;
    align-items: center;
    justify-content: space-between;
    margin-bottom: var(--space-5);
  }

  h2 {
    font-size: var(--font-size-xl);
    line-height: var(--line-height-xl);
    font-weight: var(--font-weight-semibold);
    color: var(--color-text-primary);
    letter-spacing: var(--letter-tracking-tight);
  }

  .close {
    width: 32px;
    height: 32px;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    border: none;
    background: transparent;
    color: var(--color-text-muted);
    border-radius: var(--radius-sm);
    cursor: pointer;
  }
  .close:hover { background: var(--color-neutral-100); color: var(--color-text-primary); }

  .group {
    padding-top: var(--space-4);
    margin-top: var(--space-4);
    border-top: 1px solid var(--color-border-subtle);
  }
  .group:first-of-type {
    border-top: none;
    padding-top: 0;
    margin-top: 0;
  }

  h3 {
    font-size: var(--font-size-2xs);
    line-height: var(--line-height-2xs);
    text-transform: uppercase;
    letter-spacing: var(--letter-tracking-wide);
    color: var(--color-text-muted);
    font-weight: var(--font-weight-semibold);
    margin-bottom: var(--space-3);
  }

  dl { display: flex; flex-direction: column; gap: var(--space-2); }
  .row {
    display: flex;
    justify-content: space-between;
    align-items: center;
    gap: var(--space-4);
  }
  dt { display: inline-flex; gap: 4px; }
  dd {
    font-size: var(--font-size-sm);
    line-height: var(--line-height-sm);
    color: var(--color-text-secondary);
    text-align: right;
  }
  kbd {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    min-width: 24px;
    padding: 2px 6px;
    font-family: var(--font-family-mono);
    font-size: var(--font-size-2xs);
    color: var(--color-text-primary);
    background: var(--color-surface-secondary);
    border: 1px solid var(--color-border);
    border-radius: var(--radius-sm);
  }
</style>
