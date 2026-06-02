<script lang="ts">
  import type { Snippet } from 'svelte';
  import { onMount } from 'svelte';
  import UrlInputView from '$lib/screens/UrlInputView.svelte';
  import DownloadQueueView from '$lib/screens/DownloadQueueView.svelte';
  import HistoryView from '$lib/screens/HistoryView.svelte';
  import SettingsPanel from '$lib/screens/SettingsPanel.svelte';
  import NetworkStatus from '$lib/components/NetworkStatus.svelte';
  import ErrorBoundary from '$lib/components/ErrorBoundary.svelte';
  import PasswordDialog from '$lib/components/PasswordDialog.svelte';
  import Toaster from '$lib/components/Toaster.svelte';
  import ShortcutOverlay from '$lib/components/ShortcutOverlay.svelte';
  import HelpOverlay from '$lib/components/HelpOverlay.svelte';
  import Tooltip from '$lib/components/Tooltip.svelte';
  import Icon from '$lib/components/icons/Icon.svelte';
  import { activeView, navigateTo, toggleSettings } from '$lib/stores/ui';
  import { downloadItems } from '$lib/stores/downloads';
  import { theme } from '$lib/stores/settings';
  import { overlayOpen } from '$lib/stores/keyboard';
  import { helpOpen, toggleHelp } from '$lib/stores/help';
  import { base } from '$app/paths';

  interface Props {
    children?: Snippet;
  }

  let { children }: Props = $props();

  let isMaximized = $state(false);

  onMount(async () => {
    const { getCurrentWindow } = await import('@tauri-apps/api/window');
    try {
      isMaximized = await getCurrentWindow().isMaximized();
    } catch {
      isMaximized = false;
    }
  });

  async function close() {
    const { getCurrentWindow } = await import('@tauri-apps/api/window');
    getCurrentWindow().close();
  }
  async function minimize() {
    const { getCurrentWindow } = await import('@tauri-apps/api/window');
    getCurrentWindow().minimize();
  }
  async function toggleMax() {
    const { getCurrentWindow } = await import('@tauri-apps/api/window');
    const w = getCurrentWindow();
    if (isMaximized) await w.unmaximize(); else await w.maximize();
    isMaximized = !isMaximized;
  }

  // Theme: light / dark / system
  $effect(() => {
    const root = document.documentElement;
    const t = $theme;
    if (t === 'system') {
      const mq = window.matchMedia('(prefers-color-scheme: dark)');
      const apply = () => root.setAttribute('data-theme', mq.matches ? 'dark' : 'light');
      apply();
      mq.addEventListener('change', apply);
      return () => mq.removeEventListener('change', apply);
    }
    root.setAttribute('data-theme', t);
  });

  // Queue badge variant: highest-severity in-progress item
  const queueBadge = $derived.by(() => {
    let hasError = false;
    let hasPaused = false;
    let hasActive = false;
    for (const it of $downloadItems) {
      if (it.status === 'downloading' || it.status === 'converting' || it.status === 'saving') hasActive = true;
      if (it.status === 'paused') hasPaused = true;
      if (it.status === 'error') hasError = true;
    }
    if (hasError) return 'error';
    if (hasActive) return 'active';
    if (hasPaused) return 'paused';
    return 'neutral';
  });
</script>

<a class="skip" href="#main-content">Skip to main content</a>

<div class="shell">
  <header class="titlebar" data-tauri-drag-region>
    <div class="left">
      <img class="logo" src="{base}/app-icon.png" alt="" />
      <span class="appname">AnyflipDL</span>
    </div>

    <div class="tabs" role="tablist" aria-label="Navigation">
      <Tooltip label="New download (1)">
        <button
          class="tab"
          class:tab--active={$activeView === 'url'}
          role="tab"
          aria-selected={$activeView === 'url'}
          onclick={() => navigateTo('url')}
          data-tauri-drag-region="false"
        >
          <Icon name="add-box" size={16} />
          <span>New</span>
        </button>
      </Tooltip>
      <Tooltip label="Download queue (2)">
        <button
          class="tab"
          class:tab--active={$activeView === 'downloads'}
          role="tab"
          aria-selected={$activeView === 'downloads'}
          onclick={() => navigateTo('downloads')}
          data-tauri-drag-region="false"
        >
          <Icon name="view-list" size={16} />
          <span>Queue</span>
          {#if queueBadge !== 'neutral'}
            <span class="dot dot--{queueBadge}" aria-label="Activity indicator"></span>
          {/if}
        </button>
      </Tooltip>
      <Tooltip label="History (3)">
        <button
          class="tab"
          class:tab--active={$activeView === 'history'}
          role="tab"
          aria-selected={$activeView === 'history'}
          onclick={() => navigateTo('history')}
          data-tauri-drag-region="false"
        >
          <Icon name="history" size={16} />
          <span>History</span>
        </button>
      </Tooltip>
      <Tooltip label="Settings (4)">
        <button
          class="tab"
          class:tab--active={$activeView === 'settings'}
          role="tab"
          aria-selected={$activeView === 'settings'}
          onclick={toggleSettings}
          data-tauri-drag-region="false"
        >
          <Icon name="settings" size={16} />
          <span>Settings</span>
        </button>
      </Tooltip>
    </div>

    <div class="right">
      <Tooltip label="Show help (?)">
        <button
          class="iconbtn"
          type="button"
          aria-label="Show help"
          onclick={toggleHelp}
          data-tauri-drag-region="false"
        >
          <Icon name="help" size={16} />
        </button>
      </Tooltip>
      <Tooltip label="Keyboard shortcuts (?)">
        <button
          class="iconbtn"
          type="button"
          aria-label="Keyboard shortcuts"
          onclick={() => overlayOpen.update((v) => !v)}
          data-tauri-drag-region="false"
        >
          <Icon name="keyboard" size={16} />
        </button>
      </Tooltip>
      <button class="iconbtn" type="button" aria-label="Minimize" onclick={minimize} data-tauri-drag-region="false">
        <Icon name="minus" size={16} />
      </button>
      <button class="iconbtn" type="button" aria-label={isMaximized ? 'Restore' : 'Maximize'} onclick={toggleMax} data-tauri-drag-region="false">
        <Icon name={isMaximized ? 'filter-none' : 'crop-square'} size={14} />
      </button>
      <button class="iconbtn iconbtn--close" type="button" aria-label="Close" onclick={close} data-tauri-drag-region="false">
        <Icon name="x" size={16} />
      </button>
    </div>
  </header>

  <NetworkStatus />

  <main id="main-content" class="content">
    {#if $activeView === 'url'}
      <ErrorBoundary><UrlInputView /></ErrorBoundary>
    {:else if $activeView === 'downloads'}
      <ErrorBoundary><DownloadQueueView /></ErrorBoundary>
    {:else if $activeView === 'history'}
      <ErrorBoundary><HistoryView /></ErrorBoundary>
    {:else if $activeView === 'settings'}
      <ErrorBoundary><SettingsPanel /></ErrorBoundary>
    {/if}
  </main>
</div>

<PasswordDialog />
<ShortcutOverlay />
<HelpOverlay />
<Toaster />

<style>
  .skip {
    position: absolute;
    width: 1px; height: 1px;
    padding: 0; margin: -1px;
    overflow: hidden; clip: rect(0,0,0,0);
    white-space: nowrap; border-width: 0;
  }
  .skip:focus {
    position: fixed;
    top: var(--space-2); left: var(--space-2);
    width: auto; height: auto;
    padding: var(--space-2) var(--space-4);
    margin: 0; overflow: visible; clip: auto;
    white-space: normal; z-index: 1000;
    background: var(--color-primary-500);
    color: white; border-radius: var(--radius-chip);
    text-decoration: none;
    outline: var(--focus-ring);
  }

  .shell {
    display: flex; flex-direction: column;
    height: 100vh; overflow: hidden;
    background: var(--color-background);
  }

  .titlebar {
    display: flex; align-items: center;
    height: 44px;
    padding: 0 var(--space-3);
    gap: var(--space-3);
    background: var(--color-surface-base);
    border-bottom: 1px solid var(--color-border);
    user-select: none;
    -webkit-user-select: none;
    flex-shrink: 0;
  }

  .left {
    display: flex; align-items: center; gap: var(--space-2);
    flex-shrink: 0;
    padding-left: var(--space-2);
  }

  .logo {
    display: inline-block;
    width: 28px; height: 28px;
    border-radius: 6px;
    user-select: none;
    -webkit-user-drag: none;
  }

  .appname {
    font-size: 14px;
    font-weight: var(--font-weight-semibold);
    color: var(--color-text-primary);
    letter-spacing: var(--letter-tracking-tight);
  }

  .tabs {
    display: flex;
    align-items: center;
    gap: 4px;
    flex: 1;
    justify-content: center;
    height: 100%;
  }

  .tab {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    height: 36px;
    padding: 0 var(--space-4);
    border: none;
    background: transparent;
    color: var(--color-text-secondary);
    font-family: inherit;
    font-size: var(--font-size-sm);
    font-weight: var(--font-weight-medium);
    line-height: 1;
    border-radius: var(--radius-control);
    cursor: pointer;
    transition:
      background-color var(--duration-fast) var(--ease-standard),
      color var(--duration-fast) var(--ease-standard);
    white-space: nowrap;
  }

  .tab:hover:not(.tab--active) { background: var(--color-neutral-100); }

  .tab--active {
    background: var(--color-primary-50);
    color: var(--color-primary-700);
    font-weight: var(--font-weight-semibold);
  }

  .tab:focus-visible {
    outline: var(--focus-ring);
    outline-offset: 2px;
  }

  .dot {
    width: 8px; height: 8px; border-radius: 50%;
    margin-left: 4px; flex-shrink: 0;
  }
  .dot--active  { background: var(--status-active-text); }
  .dot--paused  { background: var(--status-paused-text); }
  .dot--error   { background: var(--status-error-text); }
  .dot--neutral { background: var(--color-text-muted); }

  .right {
    display: flex; align-items: center; gap: 0;
    flex-shrink: 0;
    margin-left: auto;
  }

  .iconbtn {
    display: inline-flex;
    align-items: center; justify-content: center;
    width: 32px; height: 32px;
    margin: 0 2px;
    border: none; background: transparent;
    color: var(--color-text-muted);
    border-radius: var(--radius-sm);
    cursor: pointer;
    transition:
      background-color var(--duration-fast) var(--ease-standard),
      color var(--duration-fast) var(--ease-standard);
  }
  .iconbtn:hover { background: var(--color-neutral-100); color: var(--color-text-primary); }
  .iconbtn:focus-visible { outline: var(--focus-ring); outline-offset: 1px; }
  .iconbtn--close:hover { background: var(--color-error); color: #fff; }

  .content {
    flex: 1;
    display: flex;
    flex-direction: column;
    overflow-y: auto;
    padding: var(--space-6);
  }

  @media (prefers-reduced-motion: reduce) {
    .tab, .iconbtn { transition: none; }
  }
</style>
