<script>
  import { invoke } from "@tauri-apps/api/core";
  import { onMount } from "svelte";

  let { activeConversationId, onSelect, onNewChat, openSettings, refreshKey } = $props();

  let conversations = $state([]);
  let searchQuery = $state("");

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
  });

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

<aside class="sidebar">
  <div class="sidebar-header">
    <div class="sidebar-brand">
      <img src="/assets/logo.svg" alt="UCD" class="sidebar-logo" />
      <span class="sidebar-title">UCD</span>
    </div>
    <button class="new-chat-btn" onclick={onNewChat}>
      + New Chat
    </button>
  </div>

  <div class="search-box">
    <input
      type="text"
      bind:value={searchQuery}
      placeholder="Search conversations..."
    />
  </div>

  <div class="conversations-list">
    {#each filteredConversations as conv (conv.id)}
      <div
        class="conversation-item"
        class:active={activeConversationId === conv.id}
        role="button"
        tabindex="0"
        onclick={() => onSelect(conv.id)}
        onkeydown={(e) => e.key === 'Enter' && onSelect(conv.id)}
      >
        <span class="conv-title">{conv.title}</span>
        <button class="delete-btn" onclick={(e) => handleDelete(e, conv.id)}>
          ×
        </button>
      </div>
    {/each}
  </div>

  <div class="sidebar-footer">
    <button class="settings-btn" onclick={openSettings}>
      Settings
    </button>
  </div>
</aside>

<style>
  .sidebar {
    width: 260px;
    min-width: 260px;
    background: var(--bg-secondary);
    border-right: 1px solid var(--border);
    display: flex;
    flex-direction: column;
    height: 100%;
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

  .settings-btn {
    width: 100%;
    padding: 8px;
    border-radius: 8px;
    font-size: 13px;
    color: var(--text-secondary);
    transition: background 0.15s;
  }

  .settings-btn:hover {
    background: var(--bg-tertiary);
  }
</style>
