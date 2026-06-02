use crate::error::AppError;
use crate::services::download_orchestrator;
use crate::services::page_downloader::PauseToken;
use crate::services::validator;
use serde::Serialize;
use std::collections::VecDeque;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};
use tauri::Emitter;
use tokio::sync::{mpsc, oneshot};

/// Channel buffer size for queue commands.
const CHANNEL_BUFFER: usize = 32;

/// Maximum completed items kept for UI display.
const MAX_COMPLETED_HISTORY: usize = 100;

// ── Public API ──────────────────────────────────────────────────────

/// Create a new QueueManager and return the handle + spawned processing task.
///
/// The processing task runs a `select!` loop that handles incoming commands
/// and download completion. Only one document downloads at a time. When a
/// download completes (success or failure), the next queued item is started
/// automatically.
///
/// # Arguments
/// - `client`: Shared reqwest client for downloads.
/// - `app_handle`: Tauri AppHandle for event emission.
pub fn start<R: tauri::Runtime>(
    client: reqwest::Client,
    app_handle: tauri::AppHandle<R>,
) -> (QueueHandle, std::thread::JoinHandle<()>) {
    let (tx, rx) = mpsc::channel::<QueueCommand>(CHANNEL_BUFFER);
    let qh = QueueHandle { tx };
    // Spawn the process loop on a dedicated thread with its own Tokio runtime.
    // The Tauri setup hook runs synchronously before the async runtime is ready,
    // so we cannot use tauri::async_runtime::spawn here.
    let thread_handle = std::thread::spawn(move || {
        let rt = tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()
            .expect("Failed to create Tokio runtime for queue manager");
        rt.block_on(process_loop(rx, client, app_handle));
    });
    (qh, thread_handle)
}

/// Handle to the queue for submitting and querying download requests.
///
/// `QueueHandle` is `Clone` and can be shared across tasks/threads.
/// All operations use a `oneshot` channel for request-response, ensuring
/// the caller receives acknowledgement before proceeding.
#[derive(Clone)]
pub struct QueueHandle {
    tx: mpsc::Sender<QueueCommand>,
}

impl QueueHandle {
    /// Add a document to the download queue.
    ///
    /// Validates the URL and format before queuing. Rejects duplicates
    /// (URLs already in pending or active).
    ///
    /// If `id` is `Some`, uses that as the download ID (frontend-provided for
    /// event correlation). If `None`, generates a new UUID.
    ///
    /// Returns the assigned download ID on success.
    pub async fn enqueue(
        &self,
        id: Option<String>,
        url: String,
        format: String,
        save_path: String,
        compression: bool,
    ) -> Result<String, AppError> {
        let id = id.unwrap_or_else(|| uuid::Uuid::new_v4().to_string());
        let (resp_tx, resp_rx) = oneshot::channel();

        self.tx
            .send(QueueCommand::Enqueue {
                id: id.clone(),
                url,
                format,
                save_path,
                compression,
                resp: resp_tx,
            })
            .await
            .map_err(|_| AppError::DownloadError("Queue is shut down".to_string()))?;

        resp_rx
            .await
            .map_err(|_| AppError::DownloadError("Queue response dropped".to_string()))?
    }

    /// Cancel a specific download by ID.
    ///
    /// If the item is currently downloading, the download task is aborted.
    /// If the item is pending, it is removed from the queue.
    /// If the item is not found, returns `Ok(())` (idempotent).
    pub async fn cancel(&self, download_id: String) -> Result<(), AppError> {
        let (resp_tx, resp_rx) = oneshot::channel();

        self.tx
            .send(QueueCommand::Cancel {
                id: download_id,
                resp: resp_tx,
            })
            .await
            .map_err(|_| AppError::DownloadError("Queue is shut down".to_string()))?;

        resp_rx
            .await
            .map_err(|_| AppError::DownloadError("Queue response dropped".to_string()))?
    }

    /// Get current queue state snapshot (for UI display).
    pub async fn get_state(&self) -> Result<QueueStateSnapshot, AppError> {
        let (resp_tx, resp_rx) = oneshot::channel();

        self.tx
            .send(QueueCommand::GetState { resp: resp_tx })
            .await
            .map_err(|_| AppError::DownloadError("Queue is shut down".to_string()))?;

        resp_rx
            .await
            .map_err(|_| AppError::DownloadError("Queue response dropped".to_string()))?
    }

    /// Pause the active download.
    ///
    /// If the item is currently downloading, it will pause before the next page.
    /// If the item is not active, returns an error.
    pub async fn pause(&self, download_id: String) -> Result<(), AppError> {
        let (resp_tx, resp_rx) = oneshot::channel();

        self.tx
            .send(QueueCommand::Pause {
                id: download_id,
                resp: resp_tx,
            })
            .await
            .map_err(|_| AppError::DownloadError("Queue is shut down".to_string()))?;

        resp_rx
            .await
            .map_err(|_| AppError::DownloadError("Queue response dropped".to_string()))?
    }

    /// Resume a paused download.
    ///
    /// If the item is currently paused, it will resume downloading.
    /// If the item is not paused, returns an error.
    pub async fn resume(&self, download_id: String) -> Result<(), AppError> {
        let (resp_tx, resp_rx) = oneshot::channel();

        self.tx
            .send(QueueCommand::Resume {
                id: download_id,
                resp: resp_tx,
            })
            .await
            .map_err(|_| AppError::DownloadError("Queue is shut down".to_string()))?;

        resp_rx
            .await
            .map_err(|_| AppError::DownloadError("Queue response dropped".to_string()))?
    }

    /// Gracefully shut down the queue processor.
    ///
    /// If a download is active, it will be aborted. Pending commands are drained.
    pub async fn shutdown(&self) -> Result<(), AppError> {
        let _ = self.tx.send(QueueCommand::Shutdown).await;
        Ok(())
    }
}

// ── Commands ────────────────────────────────────────────────────────

/// Internal commands sent from `QueueHandle` to the process loop.
enum QueueCommand {
    Enqueue {
        id: String,
        url: String,
        format: String,
        save_path: String,
        compression: bool,
        resp: oneshot::Sender<Result<String, AppError>>,
    },
    Cancel {
        id: String,
        resp: oneshot::Sender<Result<(), AppError>>,
    },
    Pause {
        id: String,
        resp: oneshot::Sender<Result<(), AppError>>,
    },
    Resume {
        id: String,
        resp: oneshot::Sender<Result<(), AppError>>,
    },
    GetState {
        resp: oneshot::Sender<Result<QueueStateSnapshot, AppError>>,
    },
    Shutdown,
}

// ── State ───────────────────────────────────────────────────────────

/// Queue state tracked by the process loop.
struct QueueState {
    /// Currently active download (only one at a time).
    active: Option<ActiveDownload>,
    /// Items waiting to be processed (FIFO).
    pending: VecDeque<QueuedItem>,
    /// Completed items (kept for UI display, pruned at [`MAX_COMPLETED_HISTORY`]).
    completed: Vec<CompletedItem>,
}

/// An item currently being downloaded.
#[allow(dead_code)] // Fields are part of the data model, read via snapshot.
struct ActiveDownload {
    id: String,
    url: String,
    format: String,
    save_path: String,
    compression: bool,
    pause_token: PauseToken,
    started_at: Instant,
}

/// An item waiting in the queue.
struct QueuedItem {
    id: String,
    url: String,
    format: String,
    save_path: String,
    compression: bool,
}

/// Completion status for finished downloads.
#[derive(Debug, Clone, Serialize)]
pub enum CompletionStatus {
    Success,
    Error,
    Cancelled,
}

/// A completed download record.
#[allow(dead_code)] // Fields are part of the data model, read via snapshot.
struct CompletedItem {
    id: String,
    url: String,
    status: CompletionStatus,
    output_path: Option<String>,
    error: Option<String>,
    duration: Duration,
}

// ── Snapshot types (for frontend) ───────────────────────────────────

/// Queue state snapshot returned to frontend.
#[derive(Clone, Serialize)]
pub struct QueueStateSnapshot {
    pub active: Option<ActiveDownloadInfo>,
    pub pending: Vec<QueueItemInfo>,
    pub completed: Vec<CompletedItemInfo>,
}

#[derive(Clone, Serialize)]
pub struct ActiveDownloadInfo {
    pub id: String,
    pub url: String,
    pub format: String,
}

#[derive(Clone, Serialize)]
pub struct QueueItemInfo {
    pub id: String,
    pub url: String,
    pub format: String,
    pub position: usize,
}

#[derive(Clone, Serialize)]
pub struct CompletedItemInfo {
    pub id: String,
    pub url: String,
    pub status: String,
    pub output_path: Option<String>,
    pub error: Option<String>,
}

// ── Event payloads ──────────────────────────────────────────────────

#[derive(Clone, Serialize)]
struct ItemAddedPayload {
    id: String,
    url: String,
    position: usize,
}

#[derive(Clone, Serialize)]
struct ItemIdPayload {
    id: String,
}

#[derive(Clone, Serialize)]
struct ItemStatusPayload {
    id: String,
    status: String,
}

#[derive(Clone, Serialize)]
struct ItemCompletedPayload {
    id: String,
    #[serde(rename = "outputPath")]
    output_path: Option<String>,
}

#[derive(Clone, Serialize)]
struct ItemErrorPayload {
    id: String,
    error: String,
    retryable: bool,
}

// ── Process Loop ────────────────────────────────────────────────────

/// Main processing loop. Runs until `Shutdown` command or channel close.
///
/// Uses `tokio::select!` to multiplex between incoming commands and the
/// active download's completion. This ensures the queue is responsive to
/// new commands (enqueue, cancel, state queries) while a download is in
/// progress.
///
/// The `JoinHandle` for the active download is kept as a separate local
/// variable (not inside `QueueState`) so that `tokio::select!` can take
/// `&mut` on it, which is required by the `Future` trait.
async fn process_loop<R: tauri::Runtime>(
    mut rx: mpsc::Receiver<QueueCommand>,
    client: reqwest::Client,
    app_handle: tauri::AppHandle<R>,
) {
    tracing::info!("Queue processor started");
    let mut state = QueueState {
        active: None,
        pending: VecDeque::new(),
        completed: Vec::new(),
    };

    // JoinHandle stored separately so select! can get &mut to it.
    let mut active_handle: Option<tokio::task::JoinHandle<Result<String, AppError>>> = None;

    loop {
        if active_handle.is_some() {
            // Active download — select between commands and completion.
            tokio::select! {
                cmd = rx.recv() => {
                    match cmd {
                        Some(cmd) => {
                            // If cancel targets the active download, abort inline.
                            if let QueueCommand::Cancel { ref id, .. } = &cmd {
                                if let Some(ref active) = state.active {
                                    if active.id == *id {
                                        if let Some(h) = active_handle.take() {
                                            h.abort();
                                        }
                                    }
                                }
                            }
                            handle_command(&mut state, cmd, &client, &app_handle).await
                        }
                        None => {
                            tracing::info!("Channel closed, shutting down queue processor");
                            break;
                        }
                    }
                }
                result = active_handle.as_mut().unwrap() => {
                    handle_download_completion(&mut state, result, &client, &app_handle, &mut active_handle).await;
                    active_handle = None;
                }
            }
        } else {
            // No active download — try to start next, or wait for command.
            try_start_next(&mut state, &client, &app_handle, &mut active_handle);

            if active_handle.is_some() {
                // Just started a download, loop back to select on it.
                continue;
            }

            // Still idle — wait for a command.
            match rx.recv().await {
                Some(cmd) => handle_command(&mut state, cmd, &client, &app_handle).await,
                None => {
                    tracing::info!("Channel closed, shutting down queue processor");
                    break;
                }
            }
        }
    }

    // Graceful shutdown: abort active download if any.
    if let Some(handle) = active_handle.take() {
        tracing::info!("Aborting active download on shutdown");
        handle.abort();
    }

    tracing::info!("Queue processor shut down");
}

/// Handle an incoming command.
async fn handle_command<R: tauri::Runtime>(
    state: &mut QueueState,
    cmd: QueueCommand,
    client: &reqwest::Client,
    app_handle: &tauri::AppHandle<R>,
) {
    match cmd {
        QueueCommand::Enqueue {
            id,
            url,
            format,
            save_path,
            compression,
            resp,
        } => {
            let result = handle_enqueue(state, id, url, format, save_path, compression, app_handle);
            let _ = resp.send(result);
        }
        QueueCommand::Cancel { id, resp } => {
            let result = handle_cancel(state, &id, client, app_handle).await;
            let _ = resp.send(result);
        }
        QueueCommand::Pause { id, resp } => {
            let result = handle_pause(state, &id, app_handle);
            let _ = resp.send(result);
        }
        QueueCommand::Resume { id, resp } => {
            let result = handle_resume(state, &id, app_handle);
            let _ = resp.send(result);
        }
        QueueCommand::GetState { resp } => {
            let snapshot = build_snapshot(state);
            let _ = resp.send(Ok(snapshot));
        }
        QueueCommand::Shutdown => {
            // Handled by caller (break out of loop).
        }
    }
}

/// Handle an enqueue command.
fn handle_enqueue<R: tauri::Runtime>(
    state: &mut QueueState,
    id: String,
    url: String,
    format: String,
    save_path: String,
    compression: bool,
    app_handle: &tauri::AppHandle<R>,
) -> Result<String, AppError> {
    // Validate URL format.
    validator::validate_url(&url)?;
    validator::validate_format(&format)?;
    validator::validate_save_path(&save_path)?;

    // Check for duplicate URLs in active + pending.
    if let Some(ref active) = state.active {
        if active.url == url {
            return Err(AppError::DownloadError(
                "URL is already being downloaded".to_string(),
            ));
        }
    }
    if state.pending.iter().any(|item| item.url == url) {
        return Err(AppError::DownloadError(
            "URL is already in the queue".to_string(),
        ));
    }

    let position = state.pending.len() + if state.active.is_some() { 1 } else { 0 };
    state.pending.push_back(QueuedItem {
        id: id.clone(),
        url: url.clone(),
        format,
        save_path,
        compression,
    });

    tracing::info!(id = %id, url = %url, position = position, "Item enqueued");
    emit_item_added(app_handle, &id, &url, position);

    Ok(id)
}

/// Handle a cancel command.
///
/// For active downloads, this cleans up the state. The actual JoinHandle abort
/// is performed inline by the process loop before calling this function.
async fn handle_cancel<R: tauri::Runtime>(
    state: &mut QueueState,
    id: &str,
    _client: &reqwest::Client,
    app_handle: &tauri::AppHandle<R>,
) -> Result<(), AppError> {
    // Check if it is the active download.
    if let Some(ref active) = state.active {
        if active.id == id {
            let active = state.active.take().unwrap();
            tracing::info!(id = %id, "Cancelled active download");
            emit_item_cancelled(app_handle, id);

            state.completed.push(CompletedItem {
                id: active.id,
                url: active.url,
                status: CompletionStatus::Cancelled,
                output_path: None,
                error: None,
                duration: active.started_at.elapsed(),
            });
            prune_completed(&mut state.completed);
            return Ok(());
        }
    }

    // Check if it is in the pending queue.
    if let Some(pos) = state.pending.iter().position(|item| item.id == id) {
        state.pending.remove(pos);
        tracing::info!(id = %id, "Removed pending item from queue");
        return Ok(());
    }

    // Not found — idempotent success.
    tracing::debug!(id = %id, "Cancel requested for non-existent ID (no-op)");
    Ok(())
}

/// Handle a pause command.
///
/// Sets the pause token on the active download, causing it to wait
/// before processing the next page.
fn handle_pause<R: tauri::Runtime>(
    state: &mut QueueState,
    id: &str,
    app_handle: &tauri::AppHandle<R>,
) -> Result<(), AppError> {
    if let Some(ref active) = state.active {
        if active.id == id {
            active.pause_token.store(true, Ordering::Release);
            tracing::info!(id = %id, "Download paused");
            emit_status(app_handle, id, "paused");
            return Ok(());
        }
    }

    tracing::debug!(id = %id, "Pause requested for non-active download (no-op)");
    Err(AppError::DownloadError(
        "Cannot pause: download is not active".to_string(),
    ))
}

/// Handle a resume command.
///
/// Clears the pause token on the active download, allowing it to proceed.
fn handle_resume<R: tauri::Runtime>(
    state: &mut QueueState,
    id: &str,
    app_handle: &tauri::AppHandle<R>,
) -> Result<(), AppError> {
    if let Some(ref active) = state.active {
        if active.id == id {
            active.pause_token.store(false, Ordering::Release);
            tracing::info!(id = %id, "Download resumed");
            emit_status(app_handle, id, "downloading");
            return Ok(());
        }
    }

    tracing::debug!(id = %id, "Resume requested for non-active download (no-op)");
    Err(AppError::DownloadError(
        "Cannot resume: download is not active".to_string(),
    ))
}

/// Try to start the next queued item if idle.
///
/// If a download is started, the `JoinHandle` is stored in `active_handle`
/// and the metadata is stored in `state.active`.
fn try_start_next<R: tauri::Runtime>(
    state: &mut QueueState,
    client: &reqwest::Client,
    app_handle: &tauri::AppHandle<R>,
    active_handle: &mut Option<tokio::task::JoinHandle<Result<String, AppError>>>,
) {
    if state.active.is_some() {
        return;
    }

    // Pop next item from the front of the queue (FIFO).
    let item = match state.pending.pop_front() {
        Some(item) => item,
        None => {
            emit_idle(app_handle);
            return;
        }
    };

    // Validate URL before starting download.
    if let Err(e) = validator::validate_url(&item.url) {
        tracing::warn!(id = %item.id, url = %item.url, error = %e, "URL validation failed, skipping item");
        emit_item_error(app_handle, &item.id, &e.to_string(), false);

        state.completed.push(CompletedItem {
            id: item.id,
            url: item.url,
            status: CompletionStatus::Error,
            output_path: None,
            error: Some(e.to_string()),
            duration: Duration::ZERO,
        });
        prune_completed(&mut state.completed);

        // Try the next item.
        try_start_next(state, client, app_handle, active_handle);
        return;
    }

    tracing::info!(id = %item.id, url = %item.url, "Starting download");
    emit_item_started(app_handle, &item.id, &item.url);

    // Create pause token before spawning so it can be passed to the task.
    let pause_token: PauseToken = Arc::new(AtomicBool::new(false));

    let client = client.clone();
    let app_handle = app_handle.clone();
    let download_id = item.id.clone();
    let url = item.url.clone();
    let format = item.format.clone();
    let save_path = item.save_path.clone();
    let compression = item.compression;
    let task_pause_token = Arc::clone(&pause_token);

    let handle = tokio::spawn(async move {
        download_orchestrator::execute(
            &client,
            &download_id,
            &url,
            &format,
            &save_path,
            compression,
            &task_pause_token,
            &app_handle,
        )
        .await
    });

    *active_handle = Some(handle);
    state.active = Some(ActiveDownload {
        id: item.id,
        url: item.url,
        format: item.format,
        save_path: item.save_path,
        compression,
        pause_token,
        started_at: Instant::now(),
    });
}

/// Handle download completion (success or failure).
async fn handle_download_completion<R: tauri::Runtime>(
    state: &mut QueueState,
    result: Result<Result<String, AppError>, tokio::task::JoinError>,
    client: &reqwest::Client,
    app_handle: &tauri::AppHandle<R>,
    active_handle: &mut Option<tokio::task::JoinHandle<Result<String, AppError>>>,
) {
    let active = match state.active.take() {
        Some(a) => a,
        None => return,
    };

    let duration = active.started_at.elapsed();

    match result {
        Ok(Ok(output_path)) => {
            // Success.
            tracing::info!(
                id = %active.id,
                duration = ?duration,
                output_path = %output_path,
                "Download completed successfully"
            );
            emit_item_completed(app_handle, &active.id, Some(&output_path));

            state.completed.push(CompletedItem {
                id: active.id,
                url: active.url,
                status: CompletionStatus::Success,
                output_path: Some(output_path),
                error: None,
                duration,
            });
        }
        Ok(Err(e)) => {
            // Application error from the pipeline.
            tracing::error!(
                id = %active.id,
                error = %e,
                duration = ?duration,
                "Download failed"
            );

            let retryable = matches!(
                e,
                AppError::DownloadError(_) | AppError::NetworkError(_)
            );
            emit_item_error(app_handle, &active.id, &e.to_string(), retryable);

            state.completed.push(CompletedItem {
                id: active.id,
                url: active.url,
                status: CompletionStatus::Error,
                output_path: None,
                error: Some(e.to_string()),
                duration,
            });
        }
        Err(join_err) => {
            // Task was aborted or panicked.
            if join_err.is_cancelled() {
                tracing::info!(id = %active.id, "Download task was cancelled");
                // Cancellation is handled by handle_cancel, which already
                // moved the item to completed. Just skip here.
                return;
            }

            tracing::error!(
                id = %active.id,
                error = %join_err,
                "Download task panicked"
            );
            emit_item_error(
                app_handle,
                &active.id,
                &format!("Internal error: {}", join_err),
                false,
            );

            state.completed.push(CompletedItem {
                id: active.id,
                url: active.url,
                status: CompletionStatus::Error,
                output_path: None,
                error: Some(format!("Internal error: {}", join_err)),
                duration,
            });
        }
    }

    prune_completed(&mut state.completed);

    // Try to start the next item.
    try_start_next(state, client, app_handle, active_handle);
}

/// Build a snapshot of the current queue state.
fn build_snapshot(state: &QueueState) -> QueueStateSnapshot {
    let active = state.active.as_ref().map(|a| ActiveDownloadInfo {
        id: a.id.clone(),
        url: a.url.clone(),
        format: a.format.clone(),
    });

    let pending: Vec<QueueItemInfo> = state
        .pending
        .iter()
        .enumerate()
        .map(|(i, item)| QueueItemInfo {
            id: item.id.clone(),
            url: item.url.clone(),
            format: item.format.clone(),
            position: i,
        })
        .collect();

    let completed: Vec<CompletedItemInfo> = state
        .completed
        .iter()
        .map(|c| CompletedItemInfo {
            id: c.id.clone(),
            url: c.url.clone(),
            status: format!("{:?}", c.status),
            output_path: c.output_path.clone(),
            error: c.error.clone(),
        })
        .collect();

    QueueStateSnapshot {
        active,
        pending,
        completed,
    }
}

/// Prune completed history to [`MAX_COMPLETED_HISTORY`].
fn prune_completed(completed: &mut Vec<CompletedItem>) {
    if completed.len() > MAX_COMPLETED_HISTORY {
        let excess = completed.len() - MAX_COMPLETED_HISTORY;
        completed.drain(0..excess);
    }
}

// ── Event emitters (fire-and-forget) ────────────────────────────────

fn emit_item_added<R: tauri::Runtime>(app_handle: &tauri::AppHandle<R>, id: &str, url: &str, position: usize) {
    let payload = ItemAddedPayload {
        id: id.to_string(),
        url: url.to_string(),
        position,
    };
    if let Err(e) = app_handle.emit("queue:item-added", &payload) {
        tracing::warn!("Failed to emit queue:item-added: {}", e);
    }
}

fn emit_item_started<R: tauri::Runtime>(app_handle: &tauri::AppHandle<R>, id: &str, url: &str) {
    let payload = ItemAddedPayload {
        id: id.to_string(),
        url: url.to_string(),
        position: 0,
    };
    if let Err(e) = app_handle.emit("queue:item-started", &payload) {
        tracing::warn!("Failed to emit queue:item-started: {}", e);
    }
}

fn emit_item_completed<R: tauri::Runtime>(app_handle: &tauri::AppHandle<R>, id: &str, output_path: Option<&str>) {
    let payload = ItemCompletedPayload {
        id: id.to_string(),
        output_path: output_path.map(|s| s.to_string()),
    };
    if let Err(e) = app_handle.emit("queue:item-completed", &payload) {
        tracing::warn!("Failed to emit queue:item-completed: {}", e);
    }
}

fn emit_item_error<R: tauri::Runtime>(app_handle: &tauri::AppHandle<R>, id: &str, error: &str, retryable: bool) {
    let payload = ItemErrorPayload {
        id: id.to_string(),
        error: error.to_string(),
        retryable,
    };
    if let Err(e) = app_handle.emit("queue:item-error", &payload) {
        tracing::warn!("Failed to emit queue:item-error: {}", e);
    }
}

fn emit_item_cancelled<R: tauri::Runtime>(app_handle: &tauri::AppHandle<R>, id: &str) {
    let payload = ItemIdPayload {
        id: id.to_string(),
    };
    if let Err(e) = app_handle.emit("queue:item-cancelled", &payload) {
        tracing::warn!("Failed to emit queue:item-cancelled: {}", e);
    }
}

fn emit_status<R: tauri::Runtime>(app_handle: &tauri::AppHandle<R>, id: &str, status: &str) {
    let payload = ItemStatusPayload {
        id: id.to_string(),
        status: status.to_string(),
    };
    if let Err(e) = app_handle.emit("download:status", &payload) {
        tracing::warn!("Failed to emit download:status: {}", e);
    }
}

fn emit_idle<R: tauri::Runtime>(app_handle: &tauri::AppHandle<R>) {
    if let Err(e) = app_handle.emit("queue:idle", ()) {
        tracing::warn!("Failed to emit queue:idle: {}", e);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_handle() -> (QueueHandle, std::thread::JoinHandle<()>) {
        let client = reqwest::Client::new();
        let app = tauri::test::mock_app();
        let app_handle = app.handle().clone();
        start(client, app_handle)
    }

    // ── enqueue_returns_unique_id ───────────────────────────────────

    #[tokio::test]
    async fn enqueue_returns_unique_id() {
        let (handle, _task) = make_handle();

        let id1 = handle
            .enqueue(
                None,
                "https://anyflip.com/user/book1".to_string(),
                "pdf".to_string(),
                "/tmp".to_string(),
                false,
            )
            .await
            .unwrap();

        let id2 = handle
            .enqueue(
                None,
                "https://anyflip.com/user/book2".to_string(),
                "pdf".to_string(),
                "/tmp".to_string(),
                false,
            )
            .await
            .unwrap();

        assert_ne!(id1, id2);
        let _ = handle.shutdown().await;
    }

    // ── enqueue_rejects_duplicate_url ───────────────────────────────

    #[tokio::test]
    async fn enqueue_rejects_duplicate_url() {
        let (handle, _task) = make_handle();

        let _ = handle
            .enqueue(
                None,
                "https://anyflip.com/user/book".to_string(),
                "pdf".to_string(),
                "/tmp".to_string(),
                false,
            )
            .await
            .unwrap();

        let result = handle
            .enqueue(
                None,
                "https://anyflip.com/user/book".to_string(),
                "pdf".to_string(),
                "/tmp".to_string(),
                false,
            )
            .await;

        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(
            err.to_string().contains("already"),
            "Expected duplicate error, got: {}",
            err
        );

        let _ = handle.shutdown().await;
    }

    // ── enqueue_validates_url ───────────────────────────────────────

    #[tokio::test]
    async fn enqueue_validates_url() {
        let (handle, _task) = make_handle();

        let result = handle
            .enqueue(
                None,
                "https://google.com/not-anyflip".to_string(),
                "pdf".to_string(),
                "/tmp".to_string(),
                false,
            )
            .await;

        assert!(result.is_err());
        let _ = handle.shutdown().await;
    }

    // ── enqueue_validates_format ────────────────────────────────────

    #[tokio::test]
    async fn enqueue_validates_format() {
        let (handle, _task) = make_handle();

        let result = handle
            .enqueue(
                None,
                "https://anyflip.com/user/book".to_string(),
                "docx".to_string(),
                "/tmp".to_string(),
                false,
            )
            .await;

        assert!(result.is_err());
        let _ = handle.shutdown().await;
    }

    // ── cancel_nonexistent_is_ok ────────────────────────────────────

    #[tokio::test]
    async fn cancel_nonexistent_is_ok() {
        let (handle, _task) = make_handle();

        let result = handle.cancel("nonexistent-id".to_string()).await;
        assert!(result.is_ok());

        let _ = handle.shutdown().await;
    }

    // ── cancel_pending_removes_from_queue ───────────────────────────

    #[tokio::test]
    async fn cancel_pending_removes_from_queue() {
        let (handle, _task) = make_handle();

        // Enqueue a valid item (first item will start downloading since we
        // can't easily block it in this test context, but we can enqueue
        // a second and cancel it while it's still pending).
        let _id1 = handle
            .enqueue(
                None,
                "https://anyflip.com/user/book1".to_string(),
                "pdf".to_string(),
                "/tmp".to_string(),
                false,
            )
            .await
            .unwrap();

        let id2 = handle
            .enqueue(
                None,
                "https://anyflip.com/user/book2".to_string(),
                "pdf".to_string(),
                "/tmp".to_string(),
                false,
            )
            .await
            .unwrap();

        // Cancel the second item (which may be pending or active).
        let result = handle.cancel(id2.clone()).await;
        assert!(result.is_ok());

        // Verify it's not in pending anymore.
        let state = handle.get_state().await.unwrap();
        assert!(
            !state.pending.iter().any(|item| item.id == id2),
            "Cancelled item should not be in pending"
        );

        let _ = handle.shutdown().await;
    }

    // ── get_state_returns_snapshot ──────────────────────────────────

    #[tokio::test]
    async fn get_state_returns_snapshot() {
        let (handle, _task) = make_handle();

        let _ = handle
            .enqueue(
                None,
                "https://anyflip.com/user/book1".to_string(),
                "pdf".to_string(),
                "/tmp".to_string(),
                false,
            )
            .await
            .unwrap();

        let _ = handle
            .enqueue(
                None,
                "https://anyflip.com/user/book2".to_string(),
                "epub".to_string(),
                "/tmp".to_string(),
                false,
            )
            .await
            .unwrap();

        // Give the process loop a moment to process.
        tokio::time::sleep(Duration::from_millis(50)).await;

        let state = handle.get_state().await.unwrap();

        // At least one item should be active or completed, and at least
        // one should be pending (or both completed if downloads were fast).
        let total = state.pending.len()
            + state.completed.len()
            + if state.active.is_some() { 1 } else { 0 };
        assert!(total >= 2, "Expected at least 2 items in queue state");

        let _ = handle.shutdown().await;
    }

    // ── shutdown_aborts_active ──────────────────────────────────────

    #[tokio::test]
    async fn shutdown_aborts_active() {
        let (handle, task) = make_handle();

        // Enqueue an item (download will start and likely fail since the
        // URL is not real, but that's fine for testing shutdown).
        let _ = handle
            .enqueue(
                None,
                "https://anyflip.com/user/book".to_string(),
                "pdf".to_string(),
                "/tmp".to_string(),
                false,
            )
            .await
            .unwrap();

        // Shutdown.
        handle.shutdown().await.unwrap();

        // Task should complete within a reasonable time.
        // task is a std::thread::JoinHandle, not a tokio future, so join directly.
        let result = task.join();
        assert!(result.is_ok(), "Process loop thread panicked");
    }

    // ── error_moves_to_completed ────────────────────────────────────

    #[tokio::test]
    async fn error_moves_to_completed() {
        let (handle, _task) = make_handle();

        // Enqueue a URL that will fail during metadata fetch.
        let _ = handle
            .enqueue(
                None,
                "https://anyflip.com/user/book".to_string(),
                "pdf".to_string(),
                "/tmp/nonexistent_dir".to_string(),
                false,
            )
            .await
            .unwrap();

        // Wait for the download to fail (it will try to fetch metadata
        // from a non-existent Anyflip book).
        tokio::time::sleep(Duration::from_secs(3)).await;

        let state = handle.get_state().await.unwrap();
        assert!(
            !state.completed.is_empty(),
            "Expected at least one completed item after download failure"
        );

        let completed = &state.completed[0];
        assert_eq!(completed.status, "Error");

        let _ = handle.shutdown().await;
    }

    // ── next_item_starts_after_completion ────────────────────────────

    #[tokio::test]
    async fn next_item_starts_after_completion() {
        let (handle, _task) = make_handle();

        // Enqueue two items.
        let _id1 = handle
            .enqueue(
                None,
                "https://anyflip.com/user/book1".to_string(),
                "pdf".to_string(),
                "/tmp".to_string(),
                false,
            )
            .await
            .unwrap();

        let _id2 = handle
            .enqueue(
                None,
                "https://anyflip.com/user/book2".to_string(),
                "pdf".to_string(),
                "/tmp".to_string(),
                false,
            )
            .await
            .unwrap();

        // Wait for both to process (they will fail, but should still
        // transition through the queue).
        tokio::time::sleep(Duration::from_secs(6)).await;

        let state = handle.get_state().await.unwrap();
        assert!(
            state.completed.len() >= 2,
            "Expected at least 2 completed items, got {}",
            state.completed.len()
        );

        let _ = handle.shutdown().await;
    }
}
