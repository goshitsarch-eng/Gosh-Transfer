<!--
SPDX-License-Identifier: AGPL-3.0
Gosh Transfer - Transfers View

Card-based list of completed and failed transfers.
Shows direction (sent/received), status, and allows retry/copy actions.
-->
<script>
  import { onMount } from "svelte";
  import { invoke } from "@tauri-apps/api/core";

  // Transfer history
  let transfers = $state([]);
  let isLoading = $state(true);

  // Load transfer history on mount
  onMount(async () => {
    await loadHistory();
  });

  async function loadHistory() {
    isLoading = true;
    try {
      transfers = await invoke("get_transfer_history");
    } catch (e) {
      console.error("Failed to load transfer history:", e);
      transfers = [];
    } finally {
      isLoading = false;
    }
  }

  // Clear history
  async function clearHistory() {
    try {
      await invoke("clear_transfer_history");
      transfers = [];
    } catch (e) {
      console.error("Failed to clear history:", e);
    }
  }

  // Copy peer address
  async function copyAddress(address) {
    try {
      await navigator.clipboard.writeText(address);
    } catch (e) {
      console.error("Failed to copy address:", e);
    }
  }

  // Format file size
  function formatSize(bytes) {
    if (bytes === 0) return "0 B";
    const units = ["B", "KB", "MB", "GB"];
    const i = Math.floor(Math.log(bytes) / Math.log(1024));
    return `${(bytes / Math.pow(1024, i)).toFixed(1)} ${units[i]}`;
  }

  // Format date
  function formatDate(dateStr) {
    const date = new Date(dateStr);
    const now = new Date();
    const diff = now.getTime() - date.getTime();

    // Less than 24 hours ago
    if (diff < 86400000) {
      return date.toLocaleTimeString([], { hour: "2-digit", minute: "2-digit" });
    }
    // Less than 7 days ago
    if (diff < 604800000) {
      return date.toLocaleDateString([], { weekday: "short", hour: "2-digit", minute: "2-digit" });
    }
    // Older
    return date.toLocaleDateString([], { month: "short", day: "numeric" });
  }
</script>

<div class="view-header">
  <div class="flex justify-between items-center">
    <div>
      <h2 class="view-title">Transfers</h2>
      <p class="view-subtitle">Recent file transfer history</p>
    </div>
    {#if transfers.length > 0}
      <button class="btn btn-ghost" onclick={clearHistory}>
        Clear History
      </button>
    {/if}
  </div>
</div>

{#if isLoading}
  <div class="card">
    <div class="card-body">
      <div class="empty-state">
        <p class="text-muted">Loading...</p>
      </div>
    </div>
  </div>
{:else if transfers.length === 0}
  <div class="card">
    <div class="card-body">
      <div class="empty-state">
        <svg class="empty-state-icon" viewBox="0 0 24 24" fill="none" stroke="currentColor">
          <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 8v4l3 3m6-3a9 9 0 11-18 0 9 9 0 0118 0z"/>
        </svg>
        <h3 class="empty-state-title">No transfers yet</h3>
        <p class="empty-state-text">
          Your transfer history will appear here
        </p>
      </div>
    </div>
  </div>
{:else}
  <div class="transfers-list">
    {#each transfers as transfer}
      <div class="card">
        <div class="transfer-item">
          <!-- Direction icon -->
          <div
            class="transfer-icon"
            class:sent={transfer.direction === "sent"}
            class:received={transfer.direction === "received"}
            class:failed={transfer.status === "failed"}
          >
            {#if transfer.direction === "sent"}
              <svg width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor">
                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M4 16v1a3 3 0 003 3h10a3 3 0 003-3v-1m-4-8l-4-4m0 0L8 8m4-4v12"/>
              </svg>
            {:else}
              <svg width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor">
                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M4 16v1a3 3 0 003 3h10a3 3 0 003-3v-1m-4-4l-4 4m0 0l-4-4m4 4V4"/>
              </svg>
            {/if}
          </div>

          <!-- Transfer info -->
          <div class="transfer-info">
            <div class="transfer-peer">
              {transfer.direction === "sent" ? "To" : "From"}:
              <span class="font-mono">{transfer.peerAddress}</span>
            </div>
            <div class="transfer-files">
              {transfer.files.length} file{transfer.files.length !== 1 ? "s" : ""}
              ({formatSize(transfer.totalSize)})
            </div>
            <div class="transfer-meta">
              <span class="transfer-date">
                {formatDate(transfer.startedAt)}
              </span>
              <span
                class="transfer-status"
                class:completed={transfer.status === "completed"}
                class:failed={transfer.status === "failed"}
              >
                {#if transfer.status === "completed"}
                  Completed
                {:else if transfer.status === "failed"}
                  Failed
                {:else}
                  {transfer.status}
                {/if}
              </span>
            </div>
            {#if transfer.error}
              <div class="transfer-error">
                {transfer.error}
              </div>
            {/if}
          </div>

          <!-- Actions -->
          <div class="transfer-actions-col">
            <button
              class="btn btn-ghost btn-sm"
              onclick={() => copyAddress(transfer.peerAddress)}
              title="Copy address"
            >
              <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor">
                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M8 16H6a2 2 0 01-2-2V6a2 2 0 012-2h8a2 2 0 012 2v2m-6 12h8a2 2 0 002-2v-8a2 2 0 00-2-2h-8a2 2 0 00-2 2v8a2 2 0 002 2z"/>
              </svg>
            </button>
          </div>
        </div>
      </div>
    {/each}
  </div>
{/if}

<style>
  .transfers-list {
    display: flex;
    flex-direction: column;
    gap: var(--space-2);
  }

  .transfer-actions-col {
    display: flex;
    flex-direction: column;
    gap: var(--space-2);
  }

  .transfer-error {
    margin-top: var(--space-2);
    padding: var(--space-2) var(--space-3);
    background-color: rgba(248, 81, 73, 0.1);
    border-radius: var(--radius-sm);
    font-size: var(--font-size-sm);
    color: var(--status-error);
  }

  .transfer-date {
    color: var(--text-muted);
  }
</style>
