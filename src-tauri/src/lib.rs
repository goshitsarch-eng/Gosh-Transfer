// SPDX-License-Identifier: AGPL-3.0
// Gosh Transfer - Library exports

pub mod commands;
pub mod favorites;
pub mod history;
pub mod settings;
pub mod types;

use commands::AppState;
use favorites::FavoritesStore;
use gosh_lan_transfer::{EngineConfig, EngineEvent, GoshTransferEngine};
use history::HistoryStore;
use settings::SettingsStore;
use std::sync::Arc;
use tauri::{Emitter, Manager};
use tokio::sync::Mutex;

/// Initialize the application state
pub fn init_app_state() -> Result<AppState, types::AppError> {
    let settings_store = SettingsStore::new()?;
    let settings = settings_store.get();
    let favorites = FavoritesStore::new()?;
    let history_store = HistoryStore::new()?;

    // Build engine config from app settings
    let engine_config = EngineConfig::builder()
        .port(settings.port)
        .device_name(&settings.device_name)
        .download_dir(&settings.download_dir)
        .trusted_hosts(settings.trusted_hosts.clone())
        .receive_only(settings.receive_only)
        .build();

    // Create a channel for engine events
    let (engine, event_rx) = GoshTransferEngine::with_channel_events(engine_config);

    Ok(AppState {
        favorites,
        engine: Arc::new(Mutex::new(engine)),
        event_rx: Arc::new(Mutex::new(Some(event_rx))),
        settings_store,
        settings: tokio::sync::RwLock::new(settings),
        history_store,
    })
}

/// Run the Tauri application
#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env()
                .add_directive("gosh_transfer=info".parse().unwrap())
                .add_directive("gosh_lan_transfer=info".parse().unwrap())
                .add_directive("tower_http=info".parse().unwrap()),
        )
        .init();

    tracing::info!("Starting Gosh Transfer v{}", env!("CARGO_PKG_VERSION"));

    // Initialize application state
    let app_state = match init_app_state() {
        Ok(state) => state,
        Err(e) => {
            tracing::error!("Failed to initialize app state: {}", e);
            panic!("Failed to initialize: {}", e);
        }
    };

    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_os::init())
        .plugin(tauri_plugin_notification::init())
        .manage(app_state)
        .invoke_handler(tauri::generate_handler![
            // Favorites
            commands::list_favorites,
            commands::add_favorite,
            commands::update_favorite,
            commands::delete_favorite,
            // Network
            commands::resolve_hostname,
            commands::get_interfaces,
            commands::check_peer,
            commands::get_peer_info,
            // Transfers
            commands::send_files,
            commands::send_directory,
            commands::accept_transfer,
            commands::reject_transfer,
            commands::cancel_transfer,
            commands::accept_all_transfers,
            commands::reject_all_transfers,
            commands::get_pending_transfers,
            commands::get_transfer_history,
            commands::clear_transfer_history,
            // Settings
            commands::get_settings,
            commands::update_settings,
            commands::add_trusted_host,
            commands::remove_trusted_host,
            // Server
            commands::get_server_status,
        ])
        .setup(move |app| {
            // Apply platform-specific window effects
            #[cfg(any(target_os = "macos", target_os = "windows"))]
            let window = app.get_webview_window("main").unwrap();

            // macOS: Apply vibrancy effect to sidebar
            #[cfg(target_os = "macos")]
            {
                use window_vibrancy::{apply_vibrancy, NSVisualEffectMaterial};
                let _ = apply_vibrancy(&window, NSVisualEffectMaterial::Sidebar, None, None);
            }

            // Windows: Apply Mica backdrop effect
            #[cfg(target_os = "windows")]
            {
                use window_vibrancy::apply_mica;
                let _ = apply_mica(&window, None);
            }

            // Start the engine server
            let engine = app.state::<AppState>().engine.clone();
            tauri::async_runtime::spawn(async move {
                let mut engine = engine.lock().await;
                if let Err(e) = engine.start_server().await {
                    tracing::error!("Failed to start server: {}", e);
                }
            });

            // Set up event forwarding from engine to frontend
            let app_handle = app.handle().clone();
            let event_rx = app.state::<AppState>().event_rx.clone();

            tauri::async_runtime::spawn(async move {
                use tauri_plugin_notification::NotificationExt;

                let mut rx = {
                    let mut guard = event_rx.lock().await;
                    match guard.take() {
                        Some(rx) => rx,
                        None => return,
                    }
                };

                while let Ok(event) = rx.recv().await {
                    let (event_name, payload) = match &event {
                        EngineEvent::TransferRequest(transfer) => {
                            // Send notification for incoming transfer
                            let state = app_handle.state::<AppState>();
                            let settings = state.settings.read().await;
                            if settings.notifications_enabled {
                                let sender = transfer.sender_name.as_deref().unwrap_or("Unknown Device");
                                let file_count = transfer.files.len();
                                let body = if file_count == 1 {
                                    format!("{} wants to send you a file", sender)
                                } else {
                                    format!("{} wants to send you {} files", sender, file_count)
                                };
                                let _ = app_handle.notification()
                                    .builder()
                                    .title("Incoming Transfer")
                                    .body(&body)
                                    .show();
                            }

                            (
                                "transfer-request",
                                serde_json::json!({
                                    "type": "transferRequest",
                                    "transfer": transfer
                                }),
                            )
                        }
                        EngineEvent::TransferProgress(progress) => (
                            "transfer-progress",
                            serde_json::json!({
                                "type": "progress",
                                "progress": {
                                    "transferId": progress.transfer_id,
                                    "bytesTransferred": progress.bytes_transferred,
                                    "totalBytes": progress.total_bytes,
                                    "currentFile": progress.current_file,
                                    "speedBps": progress.speed_bps
                                }
                            }),
                        ),
                        EngineEvent::TransferComplete { transfer_id } => {
                            // Send notification for completed transfer
                            let state = app_handle.state::<AppState>();
                            let settings = state.settings.read().await;
                            if settings.notifications_enabled {
                                let _ = app_handle.notification()
                                    .builder()
                                    .title("Transfer Complete")
                                    .body("Files received successfully")
                                    .show();
                            }

                            (
                                "transfer-complete",
                                serde_json::json!({
                                    "type": "transferComplete",
                                    "transferId": transfer_id
                                }),
                            )
                        }
                        EngineEvent::TransferFailed { transfer_id, error } => (
                            "transfer-failed",
                            serde_json::json!({
                                "type": "transferFailed",
                                "transferId": transfer_id,
                                "error": error
                            }),
                        ),
                        EngineEvent::ServerStarted { port } => (
                            "server-started",
                            serde_json::json!({
                                "type": "serverStarted",
                                "port": port
                            }),
                        ),
                        EngineEvent::ServerStopped => (
                            "server-stopped",
                            serde_json::json!({
                                "type": "serverStopped"
                            }),
                        ),
                        EngineEvent::TransferRetry { transfer_id, attempt, max_attempts, error } => (
                            "transfer-retry",
                            serde_json::json!({
                                "type": "transferRetry",
                                "transferId": transfer_id,
                                "attempt": attempt,
                                "maxAttempts": max_attempts,
                                "error": error
                            }),
                        ),
                        EngineEvent::PortChanged { old_port, new_port } => (
                            "port-changed",
                            serde_json::json!({
                                "type": "portChanged",
                                "oldPort": old_port,
                                "newPort": new_port
                            }),
                        ),
                    };

                    if let Err(e) = app_handle.emit(event_name, payload) {
                        tracing::warn!("Failed to emit event: {}", e);
                    }
                }
            });

            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
