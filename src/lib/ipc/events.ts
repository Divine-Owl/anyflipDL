import { listen, type UnlistenFn } from '@tauri-apps/api/event';
import {
  handleProgress,
  handleStatus,
  handleComplete,
  handleError,
  handlePasswordRequired,
  handleEstimatedSize,
} from '../stores/downloads';

/** Event payload shapes matching Rust emission. */
interface DownloadProgressEvent {
  id: string;
  progress: number;
  currentPage: number;
  totalPages: number;
}

interface DownloadCompleteEvent {
  id: string;
  outputPath: string;
}

interface DownloadErrorEvent {
  id: string;
  error: string;
  code: string;
  retryable: boolean;
}

interface DownloadStatusEvent {
  id: string;
  status: 'downloading' | 'converting' | 'saving' | 'paused';
}

interface DownloadEstimatedSizeEvent {
  id: string;
  estimatedBytes: number;
  estimatedFormatted: string;
}

interface DownloadPasswordRequiredEvent {
  id: string;
  url: string;
}

/**
 * Set up all Tauri event listeners for download pipeline events.
 * Returns a cleanup function that unlistens all.
 */
export async function setupDownloadListeners(): Promise<UnlistenFn> {
  const unlisteners: UnlistenFn[] = [];

  unlisteners.push(
    await listen<DownloadProgressEvent>('download:progress', (event) => {
      const { id, progress, currentPage, totalPages } = event.payload;
      handleProgress(id, progress, currentPage, totalPages);
    })
  );

  unlisteners.push(
    await listen<DownloadStatusEvent>('download:status', (event) => {
      const { id, status } = event.payload;
      handleStatus(id, status);
    })
  );

  unlisteners.push(
    await listen<DownloadCompleteEvent>('download:complete', (event) => {
      const { id, outputPath } = event.payload;
      handleComplete(id, outputPath);
    })
  );

  unlisteners.push(
    await listen<DownloadErrorEvent>('download:error', (event) => {
      const { id, error, code, retryable } = event.payload;
      handleError(id, error, code, retryable);
    })
  );

  unlisteners.push(
    await listen<DownloadPasswordRequiredEvent>('download:password_required', (event) => {
      const { id, url } = event.payload;
      handlePasswordRequired(id, url);
    })
  );

  unlisteners.push(
    await listen<DownloadEstimatedSizeEvent>('download:estimated_size', (event) => {
      const { id, estimatedBytes, estimatedFormatted } = event.payload;
      handleEstimatedSize(id, estimatedBytes, estimatedFormatted);
    })
  );

  return () => {
    for (const unlisten of unlisteners) {
      unlisten();
    }
  };
}
