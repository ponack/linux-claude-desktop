<script>
  import { invoke } from "@tauri-apps/api/core";
  import { onMount } from "svelte";

  let { onClose } = $props();

  let apiKey = $state("");
  let model = $state("claude-sonnet-4-6");
  let systemPrompt = $state("");
  let theme = $state("dark");
  let saved = $state(false);
  let error = $state("");

  const models = [
    { id: "claude-opus-4-6", name: "Claude Opus 4.6 (Most capable)" },
    { id: "claude-sonnet-4-6", name: "Claude Sonnet 4.6 (Balanced)" },
    { id: "claude-haiku-4-5-20251001", name: "Claude Haiku 4.5 (Fast)" },
  ];

  onMount(async () => {
    try {
      const key = await invoke("get_api_key");
      if (key) apiKey = key;
      model = await invoke("get_model");
      const sp = await invoke("get_system_prompt");
      if (sp) systemPrompt = sp;
      theme = await invoke("get_theme");
    } catch (e) {
      console.error("Failed to load settings:", e);
    }
  });

  async function save() {
    error = "";
    saved = false;
    try {
      await invoke("set_api_key", { key: apiKey });
      await invoke("set_model", { model });
      await invoke("set_system_prompt", { prompt: systemPrompt });
      await invoke("set_theme", { theme });
      document.documentElement.setAttribute("data-theme", theme);
      saved = true;
      setTimeout(() => (saved = false), 2000);
    } catch (e) {
      error = String(e);
    }
  }
</script>

<div class="settings">
  <div class="settings-header">
    <h2>Settings</h2>
    <button class="close-btn" onclick={onClose}>Back to Chat</button>
  </div>

  <div class="settings-body">
    <div class="setting-group">
      <label for="api-key">Anthropic API Key</label>
      <input
        id="api-key"
        type="password"
        bind:value={apiKey}
        placeholder="sk-ant-..."
      />
      <p class="hint">
        Get your API key from
        <span class="link">console.anthropic.com</span>
      </p>
    </div>

    <div class="setting-group">
      <label for="model">Model</label>
      <select id="model" bind:value={model}>
        {#each models as m}
          <option value={m.id}>{m.name}</option>
        {/each}
      </select>
    </div>

    <div class="setting-group">
      <label for="theme">Theme</label>
      <select id="theme" bind:value={theme}>
        <option value="dark">Dark</option>
        <option value="light">Light</option>
      </select>
    </div>

    <div class="setting-group">
      <label for="system-prompt">System Prompt</label>
      <textarea
        id="system-prompt"
        bind:value={systemPrompt}
        placeholder="You are a helpful assistant..."
        rows="4"
      ></textarea>
      <p class="hint">
        Optional. Sets the system prompt for all new conversations.
      </p>
    </div>

    {#if error}
      <p class="error">{error}</p>
    {/if}

    <button class="save-btn" onclick={save}>
      {saved ? "Saved!" : "Save Settings"}
    </button>
  </div>
</div>

<style>
  .settings {
    padding: 24px;
    max-width: 600px;
    margin: 0 auto;
    width: 100%;
  }

  .settings-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    margin-bottom: 32px;
  }

  h2 {
    font-size: 20px;
    font-weight: 600;
  }

  .close-btn {
    padding: 6px 14px;
    border-radius: 8px;
    font-size: 13px;
    color: var(--text-secondary);
    border: 1px solid var(--border);
    transition: background 0.15s;
  }

  .close-btn:hover {
    background: var(--bg-tertiary);
  }

  .settings-body {
    display: flex;
    flex-direction: column;
    gap: 24px;
  }

  .setting-group {
    display: flex;
    flex-direction: column;
    gap: 6px;
  }

  label {
    font-size: 13px;
    font-weight: 500;
    color: var(--text-secondary);
  }

  input, select {
    padding: 10px 12px;
    background: var(--bg-input);
    border: 1px solid var(--border);
    border-radius: 8px;
    font-size: 14px;
    outline: none;
    transition: border-color 0.15s;
  }

  textarea {
    padding: 10px 12px;
    background: var(--bg-input);
    border: 1px solid var(--border);
    border-radius: 8px;
    font-size: 14px;
    outline: none;
    resize: vertical;
    min-height: 80px;
    line-height: 1.5;
    color: var(--text-primary);
    transition: border-color 0.15s;
  }

  input:focus, select:focus, textarea:focus {
    border-color: var(--accent);
  }

  select {
    cursor: pointer;
  }

  option {
    background: var(--bg-secondary);
  }

  .hint {
    font-size: 12px;
    color: var(--text-muted);
  }

  .link {
    color: var(--accent);
  }

  .error {
    color: var(--danger);
    font-size: 13px;
  }

  .save-btn {
    padding: 10px 20px;
    background: var(--accent);
    color: white;
    border-radius: 8px;
    font-size: 14px;
    font-weight: 500;
    transition: background 0.15s;
    align-self: flex-start;
  }

  .save-btn:hover {
    background: var(--accent-hover);
  }
</style>
