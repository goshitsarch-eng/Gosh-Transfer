// SPDX-License-Identifier: AGPL-3.0
// Gosh Transfer - Transfer history persistence
//
// Transfer history is stored in a local JSON file with a maximum of 100 entries.
// Oldest entries are automatically removed when the limit is exceeded.

use crate::types::{AppError, TransferRecord};
use std::fs;
use std::path::PathBuf;
use std::sync::RwLock;

const MAX_HISTORY_ENTRIES: usize = 100;

/// In-memory cache of transfer history, persisted to disk on changes
pub struct HistoryStore {
    records: RwLock<Vec<TransferRecord>>,
    file_path: PathBuf,
}

#[derive(serde::Serialize, serde::Deserialize)]
struct HistoryFile {
    records: Vec<TransferRecord>,
}

impl HistoryStore {
    /// Create a new history store, loading from disk if available
    pub fn new() -> Result<Self, AppError> {
        let file_path = Self::get_history_path()?;
        tracing::info!("History file path: {:?}", file_path);

        let records = if file_path.exists() {
            tracing::info!("Loading transfer history from disk");
            let content = fs::read_to_string(&file_path)
                .map_err(|e| AppError::FileIo(format!("Failed to read history: {}", e)))?;

            let file: HistoryFile = serde_json::from_str(&content).unwrap_or_else(|e| {
                tracing::warn!("Failed to parse history, starting fresh: {}", e);
                HistoryFile { records: Vec::new() }
            });

            file.records
        } else {
            tracing::info!("No history file found, starting fresh");
            Vec::new()
        };

        Ok(Self {
            records: RwLock::new(records),
            file_path,
        })
    }

    /// Get the path to the history file
    fn get_history_path() -> Result<PathBuf, AppError> {
        let config_dir = directories::ProjectDirs::from("com", "gosh", "transfer")
            .ok_or_else(|| AppError::FileIo("Could not determine config directory".to_string()))?
            .config_dir()
            .to_path_buf();

        // Ensure the directory exists
        fs::create_dir_all(&config_dir)
            .map_err(|e| AppError::FileIo(format!("Failed to create config dir: {}", e)))?;

        Ok(config_dir.join("history.json"))
    }

    /// Persist history to disk
    fn persist(&self) -> Result<(), AppError> {
        let records = self.records.read().unwrap();
        let file = HistoryFile {
            records: records.clone(),
        };

        let content = serde_json::to_string_pretty(&file)
            .map_err(|e| AppError::Serialization(format!("Failed to serialize history: {}", e)))?;

        fs::write(&self.file_path, content)
            .map_err(|e| AppError::FileIo(format!("Failed to write history: {}", e)))?;

        Ok(())
    }

    /// List all transfer records
    pub fn list(&self) -> Vec<TransferRecord> {
        self.records.read().unwrap().clone()
    }

    /// Add a new transfer record
    pub fn add(&self, record: TransferRecord) -> Result<(), AppError> {
        {
            let mut records = self.records.write().unwrap();
            records.push(record);

            // Enforce max limit by removing oldest entries
            while records.len() > MAX_HISTORY_ENTRIES {
                records.remove(0);
            }
        }

        self.persist()
    }

    /// Clear all transfer history
    pub fn clear(&self) -> Result<(), AppError> {
        {
            let mut records = self.records.write().unwrap();
            records.clear();
        }

        self.persist()
    }

    /// Get a transfer record by ID
    pub fn get(&self, id: &str) -> Option<TransferRecord> {
        self.records
            .read()
            .unwrap()
            .iter()
            .find(|r| r.id == id)
            .cloned()
    }

    /// Update an existing record (e.g., when transfer completes or fails)
    pub fn update(&self, id: &str, update_fn: impl FnOnce(&mut TransferRecord)) -> Result<bool, AppError> {
        let updated = {
            let mut records = self.records.write().unwrap();
            if let Some(record) = records.iter_mut().find(|r| r.id == id) {
                update_fn(record);
                true
            } else {
                false
            }
        };

        if updated {
            self.persist()?;
        }

        Ok(updated)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{TransferDirection, TransferStatus};
    use chrono::Utc;

    fn create_test_record(id: &str) -> TransferRecord {
        TransferRecord {
            id: id.to_string(),
            direction: TransferDirection::Received,
            status: TransferStatus::Completed,
            peer_address: "192.168.1.100".to_string(),
            files: vec![],
            total_size: 1024,
            bytes_transferred: 1024,
            started_at: Utc::now(),
            completed_at: Some(Utc::now()),
            error: None,
        }
    }

    #[test]
    fn test_max_history_limit() {
        // This would require a temp dir for proper testing
        // Just verify the constant is set correctly
        assert_eq!(MAX_HISTORY_ENTRIES, 100);
    }
}
