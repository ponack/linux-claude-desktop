<script>
  import { makeApi } from "../lib/api.js";

  let { onConnect } = $props();

  let url = $state("http://192.168.1.x:7432");
  let token = $state("");
  let testing = $state(false);
  let error = $state("");

  async function connect() {
    if (!url.trim() || !token.trim()) {
      error = "URL and token are required.";
      return;
    }
    testing = true;
    error = "";
    try {
      const api = makeApi({ url: url.trim(), token: token.trim() });
      await api.listConversations();
      onConnect({ url: url.trim(), token: token.trim() });
    } catch (e) {
      error = e.message || "Connection failed. Check the URL and token.";
    } finally {
      testing = false;
    }
  }
</script>

<div class="setup">
  <div class="setup-card">
    <div class="logo">
      <svg width="48" height="48" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5">
        <path d="M21 15a2 2 0 0 1-2 2H7l-4 4V5a2 2 0 0 1 2-2h14a2 2 0 0 1 2 2z"/>
      </svg>
    </div>
    <h1>LCD Companion</h1>
    <p class="subtitle">Connect to your Linux Claude Desktop</p>

    <div class="field">
      <label for="server-url">Desktop URL</label>
      <input
        id="server-url"
        type="url"
        bind:value={url}
        placeholder="http://192.168.1.x:7432"
        autocomplete="off"
        autocorrect="off"
        spellcheck="false"
      />
      <span class="hint">Enable API Server in Settings and enter your desktop's IP.</span>
    </div>

    <div class="field">
      <label for="token">Bearer Token</label>
      <input
        id="token"
        type="password"
        bind:value={token}
        placeholder="Paste token from Settings → API Server"
        autocomplete="off"
      />
    </div>

    {#if error}
      <div class="error">{error}</div>
    {/if}

    <button class="connect-btn" onclick={connect} disabled={testing}>
      {testing ? "Connecting…" : "Connect"}
    </button>
  </div>
</div>

<style>
  .setup {
    min-height: 100dvh;
    display: flex;
    align-items: center;
    justify-content: center;
    padding: 24px;
    background: var(--bg);
  }

  .setup-card {
    width: 100%;
    max-width: 400px;
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 16px;
  }

  .logo {
    color: var(--accent);
    margin-bottom: 4px;
  }

  h1 {
    font-size: 24px;
    font-weight: 700;
    margin: 0;
    color: var(--text);
  }

  .subtitle {
    font-size: 14px;
    color: var(--muted);
    margin: 0;
  }

  .field {
    width: 100%;
    display: flex;
    flex-direction: column;
    gap: 6px;
  }

  label {
    font-size: 13px;
    font-weight: 500;
    color: var(--text);
  }

  input {
    width: 100%;
    padding: 12px 14px;
    background: var(--surface);
    border: 1px solid var(--border);
    border-radius: 10px;
    color: var(--text);
    font-size: 15px;
    box-sizing: border-box;
    -webkit-appearance: none;
  }

  input:focus {
    outline: none;
    border-color: var(--accent);
  }

  .hint {
    font-size: 11px;
    color: var(--muted);
  }

  .error {
    width: 100%;
    padding: 10px 14px;
    background: rgba(233, 69, 96, 0.12);
    color: #e94560;
    border-radius: 8px;
    font-size: 13px;
  }

  .connect-btn {
    width: 100%;
    padding: 14px;
    background: var(--accent);
    color: #fff;
    border-radius: 12px;
    font-size: 16px;
    font-weight: 600;
    margin-top: 4px;
    cursor: pointer;
    transition: opacity 0.15s;
  }

  .connect-btn:disabled {
    opacity: 0.6;
    cursor: not-allowed;
  }

  .connect-btn:not(:disabled):active {
    opacity: 0.85;
  }
</style>
