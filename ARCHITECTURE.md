# Architecture

Gosh Transfer is a Tauri 2 desktop application with a Rust backend and Svelte 5 frontend. All file transfer logic is delegated to the [gosh-lan-transfer](https://github.com/goshitsarch-eng/gosh-lan-transfer) engine library.

## Module Structure

### Backend (`src-tauri/src/`)

```
src/
├── main.rs         # Entry point, calls lib::run()
├── lib.rs          # App initialization, plugin setup, event forwarding
├── commands.rs     # Tauri IPC command handlers
├── types.rs        # Shared data structures for serialization
├── settings.rs     # Settings persistence (settings.json)
├── favorites.rs    # Favorites persistence (favorites.json)
└── history.rs      # Transfer history persistence (history.json)
```

### Frontend (`src/`)

```
src/
├── main.js                     # Svelte app mount
├── App.svelte                  # Main layout, navigation, event listeners
├── lib/
│   ├── theme.js                # Theme switching, platform detection
│   └── components/
│       ├── SendView.svelte     # File/folder sending UI
│       ├── ReceiveView.svelte  # Incoming transfer approval
│       ├── TransfersView.svelte # Transfer history display
│       ├── SettingsView.svelte  # Settings form
│       └── AboutView.svelte     # About page
└── styles/
    └── global.css              # Global styles
```

## State Management

### AppState (`commands.rs:14-21`)

Central state managed by Tauri:

```rust
pub struct AppState {
    pub favorites: FavoritesStore,
    pub engine: Arc<Mutex<GoshTransferEngine>>,
    pub event_rx: Arc<Mutex<Option<broadcast::Receiver<EngineEvent>>>>,
    pub settings_store: SettingsStore,
    pub settings: RwLock<AppSettings>,
    pub history_store: HistoryStore,
}
```

The `GoshTransferEngine` from the engine crate handles all HTTP server/client operations, transfer state, and networking.

## Event Flow

### Engine to Frontend

```
GoshTransferEngine
    → EngineEvent (broadcast channel)
    → lib.rs event handler
    → Tauri emit()
    → Frontend listen()
```

Events forwarded:

| Event | Payload |
|-------|---------|
| `transfer-request` | transfer object |
| `transfer-progress` | transferId, bytesTransferred, totalBytes, currentFile, speedBps |
| `transfer-complete` | transferId |
| `transfer-failed` | transferId, error |
| `transfer-retry` | transferId, attempt, maxAttempts, error |
| `server-started` | port |
| `server-stopped` | (none) |
| `port-changed` | oldPort, newPort |

### Frontend to Backend

```
Frontend invoke()
    → Tauri command handler
    → AppState / Engine access
    → Result returned
```

## Tauri Commands

### Favorites
| Command | Returns |
|---------|---------|
| `list_favorites()` | `Vec<Favorite>` |
| `add_favorite(name, address)` | `Favorite` |
| `update_favorite(id, name?, address?)` | `Favorite` |
| `delete_favorite(id)` | `()` |

### Network
| Command | Returns |
|---------|---------|
| `resolve_hostname(address)` | `ResolveResult` |
| `get_interfaces()` | `Vec<NetworkInterface>` |
| `check_peer(address, port)` | `bool` |
| `get_peer_info(address, port)` | `JSON` |

### Transfers
| Command | Returns |
|---------|---------|
| `send_files(address, port, file_paths)` | `()` |
| `send_directory(address, port, directory_path)` | `()` |
| `accept_transfer(transfer_id)` | `String` (token) |
| `reject_transfer(transfer_id)` | `()` |
| `cancel_transfer(transfer_id)` | `()` |
| `accept_all_transfers()` | `Vec<String>` (accepted IDs) |
| `reject_all_transfers()` | `()` |
| `get_pending_transfers()` | `Vec<PendingTransfer>` |
| `get_transfer_history()` | `Vec<TransferRecord>` |
| `clear_transfer_history()` | `()` |

### Settings
| Command | Returns |
|---------|---------|
| `get_settings()` | `AppSettings` |
| `update_settings(new_settings)` | `()` |
| `add_trusted_host(host)` | `()` |
| `remove_trusted_host(host)` | `()` |

### Server
| Command | Returns |
|---------|---------|
| `get_server_status()` | `JSON` |

## Configuration

Settings, favorites, and history are stored in the OS config directory. Path determined by `directories::ProjectDirs::from("com", "gosh", "transfer")`:

| Platform | Path |
|----------|------|
| macOS | `~/Library/Application Support/com.gosh.transfer/` |
| Windows | `%APPDATA%\gosh\transfer\config\` |
| Linux | `~/.config/com.gosh.transfer/` |

Files:
| File | Purpose |
|------|---------|
| `settings.json` | Application settings |
| `favorites.json` | Saved peer addresses |
| `history.json` | Transfer history (max 100 entries, FIFO) |

## Tauri Plugins

| Plugin | Purpose |
|--------|---------|
| `tauri-plugin-shell` | Open URLs in browser |
| `tauri-plugin-dialog` | File/folder picker dialogs |
| `tauri-plugin-os` | Platform detection |
| `tauri-plugin-notification` | System notifications |

## Platform Effects

Window effects are applied conditionally at startup (`lib.rs:110-125`):

| Platform | Effect |
|----------|--------|
| macOS | NSVisualEffectMaterial::Sidebar vibrancy |
| Windows | Mica backdrop |
| Linux | None (standard window) |

## Transfer Protocol

The HTTP protocol is implemented by the gosh-lan-transfer engine. Default port is 53317.

| Endpoint | Method | Purpose |
|----------|--------|---------|
| `/health` | GET | Connectivity check |
| `/info` | GET | Device name and version |
| `/transfer` | POST | Initiate transfer request |
| `/transfer/status` | GET | Poll approval status |
| `/chunk` | POST | Stream file data |
| `/events` | GET | SSE for real-time progress |

## Known Limitations

1. Server binds to IPv4 only (`0.0.0.0`)
2. Trusted hosts require exact IP match (hostnames not resolved)
