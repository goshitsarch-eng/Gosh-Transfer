<!--
SPDX-License-Identifier: AGPL-3.0
Gosh Transfer - Receive View

Card-based layout for receiving incoming file transfers.
Includes:
- Status card showing listening port and IPs
- Pending transfer cards with accept/reject
-->
<script>
  import { onMount } from "svelte";
  import { invoke } from "@tauri-apps/api/core";

  // Props
  let { pendingTransfers = [] } = $props();

  // Server info
  let serverInfo = $state({
    port: 53317,
    interfaces: [],
    device_name: "Loading...",
  });

  // Load server info on mount
  onMount(async () => {
    try {
      const status = await invoke("get_server_status");
      serverInfo = status;
    } catch (e) {
      console.error("Failed to get server status:", e);
    }
  });

  // Accept a transfer
  async function acceptTransfer(transferId) {
    try {
      await invoke("accept_transfer", { transferId });
    } catch (e) {
      console.error("Failed to accept transfer:", e);
    }
  }

  // Reject a transfer
  async function rejectTransfer(transferId) {
    try {
      await invoke("reject_transfer", { transferId });
    } catch (e) {
      console.error("Failed to reject transfer:", e);
    }
  }

  // Format file size
  function formatSize(bytes) {
    if (bytes === 0) return "0 B";
    const units = ["B", "KB", "MB", "GB"];
    const i = Math.floor(Math.log(bytes) / Math.log(1024));
    return `${(bytes / Math.pow(1024, i)).toFixed(1)} ${units[i]}`;
  }

  // Get non-loopback interfaces
  function getExternalInterfaces(interfaces) {
    return interfaces.filter((i) => !i.isLoopback);
  }
</script>

<div class="view-header">
  <h2 class="view-title">Receive Files</h2>
  <p class="view-subtitle">Accept incoming file transfers</p>
</div>

<!-- Server Status Card -->
<div class="card">
  <div class="card-header">
    <h3 class="card-title">Listening Status</h3>
    <p class="card-subtitle">Waiting for incoming connections</p>
  </div>
  <div class="card-body">
    <div class="status-info">
      <div class="status-row">
        <span class="status-label">Device Name</span>
        <span class="status-value">{serverInfo.device_name}</span>
      </div>
      <div class="status-row">
        <span class="status-label">Port</span>
        <span class="status-value font-mono">{serverInfo.port}</span>
      </div>
    </div>

    <!-- Network interfaces -->
    <div class="interfaces-section">
      <h4 class="interfaces-title">Your Addresses</h4>
      <p class="interfaces-hint">
        Share one of these with the sender
      </p>
      <div class="interfaces-list">
        {#each getExternalInterfaces(serverInfo.interfaces) as iface}
          <div class="interface-item">
            <span class="interface-name">{iface.name}</span>
            <code class="interface-ip">{iface.ip}</code>
          </div>
        {/each}
        {#if getExternalInterfaces(serverInfo.interfaces).length === 0}
          <p class="text-muted">No network interfaces detected</p>
        {/if}
      </div>
    </div>
  </div>
</div>

<!-- Pending Transfers -->
{#if pendingTransfers.length > 0}
  <div class="pending-section">
    <h3 class="section-title">Incoming Transfers</h3>
    {#each pendingTransfers as transfer}
      <div class="card pending-transfer-card">
        <div class="card-body">
          <div class="transfer-header">
            <div class="transfer-source">
              <svg width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor">
                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9.75 17L9 20l-1 1h8l-1-1-.75-3M3 13h18M5 17h14a2 2 0 002-2V5a2 2 0 00-2-2H5a2 2 0 00-2 2v10a2 2 0 002 2z"/>
              </svg>
              <div>
                <div class="source-name">
                  {transfer.senderName || "Unknown Device"}
                </div>
                <div class="source-ip font-mono">{transfer.sourceIp}</div>
              </div>
            </div>
            <div class="transfer-size">
              {formatSize(transfer.totalSize)}
            </div>
          </div>

          <!-- File list -->
          <div class="transfer-files">
            <h4 class="files-title">
              {transfer.files.length} file{transfer.files.length !== 1 ? "s" : ""}
            </h4>
            <ul class="file-list">
              {#each transfer.files.slice(0, 5) as file}
                <li class="file-item-small">
                  <span class="file-name">{file.name}</span>
                  <span class="file-size">{formatSize(file.size)}</span>
                </li>
              {/each}
              {#if transfer.files.length > 5}
                <li class="file-item-small text-muted">
                  ...and {transfer.files.length - 5} more
                </li>
              {/if}
            </ul>
          </div>

          <!-- Actions -->
          <div class="transfer-actions">
            <button
              class="btn btn-primary"
              onclick={() => acceptTransfer(transfer.id)}
            >
              <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor">
                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M5 13l4 4L19 7"/>
              </svg>
              Accept
            </button>
            <button
              class="btn btn-destructive"
              onclick={() => rejectTransfer(transfer.id)}
            >
              <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor">
                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M6 18L18 6M6 6l12 12"/>
              </svg>
              Reject
            </button>
          </div>
        </div>
      </div>
    {/each}
  </div>
{:else}
  <!-- Empty state -->
  <div class="card">
    <div class="card-body">
      <div class="empty-state">
        <svg class="empty-state-icon" viewBox="0 0 24 24" fill="none" stroke="currentColor">
          <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M4 16v1a3 3 0 003 3h10a3 3 0 003-3v-1m-4-4l-4 4m0 0l-4-4m4 4V4"/>
        </svg>
        <h3 class="empty-state-title">Waiting for files</h3>
        <p class="empty-state-text">
          Share your IP address with the sender to receive files
        </p>
      </div>
    </div>
  </div>
{/if}

<style>
  .status-info {
    display: flex;
    flex-direction: column;
    gap: var(--space-3);
  }

  .status-row {
    display: flex;
    justify-content: space-between;
    align-items: center;
  }

  .status-label {
    color: var(--text-secondary);
  }

  .status-value {
    color: var(--text-primary);
    font-weight: 500;
  }

  .interfaces-section {
    margin-top: var(--space-4);
    padding-top: var(--space-4);
    border-top: 1px solid var(--border-muted);
  }

  .interfaces-title {
    font-size: var(--font-size-sm);
    font-weight: 600;
    color: var(--text-primary);
    margin-bottom: var(--space-1);
  }

  .interfaces-hint {
    font-size: var(--font-size-sm);
    color: var(--text-muted);
    margin-bottom: var(--space-3);
  }

  .interfaces-list {
    display: flex;
    flex-direction: column;
    gap: var(--space-2);
  }

  .interface-item {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: var(--space-2) var(--space-3);
    background-color: var(--bg-elevated);
    border-radius: var(--radius-md);
  }

  .interface-name {
    font-size: var(--font-size-sm);
    color: var(--text-secondary);
  }

  .interface-ip {
    font-size: var(--font-size-base);
    color: var(--accent);
    background: none;
    padding: 0;
  }

  .pending-section {
    margin-top: var(--space-4);
  }

  .section-title {
    font-size: var(--font-size-base);
    font-weight: 600;
    color: var(--text-primary);
    margin-bottom: var(--space-3);
  }

  .pending-transfer-card {
    border-color: var(--primary);
  }

  .transfer-header {
    display: flex;
    justify-content: space-between;
    align-items: flex-start;
    margin-bottom: var(--space-4);
  }

  .transfer-source {
    display: flex;
    align-items: center;
    gap: var(--space-3);
  }

  .transfer-source svg {
    color: var(--accent);
  }

  .source-name {
    font-weight: 500;
    color: var(--text-primary);
  }

  .source-ip {
    font-size: var(--font-size-sm);
    color: var(--text-muted);
  }

  .transfer-size {
    font-size: var(--font-size-base);
    font-weight: 500;
    color: var(--text-secondary);
  }

  .transfer-files {
    margin-bottom: var(--space-4);
    padding: var(--space-3);
    background-color: var(--bg-elevated);
    border-radius: var(--radius-md);
  }

  .files-title {
    font-size: var(--font-size-sm);
    font-weight: 500;
    color: var(--text-secondary);
    margin-bottom: var(--space-2);
  }

  .file-list {
    list-style: none;
  }

  .file-item-small {
    display: flex;
    justify-content: space-between;
    padding: var(--space-1) 0;
    font-size: var(--font-size-sm);
  }

  .file-item-small .file-name {
    color: var(--text-primary);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
    max-width: 60%;
  }

  .file-item-small .file-size {
    color: var(--text-muted);
  }

  .transfer-actions {
    display: flex;
    gap: var(--space-3);
  }
</style>
