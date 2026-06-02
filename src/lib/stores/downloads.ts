/**
 * DownloadQueueStore — central state for the download queue.
 *
 * Manages a collection of {@link DownloadItem} objects with a strict state
 * machine governing status transitions. At most one item is active
 * (downloading / converting / saving) at any time.
 *
 * @module $lib/stores/downloads
 */

import { writable, derived, get, type Readable } from 'svelte/store';
import { invoke } from '@tauri-apps/api/core';
import { saveLocation, compression, soundNotification } from '$lib/stores/settings';
import { isOnline } from '$lib/stores/network';
import { addToHistory } from '$lib/stores/history';
import { playNotificationSound } from '$lib/utils/sound';
import type {
  DownloadItem,
  DownloadStatus,
  DownloadFormat,
} from '$lib/types';
import type { DocumentMetadata } from '$lib/ipc/client';

// ---------------------------------------------------------------------------
// Writable stores
// ---------------------------------------------------------------------------

/** All items in the download queue, ordered by insertion time (FIFO). */
export const downloadItems = writable<DownloadItem[]>([]);

/** ID of the currently active item, or `null` when idle. */
export const activeDownloadId = writable<string | null>(null);

/**
 * ID of the download currently waiting for a password, or `null`.
 * When non-null, the PasswordDialog should be visible.
 */
export const passwordPromptDownloadId = writable<string | null>(null);

// ---------------------------------------------------------------------------
// Derived stores
// ---------------------------------------------------------------------------

/** The active {@link DownloadItem}, or `null` if the queue is idle. */
export const activeDownload: Readable<DownloadItem | null> = derived(
  [downloadItems, activeDownloadId],
  ([$items, $activeId]) => $items.find((i) => i.id === $activeId) ?? null,
);

/** Number of items with status `'queued'`. */
export const queueLength: Readable<number> = derived(downloadItems, ($items) =>
  $items.filter((i) => i.status === 'queued').length,
);

/** `true` when an item is actively downloading, converting, or saving. */
export const hasActiveDownload: Readable<boolean> = derived(
  activeDownloadId,
  ($id) => $id !== null,
);

/** Number of items with status `'complete'`. */
export const completedCount: Readable<number> = derived(
  downloadItems,
  ($items) => $items.filter((i) => i.status === 'complete').length,
);

/** Number of items with status `'error'`. */
export const errorCount: Readable<number> = derived(downloadItems, ($items) =>
  $items.filter((i) => i.status === 'error').length,
);

/** Items grouped by status for UI filtering. */
export const itemsByStatus: Readable<Record<DownloadStatus, DownloadItem[]>> =
  derived(downloadItems, ($items) => {
    const grouped: Record<DownloadStatus, DownloadItem[]> = {
      queued: [],
      downloading: [],
      converting: [],
      saving: [],
      paused: [],
      complete: [],
      error: [],
      'password-required': [],
    };
    for (const item of $items) {
      grouped[item.status].push(item);
    }
    return grouped;
  });

// ---------------------------------------------------------------------------
// Internal helpers
// ---------------------------------------------------------------------------

/** Reset transient fields when moving an item back to `queued`. */
function resetToQueued(item: DownloadItem): DownloadItem {
  return {
    ...item,
    status: 'queued',
    progress: 0,
    currentPage: 0,
    totalPages: 0,
    error: null,
    errorCode: null,
    retryable: true,
  };
}

// ---------------------------------------------------------------------------
// Action functions
// ---------------------------------------------------------------------------

/**
 * Adds a new item to the queue.
 *
 * If `metadata` is provided (already fetched by the caller), uses it directly.
 * Otherwise fetches document metadata from the Rust backend. Creates a
 * {@link DownloadItem} with status `'queued'`, and auto-starts queue
 * processing when idle.
 *
 * @param url    Document URL.
 * @param format Output format.
 * @param metadata Optional pre-fetched metadata to avoid a redundant IPC call.
 * @param customTitle Optional user-edited title (overrides metadata.title for display).
 * @returns The new item's ID.
 * @throws If metadata fetch fails.
 */
export async function addToQueue(
  url: string,
  format: DownloadFormat,
  metadata?: DocumentMetadata,
  customTitle?: string,
): Promise<string> {
  const meta: DocumentMetadata =
    metadata ??
    (await invoke<DocumentMetadata>('fetch_document_metadata', { url }));

  const id = crypto.randomUUID();
  const item: DownloadItem = {
    id,
    url,
    title: customTitle?.trim() || meta.title,
    pageCount: meta.pageCount,
    status: 'queued',
    progress: 0,
    currentPage: 0,
    totalPages: 0,
    format,
    error: null,
    errorCode: null,
    retryable: true,
    outputPath: null,
    estimatedBytes: null,
    estimatedFormatted: null,
    date: meta.date ?? null,
    thumbnail: meta.thumbnail ?? null,
    passwordProtected: meta.passwordProtected,
  };

  downloadItems.update((items) => [...items, item]);

  // Auto-start if queue is idle.
  processQueue();

  return id;
}

/**
 * Removes an item from the queue.
 * If the item is currently active, cancels it first.
 */
export function removeFromQueue(id: string): void {
  if (get(activeDownloadId) === id) {
    cancelActiveDownload();
  }

  downloadItems.update((items) => items.filter((i) => i.id !== id));
}

/**
 * Retries a failed item — resets status to `'queued'` and clears the error.
 * Triggers queue processing if idle.
 */
export function retryItem(id: string): void {
  downloadItems.update((items) =>
    items.map((item) => {
      if (item.id !== id || item.status !== 'error') return item;
      return resetToQueued(item);
    }),
  );

  processQueue();
}

/**
 * Updates the output format for a queued item.
 * No-op if the item is not in `'queued'` status.
 */
export function updateItemFormat(id: string, format: DownloadFormat): void {
  downloadItems.update((items) =>
    items.map((item) => {
      if (item.id !== id || item.status !== 'queued') return item;
      return { ...item, format };
    }),
  );
}

/**
 * Updates the title for a queued item.
 * No-op if the item is not in `'queued'` status.
 */
export function updateItemTitle(id: string, title: string): void {
  downloadItems.update((items) =>
    items.map((item) => {
      if (item.id !== id || item.status !== 'queued') return item;
      return { ...item, title };
    }),
  );
}

/**
 * Processes the queue: picks the next `'queued'` item and starts its download.
 * No-op if a download is already active or the queue has no queued items.
 */
export async function processQueue(): Promise<void> {
  if (get(activeDownloadId) !== null) return; // already processing
  if (!get(isOnline)) return; // offline — don't start new downloads

  const items = get(downloadItems);
  const nextItem = items.find((i) => i.status === 'queued');

  if (!nextItem) return; // nothing to process

  // Transition: queued -> downloading
  downloadItems.update((prev) =>
    prev.map((i) =>
      i.id === nextItem.id ? { ...i, status: 'downloading' as DownloadStatus } : i,
    ),
  );
  activeDownloadId.set(nextItem.id);

  try {
    // Pass the frontend-generated ID so backend events use the same identifier.
    const savePath = get(saveLocation);
    const compress = get(compression);
    await invoke('download_document', {
      id: nextItem.id,
      url: nextItem.url,
      format: nextItem.format,
      savePath,
      compression: compress,
    });
    // Enqueue succeeded. Progress/completion is handled by event listeners.
  } catch (err: unknown) {
    // If the invoke itself rejects (e.g. network error before events start).
    const message = err instanceof Error ? err.message : String(err);
    // Extract code from TauriCommandError if available.
    const code = err instanceof Error && 'code' in err ? (err as any).code : 'UNKNOWN';
    const retryable = err instanceof Error && 'retryable' in err ? (err as any).retryable : true;
    handleError(nextItem.id, message, code, retryable);
  }
}

/**
 * Cancels the active download. The item is reset to `'queued'` so it can be
 * retried later.
 */
export function cancelActiveDownload(): void {
  const currentActiveId = get(activeDownloadId);
  if (currentActiveId === null) return;

  downloadItems.update((items) =>
    items.map((item) =>
      item.id === currentActiveId ? resetToQueued(item) : item,
    ),
  );
  activeDownloadId.set(null);

  // Notify backend to abort the active download.
  invoke('cancel_download', { downloadId: currentActiveId }).catch(() => {
    // Swallow — best-effort cancellation.
  });

  // Try to start the next queued item.
  processQueue();
}

/** Clears all completed items from the queue. */
export function clearCompleted(): void {
  downloadItems.update((items) =>
    items.filter((i) => i.status !== 'complete'),
  );
}

/** Clears all error items from the queue. */
export function clearErrors(): void {
  downloadItems.update((items) => items.filter((i) => i.status !== 'error'));
}

// ---------------------------------------------------------------------------
// Event handlers — called by Tauri event listeners (set up in T9/T14)
// ---------------------------------------------------------------------------

/**
 * Handles a progress update from the backend.
 *
 * @param id          The item ID.
 * @param progress    New progress value (0-100).
 * @param currentPage Current page being downloaded.
 * @param totalPages  Total pages in the document.
 */
export function handleProgress(
  id: string,
  progress: number,
  currentPage?: number,
  totalPages?: number,
): void {
  const clamped = Math.max(0, Math.min(100, progress));
  downloadItems.update((items) =>
    items.map((item) =>
      item.id === id
        ? {
            ...item,
            progress: clamped,
            currentPage: currentPage ?? item.currentPage,
            totalPages: totalPages ?? item.totalPages,
          }
        : item,
    ),
  );
}

/**
 * Handles a status transition event from the backend
 * (e.g. downloading -> converting -> saving).
 *
 * @param id     The item ID.
 * @param status The new status.
 */
export function handleStatus(id: string, status: DownloadStatus): void {
  downloadItems.update((items) =>
    items.map((item) => (item.id === id ? { ...item, status } : item)),
  );
}

/**
 * Handles a download completion event.
 *
 * Sets the item to `'complete'`, clears the active download, and advances
 * the queue to the next queued item.
 *
 * @param id         The item ID.
 * @param outputPath The final output file path (from Rust `download:complete` event).
 */
export function handleComplete(id: string, outputPath?: string): void {
  // Capture item data before updating, so we can add it to history.
  let completedItem: DownloadItem | undefined;

  downloadItems.update((items) =>
    items.map((item) => {
      if (item.id !== id) return item;
      completedItem = {
        ...item,
        status: 'complete' as DownloadStatus,
        progress: 100,
        outputPath: outputPath ?? item.outputPath,
      };
      return completedItem;
    }),
  );

  // Play sound notification if enabled.
  if (get(soundNotification)) {
    playNotificationSound();
  }

  // Add to history if we have a valid output path.
  if (completedItem && completedItem.outputPath) {
    const item = completedItem;
    // Try to read file size asynchronously; default to 0 on failure.
    invoke<{ size: number }>('get_file_size', { path: item.outputPath })
      .then((meta) => {
        addToHistory({
          id: item.id,
          url: item.url,
          title: item.title,
          pageCount: item.pageCount,
          format: item.format,
          outputPath: item.outputPath!,
          completedAt: new Date().toISOString(),
          fileSize: meta.size,
          date: item.date,
          thumbnail: item.thumbnail,
          passwordProtected: item.passwordProtected,
        });
      })
      .catch(() => {
        addToHistory({
          id: item.id,
          url: item.url,
          title: item.title,
          pageCount: item.pageCount,
          format: item.format,
          outputPath: item.outputPath!,
          completedAt: new Date().toISOString(),
          fileSize: 0,
          date: item.date,
          thumbnail: item.thumbnail,
          passwordProtected: item.passwordProtected,
        });
      });
  }

  // Clear active and advance queue only if this was the active item.
  if (get(activeDownloadId) === id) {
    activeDownloadId.set(null);
    processQueue();
  }
}

/**
 * Handles a download error event from the backend.
 *
 * Sets the item to `'error'`, stores the error message and metadata, clears
 * the active download, and advances the queue.
 *
 * @param id        The item ID.
 * @param error     Error message.
 * @param code      Machine-readable error code (e.g., 'DOWNLOAD_FAILED').
 * @param retryable Whether the user can retry the operation.
 */
export function handleError(
  id: string,
  error: string,
  code?: string,
  retryable?: boolean,
): void {
  downloadItems.update((items) =>
    items.map((item) => {
      if (item.id !== id) return item;
      return {
        ...item,
        status: 'error' as DownloadStatus,
        error,
        errorCode: code ?? item.errorCode,
        retryable: retryable ?? item.retryable,
      };
    }),
  );

  // Clear active and advance queue only if this was the active item.
  if (get(activeDownloadId) === id) {
    activeDownloadId.set(null);
    processQueue();
  }
}

/**
 * Handles a password-required event from the backend.
 *
 * Sets the item to `'password-required'` status and opens the password dialog.
 * The download stays active (activeDownloadId is NOT cleared) so the backend
 * task remains parked waiting for the password.
 *
 * @param id  The item ID.
 * @param url The document URL (for display).
 */
export function handlePasswordRequired(id: string, _url: string): void {
  downloadItems.update((items) =>
    items.map((item) => {
      if (item.id !== id) return item;
      return { ...item, status: 'password-required' as DownloadStatus };
    }),
  );

  passwordPromptDownloadId.set(id);
}

/**
 * Submits a password for the download currently waiting for one.
 *
 * Invokes the `submit_password` Tauri command. On success, the backend
 * resumes the download and will emit status/progress events as normal.
 * On failure, marks the download as an error.
 *
 * @param password The password entered by the user.
 */
export async function submitPassword(password: string): Promise<void> {
  const id = get(passwordPromptDownloadId);
  if (!id) return;

  // Close the dialog immediately.
  passwordPromptDownloadId.set(null);

  // Transition back to downloading while the backend retries.
  downloadItems.update((items) =>
    items.map((item) => {
      if (item.id !== id) return item;
      return { ...item, status: 'downloading' as DownloadStatus, error: null, errorCode: null };
    }),
  );

  try {
    await invoke('submit_password', { downloadId: id, password });
    // Password accepted. The backend will resume emitting progress/completion
    // events. No further action needed here.
  } catch (err: unknown) {
    const message = err instanceof Error ? err.message : String(err);
    handleError(id, message, 'PASSWORD_SUBMIT_FAILED', true);
  }
}

/**
 * Cancels the password prompt without submitting a password.
 *
 * Marks the download as an error (user cancelled password entry) and clears
 * the prompt state. The backend task will be unblocked because the oneshot
 * sender is dropped, causing it to error out gracefully.
 */
export function cancelPasswordPrompt(): void {
  const id = get(passwordPromptDownloadId);
  if (!id) return;

  passwordPromptDownloadId.set(null);

  handleError(id, 'Password entry cancelled', 'PASSWORD_CANCELLED', true);
}

/**
 * Pauses the active download.
 *
 * The backend will pause before processing the next page. The item status
 * is updated to `'paused'` locally for immediate UI feedback; the backend
 * will also emit a confirming status event.
 */
export async function pauseDownload(id: string): Promise<void> {
  downloadItems.update((items) =>
    items.map((item) =>
      item.id === id ? { ...item, status: 'paused' as DownloadStatus } : item,
    ),
  );

  try {
    await invoke('pause_download', { downloadId: id });
  } catch (err: unknown) {
    // If pause failed, revert to downloading.
    const message = err instanceof Error ? err.message : String(err);
    console.warn('Pause failed:', message);
    downloadItems.update((items) =>
      items.map((item) =>
        item.id === id ? { ...item, status: 'downloading' as DownloadStatus } : item,
      ),
    );
  }
}

/**
 * Resumes a paused download.
 *
 * Clears the pause flag on the backend so the page download loop continues.
 */
export async function resumeDownload(id: string): Promise<void> {
  downloadItems.update((items) =>
    items.map((item) =>
      item.id === id ? { ...item, status: 'downloading' as DownloadStatus } : item,
    ),
  );

  try {
    await invoke('resume_download', { downloadId: id });
  } catch (err: unknown) {
    const message = err instanceof Error ? err.message : String(err);
    console.warn('Resume failed:', message);
    // Revert to paused on failure.
    downloadItems.update((items) =>
      items.map((item) =>
        item.id === id ? { ...item, status: 'paused' as DownloadStatus } : item,
      ),
    );
  }
}

/**
 * Handles an estimated file size event from the backend.
 *
 * Updates the item's estimated size fields so the UI can display them.
 *
 * @param id               The item ID.
 * @param estimatedBytes   Estimated output size in bytes.
 * @param estimatedFormatted Human-readable size string (e.g., "12.3 MB").
 */
export function handleEstimatedSize(
  id: string,
  estimatedBytes: number,
  estimatedFormatted: string,
): void {
  downloadItems.update((items) =>
    items.map((item) =>
      item.id === id ? { ...item, estimatedBytes, estimatedFormatted } : item,
    ),
  );
}
