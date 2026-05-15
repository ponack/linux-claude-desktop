<script>
  import { makeApi } from "../lib/api.js";

  let { conn, onSelect, onDisconnect } = $props();

  const api = $derived(makeApi(conn));

  let conversations = $state([]);
  let loading = $state(true);
  let error = $state("");
  let search = $state("");
  let creating = $state(false);

  let filtered = $derived(
    search.trim()
      ? conversations.filter((c) =>
          c.title.toLowerCase().includes(search.trim().toLowerCase())
        )
      : conversations
  );

  async function load() {
    loading = true;
    error = "";
    try {
      conversations = await api.listConversations();
    } catch (e) {
      error = e.message;
    } finally {
      loading = false;
    }
  }

  async function newConversation() {
    creating = true;
    try {
      const conv = await api.createConversation("New Conversation");
      await load();
      onSelect(conv.id);
    } catch (e) {
      error = e.message;
    } finally {
      creating = false;
    }
  }

  function formatDate(iso) {
    const d = new Date(iso);
    const now = new Date();
    const diff = (now - d) / 1000;
    if (diff < 60) return "just now";
    if (diff < 3600) return `${Math.floor(diff / 60)}m ago`;
    if (diff < 86400) return `${Math.floor(diff / 3600)}h ago`;
    return d.toLocaleDateString(undefined, { month: "short", day: "numeric" });
  }

  $effect(() => {
    load();
  });
</script>

<div class="screen">
  <header class="topbar">
    <h2>Conversations</h2>
    <div class="topbar-actions">
      <button class="icon-btn" onclick={load} title="Refresh" aria-label="Refresh">
        <svg width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
          <polyline points="23 4 23 10 17 10"/><path d="M20.49 15a9 9 0 1 1-2.12-9.36L23 10"/>
        </svg>
      </button>
      <button class="icon-btn" onclick={onDisconnect} title="Disconnect" aria-label="Disconnect">
        <svg width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
          <path d="M9 21H5a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2h4"/><polyline points="16 17 21 12 16 7"/><line x1="21" y1="12" x2="9" y2="12"/>
        </svg>
      </button>
    </div>
  </header>

  <div class="search-wrap">
    <svg class="search-icon" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
      <circle cx="11" cy="11" r="8"/><line x1="21" y1="21" x2="16.65" y2="16.65"/>
    </svg>
    <input
      type="search"
      bind:value={search}
      placeholder="Search conversations…"
      aria-label="Search conversations"
    />
  </div>

  <div class="list">
    {#if loading}
      <div class="state-msg">Loading…</div>
    {:else if error}
      <div class="state-msg error">{error}</div>
    {:else if filtered.length === 0}
      <div class="state-msg">No conversations yet.</div>
    {:else}
      {#each filtered as conv (conv.id)}
        <button class="conv-item" onclick={() => onSelect(conv.id)}>
          <span class="conv-title">{conv.title}</span>
          <span class="conv-date">{formatDate(conv.updated_at)}</span>
        </button>
      {/each}
    {/if}
  </div>

  <button class="fab" onclick={newConversation} disabled={creating} aria-label="New conversation">
    <svg width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5">
      <line x1="12" y1="5" x2="12" y2="19"/><line x1="5" y1="12" x2="19" y2="12"/>
    </svg>
  </button>
</div>

<style>
  .screen {
    display: flex;
    flex-direction: column;
    height: 100dvh;
    background: var(--bg);
    position: relative;
  }

  .topbar {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 16px 16px 0;
    padding-top: max(16px, env(safe-area-inset-top));
  }

  .topbar h2 {
    font-size: 22px;
    font-weight: 700;
    margin: 0;
    color: var(--text);
  }

  .topbar-actions {
    display: flex;
    gap: 4px;
  }

  .icon-btn {
    width: 40px;
    height: 40px;
    border-radius: 10px;
    display: flex;
    align-items: center;
    justify-content: center;
    color: var(--muted);
    transition: background 0.15s, color 0.15s;
  }

  .icon-btn:active {
    background: var(--surface);
    color: var(--text);
  }

  .search-wrap {
    position: relative;
    margin: 12px 16px 0;
  }

  .search-icon {
    position: absolute;
    left: 12px;
    top: 50%;
    transform: translateY(-50%);
    color: var(--muted);
    pointer-events: none;
  }

  input[type="search"] {
    width: 100%;
    padding: 10px 12px 10px 36px;
    background: var(--surface);
    border: 1px solid var(--border);
    border-radius: 10px;
    color: var(--text);
    font-size: 15px;
    box-sizing: border-box;
    -webkit-appearance: none;
  }

  input[type="search"]:focus {
    outline: none;
    border-color: var(--accent);
  }

  .list {
    flex: 1;
    overflow-y: auto;
    padding: 8px 0 80px;
    -webkit-overflow-scrolling: touch;
  }

  .state-msg {
    padding: 32px 16px;
    text-align: center;
    color: var(--muted);
    font-size: 14px;
  }

  .state-msg.error {
    color: #e94560;
  }

  .conv-item {
    display: flex;
    align-items: baseline;
    justify-content: space-between;
    gap: 12px;
    width: 100%;
    padding: 14px 16px;
    text-align: left;
    border-bottom: 1px solid var(--border);
    transition: background 0.1s;
  }

  .conv-item:active {
    background: var(--surface);
  }

  .conv-title {
    flex: 1;
    font-size: 15px;
    font-weight: 500;
    color: var(--text);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .conv-date {
    font-size: 12px;
    color: var(--muted);
    flex-shrink: 0;
  }

  .fab {
    position: fixed;
    bottom: max(24px, env(safe-area-inset-bottom));
    right: 20px;
    width: 56px;
    height: 56px;
    border-radius: 50%;
    background: var(--accent);
    color: #fff;
    display: flex;
    align-items: center;
    justify-content: center;
    box-shadow: 0 4px 16px rgba(0, 0, 0, 0.3);
    transition: opacity 0.15s, transform 0.15s;
  }

  .fab:active {
    opacity: 0.85;
    transform: scale(0.95);
  }

  .fab:disabled {
    opacity: 0.5;
  }
</style>
