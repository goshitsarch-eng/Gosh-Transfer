<!--
SPDX-License-Identifier: AGPL-3.0
Gosh Transfer - Send View

Card-based layout for sending files to a specific IP or hostname.
Includes:
- Destination card with hostname resolution
- Favorites card for saved peers
- File selection card with drag & drop
- Send action card
-->
<script>
  import { onMount } from "svelte";
  import { invoke } from "@tauri-apps/api/core";
  import { listen } from "@tauri-apps/api/event";
  import { open } from "@tauri-apps/plugin-dialog";
  import { getCurrentWebview } from "@tauri-apps/api/webview";

  // Destination state
  let destination = $state("");
  let resolveResult = $state(null);
  let isResolving = $state(false);
  let resolveError = $state("");

  // Favorites state
  let favorites = $state([]);
  let favoritesCollapsed = $state(false);
  let showAddFavorite = $state(false);
  let newFavoriteName = $state("");
  let newFavoriteAddress = $state("");

  // File selection state
  let selectedFiles = $state([]);
  let selectedDirectory = $state(null); // { path, name, itemCount }
  let isDragging = $state(false);
  let fileAddError = $state("");
  let hasTauriDrop = $state(false);

  // Transfer state
  let isSending = $state(false);
  let sendError = $state("");
  let sendSuccess = $state(false);
  let sendProgress = $state(null); // { status, bytesTransferred, totalBytes, currentFile, speedBps }
  let currentTransferId = $state(null);

  // Default port
  const DEFAULT_PORT = 53317;

  // Load favorites on mount
  onMount(async () => {
    try {
      favorites = await invoke("list_favorites");
    } catch (e) {
      console.error("Failed to load favorites:", e);
    }

    // Listen for send progress updates
    const unlistenProgress = await listen("send-progress", (event) => {
      if (isSending) {
        const data = event.payload;
        if (data.transferId) {
          currentTransferId = data.transferId;
        }
        sendProgress = {
          ...sendProgress,
          bytesTransferred: data.bytesTransferred || sendProgress?.bytesTransferred || 0,
          totalBytes: data.totalBytes || sendProgress?.totalBytes || 0,
          currentFile: data.currentFile || sendProgress?.currentFile,
          speedBps: data.speedBps || 0,
          status: 'sending'
        };
      }
    });

    let unlistenDrop = null;
    try {
      unlistenDrop = await getCurrentWebview().onDragDropEvent((event) => {
        if (event.payload.type === "enter" || event.payload.type === "over") {
          isDragging = true;
          return;
        }

        if (event.payload.type === "leave") {
          isDragging = false;
          return;
        }

        if (event.payload.type === "drop") {
          isDragging = false;
          if (event.payload.paths?.length) {
            addFilePaths(event.payload.paths);
          }
        }
      });

      hasTauriDrop = true;
    } catch (e) {
      console.error("Failed to register drag-drop handler:", e);
    }

    return () => {
      unlistenProgress();
      if (unlistenDrop) unlistenDrop();
    };
  });

  // Resolve hostname when destination changes
  let resolveTimeout;
  $effect(() => {
    if (destination.trim()) {
      clearTimeout(resolveTimeout);
      resolveTimeout = setTimeout(() => resolveHostname(), 500);
    } else {
      resolveResult = null;
      resolveError = "";
    }
  });

  async function resolveHostname() {
    const addr = destination.trim();
    if (!addr) return;

    isResolving = true;
    resolveError = "";

    try {
      resolveResult = await invoke("resolve_hostname", { address: addr });
      if (!resolveResult.success) {
        resolveError = resolveResult.error || "Resolution failed";
      }
    } catch (e) {
      resolveError = e.toString();
      resolveResult = null;
    } finally {
      isResolving = false;
    }
  }

  // Select a favorite
  function selectFavorite(fav) {
    destination = fav.address;
  }

  // Add a new favorite
  async function addFavorite() {
    if (!newFavoriteName.trim() || !newFavoriteAddress.trim()) return;

    try {
      const fav = await invoke("add_favorite", {
        name: newFavoriteName.trim(),
        address: newFavoriteAddress.trim(),
      });
      favorites = [...favorites, fav];
      newFavoriteName = "";
      newFavoriteAddress = "";
      showAddFavorite = false;
    } catch (e) {
      console.error("Failed to add favorite:", e);
    }
  }

  // Delete a favorite
  async function deleteFavorite(id) {
    try {
      await invoke("delete_favorite", { id });
      favorites = favorites.filter((f) => f.id !== id);
    } catch (e) {
      console.error("Failed to delete favorite:", e);
    }
  }

  // Handle file drop
  function handleDrop(e) {
    e.preventDefault();
    isDragging = false;

    const files = Array.from(e.dataTransfer.files);
    addFiles(files);
  }

  function handleDragOver(e) {
    e.preventDefault();
    isDragging = true;
  }

  function handleDragLeave() {
    isDragging = false;
  }

  // Open file picker
  async function openFilePicker() {
    try {
      const selected = await open({
        multiple: true,
        directory: false,
      });

      if (selected) {
        const paths = Array.isArray(selected) ? selected : [selected];
        addFilePaths(paths);
        // Clear directory selection when files are selected
        selectedDirectory = null;
      }
    } catch (e) {
      console.error("Failed to open file picker:", e);
      fileAddError = `Failed to open file picker: ${e.toString()}`;
    }
  }

  // Open folder picker
  async function openFolderPicker() {
    try {
      const selected = await open({
        multiple: false,
        directory: true,
      });

      if (selected) {
        // Clear file selection when directory is selected
        selectedFiles = [];
        selectedDirectory = {
          path: selected,
          name: selected.split(/[/\\]/).pop(),
        };
      }
    } catch (e) {
      console.error("Failed to open folder picker:", e);
      fileAddError = `Failed to open folder picker: ${e.toString()}`;
    }
  }

  // Clear directory selection
  function clearDirectory() {
    selectedDirectory = null;
  }

  function addFilePaths(paths) {
    fileAddError = "";
    for (const path of paths) {
      if (!path || selectedFiles.some((file) => file.path === path)) {
        continue;
      }

      selectedFiles = [
        ...selectedFiles,
        {
          path,
          name: path.split(/[/\\]/).pop(),
          size: 0, // Size will be determined by backend
        },
      ];
    }
  }

  // Add files from drop
  function addFiles(files) {
    fileAddError = "";
    let missingPath = false;
    for (const file of files) {
      if (!file.path) {
        missingPath = true;
        continue;
      }

      if (selectedFiles.some((selected) => selected.path === file.path)) {
        continue;
      }

      selectedFiles = [
        ...selectedFiles,
        {
          path: file.path,
          name: file.name,
          size: file.size,
        },
      ];
    }

    if (missingPath && !hasTauriDrop) {
      fileAddError = "Drag-and-drop paths are unavailable here. Use Browse instead.";
    }
  }

  // Remove a file
  function removeFile(index) {
    selectedFiles = selectedFiles.filter((_, i) => i !== index);
  }

  // Format file size
  function formatSize(bytes) {
    if (bytes === 0) return "Unknown";
    const units = ["B", "KB", "MB", "GB"];
    const i = Math.floor(Math.log(bytes) / Math.log(1024));
    return `${(bytes / Math.pow(1024, i)).toFixed(1)} ${units[i]}`;
  }

  // Send files or directory
  async function sendFiles() {
    const hasFiles = selectedFiles.length > 0;
    const hasDirectory = selectedDirectory !== null;

    if (!resolveResult?.success || (!hasFiles && !hasDirectory)) return;

    isSending = true;
    sendError = "";
    sendSuccess = false;
    currentTransferId = null;

    // Initialize progress
    const totalBytes = hasDirectory ? 0 : selectedFiles.reduce((sum, f) => sum + (f.size || 0), 0);
    sendProgress = {
      status: 'waiting',
      bytesTransferred: 0,
      totalBytes: totalBytes,
      currentFile: null,
      speedBps: 0
    };

    try {
      const ip = resolveResult.ips[0];

      if (hasDirectory) {
        await invoke("send_directory", {
          address: ip,
          port: DEFAULT_PORT,
          directoryPath: selectedDirectory.path,
        });
        selectedDirectory = null;
      } else {
        const filePaths = selectedFiles.map((f) => f.path);
        await invoke("send_files", {
          address: ip,
          port: DEFAULT_PORT,
          filePaths: filePaths,
        });
        selectedFiles = [];
      }

      sendSuccess = true;
    } catch (e) {
      sendError = e.toString();
    } finally {
      isSending = false;
      sendProgress = null;
      currentTransferId = null;
    }
  }

  // Cancel ongoing transfer
  async function cancelTransfer() {
    if (!currentTransferId) return;

    try {
      await invoke("cancel_transfer", { transferId: currentTransferId });
      sendError = "Transfer cancelled";
    } catch (e) {
      console.error("Failed to cancel transfer:", e);
    }
  }

  // Check if send is enabled
  function canSend() {
    const hasContent = selectedFiles.length > 0 || selectedDirectory !== null;
    return resolveResult?.success && hasContent && !isSending;
  }

  // Format speed
  function formatSpeed(bytesPerSec) {
    if (!bytesPerSec || bytesPerSec === 0) return "";
    const units = ["B/s", "KB/s", "MB/s", "GB/s"];
    const i = Math.floor(Math.log(bytesPerSec) / Math.log(1024));
    return `${(bytesPerSec / Math.pow(1024, i)).toFixed(1)} ${units[i]}`;
  }
</script>

<div class="view-header">
  <h2 class="view-title">Send Files</h2>
  <p class="view-subtitle">
    Transfer files to a specific IP address or hostname
  </p>
</div>

<!-- Destination Card -->
<div class="card">
  <div class="card-header">
    <h3 class="card-title">Destination</h3>
    <p class="card-subtitle">Enter an IP address or hostname</p>
  </div>
  <div class="card-body">
    <div class="form-group">
      <input
        type="text"
        class="form-input"
        class:error={resolveError}
        class:success={resolveResult?.success}
        placeholder="192.168.1.100 or hostname.local"
        bind:value={destination}
      />
      {#if isResolving}
        <p class="form-hint">Resolving...</p>
      {:else if resolveError}
        <p class="form-error">{resolveError}</p>
      {:else if resolveResult?.success}
        <p class="form-success">
          Resolved to: {resolveResult.ips.join(", ")}
        </p>
      {/if}
    </div>
  </div>
</div>

<!-- Favorites Card -->
<div class="card collapsible" class:collapsed={favoritesCollapsed}>
  <div
    class="card-header"
    onclick={() => (favoritesCollapsed = !favoritesCollapsed)}
  >
    <div>
      <h3 class="card-title">Favorites</h3>
      <p class="card-subtitle">Quick access to saved peers</p>
    </div>
    <svg class="collapse-icon" width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor">
      <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M19 9l-7 7-7-7"/>
    </svg>
  </div>
  <div class="card-body">
    {#if favorites.length === 0}
      <div class="empty-state">
        <p class="text-muted">No favorites saved yet</p>
      </div>
    {:else}
      {#each favorites as fav}
        <div class="favorite-item" onclick={() => selectFavorite(fav)}>
          <div class="favorite-icon">
            <svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor">
              <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9.75 17L9 20l-1 1h8l-1-1-.75-3M3 13h18M5 17h14a2 2 0 002-2V5a2 2 0 00-2-2H5a2 2 0 00-2 2v10a2 2 0 002 2z"/>
            </svg>
          </div>
          <div class="favorite-info">
            <div class="favorite-name">{fav.name}</div>
            <div class="favorite-address">{fav.address}</div>
          </div>
          <div class="favorite-actions">
            <button
              class="btn btn-ghost btn-sm"
              onclick={(e) => { e.stopPropagation(); deleteFavorite(fav.id); }}
            >
              <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor">
                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M6 18L18 6M6 6l12 12"/>
              </svg>
            </button>
          </div>
        </div>
      {/each}
    {/if}

    <!-- Add favorite form -->
    {#if showAddFavorite}
      <div class="add-favorite-form">
        <div class="form-group">
          <input
            type="text"
            class="form-input"
            placeholder="Name (e.g., Living Room PC)"
            bind:value={newFavoriteName}
          />
        </div>
        <div class="form-group">
          <input
            type="text"
            class="form-input"
            placeholder="Address (IP or hostname)"
            bind:value={newFavoriteAddress}
          />
        </div>
        <div class="flex gap-2">
          <button class="btn btn-primary btn-sm" onclick={addFavorite}>
            Save
          </button>
          <button
            class="btn btn-ghost btn-sm"
            onclick={() => (showAddFavorite = false)}
          >
            Cancel
          </button>
        </div>
      </div>
    {:else}
      <button
        class="btn btn-secondary btn-sm mt-4"
        onclick={() => (showAddFavorite = true)}
      >
        <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor">
          <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 4v16m8-8H4"/>
        </svg>
        Add Favorite
      </button>
    {/if}
  </div>
</div>

<!-- File Selection Card -->
<div class="card">
  <div class="card-header">
    <h3 class="card-title">Files</h3>
    <p class="card-subtitle">Select files or a folder to send</p>
  </div>
  <div class="card-body">
    <!-- Drop zone -->
    <div
      class="drop-zone"
      class:active={isDragging}
      ondrop={handleDrop}
      ondragover={handleDragOver}
      ondragleave={handleDragLeave}
      onclick={openFilePicker}
    >
      <svg class="drop-zone-icon" viewBox="0 0 24 24" fill="none" stroke="currentColor">
        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M7 16a4 4 0 01-.88-7.903A5 5 0 1115.9 6L16 6a5 5 0 011 9.9M15 13l-3-3m0 0l-3 3m3-3v12"/>
      </svg>
      <p class="drop-zone-text">Drop files here or click to browse</p>
      <p class="drop-zone-hint">Supports any file type</p>
    </div>

    <!-- Folder picker button -->
    <button class="btn btn-secondary mt-3" onclick={openFolderPicker} style="width: 100%;">
      <svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor">
        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M3 7v10a2 2 0 002 2h14a2 2 0 002-2V9a2 2 0 00-2-2h-6l-2-2H5a2 2 0 00-2 2z"/>
      </svg>
      Select Folder
    </button>

    {#if fileAddError}
      <div class="form-error mt-2">{fileAddError}</div>
    {/if}

    <!-- Selected directory -->
    {#if selectedDirectory}
      <div class="selected-directory mt-4">
        <div class="directory-item">
          <svg class="folder-icon" viewBox="0 0 24 24" fill="none" stroke="currentColor">
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M3 7v10a2 2 0 002 2h14a2 2 0 002-2V9a2 2 0 00-2-2h-6l-2-2H5a2 2 0 00-2 2z"/>
          </svg>
          <div class="directory-info">
            <div class="directory-name">{selectedDirectory.name}</div>
            <div class="directory-path">{selectedDirectory.path}</div>
          </div>
          <button class="file-remove" onclick={clearDirectory}>
            <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor">
              <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M6 18L18 6M6 6l12 12"/>
            </svg>
          </button>
        </div>
      </div>
    {/if}

    <!-- Selected files list -->
    {#if selectedFiles.length > 0}
      <ul class="file-list mt-4">
        {#each selectedFiles as file, index}
          <li class="file-item">
            <svg class="file-icon" viewBox="0 0 24 24" fill="none" stroke="currentColor">
              <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9 12h6m-6 4h6m2 5H7a2 2 0 01-2-2V5a2 2 0 012-2h5.586a1 1 0 01.707.293l5.414 5.414a1 1 0 01.293.707V19a2 2 0 01-2 2z"/>
            </svg>
            <div class="file-info">
              <div class="file-name">{file.name}</div>
              <div class="file-size">{formatSize(file.size)}</div>
            </div>
            <button class="file-remove" onclick={() => removeFile(index)}>
              <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor">
                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M6 18L18 6M6 6l12 12"/>
              </svg>
            </button>
          </li>
        {/each}
      </ul>
    {/if}
  </div>
</div>

<!-- Send Action Card -->
<div class="card">
  <div class="card-body">
    {#if sendError}
      <div class="form-error mb-4">{sendError}</div>
    {/if}
    {#if sendSuccess}
      <div class="form-success mb-4">Files sent successfully!</div>
    {/if}

    {#if isSending && sendProgress}
      <div class="send-progress mb-4">
        <div class="progress-header">
          <span class="progress-status">
            {#if sendProgress.status === 'waiting'}
              Waiting for approval...
            {:else}
              Sending{sendProgress.currentFile ? `: ${sendProgress.currentFile}` : '...'}
            {/if}
          </span>
          <div class="progress-stats">
            {#if sendProgress.speedBps > 0}
              <span class="progress-speed">{formatSpeed(sendProgress.speedBps)}</span>
            {/if}
            {#if sendProgress.totalBytes > 0}
              <span class="progress-size">
                {formatSize(sendProgress.bytesTransferred || 0)} / {formatSize(sendProgress.totalBytes)}
              </span>
            {/if}
          </div>
        </div>
        <div class="progress-bar">
          <div
            class="progress-fill"
            class:indeterminate={sendProgress.status === 'waiting'}
            style="width: {sendProgress.totalBytes > 0 ? ((sendProgress.bytesTransferred || 0) / sendProgress.totalBytes) * 100 : 0}%"
          ></div>
        </div>
      </div>
    {/if}

    <div class="send-actions">
      <button
        class="btn btn-primary btn-lg"
        disabled={!canSend()}
        onclick={sendFiles}
        style="flex: 1;"
      >
        {#if isSending}
          {#if sendProgress?.status === 'waiting'}
            Waiting for approval...
          {:else}
            Sending...
          {/if}
        {:else}
          <svg width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor">
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M4 16v1a3 3 0 003 3h10a3 3 0 003-3v-1m-4-8l-4-4m0 0L8 8m4-4v12"/>
          </svg>
          {selectedDirectory ? 'Send Folder' : 'Send Files'}
        {/if}
      </button>
      {#if isSending}
        <button
          class="btn btn-destructive btn-lg"
          onclick={cancelTransfer}
          title="Cancel transfer"
        >
          <svg width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor">
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M6 18L18 6M6 6l12 12"/>
          </svg>
        </button>
      {/if}
    </div>
  </div>
</div>

<style>
  .add-favorite-form {
    margin-top: var(--space-4);
    padding-top: var(--space-4);
    border-top: 1px solid var(--border-muted);
  }

  .send-progress {
    padding: var(--space-3);
    background: var(--bg-elevated);
    border-radius: var(--radius-md);
  }

  .progress-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    margin-bottom: var(--space-2);
  }

  .progress-status {
    color: var(--text-primary);
    font-weight: 500;
  }

  .progress-stats {
    display: flex;
    align-items: center;
    gap: var(--space-3);
  }

  .progress-speed {
    color: var(--accent);
    font-size: var(--font-size-sm);
    font-weight: 500;
  }

  .progress-size {
    color: var(--text-muted);
    font-size: var(--font-size-sm);
  }

  .progress-bar {
    height: 8px;
    background: var(--bg-base);
    border-radius: var(--radius-sm);
    overflow: hidden;
  }

  .progress-fill {
    height: 100%;
    background: var(--primary);
    transition: width 0.2s ease;
  }

  .progress-fill.indeterminate {
    width: 30% !important;
    animation: indeterminate 1.5s ease-in-out infinite;
  }

  @keyframes indeterminate {
    0% {
      transform: translateX(-100%);
    }
    100% {
      transform: translateX(400%);
    }
  }

  .send-actions {
    display: flex;
    gap: var(--space-2);
  }

  .selected-directory {
    background: var(--bg-elevated);
    border-radius: var(--radius-md);
    padding: var(--space-3);
  }

  .directory-item {
    display: flex;
    align-items: center;
    gap: var(--space-3);
  }

  .folder-icon {
    width: 24px;
    height: 24px;
    color: var(--accent);
    flex-shrink: 0;
  }

  .directory-info {
    flex: 1;
    min-width: 0;
  }

  .directory-name {
    font-weight: 500;
    color: var(--text-primary);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .directory-path {
    font-size: var(--font-size-sm);
    color: var(--text-muted);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
</style>
