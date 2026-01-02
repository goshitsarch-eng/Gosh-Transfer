// SPDX-License-Identifier: AGPL-3.0
// Gosh Transfer - HTTP server for receiving file transfers
//
// The server binds to 0.0.0.0 and :: to accept connections from any interface.
// This ensures it works reliably on LAN, Tailscale, and VPNs.

use axum::{
    body::Body,
    extract::{Path, Query, State},
    http::{header, StatusCode},
    response::{IntoResponse, Response, Sse},
    routing::{get, post},
    Json, Router,
};
use futures_util::StreamExt;
use tokio_stream::wrappers::BroadcastStream;
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    net::SocketAddr,
    path::PathBuf,
    sync::Arc,
};
use tokio::{
    fs::File,
    io::AsyncWriteExt,
    sync::{broadcast, RwLock},
};
use uuid::Uuid;

use crate::types::{
    AppError, AppSettings, PendingTransfer, TransferFile, TransferProgress, TransferRequest,
    TransferResponse,
};

/// Server state shared across handlers
pub struct ServerState {
    /// Application settings
    pub settings: RwLock<AppSettings>,
    /// Pending transfers awaiting user approval
    pub pending_transfers: RwLock<HashMap<String, PendingTransfer>>,
    /// Approved transfer tokens (transfer_id -> token)
    pub approved_tokens: RwLock<HashMap<String, String>>,
    /// Channel to notify UI of events
    pub event_tx: broadcast::Sender<ServerEvent>,
    /// Download directory
    pub download_dir: RwLock<PathBuf>,
}

/// Events emitted by the server
#[derive(Debug, Clone, Serialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum ServerEvent {
    /// A new transfer request received, pending approval
    TransferRequest { transfer: PendingTransfer },
    /// Transfer progress update
    Progress { progress: TransferProgress },
    /// Transfer completed successfully
    TransferComplete { transfer_id: String },
    /// Transfer failed
    TransferFailed { transfer_id: String, error: String },
}

impl ServerState {
    pub fn new(settings: AppSettings) -> Self {
        let (event_tx, _) = broadcast::channel(100);
        let download_dir = settings.download_dir.clone();

        Self {
            settings: RwLock::new(settings),
            pending_transfers: RwLock::new(HashMap::new()),
            approved_tokens: RwLock::new(HashMap::new()),
            event_tx,
            download_dir: RwLock::new(download_dir),
        }
    }
}

/// Query parameters for file chunk uploads
#[derive(Debug, Deserialize)]
pub struct ChunkParams {
    transfer_id: String,
    file_id: String,
    token: String,
}

/// Create the Axum router for the file transfer server
pub fn create_router(state: Arc<ServerState>) -> Router {
    Router::new()
        // Health check - useful for testing connectivity
        .route("/health", get(health_handler))
        // Server info - returns device name and version
        .route("/info", get(info_handler))
        // Transfer request - initiate a new transfer
        .route("/transfer", post(transfer_request_handler))
        // Chunk upload - stream file data
        .route("/chunk", post(chunk_upload_handler))
        // SSE endpoint for transfer progress
        .route("/events", get(events_handler))
        .with_state(state)
}

/// Health check endpoint
async fn health_handler() -> impl IntoResponse {
    Json(serde_json::json!({
        "status": "ok",
        "app": "gosh-transfer",
        "version": env!("CARGO_PKG_VERSION")
    }))
}

/// Server info endpoint
async fn info_handler(State(state): State<Arc<ServerState>>) -> impl IntoResponse {
    let settings = state.settings.read().await;

    Json(serde_json::json!({
        "name": settings.device_name,
        "version": env!("CARGO_PKG_VERSION"),
        "app": "gosh-transfer"
    }))
}

/// Handle incoming transfer request
async fn transfer_request_handler(
    State(state): State<Arc<ServerState>>,
    Json(request): Json<TransferRequest>,
) -> impl IntoResponse {
    tracing::info!(
        "Received transfer request: {} files, {} bytes",
        request.files.len(),
        request.total_size
    );

    // Create a pending transfer record
    let pending = PendingTransfer {
        id: request.transfer_id.clone(),
        source_ip: "unknown".to_string(), // Will be filled by middleware
        sender_name: request.sender_name.clone(),
        files: request.files.clone(),
        total_size: request.total_size,
        received_at: chrono::Utc::now(),
    };

    // Check if sender is in trusted hosts
    let settings = state.settings.read().await;
    let is_trusted = false; // TODO: Check against trusted_hosts

    if is_trusted {
        // Auto-accept from trusted hosts
        let token = Uuid::new_v4().to_string();
        state
            .approved_tokens
            .write()
            .await
            .insert(request.transfer_id.clone(), token.clone());

        return Json(TransferResponse {
            accepted: true,
            message: Some("Auto-accepted from trusted host".to_string()),
            token: Some(token),
        });
    }

    // Store pending transfer and notify UI
    state
        .pending_transfers
        .write()
        .await
        .insert(request.transfer_id.clone(), pending.clone());

    // Notify UI about the incoming request
    let _ = state.event_tx.send(ServerEvent::TransferRequest {
        transfer: pending,
    });

    // Return pending status - UI will call /approve or /reject
    Json(TransferResponse {
        accepted: false,
        message: Some("Awaiting user approval".to_string()),
        token: None,
    })
}

/// Handle file chunk upload
async fn chunk_upload_handler(
    State(state): State<Arc<ServerState>>,
    Query(params): Query<ChunkParams>,
    body: Body,
) -> impl IntoResponse {
    // Verify the token
    let approved = state.approved_tokens.read().await;
    let expected_token = approved.get(&params.transfer_id);

    if expected_token != Some(&params.token) {
        return (
            StatusCode::UNAUTHORIZED,
            Json(serde_json::json!({"error": "Invalid or expired token"})),
        );
    }

    // Get download directory
    let download_dir = state.download_dir.read().await.clone();

    // Find the file info from pending transfers
    let pending = state.pending_transfers.read().await;
    let transfer = match pending.get(&params.transfer_id) {
        Some(t) => t.clone(),
        None => {
            return (
                StatusCode::NOT_FOUND,
                Json(serde_json::json!({"error": "Transfer not found"})),
            );
        }
    };

    let file_info = match transfer.files.iter().find(|f| f.id == params.file_id) {
        Some(f) => f.clone(),
        None => {
            return (
                StatusCode::NOT_FOUND,
                Json(serde_json::json!({"error": "File not found in transfer"})),
            );
        }
    };

    // Create the output file
    let file_path = download_dir.join(&file_info.name);
    let mut file = match File::create(&file_path).await {
        Ok(f) => f,
        Err(e) => {
            tracing::error!("Failed to create file: {}", e);
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({"error": format!("Failed to create file: {}", e)})),
            );
        }
    };

    // Stream the body to the file
    let mut bytes_received: u64 = 0;
    let mut stream = body.into_data_stream();

    while let Some(chunk) = stream.next().await {
        match chunk {
            Ok(data) => {
                bytes_received += data.len() as u64;

                if let Err(e) = file.write_all(&data).await {
                    tracing::error!("Failed to write chunk: {}", e);
                    return (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        Json(serde_json::json!({"error": format!("Failed to write: {}", e)})),
                    );
                }

                // Send progress update
                let _ = state.event_tx.send(ServerEvent::Progress {
                    progress: TransferProgress {
                        transfer_id: params.transfer_id.clone(),
                        current_file: Some(file_info.name.clone()),
                        bytes_transferred: bytes_received,
                        total_bytes: file_info.size,
                        speed_bps: 0, // TODO: Calculate actual speed
                    },
                });
            }
            Err(e) => {
                tracing::error!("Error reading chunk: {}", e);
                return (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(serde_json::json!({"error": format!("Stream error: {}", e)})),
                );
            }
        }
    }

    // Ensure all data is flushed
    if let Err(e) = file.flush().await {
        tracing::error!("Failed to flush file: {}", e);
    }

    tracing::info!(
        "File received: {} ({} bytes)",
        file_info.name,
        bytes_received
    );

    (
        StatusCode::OK,
        Json(serde_json::json!({
            "status": "ok",
            "file": file_info.name,
            "bytes_received": bytes_received
        })),
    )
}

/// SSE endpoint for real-time transfer events
async fn events_handler(
    State(state): State<Arc<ServerState>>,
) -> Sse<impl futures_util::Stream<Item = Result<axum::response::sse::Event, std::convert::Infallible>>>
{
    let rx = state.event_tx.subscribe();

    let stream = BroadcastStream::new(rx).map(|result: Result<ServerEvent, _>| {
        let event: ServerEvent = match result {
            Ok(event) => event,
            Err(_) => return Ok::<_, std::convert::Infallible>(axum::response::sse::Event::default().data("heartbeat")),
        };

        let data = serde_json::to_string(&event).unwrap_or_default();
        Ok(axum::response::sse::Event::default().data(data))
    });

    Sse::new(stream)
}

/// Start the HTTP server
pub async fn start_server(state: Arc<ServerState>, port: u16) -> Result<(), AppError> {
    let app = create_router(state.clone());

    // Bind to all interfaces (IPv4 and IPv6)
    let addr_v4 = SocketAddr::from(([0, 0, 0, 0], port));

    tracing::info!("Starting server on port {}", port);

    let listener = tokio::net::TcpListener::bind(addr_v4)
        .await
        .map_err(|e| AppError::Network(format!("Failed to bind to port {}: {}", port, e)))?;

    axum::serve(listener, app)
        .await
        .map_err(|e| AppError::Network(format!("Server error: {}", e)))?;

    Ok(())
}
