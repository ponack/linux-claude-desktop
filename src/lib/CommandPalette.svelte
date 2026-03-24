<script>
  import { invoke } from "@tauri-apps/api/core";
  import { onMount, tick } from "svelte";

  let { onClose, onSelectConversation, onNewChat, onOpenSettings } = $props();

  let query = $state("");
  let conversations = $state([]);
  let selectedIndex = $state(0);
  let inputEl;

  const actions = [
    { id: "new-chat", label: "New Chat", shortcut: "Ctrl+N", icon: "plus", action: () => { onClose(); onNewChat(); } },
    { id: "settings", label: "Open Settings", shortcut: "Ctrl+,", icon: "gear", action: () => { onClose(); onOpenSettings(); } },
    { id: "focus-input", label: "Focus Chat Input", shortcut: "Ctrl+L", icon: "cursor", action: () => { onClose(); document.querySelector(".input-wrapper textarea")?.focus(); } },
    { id: "search", label: "Search Conversations", shortcut: "Ctrl+K", icon: "search", action: () => { onClose(); document.querySelector(".search-box input")?.focus(); } },
  ];

  let filteredItems = $derived.by(() => {
    const q = query.trim().toLowerCase();
    let items = [];

    if (!q) {
      // Show actions first, then recent conversations
      items = [
        ...actions.map(a => ({ type: "action", ...a })),
        ...conversations.slice(0, 10).map(c => ({ type: "conversation", id: c.id, label: c.title, subtitle: formatDate(c.updated_at) })),
      ];
    } else {
      // Fuzzy filter both actions and conversations
      const matchedActions = actions
        .filter(a => fuzzyMatch(a.label, q))
        .map(a => ({ type: "action", ...a, score: fuzzyScore(a.label, q) }));

      const matchedConvs = conversations
        .filter(c => fuzzyMatch(c.title, q))
        .map(c => ({ type: "conversation", id: c.id, label: c.title, subtitle: formatDate(c.updated_at), score: fuzzyScore(c.title, q) }));

      items = [...matchedActions, ...matchedConvs].sort((a, b) => b.score - a.score);
    }

    return items;
  });

  function fuzzyMatch(text, query) {
    const t = text.toLowerCase();
    let qi = 0;
    for (let i = 0; i < t.length && qi < query.length; i++) {
      if (t[i] === query[qi]) qi++;
    }
    return qi === query.length;
  }

  function fuzzyScore(text, query) {
    const t = text.toLowerCase();
    let score = 0;
    let qi = 0;
    let lastMatch = -1;
    for (let i = 0; i < t.length && qi < query.length; i++) {
      if (t[i] === query[qi]) {
        score += 10;
        // Bonus for consecutive matches
        if (lastMatch === i - 1) score += 5;
        // Bonus for matching at start
        if (i === 0) score += 15;
        // Bonus for matching after separator
        if (i > 0 && (t[i - 1] === " " || t[i - 1] === "-" || t[i - 1] === "_")) score += 10;
        lastMatch = i;
        qi++;
      }
    }
    return score;
  }

  function formatDate(dateStr) {
    try {
      const d = new Date(dateStr);
      const now = new Date();
      const diff = now - d;
      if (diff < 60000) return "just now";
      if (diff < 3600000) return `${Math.floor(diff / 60000)}m ago`;
      if (diff < 86400000) return `${Math.floor(diff / 3600000)}h ago`;
      if (diff < 604800000) return `${Math.floor(diff / 86400000)}d ago`;
      return d.toLocaleDateString();
    } catch {
      return "";
    }
  }

  $effect(() => {
    // Reset selection when query changes
    query;
    selectedIndex = 0;
  });

  onMount(async () => {
    try {
      conversations = await invoke("get_conversations");
    } catch (e) {
      console.error("Failed to load conversations:", e);
    }
    await tick();
    inputEl?.focus();
  });

  function handleKeydown(e) {
    if (e.key === "Escape") {
      e.preventDefault();
      onClose();
    } else if (e.key === "ArrowDown") {
      e.preventDefault();
      selectedIndex = Math.min(selectedIndex + 1, filteredItems.length - 1);
      scrollSelectedIntoView();
    } else if (e.key === "ArrowUp") {
      e.preventDefault();
      selectedIndex = Math.max(selectedIndex - 1, 0);
      scrollSelectedIntoView();
    } else if (e.key === "Enter") {
      e.preventDefault();
      selectItem(filteredItems[selectedIndex]);
    }
  }

  function scrollSelectedIntoView() {
    tick().then(() => {
      const el = document.querySelector(".palette-item.selected");
      el?.scrollIntoView({ block: "nearest" });
    });
  }

  function selectItem(item) {
    if (!item) return;
    if (item.type === "action") {
      item.action();
    } else if (item.type === "conversation") {
      onClose();
      onSelectConversation(item.id);
    }
  }
</script>

<div class="palette-overlay" onclick={onClose} onkeydown={handleKeydown} role="dialog" aria-label="Command palette">
  <div class="palette-container" onclick={(e) => e.stopPropagation()}>
    <div class="palette-input-wrapper">
      <svg class="palette-search-icon" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
        <circle cx="11" cy="11" r="8"/><line x1="21" y1="21" x2="16.65" y2="16.65"/>
      </svg>
      <input
        bind:this={inputEl}
        bind:value={query}
        placeholder="Search conversations and actions..."
        class="palette-input"
        aria-label="Command palette search"
      />
    </div>

    <div class="palette-results">
      {#if filteredItems.length === 0}
        <div class="palette-empty">No results found</div>
      {/if}

      {#each filteredItems as item, i (item.id || item.label + i)}
        <button
          class="palette-item"
          class:selected={i === selectedIndex}
          onclick={() => selectItem(item)}
          onmouseenter={() => (selectedIndex = i)}
          role="option"
          aria-selected={i === selectedIndex}
        >
          <div class="palette-item-left">
            {#if item.type === "action"}
              <span class="palette-icon action-icon">
                {#if item.icon === "plus"}+
                {:else if item.icon === "gear"}&#9881;
                {:else if item.icon === "cursor"}&#9998;
                {:else if item.icon === "search"}&#128269;
                {/if}
              </span>
            {:else}
              <span class="palette-icon conv-icon">
                <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
                  <path d="M21 15a2 2 0 01-2 2H7l-4 4V5a2 2 0 012-2h14a2 2 0 012 2z"/>
                </svg>
              </span>
            {/if}
            <div class="palette-item-text">
              <span class="palette-item-label">{item.label}</span>
              {#if item.subtitle}
                <span class="palette-item-subtitle">{item.subtitle}</span>
              {/if}
            </div>
          </div>
          {#if item.shortcut}
            <kbd class="palette-shortcut">{item.shortcut}</kbd>
          {/if}
        </button>
      {/each}
    </div>

    <div class="palette-footer">
      <span><kbd>&#8593;&#8595;</kbd> navigate</span>
      <span><kbd>&#9166;</kbd> select</span>
      <span><kbd>esc</kbd> close</span>
    </div>
  </div>
</div>

<style>
  .palette-overlay {
    position: fixed;
    top: 0;
    left: 0;
    right: 0;
    bottom: 0;
    background: rgba(0, 0, 0, 0.5);
    display: flex;
    justify-content: center;
    padding-top: 15vh;
    z-index: 2000;
  }

  .palette-container {
    width: 560px;
    max-width: 90vw;
    max-height: 60vh;
    background: var(--bg-secondary);
    border: 1px solid var(--border);
    border-radius: 12px;
    display: flex;
    flex-direction: column;
    box-shadow: 0 16px 48px rgba(0, 0, 0, 0.4);
    overflow: hidden;
    align-self: flex-start;
  }

  .palette-input-wrapper {
    display: flex;
    align-items: center;
    padding: 12px 16px;
    border-bottom: 1px solid var(--border);
    gap: 10px;
  }

  .palette-search-icon {
    color: var(--text-muted);
    flex-shrink: 0;
  }

  .palette-input {
    flex: 1;
    background: none;
    border: none;
    outline: none;
    font-size: 15px;
    color: var(--text-primary);
  }

  .palette-input::placeholder {
    color: var(--text-muted);
  }

  .palette-results {
    flex: 1;
    overflow-y: auto;
    padding: 6px;
  }

  .palette-empty {
    padding: 20px;
    text-align: center;
    color: var(--text-muted);
    font-size: 13px;
  }

  .palette-item {
    display: flex;
    align-items: center;
    justify-content: space-between;
    width: 100%;
    padding: 8px 12px;
    border-radius: 8px;
    text-align: left;
    transition: background 0.08s;
    gap: 8px;
  }

  .palette-item:hover,
  .palette-item.selected {
    background: var(--bg-tertiary);
  }

  .palette-item-left {
    display: flex;
    align-items: center;
    gap: 10px;
    min-width: 0;
    flex: 1;
  }

  .palette-icon {
    width: 28px;
    height: 28px;
    display: flex;
    align-items: center;
    justify-content: center;
    border-radius: 6px;
    flex-shrink: 0;
    font-size: 14px;
  }

  .action-icon {
    background: rgba(78, 204, 163, 0.15);
    color: var(--accent);
  }

  .conv-icon {
    background: var(--bg-tertiary);
    color: var(--text-muted);
  }

  .palette-item-text {
    display: flex;
    flex-direction: column;
    min-width: 0;
  }

  .palette-item-label {
    font-size: 13px;
    color: var(--text-primary);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .palette-item-subtitle {
    font-size: 11px;
    color: var(--text-muted);
  }

  .palette-shortcut {
    font-size: 11px;
    color: var(--text-muted);
    background: var(--bg-primary);
    padding: 2px 6px;
    border-radius: 4px;
    border: 1px solid var(--border);
    font-family: inherit;
    flex-shrink: 0;
  }

  .palette-footer {
    display: flex;
    gap: 16px;
    padding: 8px 16px;
    border-top: 1px solid var(--border);
    font-size: 11px;
    color: var(--text-muted);
  }

  .palette-footer kbd {
    font-size: 11px;
    background: var(--bg-primary);
    padding: 1px 4px;
    border-radius: 3px;
    border: 1px solid var(--border);
  }
</style>
