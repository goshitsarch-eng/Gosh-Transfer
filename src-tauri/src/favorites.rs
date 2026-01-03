// SPDX-License-Identifier: AGPL-3.0
// Gosh Transfer - Local favorites storage
//
// Favorites are stored in a local JSON file.
// No cloud sync, no tracking, just simple local persistence.

use crate::types::{AppError, Favorite};
use std::fs;
use std::path::PathBuf;
use std::sync::RwLock;

/// In-memory cache of favorites, persisted to disk on changes
pub struct FavoritesStore {
    favorites: RwLock<Vec<Favorite>>,
    file_path: PathBuf,
}

#[derive(serde::Serialize, serde::Deserialize)]
struct FavoritesFile {
    favorites: Vec<Favorite>,
}

impl FavoritesStore {
    /// Create a new favorites store, loading from disk if available
    pub fn new() -> Result<Self, AppError> {
        let file_path = Self::get_favorites_path()?;

        let favorites = if file_path.exists() {
            let content = fs::read_to_string(&file_path)
                .map_err(|e| AppError::FileIo(format!("Failed to read favorites: {}", e)))?;

            let file: FavoritesFile = serde_json::from_str(&content)
                .map_err(|e| AppError::Serialization(format!("Failed to parse favorites: {}", e)))?;

            file.favorites
        } else {
            Vec::new()
        };

        Ok(Self {
            favorites: RwLock::new(favorites),
            file_path,
        })
    }

    /// Get the path to the favorites file
    fn get_favorites_path() -> Result<PathBuf, AppError> {
        let config_dir = directories::ProjectDirs::from("com", "gosh", "transfer")
            .ok_or_else(|| AppError::FileIo("Could not determine config directory".to_string()))?
            .config_dir()
            .to_path_buf();

        // Ensure the directory exists
        fs::create_dir_all(&config_dir)
            .map_err(|e| AppError::FileIo(format!("Failed to create config dir: {}", e)))?;

        Ok(config_dir.join("favorites.json"))
    }

    /// Persist favorites to disk
    fn persist(&self) -> Result<(), AppError> {
        let favorites = self.favorites.read().unwrap();
        let file = FavoritesFile {
            favorites: favorites.clone(),
        };

        let content = serde_json::to_string_pretty(&file)
            .map_err(|e| AppError::Serialization(format!("Failed to serialize favorites: {}", e)))?;

        fs::write(&self.file_path, content)
            .map_err(|e| AppError::FileIo(format!("Failed to write favorites: {}", e)))?;

        Ok(())
    }

    /// List all favorites
    pub fn list(&self) -> Vec<Favorite> {
        self.favorites.read().unwrap().clone()
    }

    /// Add a new favorite
    pub fn add(&self, name: String, address: String) -> Result<Favorite, AppError> {
        let favorite = Favorite::new(name, address);

        {
            let mut favorites = self.favorites.write().unwrap();
            favorites.push(favorite.clone());
        }

        self.persist()?;
        Ok(favorite)
    }

    /// Update an existing favorite
    pub fn update(
        &self,
        id: &str,
        name: Option<String>,
        address: Option<String>,
        last_resolved_ip: Option<String>,
    ) -> Result<Favorite, AppError> {
        let updated = {
            let mut favorites = self.favorites.write().unwrap();
            let favorite = favorites
                .iter_mut()
                .find(|f| f.id == id)
                .ok_or_else(|| AppError::InvalidConfig(format!("Favorite not found: {}", id)))?;

            if let Some(name) = name {
                favorite.name = name;
            }
            if let Some(address) = address {
                favorite.address = address;
            }
            if let Some(ip) = last_resolved_ip {
                favorite.last_resolved_ip = Some(ip);
            }
            favorite.last_used = Some(chrono::Utc::now());

            favorite.clone()
        };

        self.persist()?;
        Ok(updated)
    }

    /// Delete a favorite by ID
    pub fn delete(&self, id: &str) -> Result<(), AppError> {
        {
            let mut favorites = self.favorites.write().unwrap();
            let original_len = favorites.len();
            favorites.retain(|f| f.id != id);

            if favorites.len() == original_len {
                return Err(AppError::InvalidConfig(format!("Favorite not found: {}", id)));
            }
        }

        self.persist()?;
        Ok(())
    }

    /// Get a favorite by ID
    pub fn get(&self, id: &str) -> Option<Favorite> {
        self.favorites
            .read()
            .unwrap()
            .iter()
            .find(|f| f.id == id)
            .cloned()
    }

    /// Update the last resolved IP for a favorite (by address match)
    pub fn update_resolved_ip(&self, address: &str, ip: &str) -> Result<(), AppError> {
        {
            let mut favorites = self.favorites.write().unwrap();
            for favorite in favorites.iter_mut() {
                if favorite.address == address {
                    favorite.last_resolved_ip = Some(ip.to_string());
                }
            }
        }

        self.persist()?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;

    #[test]
    fn test_favorite_creation() {
        let fav = Favorite::new("Test".to_string(), "192.168.1.100".to_string());
        assert_eq!(fav.name, "Test");
        assert_eq!(fav.address, "192.168.1.100");
        assert!(!fav.id.is_empty());
    }
}
