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
  import { open } from "@tauri-apps/plugin-dialog";
  import { getCurrentWindow } from "@tauri-apps/api/window";

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
  let isDragging = $state(false);
  let fileAddError = $state("");
  let hasTauriDrop = $state(false);

  // Transfer state
  let isSending = $state(false);
  let sendError = $state("");
  let sendSuccess = $state(false);

  // Default port
  const DEFAULT_PORT = 53317;

  // Load favorites on mount
  onMount(async () => {
    try {
      favorites = await invoke("list_favorites");
    } catch (e) {
      console.error("Failed to load favorites:", e);
    }

    try {
      const unlisten = await getCurrentWindow().onDragDropEvent((event) => {
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

      return () => {
        unlisten();
      };
    } catch (e) {
      console.error("Failed to register drag-drop handler:", e);
    }
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
      }
    } catch (e) {
      console.error("Failed to open file picker:", e);
    }
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

  // Send files
  async function sendFiles() {
    if (!resolveResult?.success || selectedFiles.length === 0) return;

    isSending = true;
    sendError = "";
    sendSuccess = false;

    try {
      const ip = resolveResult.ips[0];
      const filePaths = selectedFiles.map((f) => f.path);

      await invoke("send_files", {
        address: ip,
        port: DEFAULT_PORT,
        filePaths: filePaths,
      });

      sendSuccess = true;
      selectedFiles = [];

      // Optionally save as favorite
      // await saveFavoritePrompt();
    } catch (e) {
      sendError = e.toString();
    } finally {
      isSending = false;
    }
  }

  // Check if send is enabled
  function canSend() {
    return resolveResult?.success && selectedFiles.length > 0 && !isSending;
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
    <p class="card-subtitle">Select files to send</p>
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
    {#if fileAddError}
      <div class="form-error mt-2">{fileAddError}</div>
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
    <button
      class="btn btn-primary btn-lg"
      disabled={!canSend()}
      onclick={sendFiles}
      style="width: 100%;"
    >
      {#if isSending}
        Sending...
      {:else}
        <svg width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor">
          <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M4 16v1a3 3 0 003 3h10a3 3 0 003-3v-1m-4-8l-4-4m0 0L8 8m4-4v12"/>
        </svg>
        Send Files
      {/if}
    </button>
  </div>
</div>

<style>
  .add-favorite-form {
    margin-top: var(--space-4);
    padding-top: var(--space-4);
    border-top: 1px solid var(--border-muted);
  }
</style>
