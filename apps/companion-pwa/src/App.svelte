<script>
  import { loadConnection, saveConnection, clearConnection } from "./lib/connection.js";
  import Setup from "./views/Setup.svelte";
  import ConversationList from "./views/ConversationList.svelte";
  import ConversationView from "./views/ConversationView.svelte";

  let conn = $state(loadConnection());
  let activeConvId = $state(null);

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
  }
</script>

{#if !conn}
  <Setup {onConnect} />
{:else if activeConvId}
  <ConversationView {conn} conversationId={activeConvId} {onBack} />
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
  }

  @media (prefers-color-scheme: light) {
    :global(:root) {
      --bg: #f5f5f8;
      --surface: #ffffff;
      --border: #e0e0ea;
      --text: #1a1a2e;
      --muted: #8888a0;
      --accent: #6a4db0;
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
</style>
