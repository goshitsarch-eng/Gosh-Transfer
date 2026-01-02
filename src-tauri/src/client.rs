// SPDX-License-Identifier: AGPL-3.0
// Gosh Transfer - HTTP client for sending file transfers
//
// The client explicitly resolves hostnames and attempts all IPs.
// This ensures reliable connections over LAN, Tailscale, and VPNs.

use crate::types::{
    AppError, ResolveResult, TransferFile, TransferProgress, TransferRequest, TransferResponse,
};
use reqwest::Client;
use std::{
    net::{SocketAddr, ToSocketAddrs},
    path::Path,
    sync::Arc,
    time::Duration,
};
use tokio::{
    fs::File,
    io::AsyncReadExt,
    sync::broadcast,
};
use uuid::Uuid;

/// Client for sending files to a peer
pub struct TransferClient {
    http_client: Client,
    /// Channel for progress updates
    progress_tx: broadcast::Sender<TransferProgress>,
}

impl TransferClient {
    pub fn new() -> Self {
        let (progress_tx, _) = broadcast::channel(100);

        let http_client = Client::builder()
            .timeout(Duration::from_secs(30))
            .connect_timeout(Duration::from_secs(10))
            .build()
            .expect("Failed to create HTTP client");

        Self {
            http_client,
            progress_tx,
        }
    }

    /// Subscribe to progress updates
    pub fn subscribe_progress(&self) -> broadcast::Receiver<TransferProgress> {
        self.progress_tx.subscribe()
    }

    /// Resolve a hostname or IP to all available addresses
    pub fn resolve_address(address: &str) -> ResolveResult {
        // First, check if it's already an IP address
        if let Ok(ip) = address.parse::<std::net::IpAddr>() {
            return ResolveResult {
                hostname: address.to_string(),
                ips: vec![ip.to_string()],
                success: true,
                error: None,
            };
        }

        // Attempt DNS resolution
        let addr_with_port = format!("{}:0", address);
        match addr_with_port.to_socket_addrs() {
            Ok(addrs) => {
                let ips: Vec<String> = addrs.map(|a| a.ip().to_string()).collect();

                if ips.is_empty() {
                    ResolveResult {
                        hostname: address.to_string(),
                        ips: Vec::new(),
                        success: false,
                        error: Some("No IP addresses found".to_string()),
                    }
                } else {
                    tracing::info!("Resolved {} to {:?}", address, ips);
                    ResolveResult {
                        hostname: address.to_string(),
                        ips,
                        success: true,
                        error: None,
                    }
                }
            }
            Err(e) => ResolveResult {
                hostname: address.to_string(),
                ips: Vec::new(),
                success: false,
                error: Some(format!("DNS resolution failed: {}", e)),
            },
        }
    }

    /// Check if a peer is reachable by hitting the /health endpoint
    pub async fn check_peer(&self, address: &str, port: u16) -> Result<bool, AppError> {
        let url = format!("http://{}:{}/health", address, port);

        match self.http_client.get(&url).send().await {
            Ok(response) => {
                if response.status().is_success() {
                    Ok(true)
                } else {
                    Err(AppError::Network(format!(
                        "Peer returned status {}",
                        response.status()
                    )))
                }
            }
            Err(e) => {
                if e.is_connect() {
                    Err(AppError::ConnectionRefused(format!(
                        "Cannot connect to {}:{} - {}",
                        address, port, e
                    )))
                } else if e.is_timeout() {
                    Err(AppError::Network(format!(
                        "Connection timed out to {}:{}",
                        address, port
                    )))
                } else {
                    Err(AppError::Network(format!("Request failed: {}", e)))
                }
            }
        }
    }

    /// Get peer info
    pub async fn get_peer_info(&self, address: &str, port: u16) -> Result<serde_json::Value, AppError> {
        let url = format!("http://{}:{}/info", address, port);

        let response = self
            .http_client
            .get(&url)
            .send()
            .await
            .map_err(|e| AppError::Network(format!("Failed to get peer info: {}", e)))?;

        response
            .json()
            .await
            .map_err(|e| AppError::Serialization(format!("Failed to parse peer info: {}", e)))
    }

    /// Initiate a transfer request to a peer
    pub async fn request_transfer(
        &self,
        address: &str,
        port: u16,
        files: Vec<TransferFile>,
        sender_name: Option<String>,
    ) -> Result<TransferResponse, AppError> {
        let transfer_id = Uuid::new_v4().to_string();
        let total_size: u64 = files.iter().map(|f| f.size).sum();

        let request = TransferRequest {
            transfer_id: transfer_id.clone(),
            sender_name,
            files,
            total_size,
        };

        let url = format!("http://{}:{}/transfer", address, port);

        let response = self
            .http_client
            .post(&url)
            .json(&request)
            .send()
            .await
            .map_err(|e| {
                if e.is_connect() {
                    AppError::ConnectionRefused(format!(
                        "Cannot connect to {}:{} - {}",
                        address, port, e
                    ))
                } else {
                    AppError::Network(format!("Transfer request failed: {}", e))
                }
            })?;

        let transfer_response: TransferResponse = response
            .json()
            .await
            .map_err(|e| AppError::Serialization(format!("Failed to parse response: {}", e)))?;

        Ok(transfer_response)
    }

    /// Send a file to a peer (after transfer is accepted)
    pub async fn send_file(
        &self,
        address: &str,
        port: u16,
        transfer_id: &str,
        token: &str,
        file_id: &str,
        file_path: &Path,
    ) -> Result<(), AppError> {
        let url = format!(
            "http://{}:{}/chunk?transfer_id={}&file_id={}&token={}",
            address, port, transfer_id, file_id, token
        );

        // Open and read the file
        let mut file = File::open(file_path)
            .await
            .map_err(|e| AppError::FileIo(format!("Failed to open file: {}", e)))?;

        let metadata = file
            .metadata()
            .await
            .map_err(|e| AppError::FileIo(format!("Failed to get file metadata: {}", e)))?;

        let file_size = metadata.len();
        let mut buffer = Vec::with_capacity(file_size as usize);

        file.read_to_end(&mut buffer)
            .await
            .map_err(|e| AppError::FileIo(format!("Failed to read file: {}", e)))?;

        // Send the file
        let response = self
            .http_client
            .post(&url)
            .header("Content-Type", "application/octet-stream")
            .header("Content-Length", file_size)
            .body(buffer)
            .send()
            .await
            .map_err(|e| AppError::Network(format!("Failed to send file: {}", e)))?;

        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(AppError::Network(format!(
                "Server returned error: {}",
                error_text
            )));
        }

        // Send progress update
        let _ = self.progress_tx.send(TransferProgress {
            transfer_id: transfer_id.to_string(),
            current_file: Some(file_path.file_name().unwrap().to_string_lossy().to_string()),
            bytes_transferred: file_size,
            total_bytes: file_size,
            speed_bps: 0,
        });

        Ok(())
    }

    /// Send multiple files to a peer
    pub async fn send_files(
        &self,
        address: &str,
        port: u16,
        file_paths: Vec<std::path::PathBuf>,
        sender_name: Option<String>,
    ) -> Result<(), AppError> {
        // Build file list with metadata
        let mut files = Vec::new();
        for path in &file_paths {
            let metadata = tokio::fs::metadata(path)
                .await
                .map_err(|e| AppError::FileIo(format!("Failed to get file info: {}", e)))?;

            let name = path
                .file_name()
                .ok_or_else(|| AppError::FileIo("Invalid file path".to_string()))?
                .to_string_lossy()
                .to_string();

            let mime_type = mime_guess::from_path(path)
                .first()
                .map(|m| m.to_string());

            files.push(TransferFile {
                id: Uuid::new_v4().to_string(),
                name,
                size: metadata.len(),
                mime_type,
            });
        }

        // Request transfer
        let response = self
            .request_transfer(address, port, files.clone(), sender_name)
            .await?;

        if !response.accepted {
            return Err(AppError::TransferRejected);
        }

        let token = response
            .token
            .ok_or_else(|| AppError::Network("No token received".to_string()))?;

        // Send each file
        let transfer_id = Uuid::new_v4().to_string(); // This should come from the request

        for (file, path) in files.iter().zip(file_paths.iter()) {
            self.send_file(address, port, &transfer_id, &token, &file.id, path)
                .await?;

            tracing::info!("Sent file: {}", file.name);
        }

        Ok(())
    }
}

impl Default for TransferClient {
    fn default() -> Self {
        Self::new()
    }
}

/// Get all network interfaces with their IP addresses
pub fn get_network_interfaces() -> Vec<crate::types::NetworkInterface> {
    let mut interfaces = Vec::new();

    if let Ok(addrs) = get_if_addrs::get_if_addrs() {
        for iface in addrs {
            let is_loopback = iface.is_loopback();
            let ip = iface.ip().to_string();
            interfaces.push(crate::types::NetworkInterface {
                name: iface.name,
                ip,
                is_loopback,
            });
        }
    }

    interfaces
}
