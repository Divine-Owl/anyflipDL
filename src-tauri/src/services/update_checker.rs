use crate::error::AppError;
use serde::Serialize;

/// GitHub repository for update checks (owner/repo).
const GITHUB_REPO: &str = "AnyflipDL/AnyflipDL";

/// Response from the GitHub releases API (subset of fields).
#[derive(Debug, serde::Deserialize)]
struct GitHubRelease {
    tag_name: String,
    html_url: String,
}

/// Result of an update check.
#[derive(Debug, Clone, Serialize)]
pub struct UpdateCheckResult {
    #[serde(rename = "currentVersion")]
    pub current_version: String,
    #[serde(rename = "latestVersion")]
    pub latest_version: String,
    #[serde(rename = "updateAvailable")]
    pub update_available: bool,
    #[serde(rename = "downloadUrl")]
    pub download_url: Option<String>,
}

/// Check GitHub releases for a newer version of AnyflipDL.
///
/// Fetches the latest release from the configured GitHub repository and
/// compares its semver tag against the current application version.
///
/// # Arguments
/// - `client`: Shared reqwest client.
/// - `current_version`: The current application version (e.g., "1.0.0").
///
/// # Returns
/// An [`UpdateCheckResult`] with the comparison result.
///
/// # Errors
/// - `AppError::NetworkError` if the HTTP request fails.
/// - `AppError::MetadataError` if the response cannot be parsed.
pub async fn check_for_updates(
    client: &reqwest::Client,
    current_version: &str,
) -> Result<UpdateCheckResult, AppError> {
    let url = format!(
        "https://api.github.com/repos/{}/releases/latest",
        GITHUB_REPO
    );

    tracing::info!("Checking for updates at: {}", url);

    let response = client
        .get(&url)
        .header("User-Agent", format!("AnyflipDL/{}", current_version))
        .header("Accept", "application/vnd.github+json")
        .send()
        .await
        .map_err(|e| {
            tracing::error!("Update check HTTP request failed: {:?}", e);
            AppError::NetworkError(e.to_string())
        })?;

    if !response.status().is_success() {
        let status = response.status();
        tracing::warn!(
            "GitHub releases API returned HTTP {} for {}",
            status,
            url
        );
        // If the repo doesn't exist or has no releases, return "up to date".
        return Ok(UpdateCheckResult {
            current_version: current_version.to_string(),
            latest_version: current_version.to_string(),
            update_available: false,
            download_url: None,
        });
    }

    let release: GitHubRelease = response.json().await.map_err(|e| {
        tracing::error!("Failed to parse GitHub release JSON: {:?}", e);
        AppError::MetadataError(format!("Failed to parse update response: {}", e))
    })?;

    let latest_version = strip_version_prefix(&release.tag_name);
    let update_available = compare_versions(&latest_version, current_version) > 0;

    tracing::info!(
        current = current_version,
        latest = latest_version,
        update_available = update_available,
        "Update check complete"
    );

    Ok(UpdateCheckResult {
        current_version: current_version.to_string(),
        latest_version,
        update_available,
        download_url: if update_available {
            Some(release.html_url)
        } else {
            None
        },
    })
}

/// Strip a leading `v` or `V` prefix from a version string.
///
/// `"v1.2.3"` -> `"1.2.3"`, `"1.2.3"` -> `"1.2.3"`.
fn strip_version_prefix(tag: &str) -> String {
    tag.strip_prefix('v')
        .or_else(|| tag.strip_prefix('V'))
        .unwrap_or(tag)
        .to_string()
}

/// Compare two semver version strings.
///
/// Returns `1` if `a > b`, `-1` if `a < b`, `0` if equal.
/// Treats missing minor/patch as 0.
fn compare_versions(a: &str, b: &str) -> i32 {
    let a_parts = parse_version(a);
    let b_parts = parse_version(b);

    for (a_val, b_val) in a_parts.iter().zip(b_parts.iter()) {
        if a_val > b_val {
            return 1;
        }
        if a_val < b_val {
            return -1;
        }
    }

    // If all compared parts are equal, the longer one is greater
    // (e.g., 1.2.3 > 1.2).
    a_parts.len().cmp(&b_parts.len()) as i32
}

/// Parse a version string into a vector of numeric components.
///
/// `"1.2.3"` -> `[1, 2, 3]`, `"2.0"` -> `[2, 0]`.
fn parse_version(version: &str) -> Vec<u64> {
    version
        .split('.')
        .filter_map(|part| part.parse::<u64>().ok())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── strip_version_prefix ─────────────────────────────────────────

    #[test]
    fn strip_prefix_v() {
        assert_eq!(strip_version_prefix("v1.2.3"), "1.2.3");
    }

    #[test]
    fn strip_prefix_uppercase_v() {
        assert_eq!(strip_version_prefix("V2.0.0"), "2.0.0");
    }

    #[test]
    fn strip_no_prefix() {
        assert_eq!(strip_version_prefix("1.0.0"), "1.0.0");
    }

    // ── parse_version ────────────────────────────────────────────────

    #[test]
    fn parse_three_parts() {
        assert_eq!(parse_version("1.2.3"), vec![1, 2, 3]);
    }

    #[test]
    fn parse_two_parts() {
        assert_eq!(parse_version("2.0"), vec![2, 0]);
    }

    #[test]
    fn parse_single_part() {
        assert_eq!(parse_version("5"), vec![5]);
    }

    // ── compare_versions ─────────────────────────────────────────────

    #[test]
    fn compare_equal() {
        assert_eq!(compare_versions("1.0.0", "1.0.0"), 0);
    }

    #[test]
    fn compare_patch_greater() {
        assert_eq!(compare_versions("1.0.1", "1.0.0"), 1);
    }

    #[test]
    fn compare_minor_greater() {
        assert_eq!(compare_versions("1.1.0", "1.0.9"), 1);
    }

    #[test]
    fn compare_major_greater() {
        assert_eq!(compare_versions("2.0.0", "1.9.9"), 1);
    }

    #[test]
    fn compare_less_than() {
        assert_eq!(compare_versions("0.9.0", "1.0.0"), -1);
    }

    #[test]
    fn compare_different_lengths_a_longer() {
        assert_eq!(compare_versions("1.0.1", "1.0"), 1);
    }

    #[test]
    fn compare_different_lengths_b_longer() {
        assert_eq!(compare_versions("1.0", "1.0.1"), -1);
    }
}
