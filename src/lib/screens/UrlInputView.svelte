<script lang="ts">
  import { onMount } from 'svelte';
  import { tauriInvoke } from '$lib/ipc/client';
  import { fetchDocumentMetadata } from '$lib/ipc/metadata';
  import { addToQueue } from '$lib/stores/downloads';
  import { defaultFormat } from '$lib/stores/settings';
  import { isOffline } from '$lib/stores/network';
  import { showToast } from '$lib/stores/toasts';
  import type { DownloadFormat } from '$lib/types';
  import type { DocumentMetadata } from '$lib/ipc/client';
  import { formatDate } from '$lib/utils/date';
  import RaisedCard from '$lib/components/RaisedCard.svelte';
  import Input from '$lib/components/Input.svelte';
  import Button from '$lib/components/Button.svelte';
  import SegmentedControl from '$lib/components/SegmentedControl.svelte';
  import ErrorDisplay from '$lib/components/ErrorDisplay.svelte';
  import Icon from '$lib/components/icons/Icon.svelte';
  import Tooltip from '$lib/components/Tooltip.svelte';

  type State = 'idle' | 'validating' | 'fetching' | 'ready' | 'adding' | 'error';

  let viewState = $state<State>('idle');
  let url = $state('');
  let errorMessage = $state('');
  let errorCode = $state('');
  let isRetryable = $state(false);
  let metadata = $state<DocumentMetadata | null>(null);
  let customTitle = $state('');
  let currentFormat = $state<DownloadFormat>('pdf');
  let inputWrap: HTMLDivElement | undefined = $state();
  let thumbnailFailed = $state(false);

  const isBusy = $derived(
    viewState === 'validating' || viewState === 'fetching' || viewState === 'adding',
  );
  const isReady = $derived(viewState === 'ready' && metadata !== null);
  const offlineDisabled = $derived($isOffline);
  const canSubmit = $derived(!!url.trim() && !isBusy && !offlineDisabled);
  const formattedDate = $derived(metadata ? formatDate(metadata.date) : null);
  const showThumbnail = $derived(
    !!metadata?.thumbnail && !thumbnailFailed,
  );

  $effect(() => {
    currentFormat = $defaultFormat;
  });

  function focusInput() {
    const el = inputWrap?.querySelector<HTMLInputElement>('input');
    el?.focus();
  }

  onMount(focusInput);

  function reset() {
    viewState = 'idle';
    errorMessage = '';
    errorCode = '';
    isRetryable = false;
    metadata = null;
    customTitle = '';
  }

  async function handleSubmit() {
    if (!url.trim() || isBusy) return;
    reset();
    viewState = 'validating';
    try {
      const isValid = await tauriInvoke<boolean>('validate_url', { url: url.trim() });
      if (!isValid) {
        viewState = 'error';
        errorMessage =
          'Not a valid Anyflip URL. Expected: https://anyflip.com/user/book';
        errorCode = 'VALIDATION_ERROR';
        isRetryable = false;
        return;
      }
    } catch (e) {
      viewState = 'error';
      errorMessage =
        (e instanceof Error && e.message) ||
        'Invalid URL. Expected: https://anyflip.com/user/book';
      errorCode = 'VALIDATION_ERROR';
      isRetryable = false;
      return;
    }
    viewState = 'fetching';
    try {
      const result = await fetchDocumentMetadata(url.trim());
      metadata = result;
      customTitle = result.title;
      thumbnailFailed = false;
      viewState = 'ready';
    } catch (e) {
      viewState = 'error';
      errorMessage =
        (e instanceof Error && e.message) ||
        "Couldn't reach Anyflip. Check your connection and try again.";
      errorCode = 'METADATA_ERROR';
      isRetryable = true;
    }
  }

  async function handleAddToQueue() {
    if (!metadata || !url.trim()) return;
    viewState = 'adding';
    try {
      await addToQueue(url.trim(), currentFormat, metadata, customTitle);
      const title = customTitle.trim() || metadata.title;
      showToast({
        variant: 'success',
        title: 'Added to queue',
        detail: `"${title}" will download in the background.`,
      });
      reset();
      url = '';
      focusInput();
    } catch (e) {
      viewState = 'error';
      errorMessage =
        (e instanceof Error && e.message) ||
        "Couldn't add to queue. Try again or restart the app.";
      errorCode = 'QUEUE_ERROR';
      isRetryable = false;
    }
  }

  function handleKeydown(e: KeyboardEvent) {
    if (e.key === 'Enter' && !isBusy) {
      if (isReady) handleAddToQueue();
      else handleSubmit();
    }
  }

  function handlePaste(e: ClipboardEvent) {
    const text = e.clipboardData?.getData('text') ?? '';
    if (text.includes('anyflip.com')) {
      url = text.trim();
      handleSubmit();
    }
  }

  function clearUrl() {
    url = '';
    reset();
    focusInput();
  }
</script>

<div class="screen">
  <RaisedCard padding="lg" class="hero">
    {#if !isReady}
      <header class="hero-header">
        <h1>Download a document</h1>
        <p class="subhead">
          Paste an Anyflip URL to fetch metadata and prepare the document for offline use.
        </p>
      </header>

      <div class="field" bind:this={inputWrap}>
        <Input
          bind:value={url}
          type="url"
          placeholder="https://anyflip.com/user/book"
          mono
          disabled={isBusy}
          ariaLabel="Anyflip URL"
          trailingIcon={url ? 'x' : 'link'}
          onTrailingClick={url ? clearUrl : undefined}
          onKeydown={handleKeydown}
          onPaste={handlePaste}
        />
      </div>

      <div class="field">
        <span class="field-label">Output format</span>
        <Tooltip label="Choose the file type for the downloaded document">
          <SegmentedControl
            bind:value={currentFormat}
            options={[
              { value: 'pdf', label: 'PDF', icon: 'file-pdf' },
              { value: 'epub', label: 'EPUB', icon: 'book-open' },
            ]}
            ariaLabel="Output format"
            disabled={isBusy}
          />
        </Tooltip>
      </div>

      <div class="actions">
        <Tooltip label="Fetch metadata for the URL above (Enter)">
          <Button
            variant="primary"
            size="lg"
            full
            loading={isBusy}
            disabled={!canSubmit}
            trailingIcon="arrow-right"
            onclick={handleSubmit}
          >
            Fetch document
          </Button>
        </Tooltip>
      </div>
    {:else if metadata}
      <div class="ready">
        <div class="ready-art" aria-hidden="true">
          {#if showThumbnail}
            <img
              class="ready-thumb"
              src={metadata.thumbnail}
              alt=""
              loading="lazy"
              onerror={() => (thumbnailFailed = true)}
            />
          {:else}
            <Icon name="book-open" size={28} />
          {/if}
        </div>
        <div class="ready-body">
          <div class="ready-title" title={metadata.title}>{metadata.title}</div>
          <div class="ready-meta">
            <span>{metadata.pageCount} {metadata.pageCount === 1 ? 'page' : 'pages'}</span>
            <span class="dot">·</span>
            <span class="format-pill">{currentFormat.toUpperCase()}</span>
            {#if metadata.passwordProtected}
              <span class="lock-badge" title="Password protected" aria-label="Password protected">
                <Icon name="lock" size={12} />
              </span>
            {/if}
            {#if formattedDate}
              <span class="dot">·</span>
              <span>{formattedDate}</span>
            {/if}
          </div>
        </div>
        <div class="ready-actions">
          <Button variant="ghost" size="sm" onclick={reset}>Back</Button>
          <Button
            variant="primary"
            size="md"
            loading={viewState === 'adding'}
            disabled={offlineDisabled}
            onclick={handleAddToQueue}
          >
            Add to queue
          </Button>
        </div>
      </div>
      <div class="filename-row">
        <label class="filename-label" for="filename-input">File name</label>
        <div class="filename-input-wrap">
          <input
            id="filename-input"
            type="text"
            class="filename-input"
            bind:value={customTitle}
            placeholder={metadata.title}
            aria-label="Output file name"
          />
          <span class="filename-ext">.{currentFormat}</span>
        </div>
      </div>
    {/if}
  </RaisedCard>

  {#if viewState === 'error' && errorMessage}
    <ErrorDisplay
      message={errorMessage}
      code={errorCode}
      retryable={isRetryable}
      onretry={isRetryable ? handleSubmit : undefined}
    />
  {/if}

  <p class="support-line">
    Supports <span class="mono">anyflip.com/user/book</span> and
    <span class="mono">online.anyflip.com/user/book</span> URLs.
  </p>
</div>

<style>
  .screen {
    display: flex;
    flex-direction: column;
    gap: var(--space-5);
    max-width: 560px;
    margin: var(--space-5) auto 0;
    width: 100%;
  }

  .hero-header {
    display: flex;
    flex-direction: column;
    gap: var(--space-2);
    margin-bottom: var(--space-5);
  }

  h1 {
    margin: 0;
    font-size: var(--font-size-2xl);
    line-height: var(--line-height-2xl);
    font-weight: var(--font-weight-semibold);
    color: var(--color-text-primary);
    letter-spacing: var(--letter-tracking-tight);
  }

  .subhead {
    margin: 0;
    font-size: var(--font-size-base);
    line-height: var(--line-height-base);
    color: var(--color-text-secondary);
  }

  .field {
    display: flex;
    flex-direction: column;
    gap: var(--space-2);
    margin-bottom: var(--space-4);
  }

  .field-label {
    font-size: var(--font-size-xs);
    line-height: var(--line-height-xs);
    font-weight: var(--font-weight-semibold);
    color: var(--color-text-muted);
    text-transform: uppercase;
    letter-spacing: var(--letter-tracking-wide);
  }

  .actions { margin-top: var(--space-2); }

  .ready {
    display: flex;
    flex-direction: column;
    gap: var(--space-4);
  }

  .ready-art {
    width: 56px;
    height: 56px;
    border-radius: var(--radius-card);
    display: inline-flex;
    align-items: center;
    justify-content: center;
    background: var(--color-primary-50);
    color: var(--color-primary-700);
    overflow: hidden;
    flex-shrink: 0;
  }

  .ready-thumb {
    width: 100%;
    height: 100%;
    object-fit: cover;
    display: block;
  }

  .ready-body { display: flex; flex-direction: column; gap: var(--space-1); }

  .ready-title {
    font-size: var(--font-size-lg);
    line-height: var(--line-height-lg);
    font-weight: var(--font-weight-semibold);
    color: var(--color-text-primary);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .ready-meta {
    display: inline-flex;
    align-items: center;
    flex-wrap: wrap;
    gap: var(--space-1);
    font-size: var(--font-size-sm);
    line-height: var(--line-height-sm);
    color: var(--color-text-secondary);
  }

  .ready-meta .dot { margin: 0 2px; color: var(--color-text-muted); }

  .format-pill {
    display: inline-flex;
    align-items: center;
    padding: 1px 6px;
    border-radius: var(--radius-control);
    background: var(--color-primary-50);
    color: var(--color-primary-700);
    font-size: var(--font-size-xs);
    line-height: var(--line-height-xs);
    font-weight: var(--font-weight-semibold);
    letter-spacing: var(--letter-tracking-wide);
  }

  .lock-badge {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    width: 16px;
    height: 16px;
    border-radius: 999px;
    background: var(--status-paused-bg);
    color: var(--status-paused-text);
  }

  .ready-actions {
    display: flex;
    gap: var(--space-2);
    justify-content: flex-end;
  }

  .filename-row {
    display: flex;
    flex-direction: column;
    gap: var(--space-2);
  }

  .filename-label {
    font-size: var(--font-size-xs);
    line-height: var(--line-height-xs);
    font-weight: var(--font-weight-semibold);
    color: var(--color-text-muted);
    text-transform: uppercase;
    letter-spacing: var(--letter-tracking-wide);
  }

  .filename-input-wrap {
    display: flex;
    align-items: stretch;
  }

  .filename-input {
    flex: 1;
    min-width: 0;
    height: 42px;
    padding: 0 var(--space-3);
    border: 1px solid var(--color-border);
    border-right: none;
    border-radius: var(--radius-control) 0 0 var(--radius-control);
    background: var(--color-surface-input);
    color: var(--color-text-primary);
    font-family: inherit;
    font-size: var(--font-size-base);
    line-height: var(--line-height-base);
    transition: border-color var(--duration-fast) var(--ease-standard);
  }

  .filename-input:focus {
    outline: var(--focus-ring);
    outline-offset: 0;
    border-color: var(--color-primary-500);
  }

  .filename-ext {
    display: inline-flex;
    align-items: center;
    padding: 0 var(--space-3);
    border: 1px solid var(--color-border);
    border-radius: 0 var(--radius-control) var(--radius-control) 0;
    background: var(--color-surface-secondary);
    color: var(--color-text-muted);
    font-size: var(--font-size-base);
    line-height: var(--line-height-base);
    font-weight: var(--font-weight-medium);
    user-select: none;
  }

  .support-line {
    margin: 0;
    text-align: center;
    font-size: var(--font-size-sm);
    line-height: var(--line-height-sm);
    color: var(--color-text-muted);
  }

  .mono {
    font-family: var(--font-family-mono);
    font-size: var(--font-size-xs);
  }
</style>
