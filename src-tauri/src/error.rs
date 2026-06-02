use serde::Serialize;

#[derive(Debug, thiserror::Error)]
pub enum AppError {
    #[error("Invalid URL: {0}")]
    InvalidUrl(String),

    #[error("Network error: {0}")]
    NetworkError(String),

    #[error("Metadata error: {0}")]
    MetadataError(String),

    #[error("Download error: {0}")]
    DownloadError(String),

    #[error("Conversion error: {0}")]
    ConversionError(String),

    #[error("File system error: {0}")]
    FileSystemError(String),

    #[error("Password required: {0}")]
    PasswordRequired(String),

    #[error("Settings error: {0}")]
    SettingsError(String),
}

// Serialize for Tauri IPC
impl Serialize for AppError {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}

// Map reqwest errors
impl From<reqwest::Error> for AppError {
    fn from(err: reqwest::Error) -> Self {
        tracing::error!("Network error: {:?}", err);
        AppError::NetworkError(err.to_string())
    }
}

// Map serde_json errors
impl From<serde_json::Error> for AppError {
    fn from(err: serde_json::Error) -> Self {
        tracing::error!("JSON error: {:?}", err);
        AppError::MetadataError(err.to_string())
    }
}

// Map std::io errors
impl From<std::io::Error> for AppError {
    fn from(err: std::io::Error) -> Self {
        tracing::error!("IO error: {:?}", err);
        AppError::FileSystemError(err.to_string())
    }
}

// Map printpdf errors
impl From<printpdf::Error> for AppError {
    fn from(err: printpdf::Error) -> Self {
        tracing::error!("PDF error: {:?}", err);
        AppError::ConversionError(err.to_string())
    }
}
