/**
 * NetworkStatusStore — tracks browser online/offline state.
 *
 * Uses `navigator.onLine` and `online`/`offline` window events to keep
 * a reactive Svelte store in sync with the system's connectivity status.
 *
 * @module $lib/stores/network
 */

import { writable, derived, type Readable } from 'svelte/store';

// ---------------------------------------------------------------------------
// Writable stores
// ---------------------------------------------------------------------------

/**
 * Whether the system reports internet connectivity.
 * Initialized from `navigator.onLine` and updated by window events.
 */
export const isOnline = writable<boolean>(
  typeof navigator !== 'undefined' ? navigator.onLine : true,
);

// ---------------------------------------------------------------------------
// Derived stores
// ---------------------------------------------------------------------------

/** Convenience inverse of `isOnline` — `true` when offline. */
export const isOffline: Readable<boolean> = derived(isOnline, ($online) => !$online);

// ---------------------------------------------------------------------------
// Event listener setup / teardown
// ---------------------------------------------------------------------------

let listenersAttached = false;

function handleOnline(): void {
  isOnline.set(true);
}

function handleOffline(): void {
  isOnline.set(false);
}

/**
 * Attaches `online`/`offline` event listeners to the window.
 * Safe to call multiple times — listeners are only attached once.
 *
 * Call {@link detachNetworkListeners} to clean up (e.g. on component destroy
 * in SSR-aware contexts).
 */
export function attachNetworkListeners(): void {
  if (listenersAttached || typeof window === 'undefined') return;
  window.addEventListener('online', handleOnline);
  window.addEventListener('offline', handleOffline);
  listenersAttached = true;
}

/**
 * Removes the `online`/`offline` event listeners.
 */
export function detachNetworkListeners(): void {
  if (!listenersAttached || typeof window === 'undefined') return;
  window.removeEventListener('online', handleOnline);
  window.removeEventListener('offline', handleOffline);
  listenersAttached = false;
}

// Auto-attach on module load (browser only).
if (typeof window !== 'undefined') {
  attachNetworkListeners();
}
