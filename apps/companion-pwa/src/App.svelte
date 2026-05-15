<script>
  import { onMount } from "svelte";
  import { loadConnection, saveConnection, clearConnection } from "./lib/connection.js";
  import { getAll, remove } from "./lib/queue.js";
  import { makeApi } from "./lib/api.js";
  import Setup from "./views/Setup.svelte";
  import ConversationList from "./views/ConversationList.svelte";
  import ConversationView from "./views/ConversationView.svelte";

  let conn = $state(loadConnection());
  let activeConvId = $state(null);
  let isOnline = $state(navigator.onLine);

  // Callback registered by ConversationView so queue flush can reload its messages
  let onQueueFlushed = $state(null);

  function onConnect(newConn) {
    saveConnection(newConn);
    conn = newConn;
    activeConvId = null;
  }

  function onDisconnect() {
    clearConnection();
    conn = null;
    activeConvId = null;
  }

  function onSelect(id) {
    activeConvId = id;
  }

  function onBack() {
    activeConvId = null;
    onQueueFlushed = null;
  }

  async function flushQueue() {
    if (!conn) return;
    const api = makeApi(conn);
    const pending = getAll();
    for (const item of pending) {
      try {
        await api.sendMessage(item.conversationId, item.content);
        remove(item.id);
      } catch {
        break; // still offline or error — stop, leave remaining in queue
      }
    }
    onQueueFlushed?.();
  }

  onMount(() => {
    const goOnline = () => {
      isOnline = true;
      flushQueue();
    };
    const goOffline = () => { isOnline = false; };

    window.addEventListener("online", goOnline);
    window.addEventListener("offline", goOffline);

    // Listen for Background Sync messages from the service worker
    if ("serviceWorker" in navigator) {
      navigator.serviceWorker.addEventListener("message", (e) => {
        if (e.data?.type === "flush-queue") flushQueue();
      });
    }

    return () => {
      window.removeEventListener("online", goOnline);
      window.removeEventListener("offline", goOffline);
    };
  });
</script>

{#if !isOnline}
  <div class="offline-banner" role="status">
    Offline — messages will send when reconnected
  </div>
{/if}

{#if !conn}
  <Setup {onConnect} />
{:else if activeConvId}
  <ConversationView
    {conn}
    conversationId={activeConvId}
    {isOnline}
    {onBack}
    onRegisterFlushCallback={(cb) => { onQueueFlushed = cb; }}
  />
{:else}
  <ConversationList {conn} {onSelect} {onDisconnect} />
{/if}

<style>
  :global(*) {
    box-sizing: border-box;
    margin: 0;
    padding: 0;
    -webkit-tap-highlight-color: transparent;
  }

  :global(:root) {
    --bg: #0f0f17;
    --surface: #1a1a2e;
    --border: #2a2a40;
    --text: #e0e0f0;
    --muted: #6b6b8a;
    --accent: #7c5cbf;
    --warning: #e8a838;
    --offline-bg: #2a1a00;
  }

  @media (prefers-color-scheme: light) {
    :global(:root) {
      --bg: #f5f5f8;
      --surface: #ffffff;
      --border: #e0e0ea;
      --text: #1a1a2e;
      --muted: #8888a0;
      --accent: #6a4db0;
      --offline-bg: #fff3cd;
    }
  }

  :global(body) {
    background: var(--bg);
    color: var(--text);
    font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, sans-serif;
    -webkit-font-smoothing: antialiased;
    overscroll-behavior: none;
  }

  :global(button) {
    background: none;
    border: none;
    cursor: pointer;
    font-family: inherit;
    padding: 0;
  }

  :global(input), :global(textarea) {
    font-family: inherit;
  }

  :global(a) {
    color: var(--accent);
    text-decoration: none;
  }

  .offline-banner {
    position: fixed;
    top: 0;
    left: 0;
    right: 0;
    z-index: 999;
    background: var(--offline-bg);
    color: var(--warning);
    text-align: center;
    font-size: 13px;
    font-weight: 500;
    padding: 6px 16px;
    padding-top: max(6px, env(safe-area-inset-top));
  }
</style>
