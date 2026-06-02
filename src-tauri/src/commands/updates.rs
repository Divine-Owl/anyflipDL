use crate::error::AppError;
use crate::services::update_checker::UpdateCheckResult;

/// Check for application updates via the GitHub releases API.
///
/// Creates a dedicated HTTP client with a 10-second timeout, fetches the
/// latest release from the configured GitHub repository, and compares its
/// version tag against the compiled-in package version.
#[tauri::command]
pub async fn check_for_updates(app_handle: tauri::AppHandle) -> Result<UpdateCheckResult, AppError> {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .user_agent("Mozilla/5.0")
        .build()
        .map_err(|e| AppError::NetworkError(format!("Failed to create HTTP client: {}", e)))?;

    let version = app_handle.package_info().version.to_string();
    crate::services::update_checker::check_for_updates(&client, &version).await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cargo_pkg_version_is_valid() {
        let version = env!("CARGO_PKG_VERSION");
        assert!(
            !version.is_empty(),
            "CARGO_PKG_VERSION should not be empty"
        );
        assert!(
            version.contains('.'),
            "CARGO_PKG_VERSION should be semver: {}",
            version
        );
    }
}
