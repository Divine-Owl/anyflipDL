/**
 * HistoryStore — persists completed download records.
 *
 * Stores a list of {@link HistoryItem} objects to localStorage so the user can
 * revisit previously downloaded files. New entries are added automatically
 * when a download completes via the {@link addToHistory} action.
 *
 * @module $lib/stores/history
 */

import { writable, derived, get, type Readable } from 'svelte/store';
import { invoke } from '@tauri-apps/api/core';
import type { HistoryItem } from '$lib/types';

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

const STORAGE_KEY = 'anyflipdl:history';

// ---------------------------------------------------------------------------
// Internal helpers
// ---------------------------------------------------------------------------

/** Reads persisted history from localStorage. Returns empty array on failure. */
function readPersistedHistory(): HistoryItem[] {
  try {
    if (typeof window === 'undefined' || typeof localStorage === 'undefined') return [];
    const raw = localStorage.getItem(STORAGE_KEY);
    if (!raw) return [];
    const parsed = JSON.parse(raw);
    if (!Array.isArray(parsed)) return [];
    return parsed.map(normalizeHistoryItem);
  } catch {
    return [];
  }
}

/**
 * Normalize a parsed JSON entry into a {@link HistoryItem}, filling in defaults
 * for fields added in later versions. This is the migration path for items
 * persisted before metadata enrichment (date/thumbnail/passwordProtected) was
 * added.
 */
function normalizeHistoryItem(raw: unknown): HistoryItem {
  const r = raw as Partial<HistoryItem> & Record<string, unknown>;
  return {
    id: String(r.id ?? ''),
    url: String(r.url ?? ''),
    title: String(r.title ?? 'Untitled'),
    pageCount: Number(r.pageCount ?? 0),
    format: (r.format === 'epub' ? 'epub' : 'pdf'),
    outputPath: String(r.outputPath ?? ''),
    completedAt: String(r.completedAt ?? new Date(0).toISOString()),
    fileSize: Number(r.fileSize ?? 0),
    date: typeof r.date === 'string' ? r.date : null,
    thumbnail: typeof r.thumbnail === 'string' ? r.thumbnail : null,
    passwordProtected: Boolean(r.passwordProtected),
  };
}

/** Persists the full history array to localStorage. */
function persistHistory(items: HistoryItem[]): void {
  if (typeof localStorage !== 'undefined') {
    localStorage.setItem(STORAGE_KEY, JSON.stringify(items));
  }
}

// ---------------------------------------------------------------------------
// Writable stores
// ---------------------------------------------------------------------------

/** All completed download history entries, newest first. */
export const historyItems = writable<HistoryItem[]>(readPersistedHistory());

// Auto-persist on every change.
historyItems.subscribe((items) => {
  persistHistory(items);
});

// ---------------------------------------------------------------------------
// Derived stores
// ---------------------------------------------------------------------------

/** Total number of history entries. */
export const historyCount: Readable<number> = derived(
  historyItems,
  ($items) => $items.length,
);

// ---------------------------------------------------------------------------
// Action functions
// ---------------------------------------------------------------------------

/**
 * Adds a completed download to the history.
 *
 * @param item  The history entry to prepend (newest first).
 */
export function addToHistory(item: HistoryItem): void {
  historyItems.update((items) => [item, ...items]);
}

/**
 * Removes a single history entry by ID.
 *
 * @param id  The history item ID to remove.
 */
export function removeHistoryItem(id: string): void {
  historyItems.update((items) => items.filter((i) => i.id !== id));
}

/**
 * Renames a history item's file on disk and updates its record.
 *
 * Invokes the `rename_file` Tauri command, then updates the history entry
 * with the new title and output path.
 *
 * @param id        The history item ID.
 * @param newTitle  The new title to apply (will be sanitized).
 * @returns The new output path, or throws on failure.
 */
export async function renameHistoryItem(id: string, newTitle: string): Promise<string> {
  const items = get(historyItems);
  const item = items.find((i) => i.id === id);
  if (!item) throw new Error('History item not found');

  const result = await invoke<{ new_path: string; new_title: string }>('rename_file', {
    currentPath: item.outputPath,
    newTitle,
  });

  historyItems.update((prev) =>
    prev.map((i) =>
      i.id === id ? { ...i, title: result.new_title, outputPath: result.new_path } : i,
    ),
  );

  return result.new_path;
}

/** Clears all history entries. */
export function clearHistory(): void {
  historyItems.set([]);
}

/**
 * Opens a file using the system default application.
 * Invokes the `open_file` Tauri command.
 *
 * @param path  Absolute file path to open.
 */
export async function openFile(path: string): Promise<void> {
  await invoke('open_file', { path });
}

/**
 * Opens the parent directory of a file in the system file manager.
 *
 * @param filePath  Absolute file path whose parent directory to open.
 */
export async function openContainingFolder(filePath: string): Promise<void> {
  // Derive parent directory. Handles both `/` and `\` separators.
  const lastSep = Math.max(filePath.lastIndexOf('/'), filePath.lastIndexOf('\\'));
  const dir = lastSep > 0 ? filePath.substring(0, lastSep) : filePath;
  await invoke('open_file', { path: dir });
}
