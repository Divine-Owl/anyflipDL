use crate::error::AppError;
use std::path::PathBuf;
use std::time::Duration;
use tracing::{info, warn};
use uuid::Uuid;

/// Maximum age before a temp directory is considered stale (24 hours).
const STALE_THRESHOLD: Duration = Duration::from_secs(24 * 60 * 60);

/// Page file extension.
const PAGE_EXT: &str = "webp";

/// Manages temporary page image directories for in-progress downloads.
///
/// Each download gets an isolated directory under `{base_dir}/{uuid}/`
/// containing numbered WebP files (`1.webp`, `2.webp`, ...).
pub struct TempStorage {
    base_dir: PathBuf,
}

impl TempStorage {
    /// Initialize with the default base directory: `%TEMP%/anyflipdl/`.
    pub fn new() -> Self {
        Self {
            base_dir: std::env::temp_dir().join("anyflipdl"),
        }
    }

    /// Initialize with a custom base directory (useful for testing).
    #[cfg(test)]
    pub fn with_base_dir(base_dir: PathBuf) -> Self {
        Self { base_dir }
    }

    /// Create a new UUID-named directory under the base dir.
    /// Returns the full path to the created directory.
    pub fn create_download_dir(&self) -> Result<PathBuf, AppError> {
        let uuid = Uuid::new_v4().to_string();
        let dir = self.base_dir.join(&uuid);
        std::fs::create_dir_all(&dir)?;
        info!("Created temp download dir: {:?}", dir);
        Ok(dir)
    }

    /// Return the path to a specific page WebP: `{base}/{download_id}/{page}.webp`.
    pub fn page_path(&self, download_id: &str, page: u32) -> PathBuf {
        self.base_dir
            .join(download_id)
            .join(format!("{}.{}", page, PAGE_EXT))
    }

    /// Check if a specific page WebP already exists and is non-empty
    /// (for resume-from-cache support).
    pub fn page_exists(&self, download_id: &str, page: u32) -> bool {
        let path = self.page_path(download_id, page);
        path.exists()
            && path
                .metadata()
                .map(|m| m.len() > 0)
                .unwrap_or(false)
    }

    /// Write WebP bytes to the page file. Creates parent directories if needed.
    pub fn write_page(
        &self,
        download_id: &str,
        page: u32,
        data: &[u8],
    ) -> Result<(), AppError> {
        let path = self.page_path(download_id, page);

        // Ensure the download directory exists
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        std::fs::write(&path, data)?;
        Ok(())
    }

    /// Remove an entire download directory and all its contents.
    pub fn delete_download_dir(&self, download_id: &str) -> Result<(), AppError> {
        let dir = self.base_dir.join(download_id);
        if dir.exists() {
            std::fs::remove_dir_all(&dir)?;
            info!("Deleted temp download dir: {:?}", dir);
        } else {
            warn!("Attempted to delete non-existent temp dir: {:?}", dir);
        }
        Ok(())
    }

    /// Delete all download directories older than 24 hours.
    /// Returns the count of deleted directories.
    pub fn cleanup_stale(&self) -> Result<u32, AppError> {
        if !self.base_dir.exists() {
            return Ok(0);
        }

        let mut deleted: u32 = 0;
        let entries = std::fs::read_dir(&self.base_dir)?;

        for entry in entries {
            let entry = match entry {
                Ok(e) => e,
                Err(e) => {
                    warn!("Failed to read temp dir entry: {}", e);
                    continue;
                }
            };

            let path = entry.path();
            if !path.is_dir() {
                continue;
            }

            let is_stale = path
                .metadata()
                .and_then(|m| m.modified())
                .map(|modified| {
                    modified
                        .elapsed()
                        .unwrap_or(Duration::ZERO)
                        > STALE_THRESHOLD
                })
                .unwrap_or(false);

            if is_stale {
                match std::fs::remove_dir_all(&path) {
                    Ok(()) => {
                        info!("Cleaned stale temp dir: {:?}", path);
                        deleted += 1;
                    }
                    Err(e) => {
                        warn!("Failed to clean stale temp dir {:?}: {}", path, e);
                    }
                }
            }
        }

        if deleted > 0 {
            info!("Cleaned {} stale temp directories", deleted);
        }
        Ok(deleted)
    }

    /// Return a sorted list of page numbers that have cached (non-empty) WebPs
    /// for a given download ID.
    pub fn list_cached_pages(&self, download_id: &str) -> Result<Vec<u32>, AppError> {
        let dir = self.base_dir.join(download_id);
        if !dir.exists() {
            return Ok(Vec::new());
        }

        let mut pages: Vec<u32> = Vec::new();
        let entries = std::fs::read_dir(&dir)?;

        for entry in entries {
            let entry = match entry {
                Ok(e) => e,
                Err(e) => {
                    warn!("Failed to read page entry: {}", e);
                    continue;
                }
            };

            let path = entry.path();
            if !path.is_file() {
                continue;
            }

            // Only consider .webp files
            match path.extension().and_then(|e| e.to_str()) {
                Some(ext) if ext == PAGE_EXT => {}
                _ => continue,
            }

            // Parse page number from filename (e.g., "5.webp" -> 5)
            if let Some(stem) = path.file_stem().and_then(|s| s.to_str()) {
                if let Ok(page_num) = stem.parse::<u32>() {
                    // Only include non-empty files
                    if path
                        .metadata()
                        .map(|m| m.len() > 0)
                        .unwrap_or(false)
                    {
                        pages.push(page_num);
                    }
                }
            }
        }

        pages.sort_unstable();
        Ok(pages)
    }
}

impl Default for TempStorage {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Create an isolated TempStorage for each test.
    fn make_storage() -> (TempStorage, PathBuf) {
        let base = std::env::temp_dir()
            .join("anyflipdl_test")
            .join(Uuid::new_v4().to_string());
        (TempStorage::with_base_dir(base.clone()), base)
    }

    #[test]
    fn create_download_dir_generates_uuid_path() {
        let (storage, base) = make_storage();
        let dir = storage.create_download_dir().unwrap();

        // Directory exists
        assert!(dir.exists());
        // Path is under the base directory
        assert!(dir.starts_with(&base));
        // Directory name is a valid UUID
        let dir_name = dir.file_name().unwrap().to_str().unwrap();
        assert!(Uuid::parse_str(dir_name).is_ok());

        let _ = std::fs::remove_dir_all(&base);
    }

    #[test]
    fn create_download_dir_generates_unique_paths() {
        let (storage, base) = make_storage();
        let dir1 = storage.create_download_dir().unwrap();
        let dir2 = storage.create_download_dir().unwrap();

        assert_ne!(dir1, dir2);

        let _ = std::fs::remove_dir_all(&base);
    }

    #[test]
    fn page_path_returns_correct_path() {
        let (storage, base) = make_storage();
        let path = storage.page_path("abc", 5);

        // Should end with abc/5.webp
        assert!(path.ends_with("abc/5.webp"));
        // Should be under base_dir
        assert!(path.starts_with(&base));

        let _ = std::fs::remove_dir_all(&base);
    }

    #[test]
    fn page_exists_returns_false_for_missing() {
        let (storage, base) = make_storage();
        assert!(!storage.page_exists("nonexistent", 1));

        let _ = std::fs::remove_dir_all(&base);
    }

    #[test]
    fn page_exists_returns_true_for_existing() {
        let (storage, base) = make_storage();
        let dir = storage.create_download_dir().unwrap();
        let id = dir.file_name().unwrap().to_str().unwrap().to_string();

        storage.write_page(&id, 1, &[0xFF, 0xD8]).unwrap();
        assert!(storage.page_exists(&id, 1));
        assert!(!storage.page_exists(&id, 2));

        let _ = std::fs::remove_dir_all(&base);
    }

    #[test]
    fn write_page_creates_file_with_content() {
        let (storage, base) = make_storage();
        let dir = storage.create_download_dir().unwrap();
        let id = dir.file_name().unwrap().to_str().unwrap().to_string();

        let data = b"fake jpeg data";
        storage.write_page(&id, 3, data).unwrap();

        let path = storage.page_path(&id, 3);
        assert!(path.exists());
        let written = std::fs::read(&path).unwrap();
        assert_eq!(written, data);

        let _ = std::fs::remove_dir_all(&base);
    }

    #[test]
    fn delete_download_dir_removes_all() {
        let (storage, base) = make_storage();
        let dir = storage.create_download_dir().unwrap();
        let id = dir.file_name().unwrap().to_str().unwrap().to_string();

        // Write some pages
        storage.write_page(&id, 1, b"page1").unwrap();
        storage.write_page(&id, 2, b"page2").unwrap();
        assert!(dir.exists());

        // Delete
        storage.delete_download_dir(&id).unwrap();
        assert!(!dir.exists());

        let _ = std::fs::remove_dir_all(&base);
    }

    #[test]
    fn delete_nonexistent_dir_does_not_error() {
        let (storage, base) = make_storage();
        // Should not panic or error
        storage.delete_download_dir("nonexistent").unwrap();

        let _ = std::fs::remove_dir_all(&base);
    }

    #[test]
    fn list_cached_pages_returns_sorted() {
        let (storage, base) = make_storage();
        let dir = storage.create_download_dir().unwrap();
        let id = dir.file_name().unwrap().to_str().unwrap().to_string();

        // Write pages out of order
        storage.write_page(&id, 3, b"p3").unwrap();
        storage.write_page(&id, 1, b"p1").unwrap();
        storage.write_page(&id, 5, b"p5").unwrap();

        let pages = storage.list_cached_pages(&id).unwrap();
        assert_eq!(pages, vec![1, 3, 5]);

        let _ = std::fs::remove_dir_all(&base);
    }

    #[test]
    fn list_cached_pages_returns_empty() {
        let (storage, base) = make_storage();
        let dir = storage.create_download_dir().unwrap();
        let id = dir.file_name().unwrap().to_str().unwrap().to_string();

        let pages = storage.list_cached_pages(&id).unwrap();
        assert!(pages.is_empty());

        let _ = std::fs::remove_dir_all(&base);
    }

    #[test]
    fn list_cached_pages_skips_non_webp_files() {
        let (storage, base) = make_storage();
        let dir = storage.create_download_dir().unwrap();
        let id = dir.file_name().unwrap().to_str().unwrap().to_string();

        // Write a WebP and a non-WebP file
        storage.write_page(&id, 1, b"p1").unwrap();
        std::fs::write(dir.join("notes.txt"), b"hello").unwrap();

        let pages = storage.list_cached_pages(&id).unwrap();
        assert_eq!(pages, vec![1]);

        let _ = std::fs::remove_dir_all(&base);
    }

    #[test]
    fn page_exists_returns_false_for_empty_file() {
        let (storage, base) = make_storage();
        let dir = storage.create_download_dir().unwrap();
        let id = dir.file_name().unwrap().to_str().unwrap().to_string();

        // Write empty file
        std::fs::write(storage.page_path(&id, 1), b"").unwrap();
        assert!(!storage.page_exists(&id, 1));

        let _ = std::fs::remove_dir_all(&base);
    }
}
