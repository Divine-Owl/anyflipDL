use crate::error::AppError;
use crate::models::DocumentMetadata;

#[tauri::command]
pub async fn validate_url(url: String) -> Result<bool, AppError> {
    if url.is_empty() {
        return Err(AppError::InvalidUrl("URL cannot be empty".to_string()));
    }
    match crate::services::validator::validate_url(&url) {
        Ok(()) => Ok(true),
        Err(_) => Ok(false),
    }
}

#[tauri::command]
pub async fn fetch_document_metadata(url: String) -> Result<DocumentMetadata, AppError> {
    let parsed_url = crate::services::url_parser::parse(&url)?;
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(30))
        .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/131.0.0.0 Safari/537.36")
        .build()
        .map_err(|e| AppError::NetworkError(format!("Failed to create HTTP client: {}", e)))?;
    crate::services::metadata_fetcher::fetch(&client, &parsed_url).await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn validate_url_valid() {
        let result = validate_url("https://anyflip.com/u/b".to_string()).await;
        assert_eq!(result.unwrap(), true);
    }

    #[tokio::test]
    async fn validate_url_invalid() {
        let result = validate_url("https://google.com".to_string()).await;
        assert_eq!(result.unwrap(), false);
    }

    #[tokio::test]
    async fn validate_url_empty() {
        let result = validate_url("".to_string()).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("empty"));
    }
}
