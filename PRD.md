# Product Requirements Document

## Product Overview

Gosh Transfer is a desktop application for transferring files between computers on the same network. Unlike cloud-based sharing services or automatic discovery tools, Gosh Transfer requires users to explicitly specify the destination by IP address or hostname. This makes it predictable, privacy-respecting, and reliable across different network configurations including VPNs and Tailscale.

### Problem Statement

Existing file transfer solutions fall into two categories, each with tradeoffs:

1. **Cloud services** (Google Drive, Dropbox, WeTransfer): Require internet connectivity, upload files to third-party servers, impose size limits, and may have privacy implications.

2. **Auto-discovery tools** (AirDrop, LocalSend): Rely on mDNS/Bonjour which often fails across VLANs, VPNs, or corporate networks. Users have little control over what devices appear or how connections are established.

Gosh Transfer addresses both by providing direct peer-to-peer transfers where the user explicitly controls the connection.

### Product Vision

A file transfer tool that works everywhere your network reaches, without surprises. You type an address, you send files, they arrive. No accounts, no cloud, no discovery magic.

## Target Users

### Primary Users

**Home users with multiple devices**
- Want to move files between a desktop and laptop on the same WiFi
- May use Tailscale to connect devices across locations
- Value simplicity over features

**Small office/home office workers**
- Need to share files with colleagues on the local network
- May have segmented networks where auto-discovery fails
- Prefer not to use cloud services for sensitive documents

**Technical users**
- Comfortable with IP addresses and hostnames
- Want predictable, scriptable behavior
- May run the app on headless machines or servers

### Secondary Users

**Users behind corporate firewalls**
- Auto-discovery blocked by IT policies
- Need a tool that works with explicit addressing

**Privacy-conscious users**
- Want files to travel directly between devices
- Prefer no cloud intermediary

## Use Cases

### UC1: Send Files to Known Device

**Actor**: User with files to send

**Precondition**: Both devices running Gosh Transfer on reachable network

**Flow**:
1. User opens Gosh Transfer on sending device
2. User enters recipient's IP address or hostname
3. Application resolves address and shows connection status
4. User selects files via drag-and-drop or file picker
5. User clicks Send
6. Recipient sees incoming transfer notification
7. Recipient accepts or rejects
8. Files transfer with progress indication
9. Files appear in recipient's download folder

### UC2: Send Folder with Structure

**Actor**: User with folder to send

**Flow**:
1. User clicks "Select Folder" button
2. User chooses a directory
3. Application displays folder name
4. User sends to recipient
5. Recipient receives folder with internal structure preserved

### UC3: Receive from Trusted Device

**Actor**: User receiving files regularly from same sender

**Flow**:
1. User adds sender's IP to trusted hosts in settings
2. Sender initiates transfer
3. Transfer auto-accepts without prompt
4. Files arrive in download folder
5. System notification confirms completion

### UC4: Batch Transfer Management

**Actor**: User receiving multiple transfers simultaneously

**Flow**:
1. Multiple senders initiate transfers
2. User sees pending transfers list with "Accept All" / "Reject All" buttons
3. User clicks "Accept All"
4. All transfers proceed in parallel
5. Progress shown for each active transfer

### UC5: Cancel In-Progress Transfer

**Actor**: User who started wrong transfer

**Flow**:
1. User initiates transfer
2. User realizes mistake mid-transfer
3. User clicks cancel button
4. Transfer stops immediately
5. Partial files cleaned up on recipient

## Functional Requirements

### FR1: File Transfer

| ID | Requirement | Priority |
|----|-------------|----------|
| FR1.1 | Send one or more files to a specified IP/hostname | Must |
| FR1.2 | Send a directory with preserved structure | Must |
| FR1.3 | Display transfer progress (bytes, percentage) | Must |
| FR1.4 | Display transfer speed | Must |
| FR1.5 | Cancel in-progress transfers | Must |
| FR1.6 | Resume interrupted transfers | Won't (v1) |

### FR2: Receiving

| ID | Requirement | Priority |
|----|-------------|----------|
| FR2.1 | Show incoming transfer requests for approval | Must |
| FR2.2 | Accept or reject individual transfers | Must |
| FR2.3 | Accept or reject all pending transfers at once | Must |
| FR2.4 | Auto-accept from trusted hosts | Must |
| FR2.5 | Configure download directory | Must |
| FR2.6 | Handle filename conflicts (append number) | Must |

### FR3: Network

| ID | Requirement | Priority |
|----|-------------|----------|
| FR3.1 | Resolve hostnames to IP addresses | Must |
| FR3.2 | Show local network interfaces and IPs | Must |
| FR3.3 | Check if peer is reachable before sending | Should |
| FR3.4 | Configurable port number | Must |
| FR3.5 | Dynamic port changes without restart | Should |

### FR4: Favorites

| ID | Requirement | Priority |
|----|-------------|----------|
| FR4.1 | Save frequently used addresses | Must |
| FR4.2 | Edit favorite name and address | Must |
| FR4.3 | Delete favorites | Must |
| FR4.4 | Quick-select favorite to populate address | Must |

### FR5: History

| ID | Requirement | Priority |
|----|-------------|----------|
| FR5.1 | Display list of past transfers | Must |
| FR5.2 | Persist history across app restarts | Must |
| FR5.3 | Clear history | Must |
| FR5.4 | Limit history size (prevent unbounded growth) | Must |

### FR6: Settings

| ID | Requirement | Priority |
|----|-------------|----------|
| FR6.1 | Configure device name shown to peers | Must |
| FR6.2 | Configure download directory | Must |
| FR6.3 | Manage trusted hosts list | Must |
| FR6.4 | Enable/disable notifications | Must |
| FR6.5 | Select theme (dark/light/system) | Should |
| FR6.6 | Receive-only mode (hide send UI) | Should |

### FR7: Notifications

| ID | Requirement | Priority |
|----|-------------|----------|
| FR7.1 | Notify on incoming transfer request | Must |
| FR7.2 | Notify on transfer completion | Should |
| FR7.3 | Respect system notification settings | Must |

## Non-Functional Requirements

### NFR1: Performance

| ID | Requirement | Target |
|----|-------------|--------|
| NFR1.1 | Application startup time | < 3 seconds |
| NFR1.2 | Memory usage (idle) | < 100 MB |
| NFR1.3 | Transfer speed | Limited only by network/disk |
| NFR1.4 | UI responsiveness during transfer | No blocking, smooth updates |

### NFR2: Reliability

| ID | Requirement | Target |
|----|-------------|--------|
| NFR2.1 | Settings persistence | Never lose user configuration |
| NFR2.2 | Graceful handling of network errors | Clear error messages, no crashes |
| NFR2.3 | Clean shutdown | No orphaned processes or locked files |

### NFR3: Usability

| ID | Requirement | Target |
|----|-------------|--------|
| NFR3.1 | First-use without configuration | Works with defaults |
| NFR3.2 | Drag-and-drop file selection | Standard OS behavior |
| NFR3.3 | Keyboard navigation | All functions accessible |
| NFR3.4 | Clear status indication | User always knows what's happening |

### NFR4: Compatibility

| ID | Requirement | Target |
|----|-------------|--------|
| NFR4.1 | macOS support | 10.15+ (Catalina and later) |
| NFR4.2 | Windows support | Windows 10+ |
| NFR4.3 | Linux support | Modern distributions (Ubuntu 20.04+, Fedora 34+) |
| NFR4.4 | Cross-platform transfers | Any supported OS to any other |

### NFR5: Security

| ID | Requirement | Target |
|----|-------------|--------|
| NFR5.1 | No cloud dependency | All transfers peer-to-peer |
| NFR5.2 | No telemetry | Zero data collection |
| NFR5.3 | Sandboxed file access | Only read selected files, write to download dir |
| NFR5.4 | No automatic execution | Received files are never executed |

## Success Metrics

### Adoption Metrics

| Metric | Target |
|--------|--------|
| GitHub stars | Community interest indicator |
| Download count | Usage volume |
| Issue reports | Active user engagement |

### Quality Metrics

| Metric | Target |
|--------|--------|
| Crash-free sessions | > 99% |
| Transfer success rate | > 99% (when network permits) |
| User-reported bugs | Decreasing trend |

### User Satisfaction

| Metric | Method |
|--------|--------|
| Feature requests | GitHub issues tagged "enhancement" |
| User testimonials | Organic mentions, reviews |

## Constraints

### Technical Constraints

1. **No NAT traversal**: Devices must be on the same network or connected via VPN/Tailscale. The application does not implement hole-punching or relay servers.

2. **IPv4 only**: Current implementation binds to `0.0.0.0`. IPv6 support is not implemented.

3. **Single port**: All communication happens on one port (default 53317). The protocol does not support port negotiation.

4. **No encryption**: Transfers are plain HTTP. Security depends on the underlying network (use VPN for sensitive data).

### Resource Constraints

1. **Open source project**: Development is community-driven with no dedicated team.

2. **No infrastructure**: No servers, no cloud services, no ongoing operational costs.

### Compatibility Constraints

1. **Tauri framework**: UI and desktop integration limited to Tauri 2.x capabilities.

2. **Engine coupling**: Transfer behavior determined by gosh-lan-transfer crate. Changes require engine updates.

## Assumptions

1. Users have basic understanding of IP addresses or hostnames
2. Users can identify the IP of the target device (via system settings, router, or asking the recipient)
3. Network allows direct TCP connections between devices on the configured port
4. Users trust devices on their local network

## Out of Scope

The following are explicitly not planned for v1:

- Automatic device discovery (mDNS/Bonjour)
- Cloud relay for NAT traversal
- End-to-end encryption
- Mobile applications (iOS/Android)
- Browser-based transfers
- Transfer resume after interruption
- Bandwidth throttling
- Compression
- File synchronization (this is transfer, not sync)

## Future Considerations

Features that may be considered for future versions:

1. **IPv6 support**: Bind to `::` for dual-stack
2. **Transfer resume**: Save progress, resume interrupted transfers
3. **QR code sharing**: Generate QR with IP/port for easy mobile entry
4. **CLI mode**: Headless operation for servers
5. **Encryption option**: Optional TLS for sensitive transfers
6. **Transfer scheduling**: Queue transfers for later

## Revision History

| Version | Date | Changes |
|---------|------|---------|
| 1.0 | 2024-12 | Initial PRD for Gosh Transfer 2.x |
