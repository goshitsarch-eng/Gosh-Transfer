# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

- **Directory transfer support**: Send entire folders with preserved structure via new "Select Folder" button
- **Transfer cancellation**: Cancel in-progress transfers from both Send and Receive views
- **Transfer speed display**: Real-time speed (KB/s, MB/s) shown during active transfers
- **Persistent transfer history**: History now persists across app restarts (stored in `history.json`, max 100 entries)
- **Batch accept/reject**: "Accept All" and "Reject All" buttons when multiple transfers are pending
- **System notifications**: Native OS notifications for incoming transfers and completions (respects settings)
- **Transfer retry events**: UI can now display retry attempts when transfers encounter transient failures
- **Port change events**: Server now emits events when port changes dynamically

### Changed

- **Dynamic port configuration**: Port changes now take effect immediately without requiring app restart
- Updated gosh-lan-transfer engine to v0.2.1 with new features
- Settings hint updated to reflect live port changes

### Fixed

- Transfer history no longer lost when closing the application

## [2.0.3] - 2024-12-01

### Added

- Initial Tauri 2 desktop application release
- File transfer over LAN, Tailscale, and VPNs
- Favorites management with persistence
- Settings persistence (port, device name, download directory, trusted hosts, theme)
- Receive-only mode option
- Hostname resolution with debounced lookup
- Platform-specific effects (macOS vibrancy, Windows Mica)
- Dark/light/system theme support
