// SPDX-License-Identifier: AGPL-3.0
// Gosh Transfer - Tauri command handlers
//
// All UI interactions go through these commands.
// The frontend communicates ONLY via Tauri commands/events.

use crate::{favorites::FavoritesStore, settings::SettingsStore, types::*};
use gosh_lan_transfer::{EngineConfig, EngineEvent, GoshTransferEngine};
use std::{path::PathBuf, sync::Arc};
use tauri::{AppHandle, Emitter, State};
use tokio::sync::{broadcast, Mutex, RwLock};

/// Application state managed by Tauri
pub struct AppState {
    pub favorites: FavoritesStore,
    pub engine: Arc<Mutex<GoshTransferEngine>>,
    pub event_rx: Arc<Mutex<Option<broadcast::Receiver<EngineEvent>>>>,
    pub settings_store: SettingsStore,
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
    let result = GoshTransferEngine::resolve_address(&address);
    Ok(ResolveResult {
        hostname: result.hostname,
        ips: result.ips,
        success: result.success,
        error: result.error,
    })
}

/// Get all network interfaces
#[tauri::command]
pub async fn get_interfaces() -> Result<Vec<NetworkInterface>, String> {
    let interfaces = GoshTransferEngine::get_network_interfaces();
    Ok(interfaces
        .into_iter()
        .map(|i| NetworkInterface {
            name: i.name,
            ip: i.ip,
            is_loopback: i.is_loopback,
        })
        .collect())
}

/// Check if a peer is reachable
#[tauri::command]
pub async fn check_peer(
    state: State<'_, AppState>,
    address: String,
    port: u16,
) -> Result<bool, String> {
    let engine = state.engine.lock().await;
    engine
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
    let engine = state.engine.lock().await;
    engine
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

    let engine = state.engine.lock().await;
    engine
        .send_files(&address, port, paths)
        .await
        .map_err(|e| e.to_string())
}

/// Accept a pending transfer
#[tauri::command]
pub async fn accept_transfer(
    state: State<'_, AppState>,
    transfer_id: String,
) -> Result<String, String> {
    let engine = state.engine.lock().await;
    engine
        .accept_transfer(&transfer_id)
        .await
        .map_err(|e| e.to_string())
}

/// Reject a pending transfer
#[tauri::command]
pub async fn reject_transfer(
    state: State<'_, AppState>,
    transfer_id: String,
) -> Result<(), String> {
    let engine = state.engine.lock().await;
    engine
        .reject_transfer(&transfer_id)
        .await
        .map_err(|e| e.to_string())
}

/// Get all pending transfers
#[tauri::command]
pub async fn get_pending_transfers(
    state: State<'_, AppState>,
) -> Result<Vec<PendingTransfer>, String> {
    let engine = state.engine.lock().await;
    let pending = engine.get_pending_transfers().await;
    Ok(pending
        .into_iter()
        .map(|p| PendingTransfer {
            id: p.id,
            source_ip: p.source_ip,
            sender_name: p.sender_name,
            files: p
                .files
                .into_iter()
                .map(|f| TransferFile {
                    id: f.id,
                    name: f.name,
                    size: f.size,
                    mime_type: f.mime_type,
                })
                .collect(),
            total_size: p.total_size,
            received_at: p.received_at,
        })
        .collect())
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
    // Persist settings to disk
    state
        .settings_store
        .update(new_settings.clone())
        .map_err(|e| e.to_string())?;

    let mut settings = state.settings.write().await;
    *settings = new_settings.clone();
    drop(settings);

    // Update engine config
    let engine_config = EngineConfig::builder()
        .port(new_settings.port)
        .device_name(&new_settings.device_name)
        .download_dir(&new_settings.download_dir)
        .trusted_hosts(new_settings.trusted_hosts.clone())
        .receive_only(new_settings.receive_only)
        .build();

    let mut engine = state.engine.lock().await;
    engine.update_config(engine_config).await;
    drop(engine);

    let _ = app.emit("settings-updated", new_settings);

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

    let mut engine = state.engine.lock().await;
    engine.add_trusted_host(host).await;

    Ok(())
}

/// Remove a trusted host
#[tauri::command]
pub async fn remove_trusted_host(state: State<'_, AppState>, host: String) -> Result<(), String> {
    let mut settings = state.settings.write().await;
    settings.trusted_hosts.retain(|h| h != &host);
    drop(settings);

    let mut engine = state.engine.lock().await;
    engine.remove_trusted_host(&host).await;

    Ok(())
}

// ============================================================================
// SERVER COMMANDS
// ============================================================================

/// Get server status
#[tauri::command]
pub async fn get_server_status(state: State<'_, AppState>) -> Result<serde_json::Value, String> {
    let settings = state.settings.read().await;
    let interfaces = GoshTransferEngine::get_network_interfaces();
    let engine = state.engine.lock().await;

    Ok(serde_json::json!({
        "running": engine.is_server_running(),
        "port": settings.port,
        "interfaces": interfaces,
        "device_name": settings.device_name
    }))
}
