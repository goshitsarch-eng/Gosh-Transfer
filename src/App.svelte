<!--
SPDX-License-Identifier: AGPL-3.0
Gosh Transfer - Main application component

Single-window UI with permanent sidebar and card-based main content.
-->
<script>
  import { onMount } from "svelte";
  import { invoke } from "@tauri-apps/api/core";
  import { listen } from "@tauri-apps/api/event";
  import { initTheme, applyTheme, initPlatform } from "./lib/theme.js";

  // View components
  import SendView from "./lib/components/SendView.svelte";
  import ReceiveView from "./lib/components/ReceiveView.svelte";
  import TransfersView from "./lib/components/TransfersView.svelte";
  import SettingsView from "./lib/components/SettingsView.svelte";
  import AboutView from "./lib/components/AboutView.svelte";

  // Theme controller (set after initialization)
  let themeController = null;

  // Navigation state
  let currentView = $state("send");
  let receiveOnly = $state(false);

  // Server status
  let serverStatus = $state({
    running: false,
    port: 53317,
    interfaces: [],
    device_name: "Loading...",
  });

  // Pending transfer notifications
  let pendingTransfers = $state([]);

  // Active transfers (in progress) and recent results
  let activeTransfers = $state([]);
  let recentResults = $state([]);

  // Navigation items
  const navItems = [
    { id: "send", label: "Send", icon: "upload" },
    { id: "receive", label: "Receive", icon: "download" },
    { id: "transfers", label: "Transfers", icon: "history" },
    { id: "settings", label: "Settings", icon: "settings" },
    { id: "about", label: "About", icon: "info" },
  ];

  // Load server status on mount
  onMount(async () => {
    // Initialize platform detection for native styling
    initPlatform();

    try {
      serverStatus = await invoke("get_server_status");
    } catch (e) {
      console.error("Failed to get server status:", e);
    }

    try {
      const settings = await invoke("get_settings");
      receiveOnly = settings.receiveOnly ?? false;
      if (receiveOnly) {
        currentView = "receive";
      }
      // Initialize theme
      themeController = initTheme(settings.theme ?? "system");
    } catch (e) {
      console.error("Failed to get settings:", e);
      // Default to system theme if settings fail to load
      themeController = initTheme("system");
    }

    // Listen for incoming transfer requests
    const unlistenRequest = await listen("transfer-request", (event) => {
      pendingTransfers = [...pendingTransfers, event.payload.transfer];
    });

    // Listen for progress updates (receiving)
    const unlistenProgress = await listen("transfer-progress", (event) => {
      const progress = event.payload.progress;
      const existing = activeTransfers.find(t => t.id === progress.transferId);
      if (existing) {
        activeTransfers = activeTransfers.map(t =>
          t.id === progress.transferId ? { ...t, ...progress } : t
        );
      }
    });

    // Listen for transfer completions
    const unlistenComplete = await listen("transfer-complete", (event) => {
      const { transferId } = event.payload;
      const completed = activeTransfers.find(t => t.id === transferId);
      activeTransfers = activeTransfers.filter(t => t.id !== transferId);
      pendingTransfers = pendingTransfers.filter(t => t.id !== transferId);
      if (completed) {
        recentResults = [...recentResults, { ...completed, status: 'completed' }];
        setTimeout(() => {
          recentResults = recentResults.filter(r => r.id !== transferId);
        }, 5000);
      }
    });

    // Listen for transfer failures
    const unlistenFailed = await listen("transfer-failed", (event) => {
      const { transferId, error } = event.payload;
      activeTransfers = activeTransfers.filter(t => t.id !== transferId);
      pendingTransfers = pendingTransfers.filter(t => t.id !== transferId);
      recentResults = [...recentResults, { id: transferId, status: 'failed', error }];
      setTimeout(() => {
        recentResults = recentResults.filter(r => r.id !== transferId);
      }, 8000);
    });

    const unlistenSettings = await listen("settings-updated", (event) => {
      receiveOnly = event.payload.receiveOnly ?? false;
      if (receiveOnly && currentView === "send") {
        currentView = "receive";
      }
      // Update theme if changed
      if (event.payload.theme && themeController) {
        themeController.setTheme(event.payload.theme);
      }
    });

    return () => {
      unlistenRequest();
      unlistenProgress();
      unlistenComplete();
      unlistenFailed();
      unlistenSettings();
      if (themeController) {
        themeController.cleanup();
      }
    };
  });

  // Handler for immediate theme changes from SettingsView
  function handleThemeChange(theme) {
    if (themeController) {
      themeController.setTheme(theme);
    } else {
      applyTheme(theme);
    }
  }

  // Navigate to a view
  function navigate(viewId) {
    if (receiveOnly && viewId === "send") return;
    currentView = viewId;
  }

  async function acceptPendingTransfer(transferId) {
    try {
      await invoke("accept_transfer", { transferId });
      // Move to active transfers (will get progress updates)
      const transfer = pendingTransfers.find(t => t.id === transferId);
      if (transfer) {
        activeTransfers = [...activeTransfers, {
          ...transfer,
          bytesTransferred: 0,
          totalBytes: transfer.totalSize
        }];
      }
      pendingTransfers = pendingTransfers.filter((t) => t.id !== transferId);
    } catch (e) {
      console.error("Failed to accept transfer:", e);
    }
  }

  function dismissResult(resultId) {
    recentResults = recentResults.filter(r => r.id !== resultId);
  }

  async function rejectPendingTransfer(transferId) {
    try {
      await invoke("reject_transfer", { transferId });
      pendingTransfers = pendingTransfers.filter((t) => t.id !== transferId);
    } catch (e) {
      console.error("Failed to reject transfer:", e);
    }
  }

  $effect(() => {
    if (receiveOnly && currentView === "send") {
      currentView = "receive";
    }
  });

  // Get icon SVG
  function getIcon(name) {
    const icons = {
      upload: `<path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M4 16v1a3 3 0 003 3h10a3 3 0 003-3v-1m-4-8l-4-4m0 0L8 8m4-4v12"/>`,
      download: `<path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M4 16v1a3 3 0 003 3h10a3 3 0 003-3v-1m-4-4l-4 4m0 0l-4-4m4 4V4"/>`,
      history: `<path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 8v4l3 3m6-3a9 9 0 11-18 0 9 9 0 0118 0z"/>`,
      settings: `<path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M10.325 4.317c.426-1.756 2.924-1.756 3.35 0a1.724 1.724 0 002.573 1.066c1.543-.94 3.31.826 2.37 2.37a1.724 1.724 0 001.065 2.572c1.756.426 1.756 2.924 0 3.35a1.724 1.724 0 00-1.066 2.573c.94 1.543-.826 3.31-2.37 2.37a1.724 1.724 0 00-2.572 1.065c-.426 1.756-2.924 1.756-3.35 0a1.724 1.724 0 00-2.573-1.066c-1.543.94-3.31-.826-2.37-2.37a1.724 1.724 0 00-1.065-2.572c-1.756-.426-1.756-2.924 0-3.35a1.724 1.724 0 001.066-2.573c-.94-1.543.826-3.31 2.37-2.37.996.608 2.296.07 2.572-1.065z"/><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M15 12a3 3 0 11-6 0 3 3 0 016 0z"/>`,
      info: `<path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M13 16h-1v-4h-1m1-4h.01M21 12a9 9 0 11-18 0 9 9 0 0118 0z"/>`,
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
    {#each (receiveOnly ? navItems.filter((item) => item.id !== "send") : navItems) as item}
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
      <ReceiveView
        {pendingTransfers}
        {activeTransfers}
        {recentResults}
        onAccept={acceptPendingTransfer}
        onReject={rejectPendingTransfer}
        onDismissResult={dismissResult}
      />
    {:else if currentView === "transfers"}
      <TransfersView />
    {:else if currentView === "settings"}
      <SettingsView {serverStatus} onThemeChange={handleThemeChange} />
    {:else if currentView === "about"}
      <AboutView />
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
