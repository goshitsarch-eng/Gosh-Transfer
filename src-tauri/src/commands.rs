// SPDX-License-Identifier: AGPL-3.0
// Gosh Transfer - Tauri command handlers
//
// All UI interactions go through these commands.
// The frontend communicates ONLY via Tauri commands/events.

use crate::{
    client::{get_network_interfaces, TransferClient},
    favorites::FavoritesStore,
    server::ServerState,
    types::*,
};
use std::{path::PathBuf, sync::Arc};
use tauri::{AppHandle, Emitter, State};
use tokio::sync::RwLock;

/// Application state managed by Tauri
pub struct AppState {
    pub favorites: FavoritesStore,
    pub client: TransferClient,
    pub server_state: Arc<ServerState>,
    pub settings: RwLock<AppSettings>,
    pub transfer_history: RwLock<Vec<TransferRecord>>,
}

// ============================================================================
// FAVORITES COMMANDS
// ============================================================================

/// List all saved favorites
#[tauri::command]
pub async fn list_favorites(state: State<'_, AppState>) -> Result<Vec<Favorite>, String> {
    Ok(state.favorites.list())
}

/// Add a new favorite
#[tauri::command]
pub async fn add_favorite(
    state: State<'_, AppState>,
    name: String,
    address: String,
) -> Result<Favorite, String> {
    state
        .favorites
        .add(name, address)
        .map_err(|e| e.to_string())
}

/// Update an existing favorite
#[tauri::command]
pub async fn update_favorite(
    state: State<'_, AppState>,
    id: String,
    name: Option<String>,
    address: Option<String>,
) -> Result<Favorite, String> {
    state
        .favorites
        .update(&id, name, address, None)
        .map_err(|e| e.to_string())
}

/// Delete a favorite
#[tauri::command]
pub async fn delete_favorite(state: State<'_, AppState>, id: String) -> Result<(), String> {
    state.favorites.delete(&id).map_err(|e| e.to_string())
}

// ============================================================================
// NETWORK COMMANDS
// ============================================================================

/// Resolve a hostname to IP addresses
#[tauri::command]
pub async fn resolve_hostname(address: String) -> Result<ResolveResult, String> {
    let result = TransferClient::resolve_address(&address);
    Ok(result)
}

/// Get all network interfaces
#[tauri::command]
pub async fn get_interfaces() -> Result<Vec<NetworkInterface>, String> {
    Ok(get_network_interfaces())
}

/// Check if a peer is reachable
#[tauri::command]
pub async fn check_peer(
    state: State<'_, AppState>,
    address: String,
    port: u16,
) -> Result<bool, String> {
    state
        .client
        .check_peer(&address, port)
        .await
        .map_err(|e| e.to_string())
}

/// Get peer information
#[tauri::command]
pub async fn get_peer_info(
    state: State<'_, AppState>,
    address: String,
    port: u16,
) -> Result<serde_json::Value, String> {
    state
        .client
        .get_peer_info(&address, port)
        .await
        .map_err(|e| e.to_string())
}

// ============================================================================
// TRANSFER COMMANDS
// ============================================================================

/// Send files to a peer
#[tauri::command]
pub async fn send_files(
    state: State<'_, AppState>,
    address: String,
    port: u16,
    file_paths: Vec<String>,
) -> Result<(), String> {
    let paths: Vec<PathBuf> = file_paths.into_iter().map(PathBuf::from).collect();

    let settings = state.settings.read().await;
    if settings.receive_only {
        return Err("Sending is disabled in receive-only mode".to_string());
    }
    let sender_name = Some(settings.device_name.clone());
    drop(settings);

    state
        .client
        .send_files(&address, port, paths, sender_name)
        .await
        .map_err(|e| e.to_string())
}

/// Accept a pending transfer
#[tauri::command]
pub async fn accept_transfer(
    state: State<'_, AppState>,
    transfer_id: String,
) -> Result<String, String> {
    // Generate a token for this transfer
    let token = uuid::Uuid::new_v4().to_string();

    // Add to approved tokens
    state
        .server_state
        .approved_tokens
        .write()
        .await
        .insert(transfer_id.clone(), token.clone());

    state
        .server_state
        .rejected_transfers
        .write()
        .await
        .remove(&transfer_id);

    Ok(token)
}

/// Reject a pending transfer
#[tauri::command]
pub async fn reject_transfer(
    state: State<'_, AppState>,
    transfer_id: String,
) -> Result<(), String> {
    // Remove from pending transfers
    state
        .server_state
        .pending_transfers
        .write()
        .await
        .remove(&transfer_id);

    state
        .server_state
        .approved_tokens
        .write()
        .await
        .remove(&transfer_id);

    state
        .server_state
        .rejected_transfers
        .write()
        .await
        .insert(transfer_id, "Rejected by user".to_string());

    Ok(())
}

/// Get all pending transfers
#[tauri::command]
pub async fn get_pending_transfers(
    state: State<'_, AppState>,
) -> Result<Vec<PendingTransfer>, String> {
    let pending = state.server_state.pending_transfers.read().await;
    Ok(pending.values().cloned().collect())
}

/// Get transfer history
#[tauri::command]
pub async fn get_transfer_history(
    state: State<'_, AppState>,
) -> Result<Vec<TransferRecord>, String> {
    let history = state.transfer_history.read().await;
    Ok(history.clone())
}

/// Clear transfer history
#[tauri::command]
pub async fn clear_transfer_history(state: State<'_, AppState>) -> Result<(), String> {
    let mut history = state.transfer_history.write().await;
    history.clear();
    Ok(())
}

// ============================================================================
// SETTINGS COMMANDS
// ============================================================================

/// Get current settings
#[tauri::command]
pub async fn get_settings(state: State<'_, AppState>) -> Result<AppSettings, String> {
    let settings = state.settings.read().await;
    Ok(settings.clone())
}

/// Update settings
#[tauri::command]
pub async fn update_settings(
    state: State<'_, AppState>,
    app: AppHandle,
    new_settings: AppSettings,
) -> Result<(), String> {
    let mut settings = state.settings.write().await;
    *settings = new_settings;
    let updated_settings = settings.clone();
    drop(settings);

    // Also update server state
    let mut server_settings = state.server_state.settings.write().await;
    *server_settings = updated_settings.clone();

    let mut download_dir = state.server_state.download_dir.write().await;
    *download_dir = updated_settings.download_dir.clone();

    let _ = app.emit("settings-updated", updated_settings);

    Ok(())
}

/// Add a trusted host
#[tauri::command]
pub async fn add_trusted_host(state: State<'_, AppState>, host: String) -> Result<(), String> {
    let mut settings = state.settings.write().await;
    if !settings.trusted_hosts.contains(&host) {
        settings.trusted_hosts.push(host.clone());
    }
    drop(settings);

    let mut server_settings = state.server_state.settings.write().await;
    if !server_settings.trusted_hosts.contains(&host) {
        server_settings.trusted_hosts.push(host);
    }
    Ok(())
}

/// Remove a trusted host
#[tauri::command]
pub async fn remove_trusted_host(state: State<'_, AppState>, host: String) -> Result<(), String> {
    let mut settings = state.settings.write().await;
    settings.trusted_hosts.retain(|h| h != &host);
    drop(settings);

    let mut server_settings = state.server_state.settings.write().await;
    server_settings.trusted_hosts.retain(|h| h != &host);
    Ok(())
}

// ============================================================================
// SERVER COMMANDS
// ============================================================================

/// Get server status
#[tauri::command]
pub async fn get_server_status(state: State<'_, AppState>) -> Result<serde_json::Value, String> {
    let settings = state.settings.read().await;
    let interfaces = get_network_interfaces();

    Ok(serde_json::json!({
        "running": true,
        "port": settings.port,
        "interfaces": interfaces,
        "device_name": settings.device_name
    }))
}
