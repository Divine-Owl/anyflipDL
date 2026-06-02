import { writable } from 'svelte/store';

export const overlayOpen = writable(false);

export function openOverlay(): void { overlayOpen.set(true); }
export function closeOverlay(): void { overlayOpen.set(false); }
export function toggleOverlay(): void { overlayOpen.update((v) => !v); }

export function isEditableTarget(target: EventTarget | null): boolean {
  if (!(target instanceof HTMLElement)) return false;
  const tag = target.tagName;
  if (tag === 'INPUT' || tag === 'TEXTAREA' || tag === 'SELECT') return true;
  if (target.isContentEditable) return true;
  return false;
}
