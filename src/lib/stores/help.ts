/**
 * HelpStore — controls visibility of the onboarding help overlay.
 *
 * Mirrors the pattern used by `keyboard.ts` for the keyboard shortcuts overlay.
 * The first-launch trigger lives in `settings.ts:loadSettings`, which calls
 * {@link openHelp} when `seenOnboarding` is `false`.
 *
 * @module $lib/stores/help
 */

import { writable } from 'svelte/store';

/** `true` when the help overlay should be rendered. */
export const helpOpen = writable<boolean>(false);

/** Opens the help overlay. */
export function openHelp(): void {
  helpOpen.set(true);
}

/** Closes the help overlay. */
export function closeHelp(): void {
  helpOpen.set(false);
}

/** Toggles the help overlay. */
export function toggleHelp(): void {
  helpOpen.update((v) => !v);
}
