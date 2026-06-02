use crate::error::AppError;
use crate::services::validator;

/// Regex for allowed path segment characters: alphanumeric, hyphens, underscores.
const SEGMENT_PATTERN: &str = "^[a-zA-Z0-9_-]+$";

/// Parsed result of an Anyflip URL.
///
/// Contains the extracted `user` and `book` slugs, plus a normalized `base_url`
/// that always points to the `online.anyflip.com` CDN origin.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParsedUrl {
    /// The Anyflip user/author slug (e.g., "someuser").
    pub user: String,
    /// The book/document slug (e.g., "my-book-title").
    pub book: String,
    /// Normalized base URL: `https://online.anyflip.com/{user}/{book}`.
    pub base_url: String,
}

impl ParsedUrl {
    /// Build the config.js URL for this document.
    pub fn config_js_url(&self) -> String {
        format!("{}/mobile/javascript/config.js", self.base_url)
    }

    /// Build the page image URL for a given 1-based page index.
    pub fn page_url(&self, page: usize) -> String {
        format!("{}/files/mobile/{}.webp", self.base_url, page)
    }

    /// Build the large page image URL from a given filename.
    pub fn large_page_url(&self, filename: &str) -> String {
        format!("{}/files/large/{}", self.base_url, filename)
    }

    /// Build the temp directory name for this document (used by TempStorage).
    pub fn temp_dir_name(&self) -> String {
        format!("{}_{}", self.user, self.book)
    }
}

/// Parse an Anyflip URL and extract user + book path.
///
/// # Accepted formats
/// - `https://anyflip.com/{user}/{book}`
/// - `https://anyflip.com/{user}/{book}/` (trailing slash)
/// - `https://online.anyflip.com/{user}/{book}`
/// - `https://online.anyflip.com/{user}/{book}/`
///
/// # Returns
/// `ParsedUrl { user, book, base_url }` where base_url is normalized to
/// `https://online.anyflip.com/{user}/{book}` (the CDN origin).
///
/// # Errors
/// - `AppError::InvalidUrl` if URL is malformed, wrong host, or missing segments.
pub fn parse(url: &str) -> Result<ParsedUrl, AppError> {
    // Delegate to validator first
    validator::validate_url(url)?;

    // Auto-upgrade http:// to https:// for url::Url parsing
    let normalized = if url.starts_with("http://") {
        url.replacen("http://", "https://", 1)
    } else {
        url.to_string()
    };

    let parsed = url::Url::parse(&normalized)
        .map_err(|e| AppError::InvalidUrl(format!("Failed to parse URL: {}", e)))?;

    let path = parsed.path();
    let parts: Vec<&str> = path.trim_matches('/').split('/').collect();

    // Need at least user/book. Extra segments (default, page numbers) are ignored.
    if parts.len() < 2 || parts[0].is_empty() || parts[1].is_empty() {
        return Err(AppError::InvalidUrl(
            "URL must contain at least /{user}/{book} path".to_string(),
        ));
    }

    let user = parts[0].to_string();
    let book = parts[1].to_string();

    // Validate segment characters: [a-zA-Z0-9_-]+
    let re = regex::Regex::new(SEGMENT_PATTERN).expect("invalid regex constant");
    if !re.is_match(&user) || !re.is_match(&book) {
        return Err(AppError::InvalidUrl(format!(
            "Invalid path segment: user='{}', book='{}'",
            user, book
        )));
    }

    // Normalize base_url to online.anyflip.com (the CDN origin)
    let base_url = format!("https://online.anyflip.com/{}/{}", user, book);

    Ok(ParsedUrl {
        user,
        book,
        base_url,
    })
}

/// Convenience: validate + parse in one call. Used at command boundary.
///
/// This is functionally identical to `parse()` since `parse()` already calls
/// `validator::validate_url()` internally. Provided as an explicit entry point
/// for Tauri command handlers.
pub fn validate_and_parse(url: &str) -> Result<ParsedUrl, AppError> {
    parse(url)
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── Successful parsing ──────────────────────────────────────────

    #[test]
    fn parse_standard_url() {
        let result = parse("https://anyflip.com/johndoe/mybook").unwrap();
        assert_eq!(result.user, "johndoe");
        assert_eq!(result.book, "mybook");
    }

    #[test]
    fn parse_online_url() {
        let result = parse("https://online.anyflip.com/johndoe/mybook").unwrap();
        assert_eq!(result.user, "johndoe");
        assert_eq!(result.book, "mybook");
        assert_eq!(result.base_url, "https://online.anyflip.com/johndoe/mybook");
    }

    #[test]
    fn normalizes_to_online() {
        let result = parse("https://anyflip.com/u/b").unwrap();
        assert_eq!(result.base_url, "https://online.anyflip.com/u/b");
    }

    #[test]
    fn strips_trailing_slash() {
        let result = parse("https://anyflip.com/u/b/").unwrap();
        assert_eq!(result.user, "u");
        assert_eq!(result.book, "b");
    }

    #[test]
    fn http_upgraded_and_parsed() {
        let result = parse("http://anyflip.com/user/book").unwrap();
        assert_eq!(result.user, "user");
        assert_eq!(result.book, "book");
        assert!(result.base_url.starts_with("https://"));
    }

    #[test]
    fn hyphens_in_segments() {
        let result = parse("https://anyflip.com/my-user/my-book").unwrap();
        assert_eq!(result.user, "my-user");
        assert_eq!(result.book, "my-book");
    }

    #[test]
    fn underscores_in_segments() {
        let result = parse("https://anyflip.com/my_user/my_book").unwrap();
        assert_eq!(result.user, "my_user");
        assert_eq!(result.book, "my_book");
    }

    #[test]
    fn numeric_segments() {
        let result = parse("https://anyflip.com/user123/book456").unwrap();
        assert_eq!(result.user, "user123");
        assert_eq!(result.book, "book456");
    }

    // ── Derived URL methods ─────────────────────────────────────────

    #[test]
    fn config_js_url() {
        let result = parse("https://anyflip.com/johndoe/mybook").unwrap();
        assert_eq!(
            result.config_js_url(),
            "https://online.anyflip.com/johndoe/mybook/mobile/javascript/config.js"
        );
    }

    #[test]
    fn page_url() {
        let result = parse("https://anyflip.com/johndoe/mybook").unwrap();
        assert_eq!(
            result.page_url(5),
            "https://online.anyflip.com/johndoe/mybook/files/mobile/5.webp"
        );
    }

    #[test]
    fn page_url_first_page() {
        let result = parse("https://anyflip.com/u/b").unwrap();
        assert_eq!(
            result.page_url(1),
            "https://online.anyflip.com/u/b/files/mobile/1.webp"
        );
    }

    #[test]
    fn large_page_url() {
        let result = parse("https://anyflip.com/johndoe/mybook").unwrap();
        assert_eq!(
            result.large_page_url("page005.webp"),
            "https://online.anyflip.com/johndoe/mybook/files/large/page005.webp"
        );
    }

    #[test]
    fn temp_dir_name() {
        let result = parse("https://anyflip.com/a/b").unwrap();
        assert_eq!(result.temp_dir_name(), "a_b");
    }

    // ── Rejection tests ─────────────────────────────────────────────

    #[test]
    fn rejects_empty_url() {
        assert!(parse("").is_err());
    }

    #[test]
    fn rejects_invalid_host() {
        assert!(parse("https://evil.com/u/b").is_err());
    }

    #[test]
    fn rejects_path_traversal() {
        assert!(parse("https://anyflip.com/../b").is_err());
    }

    #[test]
    fn rejects_special_chars_in_user() {
        let err = parse("https://anyflip.com/u@ser/book").unwrap_err();
        assert!(err.to_string().contains("Invalid path segment"));
    }

    #[test]
    fn rejects_special_chars_in_book() {
        let err = parse("https://anyflip.com/user/b%40ok").unwrap_err();
        assert!(err.to_string().contains("Invalid path segment"));
    }

    #[test]
    fn rejects_missing_book() {
        assert!(parse("https://anyflip.com/user").is_err());
    }

    #[test]
    fn accepts_extra_segments() {
        let result = parse("https://anyflip.com/a/b/c").unwrap();
        assert_eq!(result.user, "a");
        assert_eq!(result.book, "b");
    }

    #[test]
    // https://anyflip.com/ (no segments at all)
    fn rejects_bare_domain() {
        assert!(parse("https://anyflip.com/").is_err());
    }

    // ── validate_and_parse tests ────────────────────────────────────

    #[test]
    fn validate_and_parse_delegates_correctly() {
        let result = validate_and_parse("https://anyflip.com/user/book").unwrap();
        assert_eq!(result.user, "user");
        assert_eq!(result.book, "book");
    }

    #[test]
    fn validate_and_parse_rejects_invalid() {
        assert!(validate_and_parse("https://google.com/user/book").is_err());
    }

    // ── PartialEq / Clone ───────────────────────────────────────────

    #[test]
    fn parsed_url_equality() {
        let a = parse("https://anyflip.com/u/b").unwrap();
        let b = parse("https://online.anyflip.com/u/b").unwrap();
        assert_eq!(a, b);
    }

    #[test]
    fn parsed_url_clone() {
        let a = parse("https://anyflip.com/u/b").unwrap();
        let b = a.clone();
        assert_eq!(a, b);
    }
}
