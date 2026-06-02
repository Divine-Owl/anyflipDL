<script lang="ts">
  import {
    historyItems,
    historyCount,
    removeHistoryItem,
    renameHistoryItem,
    clearHistory,
    openFile,
    openContainingFolder,
  } from '$lib/stores/history';
  import type { HistoryItem } from '$lib/types';
  import { formatDate } from '$lib/utils/date';
  import RaisedCard from '$lib/components/RaisedCard.svelte';
  import StatusToken from '$lib/components/StatusToken.svelte';
  import Button from '$lib/components/Button.svelte';
  import Tooltip from '$lib/components/Tooltip.svelte';
  import EmptyState from '$lib/components/EmptyState.svelte';
  import Icon from '$lib/components/icons/Icon.svelte';

  let confirmClear = $state(false);
  let confirmTimer: ReturnType<typeof setTimeout> | null = null;

  const removedItems = $state(
    new Map<string, { item: HistoryItem; timeoutId: ReturnType<typeof setTimeout> }>(),
  );

  // Per-item thumbnail failure tracking.
  let thumbFailed = $state(new Set<string>());

  let editingId = $state<string | null>(null);
  let editTitle = $state('');
  let renameInput: HTMLInputElement | undefined = $state();
  let renameError = $state<string | null>(null);

  function startRename(item: HistoryItem) {
    editingId = item.id;
    editTitle = item.title;
    renameError = null;
    requestAnimationFrame(() => renameInput?.select());
  }

  async function commitRename(id: string) {
    const trimmed = editTitle.trim();
    if (!trimmed) {
      editingId = null;
      return;
    }
    const item = $historyItems.find((i) => i.id === id);
    if (!item || trimmed === item.title) {
      editingId = null;
      return;
    }
    try {
      await renameHistoryItem(id, trimmed);
      editingId = null;
      renameError = null;
    } catch (err) {
      renameError = err instanceof Error ? err.message : 'Rename failed';
    }
  }

  function cancelRename() {
    editingId = null;
    renameError = null;
  }

  function handleRemove(item: HistoryItem) {
    const existing = removedItems.get(item.id);
    if (existing) {
      clearTimeout(existing.timeoutId);
      removedItems.delete(item.id);
      removeHistoryItem(item.id);
      return;
    }
    removedItems.set(item.id, {
      item: { ...item },
      timeoutId: setTimeout(() => {
        removedItems.delete(item.id);
        removeHistoryItem(item.id);
      }, 3000),
    });
    removeHistoryItem(item.id);
  }

  function undoRemove(id: string) {
    const entry = removedItems.get(id);
    if (!entry) return;
    clearTimeout(entry.timeoutId);
    removedItems.delete(id);
    historyItems.update((items) => [...items, entry.item]);
  }

  function formatFileSize(bytes: number): string {
    if (bytes <= 0) return 'Unknown';
    if (bytes < 1024) return `${bytes} B`;
    if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
    return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
  }

  function formatCompletedAt(iso: string): string {
    try {
      const d = new Date(iso);
      return d.toLocaleDateString(undefined, {
        year: 'numeric',
        month: 'short',
        day: 'numeric',
        hour: '2-digit',
        minute: '2-digit',
      });
    } catch {
      return iso;
    }
  }

  async function handleOpenFile(item: HistoryItem) {
    try {
      await openFile(item.outputPath);
    } catch {
      // Silently fail — file may have been moved/deleted.
    }
  }

  async function handleOpenFolder(item: HistoryItem) {
    try {
      await openContainingFolder(item.outputPath);
    } catch {
      // Silently fail.
    }
  }

  function handleClear() {
    if (confirmClear) {
      clearHistory();
      confirmClear = false;
      if (confirmTimer) {
        clearTimeout(confirmTimer);
        confirmTimer = null;
      }
    } else {
      confirmClear = true;
      confirmTimer = setTimeout(() => {
        confirmClear = false;
        confirmTimer = null;
      }, 3000);
    }
  }
</script>

<div class="screen">
  <header class="page-header">
    <h1>History</h1>
    <div class="stats">
      {#if $historyCount > 0}
        <StatusToken variant="neutral" label="{$historyCount} {$historyCount === 1 ? 'item' : 'items'}" />
      {/if}
    </div>
    <div class="header-actions">
      {#if $historyCount > 0}
        <Button
          variant={confirmClear ? 'danger' : 'ghost'}
          size="sm"
          onclick={handleClear}
        >
          {confirmClear ? 'Confirm clear' : 'Clear history'}
        </Button>
      {/if}
    </div>
  </header>

  {#if $historyItems.length === 0}
    <EmptyState
      icon="archive"
      heading="Nothing downloaded yet"
      description="Completed downloads will appear here."
    />
  {:else}
    <RaisedCard padding="none" class="list">
      <ul class="rows">
        {#each $historyItems as item (item.id)}
          <li class="row">
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
                  {#if editingId === item.id}
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
                    <Tooltip label={item.title} placement="top">
                      <button
                        type="button"
                        class="title"
                        ondblclick={() => startRename(item)}
                        title="Double-click to rename"
                      >
                        {item.title}
                      </button>
                    </Tooltip>
                  {/if}
                  {#if editingId === item.id && renameError}
                    <div class="rename-error">{renameError}</div>
                  {/if}
                  <div class="meta">
                    <span>{item.pageCount} {item.pageCount === 1 ? 'page' : 'pages'}</span>
                    <span class="sep">·</span>
                    <span>{formatFileSize(item.fileSize)}</span>
                    {#if item.passwordProtected}
                      <span class="lock-badge" title="Password protected" aria-label="Password protected">
                        <Icon name="lock" size={12} />
                      </span>
                    {/if}
                    {#if item.date && formatDate(item.date)}
                      <span class="sep">·</span>
                      <span>{formatDate(item.date)}</span>
                    {/if}
                    <span class="sep">·</span>
                    <span>{formatCompletedAt(item.completedAt)}</span>
                  </div>
                </div>
                <StatusToken variant="active" label={item.format.toUpperCase()} compact />
              </div>
            </div>

            <div class="row-actions">
              <Button variant="ghost" size="sm" leadingIcon="external-link" onclick={() => handleOpenFile(item)}>
                Open
              </Button>
              <Button variant="ghost" size="sm" leadingIcon="folder-open" onclick={() => handleOpenFolder(item)}>
                Show in folder
              </Button>
              <button
                type="button"
                class="row-icon"
                onclick={() => handleRemove(item)}
                aria-label="Remove from history"
                title="Remove"
              >
                <Icon name="trash" size={16} />
              </button>
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

  .stats { display: inline-flex; align-items: center; gap: var(--space-2); }
  .header-actions { display: inline-flex; align-items: center; gap: var(--space-2); margin-left: auto; }

  .rows { list-style: none; margin: 0; padding: 0; }

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
  .row--removed { background: var(--color-surface-secondary); padding: var(--space-3) var(--space-5); }

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

  .row-main { flex: 1; min-width: 0; }

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
    max-width: 100%;
    cursor: text;
  }
  .title:hover { color: var(--color-primary-700); }
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
  .rename-error { margin-top: 2px; font-size: var(--font-size-xs); color: var(--color-error); }

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
