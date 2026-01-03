// SPDX-License-Identifier: AGPL-3.0
// Gosh Transfer - Settings persistence
//
// Settings are stored in a local JSON file.
// No cloud sync, no tracking, just simple local persistence.

use crate::types::{AppError, AppSettings};
use std::fs;
use std::path::PathBuf;
use std::sync::RwLock;

/// In-memory cache of settings, persisted to disk on changes
pub struct SettingsStore {
    settings: RwLock<AppSettings>,
    file_path: PathBuf,
}

impl SettingsStore {
    /// Create a new settings store, loading from disk if available
    pub fn new() -> Result<Self, AppError> {
        let file_path = Self::get_settings_path()?;

        let settings = if file_path.exists() {
            let content = fs::read_to_string(&file_path)
                .map_err(|e| AppError::FileIo(format!("Failed to read settings: {}", e)))?;

            serde_json::from_str(&content).unwrap_or_else(|e| {
                tracing::warn!("Failed to parse settings, using defaults: {}", e);
                AppSettings::default()
            })
        } else {
            AppSettings::default()
        };

        Ok(Self {
            settings: RwLock::new(settings),
            file_path,
        })
    }

    /// Get the path to the settings file
    fn get_settings_path() -> Result<PathBuf, AppError> {
        let config_dir = directories::ProjectDirs::from("com", "gosh", "transfer")
            .ok_or_else(|| AppError::FileIo("Could not determine config directory".to_string()))?
            .config_dir()
            .to_path_buf();

        // Ensure the directory exists
        fs::create_dir_all(&config_dir)
            .map_err(|e| AppError::FileIo(format!("Failed to create config dir: {}", e)))?;

        Ok(config_dir.join("settings.json"))
    }

    /// Persist settings to disk
    fn persist(&self) -> Result<(), AppError> {
        let settings = self.settings.read().unwrap();

        let content = serde_json::to_string_pretty(&*settings)
            .map_err(|e| AppError::Serialization(format!("Failed to serialize settings: {}", e)))?;

        fs::write(&self.file_path, content)
            .map_err(|e| AppError::FileIo(format!("Failed to write settings: {}", e)))?;

        Ok(())
    }

    /// Get current settings
    pub fn get(&self) -> AppSettings {
        self.settings.read().unwrap().clone()
    }

    /// Update settings and persist to disk
    pub fn update(&self, new_settings: AppSettings) -> Result<(), AppError> {
        {
            let mut settings = self.settings.write().unwrap();
            *settings = new_settings;
        }

        self.persist()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_settings() {
        let settings = AppSettings::default();
        assert_eq!(settings.port, 53317);
        assert_eq!(settings.theme, "system");
    }
}
