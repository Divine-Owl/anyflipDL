/**
 * Shared type definitions for AnyflipDL.
 *
 * @module $lib/types
 */

/** Valid download lifecycle statuses. */
export type DownloadStatus =
  | 'queued'
  | 'downloading'
  | 'converting'
  | 'saving'
  | 'paused'
  | 'complete'
  | 'error'
  | 'password-required';

/** Supported output formats. */
export type DownloadFormat = 'pdf' | 'epub';

/** A single item in the download queue. */
export interface DownloadItem {
  readonly id: string;
  readonly url: string;
  readonly title: string;
  readonly pageCount: number;
  status: DownloadStatus;
  progress: number; // 0-100
  /** Current page being downloaded (updated via progress events). */
  currentPage: number;
  /** Total pages in the document (updated via progress events). */
  totalPages: number;
  format: DownloadFormat;
  error: string | null;
  /** Error code from the backend (e.g., 'DOWNLOAD_FAILED'). */
  errorCode: string | null;
  /** Whether the failed operation can be retried. */
  retryable: boolean;
  outputPath: string | null;
  /** Estimated output file size in bytes (set after metadata fetch). */
  estimatedBytes: number | null;
  /** Human-readable estimated file size (e.g., "12.3 MB"). */
  estimatedFormatted: string | null;
  /** Document publication or creation date (raw string from Anyflip). */
  date: string | null;
  /** URL to the document's cover image (`<base_url>/files/large/1.jpg`). */
  thumbnail: string | null;
  /** `true` if the document required a password to fetch metadata. */
  passwordProtected: boolean;
}

/** Application settings persisted to the Rust config file. */
export interface Settings {
  version: number;
  saveLocation: string;
  defaultFormat: DownloadFormat;
  theme: 'light' | 'dark' | 'system';
  compression: boolean;
  soundNotification: boolean;
  fileNamingPattern: string;
  /** Whether the user has seen the onboarding help overlay. */
  seenOnboarding: boolean;
}

/** A completed download entry stored in the history. */
export interface HistoryItem {
  readonly id: string;
  readonly url: string;
  readonly title: string;
  readonly pageCount: number;
  readonly format: DownloadFormat;
  readonly outputPath: string;
  readonly completedAt: string; // ISO 8601 timestamp
  readonly fileSize: number; // bytes, 0 if unknown
  /** Document publication or creation date (raw string from Anyflip). */
  date: string | null;
  /** URL to the document's cover image. */
  thumbnail: string | null;
  /** `true` if the document required a password. */
  passwordProtected: boolean;
}

/** Result of an update check from the GitHub releases API. */
export interface UpdateCheckResult {
  /** Current application version (e.g., "1.0.0"). */
  readonly currentVersion: string;
  /** Latest version found on GitHub (e.g., "1.1.0"). */
  readonly latestVersion: string;
  /** Whether a newer version is available. */
  readonly updateAvailable: boolean;
  /** URL to the release page, only set when `updateAvailable` is true. */
  readonly downloadUrl: string | null;
}

/** Active navigation view. */
export type ActiveView = 'url' | 'downloads' | 'settings' | 'history';

