/**
 * Barrel export for all AnyflipDL stores.
 *
 * @module $lib/stores
 */

// Download queue
export {
  downloadItems,
  activeDownloadId,
  activeDownload,
  queueLength,
  hasActiveDownload,
  completedCount,
  errorCount,
  itemsByStatus,
  addToQueue,
  removeFromQueue,
  retryItem,
  updateItemFormat,
  processQueue,
  cancelActiveDownload,
  clearCompleted,
  clearErrors,
  handleProgress,
  handleStatus,
  handleComplete,
  handleError,
} from './downloads';

// Settings
export {
  saveLocation,
  defaultFormat,
  theme,
  compression,
  isLoading,
  settingsError,
  currentSettings,
  loadSettings,
  saveSettings,
  pickSaveLocation,
  updateSetting,
} from './settings';

// Download history
export {
  historyItems,
  historyCount,
  addToHistory,
  removeHistoryItem,
  clearHistory,
  openFile,
  openContainingFolder,
} from './history';

// UI state
export {
  activeView,
  isUrlView,
  isDownloadsView,
  isHistoryView,
  isSettingsOpen,
  navigateTo,
  toggleSettings,
} from './ui';
