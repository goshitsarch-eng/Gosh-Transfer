<!--
SPDX-License-Identifier: AGPL-3.0
Gosh Transfer - Main application component

Single-window UI with permanent sidebar and card-based main content.
NOTICE: This project is NOT affiliated with Motrix.
-->
<script>
  import { onMount } from "svelte";
  import { invoke } from "@tauri-apps/api/core";
  import { listen } from "@tauri-apps/api/event";

  // View components
  import SendView from "./lib/components/SendView.svelte";
  import ReceiveView from "./lib/components/ReceiveView.svelte";
  import TransfersView from "./lib/components/TransfersView.svelte";
  import SettingsView from "./lib/components/SettingsView.svelte";

  // Navigation state
  let currentView = $state("send");

  // Server status
  let serverStatus = $state({
    running: false,
    port: 53317,
    interfaces: [],
    device_name: "Loading...",
  });

  // Pending transfer notifications
  let pendingTransfers = $state([]);

  // Navigation items
  const navItems = [
    { id: "send", label: "Send", icon: "upload" },
    { id: "receive", label: "Receive", icon: "download" },
    { id: "transfers", label: "Transfers", icon: "history" },
    { id: "settings", label: "Settings", icon: "settings" },
  ];

  // Load server status on mount
  onMount(async () => {
    try {
      serverStatus = await invoke("get_server_status");
    } catch (e) {
      console.error("Failed to get server status:", e);
    }

    // Listen for incoming transfer requests
    const unlistenRequest = await listen("transfer-request", (event) => {
      pendingTransfers = [...pendingTransfers, event.payload.transfer];
    });

    // Listen for transfer completions
    const unlistenComplete = await listen("transfer-complete", (event) => {
      // Remove from pending
      pendingTransfers = pendingTransfers.filter(
        (t) => t.id !== event.payload.transfer_id
      );
    });

    return () => {
      unlistenRequest();
      unlistenComplete();
    };
  });

  // Navigate to a view
  function navigate(viewId) {
    currentView = viewId;
  }

  // Get icon SVG
  function getIcon(name) {
    const icons = {
      upload: `<path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M4 16v1a3 3 0 003 3h10a3 3 0 003-3v-1m-4-8l-4-4m0 0L8 8m4-4v12"/>`,
      download: `<path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M4 16v1a3 3 0 003 3h10a3 3 0 003-3v-1m-4-4l-4 4m0 0l-4-4m4 4V4"/>`,
      history: `<path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 8v4l3 3m6-3a9 9 0 11-18 0 9 9 0 0118 0z"/>`,
      settings: `<path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M10.325 4.317c.426-1.756 2.924-1.756 3.35 0a1.724 1.724 0 002.573 1.066c1.543-.94 3.31.826 2.37 2.37a1.724 1.724 0 001.065 2.572c1.756.426 1.756 2.924 0 3.35a1.724 1.724 0 00-1.066 2.573c.94 1.543-.826 3.31-2.37 2.37a1.724 1.724 0 00-2.572 1.065c-.426 1.756-2.924 1.756-3.35 0a1.724 1.724 0 00-2.573-1.066c-1.543.94-3.31-.826-2.37-2.37a1.724 1.724 0 00-1.065-2.572c-1.756-.426-1.756-2.924 0-3.35a1.724 1.724 0 001.066-2.573c-.94-1.543.826-3.31 2.37-2.37.996.608 2.296.07 2.572-1.065z"/><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M15 12a3 3 0 11-6 0 3 3 0 016 0z"/>`,
    };
    return icons[name] || "";
  }
</script>

<div class="app-layout">
  <!-- Sidebar -->
  <aside class="sidebar">
    <!-- Logo and app name -->
    <div class="sidebar-header">
      <div class="sidebar-logo">
        <svg
          viewBox="0 0 24 24"
          fill="none"
          stroke="currentColor"
          stroke-width="2"
        >
          <path
            d="M13 2L3 14h9l-1 8 10-12h-9l1-8z"
            stroke-linejoin="round"
            stroke-linecap="round"
          />
        </svg>
        <h1>Gosh Transfer</h1>
      </div>
    </div>

    <!-- Navigation -->
    <nav class="sidebar-nav">
      {#each navItems as item}
        <button
          class="nav-item"
          class:active={currentView === item.id}
          onclick={() => navigate(item.id)}
        >
          <svg viewBox="0 0 24 24" fill="none" stroke="currentColor">
            {@html getIcon(item.icon)}
          </svg>
          <span>{item.label}</span>
          {#if item.id === "receive" && pendingTransfers.length > 0}
            <span class="notification-badge">{pendingTransfers.length}</span>
          {/if}
        </button>
      {/each}
    </nav>

    <!-- Server status -->
    <div class="sidebar-footer">
      <div class="server-status">
        <span
          class="status-dot"
          class:offline={!serverStatus.running}
        ></span>
        <span>Port {serverStatus.port}</span>
      </div>
    </div>
  </aside>

  <!-- Main content area -->
  <main class="main-content">
    {#if currentView === "send"}
      <SendView />
    {:else if currentView === "receive"}
      <ReceiveView {pendingTransfers} />
    {:else if currentView === "transfers"}
      <TransfersView />
    {:else if currentView === "settings"}
      <SettingsView {serverStatus} />
    {/if}
  </main>
</div>

<style>
  .notification-badge {
    background-color: var(--primary);
    color: white;
    font-size: 11px;
    font-weight: 600;
    padding: 2px 6px;
    border-radius: 10px;
    margin-left: auto;
  }
</style>
