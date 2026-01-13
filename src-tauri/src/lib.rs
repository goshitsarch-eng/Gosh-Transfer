// SPDX-License-Identifier: AGPL-3.0
// Gosh Transfer - Library exports

pub mod commands;
pub mod favorites;
pub mod settings;
pub mod types;

use commands::AppState;
use favorites::FavoritesStore;
use gosh_lan_transfer::{EngineConfig, EngineEvent, GoshTransferEngine};
use settings::SettingsStore;
use std::sync::Arc;
use tauri::{Emitter, Manager};
use tokio::sync::Mutex;

/// Initialize the application state
pub fn init_app_state() -> Result<AppState, types::AppError> {
    let settings_store = SettingsStore::new()?;
    let settings = settings_store.get();
    let favorites = FavoritesStore::new()?;

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
        transfer_history: tokio::sync::RwLock::new(Vec::new()),
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
            commands::accept_transfer,
            commands::reject_transfer,
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
                let mut rx = {
                    let mut guard = event_rx.lock().await;
                    match guard.take() {
                        Some(rx) => rx,
                        None => return,
                    }
                };

                while let Ok(event) = rx.recv().await {
                    let (event_name, payload) = match &event {
                        EngineEvent::TransferRequest(transfer) => (
                            "transfer-request",
                            serde_json::json!({
                                "type": "transferRequest",
                                "transfer": transfer
                            }),
                        ),
                        EngineEvent::TransferProgress(progress) => (
                            "transfer-progress",
                            serde_json::json!({
                                "type": "progress",
                                "progress": progress
                            }),
                        ),
                        EngineEvent::TransferComplete { transfer_id } => (
                            "transfer-complete",
                            serde_json::json!({
                                "type": "transferComplete",
                                "transferId": transfer_id
                            }),
                        ),
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
