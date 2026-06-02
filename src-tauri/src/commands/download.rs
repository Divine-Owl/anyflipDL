use crate::error::AppError;
use crate::services::download_orchestrator;
use crate::services::queue_manager::QueueHandle;

/// Enqueue a document for download processing.
///
/// If `id` is provided, the queue will use that as the download ID (so the
/// frontend and backend share the same identifier for event correlation).
/// If `id` is `None`, the backend generates a UUID.
///
/// Returns the download ID used by the queue.
#[tauri::command]
pub async fn download_document(
    id: Option<String>,
    url: String,
    format: String,
    save_path: String,
    compression: Option<bool>,
    queue: tauri::State<'_, QueueHandle>,
) -> Result<String, AppError> {
    queue
        .enqueue(id, url, format, save_path, compression.unwrap_or(false))
        .await
}

/// Cancel a queued or active download by ID.
///
/// If the item is currently downloading, the download task is aborted.
/// If the item is pending, it is removed from the queue.
/// If the item is not found, returns `Ok(())` (idempotent).
#[tauri::command]
pub async fn cancel_download(
    download_id: String,
    queue: tauri::State<'_, QueueHandle>,
) -> Result<(), AppError> {
    queue.cancel(download_id).await
}

/// Submit a password for a password-protected download.
///
/// When a download encounters a 403 (password-protected document), the backend
/// emits a `download:password_required` event and pauses. The frontend collects
/// the password from the user and calls this command to resume the download.
///
/// # Arguments
/// - `download_id`: The ID of the download waiting for a password.
/// - `password`: The document access password entered by the user.
///
/// # Errors
/// - `AppError::DownloadError` if no pending password request exists for this ID.
#[tauri::command]
pub async fn submit_password(
    download_id: String,
    password: String,
) -> Result<(), AppError> {
    download_orchestrator::submit_password_to_pending(&download_id, password)
}

/// Pause the active download.
///
/// The download will pause before processing the next page. Use
/// `resume_download` to continue.
#[tauri::command]
pub async fn pause_download(
    download_id: String,
    queue: tauri::State<'_, QueueHandle>,
) -> Result<(), AppError> {
    queue.pause(download_id).await
}

/// Resume a paused download.
///
/// Continues downloading from where it left off.
#[tauri::command]
pub async fn resume_download(
    download_id: String,
    queue: tauri::State<'_, QueueHandle>,
) -> Result<(), AppError> {
    queue.resume(download_id).await
}
