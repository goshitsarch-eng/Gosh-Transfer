# Technical Specification

Gosh Transfer is a cross-platform desktop application for peer-to-peer file transfers over local networks, Tailscale, and VPNs.

## Technology Stack

### Runtime Environment

| Layer | Technology | Version |
|-------|------------|---------|
| Desktop Framework | Tauri | 2.x |
| Backend Language | Rust | 2021 Edition |
| Frontend Framework | Svelte | 5.x |
| Build Tool | Vite | 7.x |
| Package Manager | npm | 18+ |

### Core Dependencies

**Backend (Rust)**

| Crate | Purpose |
|-------|---------|
| `gosh-lan-transfer` | Transfer engine (HTTP server/client, file streaming) |
| `tauri` | Desktop application framework |
| `tauri-plugin-dialog` | Native file/folder picker |
| `tauri-plugin-notification` | System notifications |
| `tauri-plugin-os` | Platform detection |
| `tauri-plugin-shell` | URL opening |
| `tokio` | Async runtime |
| `serde` / `serde_json` | Serialization |
| `chrono` | Timestamps |
| `uuid` | Unique identifiers |
| `directories` | OS config paths |
| `tracing` | Logging |
| `window-vibrancy` | macOS/Windows visual effects |

**Frontend (JavaScript)**

| Package | Purpose |
|---------|---------|
| `@tauri-apps/api` | Tauri IPC bridge |
| `@tauri-apps/plugin-dialog` | File picker bindings |
| `@tauri-apps/plugin-notification` | Notification bindings |
| `@tauri-apps/plugin-os` | Platform detection bindings |
| `svelte` | UI framework |

## System Architecture

### Component Diagram

```
┌─────────────────────────────────────────────────────────────┐
│                     Tauri Application                        │
├─────────────────────────────────────────────────────────────┤
│  Frontend (Svelte 5)                                        │
│  ┌─────────┐ ┌─────────┐ ┌─────────┐ ┌─────────┐           │
│  │ SendView│ │ReceiveV.│ │Transfer │ │Settings │           │
│  └────┬────┘ └────┬────┘ └────┬────┘ └────┬────┘           │
│       │           │           │           │                 │
│       └───────────┴─────┬─────┴───────────┘                 │
│                         │                                   │
│                    invoke() / listen()                      │
├─────────────────────────┼───────────────────────────────────┤
│  Backend (Rust)         │                                   │
│                         ▼                                   │
│  ┌──────────────────────────────────────┐                  │
│  │           Tauri Commands              │                  │
│  │  (commands.rs - 20 IPC handlers)      │                  │
│  └──────────────────┬───────────────────┘                  │
│                     │                                       │
│         ┌───────────┼───────────┐                          │
│         ▼           ▼           ▼                          │
│  ┌───────────┐ ┌─────────┐ ┌─────────┐                     │
│  │  Engine   │ │Settings │ │Favorites│                     │
│  │(external) │ │  Store  │ │  Store  │                     │
│  └─────┬─────┘ └────┬────┘ └────┬────┘                     │
│        │            │           │                          │
│        │            └─────┬─────┘                          │
│        │                  ▼                                │
│        │           ┌───────────┐                           │
│        │           │ JSON Files│                           │
│        │           └───────────┘                           │
│        ▼                                                   │
│  ┌───────────────────────────────────────┐                 │
│  │      gosh-lan-transfer Engine          │                 │
│  │  ┌─────────────┐  ┌─────────────┐     │                 │
│  │  │ HTTP Server │  │ HTTP Client │     │                 │
│  │  │  (Axum)     │  │  (Reqwest)  │     │                 │
│  │  └──────┬──────┘  └──────┬──────┘     │                 │
│  └─────────┼────────────────┼────────────┘                 │
│            │                │                              │
└────────────┼────────────────┼──────────────────────────────┘
             │                │
             ▼                ▼
        Port 53317      Remote Peers
```

### Design Decisions

**Engine Delegation**: All transfer logic lives in the `gosh-lan-transfer` crate. This separation allows the engine to be reused in other applications (like the GTK4 Linux version) while keeping the Tauri app focused on UI concerns.

**Event-Driven Architecture**: The engine emits events through a broadcast channel. The Tauri backend subscribes to these events and forwards them to the frontend via Tauri's event system. This decouples the UI from transfer state management.

**Synchronous Persistence**: Settings, favorites, and history are persisted synchronously on every change. This trades some performance for simplicity and data safety—users won't lose data if the app crashes.

**No Database**: JSON files provide adequate storage for the expected data volumes (dozens of favorites, hundreds of history entries). A database would add complexity without meaningful benefit.

## Data Models

### AppSettings

```typescript
interface AppSettings {
  port: number;              // Default: 53317
  deviceName: string;        // Default: system hostname
  downloadDir: string;       // Default: OS downloads folder
  trustedHosts: string[];    // IPs for auto-accept
  receiveOnly: boolean;      // Hide send functionality
  notificationsEnabled: boolean;
  theme: "dark" | "light" | "system";
}
```

### Favorite

```typescript
interface Favorite {
  id: string;                // UUID v4
  name: string;              // User-defined label
  address: string;           // IP or hostname
  lastResolvedIp?: string;   // Cached resolution
  lastUsed?: string;         // ISO 8601 timestamp
}
```

### TransferRecord

```typescript
interface TransferRecord {
  id: string;
  direction: "sent" | "received";
  status: "pending" | "inProgress" | "completed" | "failed" | "rejected" | "cancelled";
  peerAddress: string;
  files: TransferFile[];
  totalSize: number;
  bytesTransferred: number;
  startedAt: string;         // ISO 8601
  completedAt?: string;
  error?: string;
}

interface TransferFile {
  id: string;
  name: string;
  size: number;
  mimeType?: string;
}
```

### PendingTransfer

```typescript
interface PendingTransfer {
  id: string;
  sourceIp: string;
  senderName?: string;
  files: TransferFile[];
  totalSize: number;
  receivedAt: string;        // ISO 8601
}
```

## Network Protocol

The transfer protocol is HTTP-based, implemented by the gosh-lan-transfer engine.

### Endpoints

| Endpoint | Method | Request | Response |
|----------|--------|---------|----------|
| `/health` | GET | — | `200 OK` |
| `/info` | GET | — | `{ deviceName, version }` |
| `/transfer` | POST | `TransferRequest` | `{ accepted, token?, pending? }` |
| `/transfer/status` | GET | `?id=` | `{ status, token? }` |
| `/chunk` | POST | File body | `200 OK` |
| `/events` | GET | — | SSE stream |

### Transfer Flow

```
Sender                              Receiver
  │                                    │
  │  POST /transfer                    │
  │  {id, senderName, files, size}     │
  │ ─────────────────────────────────► │
  │                                    │
  │     {accepted: false, pending: true}│
  │ ◄───────────────────────────────── │
  │                                    │
  │  GET /transfer/status?id=xxx       │  User reviews
  │ ─────────────────────────────────► │  in UI
  │     {status: "pending"}            │
  │ ◄───────────────────────────────── │
  │         ... polling ...            │
  │                                    │
  │  GET /transfer/status?id=xxx       │  User clicks
  │ ─────────────────────────────────► │  Accept
  │     {status: "accepted", token}    │
  │ ◄───────────────────────────────── │
  │                                    │
  │  POST /chunk?id=xxx&file=0&token=  │
  │  [file bytes]                      │
  │ ─────────────────────────────────► │
  │     200 OK                         │
  │ ◄───────────────────────────────── │
  │         ... repeat per file ...    │
  │                                    │
```

### Trusted Hosts

When a transfer request arrives from an IP in the trusted hosts list, the receiver automatically accepts and returns a token immediately, skipping the approval UI.

## File Storage

### Configuration Directory

Determined by `directories::ProjectDirs::from("com", "gosh", "transfer")`:

| Platform | Path |
|----------|------|
| macOS | `~/Library/Application Support/com.gosh.transfer/` |
| Windows | `%APPDATA%\gosh\transfer\config\` |
| Linux | `~/.config/com.gosh.transfer/` |

### File Formats

All files use pretty-printed JSON for human readability.

**settings.json**
```json
{
  "port": 53317,
  "deviceName": "MacBook Pro",
  "downloadDir": "/Users/alice/Downloads",
  "trustedHosts": ["192.168.1.50"],
  "receiveOnly": false,
  "notificationsEnabled": true,
  "theme": "system"
}
```

**favorites.json**
```json
{
  "favorites": [
    {
      "id": "550e8400-e29b-41d4-a716-446655440000",
      "name": "Living Room PC",
      "address": "192.168.1.100",
      "lastResolvedIp": "192.168.1.100",
      "lastUsed": "2024-01-15T10:30:00Z"
    }
  ]
}
```

**history.json**
```json
{
  "records": [
    {
      "id": "transfer-uuid",
      "direction": "received",
      "status": "completed",
      "peerAddress": "192.168.1.100",
      "files": [{"id": "f1", "name": "photo.jpg", "size": 1048576}],
      "totalSize": 1048576,
      "bytesTransferred": 1048576,
      "startedAt": "2024-01-15T10:30:00Z",
      "completedAt": "2024-01-15T10:30:05Z"
    }
  ]
}
```

### History Limits

Transfer history is capped at 100 entries. When a new record is added and the limit is exceeded, the oldest entry is removed (FIFO).

## Security Considerations

### Network Security

The application does not implement encryption at the transport layer. All transfers occur over plain HTTP. For secure transfers:

1. Use a VPN or Tailscale (traffic encrypted at network layer)
2. Transfer only on trusted local networks
3. Do not expose the port to the internet

### File System Security

- Received files are written to the user-configured download directory only
- File paths from senders are sanitized; only the filename is used
- Name conflicts are resolved by appending `(1)`, `(2)`, etc.
- The application never executes received files

### Trust Model

- Trusted hosts auto-accept transfers without user confirmation
- Trusted hosts are stored as exact IP addresses
- Hostname resolution happens before matching (but resolved IPs are not added to the trust list automatically)

### Content Security Policy

The Tauri webview enforces a strict CSP:
```
default-src 'self'; style-src 'self' 'unsafe-inline'; script-src 'self'
```

## Performance Characteristics

### Resource Usage

| Metric | Typical Value |
|--------|---------------|
| Memory (idle) | ~50-80 MB |
| Memory (transferring) | ~100-150 MB |
| CPU (idle) | <1% |
| Disk (config) | <100 KB |

### Transfer Performance

Transfer speed is limited by network bandwidth and disk I/O. The engine streams files directly to disk without buffering entire files in memory.

### Startup Time

Cold start typically completes in under 2 seconds, including:
- Tauri initialization
- Settings/favorites/history loading
- HTTP server binding
- Window creation and rendering

## Platform Support

### Build Targets

| Platform | Architecture | Bundle Format |
|----------|--------------|---------------|
| macOS | x86_64, aarch64 | .dmg, .app |
| Windows | x86_64, aarch64 | .exe, .msi (NSIS) |
| Linux | x86_64 | .deb, .AppImage |

### Minimum OS Versions

| Platform | Minimum Version |
|----------|-----------------|
| macOS | 10.15 (Catalina) |
| Windows | 10 |
| Linux | Kernel 4.x, glibc 2.31+ |

### Platform-Specific Features

| Feature | macOS | Windows | Linux |
|---------|-------|---------|-------|
| Window vibrancy | Sidebar effect | Mica backdrop | — |
| System notifications | Native | Native | Native (via D-Bus) |
| Title bar | Overlay style | Overlay style | Standard |

## Build and Deployment

### Development

```bash
npm install              # Install JS dependencies
npm run tauri dev        # Start dev server with hot reload
```

### Production Build

```bash
npm run tauri build      # Build for current platform
```

### Cross-Platform Builds

macOS builds require Xcode and optionally signing certificates. Windows builds require Visual Studio Build Tools. Linux builds require standard development packages (webkit2gtk, etc.).

### Environment Variables

| Variable | Purpose |
|----------|---------|
| `RUST_LOG` | Control logging verbosity |

## Monitoring and Debugging

### Logging

The application uses `tracing` for structured logging. Default log levels:
- `gosh_transfer`: info
- `gosh_lan_transfer`: info
- `tower_http`: info

Logs are written to stderr. In development, they appear in the terminal. In production, they're captured by the OS (Console.app on macOS, Event Viewer on Windows).

### Debugging Transfers

1. Check server status in sidebar (green dot = running)
2. Verify port is not blocked by firewall
3. Test connectivity with `/health` endpoint
4. Check logs for error messages

### Common Issues

| Symptom | Likely Cause |
|---------|--------------|
| Server won't start | Port already in use |
| Peer not reachable | Firewall blocking port 53317 |
| Transfer stuck at 0% | Approval pending on receiver |
| Files not appearing | Check download directory setting |
