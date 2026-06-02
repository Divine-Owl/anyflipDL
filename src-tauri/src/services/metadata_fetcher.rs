use crate::error::AppError;
use crate::models::DocumentMetadata;
use crate::services::url_parser::ParsedUrl;
use regex::Regex;
use serde::Deserialize;
use std::sync::LazyLock;

/// Regex to extract the JSON object from Anyflip's JS wrapper.
///
/// Matches the first `{...}` block in the response, using dotall mode (`(?s)`)
/// so that `.` matches newlines. Compiled once via `LazyLock`.
static CONFIG_JSON_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?s)\{.*\}").expect("invalid regex constant"));

/// Raw structure matching Anyflip's config.js shape.
///
/// Handles multiple known field name variants via serde aliases so that
/// different Anyflip hosting versions are supported transparently.
#[derive(Debug, Deserialize)]
struct RawConfig {
    #[serde(alias = "docTitle", alias = "title")]
    title: Option<String>,
    #[serde(alias = "pageCount", alias = "totalPages", alias = "numPages")]
    page_count: Option<usize>,
    #[serde(alias = "pageFileNames", alias = "pages")]
    page_filenames: Option<Vec<String>>,
    #[serde(
        alias = "date",
        alias = "createDate",
        alias = "bookCreateDate",
        alias = "publishDate"
    )]
    date: Option<String>,
}

/// Fetch document metadata from Anyflip's config.js endpoint.
///
/// # Arguments
/// - `client`: Shared reqwest client (configured with timeout, user-agent).
/// - `parsed_url`: Validated `ParsedUrl` from `url_parser::parse`.
///
/// # Returns
/// `DocumentMetadata` with `title`, `page_count`, and `page_filenames`.
///
/// # Errors
/// - `AppError::NetworkError` if the HTTP request fails (timeout, DNS, connection).
/// - `AppError::MetadataError` if HTTP status is not 2xx, response is not valid
///   UTF-8, config.js contains no parseable JSON, or `page_count` is missing / 0.
pub async fn fetch(
    client: &reqwest::Client,
    parsed_url: &ParsedUrl,
) -> Result<DocumentMetadata, AppError> {
    let config_url = parsed_url.config_js_url();
    tracing::info!("Fetching metadata from: {}", config_url);

    let response = client.get(&config_url).send().await.map_err(|e| {
        tracing::error!("HTTP request to {} failed: {:?}", config_url, e);
        AppError::NetworkError(e.to_string())
    })?;

    if !response.status().is_success() {
        let status = response.status();
        tracing::error!("config.js returned HTTP {} for {}", status, config_url);

        // HTTP 403 on config.js indicates the document is password-protected.
        if status == reqwest::StatusCode::FORBIDDEN {
            return Err(AppError::PasswordRequired(
                "Document is password-protected".to_string(),
            ));
        }

        return Err(AppError::MetadataError(format!(
            "Failed to fetch config.js: HTTP {}",
            status
        )));
    }

    let body = response.text().await.map_err(|e| {
        tracing::error!("Failed to read config.js body: {:?}", e);
        AppError::NetworkError(e.to_string())
    })?;

    let config = parse_config_js(&body)?;

    // If page count is missing (new config format), probe pages to determine count.
    let page_count = config.get("numPages").and_then(|v| v.as_u64()).map(|n| n as usize);
    if page_count.is_none() || page_count == Some(0) {
        tracing::info!("Page count not found in config.js, probing pages...");
        let count = probe_page_count(client, parsed_url).await?;
        let mut config = config;
        if let serde_json::Value::Object(ref mut map) = config {
            map.insert("numPages".to_string(), serde_json::Value::Number(count.into()));
            if !map.contains_key("title") {
                map.insert("title".to_string(), serde_json::Value::String("Untitled".to_string()));
            }
        }
        return extract_metadata(config, parsed_url);
    }

    extract_metadata(config, parsed_url)
}

/// Fetch document metadata with a password for protected documents.
///
/// Appends the password as a query parameter to the config.js URL, which is
/// how Anyflip validates document access passwords.
///
/// # Arguments
/// - `client`: Shared reqwest client.
/// - `parsed_url`: Validated `ParsedUrl`.
/// - `password`: The document access password.
///
/// # Errors
/// Same as [`fetch`], plus `AppError::PasswordRequired` if the password is
/// incorrect (HTTP 403 persists).
pub async fn fetch_with_password(
    client: &reqwest::Client,
    parsed_url: &ParsedUrl,
    password: &str,
) -> Result<DocumentMetadata, AppError> {
    let config_url = format!("{}?password={}", parsed_url.config_js_url(), password);
    tracing::info!("Fetching metadata with password from: {}", config_url);

    let response = client.get(&config_url).send().await.map_err(|e| {
        tracing::error!("HTTP request to {} failed: {:?}", config_url, e);
        AppError::NetworkError(e.to_string())
    })?;

    if !response.status().is_success() {
        let status = response.status();
        tracing::error!("config.js returned HTTP {} for {}", status, config_url);

        if status == reqwest::StatusCode::FORBIDDEN {
            return Err(AppError::PasswordRequired(
                "Incorrect password or document is still protected".to_string(),
            ));
        }

        return Err(AppError::MetadataError(format!(
            "Failed to fetch config.js: HTTP {}",
            status
        )));
    }

    let body = response.text().await.map_err(|e| {
        tracing::error!("Failed to read config.js body: {:?}", e);
        AppError::NetworkError(e.to_string())
    })?;

    let config = parse_config_js(&body)?;

    // If page count is missing (new config format), probe pages to determine count.
    let page_count = config.get("numPages").and_then(|v| v.as_u64()).map(|n| n as usize);
    if page_count.is_none() || page_count == Some(0) {
        tracing::info!("Page count not found in config.js (password flow), probing pages...");
        let count = probe_page_count(client, parsed_url).await?;
        let mut config = config;
        if let serde_json::Value::Object(ref mut map) = config {
            map.insert("numPages".to_string(), serde_json::Value::Number(count.into()));
            if !map.contains_key("title") {
                map.insert("title".to_string(), serde_json::Value::String("Untitled".to_string()));
            }
        }
        let mut metadata = extract_metadata(config, parsed_url)?;
        metadata.password_protected = true;
        return Ok(metadata);
    }

    let mut metadata = extract_metadata(config, parsed_url)?;
    metadata.password_protected = true;
    Ok(metadata)
}

/// Strip the JavaScript wrapper from Anyflip's config.js and parse the
/// embedded JSON object.
///
/// Supports two config.js formats:
///
/// **Old format:**
/// ```javascript
/// var config = { "docTitle": "...", "pageCount": 10 };
/// ```
///
/// **New format (2024+):**
/// ```javascript
/// var htmlConfig = {
///   "bookConfig": "<base64-encoded navigation payload>",
///   "meta": { "title": "...", "pageCount": 308, ... },
///   "fliphtml5_pages": [{ "n": ["../files/mobile/1.webp"], "t": "..." }]
/// };
/// ```
/// For the new format, the `bookConfig` string is base64-encoded navigation
/// data used by the player and is not needed for our metadata. We extract
/// `title` and `pageCount` from the sibling `meta` object, and
/// `pageFileNames` from `fliphtml5_pages`.
///
/// # Errors
/// Returns `AppError::MetadataError` if no JSON object is found, if the
/// extracted text is not valid JSON, or if the new format is missing both
/// the `meta` object and `fliphtml5_pages`.
fn parse_config_js(raw_js: &str) -> Result<serde_json::Value, AppError> {
    let json_match = CONFIG_JSON_RE
        .find(raw_js)
        .ok_or_else(|| {
            tracing::error!("No JSON object found in config.js response");
            AppError::MetadataError("No JSON object found in config.js".to_string())
        })?;

    let json_str = json_match.as_str();
    let value: serde_json::Value = serde_json::from_str(json_str).map_err(|e| {
        tracing::error!("Failed to parse JSON from config.js: {:?}", e);
        AppError::MetadataError(format!("Unparseable config.js JSON: {}", e))
    })?;

    if value.get("bookConfig").is_some() {
        tracing::info!("Detected new htmlConfig format, extracting from meta object");
        return extract_metadata_from_new_format(&value);
    }

    Ok(value)
}

/// Extract metadata from the new `htmlConfig` format (2024+).
///
/// Pulls `title` and `pageCount` from the `meta` sibling object, and
/// `pageFileNames` from `fliphtml5_pages` (first element of each entry's
/// `n` array). The base64-encoded `bookConfig` string itself is a navigation
/// payload used by the player and is not needed for our metadata.
///
/// # Errors
/// Returns `AppError::MetadataError` if both the `meta` object and
/// `fliphtml5_pages` are absent or empty.
fn extract_metadata_from_new_format(
    value: &serde_json::Value,
) -> Result<serde_json::Value, AppError> {
    let mut config = serde_json::Map::new();

    if let Some(meta) = value.get("meta").and_then(|v| v.as_object()) {
        if let Some(title) = meta.get("title").and_then(|v| v.as_str()) {
            if !title.is_empty() {
                config.insert(
                    "title".to_string(),
                    serde_json::Value::String(title.to_string()),
                );
            }
        }
        if let Some(count) = meta.get("pageCount").and_then(|v| v.as_u64()) {
            if count > 0 {
                config.insert(
                    "numPages".to_string(),
                    serde_json::Value::Number(count.into()),
                );
            }
        }
    }

    if let Some(pages) = value.get("fliphtml5_pages").and_then(|v| v.as_array()) {
        let filenames: Vec<String> = pages
            .iter()
            .filter_map(|p| {
                p.get("n")
                    .and_then(|n| n.as_array())
                    .and_then(|arr| arr.first())
                    .and_then(|v| v.as_str())
                    .map(basename_from_path)
            })
            .collect();
        if !filenames.is_empty() {
            config.insert(
                "pageFileNames".to_string(),
                serde_json::Value::Array(
                    filenames
                        .into_iter()
                        .map(serde_json::Value::String)
                        .collect(),
                ),
            );
        }
    }

    if config.is_empty() {
        return Err(AppError::MetadataError(
            "New-format config.js missing both meta and fliphtml5_pages".to_string(),
        ));
    }

    tracing::info!(
        "Extracted from new format: title={:?}, numPages={:?}, pageFileNames={:?}",
        config.get("title"),
        config.get("numPages"),
        config
            .get("pageFileNames")
            .and_then(|v| v.as_array())
            .map(|a| a.len())
    );

    Ok(serde_json::Value::Object(config))
}

/// Extract the bare filename (e.g. `"abc123.webp"`) from a path that may
/// contain a directory prefix (e.g. `"../files/mobile/abc123.webp"`).
///
/// Anyflip's new-format config.js stores `n` (normal/full-size) and `t`
/// (thumbnail) entries as relative paths like `../files/mobile/HASH.webp`.
/// The page downloader expects bare basenames so it can join them onto
/// `files/large/<filename>` (and the corresponding `files/thumb/<filename>`
/// for previews). If the prefix leaks through, the resulting URL becomes
/// `.../files/large/files/mobile/HASH.webp` which returns HTTP 403 from
/// Anyflip's CDN and is misclassified as a password error.
///
/// If the input has no path separators, or ends in `..` with no filename,
/// the original string is returned unchanged so the downstream error path
/// is preserved (the page downloader will still surface a clean failure).
fn basename_from_path(path: &str) -> String {
    std::path::Path::new(path)
        .file_name()
        .and_then(|f| f.to_str())
        .map(|s| s.to_string())
        .unwrap_or_else(|| path.to_string())
}

/// Extract `DocumentMetadata` from a parsed JSON value.
///
/// Uses the `RawConfig` struct with serde aliases to accept multiple known
/// field name variants (`docTitle`/`title`, `pageCount`/`totalPages`/`numPages`,
/// `pageFileNames`/`pages`).
///
/// # Errors
/// Returns `AppError::MetadataError` if `page_count` is missing or 0.
fn extract_metadata(
    config: serde_json::Value,
    parsed_url: &ParsedUrl,
) -> Result<DocumentMetadata, AppError> {
    let raw: RawConfig = serde_json::from_value(config).map_err(|e| {
        tracing::error!("Failed to deserialize RawConfig: {:?}", e);
        AppError::MetadataError(format!("Invalid config.js structure: {}", e))
    })?;

    let title = raw.title.unwrap_or_else(|| {
        tracing::warn!("config.js missing title field, defaulting to 'Untitled'");
        "Untitled".to_string()
    });

    let page_count = raw.page_count.ok_or_else(|| {
        tracing::error!("config.js missing page count field");
        AppError::MetadataError("Missing page count in config.js".to_string())
    })?;

    if page_count == 0 {
        tracing::error!("config.js reports 0 pages");
        return Err(AppError::MetadataError(
            "Page count is 0 in config.js".to_string(),
        ));
    }

    let page_filenames = raw.page_filenames.unwrap_or_default();
    let date = raw.date;
    let thumbnail = derive_thumbnail_url(parsed_url);

    tracing::info!(
        "Parsed metadata: title='{}', pages={}, filenames={}, date={:?}, thumbnail={:?}",
        title,
        page_count,
        page_filenames.len(),
        date,
        thumbnail
    );

    Ok(DocumentMetadata {
        url: parsed_url.base_url.clone(),
        title,
        page_count,
        page_filenames,
        author: Some(parsed_url.user.clone()),
        date,
        thumbnail,
        password_protected: false,
    })
}

/// Derive a cover-image URL for the document.
///
/// Anyflip serves the largest page render under `files/large/`, and page 1 is
/// the cover. Returns `None` if the base URL is empty. The frontend uses an
/// `<img onerror>` fallback to a book icon if the URL 404s on a particular
/// document.
fn derive_thumbnail_url(parsed_url: &ParsedUrl) -> Option<String> {
    if parsed_url.base_url.is_empty() {
        return None;
    }
    Some(format!("{}/files/large/1.jpg", parsed_url.base_url))
}

/// Probe page count by requesting pages until we get a non-200 response.
///
/// Uses exponential probing: check pages 1, 2, 4, 8, 16, 32, 64... until 403/404,
/// then binary search within the last interval.
///
/// This is used when the config.js format doesn't contain the page count
/// (new Anyflip format with encoded `bookConfig` string).
async fn probe_page_count(
    client: &reqwest::Client,
    parsed_url: &ParsedUrl,
) -> Result<u32, AppError> {
    let base_url = &parsed_url.base_url;
    let referer = format!("{}/", base_url);

    // Exponential probing: check 1, 2, 4, 8, 16, 32...
    let mut last_ok = 0u32;
    let mut probe = 1u32;

    loop {
        let url = format!("{}/files/mobile/{}.webp", base_url, probe);
        let status = client
            .get(&url)
            .header("Referer", &referer)
            .send()
            .await
            .map(|r| r.status())
            .unwrap_or(reqwest::StatusCode::NOT_FOUND);

        if status == reqwest::StatusCode::OK {
            last_ok = probe;
            probe *= 2;
            if probe > 10000 {
                // Safety limit
                return Err(AppError::MetadataError(
                    "Document has more than 10000 pages".to_string(),
                ));
            }
        } else {
            break;
        }
    }

    if last_ok == 0 {
        return Err(AppError::MetadataError(
            "No pages found for document".to_string(),
        ));
    }

    // If we found pages, binary search between last_ok and probe
    if probe > last_ok + 1 {
        let mut lo = last_ok + 1;
        let mut hi = probe - 1;
        while lo <= hi {
            let mid = (lo + hi) / 2;
            let url = format!("{}/files/mobile/{}.webp", base_url, mid);
            let status = client
                .get(&url)
                .header("Referer", &referer)
                .send()
                .await
                .map(|r| r.status())
                .unwrap_or(reqwest::StatusCode::NOT_FOUND);

            if status == reqwest::StatusCode::OK {
                last_ok = mid;
                lo = mid + 1;
            } else {
                hi = mid - 1;
            }
        }
    }

    tracing::info!("Probed page count: {}", last_ok);
    Ok(last_ok)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::services::url_parser;

    // ── Helpers ────────────────────────────────────────────────────────

    /// Build a `ParsedUrl` for testing convenience.
    fn test_parsed_url() -> ParsedUrl {
        url_parser::parse("https://anyflip.com/testuser/testbook").unwrap()
    }

    // ── parse_config_js tests ─────────────────────────────────────────

    #[test]
    fn parse_standard_config_js() {
        let input = r#"var config = {"docTitle":"X","pageCount":10};"#;
        let val = parse_config_js(input).unwrap();
        assert_eq!(val["docTitle"], "X");
        assert_eq!(val["pageCount"], 10);
    }

    #[test]
    fn parse_let_config() {
        let input = r#"let config = {"docTitle":"Y","pageCount":5};"#;
        let val = parse_config_js(input).unwrap();
        assert_eq!(val["docTitle"], "Y");
    }

    #[test]
    fn parse_const_config() {
        let input = r#"const config = {"docTitle":"Z","pageCount":3};"#;
        let val = parse_config_js(input).unwrap();
        assert_eq!(val["docTitle"], "Z");
    }

    #[test]
    fn parse_with_whitespace() {
        let input = r#"
            var config = {
                "docTitle": "Spaced Out",
                "pageCount": 42
            };
        "#;
        let val = parse_config_js(input).unwrap();
        assert_eq!(val["docTitle"], "Spaced Out");
        assert_eq!(val["pageCount"], 42);
    }

    #[test]
    fn parse_no_wrapper_raw_json() {
        let input = r#"{"docTitle":"Raw","pageCount":7}"#;
        let val = parse_config_js(input).unwrap();
        assert_eq!(val["docTitle"], "Raw");
    }

    #[test]
    fn parse_invalid_js_returns_error() {
        let input = "not valid at all";
        let err = parse_config_js(input).unwrap_err();
        assert!(err.to_string().contains("No JSON object"));
    }

    #[test]
    fn parse_malformed_json_returns_error() {
        let input = r#"var config = {invalid json};"#;
        let err = parse_config_js(input).unwrap_err();
        assert!(err.to_string().contains("Unparseable"));
    }

    // ── new htmlConfig format tests (2024+) ───────────────────────────

    #[test]
    fn parse_new_format_extracts_title_and_page_count_from_meta() {
        let input = r#"var htmlConfig = {
            "bookConfig": "v010001102856JepyGfqw...",
            "meta": {
                "title": "Monogatari Series Volume 1",
                "pageCount": 308,
                "url": "https://online.anyflip.com/jjyfm/ikur/index.html"
            }
        };"#;
        let val = parse_config_js(input).unwrap();
        assert_eq!(val["title"], "Monogatari Series Volume 1");
        assert_eq!(val["numPages"], 308);
    }

    #[test]
    fn parse_new_format_extracts_page_filenames_from_fliphtml5_pages() {
        let input = r#"var htmlConfig = {
            "bookConfig": "v01abc...",
            "meta": { "title": "X", "pageCount": 2 },
            "fliphtml5_pages": [
                {"n": ["../files/mobile/1.webp"], "t": "../files/thumb/1.webp"},
                {"n": ["../files/mobile/2.webp"], "t": "../files/thumb/2.webp"}
            ]
        };"#;
        let val = parse_config_js(input).unwrap();
        assert_eq!(val["title"], "X");
        assert_eq!(val["numPages"], 2);
        let files = val["pageFileNames"].as_array().unwrap();
        assert_eq!(files.len(), 2);
        assert_eq!(files[0], "1.webp");
        assert_eq!(files[1], "2.webp");
    }

    #[test]
    fn parse_new_format_ignores_empty_title() {
        let input = r#"var htmlConfig = {
            "bookConfig": "v01abc...",
            "meta": { "title": "", "pageCount": 5 }
        };"#;
        let val = parse_config_js(input).unwrap();
        assert!(val.get("title").is_none(), "empty title should not be inserted");
        assert_eq!(val["numPages"], 5);
    }

    #[test]
    fn parse_new_format_ignores_zero_page_count() {
        let input = r#"var htmlConfig = {
            "bookConfig": "v01abc...",
            "meta": { "title": "X", "pageCount": 0 }
        };"#;
        let val = parse_config_js(input).unwrap();
        assert_eq!(val["title"], "X");
        assert!(val.get("numPages").is_none(), "zero pageCount should not be inserted");
    }

    #[test]
    fn parse_new_format_missing_meta_and_fliphtml5_pages_returns_error() {
        let input = r#"var htmlConfig = { "bookConfig": "v01abc...", "bmtConfig": {} };"#;
        let err = parse_config_js(input).unwrap_err();
        assert!(err.to_string().contains("missing both meta and fliphtml5_pages"));
    }

    #[test]
    fn parse_new_format_meta_only_no_pages_still_works() {
        let input = r#"var htmlConfig = {
            "bookConfig": "v01abc...",
            "meta": { "title": "Solo", "pageCount": 7 }
        };"#;
        let val = parse_config_js(input).unwrap();
        assert_eq!(val["title"], "Solo");
        assert_eq!(val["numPages"], 7);
        assert!(val.get("pageFileNames").is_none());
    }

    #[test]
    fn parse_new_format_real_fixture_from_jjyfm_ikur() {
        // Sanitized excerpt of the live response from
        // https://online.anyflip.com/jjyfm/ikur/mobile/javascript/config.js
        let input = r#"var htmlConfig = {
            "bookConfig": "v010001102856JepyGfqwGXo3AdmvJej8PSk3Jeu6MTszBdn8ESsvKSh3Hah...",
            "meta": {
                "title": "Monogatari Series Volume 1 - Bakemonogatari Part 1",
                "description": "Bakemonogatari Part 1",
                "url": "https://online.anyflip.com/jjyfm/ikur/index.html",
                "pageCount": 308,
                "htmlTemplate": "Handy"
            },
            "bmtConfig": {"duration": 500},
            "fliphtml5_pages": [
                {"n": ["../files/mobile/1.webp"], "t": "../files/thumb/1.webp"},
                {"n": ["../files/mobile/2.webp"], "t": "../files/thumb/2.webp"},
                {"n": ["../files/mobile/308.webp"], "t": "../files/thumb/308.webp"}
            ]
        };"#;
        let val = parse_config_js(input).unwrap();
        assert_eq!(
            val["title"],
            "Monogatari Series Volume 1 - Bakemonogatari Part 1"
        );
        assert_eq!(val["numPages"], 308);
        let files = val["pageFileNames"].as_array().unwrap();
        assert_eq!(files.len(), 3);
        assert_eq!(files[0], "1.webp");
        assert_eq!(files[2], "308.webp");
    }

    #[test]
    fn full_pipeline_new_format() {
        let input = r#"var htmlConfig = {
            "bookConfig": "v01abc...",
            "meta": { "title": "Pipeline Book", "pageCount": 12 },
            "fliphtml5_pages": [
                {"n": ["../files/mobile/1.webp"], "t": "../files/thumb/1.webp"}
            ]
        };"#;
        let config = parse_config_js(input).unwrap();
        let meta = extract_metadata(config, &test_parsed_url()).unwrap();
        assert_eq!(meta.title, "Pipeline Book");
        assert_eq!(meta.page_count, 12);
        assert_eq!(meta.page_filenames, vec!["1.webp"]);
        assert!(!meta.password_protected);
    }

    #[test]
    fn new_format_strips_relative_path_prefix_from_page_filenames() {
        // Regression: new-format config.js stores filenames as
        // "../files/mobile/HASH.webp" but page_downloader.page_url expects
        // a bare basename (e.g. "HASH.webp"). When the prefix leaks into
        // page_filenames, the constructed URL becomes
        // `.../files/large/files/mobile/HASH.webp` which returns 403 and
        // is (incorrectly) classified as a password error.
        let input = r#"var htmlConfig = {
            "bookConfig": "v01abc...",
            "meta": { "title": "Hashed Doc", "pageCount": 3 },
            "fliphtml5_pages": [
                {"n": ["../files/mobile/abc123.webp"]},
                {"n": ["../files/mobile/def456.webp"]},
                {"n": ["../files/mobile/ghi789.webp"]}
            ]
        };"#;
        let config = parse_config_js(input).unwrap();
        let meta = extract_metadata(config, &test_parsed_url()).unwrap();
        assert_eq!(
            meta.page_filenames,
            vec!["abc123.webp", "def456.webp", "ghi789.webp"],
            "page_filenames should be bare basenames, not relative paths"
        );
    }

    #[test]
    fn new_format_legacy_numeric_naming_also_strips_prefix() {
        // Older documents use numeric names like "../files/mobile/1.webp".
        // These must also be normalized so the page_downloader contract
        // (bare basename) holds uniformly.
        let input = r#"var htmlConfig = {
            "bookConfig": "v01abc...",
            "meta": { "title": "Legacy Doc", "pageCount": 2 },
            "fliphtml5_pages": [
                {"n": ["../files/mobile/1.webp"]},
                {"n": ["../files/mobile/2.webp"]}
            ]
        };"#;
        let config = parse_config_js(input).unwrap();
        let meta = extract_metadata(config, &test_parsed_url()).unwrap();
        assert_eq!(meta.page_filenames, vec!["1.webp", "2.webp"]);
    }

    // ── extract_metadata tests ────────────────────────────────────────

    #[test]
    fn extract_with_doc_title() {
        let config = serde_json::json!({"docTitle": "My Doc", "pageCount": 10});
        let meta = extract_metadata(config, &test_parsed_url()).unwrap();
        assert_eq!(meta.title, "My Doc");
    }

    #[test]
    fn extract_with_title_alias() {
        let config = serde_json::json!({"title": "My Doc", "pageCount": 10});
        let meta = extract_metadata(config, &test_parsed_url()).unwrap();
        assert_eq!(meta.title, "My Doc");
    }

    #[test]
    fn extract_missing_title_defaults_to_untitled() {
        let config = serde_json::json!({"pageCount": 10});
        let meta = extract_metadata(config, &test_parsed_url()).unwrap();
        assert_eq!(meta.title, "Untitled");
    }

    #[test]
    fn extract_page_count() {
        let config = serde_json::json!({"docTitle": "X", "pageCount": 42});
        let meta = extract_metadata(config, &test_parsed_url()).unwrap();
        assert_eq!(meta.page_count, 42);
    }

    #[test]
    fn extract_page_count_alias_total_pages() {
        let config = serde_json::json!({"docTitle": "X", "totalPages": 99});
        let meta = extract_metadata(config, &test_parsed_url()).unwrap();
        assert_eq!(meta.page_count, 99);
    }

    #[test]
    fn extract_page_count_alias_num_pages() {
        let config = serde_json::json!({"docTitle": "X", "numPages": 7});
        let meta = extract_metadata(config, &test_parsed_url()).unwrap();
        assert_eq!(meta.page_count, 7);
    }

    #[test]
    fn extract_zero_pages_returns_error() {
        let config = serde_json::json!({"docTitle": "X", "pageCount": 0});
        let err = extract_metadata(config, &test_parsed_url()).unwrap_err();
        assert!(err.to_string().contains("Page count is 0"));
    }

    #[test]
    fn extract_missing_page_count_returns_error() {
        let config = serde_json::json!({"docTitle": "X"});
        let err = extract_metadata(config, &test_parsed_url()).unwrap_err();
        assert!(err.to_string().contains("Missing page count"));
    }

    #[test]
    fn extract_with_filenames() {
        let config = serde_json::json!({
            "docTitle": "X",
            "pageCount": 2,
            "pageFileNames": ["a.jpg", "b.jpg"]
        });
        let meta = extract_metadata(config, &test_parsed_url()).unwrap();
        assert_eq!(meta.page_filenames, vec!["a.jpg", "b.jpg"]);
    }

    #[test]
    fn extract_with_filenames_alias_pages() {
        let config = serde_json::json!({
            "docTitle": "X",
            "pageCount": 2,
            "pages": ["c.webp", "d.webp"]
        });
        let meta = extract_metadata(config, &test_parsed_url()).unwrap();
        assert_eq!(meta.page_filenames, vec!["c.webp", "d.webp"]);
    }

    #[test]
    fn extract_without_filenames_defaults_to_empty() {
        let config = serde_json::json!({"docTitle": "X", "pageCount": 5});
        let meta = extract_metadata(config, &test_parsed_url()).unwrap();
        assert!(meta.page_filenames.is_empty());
    }

    #[test]
    fn extract_sets_url_from_parsed_url() {
        let config = serde_json::json!({"docTitle": "X", "pageCount": 1});
        let meta = extract_metadata(config, &test_parsed_url()).unwrap();
        assert_eq!(meta.url, "https://online.anyflip.com/testuser/testbook");
    }

    #[test]
    fn extract_all_fields_present() {
        let config = serde_json::json!({
            "docTitle": "Full Document",
            "pageCount": 3,
            "pageFileNames": ["1.jpg", "2.jpg", "3.jpg"]
        });
        let meta = extract_metadata(config, &test_parsed_url()).unwrap();
        assert_eq!(meta.title, "Full Document");
        assert_eq!(meta.page_count, 3);
        assert_eq!(meta.page_filenames.len(), 3);
        assert_eq!(
            meta.url,
            "https://online.anyflip.com/testuser/testbook"
        );
    }

    // ── End-to-end parse + extract ─────────────────────────────────────

    #[test]
    fn full_pipeline_var_config() {
        let input = r#"var config = {"docTitle":"Pipeline Test","pageCount":25,"pageFileNames":["1.jpg"]};"#;
        let config = parse_config_js(input).unwrap();
        let meta = extract_metadata(config, &test_parsed_url()).unwrap();
        assert_eq!(meta.title, "Pipeline Test");
        assert_eq!(meta.page_count, 25);
        assert_eq!(meta.page_filenames, vec!["1.jpg"]);
    }

    #[test]
    fn full_pipeline_with_aliases() {
        let input = r#"let config = {"title":"Aliased","totalPages":8,"pages":["a.webp"]};"#;
        let config = parse_config_js(input).unwrap();
        let meta = extract_metadata(config, &test_parsed_url()).unwrap();
        assert_eq!(meta.title, "Aliased");
        assert_eq!(meta.page_count, 8);
        assert_eq!(meta.page_filenames, vec!["a.webp"]);
    }

    #[test]
    fn full_pipeline_minimal_valid() {
        let input = r#"const config = {"pageCount":1};"#;
        let config = parse_config_js(input).unwrap();
        let meta = extract_metadata(config, &test_parsed_url()).unwrap();
        assert_eq!(meta.title, "Untitled");
        assert_eq!(meta.page_count, 1);
        assert!(meta.page_filenames.is_empty());
    }

    // ── date field tests ──────────────────────────────────────────────

    #[test]
    fn extract_with_date() {
        let config = serde_json::json!({"docTitle": "X", "pageCount": 5, "date": "2024-08-15"});
        let meta = extract_metadata(config, &test_parsed_url()).unwrap();
        assert_eq!(meta.date, Some("2024-08-15".to_string()));
    }

    #[test]
    fn extract_date_alias_create_date() {
        let config = serde_json::json!({"docTitle": "X", "pageCount": 5, "createDate": "2023-01-01"});
        let meta = extract_metadata(config, &test_parsed_url()).unwrap();
        assert_eq!(meta.date, Some("2023-01-01".to_string()));
    }

    #[test]
    fn extract_date_alias_book_create_date() {
        let config = serde_json::json!({"docTitle": "X", "pageCount": 5, "bookCreateDate": "March 3, 2022"});
        let meta = extract_metadata(config, &test_parsed_url()).unwrap();
        assert_eq!(meta.date, Some("March 3, 2022".to_string()));
    }

    #[test]
    fn extract_date_alias_publish_date() {
        let config = serde_json::json!({"docTitle": "X", "pageCount": 5, "publishDate": "2021-12-25"});
        let meta = extract_metadata(config, &test_parsed_url()).unwrap();
        assert_eq!(meta.date, Some("2021-12-25".to_string()));
    }

    #[test]
    fn extract_missing_date_is_none() {
        let config = serde_json::json!({"docTitle": "X", "pageCount": 5});
        let meta = extract_metadata(config, &test_parsed_url()).unwrap();
        assert!(meta.date.is_none());
    }

    // ── thumbnail tests ───────────────────────────────────────────────

    #[test]
    fn extract_thumbnail_derived_from_base_url() {
        let config = serde_json::json!({"docTitle": "X", "pageCount": 5});
        let meta = extract_metadata(config, &test_parsed_url()).unwrap();
        assert_eq!(
            meta.thumbnail,
            Some("https://online.anyflip.com/testuser/testbook/files/large/1.jpg".to_string())
        );
    }

    #[test]
    fn extract_thumbnail_present_even_when_date_missing() {
        let config = serde_json::json!({"docTitle": "X", "pageCount": 5});
        let meta = extract_metadata(config, &test_parsed_url()).unwrap();
        assert!(meta.thumbnail.is_some());
    }

    // ── password_protected tests ──────────────────────────────────────

    #[test]
    fn extract_password_protected_defaults_to_false() {
        let config = serde_json::json!({"docTitle": "X", "pageCount": 5});
        let meta = extract_metadata(config, &test_parsed_url()).unwrap();
        assert!(!meta.password_protected);
    }

    // ── basename_from_path helper tests ───────────────────────────────

    #[test]
    fn basename_strips_relative_directory_prefix() {
        assert_eq!(
            basename_from_path("../files/mobile/abc123.webp"),
            "abc123.webp"
        );
    }

    #[test]
    fn basename_strips_thumb_prefix() {
        assert_eq!(
            basename_from_path("../files/thumb/abc123.webp"),
            "abc123.webp"
        );
    }

    #[test]
    fn basename_passes_through_already_bare_filename() {
        assert_eq!(basename_from_path("page001.webp"), "page001.webp");
    }

    #[test]
    fn basename_handles_numeric_legacy_names() {
        assert_eq!(basename_from_path("../files/mobile/1.webp"), "1.webp");
    }

    // ── derive_thumbnail_url helper tests ────────────────────────────

    #[test]
    fn derive_thumbnail_url_empty_base_returns_none() {
        let parsed = url_parser::parse("https://anyflip.com/testuser/testbook").unwrap();
        let mut with_empty = parsed.clone();
        with_empty.base_url = String::new();
        assert!(derive_thumbnail_url(&with_empty).is_none());
    }

    #[test]
    fn derive_thumbnail_url_uses_base_url() {
        let parsed = url_parser::parse("https://anyflip.com/testuser/testbook").unwrap();
        let url = derive_thumbnail_url(&parsed).unwrap();
        assert_eq!(
            url,
            "https://online.anyflip.com/testuser/testbook/files/large/1.jpg"
        );
    }
}
