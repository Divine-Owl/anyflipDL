/**
 * Tauri IPC Client -- typed wrapper around `invoke()` with consistent error handling.
 *
 * All frontend-to-backend IPC calls route through `tauriInvoke`, which maps Rust
 * `AppError` variants to frontend `TauriCommandError` instances with typed error
 * codes and retryable flags.
 *
 * @module lib/ipc/client
 */

import { invoke, type InvokeArgs } from '@tauri-apps/api/core';
import type { Settings } from '$lib/types';

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

/** Error codes mapped from Rust `AppError` variants. */
export type ErrorCode =
  | 'INVALID_URL'
  | 'NETWORK_ERROR'
  | 'METADATA_ERROR'
  | 'DOWNLOAD_ERROR'
  | 'CONVERSION_ERROR'
  | 'FILESYSTEM_ERROR'
  | 'SETTINGS_ERROR'
  | 'UNKNOWN';

/** Metadata returned by the `fetch_document_metadata` Rust command. */
export interface DocumentMetadata {
  /** Canonical URL of the document. */
  url: string;
  /** Document title extracted from config.js. */
  title: string;
  /** Total number of pages. */
  pageCount: number;
  /** Page filenames from config.js. May be empty if config.js doesn't list them. */
  pageFilenames: string[];
  /** Document publication or creation date (raw string from Anyflip). */
  date: string | null;
  /** URL to the document's cover image. Frontend uses `<img onerror>` fallback. */
  thumbnail: string | null;
  /** `true` if the document required a password to fetch metadata. */
  passwordProtected: boolean;
}

/** Emitted during page download progress. */
export interface DownloadProgressEvent {
  id: string;
  /** Download progress percentage (0-100). */
  progress: number;
  currentPage: number;
  totalPages: number;
}

/** Emitted when download pipeline completes successfully. */
export interface DownloadCompleteEvent {
  id: string;
  /** Full path to the generated PDF/EPUB file. */
  outputPath: string;
}

/** Emitted when download pipeline encounters an error. */
export interface DownloadErrorEvent {
  id: string;
  /** Human-readable error message. */
  error: string;
  /** Error code (e.g., 'NETWORK_ERROR'). */
  code: string;
  /** Whether the user can retry the operation. */
  retryable: boolean;
}

/** Emitted on pipeline phase transitions. */
export interface DownloadStatusEvent {
  id: string;
  status: 'downloading' | 'converting' | 'saving';
}

/** Supported download output formats. */
export type DownloadFormat = 'pdf' | 'epub';

// ---------------------------------------------------------------------------
// Error class
// ---------------------------------------------------------------------------

/**
 * Error class for Tauri command failures.
 * Wraps the Rust `AppError` into a typed frontend error with a machine-readable
 * code, a retryable flag, and the original raw error for debugging.
 */
export class TauriCommandError extends Error {
  /** Machine-readable error code mapped from Rust `AppError` variant. */
  readonly code: ErrorCode;
  /** Whether the operation can be retried. */
  readonly retryable: boolean;
  /** The original error caught from `invoke()`. */
  readonly raw: unknown;

  constructor(code: ErrorCode, message: string, retryable: boolean, raw: unknown) {
    super(message);
    this.name = 'TauriCommandError';
    this.code = code;
    this.retryable = retryable;
    this.raw = raw;
  }
}

// ---------------------------------------------------------------------------
// Error mapping
// ---------------------------------------------------------------------------

/**
 * Maps an unknown error thrown by `invoke()` to a typed `{ code, message, retryable }`
 * triple. The mapping follows the Rust `AppError` variant table in api-conventions.md.
 */
function mapError(raw: unknown): { code: ErrorCode; message: string; retryable: boolean } {
  // Tauri serializes Rust errors as string messages. We pattern-match on the
  // serialized variant name that appears in the error string.
  const msg = typeof raw === 'string' ? raw : String(raw);

  if (msg.includes('InvalidUrl') || msg.includes('Invalid URL')) {
    return { code: 'INVALID_URL', message: msg, retryable: false };
  }
  if (msg.includes('NetworkError') || msg.includes('Network error')) {
    return { code: 'NETWORK_ERROR', message: msg, retryable: true };
  }
  if (msg.includes('MetadataError') || msg.includes('Metadata error')) {
    return { code: 'METADATA_ERROR', message: msg, retryable: true };
  }
  if (msg.includes('DownloadError') || msg.includes('Download error')) {
    return { code: 'DOWNLOAD_ERROR', message: msg, retryable: true };
  }
  if (msg.includes('ConversionError') || msg.includes('Conversion error')) {
    return { code: 'CONVERSION_ERROR', message: msg, retryable: false };
  }
  if (msg.includes('FileSystemError') || msg.includes('File system error') || msg.includes('Filesystem error')) {
    // Filesystem errors may or may not be retryable depending on the
    // underlying cause (disk full vs transient lock). Default to retryable.
    return { code: 'FILESYSTEM_ERROR', message: msg, retryable: true };
  }
  if (msg.includes('SettingsError') || msg.includes('Settings error')) {
    return { code: 'SETTINGS_ERROR', message: msg, retryable: false };
  }

  return { code: 'UNKNOWN', message: msg, retryable: false };
}

// ---------------------------------------------------------------------------
// Core invoke wrapper
// ---------------------------------------------------------------------------

/**
 * Typed wrapper around Tauri `invoke()`.
 *
 * Provides consistent error handling: any error thrown by `invoke()` is caught
 * and re-thrown as a `TauriCommandError` with a typed error code, retryable
 * flag, and the original raw error.
 *
 * @param command - Tauri command name (snake_case).
 * @param args - Command arguments (camelCase in TS, auto-converted to snake_case by Tauri).
 * @returns Promise resolving to the command result.
 * @throws {TauriCommandError} On command failure.
 *
 * @example
 * ```ts
 * const metadata = await tauriInvoke<DocumentMetadata>('fetch_document_metadata', { url });
 * ```
 */
export async function tauriInvoke<T>(command: string, args?: InvokeArgs): Promise<T> {
  try {
    return await invoke<T>(command, args);
  } catch (raw: unknown) {
    const { code, message, retryable } = mapError(raw);
    throw new TauriCommandError(code, message, retryable, raw);
  }
}

// ---------------------------------------------------------------------------
// Typed command wrappers
// ---------------------------------------------------------------------------

/**
 * Validates an Anyflip URL format.
 *
 * @param url - URL to validate.
 * @returns `true` if the URL matches the Anyflip pattern, `false` otherwise.
 * @throws {TauriCommandError} On IPC failure.
 */
export async function validateUrl(url: string): Promise<boolean> {
  return tauriInvoke<boolean>('validate_url', { url });
}

/**
 * Enqueues a document for download processing.
 *
 * The download runs asynchronously via the backend queue manager.
 * Progress and completion are reported through Tauri events, not the
 * return value of this function.
 *
 * @param url - Anyflip document URL.
 * @param format - Output format ('pdf' or 'epub').
 * @param savePath - Directory to save the generated file.
 * @param id - Optional frontend-generated ID for event correlation. If not
 *             provided, the backend generates a UUID.
 * @returns The download ID used by the queue.
 * @throws {TauriCommandError} On enqueue failure (invalid URL, duplicate, etc.).
 */
export async function downloadDocument(
  url: string,
  format: DownloadFormat,
  savePath: string,
  id?: string
): Promise<string> {
  return tauriInvoke<string>('download_document', { id, url, format, savePath });
}

/**
 * Cancels a queued or active download by ID.
 *
 * If the item is currently downloading, the download task is aborted.
 * If the item is pending, it is removed from the queue.
 * If the item is not found, this is a no-op (idempotent).
 *
 * @param downloadId - The download ID to cancel.
 * @throws {TauriCommandError} On IPC failure.
 */
export async function cancelDownload(downloadId: string): Promise<void> {
  return tauriInvoke<void>('cancel_download', { downloadId });
}

/**
 * Loads application settings from the config file.
 *
 * @returns The current settings.
 * @throws {TauriCommandError} On config read failure.
 */
export async function getSettings(): Promise<Settings> {
  return tauriInvoke<Settings>('get_settings');
}

/**
 * Saves application settings to the config file.
 *
 * @param settings - Settings to persist.
 * @throws {TauriCommandError} On config write failure.
 */
export async function saveSettings(settings: Settings): Promise<void> {
  return tauriInvoke<void>('save_settings', { settings });
}

/**
 * Opens a native directory picker dialog.
 *
 * @returns The selected directory path, or `null` if the user cancelled.
 * @throws {TauriCommandError} On dialog failure.
 */
export async function pickDirectory(): Promise<string | null> {
  return tauriInvoke<string | null>('pick_directory');
}

/**
 * Opens a file in the default system application.
 *
 * @param path - Full path to the file to open.
 * @throws {TauriCommandError} On failure.
 */
export async function openFile(path: string): Promise<void> {
  return tauriInvoke<void>('open_file', { path });
}

/**
 * Result of an update check from the GitHub releases API.
 * Re-exported here for convenience so callers don't need a separate import.
 */
export interface UpdateCheckResult {
  readonly currentVersion: string;
  readonly latestVersion: string;
  readonly updateAvailable: boolean;
  readonly downloadUrl: string | null;
}

/**
 * Checks for application updates via the GitHub releases API.
 *
 * Fetches the latest release and compares it against the current version.
 *
 * @returns The update check result.
 * @throws {TauriCommandError} On network or parsing failure.
 */
export async function checkForUpdates(): Promise<UpdateCheckResult> {
  return tauriInvoke<UpdateCheckResult>('check_for_updates');
}
