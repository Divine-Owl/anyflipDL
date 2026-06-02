use crate::error::AppError;
use crate::models::Settings;

#[tauri::command]
pub async fn get_settings() -> Result<Settings, AppError> {
    Ok(crate::data::config::Config::load())
}

#[tauri::command]
pub async fn save_settings(settings: Settings) -> Result<(), AppError> {
    if settings.save_location.is_empty() {
        return Err(AppError::InvalidUrl(
            "Save location cannot be empty".to_string(),
        ));
    }

    match settings.default_format.as_str() {
        "pdf" | "epub" => {}
        other => {
            return Err(AppError::SettingsError(format!(
                "Invalid format '{}': must be 'pdf' or 'epub'",
                other
            )));
        }
    }

    match settings.theme.as_str() {
        "light" | "dark" => {}
        other => {
            return Err(AppError::SettingsError(format!(
                "Invalid theme '{}': must be 'light' or 'dark'",
                other
            )));
        }
    }

    settings.save()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_settings(format: &str, theme: &str, save_location: &str) -> Settings {
        Settings {
            version: 2,
            save_location: save_location.to_string(),
            default_format: format.to_string(),
            theme: theme.to_string(),
            compression: false,
            sound_notification: true,
            file_naming_pattern: "{title}".to_string(),
            seen_onboarding: false,
        }
    }

    #[tokio::test]
    async fn save_settings_valid_format_pdf() {
        assert!(save_settings(make_settings("pdf", "light", "/tmp")).await.is_ok());
    }

    #[tokio::test]
    async fn save_settings_valid_format_epub() {
        assert!(save_settings(make_settings("epub", "light", "/tmp")).await.is_ok());
    }

    #[tokio::test]
    async fn save_settings_invalid_format() {
        let result = save_settings(make_settings("docx", "light", "/tmp")).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Invalid format"));
    }

    #[tokio::test]
    async fn save_settings_valid_theme_light() {
        assert!(save_settings(make_settings("pdf", "light", "/tmp")).await.is_ok());
    }

    #[tokio::test]
    async fn save_settings_valid_theme_dark() {
        assert!(save_settings(make_settings("pdf", "dark", "/tmp")).await.is_ok());
    }

    #[tokio::test]
    async fn save_settings_invalid_theme() {
        let result = save_settings(make_settings("pdf", "rainbow", "/tmp")).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Invalid theme"));
    }

    #[tokio::test]
    async fn save_settings_empty_location() {
        let result = save_settings(make_settings("pdf", "light", "")).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Save location cannot be empty"));
    }
}
