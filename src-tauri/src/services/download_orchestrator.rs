use crate::data::config::Config;
use crate::data::output::OutputFile;
use crate::error::AppError;
use crate::services::metadata_fetcher;
use crate::services::page_downloader;
use crate::services::url_parser;
use crate::services::validator;
use serde::Serialize;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{LazyLock, Mutex};
use tauri::Emitter;
use tokio::sync::oneshot;

/// Average page size estimate for PDF output (150 KB/page).
const ESTIMATED_PAGE_SIZE_PDF: u64 = 150 * 1024;

/// Average page size estimate for EPUB output (120 KB/page).
const ESTIMATED_PAGE_SIZE_EPUB: u64 = 120 * 1024;

/// Event payload for status transitions.
#[derive(Clone, Serialize)]
struct StatusPayload {
    id: String,
    status: String,
}

/// Event payload for completion.
#[derive(Clone, Serialize)]
struct CompletePayload {
    id: String,
    #[serde(rename = "outputPath")]
    output_path: String,
}

/// Event payload for errors.
#[derive(Clone, Serialize)]
struct ErrorPayload {
    id: String,
    error: String,
    code: String,
    retryable: bool,
}

/// Event payload for estimated file size.
#[derive(Clone, Serialize)]
struct EstimatedSizePayload {
    id: String,
    #[serde(rename = "estimatedBytes")]
    estimated_bytes: u64,
    #[serde(rename = "estimatedFormatted")]
    estimated_formatted: String,
}

/// Event payload for password-required notification.
#[derive(Clone, Serialize)]
struct PasswordRequiredPayload {
    id: String,
    url: String,
}

/// Shared state for pending password submissions.
///
/// Maps download ID to a oneshot sender. When the orchestrator detects a 403
/// (password-protected document), it creates a oneshot channel, stores the
/// sender here, emits `download:password_required`, and awaits the receiver.
/// When the user submits a password via `submit_password`, the sender is taken
/// and the password is delivered, unblocking the orchestrator.
static PENDING_PASSWORDS: LazyLock<Mutex<HashMap<String, oneshot::Sender<String>>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

/// Execute the full download pipeline for a single Anyflip document.
///
/// This is the primary entry point for downloading documents. It coordinates
/// all seven phases: validation, metadata fetching, page downloading, format
/// conversion, output verification, and cleanup.
///
/// # Pipeline Phases
///
/// 1. **Validate** -- URL, format, save path
/// 2. **Metadata** -- Fetch config.js from Anyflip
/// 3. **Prepare** -- Create temp directory
/// 4. **Download** -- Download all page JPGs (concurrent, with resume)
/// 5. **Convert** -- Generate PDF or EPUB from JPGs
/// 6. **Verify** -- Confirm output file exists and is valid
/// 7. **Cleanup** -- Delete temp directory (success only)
///
/// # Error Recovery
///
/// On failure, the temp directory is preserved. Re-calling `execute` with the
/// same URL will resume from cached pages (only missing pages are downloaded).
///
/// # Events Emitted
///
/// - `download:status` -- Phase transitions (downloading, converting)
/// - `download:progress` -- Per-page progress (from PageDownloader)
/// - `download:complete` -- Success with output path
/// - `download:error` -- Failure with error details
///
/// # Errors
/// - `AppError::InvalidUrl` -- URL validation failed.
/// - `AppError::MetadataError` -- config.js fetch/parse failed.
/// - `AppError::DownloadError` -- page download failed (retryable).
/// - `AppError::ConversionError` -- PDF/EPUB generation failed.
/// - `AppError::FileSystemError` -- file I/O failed.
pub async fn execute<R: tauri::Runtime>(
    client: &reqwest::Client,
    download_id: &str,
    url: &str,
    format: &str,
    save_path: &str,
    _compression: bool,
    pause_token: &page_downloader::PauseToken,
    app_handle: &tauri::AppHandle<R>,
) -> Result<String, AppError> {
    tracing::info!(
        download_id = %download_id,
        url = %url,
        format = %format,
        "Starting download pipeline"
    );

    // ── Phase 1: Validate ───────────────────────────────────────────
    validator::validate_url(url)?;
    validator::validate_format(format)?;
    validator::validate_save_path(save_path)?;

    let parsed_url = url_parser::parse(url)?;

    // ── Phase 2: Fetch Metadata (with password retry) ──────────────
    let metadata = match metadata_fetcher::fetch(client, &parsed_url).await {
        Ok(m) => m,
        Err(AppError::PasswordRequired(_)) => {
            tracing::info!(download_id = %download_id, "Metadata fetch requires password");
            let password = wait_for_password(download_id, url, app_handle).await?;
            // Retry metadata fetch with password as query param.
            metadata_fetcher::fetch_with_password(client, &parsed_url, &password).await?
        }
        Err(e) => return Err(e),
    };
    validator::validate_page_count(metadata.page_count)?;

    tracing::info!(
        title = %metadata.title,
        page_count = metadata.page_count,
        "Metadata fetched"
    );

    // ── Estimated File Size ─────────────────────────────────────────
    let estimated_bytes = estimate_file_size(metadata.page_count, format);
    emit_estimated_size(app_handle, download_id, estimated_bytes);

    // ── Phase 3: Prepare Temp Dir ───────────────────────────────────
    let temp_dir = temp_base_dir().join(parsed_url.temp_dir_name());
    tokio::fs::create_dir_all(&temp_dir).await.map_err(|e| {
        AppError::FileSystemError(format!(
            "Failed to create temp directory {}: {}",
            temp_dir.display(),
            e
        ))
    })?;

    tracing::info!(temp_dir = %temp_dir.display(), "Temp directory ready");

    // ── Phase 4: Download Pages (with password retry) ──────────────
    emit_status(app_handle, download_id, "downloading");

    match page_downloader::download_all(
        client,
        &parsed_url,
        &metadata,
        &temp_dir,
        app_handle,
        download_id,
        pause_token,
    )
    .await
    {
        Ok(()) => {}
        Err(AppError::PasswordRequired(_)) => {
            tracing::info!(download_id = %download_id, "Page download requires password");
            let password = wait_for_password(download_id, url, app_handle).await?;
            // Retry page download with password as query param.
            page_downloader::download_all_with_password(
                client,
                &parsed_url,
                &metadata,
                &temp_dir,
                app_handle,
                download_id,
                pause_token,
                &password,
            )
            .await?;
        }
        Err(e) => {
            tracing::error!(
                download_id = %download_id,
                phase = "download",
                error = %e,
                "Pipeline failed during page download"
            );
            emit_error(app_handle, download_id, &e.to_string(), "DOWNLOAD_FAILED", true);
            // Preserve temp dir for resume.
            return Err(e);
        }
    }

    // ── Phase 5: Convert ────────────────────────────────────────────
    emit_status(app_handle, download_id, "converting");

    let config = Config::load();
    let author = metadata.author.clone().unwrap_or_default();
    let output_filename = format!(
        "{}.{}",
        OutputFile::apply_naming_pattern(
            &config.file_naming_pattern,
            &metadata.title,
            &author,
            metadata.page_count,
            format,
        ),
        format
    );
    let save_dir = Path::new(save_path);
    let output_path = save_dir.join(&output_filename);

    // Ensure save directory exists.
    if let Some(parent) = output_path.parent() {
        tokio::fs::create_dir_all(parent).await.map_err(|e| {
            AppError::FileSystemError(format!(
                "Failed to create output directory {}: {}",
                parent.display(),
                e
            ))
        })?;
    }

    let convert_result = match format {
        "pdf" => {
            crate::services::pdf_generator::generate(
                &temp_dir,
                metadata.page_count,
                &output_path,
                Some(&metadata),
            )
            .await
        }
        "epub" => {
            crate::services::epub_generator::generate(
                &temp_dir,
                metadata.page_count,
                &output_path,
                Some(&metadata),
            )
            .await
        }
        _ => {
            // Should not reach here due to Phase 1 validation.
            let e = AppError::ConversionError(format!("Unsupported format: {}", format));
            emit_error(
                app_handle,
                download_id,
                &e.to_string(),
                "UNSUPPORTED_FORMAT",
                false,
            );
            return Err(e);
        }
    };

    if let Err(e) = convert_result {
        tracing::error!(
            download_id = %download_id,
            phase = "convert",
            format = %format,
            error = %e,
            "Pipeline failed during conversion"
        );
        emit_error(app_handle, download_id, &e.to_string(), "CONVERSION_FAILED", false);
        // Preserve temp dir for retry.
        return Err(e);
    }

    // ── Phase 6: Verify Output ──────────────────────────────────────
    if !output_path.exists() {
        let e = AppError::ConversionError(format!(
            "Output file does not exist after conversion: {}",
            output_path.display()
        ));
        tracing::error!(download_id = %download_id, phase = "verify", error = %e);
        emit_error(app_handle, download_id, &e.to_string(), "VERIFY_FAILED", false);
        return Err(e);
    }

    let file_size = tokio::fs::metadata(&output_path)
        .await
        .map(|m| m.len())
        .unwrap_or(0);

    if file_size == 0 {
        let e = AppError::ConversionError(format!(
            "Output file is empty: {}",
            output_path.display()
        ));
        tracing::error!(download_id = %download_id, phase = "verify", error = %e);
        emit_error(app_handle, download_id, &e.to_string(), "VERIFY_FAILED", false);
        return Err(e);
    }

    tracing::info!(
        output_path = %output_path.display(),
        file_size = file_size,
        "Output file verified"
    );

    // ── Phase 7: Cleanup ────────────────────────────────────────────
    if let Err(e) = tokio::fs::remove_dir_all(&temp_dir).await {
        // Non-fatal: log warning but still return success.
        tracing::warn!(
            temp_dir = %temp_dir.display(),
            error = %e,
            "Failed to clean up temp directory (non-fatal)"
        );
    } else {
        tracing::info!(temp_dir = %temp_dir.display(), "Temp directory cleaned up");
    }

    // ── Emit Completion ─────────────────────────────────────────────
    emit_complete(app_handle, download_id, &output_path);

    tracing::info!(
        download_id = %download_id,
        output_path = %output_path.display(),
        "Download pipeline complete"
    );

    Ok(output_path.to_string_lossy().to_string())
}

/// Return the base temp directory: `{system_temp}/anyflipdl`.
fn temp_base_dir() -> PathBuf {
    std::env::temp_dir().join("anyflipdl")
}

/// Emit a `download:status` event. Fire-and-forget.
fn emit_status<R: tauri::Runtime>(app_handle: &tauri::AppHandle<R>, download_id: &str, status: &str) {
    let payload = StatusPayload {
        id: download_id.to_string(),
        status: status.to_string(),
    };

    if let Err(e) = app_handle.emit("download:status", &payload) {
        tracing::warn!("Failed to emit download:status event: {}", e);
    }
}

/// Emit a `download:complete` event. Fire-and-forget.
fn emit_complete<R: tauri::Runtime>(app_handle: &tauri::AppHandle<R>, download_id: &str, output_path: &Path) {
    let payload = CompletePayload {
        id: download_id.to_string(),
        output_path: output_path.to_string_lossy().to_string(),
    };

    if let Err(e) = app_handle.emit("download:complete", &payload) {
        tracing::warn!("Failed to emit download:complete event: {}", e);
    }
}

/// Emit a `download:error` event. Fire-and-forget.
fn emit_error<R: tauri::Runtime>(
    app_handle: &tauri::AppHandle<R>,
    download_id: &str,
    error: &str,
    code: &str,
    retryable: bool,
) {
    let payload = ErrorPayload {
        id: download_id.to_string(),
        error: error.to_string(),
        code: code.to_string(),
        retryable,
    };

    if let Err(e) = app_handle.emit("download:error", &payload) {
        tracing::warn!("Failed to emit download:error event: {}", e);
    }
}

/// Wait for the user to submit a password for a password-protected document.
///
/// Creates a oneshot channel, stores the sender in [`PENDING_PASSWORDS`], emits
/// a `download:password_required` event, and blocks until the password arrives
/// or the sender is dropped (e.g., download cancelled).
///
/// # Errors
/// - `AppError::DownloadError` if the password channel was dropped (cancelled).
async fn wait_for_password<R: tauri::Runtime>(
    download_id: &str,
    url: &str,
    app_handle: &tauri::AppHandle<R>,
) -> Result<String, AppError> {
    let (tx, rx) = oneshot::channel::<String>();

    // Register the sender so `submit_password` can find it.
    {
        let mut pending = PENDING_PASSWORDS.lock().map_err(|e| {
            AppError::DownloadError(format!("Password state lock poisoned: {}", e))
        })?;
        pending.insert(download_id.to_string(), tx);
    }

    // Notify the frontend.
    emit_password_required(app_handle, download_id, url);

    // Block until password arrives or channel is dropped.
    rx.await.map_err(|_| {
        // Clean up if somehow still present.
        cleanup_pending_password(download_id);
        AppError::DownloadError("Password submission was cancelled".to_string())
    })
}

/// Submit a password for a pending password-protected download.
///
/// Called by the `submit_password` Tauri command. Finds the pending oneshot
/// sender for `download_id` and sends the password through it, unblocking
/// the download orchestrator.
///
/// # Errors
/// - `AppError::DownloadError` if no pending password request exists for this ID
///   (expired, already submitted, or invalid ID).
pub fn submit_password_to_pending(download_id: &str, password: String) -> Result<(), AppError> {
    let tx = {
        let mut pending = PENDING_PASSWORDS.lock().map_err(|e| {
            AppError::DownloadError(format!("Password state lock poisoned: {}", e))
        })?;
        pending.remove(download_id)
    };

    match tx {
        Some(tx) => tx.send(password).map_err(|_| {
            AppError::DownloadError("Password receiver was dropped".to_string())
        }),
        None => Err(AppError::DownloadError(format!(
            "No pending password request for download '{}'",
            download_id
        ))),
    }
}

/// Clean up a pending password entry (best-effort).
fn cleanup_pending_password(download_id: &str) {
    if let Ok(mut pending) = PENDING_PASSWORDS.lock() {
        pending.remove(download_id);
    }
}

/// Emit a `download:password_required` event. Fire-and-forget.
fn emit_password_required<R: tauri::Runtime>(
    app_handle: &tauri::AppHandle<R>,
    download_id: &str,
    url: &str,
) {
    let payload = PasswordRequiredPayload {
        id: download_id.to_string(),
        url: url.to_string(),
    };

    if let Err(e) = app_handle.emit("download:password_required", &payload) {
        tracing::warn!("Failed to emit download:password_required event: {}", e);
    }
}

/// Estimate output file size based on page count and format.
///
/// Uses average observed page sizes: 150 KB/page for PDF, 120 KB/page for EPUB.
fn estimate_file_size(page_count: usize, format: &str) -> u64 {
    let per_page = match format {
        "epub" => ESTIMATED_PAGE_SIZE_EPUB,
        _ => ESTIMATED_PAGE_SIZE_PDF,
    };
    page_count as u64 * per_page
}

/// Emit a `download:estimated_size` event. Fire-and-forget.
fn emit_estimated_size<R: tauri::Runtime>(
    app_handle: &tauri::AppHandle<R>,
    download_id: &str,
    estimated_bytes: u64,
) {
    let payload = EstimatedSizePayload {
        id: download_id.to_string(),
        estimated_bytes,
        estimated_formatted: format_bytes(estimated_bytes),
    };

    if let Err(e) = app_handle.emit("download:estimated_size", &payload) {
        tracing::warn!("Failed to emit download:estimated_size event: {}", e);
    }
}

/// Format a byte count into a human-readable string (e.g., "12.3 MB").
fn format_bytes(bytes: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = 1024 * KB;
    const GB: u64 = 1024 * MB;

    if bytes >= GB {
        format!("{:.1} GB", bytes as f64 / GB as f64)
    } else if bytes >= MB {
        format!("{:.1} MB", bytes as f64 / MB as f64)
    } else if bytes >= KB {
        format!("{:.1} KB", bytes as f64 / KB as f64)
    } else {
        format!("{} B", bytes)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    use std::sync::atomic::AtomicBool;

    // ── Validate URL first ──────────────────────────────────────────

    #[tokio::test]
    async fn execute_validates_url_first() {
        let client = reqwest::Client::new();
        let app = tauri::test::mock_app();
        let app_handle = app.handle().clone();
        let pause_token: page_downloader::PauseToken = Arc::new(AtomicBool::new(false));

        let result = execute(
            &client,
            "test-1",
            "https://google.com/not-anyflip",
            "pdf",
            "/tmp",
            false,
            &pause_token,
            &app_handle,
        )
        .await;

        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(
            err.to_string().contains("Not an Anyflip URL"),
            "Expected InvalidUrl, got: {}",
            err
        );
    }

    // ── Validate format ─────────────────────────────────────────────

    #[tokio::test]
    async fn execute_validates_format() {
        let client = reqwest::Client::new();
        let app = tauri::test::mock_app();
        let app_handle = app.handle().clone();
        let pause_token: page_downloader::PauseToken = Arc::new(AtomicBool::new(false));

        let result = execute(
            &client,
            "test-2",
            "https://anyflip.com/user/book",
            "docx",
            "/tmp",
            false,
            &pause_token,
            &app_handle,
        )
        .await;

        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(
            err.to_string().contains("must be 'pdf' or 'epub'"),
            "Expected format validation error, got: {}",
            err
        );
    }

    // ── Validate save path ──────────────────────────────────────────

    #[tokio::test]
    async fn execute_validates_save_path() {
        let client = reqwest::Client::new();
        let app = tauri::test::mock_app();
        let app_handle = app.handle().clone();
        let pause_token: page_downloader::PauseToken = Arc::new(AtomicBool::new(false));

        let result = execute(
            &client,
            "test-3",
            "https://anyflip.com/user/book",
            "pdf",
            "",
            false,
            &pause_token,
            &app_handle,
        )
        .await;

        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(
            err.to_string().contains("Save path cannot be empty"),
            "Expected save path error, got: {}",
            err
        );
    }

    // ── sanitize_filename edge cases ────────────────────────────────

    #[test]
    fn sanitize_filename_edge_cases() {
        // These test OutputFile::sanitize_filename which is used in the pipeline.
        assert_eq!(
            OutputFile::sanitize_filename("My Book: Vol/1?"),
            "My Book Vol 1"
        );
        assert_eq!(
            OutputFile::sanitize_filename(r#"a\b:c*d"e<f>g|h?i"#),
            "a b c d e f g h i"
        );
        assert_eq!(OutputFile::sanitize_filename(""), "untitled");
        assert_eq!(OutputFile::sanitize_filename("   "), "untitled");
    }
}
