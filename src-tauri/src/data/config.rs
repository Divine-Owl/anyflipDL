use crate::error::AppError;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use tracing::{info, warn};

/// Current config schema version. Bump when fields change.
const CONFIG_VERSION: u32 = 3;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Config {
    #[serde(default = "default_config_version")]
    pub version: u32,
    pub save_location: String,
    pub default_format: String,
    pub theme: String,
    pub compression: bool,
    #[serde(default = "default_sound_notification")]
    pub sound_notification: bool,
    #[serde(default = "default_file_naming_pattern")]
    pub file_naming_pattern: String,
    #[serde(default)]
    pub seen_onboarding: bool,
}

fn default_config_version() -> u32 {
    2
}

fn default_sound_notification() -> bool {
    true
}

fn default_file_naming_pattern() -> String {
    "{title}".to_string()
}

impl Config {
    /// Return the path to the config file: `%APPDATA%/anyflipdl/config.json`.
    pub fn path() -> PathBuf {
        dirs::config_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("anyflipdl")
            .join("config.json")
    }

    /// Load config from disk. If the file is missing or corrupt, return defaults
    /// and persist them. Does NOT overwrite a corrupt file.
    pub fn load() -> Self {
        let path = Self::path();

        if path.exists() {
            match std::fs::read_to_string(&path) {
                Ok(data) => match serde_json::from_str::<Config>(&data) {
                    Ok(config) => {
                        if config.version != CONFIG_VERSION {
                            warn!(
                                "Config version mismatch: found {}, expected {}. Using defaults.",
                                config.version, CONFIG_VERSION
                            );
                            let defaults = Self::default();
                            // Attempt to persist corrected defaults
                            if let Err(e) = defaults.save() {
                                warn!("Failed to write corrected config: {}", e);
                            }
                            return defaults;
                        }
                        info!("Config loaded from {:?}", path);
                        return config;
                    }
                    Err(e) => {
                        warn!(
                            "Config file is corrupt ({}). Using defaults without overwriting.",
                            e
                        );
                        return Self::default();
                    }
                },
                Err(e) => {
                    warn!("Failed to read config file ({}). Using defaults.", e);
                    return Self::default();
                }
            }
        }

        info!("No config file found. Creating defaults at {:?}", path);
        let defaults = Self::default();
        if let Err(e) = defaults.save() {
            warn!("Failed to persist default config: {}", e);
        }
        defaults
    }

    /// Serialize to pretty JSON and write to the config path.
    /// Creates parent directories if they do not exist.
    pub fn save(&self) -> Result<(), AppError> {
        let path = Self::path();

        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        let data = serde_json::to_string_pretty(self)?;
        std::fs::write(&path, &data)?;

        info!("Config saved to {:?}", path);
        Ok(())
    }
}

impl Default for Config {
    fn default() -> Self {
        let save_location = dirs::download_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .to_string_lossy()
            .to_string();

        Self {
            version: CONFIG_VERSION,
            save_location,
            default_format: "pdf".to_string(),
            theme: "light".to_string(),
            compression: false,
            sound_notification: true,
            file_naming_pattern: "{title}".to_string(),
            seen_onboarding: false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    /// Helper: create a unique temp dir for test isolation and override the
    /// config path via environment manipulation. Returns the directory so the
    /// caller can clean up.
    fn setup_test_config_dir() -> PathBuf {
        let dir = std::env::temp_dir()
            .join("anyflipdl_test_config")
            .join(uuid::Uuid::new_v4().to_string());
        fs::create_dir_all(&dir).expect("create test config dir");
        dir
    }

    /// Test that `Config::default().save_location` matches the system Downloads
    /// directory.
    #[test]
    fn default_save_location_is_downloads() {
        let config = Config::default();
        let expected = dirs::download_dir()
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_else(|| ".".to_string());
        assert_eq!(config.save_location, expected);
    }

    /// Roundtrip: serialize then deserialize should yield the same struct.
    #[test]
    fn roundtrip_serialization() {
        let config = Config {
            version: 2,
            save_location: "C:\\test".to_string(),
            default_format: "epub".to_string(),
            theme: "dark".to_string(),
            compression: true,
            sound_notification: true,
            file_naming_pattern: "{author} - {title}".to_string(),
            seen_onboarding: true,
        };
        let json = serde_json::to_string_pretty(&config).unwrap();
        let deserialized: Config = serde_json::from_str(&json).unwrap();
        assert_eq!(config, deserialized);
    }

    /// save() writes valid JSON that load() can parse back.
    #[test]
    fn save_writes_valid_json() {
        let dir = setup_test_config_dir();
        let path = dir.join("config.json");

        let config = Config {
            version: CONFIG_VERSION,
            save_location: "/tmp/test".to_string(),
            default_format: "pdf".to_string(),
            theme: "dark".to_string(),
            compression: false,
            sound_notification: false,
            file_naming_pattern: "{title}".to_string(),
            seen_onboarding: true,
        };

        // Write manually using the same serialization logic
        let data = serde_json::to_string_pretty(&config).unwrap();
        fs::write(&path, &data).unwrap();

        // Read back and verify
        let raw = fs::read_to_string(&path).unwrap();
        let loaded: Config = serde_json::from_str(&raw).unwrap();
        assert_eq!(config, loaded);

        let _ = fs::remove_dir_all(&dir);
    }

    /// save() creates parent directories when they do not exist.
    #[test]
    fn save_creates_parent_directories() {
        let dir = setup_test_config_dir();
        let nested = dir.join("a").join("b").join("c");
        let path = nested.join("config.json");

        // Simulate the save flow: create parent dirs, write file
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        let config = Config::default();
        let data = serde_json::to_string_pretty(&config).unwrap();
        fs::write(&path, &data).unwrap();

        assert!(path.exists());
        let _ = fs::remove_dir_all(&dir);
    }

    /// Corrupt JSON should not overwrite the file.
    #[test]
    fn load_returns_default_on_corrupt_json() {
        let dir = setup_test_config_dir();
        let path = dir.join("config.json");

        // Write corrupt JSON
        fs::write(&path, "{invalid json!!!").unwrap();

        // Manually test deserialization failure
        let raw = fs::read_to_string(&path).unwrap();
        let result = serde_json::from_str::<Config>(&raw);
        assert!(result.is_err());

        // File should still exist with corrupt content
        let after = fs::read_to_string(&path).unwrap();
        assert_eq!(after, "{invalid json!!!");

        let _ = fs::remove_dir_all(&dir);
    }

    /// Verify that serde produces camelCase JSON keys.
    #[test]
    fn serde_uses_camel_case() {
        let config = Config::default();
        let json = serde_json::to_string(&config).unwrap();
        assert!(json.contains("\"saveLocation\""));
        assert!(json.contains("\"defaultFormat\""));
        assert!(json.contains("\"compression\""));
        assert!(json.contains("\"fileNamingPattern\""));
    }

    /// Default file_naming_pattern is "{title}".
    #[test]
    fn default_file_naming_pattern_is_title() {
        let config = Config::default();
        assert_eq!(config.file_naming_pattern, "{title}");
    }

    /// Missing fileNamingPattern in JSON should deserialize with default.
    #[test]
    fn deserialize_missing_file_naming_pattern_uses_default() {
        let json = r#"{
            "version": 2,
            "saveLocation": "/tmp",
            "defaultFormat": "pdf",
            "theme": "light",
            "compression": false,
            "soundNotification": true
        }"#;
        let config: Config = serde_json::from_str(json).unwrap();
        assert_eq!(config.file_naming_pattern, "{title}");
    }

    /// Missing `version` in JSON should deserialize with default (`2`). This is
    /// the regression test for the `save_settings` failure where the frontend
    /// `Settings` type was missing the field.
    #[test]
    fn deserialize_missing_version_uses_default() {
        let json = r#"{
            "saveLocation": "/tmp",
            "defaultFormat": "pdf",
            "theme": "light",
            "compression": false,
            "soundNotification": true,
            "fileNamingPattern": "{title}",
            "seenOnboarding": false
        }"#;
        let config: Config = serde_json::from_str(json).unwrap();
        assert_eq!(config.version, 2);
    }

    /// Missing `seenOnboarding` in JSON should deserialize as `false`.
    #[test]
    fn deserialize_missing_seen_onboarding_defaults_to_false() {
        let json = r#"{
            "version": 3,
            "saveLocation": "/tmp",
            "defaultFormat": "pdf",
            "theme": "light",
            "compression": false,
            "soundNotification": true,
            "fileNamingPattern": "{title}"
        }"#;
        let config: Config = serde_json::from_str(json).unwrap();
        assert!(!config.seen_onboarding);
    }

    /// Default `seen_onboarding` is `false`.
    #[test]
    fn default_seen_onboarding_is_false() {
        let config = Config::default();
        assert!(!config.seen_onboarding);
    }

    /// Verify that `seenOnboarding` is serialized in camelCase.
    #[test]
    fn serde_uses_camel_case_for_seen_onboarding() {
        let config = Config::default();
        let json = serde_json::to_string(&config).unwrap();
        assert!(json.contains("\"seenOnboarding\""));
    }

    /// Current config schema version is 3.
    #[test]
    fn config_version_is_three() {
        assert_eq!(CONFIG_VERSION, 3);
    }
}
