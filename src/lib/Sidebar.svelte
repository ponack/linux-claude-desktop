<script>
  import { invoke } from "@tauri-apps/api/core";
  import { listen } from "@tauri-apps/api/event";
  import { onMount } from "svelte";

  let { activeConversationId, onSelect, onNewChat, openSettings, openComparison, openComputerUse, onBackToChat, currentView = "chat", refreshKey, collapsed = false } = $props();

  let conversations = $state([]);
  let searchQuery = $state("");

  // Update system
  let updateInfo = $state(null);
  let showUpdateDialog = $state(false);
  let updateState = $state("idle"); // idle | downloading | installing | done | error
  let downloadProgress = $state(0);
  let updateError = $state("");
  let downloadedPath = $state("");
  let checkIntervalId = null;

  let filteredConversations = $derived(
    searchQuery.trim()
      ? conversations.filter((c) =>
          c.title.toLowerCase().includes(searchQuery.trim().toLowerCase())
        )
      : conversations
  );

  async function loadConversations() {
    try {
      conversations = await invoke("get_conversations");
    } catch (e) {
      console.error("Failed to load conversations:", e);
    }
  }

  onMount(() => {
    loadConversations();
    setupUpdateChecking();

    const unlisten = listen("update-progress", (event) => {
      downloadProgress = event.payload;
    });

    return () => {
      if (checkIntervalId) clearInterval(checkIntervalId);
      unlisten.then((fn) => fn());
    };
  });

  async function setupUpdateChecking() {
    const interval = await invoke("get_update_interval").catch(() => "startup");

    if (interval === "never") return;

    // Always check on startup (unless "never")
    await checkUpdates();

    // Set up recurring checks if interval is numeric
    const ms = parseInt(interval);
    if (!isNaN(ms) && ms > 0) {
      checkIntervalId = setInterval(checkUpdates, ms);
    }
  }

  async function checkUpdates() {
    try {
      const info = await invoke("check_for_updates");
      if (info.has_update) {
        // Check if user skipped this version
        const skipped = await invoke("get_skipped_version").catch(() => "");
        if (skipped === info.latest_version) return;
        updateInfo = info;
      }
    } catch (e) {
      // Silent fail — update check is non-critical
    }
  }

  function openUpdateDialog() {
    showUpdateDialog = true;
    updateState = "idle";
    updateError = "";
    downloadProgress = 0;
  }

  function closeUpdateDialog() {
    showUpdateDialog = false;
  }

  async function skipVersion() {
    if (updateInfo) {
      await invoke("set_skipped_version", { version: updateInfo.latest_version }).catch(() => {});
    }
    updateInfo = null;
    showUpdateDialog = false;
  }

  async function downloadAndInstall() {
    if (!updateInfo?.deb_asset_url) {
      updateError = "No .deb package found for this release. Please download manually.";
      updateState = "error";
      return;
    }

    updateState = "downloading";
    downloadProgress = 0;
    updateError = "";

    try {
      downloadedPath = await invoke("download_update", { url: updateInfo.deb_asset_url });
      updateState = "installing";

      await invoke("install_update", { debPath: downloadedPath });
      updateState = "done";
    } catch (e) {
      updateError = String(e);
      updateState = "error";
    }
  }

  async function restartApp() {
    try {
      await invoke("restart_app");
    } catch (e) {
      // Fallback if restart command fails
      window.location.reload();
    }
  }

  $effect(() => {
    // Re-fetch when refreshKey changes
    refreshKey;
    loadConversations();
  });

  async function handleDelete(e, id) {
    e.stopPropagation();
    try {
      await invoke("delete_conversation", { id });
      if (activeConversationId === id) {
        onNewChat();
      }
      await loadConversations();
    } catch (err) {
      console.error("Failed to delete:", err);
    }
  }
</script>

<aside class="sidebar" class:collapsed aria-label="Conversations sidebar">
  <div class="sidebar-header">
    <div class="sidebar-brand">
      <img src="/assets/logo.svg" alt="Linux Claude Desktop" class="sidebar-logo" />
      {#if !collapsed}
        <span class="sidebar-title">LCD</span>
      {/if}
    </div>
    {#if !collapsed}
      <button class="new-chat-btn" onclick={onNewChat} aria-label="Start new chat">
        + New Chat
      </button>
    {/if}
  </div>

  {#if !collapsed}
    <div class="search-box">
      <input
        type="text"
        bind:value={searchQuery}
        placeholder="Search conversations..."
        aria-label="Search conversations"
      />
    </div>

    <div class="conversations-list" role="list" aria-label="Conversation history">
      {#each filteredConversations as conv (conv.id)}
        <div
          class="conversation-item"
          class:active={activeConversationId === conv.id}
          role="button"
          tabindex="0"
          onclick={() => onSelect(conv.id)}
          onkeydown={(e) => e.key === 'Enter' && onSelect(conv.id)}
          aria-label="Open conversation: {conv.title}"
          aria-current={activeConversationId === conv.id ? "true" : undefined}
        >
          <span class="conv-title">{conv.title}</span>
          <button class="delete-btn" onclick={(e) => handleDelete(e, conv.id)} aria-label="Delete conversation: {conv.title}">
            ×
          </button>
        </div>
      {/each}
    </div>
  {/if}

  <div class="sidebar-footer">
    {#if !collapsed && updateInfo}
      <button class="update-banner" onclick={openUpdateDialog} aria-label="Update available: version {updateInfo.latest_version}">
        Update available: v{updateInfo.latest_version}
      </button>
    {/if}
    <button
      class="nav-btn"
      class:active={currentView === "chat"}
      onclick={onBackToChat}
      aria-label="Chat"
      title="Chat (Ctrl+N)"
    >
      <svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
        <path d="M21 15a2 2 0 0 1-2 2H7l-4 4V5a2 2 0 0 1 2-2h14a2 2 0 0 1 2 2z"/>
      </svg>
      {#if !collapsed}<span>Chat</span>{/if}
    </button>
    <button
      class="nav-btn"
      class:active={currentView === "computer-use"}
      onclick={openComputerUse}
      aria-label="Computer use"
      title="Computer Use (Ctrl+Shift+U)"
    >
      <svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
        <rect x="2" y="3" width="20" height="14" rx="2"/>
        <path d="M8 21h8M12 17v4"/>
      </svg>
      {#if !collapsed}<span>Computer</span>{/if}
    </button>
    <button
      class="nav-btn"
      class:active={currentView === "compare"}
      onclick={openComparison}
      aria-label="Compare models"
      title="Compare models (Ctrl+Shift+M)"
    >
      <svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
        <rect x="2" y="3" width="8" height="18" rx="1"/><rect x="14" y="3" width="8" height="18" rx="1"/>
      </svg>
      {#if !collapsed}<span>Compare</span>{/if}
    </button>
    <button
      class="nav-btn"
      class:active={currentView === "settings"}
      onclick={openSettings}
      aria-label="Settings"
      title="Settings (Ctrl+,)"
    >
      <svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
        <circle cx="12" cy="12" r="3"></circle>
        <path d="M19.4 15a1.65 1.65 0 0 0 .33 1.82l.06.06a2 2 0 0 1 0 2.83 2 2 0 0 1-2.83 0l-.06-.06a1.65 1.65 0 0 0-1.82-.33 1.65 1.65 0 0 0-1 1.51V21a2 2 0 0 1-2 2 2 2 0 0 1-2-2v-.09A1.65 1.65 0 0 0 9 19.4a1.65 1.65 0 0 0-1.82.33l-.06.06a2 2 0 0 1-2.83 0 2 2 0 0 1 0-2.83l.06-.06A1.65 1.65 0 0 0 4.68 15a1.65 1.65 0 0 0-1.51-1H3a2 2 0 0 1-2-2 2 2 0 0 1 2-2h.09A1.65 1.65 0 0 0 4.6 9a1.65 1.65 0 0 0-.33-1.82l-.06-.06a2 2 0 0 1 0-2.83 2 2 0 0 1 2.83 0l.06.06A1.65 1.65 0 0 0 9 4.68a1.65 1.65 0 0 0 1-1.51V3a2 2 0 0 1 2-2 2 2 0 0 1 2 2v.09a1.65 1.65 0 0 0 1 1.51 1.65 1.65 0 0 0 1.82-.33l.06-.06a2 2 0 0 1 2.83 0 2 2 0 0 1 0 2.83l-.06.06A1.65 1.65 0 0 0 19.4 9a1.65 1.65 0 0 0 1.51 1H21a2 2 0 0 1 2 2 2 2 0 0 1-2 2h-.09a1.65 1.65 0 0 0-1.51 1z"></path>
      </svg>
      {#if !collapsed}<span>Settings</span>{/if}
    </button>
  </div>
</aside>

{#if showUpdateDialog}
  <div class="update-overlay" onclick={closeUpdateDialog} role="dialog" aria-modal="true" aria-labelledby="update-dialog-title">
    <div class="update-dialog" onclick={(e) => e.stopPropagation()}>
      <div class="update-dialog-header">
        <h3 id="update-dialog-title">Update Available</h3>
        <button class="update-close" onclick={closeUpdateDialog} aria-label="Close">&times;</button>
      </div>

      <div class="update-dialog-body">
        <div class="version-info">
          <span class="version-badge old">v{updateInfo.current_version}</span>
          <span class="version-arrow">&rarr;</span>
          <span class="version-badge new">v{updateInfo.latest_version}</span>
        </div>

        {#if updateInfo.release_notes}
          <div class="release-notes">
            <h4>Release Notes</h4>
            <div class="release-notes-content">{updateInfo.release_notes}</div>
          </div>
        {/if}

        {#if updateState === "downloading"}
          <div class="progress-section">
            <div class="progress-label">Downloading... {downloadProgress}%</div>
            <div class="progress-bar">
              <div class="progress-fill" style="width: {downloadProgress}%"></div>
            </div>
          </div>
        {:else if updateState === "installing"}
          <div class="progress-section">
            <div class="progress-label">Installing... Please enter your password if prompted.</div>
          </div>
        {:else if updateState === "done"}
          <div class="update-success">
            Update installed successfully. Restart to apply.
          </div>
        {:else if updateState === "error"}
          <div class="update-error">{updateError}</div>
        {/if}
      </div>

      <div class="update-dialog-footer">
        {#if updateState === "idle"}
          <button class="update-btn primary" onclick={downloadAndInstall}>
            Download & Install
          </button>
          <a class="update-btn secondary" href={updateInfo.download_url} target="_blank" rel="noopener">
            View on GitHub
          </a>
          <button class="update-btn ghost" onclick={skipVersion}>
            Skip this version
          </button>
        {:else if updateState === "done"}
          <button class="update-btn primary" onclick={restartApp}>
            Restart Now
          </button>
          <button class="update-btn secondary" onclick={closeUpdateDialog}>
            Later
          </button>
        {:else if updateState === "error"}
          <button class="update-btn primary" onclick={downloadAndInstall}>
            Retry
          </button>
          <a class="update-btn secondary" href={updateInfo.download_url} target="_blank" rel="noopener">
            Download Manually
          </a>
          <button class="update-btn ghost" onclick={closeUpdateDialog}>
            Close
          </button>
        {/if}
      </div>
    </div>
  </div>
{/if}

<style>
  .sidebar {
    width: 260px;
    min-width: 260px;
    background: var(--bg-secondary);
    border-right: 1px solid var(--border);
    display: flex;
    flex-direction: column;
    height: 100%;
    transition: width 0.2s ease, min-width 0.2s ease;
  }

  .sidebar.collapsed {
    width: 56px;
    min-width: 56px;
  }

  .sidebar.collapsed .sidebar-header {
    align-items: center;
  }

  .sidebar.collapsed .sidebar-footer {
    align-items: center;
  }

  .sidebar.collapsed .nav-btn {
    width: 36px;
    height: 36px;
    padding: 0;
    justify-content: center;
  }

  .sidebar-header {
    padding: 12px;
    border-bottom: 1px solid var(--border);
    display: flex;
    flex-direction: column;
    gap: 10px;
  }

  .sidebar-brand {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 0 2px;
  }

  .sidebar-logo {
    width: 28px;
    height: 28px;
  }

  .sidebar-title {
    font-size: 15px;
    font-weight: 700;
    color: var(--text-primary);
    letter-spacing: 0.5px;
  }

  .new-chat-btn {
    width: 100%;
    padding: 10px;
    background: var(--accent);
    color: white;
    border-radius: 8px;
    font-size: 14px;
    font-weight: 500;
    transition: background 0.2s;
  }

  .new-chat-btn:hover {
    background: var(--accent-hover);
  }

  .search-box {
    padding: 8px 12px 0;
  }

  .search-box input {
    width: 100%;
    padding: 7px 10px;
    background: var(--bg-input);
    border: 1px solid var(--border);
    border-radius: 8px;
    font-size: 13px;
    outline: none;
    transition: border-color 0.15s;
    box-sizing: border-box;
  }

  .search-box input:focus {
    border-color: var(--accent);
  }

  .conversations-list {
    flex: 1;
    overflow-y: auto;
    padding: 8px;
  }

  .conversation-item {
    width: 100%;
    padding: 10px 12px;
    border-radius: 8px;
    text-align: left;
    display: flex;
    align-items: center;
    justify-content: space-between;
    transition: background 0.15s;
    margin-bottom: 2px;
  }

  .conversation-item:hover {
    background: var(--bg-tertiary);
  }

  .conversation-item.active {
    background: var(--bg-tertiary);
  }

  .conv-title {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    flex: 1;
    font-size: 13px;
  }

  .delete-btn {
    opacity: 0;
    font-size: 18px;
    color: var(--text-muted);
    padding: 0 4px;
    transition: opacity 0.15s;
  }

  .conversation-item:hover .delete-btn {
    opacity: 1;
  }

  .delete-btn:hover {
    color: var(--danger);
  }

  .sidebar-footer {
    padding: 12px;
    border-top: 1px solid var(--border);
  }

  .update-banner {
    display: block;
    width: 100%;
    padding: 8px;
    margin-bottom: 8px;
    background: rgba(78, 204, 163, 0.15);
    color: var(--success);
    border-radius: 8px;
    font-size: 12px;
    font-weight: 500;
    text-align: center;
    transition: background 0.15s;
    cursor: pointer;
  }

  .update-banner:hover {
    background: rgba(78, 204, 163, 0.25);
  }

  .nav-btn {
    width: 100%;
    padding: 8px;
    border-radius: 8px;
    font-size: 13px;
    color: var(--text-secondary);
    transition: background 0.15s, color 0.15s;
    display: flex;
    align-items: center;
    gap: 8px;
  }

  .nav-btn:hover {
    background: var(--bg-tertiary);
    color: var(--text-primary);
  }

  .nav-btn.active {
    background: var(--bg-tertiary);
    color: var(--accent, #a78bfa);
  }

  /* Update Dialog */
  .update-overlay {
    position: fixed;
    top: 0;
    left: 0;
    right: 0;
    bottom: 0;
    background: rgba(0, 0, 0, 0.6);
    display: flex;
    align-items: center;
    justify-content: center;
    z-index: 1000;
  }

  .update-dialog {
    background: var(--bg-secondary);
    border: 1px solid var(--border);
    border-radius: 12px;
    width: 480px;
    max-width: 90vw;
    max-height: 80vh;
    display: flex;
    flex-direction: column;
    box-shadow: 0 8px 32px rgba(0, 0, 0, 0.3);
  }

  .update-dialog-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: 16px 20px;
    border-bottom: 1px solid var(--border);
  }

  .update-dialog-header h3 {
    font-size: 16px;
    font-weight: 600;
    margin: 0;
  }

  .update-close {
    font-size: 20px;
    color: var(--text-muted);
    padding: 0 4px;
    line-height: 1;
  }

  .update-close:hover {
    color: var(--text-primary);
  }

  .update-dialog-body {
    padding: 20px;
    overflow-y: auto;
    display: flex;
    flex-direction: column;
    gap: 16px;
  }

  .version-info {
    display: flex;
    align-items: center;
    gap: 10px;
    justify-content: center;
  }

  .version-badge {
    padding: 4px 12px;
    border-radius: 6px;
    font-size: 13px;
    font-weight: 600;
    font-family: "JetBrains Mono", "Fira Code", monospace;
  }

  .version-badge.old {
    background: var(--bg-tertiary);
    color: var(--text-muted);
  }

  .version-badge.new {
    background: rgba(78, 204, 163, 0.2);
    color: var(--success);
  }

  .version-arrow {
    color: var(--text-muted);
    font-size: 16px;
  }

  .release-notes {
    display: flex;
    flex-direction: column;
    gap: 8px;
  }

  .release-notes h4 {
    font-size: 13px;
    font-weight: 600;
    color: var(--text-secondary);
    margin: 0;
  }

  .release-notes-content {
    font-size: 13px;
    line-height: 1.6;
    color: var(--text-secondary);
    background: var(--bg-primary);
    padding: 12px;
    border-radius: 8px;
    max-height: 200px;
    overflow-y: auto;
    white-space: pre-wrap;
  }

  .progress-section {
    display: flex;
    flex-direction: column;
    gap: 8px;
  }

  .progress-label {
    font-size: 13px;
    color: var(--text-secondary);
  }

  .progress-bar {
    height: 6px;
    background: var(--bg-tertiary);
    border-radius: 3px;
    overflow: hidden;
  }

  .progress-fill {
    height: 100%;
    background: var(--accent);
    border-radius: 3px;
    transition: width 0.2s;
  }

  .update-success {
    padding: 12px;
    background: rgba(78, 204, 163, 0.15);
    color: var(--success);
    border-radius: 8px;
    font-size: 13px;
    font-weight: 500;
    text-align: center;
  }

  .update-error {
    padding: 12px;
    background: rgba(233, 69, 96, 0.1);
    color: var(--danger);
    border-radius: 8px;
    font-size: 13px;
  }

  .update-dialog-footer {
    padding: 16px 20px;
    border-top: 1px solid var(--border);
    display: flex;
    gap: 8px;
    flex-wrap: wrap;
  }

  .update-btn {
    padding: 8px 16px;
    border-radius: 8px;
    font-size: 13px;
    font-weight: 500;
    transition: background 0.15s, opacity 0.15s;
    text-decoration: none;
    text-align: center;
  }

  .update-btn.primary {
    background: var(--accent);
    color: white;
  }

  .update-btn.primary:hover {
    background: var(--accent-hover);
  }

  .update-btn.secondary {
    border: 1px solid var(--border);
    color: var(--text-secondary);
  }

  .update-btn.secondary:hover {
    background: var(--bg-tertiary);
  }

  .update-btn.ghost {
    color: var(--text-muted);
    margin-left: auto;
  }

  .update-btn.ghost:hover {
    color: var(--text-secondary);
  }
</style>
