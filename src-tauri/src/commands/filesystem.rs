use crate::data::output::OutputFile;
use crate::error::AppError;
use std::path::Path;

#[derive(serde::Serialize)]
pub struct FileSizeResult {
    pub size: u64,
}

#[derive(serde::Serialize)]
pub struct RenameResult {
    pub new_path: String,
    pub new_title: String,
}

#[tauri::command]
pub async fn get_file_size(path: String) -> Result<FileSizeResult, AppError> {
    if path.is_empty() {
        return Err(AppError::FileSystemError(
            "File path cannot be empty".to_string(),
        ));
    }

    let metadata = std::fs::metadata(&path).map_err(|e| {
        AppError::FileSystemError(format!("Failed to read file metadata: {}", e))
    })?;

    Ok(FileSizeResult {
        size: metadata.len(),
    })
}

#[tauri::command]
pub async fn pick_directory(app_handle: tauri::AppHandle) -> Result<Option<String>, AppError> {
    use tauri_plugin_dialog::DialogExt;
    let path = app_handle
        .dialog()
        .file()
        .set_title("Select Save Location")
        .blocking_pick_folder();
    Ok(path.map(|p| p.to_string()))
}

#[tauri::command]
pub async fn open_file(path: String, app_handle: tauri::AppHandle) -> Result<(), AppError> {
    if path.is_empty() {
        return Err(AppError::InvalidUrl(
            "File path cannot be empty".to_string(),
        ));
    }

    if !std::path::Path::new(&path).exists() {
        return Err(AppError::FileSystemError(format!(
            "File not found: {}",
            path
        )));
    }

    use tauri_plugin_shell::ShellExt;
    #[allow(deprecated)]
    app_handle
        .shell()
        .open(path, None)
        .map_err(|e| AppError::FileSystemError(format!("Failed to open file: {}", e)))?;

    Ok(())
}

/// Rename a downloaded file on disk.
///
/// Takes the current file path and a new title, sanitizes the title,
/// renames the file preserving the extension, and returns the new path.
#[tauri::command]
pub async fn rename_file(current_path: String, new_title: String) -> Result<RenameResult, AppError> {
    if current_path.is_empty() {
        return Err(AppError::FileSystemError(
            "File path cannot be empty".to_string(),
        ));
    }

    let path = Path::new(&current_path);
    if !path.exists() {
        return Err(AppError::FileSystemError(format!(
            "File not found: {}",
            current_path
        )));
    }

    let sanitized = OutputFile::sanitize_filename(&new_title);
    if sanitized == "untitled" && new_title.trim().is_empty() {
        return Err(AppError::FileSystemError(
            "Title cannot be empty".to_string(),
        ));
    }

    let extension = path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("pdf");

    let parent = path.parent().unwrap_or(Path::new("."));
    let new_filename = format!("{}.{}", sanitized, extension);
    let new_path = parent.join(&new_filename);

    // Check if target already exists and is not the same file
    if new_path.exists() && new_path != path {
        return Err(AppError::FileSystemError(format!(
            "A file named '{}' already exists",
            new_filename
        )));
    }

    // Only rename if the path actually changes
    if new_path != path {
        std::fs::rename(path, &new_path).map_err(|e| {
            AppError::FileSystemError(format!("Failed to rename file: {}", e))
        })?;
    }

    Ok(RenameResult {
        new_path: new_path.to_string_lossy().to_string(),
        new_title: sanitized,
    })
}
