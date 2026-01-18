<!--
SPDX-License-Identifier: AGPL-3.0
Gosh Transfer - Settings View

Application settings including:
- Device name
- Port configuration
- Download directory
- Trusted hosts
-->
<script>
  import { onMount } from "svelte";
  import { invoke } from "@tauri-apps/api/core";
  import { open } from "@tauri-apps/plugin-dialog";

  // Props
  let { serverStatus = {}, onThemeChange = () => {} } = $props();

  // Settings state
  let settings = $state({
    port: 53317,
    deviceName: "",
    downloadDir: "",
    trustedHosts: [],
    receiveOnly: false,
    notificationsEnabled: true,
    theme: "system",
  });

  let isSaving = $state(false);
  let saveMessage = $state("");

  // Load settings on mount
  onMount(async () => {
    try {
      const loaded = await invoke("get_settings");
      settings = {
        port: loaded.port,
        deviceName: loaded.deviceName,
        downloadDir: loaded.downloadDir,
        trustedHosts: loaded.trustedHosts || [],
        receiveOnly: loaded.receiveOnly ?? false,
        notificationsEnabled: loaded.notificationsEnabled,
        theme: loaded.theme ?? "system",
      };
    } catch (e) {
      console.error("Failed to load settings:", e);
    }
  });

  // Save settings
  async function saveSettings() {
    isSaving = true;
    saveMessage = "";

    try {
      await invoke("update_settings", {
        newSettings: {
          port: settings.port,
          deviceName: settings.deviceName,
          downloadDir: settings.downloadDir,
          trustedHosts: settings.trustedHosts,
          receiveOnly: settings.receiveOnly,
          notificationsEnabled: settings.notificationsEnabled,
          theme: settings.theme,
        },
      });
      saveMessage = "Settings saved";
      setTimeout(() => (saveMessage = ""), 3000);
    } catch (e) {
      saveMessage = "Failed to save: " + e.toString();
    } finally {
      isSaving = false;
    }
  }

  // Browse for download directory
  async function browseDownloadDir() {
    try {
      const selected = await open({
        directory: true,
        multiple: false,
      });
      if (selected) {
        settings.downloadDir = selected;
      }
    } catch (e) {
      console.error("Failed to open directory picker:", e);
    }
  }

  // Add trusted host
  let newTrustedHost = $state("");
  function addTrustedHost() {
    if (newTrustedHost.trim() && !settings.trustedHosts.includes(newTrustedHost.trim())) {
      settings.trustedHosts = [...settings.trustedHosts, newTrustedHost.trim()];
      newTrustedHost = "";
    }
  }

  // Remove trusted host
  function removeTrustedHost(host) {
    settings.trustedHosts = settings.trustedHosts.filter((h) => h !== host);
  }

  // Toggle notifications
  function toggleNotifications() {
    settings.notificationsEnabled = !settings.notificationsEnabled;
  }

  function toggleReceiveOnly() {
    settings.receiveOnly = !settings.receiveOnly;
  }

  // Set theme with immediate preview
  function setTheme(theme) {
    settings.theme = theme;
    onThemeChange(theme);
  }
</script>

<div class="view-header">
  <h2 class="view-title">Settings</h2>
  <p class="view-subtitle">Configure your Gosh Transfer preferences</p>
</div>

<!-- Device Settings -->
<div class="card">
  <div class="card-header">
    <h3 class="card-title">Device</h3>
    <p class="card-subtitle">How others see your device</p>
  </div>
  <div class="card-body">
    <div class="form-group">
      <label class="form-label" for="device-name">Device Name</label>
      <input
        id="device-name"
        type="text"
        class="form-input"
        bind:value={settings.deviceName}
        placeholder="My Computer"
      />
      <p class="form-hint">This name is shown to other devices during transfers</p>
    </div>
  </div>
</div>

<!-- Appearance Settings -->
<div class="card">
  <div class="card-header">
    <h3 class="card-title">Appearance</h3>
    <p class="card-subtitle">Choose your preferred color theme</p>
  </div>
  <div class="card-body">
    <div class="setting-row">
      <div>
        <div class="setting-label">Theme</div>
        <div class="setting-description">
          Select dark, light, or follow your system preference
        </div>
      </div>
      <div class="theme-toggle">
        <button
          class:active={settings.theme === "dark"}
          onclick={() => setTheme("dark")}
        >
          Dark
        </button>
        <button
          class:active={settings.theme === "light"}
          onclick={() => setTheme("light")}
        >
          Light
        </button>
        <button
          class:active={settings.theme === "system"}
          onclick={() => setTheme("system")}
        >
          System
        </button>
      </div>
    </div>
  </div>
</div>

<!-- Network Settings -->
<div class="card">
  <div class="card-header">
    <h3 class="card-title">Network</h3>
    <p class="card-subtitle">Server and connection settings</p>
  </div>
  <div class="card-body">
    <div class="form-group">
      <label class="form-label" for="port">Port</label>
      <input
        id="port"
        type="number"
        class="form-input"
        bind:value={settings.port}
        min="1024"
        max="65535"
      />
      <p class="form-hint">
        Default: 53317. Changes take effect immediately.
      </p>
    </div>

    <!-- Network interfaces (read-only) -->
    <div class="interfaces-info">
      <h4 class="subsection-title">Active Interfaces</h4>
      {#if serverStatus.interfaces?.length > 0}
        <div class="interfaces-grid">
          {#each serverStatus.interfaces.filter(i => !i.isLoopback) as iface}
            <div class="interface-badge">
              <span class="interface-name-small">{iface.name}</span>
              <code class="interface-ip-small">{iface.ip}</code>
            </div>
          {/each}
        </div>
      {:else}
        <p class="text-muted">No interfaces detected</p>
      {/if}
    </div>
  </div>
</div>

<!-- Transfer Mode -->
<div class="card">
  <div class="card-header">
    <h3 class="card-title">Transfer Mode</h3>
    <p class="card-subtitle">Control outgoing transfers</p>
  </div>
  <div class="card-body">
    <div class="setting-row">
      <div>
        <div class="setting-label">Receive-only Mode</div>
        <div class="setting-description">
          Disable sending and hide the Send tab
        </div>
      </div>
      <button
        class="toggle"
        class:active={settings.receiveOnly}
        onclick={toggleReceiveOnly}
      >
        <span class="toggle-knob"></span>
      </button>
    </div>
  </div>
</div>

<!-- Storage Settings -->
<div class="card">
  <div class="card-header">
    <h3 class="card-title">Storage</h3>
    <p class="card-subtitle">Where received files are saved</p>
  </div>
  <div class="card-body">
    <div class="form-group">
      <label class="form-label" for="download-dir">Download Directory</label>
      <div class="input-with-button">
        <input
          id="download-dir"
          type="text"
          class="form-input"
          bind:value={settings.downloadDir}
          placeholder="~/Downloads"
        />
        <button class="btn btn-secondary" onclick={browseDownloadDir}>
          Browse
        </button>
      </div>
    </div>
  </div>
</div>

<!-- Trusted Hosts -->
<div class="card">
  <div class="card-header">
    <h3 class="card-title">Trusted Hosts</h3>
    <p class="card-subtitle">Auto-accept transfers from these addresses</p>
  </div>
  <div class="card-body">
    {#if settings.trustedHosts.length > 0}
      <ul class="trusted-hosts-list">
        {#each settings.trustedHosts as host}
          <li class="trusted-host-item">
            <code>{host}</code>
            <button
              class="btn btn-ghost btn-sm"
              onclick={() => removeTrustedHost(host)}
            >
              <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor">
                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M6 18L18 6M6 6l12 12"/>
              </svg>
            </button>
          </li>
        {/each}
      </ul>
    {:else}
      <p class="text-muted mb-4">No trusted hosts configured</p>
    {/if}

    <div class="add-trusted-host">
      <input
        type="text"
        class="form-input"
        bind:value={newTrustedHost}
        placeholder="192.168.1.100 or hostname"
        onkeydown={(e) => e.key === "Enter" && addTrustedHost()}
      />
      <button class="btn btn-secondary" onclick={addTrustedHost}>
        Add
      </button>
    </div>
  </div>
</div>

<!-- Notifications -->
<div class="card">
  <div class="card-header">
    <h3 class="card-title">Notifications</h3>
    <p class="card-subtitle">System notification preferences</p>
  </div>
  <div class="card-body">
    <div class="setting-row">
      <div>
        <div class="setting-label">Enable Notifications</div>
        <div class="setting-description">
          Show system notifications for incoming transfers
        </div>
      </div>
      <button
        class="toggle"
        class:active={settings.notificationsEnabled}
        onclick={toggleNotifications}
      >
        <span class="toggle-knob"></span>
      </button>
    </div>
  </div>
</div>

<!-- Save Button -->
<div class="save-section">
  {#if saveMessage}
    <span
      class="save-message"
      class:success={saveMessage === "Settings saved"}
    >
      {saveMessage}
    </span>
  {/if}
  <button
    class="btn btn-primary"
    onclick={saveSettings}
    disabled={isSaving}
  >
    {isSaving ? "Saving..." : "Save Settings"}
  </button>
</div>


<style>
  .input-with-button {
    display: flex;
    gap: var(--space-2);
  }

  .input-with-button .form-input {
    flex: 1;
  }

  .interfaces-info {
    margin-top: var(--space-4);
    padding-top: var(--space-4);
    border-top: 1px solid var(--border-muted);
  }

  .subsection-title {
    font-size: var(--font-size-sm);
    font-weight: 500;
    color: var(--text-secondary);
    margin-bottom: var(--space-3);
  }

  .interfaces-grid {
    display: flex;
    flex-wrap: wrap;
    gap: var(--space-2);
  }

  .interface-badge {
    display: inline-flex;
    align-items: center;
    gap: var(--space-2);
    padding: var(--space-2) var(--space-3);
    background-color: var(--bg-elevated);
    border-radius: var(--radius-md);
  }

  .interface-name-small {
    font-size: var(--font-size-sm);
    color: var(--text-muted);
  }

  .interface-ip-small {
    font-size: var(--font-size-sm);
    color: var(--accent);
    background: none;
    padding: 0;
  }

  .trusted-hosts-list {
    list-style: none;
    margin-bottom: var(--space-4);
  }

  .trusted-host-item {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: var(--space-2) var(--space-3);
    background-color: var(--bg-elevated);
    border-radius: var(--radius-md);
    margin-bottom: var(--space-2);
  }

  .trusted-host-item code {
    font-size: var(--font-size-sm);
    color: var(--text-primary);
    background: none;
  }

  .add-trusted-host {
    display: flex;
    gap: var(--space-2);
  }

  .add-trusted-host .form-input {
    flex: 1;
  }

  .save-section {
    display: flex;
    justify-content: flex-end;
    align-items: center;
    gap: var(--space-4);
    margin-top: var(--space-4);
    padding-top: var(--space-4);
    border-top: 1px solid var(--border-muted);
  }

  .save-message {
    font-size: var(--font-size-sm);
    color: var(--status-error);
  }

  .save-message.success {
    color: var(--status-success);
  }

</style>
