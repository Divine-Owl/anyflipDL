use crate::error::AppError;
use crate::models::DocumentMetadata;
use crate::services::url_parser::ParsedUrl;
use serde::Serialize;
use std::path::Path;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;
use tauri::Emitter;
use tokio::sync::Semaphore;

/// Maximum concurrent page downloads.
const MAX_CONCURRENT: usize = 4;

/// HTTP request timeout per page.
const PAGE_TIMEOUT: Duration = Duration::from_secs(30);

/// Maximum retries per page on transient failure.
const MAX_RETRIES: u32 = 3;

/// How long to sleep between pause-check polls.
const PAUSE_POLL_INTERVAL: Duration = Duration::from_millis(200);

/// Shared flag for pausing a download.
///
/// When `true`, the page download loop will sleep-wait before each page
/// until the flag is set back to `false`.
pub type PauseToken = Arc<AtomicBool>;

/// Event payload for download progress updates.
#[derive(Clone, Serialize)]
struct ProgressPayload {
    id: String,
    progress: u8,
    #[serde(rename = "currentPage")]
    current_page: usize,
    #[serde(rename = "totalPages")]
    total_pages: usize,
}

/// Download all page JPGs for a document.
///
/// Downloads up to [`MAX_CONCURRENT`] pages in parallel using a semaphore-bounded
/// `JoinSet`. Each page is retried up to [`MAX_RETRIES`] times on transient failures.
/// Pages that already exist in `temp_dir` with size > 0 are skipped (resume-from-cache).
///
/// Emits `download:progress` after each page completes and `download:error` when a
/// page fails permanently after retries.
///
/// # Errors
/// - `AppError::DownloadError` if any page fails after all retries.
/// - `AppError::FileSystemError` if the temp directory cannot be written to.
pub async fn download_all<R: tauri::Runtime>(
    client: &reqwest::Client,
    parsed_url: &ParsedUrl,
    metadata: &DocumentMetadata,
    temp_dir: &Path,
    app_handle: &tauri::AppHandle<R>,
    download_id: &str,
    pause_token: &PauseToken,
) -> Result<(), AppError> {
    let page_count = metadata.page_count;
    if page_count == 0 {
        tracing::info!("No pages to download, returning early");
        return Ok(());
    }

    // Ensure temp directory exists.
    tokio::fs::create_dir_all(temp_dir).await.map_err(|e| {
        AppError::FileSystemError(format!(
            "Failed to create temp directory {}: {}",
            temp_dir.display(),
            e
        ))
    })?;

    tracing::info!(
        download_id = %download_id,
        page_count = page_count,
        "Starting page download"
    );

    let semaphore = Arc::new(Semaphore::new(MAX_CONCURRENT));
    let mut join_set = tokio::task::JoinSet::new();

    for page in 1..=page_count {
        // Check pause flag — wait until unpaused before proceeding.
        wait_if_paused(pause_token).await;

        // Resume-from-cache: skip pages with existing non-empty JPGs.
        if check_cache(temp_dir, page) {
            tracing::debug!(page = page, "Page cached, skipping");
            emit_progress(app_handle, download_id, page, page_count);
            continue;
        }

        // Delete zero-byte files (corrupt partial downloads).
        let page_path = temp_dir.join(format!("{}.webp", page));
        if page_path.exists() {
            let _ = tokio::fs::remove_file(&page_path).await;
        }

        let url = page_url(parsed_url, metadata, page);
        let client = client.clone();
        let semaphore = Arc::clone(&semaphore);
        let temp_dir = temp_dir.to_path_buf();
        let download_id = download_id.to_string();
        let app_handle = app_handle.clone();
        let referer = parsed_url.base_url.clone();

        join_set.spawn(async move {
            let permit = semaphore.acquire_owned().await.map_err(|e| {
                AppError::DownloadError(format!("Semaphore closed: {}", e))
            })?;

            let page_path = temp_dir.join(format!("{}.webp", page));
            let result = download_page_with_retry(
                &client,
                &url,
                &page_path,
                &download_id,
                page,
                page_count,
                &referer,
                &app_handle,
            )
            .await;

            drop(permit);
            result
        });
    }

    // Collect results — return the first error, if any.
    let mut first_error: Option<AppError> = None;
    while let Some(result) = join_set.join_next().await {
        match result {
            Ok(Ok(())) => {}
            Ok(Err(e)) => {
                if first_error.is_some() {
                    tracing::error!("Additional page download error: {}", e);
                } else {
                    first_error = Some(e);
                }
            }
            Err(join_err) => {
                let e = AppError::DownloadError(format!("Task join error: {}", join_err));
                if first_error.is_some() {
                    tracing::error!("Additional task join error: {}", join_err);
                } else {
                    first_error = Some(e);
                }
            }
        }
    }

    match first_error {
        Some(e) => Err(e),
        None => {
            tracing::info!(
                download_id = %download_id,
                "All {} pages downloaded successfully",
                page_count
            );
            Ok(())
        }
    }
}

/// Download a single page with retry logic.
///
/// Attempts the download up to [`MAX_RETRIES`] + 1 times. Emits `download:error`
/// on the final failure.
async fn download_page_with_retry<R: tauri::Runtime>(
    client: &reqwest::Client,
    url: &str,
    page_path: &Path,
    download_id: &str,
    page: usize,
    total_pages: usize,
    referer: &str,
    app_handle: &tauri::AppHandle<R>,
) -> Result<(), AppError> {
    let mut last_err: Option<AppError> = None;

    for attempt in 0..=MAX_RETRIES {
        match download_page(client, url, page_path, referer).await {
            Ok(()) => {
                emit_progress(app_handle, download_id, page, total_pages);
                return Ok(());
            }
            Err(e) => {
                if attempt < MAX_RETRIES {
                    tracing::warn!(
                        page = page,
                        attempt = attempt + 1,
                        max_retries = MAX_RETRIES,
                        error = %e,
                        "Page download failed, retrying"
                    );
                }
                last_err = Some(e);
            }
        }
    }

    // All retries exhausted.
    let error = last_err.unwrap_or_else(|| {
        AppError::DownloadError(format!("Page {} failed for unknown reason", page))
    });

    tracing::error!(page = page, error = %error, "Page download failed after all retries");
    // Do NOT emit download:error here. The error propagates to the orchestrator,
    // which emits it at the pipeline level. Emitting here would cause a double
    // error event, triggering processQueue() twice on the frontend.
    Err(error)
}

/// Download a single page JPG: HTTP GET, validate JPEG header, write to disk.
///
/// Sets the `Referer` header to the book URL, which is required by Anyflip's
/// CloudFront CDN to allow page image downloads.
async fn download_page(
    client: &reqwest::Client,
    url: &str,
    page_path: &Path,
    referer: &str,
) -> Result<(), AppError> {
    let response = client
        .get(url)
        .timeout(PAGE_TIMEOUT)
        .header("Referer", referer)
        .send()
        .await
        .map_err(|e| AppError::DownloadError(format!("HTTP request failed for {}: {}", url, e)))?;

    if !response.status().is_success() {
        let status = response.status();

        // HTTP 403 indicates password-protected content.
        if status == reqwest::StatusCode::FORBIDDEN {
            return Err(AppError::PasswordRequired(
                "Password required to download pages".to_string(),
            ));
        }

        return Err(AppError::DownloadError(format!(
            "HTTP {} for page {}",
            status.as_u16(),
            url
        )));
    }

    let bytes = response.bytes().await.map_err(|e| {
        AppError::DownloadError(format!("Failed to read response body for {}: {}", url, e))
    })?;

    // Validate image format: accept both JPEG (SOI: 0xFFD8) and WebP (RIFF...WEBP).
    let is_jpeg = bytes.len() >= 2 && bytes[0] == 0xFF && bytes[1] == 0xD8;
    let is_webp = bytes.len() >= 12
        && &bytes[0..4] == b"RIFF"
        && &bytes[8..12] == b"WEBP";
    if !is_jpeg && !is_webp {
        return Err(AppError::DownloadError(format!(
            "Invalid image data for {} ({} bytes, expected JPEG or WebP magic bytes)",
            url,
            bytes.len()
        )));
    }

    // Ensure parent directory exists.
    if let Some(parent) = page_path.parent() {
        tokio::fs::create_dir_all(parent).await.map_err(|e| {
            AppError::FileSystemError(format!(
                "Failed to create directory {}: {}",
                parent.display(),
                e
            ))
        })?;
    }

    tokio::fs::write(page_path, &bytes).await.map_err(|e| {
        AppError::FileSystemError(format!(
            "Failed to write page to {}: {}",
            page_path.display(),
            e
        ))
    })?;

    Ok(())
}

/// Block until the pause flag is cleared.
///
/// Polls the flag every [`PAUSE_POLL_INTERVAL`]. This is intentionally a busy-wait
/// rather than a `Notify` because pause/resume is a rare user action and the
/// simplicity of an atomic flag outweighs the negligible CPU cost of polling at
/// 200ms intervals.
async fn wait_if_paused(token: &PauseToken) {
    while token.load(Ordering::Acquire) {
        tokio::time::sleep(PAUSE_POLL_INTERVAL).await;
    }
}

/// Check if a page JPG already exists and is non-empty (resume-from-cache).
fn check_cache(temp_dir: &Path, page: usize) -> bool {
    let path = temp_dir.join(format!("{}.webp", page));
    path.exists()
        && std::fs::metadata(&path)
            .map(|m| m.len() > 0)
            .unwrap_or(false)
}

/// Construct the CDN URL for a page.
///
/// Uses filenames from metadata if available, otherwise falls back to
/// sequential `{page}.webp` naming.
fn page_url(parsed_url: &ParsedUrl, metadata: &DocumentMetadata, page: usize) -> String {
    if !metadata.page_filenames.is_empty() {
        // page_filenames is 0-indexed, pages are 1-indexed.
        if let Some(filename) = metadata.page_filenames.get(page - 1) {
            return parsed_url.large_page_url(filename);
        }
    }
    parsed_url.page_url(page)
}

/// Emit a `download:progress` event. Fire-and-forget.
fn emit_progress<R: tauri::Runtime>(
    app_handle: &tauri::AppHandle<R>,
    download_id: &str,
    current_page: usize,
    total_pages: usize,
) {
    let progress = if total_pages > 0 {
        ((current_page as f64 / total_pages as f64) * 100.0) as u8
    } else {
        0
    };

    let payload = ProgressPayload {
        id: download_id.to_string(),
        progress,
        current_page,
        total_pages,
    };

    if let Err(e) = app_handle.emit("download:progress", &payload) {
        tracing::warn!("Failed to emit download:progress event: {}", e);
    }
}

/// Download all page JPGs with a password for protected documents.
///
/// Same as [`download_all`] but appends the password as a query parameter to
/// each page URL. Used when the initial download attempt returns HTTP 403.
///
/// # Errors
/// Same as [`download_all`], plus `AppError::PasswordRequired` if the password
/// is incorrect (HTTP 403 persists).
pub async fn download_all_with_password<R: tauri::Runtime>(
    client: &reqwest::Client,
    parsed_url: &ParsedUrl,
    metadata: &DocumentMetadata,
    temp_dir: &Path,
    app_handle: &tauri::AppHandle<R>,
    download_id: &str,
    pause_token: &PauseToken,
    password: &str,
) -> Result<(), AppError> {
    let page_count = metadata.page_count;
    if page_count == 0 {
        tracing::info!("No pages to download, returning early");
        return Ok(());
    }

    tokio::fs::create_dir_all(temp_dir).await.map_err(|e| {
        AppError::FileSystemError(format!(
            "Failed to create temp directory {}: {}",
            temp_dir.display(),
            e
        ))
    })?;

    tracing::info!(
        download_id = %download_id,
        page_count = page_count,
        "Starting page download with password"
    );

    let semaphore = Arc::new(Semaphore::new(MAX_CONCURRENT));
    let mut join_set = tokio::task::JoinSet::new();

    for page in 1..=page_count {
        wait_if_paused(pause_token).await;

        if check_cache(temp_dir, page) {
            tracing::debug!(page = page, "Page cached, skipping");
            emit_progress(app_handle, download_id, page, page_count);
            continue;
        }

        let page_path = temp_dir.join(format!("{}.webp", page));
        if page_path.exists() {
            let _ = tokio::fs::remove_file(&page_path).await;
        }

        let url = page_url_with_password(parsed_url, metadata, page, password);
        let client = client.clone();
        let semaphore = Arc::clone(&semaphore);
        let temp_dir = temp_dir.to_path_buf();
        let download_id = download_id.to_string();
        let app_handle = app_handle.clone();
        let referer = parsed_url.base_url.clone();

        join_set.spawn(async move {
            let permit = semaphore.acquire_owned().await.map_err(|e| {
                AppError::DownloadError(format!("Semaphore closed: {}", e))
            })?;

            let page_path = temp_dir.join(format!("{}.webp", page));
            let result = download_page_with_retry(
                &client,
                &url,
                &page_path,
                &download_id,
                page,
                page_count,
                &referer,
                &app_handle,
            )
            .await;

            drop(permit);
            result
        });
    }

    let mut first_error: Option<AppError> = None;
    while let Some(result) = join_set.join_next().await {
        match result {
            Ok(Ok(())) => {}
            Ok(Err(e)) => {
                if first_error.is_some() {
                    tracing::error!("Additional page download error: {}", e);
                } else {
                    first_error = Some(e);
                }
            }
            Err(join_err) => {
                let e = AppError::DownloadError(format!("Task join error: {}", join_err));
                if first_error.is_some() {
                    tracing::error!("Additional task join error: {}", join_err);
                } else {
                    first_error = Some(e);
                }
            }
        }
    }

    match first_error {
        Some(e) => Err(e),
        None => {
            tracing::info!(
                download_id = %download_id,
                "All {} pages downloaded successfully (with password)",
                page_count
            );
            Ok(())
        }
    }
}

/// Construct the CDN URL for a page with a password query parameter.
fn page_url_with_password(
    parsed_url: &ParsedUrl,
    metadata: &DocumentMetadata,
    page: usize,
    password: &str,
) -> String {
    let base = if !metadata.page_filenames.is_empty() {
        if let Some(filename) = metadata.page_filenames.get(page - 1) {
            parsed_url.large_page_url(filename)
        } else {
            parsed_url.page_url(page)
        }
    } else {
        parsed_url.page_url(page)
    };
    let encoded_password: String =
        url::form_urlencoded::byte_serialize(password.as_bytes()).collect();
    format!("{}?password={}", base, encoded_password)
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── check_cache tests ───────────────────────────────────────────

    #[test]
    fn check_cache_returns_true_for_existing() {
        let dir = std::env::temp_dir()
            .join("anyflipdl_test_cache")
            .join(uuid::Uuid::new_v4().to_string());
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("1.webp"), &[0x52, 0x49, 0x46, 0x46]).unwrap();

        assert!(check_cache(&dir, 1));

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn check_cache_returns_false_for_missing() {
        let dir = std::env::temp_dir()
            .join("anyflipdl_test_cache")
            .join(uuid::Uuid::new_v4().to_string());
        std::fs::create_dir_all(&dir).unwrap();

        assert!(!check_cache(&dir, 1));

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn check_cache_returns_false_for_empty() {
        let dir = std::env::temp_dir()
            .join("anyflipdl_test_cache")
            .join(uuid::Uuid::new_v4().to_string());
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("1.webp"), b"").unwrap();

        assert!(!check_cache(&dir, 1));

        let _ = std::fs::remove_dir_all(&dir);
    }

    // ── page_url tests ─────────────────────────────────────────────

    fn make_parsed_url() -> ParsedUrl {
        ParsedUrl {
            user: "testuser".to_string(),
            book: "testbook".to_string(),
            base_url: "https://online.anyflip.com/testuser/testbook".to_string(),
        }
    }

    #[test]
    fn page_url_with_filenames() {
        let parsed = make_parsed_url();
        let metadata = DocumentMetadata {
            url: parsed.base_url.clone(),
            title: "Test".to_string(),
            page_count: 3,
            page_filenames: vec![
                "page001.webp".to_string(),
                "page002.webp".to_string(),
                "page003.webp".to_string(),
            ],
            author: None,
            date: None,
            thumbnail: None,
            password_protected: false,
        };

        let url = page_url(&parsed, &metadata, 1);
        assert_eq!(
            url,
            "https://online.anyflip.com/testuser/testbook/files/large/page001.webp"
        );

        let url = page_url(&parsed, &metadata, 3);
        assert_eq!(
            url,
            "https://online.anyflip.com/testuser/testbook/files/large/page003.webp"
        );
    }

    #[test]
    fn page_url_without_filenames() {
        let parsed = make_parsed_url();
        let metadata = DocumentMetadata {
            url: parsed.base_url.clone(),
            title: "Test".to_string(),
            page_count: 10,
            page_filenames: vec![],
            author: None,
            date: None,
            thumbnail: None,
            password_protected: false,
        };

        let url = page_url(&parsed, &metadata, 5);
        assert_eq!(
            url,
            "https://online.anyflip.com/testuser/testbook/files/mobile/5.webp"
        );
    }

    #[test]
    fn page_url_fallback_when_index_out_of_bounds() {
        let parsed = make_parsed_url();
        let metadata = DocumentMetadata {
            url: parsed.base_url.clone(),
            title: "Test".to_string(),
            page_count: 3,
            page_filenames: vec!["only_first.webp".to_string()],
            author: None,
            date: None,
            thumbnail: None,
            password_protected: false,
        };

        // Page 2 has no filename entry, falls back to sequential.
        let url = page_url(&parsed, &metadata, 2);
        assert_eq!(
            url,
            "https://online.anyflip.com/testuser/testbook/files/mobile/2.webp"
        );
    }

    // ── download_page JPEG validation tests ─────────────────────────

    #[tokio::test]
    async fn download_page_rejects_non_jpeg_response() {
        let mut server = mockito::Server::new();
        let mock = server
            .mock("GET", "/files/mobile/1.webp")
            .with_status(200)
            .with_body(b"<html>not a jpeg</html>")
            .create();

        let client = reqwest::Client::new();
        let dir = std::env::temp_dir()
            .join("anyflipdl_test_dl")
            .join(uuid::Uuid::new_v4().to_string());
        tokio::fs::create_dir_all(&dir).await.unwrap();

        let result = download_page(&client, &format!("{}/files/mobile/1.webp", server.url()), &dir.join("1.webp"), "").await;
        assert!(result.is_err());
        let err_msg = result.unwrap_err().to_string();
        assert!(err_msg.contains("Invalid image data"), "Expected image validation error, got: {}", err_msg);

        mock.assert();
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[tokio::test]
    async fn download_page_accepts_valid_jpeg_header() {
        let mut server = mockito::Server::new();
        // Minimal JPEG: SOI + EOI
        let jpeg_data = vec![0xFF, 0xD8, 0xFF, 0xD9];
        let mock = server
            .mock("GET", "/files/mobile/1.webp")
            .with_status(200)
            .with_body(&jpeg_data)
            .create();

        let client = reqwest::Client::new();
        let dir = std::env::temp_dir()
            .join("anyflipdl_test_dl")
            .join(uuid::Uuid::new_v4().to_string());
        tokio::fs::create_dir_all(&dir).await.unwrap();

        let result = download_page(&client, &format!("{}/files/mobile/1.webp", server.url()), &dir.join("1.webp"), "").await;
        assert!(result.is_ok());
        assert!(dir.join("1.webp").exists());

        mock.assert();
        let _ = std::fs::remove_dir_all(&dir);
    }

    // ── download_all resume-from-cache test ─────────────────────────

    #[tokio::test]
    async fn download_all_skips_cached_pages() {
        let mut server = mockito::Server::new();

        // Only page 2 should be downloaded (page 1 and 3 are cached).
        let jpeg_data = vec![0xFF, 0xD8, 0xFF, 0xD9];
        let mock_p2 = server
            .mock("GET", "/files/mobile/2.webp")
            .with_status(200)
            .with_body(&jpeg_data)
            .expect(1)
            .create();

        let dir = std::env::temp_dir()
            .join("anyflipdl_test_resume")
            .join(uuid::Uuid::new_v4().to_string());
        tokio::fs::create_dir_all(&dir).await.unwrap();

        // Pre-populate cached pages.
        tokio::fs::write(dir.join("1.webp"), &[0x52, 0x49, 0x46, 0x46])
            .await
            .unwrap();
        tokio::fs::write(dir.join("3.webp"), &[0x52, 0x49, 0x46, 0x46])
            .await
            .unwrap();

        let parsed = ParsedUrl {
            user: "testuser".to_string(),
            book: "testbook".to_string(),
            base_url: server.url(),
        };
        let metadata = DocumentMetadata {
            url: server.url(),
            title: "Test".to_string(),
            page_count: 3,
            page_filenames: vec![],
            author: None,
            date: None,
            thumbnail: None,
            password_protected: false,
        };

        let client = reqwest::Client::new();
        let app = tauri::test::mock_app();
        let app_handle = app.handle().clone();

        let pause_token: PauseToken = Arc::new(AtomicBool::new(false));
        let result = download_all(
            &client,
            &parsed,
            &metadata,
            &dir,
            &app_handle,
            "test-resume",
            &pause_token,
        )
        .await;

        assert!(result.is_ok());
        mock_p2.assert();

        let _ = std::fs::remove_dir_all(&dir);
    }
}
