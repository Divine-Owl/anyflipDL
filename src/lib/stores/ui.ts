/**
 * UIStateStore — manages ephemeral UI state.
 *
 * Tracks which view is active. `activeView` is session-scoped.
 *
 * @module $lib/stores/ui
 */

import { writable, derived, get, type Readable } from 'svelte/store';
import type { ActiveView } from '$lib/types';

// ---------------------------------------------------------------------------
// Writable stores
// ---------------------------------------------------------------------------

/**
 * Currently active view.
 *
 * `'settings'` is treated as an overlay, not a primary navigation target.
 * Use {@link navigateTo} for primary navigation and {@link toggleSettings}
 * for the settings panel.
 */
export const activeView = writable<ActiveView>('url');

// ---------------------------------------------------------------------------
// Derived stores
// ---------------------------------------------------------------------------

/** `true` when the URL input view is active. */
export const isUrlView: Readable<boolean> = derived(
  activeView,
  ($view) => $view === 'url',
);

/** `true` when the downloads view is active. */
export const isDownloadsView: Readable<boolean> = derived(
  activeView,
  ($view) => $view === 'downloads',
);

/** `true` when the settings panel is open. */
export const isSettingsOpen: Readable<boolean> = derived(
  activeView,
  ($view) => $view === 'settings',
);

// ---------------------------------------------------------------------------
// Action functions
// ---------------------------------------------------------------------------

/** `true` when the history view is active. */
export const isHistoryView: Readable<boolean> = derived(
  activeView,
  ($view) => $view === 'history',
);

/**
 * Navigates to a primary view.
 * If the settings panel is open, it is closed first.
 *
 * @param view The target view (`'url'`, `'downloads'`, or `'history'`).
 */
export function navigateTo(view: 'url' | 'downloads' | 'history'): void {
  activeView.set(view);
}

/**
 * Toggles the settings panel overlay.
 * Opens if closed, closes if open (returning to `'url'`).
 */
export function toggleSettings(): void {
  const current = get(activeView);
  activeView.set(current === 'settings' ? 'url' : 'settings');
}

