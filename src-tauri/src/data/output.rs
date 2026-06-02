use crate::error::AppError;
use std::path::{Path, PathBuf};
use tracing::info;

/// Maximum filename length (excluding extension).
const MAX_FILENAME_LEN: usize = 200;

/// Fallback filename when the sanitized result is empty.
const FALLBACK_FILENAME: &str = "untitled";

/// Characters that are unsafe in filenames on Windows/macOS/Linux.
const FORBIDDEN_CHARS: &[char] = &['\\', '/', ':', '*', '?', '"', '<', '>', '|'];

/// Return the current local date as `YYYY-MM-DD`.
fn current_date_string() -> String {
    // Use SystemTime + manual offset to avoid adding a chrono dependency.
    // This gives UTC; for local date we apply a rough offset. For production
    // accuracy we'd use chrono, but YYYY-MM-DD from UTC is close enough for
    // filename purposes (off by at most one day near midnight).
    let duration = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default();
    let secs = duration.as_secs();
    let days = (secs / 86400) as i64;
    // Convert days since epoch to Y-M-D using the civil calendar algorithm.
    let (y, m, d) = civil_from_days(days);
    format!("{:04}-{:02}-{:02}", y, m, d)
}

/// Convert days since Unix epoch to (year, month, day) using the civil
/// calendar algorithm. Adapted from Howard Hinnant's `chrono` algorithms.
fn civil_from_days(days: i64) -> (i64, u32, u32) {
    let z = days + 719468;
    let era = if z >= 0 { z } else { z - 146096 } / 146097;
    let doe = (z - era * 146097) as u32;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let y = yoe as i64 + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = if m <= 2 { y + 1 } else { y };
    (y, m, d)
}

/// Unit struct with static methods for output file operations.
pub struct OutputFile;

impl OutputFile {
    /// Sanitize a title for use as a filesystem filename.
    ///
    /// Rules:
    /// - Remove forbidden characters: `\ / : * ? " < > |`
    /// - Collapse consecutive spaces into a single space
    /// - Trim leading/trailing whitespace
    /// - Truncate to 200 characters
    /// - Return `"untitled"` if the result is empty
    pub fn sanitize_filename(title: &str) -> String {
        // Replace forbidden characters with spaces, then collapse
        let sanitized: String = title
            .chars()
            .map(|c| if FORBIDDEN_CHARS.contains(&c) { ' ' } else { c })
            .collect();

        // Collapse consecutive whitespace into single spaces
        let collapsed = sanitized
            .split_whitespace()
            .collect::<Vec<&str>>()
            .join(" ");

        let trimmed = collapsed.trim();

        if trimmed.is_empty() {
            return FALLBACK_FILENAME.to_string();
        }

        // Truncate to max length
        if trimmed.len() > MAX_FILENAME_LEN {
            trimmed[..MAX_FILENAME_LEN].to_string()
        } else {
            trimmed.to_string()
        }
    }

    /// Apply a naming pattern by substituting tokens and sanitizing the result.
    ///
    /// Supported tokens:
    /// - `{title}` -- document title
    /// - `{author}` -- author name (falls back to empty string)
    /// - `{date}` -- current date in YYYY-MM-DD format
    /// - `{pageCount}` -- total page count
    /// - `{format}` -- output format (pdf/epub)
    ///
    /// After substitution, the result is passed through `sanitize_filename()`.
    /// Unknown tokens are left as-is.
    pub fn apply_naming_pattern(
        pattern: &str,
        title: &str,
        author: &str,
        page_count: usize,
        format: &str,
    ) -> String {
        let date = current_date_string();

        let result = pattern
            .replace("{title}", title)
            .replace("{author}", author)
            .replace("{date}", &date)
            .replace("{pageCount}", &page_count.to_string())
            .replace("{format}", format);

        Self::sanitize_filename(&result)
    }

    /// Construct the full output path: `{save_dir}/{sanitized_title}.{format}`.
    pub fn output_path(save_dir: &str, title: &str, format: &str) -> PathBuf {
        let sanitized = Self::sanitize_filename(title);
        Path::new(save_dir).join(format!("{}.{}", sanitized, format))
    }

    /// Write bytes to a file. Creates parent directories if needed.
    /// Automatically calls `verify()` after writing.
    pub fn write(path: &Path, data: &[u8]) -> Result<(), AppError> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        std::fs::write(path, data)?;
        info!("Wrote {} bytes to {:?}", data.len(), path);

        Self::verify(path)?;
        Ok(())
    }

    /// Verify that a file exists and has non-zero size.
    pub fn verify(path: &Path) -> Result<(), AppError> {
        if !path.exists() {
            return Err(AppError::FileSystemError(format!(
                "File does not exist: {:?}",
                path
            )));
        }

        let metadata = path.metadata()?;
        if metadata.len() == 0 {
            return Err(AppError::FileSystemError(format!(
                "File is empty: {:?}",
                path
            )));
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sanitize_removes_special_chars() {
        // Forbidden characters are replaced with spaces, then collapsed
        let result = OutputFile::sanitize_filename(r#"My Book: Vol/1?"#);
        assert_eq!(result, "My Book Vol 1");
    }

    #[test]
    fn sanitize_removes_all_nine_forbidden_chars() {
        let result = OutputFile::sanitize_filename(r#"a\b/c:d*e?f"g<h>i|j"#);
        // All forbidden chars removed, remaining letters collapse
        assert_eq!(result, "a b c d e f g h i j");
    }

    #[test]
    fn sanitize_collapses_spaces() {
        let result = OutputFile::sanitize_filename("My   Book");
        assert_eq!(result, "My Book");
    }

    #[test]
    fn sanitize_trims_whitespace() {
        let result = OutputFile::sanitize_filename("  My Book  ");
        assert_eq!(result, "My Book");
    }

    #[test]
    fn sanitize_truncates_long_title() {
        let long_title = "A".repeat(300);
        let result = OutputFile::sanitize_filename(&long_title);
        assert!(result.len() <= MAX_FILENAME_LEN);
        assert_eq!(result.len(), MAX_FILENAME_LEN);
    }

    #[test]
    fn sanitize_empty_gives_untitled() {
        assert_eq!(OutputFile::sanitize_filename(""), FALLBACK_FILENAME);
    }

    #[test]
    fn sanitize_only_special_chars_gives_untitled() {
        assert_eq!(
            OutputFile::sanitize_filename(r#":/*?"<>|\\"#),
            FALLBACK_FILENAME
        );
    }

    #[test]
    fn sanitize_preserves_unicode() {
        let result = OutputFile::sanitize_filename("Buch \u{00fc}ber 123");
        assert_eq!(result, "Buch \u{00fc}ber 123");
    }

    #[test]
    fn output_path_joins_correctly() {
        let path = OutputFile::output_path("/downloads", "My Book", "pdf");
        assert_eq!(path, PathBuf::from("/downloads/My Book.pdf"));
    }

    #[test]
    fn apply_naming_pattern_title_only() {
        let result = OutputFile::apply_naming_pattern("{title}", "My Book", "", 100, "pdf");
        assert_eq!(result, "My Book");
    }

    #[test]
    fn apply_naming_pattern_multiple_tokens() {
        let result = OutputFile::apply_naming_pattern(
            "{author} - {title} ({pageCount}p)",
            "My Book",
            "John Doe",
            42,
            "pdf",
        );
        assert_eq!(result, "John Doe - My Book (42p)");
    }

    #[test]
    fn apply_naming_pattern_format_token() {
        let result =
            OutputFile::apply_naming_pattern("{title} [{format}]", "Report", "", 10, "epub");
        assert_eq!(result, "Report [epub]");
    }

    #[test]
    fn apply_naming_pattern_date_token() {
        let result = OutputFile::apply_naming_pattern("{date} - {title}", "Doc", "", 5, "pdf");
        // Date should be YYYY-MM-DD format (10 chars), then " - Doc"
        assert!(result.ends_with("- Doc"));
        assert_eq!(result.len(), 17); // 10 + 3 + 4
    }

    #[test]
    fn apply_naming_pattern_sanitizes_result() {
        let result =
            OutputFile::apply_naming_pattern("{title}", "Bad: File? Name", "", 1, "pdf");
        assert_eq!(result, "Bad File Name");
    }

    #[test]
    fn apply_naming_pattern_unknown_token_preserved() {
        // Unknown tokens survive substitution but get sanitized if they
        // contain forbidden chars (braces are not forbidden).
        let result =
            OutputFile::apply_naming_pattern("{title} {unknown}", "Book", "", 1, "pdf");
        assert_eq!(result, "Book {unknown}");
    }

    #[test]
    fn apply_naming_pattern_empty_author() {
        let result =
            OutputFile::apply_naming_pattern("{author} - {title}", "Book", "", 1, "pdf");
        // Empty author produces " - Book" which trims to "- Book"
        assert_eq!(result, "- Book");
    }

    #[test]
    fn output_path_sanitizes_title() {
        let path = OutputFile::output_path("/downloads", "My: Book?", "epub");
        assert_eq!(path.file_name().unwrap(), "My Book.epub");
        assert!(path.parent().unwrap().ends_with("downloads"));
    }

    #[test]
    fn write_creates_file_with_content() {
        let dir = std::env::temp_dir().join("anyflipdl_test_output").join(uuid::Uuid::new_v4().to_string());
        let path = dir.join("test.pdf");

        OutputFile::write(&path, b"PDF content").unwrap();
        assert!(path.exists());
        assert_eq!(std::fs::read(&path).unwrap(), b"PDF content");

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn write_creates_parent_directories() {
        let dir = std::env::temp_dir()
            .join("anyflipdl_test_output")
            .join(uuid::Uuid::new_v4().to_string())
            .join("nested")
            .join("path");
        let path = dir.join("test.pdf");

        OutputFile::write(&path, b"data").unwrap();
        assert!(path.exists());

        let _ = std::fs::remove_dir_all(
            std::env::temp_dir()
                .join("anyflipdl_test_output"),
        );
    }

    #[test]
    fn verify_passes_for_nonempty_file() {
        let dir = std::env::temp_dir().join("anyflipdl_test_output").join(uuid::Uuid::new_v4().to_string());
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("test.pdf");
        std::fs::write(&path, b"data").unwrap();

        assert!(OutputFile::verify(&path).is_ok());

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn verify_fails_for_missing_file() {
        let path = Path::new("/nonexistent/path/file.pdf");
        let result = OutputFile::verify(path);
        assert!(result.is_err());
    }

    #[test]
    fn verify_fails_for_empty_file() {
        let dir = std::env::temp_dir().join("anyflipdl_test_output").join(uuid::Uuid::new_v4().to_string());
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("empty.pdf");
        std::fs::write(&path, b"").unwrap();

        let result = OutputFile::verify(&path);
        assert!(result.is_err());

        let _ = std::fs::remove_dir_all(&dir);
    }
}
