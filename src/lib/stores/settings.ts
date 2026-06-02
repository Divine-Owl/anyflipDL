/**
 * SettingsStore: persists and exposes application settings.
 *
 * Bridges the Rust config file with Svelte components. Loads settings on app
 * init via Tauri IPC and writes changes back to disk.
 *
 * @module $lib/stores/settings
 */

import { writable, derived, get, type Readable } from 'svelte/store';
import { invoke } from '@tauri-apps/api/core';
import type { Settings, DownloadFormat } from '$lib/types';

export const CURRENT_SETTINGS_VERSION = 3;

// ---------------------------------------------------------------------------
// Writable stores
// ---------------------------------------------------------------------------

/** User-configured directory for saving downloaded files. */
export const saveLocation = writable<string>('');

/** Default format for new downloads. */
export const defaultFormat = writable<DownloadFormat>('pdf');

/** UI theme preference. */
export const theme = writable<'light' | 'dark' | 'system'>('system');

/** Whether to enable file compression. */
export const compression = writable<boolean>(false);

/** Whether to play a sound notification when a download completes. */
export const soundNotification = writable<boolean>(true);

/** File naming pattern with token substitution (e.g., `{title}`, `{author}`). */
export const fileNamingPattern = writable<string>('{title}');

/** Whether the user has seen the onboarding help overlay. */
export const seenOnboarding = writable<boolean>(false);

/** `true` while settings are being loaded from disk. */
export const isLoading = writable<boolean>(true);

/** Last error from a settings operation, or `null`. */
export const settingsError = writable<string | null>(null);

// ---------------------------------------------------------------------------
// Derived stores
// ---------------------------------------------------------------------------

/** Combined settings object for passing to Tauri commands. */
export const currentSettings: Readable<Settings> = derived(
  [
    saveLocation,
    defaultFormat,
    theme,
    compression,
    soundNotification,
    fileNamingPattern,
    seenOnboarding,
  ],
  ([
    $saveLocation,
    $defaultFormat,
    $theme,
    $compression,
    $soundNotification,
    $fileNamingPattern,
    $seenOnboarding,
  ]) => ({
    version: CURRENT_SETTINGS_VERSION,
    saveLocation: $saveLocation,
    defaultFormat: $defaultFormat,
    theme: $theme,
    compression: $compression,
    soundNotification: $soundNotification,
    fileNamingPattern: $fileNamingPattern,
    seenOnboarding: $seenOnboarding,
  }),
);

// ---------------------------------------------------------------------------
// Internal helpers
// ---------------------------------------------------------------------------

/** Resolves `'system'` to `'light' | 'dark'` using the OS preference. */
function resolveSystemTheme(themeValue: 'light' | 'dark' | 'system'): 'light' | 'dark' {
  if (themeValue !== 'system') return themeValue;
  if (typeof window === 'undefined') return 'light';
  return window.matchMedia('(prefers-color-scheme: dark)').matches ? 'dark' : 'light';
}

/** Applies the `data-theme` attribute to `<html>` based on the current theme value. */
function applyTheme(themeValue: 'light' | 'dark' | 'system'): void {
  if (typeof document !== 'undefined') {
    document.documentElement.setAttribute('data-theme', resolveSystemTheme(themeValue));
  }
}

// ---------------------------------------------------------------------------
// Action functions
// ---------------------------------------------------------------------------

/**
 * Loads settings from the Rust config file.
 * Called once during app initialization.
 */
export async function loadSettings(): Promise<void> {
  isLoading.set(true);
  settingsError.set(null);

  try {
    const data = await invoke<Settings>('get_settings');

    saveLocation.set(data.saveLocation);
    defaultFormat.set(data.defaultFormat);
    theme.set(data.theme);
    compression.set(data.compression);
    soundNotification.set(data.soundNotification);
    fileNamingPattern.set(data.fileNamingPattern ?? '{title}');
    seenOnboarding.set(data.seenOnboarding ?? false);

    applyTheme(data.theme);

    if (!(data.seenOnboarding ?? false)) {
      maybeShowFirstLaunchHelp();
    }
  } catch (err: unknown) {
    const message = err instanceof Error ? err.message : String(err);
    settingsError.set(message);
  } finally {
    isLoading.set(false);
  }
}

/**
 * Auto-open the help overlay on first launch. Imports the help store
 * dynamically to avoid a circular module dependency at startup.
 */
function maybeShowFirstLaunchHelp(): void {
  setTimeout(async () => {
    try {
      const { openHelp } = await import('$lib/stores/help');
      openHelp();
    } catch {
      // Help store not yet wired; ignore.
    }
  }, 400);
}

/**
 * Saves current store values to the Rust config file.
 */
export async function saveSettings(): Promise<void> {
  const current = get(currentSettings);

  settingsError.set(null);

  try {
    await invoke('save_settings', { settings: current });
  } catch (err: unknown) {
    const message = err instanceof Error ? err.message : String(err);
    settingsError.set(message);
  }
}

/**
 * Opens the native directory picker and updates `saveLocation`.
 * Persists the change to disk.
 */
export async function pickSaveLocation(): Promise<void> {
  try {
    const selected = await invoke<string>('pick_directory');
    if (selected) {
      saveLocation.set(selected);
      await saveSettings();
    }
  } catch (err: unknown) {
    const message = err instanceof Error ? err.message : String(err);
    settingsError.set(message);
  }
}

/**
 * Updates a single setting and persists to disk.
 *
 * @param key   The setting key.
 * @param value The new value.
 */
export async function updateSetting<K extends keyof Settings>(
  key: K,
  value: Settings[K],
): Promise<void> {
  switch (key) {
    case 'version':
      // Version is owned by the backend; ignore inbound writes.
      return;
    case 'saveLocation':
      saveLocation.set(value as string);
      break;
    case 'defaultFormat':
      defaultFormat.set(value as DownloadFormat);
      break;
    case 'theme':
      theme.set(value as 'light' | 'dark' | 'system');
      applyTheme(value as 'light' | 'dark' | 'system');
      break;
    case 'compression':
      compression.set(value as boolean);
      break;
    case 'soundNotification':
      soundNotification.set(value as boolean);
      break;
    case 'fileNamingPattern':
      fileNamingPattern.set(value as string);
      break;
    case 'seenOnboarding':
      seenOnboarding.set(value as boolean);
      break;
  }

  await saveSettings();
}
