<script lang="ts">
  import {
    downloadItems,
    activeDownload,
    queueLength,
    completedCount,
    errorCount,
    retryItem,
    cancelActiveDownload,
    pauseDownload,
    resumeDownload,
    updateItemFormat,
    updateItemTitle,
    removeFromQueue,
    clearCompleted,
    clearErrors,
  } from '$lib/stores/downloads';
  import { showToast } from '$lib/stores/toasts';
  import { soundNotification } from '$lib/stores/settings';
  import { playNotificationSound } from '$lib/utils/sound';
  import { navigateTo } from '$lib/stores/ui';
  import { isOffline } from '$lib/stores/network';
  import type { DownloadItem, DownloadStatus } from '$lib/types';
  import { formatDate } from '$lib/utils/date';
  import RaisedCard from '$lib/components/RaisedCard.svelte';
  import StatusToken from '$lib/components/StatusToken.svelte';
  import Button from '$lib/components/Button.svelte';
  import ProgressBar from '$lib/components/ProgressBar.svelte';
  import SegmentedControl from '$lib/components/SegmentedControl.svelte';
  import ErrorDisplay from '$lib/components/ErrorDisplay.svelte';
  import EmptyState from '$lib/components/EmptyState.svelte';
  import Icon from '$lib/components/icons/Icon.svelte';

  // Undo state for destructive removes (pending, not yet permanent).
  const removedItems = $state(
    new Map<string, { item: DownloadItem; timeoutId: ReturnType<typeof setTimeout> }>(),
  );

  // Inline rename state.
  let editingId = $state<string | null>(null);
  let editTitle = $state('');
  let renameInput: HTMLInputElement | undefined = $state();

  // Per-item thumbnail failure tracking (id -> failed).
  let thumbFailed = $state(new Set<string>());

  // Toast on complete: detect transitions into 'complete' status.
  let lastSeen = new Map<string, DownloadStatus>();
  $effect(() => {
    const seen = lastSeen;
    for (const item of $downloadItems) {
      const prev = seen.get(item.id);
      if (prev !== undefined && prev !== item.status) {
        if (item.status === 'complete') {
          showToast({
            variant: 'success',
            title: 'Download complete',
            detail: `"${item.title}" finished downloading.`,
          });
          if ($soundNotification) playNotificationSound();
        } else if (item.status === 'error' && item.error) {
          showToast({
            variant: 'error',
            title: 'Download failed',
            detail: item.error,
            duration: 6000,
          });
        }
      }
      seen.set(item.id, item.status);
    }
  });

  function tokenFor(status: DownloadStatus): { variant: 'neutral' | 'active' | 'paused' | 'success' | 'error' | 'offline'; label: string } {
    switch (status) {
      case 'queued': return { variant: 'neutral', label: 'Queued' };
      case 'downloading': return { variant: 'active', label: 'Downloading' };
      case 'converting': return { variant: 'active', label: 'Converting' };
      case 'saving': return { variant: 'active', label: 'Saving' };
      case 'paused': return { variant: 'paused', label: 'Paused' };
      case 'complete': return { variant: 'success', label: 'Complete' };
      case 'error': return { variant: 'error', label: 'Error' };
      case 'password-required': return { variant: 'paused', label: 'Password' };
    }
  }

  function isActive(item: DownloadItem): boolean {
    return (
      item.status === 'downloading' ||
      item.status === 'converting' ||
      item.status === 'saving' ||
      item.status === 'paused'
    );
  }

  function isProgressVisible(item: DownloadItem): boolean {
    return (
      item.status === 'downloading' ||
      item.status === 'converting' ||
      item.status === 'saving' ||
      item.status === 'paused'
    );
  }

  function startRename(item: DownloadItem) {
    if (item.status !== 'queued') return;
    editingId = item.id;
    editTitle = item.title;
    requestAnimationFrame(() => renameInput?.select());
  }

  function commitRename(id: string) {
    const trimmed = editTitle.trim();
    if (trimmed) updateItemTitle(id, trimmed);
    editingId = null;
  }

  function cancelRename() {
    editingId = null;
  }

  function handleRemove(item: DownloadItem) {
    if (isActive(item)) return;
    const existing = removedItems.get(item.id);
    if (existing) {
      clearTimeout(existing.timeoutId);
      removedItems.delete(item.id);
      removeFromQueue(item.id);
      return;
    }
    removedItems.set(item.id, {
      item: { ...item },
      timeoutId: setTimeout(() => {
        removedItems.delete(item.id);
        removeFromQueue(item.id);
      }, 3000),
    });
    removeFromQueue(item.id);
  }

  function undoRemove(id: string) {
    const entry = removedItems.get(id);
    if (!entry) return;
    clearTimeout(entry.timeoutId);
    removedItems.delete(id);
    downloadItems.update((items) => [...items, entry.item]);
  }

  function goStart() {
    navigateTo('url');
  }
</script>

<div class="screen">
  <header class="page-header">
    <h1>Queue</h1>
    <div class="stats">
      {#if $queueLength > 0}
        <StatusToken variant="neutral" label="{$queueLength} queued" />
      {/if}
      {#if $completedCount > 0}
        <StatusToken variant="success" label="{$completedCount} done" />
      {/if}
      {#if $errorCount > 0}
        <StatusToken variant="error" label="{$errorCount} failed" />
      {/if}
    </div>
    <div class="header-actions">
      {#if $completedCount > 0}
        <Button variant="ghost" size="sm" onclick={clearCompleted}>Clear completed</Button>
      {/if}
      {#if $errorCount > 0}
        <Button variant="ghost" size="sm" onclick={clearErrors}>Clear errors</Button>
      {/if}
    </div>
  </header>

  {#if $downloadItems.length === 0}
    <EmptyState
      icon="download"
      heading="No downloads yet"
      description="Paste an Anyflip URL in the New tab to get started."
      actionLabel="Start a download"
      onAction={goStart}
    />
  {:else}
    <RaisedCard padding="none" class="list">
      <ul class="rows">
        {#each $downloadItems as item (item.id)}
          {@const tok = tokenFor(item.status)}
          {@const active = isActive(item)}
          {@const showProgress = isProgressVisible(item)}
          <li class="row" class:row--active={$activeDownload?.id === item.id} class:row--complete={item.status === 'complete'}>
            <div class="row-art" aria-hidden="true">
              {#if item.thumbnail && !thumbFailed.has(item.id)}
                <img
                  class="row-thumb"
                  src={item.thumbnail}
                  alt=""
                  loading="lazy"
                  onerror={() => thumbFailed.add(item.id)}
                />
              {:else}
                <Icon name="book-open" size={20} />
              {/if}
            </div>
            <div class="row-main">
              <div class="row-top">
                <div class="title-block">
                  {#if editingId === item.id && item.status === 'queued'}
                    <input
                      class="rename-input"
                      type="text"
                      bind:value={editTitle}
                      bind:this={renameInput}
                      onblur={() => commitRename(item.id)}
                      onkeydown={(e) => {
                        if (e.key === 'Enter') commitRename(item.id);
                        if (e.key === 'Escape') cancelRename();
                      }}
                      aria-label="Edit title"
                    />
                  {:else}
                    <button
                      type="button"
                      class="title"
                      class:title--editable={item.status === 'queued'}
                      ondblclick={() => startRename(item)}
                      title={item.status === 'queued' ? 'Double-click to rename' : item.title}
                    >
                      {item.title}
                    </button>
                  {/if}
                  <div class="meta">
                    <span>{item.pageCount} {item.pageCount === 1 ? 'page' : 'pages'}</span>
                    <span class="sep">·</span>
                    <span class="format-pill">{item.format.toUpperCase()}</span>
                    {#if item.passwordProtected}
                      <span class="lock-badge" title="Password protected" aria-label="Password protected">
                        <Icon name="lock" size={12} />
                      </span>
                    {/if}
                    {#if item.date && formatDate(item.date)}
                      <span class="sep">·</span>
                      <span>{formatDate(item.date)}</span>
                    {/if}
                    {#if item.estimatedFormatted}
                      <span class="sep">·</span>
                      <span>~{item.estimatedFormatted}</span>
                    {/if}
                    {#if showProgress && item.totalPages > 0}
                      <span class="sep">·</span>
                      <span>Page {item.currentPage}/{item.totalPages}</span>
                    {/if}
                  </div>
                </div>
                <StatusToken variant={tok.variant} label={tok.label} />
              </div>

              {#if showProgress}
                <div class="progress-row">
                  <ProgressBar
                    progress={item.progress}
                    currentPage={item.currentPage}
                    totalPages={item.totalPages}
                    size="xs"
                  />
                  <span class="progress-num">{Math.round(item.progress)}%</span>
                </div>
              {/if}

              {#if item.status === 'error' && item.error}
                <ErrorDisplay
                  message={item.error}
                  code={item.errorCode ?? undefined}
                  retryable={item.retryable}
                  onretry={() => retryItem(item.id)}
                  compact
                />
              {/if}
            </div>

            <div class="row-actions">
              {#if item.status === 'queued'}
                <SegmentedControl
                  value={item.format}
                  options={[
                    { value: 'pdf', label: 'PDF' },
                    { value: 'epub', label: 'EPUB' },
                  ]}
                  ariaLabel="Output format"
                  onChange={(v) => updateItemFormat(item.id, v as 'pdf' | 'epub')}
                />
              {/if}

              {#if item.status === 'error'}
                <Button
                  variant="ghost"
                  size="sm"
                  leadingIcon="refresh"
                  disabled={$isOffline}
                  onclick={() => retryItem(item.id)}
                >
                  Retry
                </Button>
              {/if}

              {#if $activeDownload?.id === item.id && (item.status === 'downloading' || item.status === 'converting' || item.status === 'saving')}
                <Button
                  variant="ghost"
                  size="sm"
                  leadingIcon="pause"
                  onclick={() => pauseDownload(item.id)}
                >
                  Pause
                </Button>
                <Button
                  variant="ghost"
                  size="sm"
                  leadingIcon="x"
                  onclick={cancelActiveDownload}
                >
                  Cancel
                </Button>
              {/if}

              {#if $activeDownload?.id === item.id && item.status === 'paused'}
                <Button
                  variant="primary"
                  size="sm"
                  leadingIcon="play"
                  onclick={() => resumeDownload(item.id)}
                >
                  Resume
                </Button>
                <Button
                  variant="ghost"
                  size="sm"
                  leadingIcon="x"
                  onclick={cancelActiveDownload}
                >
                  Cancel
                </Button>
              {/if}

              {#if !active}
                <button
                  type="button"
                  class="row-icon"
                  onclick={() => handleRemove(item)}
                  aria-label="Remove from queue"
                  title="Remove"
                >
                  <Icon name="trash" size={16} />
                </button>
              {/if}
            </div>
          </li>
        {/each}

        {#each [...removedItems.entries()] as [id, { item }] (id)}
          <li class="row row--removed">
            <div class="removed-row">
              <span class="removed-label">Removed</span>
              <span class="removed-title" title={item.title}>{item.title}</span>
              <Button variant="ghost" size="sm" onclick={() => undoRemove(id)}>Undo</Button>
            </div>
          </li>
        {/each}
      </ul>
    </RaisedCard>
  {/if}
</div>

<style>
  .screen {
    display: flex;
    flex-direction: column;
    gap: var(--space-5);
    max-width: 800px;
    margin: 0 auto;
    width: 100%;
  }

  .page-header {
    display: flex;
    align-items: center;
    gap: var(--space-3);
    flex-wrap: wrap;
  }

  h1 {
    margin: 0;
    font-size: var(--font-size-2xl);
    line-height: var(--line-height-2xl);
    font-weight: var(--font-weight-semibold);
    color: var(--color-text-primary);
    letter-spacing: var(--letter-tracking-tight);
  }

  .stats {
    display: inline-flex;
    align-items: center;
    gap: var(--space-2);
    flex-wrap: wrap;
  }

  .header-actions {
    display: inline-flex;
    align-items: center;
    gap: var(--space-2);
    margin-left: auto;
  }

  .rows {
    list-style: none;
    margin: 0;
    padding: 0;
  }

  .row {
    display: flex;
    align-items: flex-start;
    gap: var(--space-3);
    padding: var(--space-4) var(--space-5);
    border-top: 1px solid var(--color-border);
    transition: background-color var(--duration-fast) var(--ease-standard);
  }

  .row:first-child { border-top: none; }
  .row:hover { background: var(--color-surface-secondary); }

  .row--active { background: var(--color-primary-50); }
  .row--active:hover { background: var(--color-primary-50); }
  .row--complete { opacity: 0.72; }
  .row--removed {
    background: var(--color-surface-secondary);
    padding: var(--space-3) var(--space-5);
  }

  .row-art {
    width: 44px;
    height: 56px;
    border-radius: var(--radius-control);
    display: inline-flex;
    align-items: center;
    justify-content: center;
    background: var(--color-primary-50);
    color: var(--color-primary-700);
    overflow: hidden;
    flex-shrink: 0;
  }

  .row-thumb {
    width: 100%;
    height: 100%;
    object-fit: cover;
    display: block;
  }

  .row-main {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
    gap: var(--space-2);
  }

  .row-top {
    display: flex;
    align-items: flex-start;
    justify-content: space-between;
    gap: var(--space-3);
  }

  .title-block { min-width: 0; flex: 1; }

  .title {
    appearance: none;
    background: transparent;
    border: none;
    padding: 0;
    margin: 0;
    text-align: left;
    font: inherit;
    font-weight: var(--font-weight-semibold);
    font-size: var(--font-size-base);
    line-height: var(--line-height-base);
    color: var(--color-text-primary);
    display: block;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    width: 100%;
    cursor: default;
  }
  .title--editable { cursor: text; }
  .title--editable:hover { color: var(--color-primary-700); }
  .title:focus-visible { outline: var(--focus-ring); outline-offset: 2px; border-radius: 4px; }

  .rename-input {
    width: 100%;
    font: inherit;
    font-weight: var(--font-weight-semibold);
    color: var(--color-text-primary);
    background: var(--color-surface-input);
    border: 1px solid var(--color-primary-500);
    border-radius: var(--radius-control);
    padding: var(--space-1) var(--space-3);
    height: 32px;
  }
  .rename-input:focus { outline: var(--focus-ring); outline-offset: 0; }

  .meta {
    display: flex;
    align-items: center;
    gap: 6px;
    flex-wrap: wrap;
    margin-top: 2px;
    font-size: var(--font-size-xs);
    line-height: var(--line-height-xs);
    color: var(--color-text-muted);
  }
  .meta .sep { color: var(--color-text-muted); }
  .meta .format-pill {
    display: inline-flex;
    align-items: center;
    padding: 1px 6px;
    background: var(--color-surface-secondary);
    border: 1px solid var(--color-border);
    border-radius: var(--radius-sm);
    font-weight: var(--font-weight-semibold);
    color: var(--color-text-secondary);
  }
  .meta .lock-badge {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    width: 16px;
    height: 16px;
    border-radius: 999px;
    background: var(--status-paused-bg);
    color: var(--status-paused-text);
  }

  .progress-row {
    display: flex;
    align-items: center;
    gap: var(--space-3);
  }

  .progress-num {
    font-size: var(--font-size-xs);
    line-height: var(--line-height-xs);
    font-weight: var(--font-weight-semibold);
    color: var(--color-text-secondary);
    font-variant-numeric: tabular-nums;
    flex-shrink: 0;
  }

  .row-actions {
    display: flex;
    align-items: center;
    gap: var(--space-2);
    flex-shrink: 0;
  }

  .row-icon {
    appearance: none;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    width: 32px;
    height: 32px;
    border: none;
    background: transparent;
    color: var(--color-text-muted);
    border-radius: var(--radius-sm);
    cursor: pointer;
    opacity: 0;
    transition:
      background-color var(--duration-fast) var(--ease-standard),
      color var(--duration-fast) var(--ease-standard),
      opacity var(--duration-fast) var(--ease-standard);
  }
  .row:hover .row-icon,
  .row:focus-within .row-icon { opacity: 1; }
  .row-icon:hover { background: var(--color-neutral-100); color: var(--color-error); }
  .row-icon:focus-visible { outline: var(--focus-ring); outline-offset: 1px; opacity: 1; }

  .removed-row {
    display: flex;
    align-items: center;
    gap: var(--space-3);
    width: 100%;
  }
  .removed-label {
    font-size: var(--font-size-xs);
    line-height: var(--line-height-xs);
    font-weight: var(--font-weight-semibold);
    color: var(--color-text-muted);
    text-transform: uppercase;
    letter-spacing: var(--letter-tracking-wide);
  }
  .removed-title {
    flex: 1;
    min-width: 0;
    font-size: var(--font-size-sm);
    line-height: var(--line-height-sm);
    color: var(--color-text-secondary);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  @media (prefers-reduced-motion: reduce) {
    .row, .row-icon { transition: none; }
  }
</style>
