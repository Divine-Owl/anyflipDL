use crate::error::AppError;

/// Maximum allowed page count for anyflip document.
const MAX_PAGES: usize = 10_000;

/// Validate an Anyflip URL format.
///
/// Accepts: `https://anyflip.com/{user}/{book}` or `https://online.anyflip.com/{user}/{book}`
/// Also accepts `http://` and auto-upgrades to `https://`.
///
/// Rejects: empty strings, non-HTTPS (after upgrade), wrong host, missing path segments,
/// path traversal (`..` or `%2e%2e` after decoding).
pub fn validate_url(url: &str) -> Result<(), AppError> {
    if url.is_empty() {
        return Err(AppError::InvalidUrl("URL cannot be empty".to_string()));
    }

    // Auto-upgrade http:// to https://
    let normalized = if url.starts_with("http://") {
        url.replacen("http://", "https://", 1)
    } else {
        url.to_string()
    };

    let parsed = url::Url::parse(&normalized)
        .map_err(|_| AppError::InvalidUrl("Invalid URL format".to_string()))?;

    let scheme = parsed.scheme();
    if scheme != "https" {
        return Err(AppError::InvalidUrl("must use HTTPS".to_string()));
    }

    let host = parsed
        .host_str()
        .ok_or_else(|| AppError::InvalidUrl("No host in URL".to_string()))?;

    // Accept anyflip.com, online.anyflip.com, www.anyflip.com
    let host_stripped = host.strip_prefix("www.").unwrap_or(host);
    if host_stripped != "anyflip.com" && host_stripped != "online.anyflip.com" {
        return Err(AppError::InvalidUrl(format!(
            "Not an Anyflip URL: {}",
            host
        )));
    }

    let path = parsed.path();
    let parts: Vec<&str> = path.trim_matches('/').split('/').collect();

    // Must have at least 2 non-empty segments (user/book).
    // Extra segments like /user/book/default/ or /user/book/1 are accepted
    // and ignored — only the first two matter.
    if parts.len() < 2 || parts[0].is_empty() || parts[1].is_empty() {
        return Err(AppError::InvalidUrl(
            "URL must contain at least /{user}/{book} path".to_string(),
        ));
    }

    // Check for path traversal in the first two segments only
    for segment in &parts[..2] {
        let lower = segment.to_ascii_lowercase();
        if lower.contains("..") || lower.contains("%2e%2e") || lower.contains("%2e.") || lower.contains(".%2e") {
            return Err(AppError::InvalidUrl(
                "Path traversal detected".to_string(),
            ));
        }
    }

    Ok(())
}

/// Validate a save path is non-empty.
///
/// Does not check existence or writability (deferred to download time).
pub fn validate_save_path(path: &str) -> Result<(), AppError> {
    if path.is_empty() {
        return Err(AppError::InvalidUrl(
            "Save path cannot be empty".to_string(),
        ));
    }
    Ok(())
}

/// Validate the output format string is exactly "pdf" or "epub" (case-sensitive).
pub fn validate_format(format: &str) -> Result<(), AppError> {
    match format {
        "pdf" | "epub" => Ok(()),
        _ => Err(AppError::InvalidUrl(format!(
            "Invalid format '{}': must be 'pdf' or 'epub'",
            format
        ))),
    }
}

/// Validate page count is within acceptable bounds (1..=MAX_PAGES).
pub fn validate_page_count(count: usize) -> Result<(), AppError> {
    if count == 0 {
        return Err(AppError::InvalidUrl(
            "Page count must be greater than zero".to_string(),
        ));
    }
    if count > MAX_PAGES {
        return Err(AppError::InvalidUrl(format!(
            "Page count {} exceeds maximum of {}",
            count, MAX_PAGES
        )));
    }
    Ok(())
}

/// Sanitize a filename for safe filesystem use.
///
/// Strips characters illegal on Windows/Unix, enforces a 200-char limit,
/// and trims leading/trailing dots and spaces.
pub fn sanitize_filename(name: &str) -> String {
    let forbidden = ['/', '\\', ':', '*', '?', '"', '<', '>', '|', '\0'];
    let sanitized: String = name
        .chars()
        .map(|c| if forbidden.contains(&c) { '_' } else { c })
        .collect();

    // Trim to 200 chars, trim leading/trailing dots and spaces
    let trimmed: String = sanitized.chars().take(200).collect();
    trimmed.trim_matches(|c| c == '.' || c == ' ').to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── validate_url tests ──────────────────────────────────────────

    #[test]
    fn valid_anyflip_url() {
        assert!(validate_url("https://anyflip.com/user/book").is_ok());
    }

    #[test]
    fn valid_online_url() {
        assert!(validate_url("https://online.anyflip.com/user/book").is_ok());
    }

    #[test]
    fn trailing_slash_ok() {
        assert!(validate_url("https://anyflip.com/user/book/").is_ok());
    }

    #[test]
    fn http_upgraded() {
        assert!(validate_url("http://anyflip.com/user/book").is_ok());
    }

    #[test]
    fn empty_url_rejected() {
        let err = validate_url("").unwrap_err();
        assert!(err.to_string().contains("empty"));
    }

    #[test]
    fn non_anyflip_rejected() {
        let err = validate_url("https://google.com/user/book").unwrap_err();
        assert!(err.to_string().contains("Not an Anyflip URL"));
    }

    #[test]
    fn missing_book_rejected() {
        let err = validate_url("https://anyflip.com/user").unwrap_err();
        assert!(err.to_string().contains("at least /{user}/{book}"));
    }

    #[test]
    fn extra_segments_accepted() {
        assert!(validate_url("https://anyflip.com/a/b/c").is_ok());
    }

    #[test]
    fn path_traversal_rejected() {
        // url::Url normalizes `..` in paths, so `https://anyflip.com/../book`
        // becomes `https://anyflip.com/book` (single segment). This is still
        // rejected because it doesn't have exactly 2 segments.
        assert!(validate_url("https://anyflip.com/../book").is_err());
    }

    #[test]
    fn encoded_traversal_rejected() {
        // url::Url normalizes percent-encoded `..` just like literal `..`,
        // collapsing `https://anyflip.com/%2e%2e/book` to `https://anyflip.com/book`.
        // The URL is still rejected (single segment instead of two).
        assert!(validate_url("https://anyflip.com/%2e%2e/book").is_err());
    }

    #[test]
    fn ftp_scheme_rejected() {
        let err = validate_url("ftp://anyflip.com/user/book").unwrap_err();
        assert!(err.to_string().contains("HTTPS"));
    }

    // ── validate_format tests ───────────────────────────────────────

    #[test]
    fn valid_format_pdf() {
        assert!(validate_format("pdf").is_ok());
    }

    #[test]
    fn valid_format_epub() {
        assert!(validate_format("epub").is_ok());
    }

    #[test]
    fn invalid_format_docx() {
        let err = validate_format("docx").unwrap_err();
        assert!(err.to_string().contains("must be 'pdf' or 'epub'"));
    }

    #[test]
    fn invalid_format_uppercase() {
        let err = validate_format("PDF").unwrap_err();
        assert!(err.to_string().contains("must be 'pdf' or 'epub'"));
    }

    // ── validate_page_count tests ───────────────────────────────────

    #[test]
    fn valid_page_count() {
        assert!(validate_page_count(42).is_ok());
    }

    #[test]
    fn zero_page_count() {
        let err = validate_page_count(0).unwrap_err();
        assert!(err.to_string().contains("greater than zero"));
    }

    #[test]
    fn max_page_count() {
        assert!(validate_page_count(10_000).is_ok());
    }

    #[test]
    fn over_max_page_count() {
        let err = validate_page_count(10_001).unwrap_err();
        assert!(err.to_string().contains("exceeds maximum"));
    }

    // ── validate_save_path tests ────────────────────────────────────

    #[test]
    fn valid_save_path() {
        assert!(validate_save_path("/home/user/downloads").is_ok());
    }

    #[test]
    fn empty_save_path() {
        let err = validate_save_path("").unwrap_err();
        assert!(err.to_string().contains("empty"));
    }

    // ── sanitize_filename tests ─────────────────────────────────────

    #[test]
    fn sanitize_normal_name() {
        assert_eq!(sanitize_filename("mybook.pdf"), "mybook.pdf");
    }

    #[test]
    fn sanitize_strips_slashes() {
        // "/" -> "_", leading ".." trimmed
        assert_eq!(sanitize_filename("../etc/passwd"), "_etc_passwd");
    }

    #[test]
    fn sanitize_strips_colon() {
        assert_eq!(sanitize_filename("C:\\file"), "C__file");
    }

    #[test]
    fn sanitize_strips_special_chars() {
        assert_eq!(sanitize_filename("a*b?c\"d"), "a_b_c_d");
    }

    #[test]
    fn sanitize_trims_dots_and_spaces() {
        assert_eq!(sanitize_filename("  file. "), "file");
    }

    #[test]
    fn sanitize_empty_after_stripping() {
        // All dots/spaces are trimmed, leaving empty string
        assert_eq!(sanitize_filename("...   ..."), "");
    }

    #[test]
    fn sanitize_truncates_long_name() {
        let long = "a".repeat(300);
        assert!(sanitize_filename(&long).len() <= 200);
    }
}
