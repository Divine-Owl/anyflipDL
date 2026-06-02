<script lang="ts">
  import { onMount } from 'svelte';
  import { get } from 'svelte/store';
  import LoadingSkeleton from '$lib/components/LoadingSkeleton.svelte';
  import ErrorDisplay from '$lib/components/ErrorDisplay.svelte';
  import CopyrightDisclaimer from '$lib/components/CopyrightDisclaimer.svelte';
  import RaisedCard from '$lib/components/RaisedCard.svelte';
  import Input from '$lib/components/Input.svelte';
  import Button from '$lib/components/Button.svelte';
  import SegmentedControl from '$lib/components/SegmentedControl.svelte';
  import Icon from '$lib/components/icons/Icon.svelte';
  import Tooltip from '$lib/components/Tooltip.svelte';
  import {
    saveLocation,
    defaultFormat,
    theme,
    soundNotification,
    fileNamingPattern,
    isLoading,
    settingsError,
    loadSettings,
    pickSaveLocation,
    updateSetting,
  } from '$lib/stores/settings';
  import { checkForUpdates, type UpdateCheckResult } from '$lib/ipc/client';

  type SettingsState = 'loading' | 'idle' | 'error';
  let settingsState: SettingsState = $state('loading');

  const isDisabled = $derived(settingsState === 'loading');

  type UpdateState = 'idle' | 'checking' | 'up-to-date' | 'update-available' | 'error';
  let updateState: UpdateState = $state('idle');
  let updateLatestVersion = $state('');
  let updateDownloadUrl = $state('');
  let updateErrorMessage = $state('');
  let currentVersion = $state('');

  async function init() {
    settingsState = 'loading';
    await loadSettings();
    const err = get(settingsError);
    settingsState = err ? 'error' : 'idle';
  }

  async function handleBrowse() {
    await pickSaveLocation();
  }

  function handleFormatChange(value: string) {
    updateSetting('defaultFormat', value as 'pdf' | 'epub');
  }

  function handleThemeChange(value: string) {
    updateSetting('theme', value as 'light' | 'dark' | 'system');
  }

  function handleSoundToggle() {
    updateSetting('soundNotification', !$soundNotification);
  }

  function handleNamingPatternInput(e: Event) {
    const target = e.currentTarget as HTMLInputElement;
    updateSetting('fileNamingPattern', target.value);
  }

  async function handleCheckForUpdates() {
    updateState = 'checking';
    updateErrorMessage = '';
    try {
      const result: UpdateCheckResult = await checkForUpdates();
      currentVersion = result.currentVersion;
      if (result.updateAvailable) {
        updateState = 'update-available';
        updateLatestVersion = result.latestVersion;
        updateDownloadUrl = result.downloadUrl ?? '';
      } else {
        updateState = 'up-to-date';
      }
    } catch (err) {
      updateState = 'error';
      updateErrorMessage = err instanceof Error ? err.message : 'Failed to check for updates';
    }
  }

  onMount(() => {
    init();
  });
</script>

<div class="screen">
  <header class="page-header">
    <h1>Settings</h1>
  </header>

  {#if settingsState === 'loading'}
    <LoadingSkeleton count={3} height="64px" gap="var(--space-4)" />
  {:else}
    <section class="section">
      <h2 class="section-title">Download</h2>
      <RaisedCard padding="md">
        <div class="field">
          <label class="field-label" for="save-location-input">Save location</label>
          <div class="row">
            <Input
              id="save-location-input"
              value={$saveLocation}
              placeholder="No folder selected"
              ariaLabel="Save location directory path"
              mono
              disabled={isDisabled}
              readonly
            />
            <Button variant="secondary" size="md" leadingIcon="folder-open" onclick={handleBrowse} disabled={isDisabled}>
              Browse
            </Button>
          </div>
        </div>

        <div class="field">
          <span class="field-label">Default format</span>
          <SegmentedControl
            value={$defaultFormat}
            options={[
              { value: 'pdf', label: 'PDF', icon: 'file-pdf' },
              { value: 'epub', label: 'EPUB', icon: 'book-open' },
            ]}
            ariaLabel="Default format"
            disabled={isDisabled}
            onChange={handleFormatChange}
          />
        </div>

        <div class="field">
          <label class="field-label" for="naming-pattern-input">File naming pattern</label>
          <Tooltip label={'Use tokens like {title} to build the saved file name'}>
            <Input
              id="naming-pattern-input"
              value={$fileNamingPattern}
              placeholder={'{title}'}
              ariaLabel="File naming pattern"
              mono
              disabled={isDisabled}
              onInput={handleNamingPatternInput}
            />
          </Tooltip>
          <p class="help-text">
            Available tokens: <code>{'{title}'}</code>, <code>{'{author}'}</code>,
            <code>{'{date}'}</code>, <code>{'{pageCount}'}</code>, <code>{'{format}'}</code>
          </p>
        </div>
      </RaisedCard>
    </section>

    <section class="section">
      <h2 class="section-title">Appearance</h2>
      <RaisedCard padding="md">
        <div class="field">
          <span class="field-label">Theme</span>
          <SegmentedControl
            value={$theme}
            options={[
              { value: 'light', label: 'Light', icon: 'sun' },
              { value: 'dark', label: 'Dark', icon: 'moon' },
              { value: 'system', label: 'System', icon: 'monitor' },
            ]}
            ariaLabel="Application theme"
            disabled={isDisabled}
            onChange={handleThemeChange}
          />
        </div>

        <div class="switch-row">
          <span class="field-label switch-label">Play sound on download complete</span>
          <button
            type="button"
            class="switch"
            class:switch--on={$soundNotification}
            role="switch"
            aria-checked={$soundNotification}
            disabled={isDisabled}
            onclick={handleSoundToggle}
            aria-label="Toggle sound notification on download complete"
          >
            <span class="knob" aria-hidden="true"></span>
          </button>
        </div>
      </RaisedCard>
    </section>

    <section class="section">
      <h2 class="section-title">About</h2>
      <RaisedCard padding="md">
        <p class="about-line">
          AnyflipDL <span class="version">{currentVersion ? `v${currentVersion}` : 'v1.0.0'}</span>
          <span class="sep">·</span>
          Download Anyflip documents as PDF or EPUB
        </p>

        <div class="update-check">
          <Button
            variant="secondary"
            size="md"
            loading={updateState === 'checking'}
            disabled={isDisabled}
            onclick={handleCheckForUpdates}
          >
            Check for updates
          </Button>

          {#if updateState === 'up-to-date'}
            <p class="update-status update-status--ok">
              <Icon name="check" size={14} /> You're up to date
            </p>
          {:else if updateState === 'update-available'}
            <p class="update-status update-status--available">
              <Icon name="info" size={14} />
              Update available: v{updateLatestVersion}
              {#if updateDownloadUrl}
                <a class="update-link" href={updateDownloadUrl} target="_blank" rel="noopener noreferrer">Download</a>
              {/if}
            </p>
          {:else if updateState === 'error'}
            <p class="update-status update-status--error">
              <Icon name="warning" size={14} /> {updateErrorMessage}
            </p>
          {/if}
        </div>

        <div class="disclaimer">
          <CopyrightDisclaimer variant="full" />
        </div>
      </RaisedCard>
    </section>

    {#if $settingsError}
      <ErrorDisplay message={$settingsError} variant="block" />
    {/if}

    <p class="compact-copyright">
      <CopyrightDisclaimer />
    </p>
  {/if}
</div>

<style>
  .screen {
    display: flex;
    flex-direction: column;
    gap: var(--space-5);
    max-width: 640px;
    margin: 0 auto;
    width: 100%;
  }

  .page-header { display: flex; align-items: center; gap: var(--space-3); }

  h1 {
    margin: 0;
    font-size: var(--font-size-2xl);
    line-height: var(--line-height-2xl);
    font-weight: var(--font-weight-semibold);
    color: var(--color-text-primary);
    letter-spacing: var(--letter-tracking-tight);
  }

  .section { display: flex; flex-direction: column; gap: var(--space-2); }

  .section-title {
    margin: 0 0 var(--space-1);
    padding-bottom: var(--space-2);
    border-bottom: 1px solid var(--color-border);
    font-size: var(--font-size-xs);
    line-height: var(--line-height-xs);
    font-weight: var(--font-weight-semibold);
    color: var(--color-text-muted);
    text-transform: uppercase;
    letter-spacing: var(--letter-tracking-wide);
  }

  .field {
    display: flex;
    flex-direction: column;
    gap: var(--space-2);
    margin-bottom: var(--space-4);
  }
  .field:last-child { margin-bottom: 0; }

  .field-label {
    font-size: var(--font-size-sm);
    line-height: var(--line-height-sm);
    font-weight: var(--font-weight-medium);
    color: var(--color-text-primary);
  }

  .row {
    display: flex;
    gap: var(--space-2);
    align-items: stretch;
  }

  .help-text {
    margin: 0;
    font-size: var(--font-size-xs);
    line-height: 1.55;
    color: var(--color-text-muted);
  }
  .help-text code {
    font-family: var(--font-family-mono);
    font-size: var(--font-size-xs);
    background: var(--color-surface-secondary);
    padding: 1px 4px;
    border-radius: var(--radius-sm);
  }

  .switch-row {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: var(--space-3);
    padding-top: var(--space-2);
  }
  .switch-label { margin: 0; }

  .switch {
    position: relative;
    width: 44px;
    height: 24px;
    background: var(--color-neutral-200);
    border: 1px solid var(--color-border);
    border-radius: var(--radius-pill);
    cursor: pointer;
    flex-shrink: 0;
    padding: 0;
    transition:
      background-color var(--duration-fast) var(--ease-standard),
      border-color var(--duration-fast) var(--ease-standard);
  }
  .switch--on {
    background: var(--color-primary-500);
    border-color: var(--color-primary-500);
  }
  .switch:disabled { opacity: 0.5; cursor: not-allowed; }
  .switch:focus-visible { outline: var(--focus-ring); outline-offset: 2px; }
  .knob {
    position: absolute;
    top: 2px;
    left: 2px;
    width: 18px;
    height: 18px;
    background: white;
    border-radius: 50%;
    box-shadow: var(--shadow-whisper);
    transition: transform var(--duration-fast) var(--ease-standard);
  }
  .switch--on .knob { transform: translateX(20px); }

  .about-line {
    margin: 0 0 var(--space-3);
    font-size: var(--font-size-sm);
    line-height: var(--line-height-sm);
    color: var(--color-text-secondary);
  }
  .about-line .version { color: var(--color-text-primary); font-weight: var(--font-weight-semibold); }
  .about-line .sep { margin: 0 6px; color: var(--color-text-muted); }

  .update-check {
    display: flex;
    flex-direction: column;
    gap: var(--space-2);
    margin-bottom: var(--space-3);
  }

  .update-status {
    display: inline-flex;
    align-items: center;
    gap: var(--space-2);
    margin: 0;
    font-size: var(--font-size-sm);
  }
  .update-status--ok { color: var(--color-success); }
  .update-status--available { color: var(--color-warning); }
  .update-status--error { color: var(--color-error); }
  .update-link {
    color: var(--color-primary-500);
    text-decoration: underline;
    margin-left: var(--space-2);
  }
  .update-link:hover { color: var(--color-primary-600); }

  .disclaimer { margin-top: var(--space-3); }

  .compact-copyright {
    margin: 0;
    text-align: center;
  }

  @media (prefers-reduced-motion: reduce) {
    .switch, .knob { transition: none; }
  }
</style>
