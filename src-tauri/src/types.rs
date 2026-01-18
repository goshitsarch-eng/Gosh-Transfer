// SPDX-License-Identifier: AGPL-3.0
// Gosh Transfer - Shared types for the application
//
// These types are used for Tauri IPC serialization.
// The gosh-lan-transfer engine provides the core transfer logic.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use uuid::Uuid;

/// A saved peer/favorite for quick access
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Favorite {
    /// Unique identifier
    pub id: String,
    /// User-friendly name (e.g., "Living Room PC")
    pub name: String,
    /// Hostname or IP address
    pub address: String,
    /// Last successfully resolved IP (if available)
    pub last_resolved_ip: Option<String>,
    /// When this favorite was last used
    pub last_used: Option<DateTime<Utc>>,
}

impl Favorite {
    pub fn new(name: String, address: String) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            name,
            address,
            last_resolved_ip: None,
            last_used: None,
        }
    }
}

/// Direction of a transfer
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum TransferDirection {
    Sent,
    Received,
}

/// Status of a transfer
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum TransferStatus {
    Pending,
    InProgress,
    Completed,
    Failed,
    Rejected,
    Cancelled,
}

/// A single file in a transfer
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TransferFile {
    /// File name (not full path for security)
    pub name: String,
    /// File size in bytes
    pub size: u64,
    /// MIME type (if detected)
    pub mime_type: Option<String>,
    /// Unique identifier for this file in the transfer
    pub id: String,
}

/// A completed or failed transfer record
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TransferRecord {
    /// Unique identifier
    pub id: String,
    /// Direction of transfer
    pub direction: TransferDirection,
    /// Status of the transfer
    pub status: TransferStatus,
    /// Peer address (IP or hostname)
    pub peer_address: String,
    /// Files transferred
    pub files: Vec<TransferFile>,
    /// Total size transferred
    pub total_size: u64,
    /// Bytes actually transferred (for progress/partial)
    pub bytes_transferred: u64,
    /// When the transfer started
    pub started_at: DateTime<Utc>,
    /// When the transfer completed (or failed)
    pub completed_at: Option<DateTime<Utc>>,
    /// Error message if failed
    pub error: Option<String>,
}

/// An incoming transfer pending user approval
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PendingTransfer {
    /// Transfer ID
    pub id: String,
    /// Source IP address
    pub source_ip: String,
    /// Optional sender name
    pub sender_name: Option<String>,
    /// Files to be received
    pub files: Vec<TransferFile>,
    /// Total size
    pub total_size: u64,
    /// When the request was received
    pub received_at: DateTime<Utc>,
}

/// Network interface information
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NetworkInterface {
    /// Interface name
    pub name: String,
    /// IP address
    pub ip: String,
    /// Whether this is a loopback interface
    pub is_loopback: bool,
}

/// DNS resolution result
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ResolveResult {
    /// Original hostname/address
    pub hostname: String,
    /// Resolved IP addresses
    pub ips: Vec<String>,
    /// Whether resolution was successful
    pub success: bool,
    /// Error message if failed
    pub error: Option<String>,
}

/// Application settings
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AppSettings {
    /// Port for the HTTP server (default: 53317)
    pub port: u16,
    /// Device name shown to peers
    pub device_name: String,
    /// Default download directory
    pub download_dir: PathBuf,
    /// Auto-accept from trusted hosts
    pub trusted_hosts: Vec<String>,
    /// Receive-only mode (disable sending)
    pub receive_only: bool,
    /// Show system notifications
    pub notifications_enabled: bool,
    /// Theme preference: "dark", "light", or "system"
    #[serde(default = "default_theme")]
    pub theme: String,
}

fn default_theme() -> String {
    "system".to_string()
}

impl Default for AppSettings {
    fn default() -> Self {
        let download_dir = directories::UserDirs::new()
            .and_then(|d| d.download_dir().map(|p| p.to_path_buf()))
            .unwrap_or_else(|| PathBuf::from("."));

        Self {
            port: 53317,
            device_name: hostname::get()
                .map(|h| h.to_string_lossy().to_string())
                .unwrap_or_else(|_| "Gosh Device".to_string()),
            download_dir,
            trusted_hosts: Vec::new(),
            receive_only: false,
            notifications_enabled: true,
            theme: default_theme(),
        }
    }
}

/// Error types for the application
#[derive(Debug, thiserror::Error)]
pub enum AppError {
    #[error("Network error: {0}")]
    Network(String),

    #[error("DNS resolution failed: {0}")]
    DnsResolution(String),

    #[error("Connection refused: {0}")]
    ConnectionRefused(String),

    #[error("Transfer rejected by peer")]
    TransferRejected,

    #[error("File I/O error: {0}")]
    FileIo(String),

    #[error("Serialization error: {0}")]
    Serialization(String),

    #[error("Server not running")]
    ServerNotRunning,

    #[error("Invalid configuration: {0}")]
    InvalidConfig(String),
}

// Allow AppError to be returned from Tauri commands
impl serde::Serialize for AppError {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}
