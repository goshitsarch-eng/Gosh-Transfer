// SPDX-License-Identifier: AGPL-3.0
// Gosh Transfer - Library exports
//
// NOTICE: This project is NOT affiliated with Motrix or any other download manager.
// This is an independent, open-source project.

pub mod client;
pub mod commands;
pub mod favorites;
pub mod server;
pub mod types;

use commands::AppState;
use favorites::FavoritesStore;
use server::ServerState;
use std::sync::Arc;
use types::AppSettings;

/// Initialize the application state
pub fn init_app_state() -> Result<AppState, types::AppError> {
    let settings = AppSettings::default();
    let favorites = FavoritesStore::new()?;
    let client = client::TransferClient::new();
    let server_state = Arc::new(ServerState::new(settings.clone()));

    Ok(AppState {
        favorites,
        client,
        server_state,
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

    // Get the server state for the background task
    let server_state = app_state.server_state.clone();
    let port = 53317; // Default port

    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_dialog::init())
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
            // Start the HTTP server in the background
            let state = server_state.clone();
            tauri::async_runtime::spawn(async move {
                if let Err(e) = server::start_server(state, port).await {
                    tracing::error!("Server error: {}", e);
                }
            });

            // Set up event forwarding from server to frontend
            let app_handle = app.handle().clone();
            let state = app
                .state::<AppState>()
                .server_state
                .clone();
            let mut rx = state.event_tx.subscribe();

            tauri::async_runtime::spawn(async move {
                while let Ok(event) = rx.recv().await {
                    let event_name = match &event {
                        server::ServerEvent::TransferRequest { .. } => "transfer-request",
                        server::ServerEvent::Progress { .. } => "transfer-progress",
                        server::ServerEvent::TransferComplete { .. } => "transfer-complete",
                        server::ServerEvent::TransferFailed { .. } => "transfer-failed",
                    };

                    if let Err(e) = app_handle.emit(event_name, &event) {
                        tracing::warn!("Failed to emit event: {}", e);
                    }
                }
            });

            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
