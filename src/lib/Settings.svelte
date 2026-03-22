<script>
  import { invoke } from "@tauri-apps/api/core";
  import { onMount } from "svelte";

  let { onClose } = $props();

  // Provider settings
  let provider = $state("anthropic");
  let apiKey = $state("");
  let openaiApiKey = $state("");
  let openaiBaseUrl = $state("https://api.openai.com");
  let ollamaBaseUrl = $state("http://localhost:11434");
  let model = $state("claude-sonnet-4-6");
  let ollamaModels = $state([]);
  let fetchingOllamaModels = $state(false);

  // General settings
  let systemPrompt = $state("");
  let theme = $state("dark");
  let customCss = $state("");
  let saved = $state(false);
  let error = $state("");

  // Projects
  let projects = $state([]);
  let newProjectName = $state("");
  let newProjectContext = $state("");
  let editingProject = $state(null);

  // MCP Servers
  let mcpServers = $state([]);
  let newMcpName = $state("");
  let newMcpCommand = $state("");
  let newMcpArgs = $state("");

  // Prompt Library
  let prompts = $state([]);
  let newPromptName = $state("");
  let newPromptContent = $state("");
  let editingPrompt = $state(null);

  // Custom Commands (Plugin System)
  let customCommands = $state([]);
  let newCmdName = $state("");
  let newCmdCommand = $state("");
  let newCmdDescription = $state("");

  // Update settings
  let updateInterval = $state("86400000");

  // About
  let appVersion = $state("");
  let appArch = $state("");
  let appOs = $state("");

  const anthropicModels = [
    { id: "claude-opus-4-6", name: "Claude Opus 4.6 (Most capable)" },
    { id: "claude-sonnet-4-6", name: "Claude Sonnet 4.6 (Balanced)" },
    { id: "claude-haiku-4-5-20251001", name: "Claude Haiku 4.5 (Fast)" },
  ];

  const openaiModels = [
    { id: "gpt-4o", name: "GPT-4o (Most capable)" },
    { id: "gpt-4o-mini", name: "GPT-4o Mini (Fast)" },
    { id: "o1", name: "o1 (Reasoning)" },
    { id: "o3-mini", name: "o3-mini (Reasoning, fast)" },
  ];

  let availableModels = $derived(
    provider === "anthropic" ? anthropicModels :
    provider === "openai" ? openaiModels :
    ollamaModels.map(m => ({ id: m, name: m }))
  );

  onMount(async () => {
    try {
      provider = await invoke("get_provider");
      const key = await invoke("get_api_key");
      if (key) apiKey = key;
      const oaiKey = await invoke("get_openai_api_key");
      if (oaiKey) openaiApiKey = oaiKey;
      openaiBaseUrl = await invoke("get_openai_base_url");
      ollamaBaseUrl = await invoke("get_ollama_base_url");
      model = await invoke("get_model");
      const sp = await invoke("get_system_prompt");
      if (sp) systemPrompt = sp;
      theme = await invoke("get_theme");
      customCss = await invoke("get_custom_css");
      await loadProjects();
      await loadMcpServers();
      await loadPrompts();
      await loadCustomCommands();
      updateInterval = await invoke("get_update_interval");

      try {
        const info = await invoke("get_app_info");
        appVersion = info.version;
        appArch = info.arch;
        appOs = info.os;
      } catch (_) {}

      if (provider === "ollama") {
        fetchOllamaModels();
      }
    } catch (e) {
      console.error("Failed to load settings:", e);
    }
  });

  async function fetchOllamaModels() {
    fetchingOllamaModels = true;
    try {
      ollamaModels = await invoke("fetch_ollama_models", { baseUrl: ollamaBaseUrl });
    } catch (e) {
      console.error("Failed to fetch Ollama models:", e);
      ollamaModels = [];
    }
    fetchingOllamaModels = false;
  }

  async function handleProviderChange() {
    if (provider === "ollama") {
      await fetchOllamaModels();
      if (ollamaModels.length > 0 && !ollamaModels.includes(model)) {
        model = ollamaModels[0];
      }
    } else if (provider === "anthropic") {
      if (!anthropicModels.find(m => m.id === model)) {
        model = "claude-sonnet-4-6";
      }
    } else if (provider === "openai") {
      if (!openaiModels.find(m => m.id === model)) {
        model = "gpt-4o";
      }
    }
  }

  async function loadMcpServers() {
    try { mcpServers = await invoke("get_mcp_servers"); }
    catch (e) { mcpServers = []; }
  }

  async function addMcpServer() {
    if (!newMcpName.trim() || !newMcpCommand.trim()) return;
    mcpServers = [...mcpServers, {
      name: newMcpName.trim(),
      command: newMcpCommand.trim(),
      args: newMcpArgs.trim() ? newMcpArgs.trim().split(/\s+/) : [],
      env: {},
    }];
    await saveMcpServers();
    newMcpName = ""; newMcpCommand = ""; newMcpArgs = "";
  }

  async function removeMcpServer(index) {
    mcpServers = mcpServers.filter((_, i) => i !== index);
    await saveMcpServers();
  }

  async function saveMcpServers() {
    try { await invoke("set_mcp_servers", { servers: mcpServers }); }
    catch (e) { error = String(e); }
  }

  async function loadProjects() {
    try { projects = await invoke("get_projects"); }
    catch (e) { console.error("Failed to load projects:", e); }
  }

  async function addProject() {
    if (!newProjectName.trim()) return;
    try {
      await invoke("create_project", { name: newProjectName.trim(), context: newProjectContext.trim() });
      newProjectName = ""; newProjectContext = "";
      await loadProjects();
    } catch (e) { error = String(e); }
  }

  async function saveProject(project) {
    try {
      await invoke("update_project", { id: project.id, name: project.name, context: project.context });
      editingProject = null;
      await loadProjects();
    } catch (e) { error = String(e); }
  }

  async function removeProject(id) {
    try { await invoke("delete_project", { id }); await loadProjects(); }
    catch (e) { error = String(e); }
  }

  async function loadPrompts() {
    try { prompts = await invoke("get_prompts"); }
    catch (e) { console.error("Failed to load prompts:", e); }
  }

  async function addPrompt() {
    if (!newPromptName.trim() || !newPromptContent.trim()) return;
    try {
      await invoke("create_prompt", { name: newPromptName.trim(), content: newPromptContent.trim() });
      newPromptName = ""; newPromptContent = "";
      await loadPrompts();
    } catch (e) { error = String(e); }
  }

  async function savePrompt(prompt) {
    try {
      await invoke("update_prompt", { id: prompt.id, name: prompt.name, content: prompt.content });
      editingPrompt = null;
      await loadPrompts();
    } catch (e) { error = String(e); }
  }

  async function removePrompt(id) {
    try { await invoke("delete_prompt", { id }); await loadPrompts(); }
    catch (e) { error = String(e); }
  }

  async function loadCustomCommands() {
    try { customCommands = await invoke("get_custom_commands"); }
    catch (e) { customCommands = []; }
  }

  async function addCustomCommand() {
    if (!newCmdName.trim() || !newCmdCommand.trim()) return;
    customCommands = [...customCommands, {
      name: newCmdName.trim(),
      command: newCmdCommand.trim(),
      description: newCmdDescription.trim(),
    }];
    await saveCustomCommands();
    newCmdName = ""; newCmdCommand = ""; newCmdDescription = "";
  }

  async function removeCustomCommand(index) {
    customCommands = customCommands.filter((_, i) => i !== index);
    await saveCustomCommands();
  }

  async function saveCustomCommands() {
    try { await invoke("set_custom_commands", { commands: customCommands }); }
    catch (e) { error = String(e); }
  }

  function validateSettings() {
    if (provider === "anthropic" && !apiKey.trim()) {
      return "Anthropic API key is required. Get one at console.anthropic.com";
    }
    if (provider === "anthropic" && !apiKey.trim().startsWith("sk-ant-")) {
      return "Anthropic API key should start with 'sk-ant-'";
    }
    if (provider === "openai" && !openaiApiKey.trim()) {
      return "OpenAI API key is required.";
    }
    if (provider === "openai" && openaiBaseUrl.trim() && !openaiBaseUrl.trim().startsWith("http")) {
      return "OpenAI base URL must start with http:// or https://";
    }
    if (provider === "ollama" && ollamaBaseUrl.trim() && !ollamaBaseUrl.trim().startsWith("http")) {
      return "Ollama URL must start with http:// or https://";
    }
    return null;
  }

  async function save() {
    error = "";
    saved = false;

    const validationError = validateSettings();
    if (validationError) {
      error = validationError;
      return;
    }

    try {
      await invoke("set_provider", { provider });
      await invoke("set_api_key", { key: apiKey });
      await invoke("set_openai_api_key", { key: openaiApiKey });
      await invoke("set_openai_base_url", { url: openaiBaseUrl });
      await invoke("set_ollama_base_url", { url: ollamaBaseUrl });
      await invoke("set_model", { model });
      await invoke("set_system_prompt", { prompt: systemPrompt });
      await invoke("set_theme", { theme });
      await invoke("set_custom_css", { css: customCss });
      await invoke("set_update_interval", { interval: updateInterval });
      document.documentElement.setAttribute("data-theme", theme);

      let styleEl = document.getElementById("custom-css");
      if (!styleEl) {
        styleEl = document.createElement("style");
        styleEl.id = "custom-css";
        document.head.appendChild(styleEl);
      }
      styleEl.textContent = customCss;

      saved = true;
      setTimeout(() => (saved = false), 2000);
    } catch (e) {
      error = String(e);
    }
  }

  const themePresets = [
    { name: "None", css: "" },
    { name: "Nord", css: `:root, [data-theme="dark"] {
  --bg-primary: #2e3440;
  --bg-secondary: #3b4252;
  --bg-tertiary: #434c5e;
  --bg-input: #2e3440;
  --text-primary: #eceff4;
  --text-secondary: #d8dee9;
  --text-muted: #7b88a1;
  --accent: #88c0d0;
  --accent-hover: #8fbcbb;
  --border: #4c566a;
  --user-bubble: #434c5e;
  --assistant-bubble: #2e3440;
  --danger: #bf616a;
  --success: #a3be8c;
}` },
    { name: "Solarized", css: `:root, [data-theme="dark"] {
  --bg-primary: #002b36;
  --bg-secondary: #073642;
  --bg-tertiary: #094858;
  --bg-input: #002b36;
  --text-primary: #fdf6e3;
  --text-secondary: #eee8d5;
  --text-muted: #657b83;
  --accent: #268bd2;
  --accent-hover: #2aa198;
  --border: #094858;
  --user-bubble: #094858;
  --assistant-bubble: #002b36;
  --danger: #dc322f;
  --success: #859900;
}` },
    { name: "Monokai", css: `:root, [data-theme="dark"] {
  --bg-primary: #272822;
  --bg-secondary: #1e1f1a;
  --bg-tertiary: #3e3d32;
  --bg-input: #272822;
  --text-primary: #f8f8f2;
  --text-secondary: #cfcfc2;
  --text-muted: #75715e;
  --accent: #a6e22e;
  --accent-hover: #b6f23e;
  --border: #3e3d32;
  --user-bubble: #3e3d32;
  --assistant-bubble: #272822;
  --danger: #f92672;
  --success: #a6e22e;
}` },
    { name: "Dracula", css: `:root, [data-theme="dark"] {
  --bg-primary: #282a36;
  --bg-secondary: #21222c;
  --bg-tertiary: #44475a;
  --bg-input: #282a36;
  --text-primary: #f8f8f2;
  --text-secondary: #d4d4dc;
  --text-muted: #6272a4;
  --accent: #bd93f9;
  --accent-hover: #caa4fa;
  --border: #44475a;
  --user-bubble: #44475a;
  --assistant-bubble: #282a36;
  --danger: #ff5555;
  --success: #50fa7b;
}` },
  ];

  function applyPreset(preset) {
    customCss = preset.css;
  }
</script>

<div class="settings">
  <div class="settings-header">
    <h2>Settings</h2>
    <button class="close-btn" onclick={onClose} aria-label="Back to chat">Back to Chat</button>
  </div>

  <div class="settings-body">
    <div class="setting-group">
      <label for="provider">Provider</label>
      <select id="provider" bind:value={provider} onchange={handleProviderChange}>
        <option value="anthropic">Anthropic (Claude)</option>
        <option value="openai">OpenAI</option>
        <option value="ollama">Ollama (Local)</option>
      </select>
    </div>

    {#if provider === "anthropic"}
      <div class="setting-group">
        <label for="api-key">Anthropic API Key</label>
        <input id="api-key" type="password" bind:value={apiKey} placeholder="sk-ant-..." />
        <p class="hint">Get your API key from <span class="link">console.anthropic.com</span></p>
      </div>
    {/if}

    {#if provider === "openai"}
      <div class="setting-group">
        <label for="openai-key">OpenAI API Key</label>
        <input id="openai-key" type="password" bind:value={openaiApiKey} placeholder="sk-..." />
      </div>
      <div class="setting-group">
        <label for="openai-url">Base URL</label>
        <input id="openai-url" type="text" bind:value={openaiBaseUrl} placeholder="https://api.openai.com" />
        <p class="hint">Change for OpenAI-compatible APIs (e.g. Together, Groq, Azure)</p>
      </div>
    {/if}

    {#if provider === "ollama"}
      <div class="setting-group">
        <label for="ollama-url">Ollama URL</label>
        <div class="input-row">
          <input id="ollama-url" type="text" bind:value={ollamaBaseUrl} placeholder="http://localhost:11434" />
          <button class="small-btn accent" onclick={fetchOllamaModels} disabled={fetchingOllamaModels}>
            {fetchingOllamaModels ? "..." : "Refresh"}
          </button>
        </div>
        <p class="hint">
          {ollamaModels.length > 0
            ? `Found ${ollamaModels.length} model(s)`
            : "Make sure Ollama is running locally"}
        </p>
      </div>
    {/if}

    <div class="setting-group">
      <label for="model">Model</label>
      {#if provider === "ollama" && ollamaModels.length === 0}
        <input id="model" type="text" bind:value={model} placeholder="e.g. llama3.2" />
      {:else}
        <select id="model" bind:value={model}>
          {#each availableModels as m}
            <option value={m.id}>{m.name}</option>
          {/each}
        </select>
      {/if}
    </div>

    <div class="setting-group">
      <label for="theme">Theme</label>
      <select id="theme" bind:value={theme}>
        <option value="dark">Dark</option>
        <option value="light">Light</option>
      </select>
    </div>

    <div class="setting-group">
      <label for="update-interval">Check for Updates</label>
      <select id="update-interval" bind:value={updateInterval} onchange={() => invoke("set_update_interval", { interval: updateInterval })}>
        <option value="never">Never</option>
        <option value="startup">On startup only</option>
        <option value="1800000">Every 30 minutes</option>
        <option value="3600000">Every hour</option>
        <option value="86400000">Once a day</option>
      </select>
      <p class="hint">How often to check GitHub for new releases.</p>
    </div>

    <div class="setting-group">
      <label>Custom Theme (CSS)</label>
      <div class="preset-row">
        {#each themePresets as preset}
          <button
            class="preset-btn"
            class:active={customCss === preset.css}
            onclick={() => applyPreset(preset)}
          >{preset.name}</button>
        {/each}
      </div>
      <textarea
        bind:value={customCss}
        placeholder={"/* Override CSS variables here */\n:root, [data-theme='dark'] {\n  --accent: #ff6b6b;\n}"}
        rows="4"
        class="code-textarea"
      ></textarea>
      <p class="hint">Override theme variables or add custom styles. Presets give you a starting point.</p>
    </div>

    <div class="setting-group">
      <label for="system-prompt">System Prompt</label>
      <textarea id="system-prompt" bind:value={systemPrompt} placeholder="You are a helpful assistant..." rows="4"></textarea>
      <p class="hint">Optional. Sets the system prompt for all new conversations.</p>
    </div>

    <div class="setting-group">
      <label>Prompt Library</label>
      <p class="hint">Save reusable prompts. Insert them into chat with the bookmark button.</p>

      {#each prompts as prompt (prompt.id)}
        <div class="project-item">
          {#if editingPrompt === prompt.id}
            <input type="text" bind:value={prompt.name} placeholder="Prompt name" />
            <textarea bind:value={prompt.content} placeholder="Prompt content..." rows="3"></textarea>
            <div class="project-actions">
              <button class="small-btn accent" onclick={() => savePrompt(prompt)}>Save</button>
              <button class="small-btn" onclick={() => (editingPrompt = null)}>Cancel</button>
            </div>
          {:else}
            <div class="project-header">
              <span class="project-name">{prompt.name}</span>
              <div class="project-actions">
                <button class="small-btn" onclick={() => (editingPrompt = prompt.id)}>Edit</button>
                <button class="small-btn danger" onclick={() => removePrompt(prompt.id)}>Delete</button>
              </div>
            </div>
            <p class="project-context-preview">{prompt.content.length > 100 ? prompt.content.slice(0, 100) + '...' : prompt.content}</p>
          {/if}
        </div>
      {/each}

      <div class="new-project">
        <input type="text" bind:value={newPromptName} placeholder="Prompt name" />
        <textarea bind:value={newPromptContent} placeholder="Prompt content..." rows="2"></textarea>
        <button class="small-btn accent" onclick={addPrompt} disabled={!newPromptName.trim() || !newPromptContent.trim()}>
          Add Prompt
        </button>
      </div>
    </div>

    <div class="setting-group">
      <label>Projects</label>
      <p class="hint">Projects inject persistent context into conversations assigned to them.</p>

      {#each projects as project (project.id)}
        <div class="project-item">
          {#if editingProject === project.id}
            <input type="text" bind:value={project.name} placeholder="Project name" />
            <textarea bind:value={project.context} placeholder="Project context/instructions..." rows="3"></textarea>
            <div class="project-actions">
              <button class="small-btn accent" onclick={() => saveProject(project)}>Save</button>
              <button class="small-btn" onclick={() => (editingProject = null)}>Cancel</button>
            </div>
          {:else}
            <div class="project-header">
              <span class="project-name">{project.name}</span>
              <div class="project-actions">
                <button class="small-btn" onclick={() => (editingProject = project.id)}>Edit</button>
                <button class="small-btn danger" onclick={() => removeProject(project.id)}>Delete</button>
              </div>
            </div>
            {#if project.context}
              <p class="project-context-preview">{project.context.length > 100 ? project.context.slice(0, 100) + '...' : project.context}</p>
            {/if}
          {/if}
        </div>
      {/each}

      <div class="new-project">
        <input type="text" bind:value={newProjectName} placeholder="New project name" />
        <textarea bind:value={newProjectContext} placeholder="Project context/instructions (optional)" rows="2"></textarea>
        <button class="small-btn accent" onclick={addProject} disabled={!newProjectName.trim()}>
          Add Project
        </button>
      </div>
    </div>

    <div class="setting-group">
      <label>MCP Servers</label>
      <p class="hint">Connect to Model Context Protocol servers for tool use (Anthropic provider only).</p>

      {#each mcpServers as server, i (i)}
        <div class="project-item">
          <div class="project-header">
            <span class="project-name">{server.name}</span>
            <button class="small-btn danger" onclick={() => removeMcpServer(i)}>Remove</button>
          </div>
          <p class="project-context-preview"><code>{server.command} {server.args.join(' ')}</code></p>
        </div>
      {/each}

      <div class="new-project">
        <input type="text" bind:value={newMcpName} placeholder="Server name (e.g. filesystem)" />
        <input type="text" bind:value={newMcpCommand} placeholder="Command (e.g. npx)" />
        <input type="text" bind:value={newMcpArgs} placeholder="Arguments (space-separated)" />
        <button class="small-btn accent" onclick={addMcpServer} disabled={!newMcpName.trim() || !newMcpCommand.trim()}>
          Add MCP Server
        </button>
      </div>
    </div>

    <div class="setting-group">
      <label>Custom Commands</label>
      <p class="hint">Define slash commands that run shell scripts. Type <code>/</code> in chat to use them.</p>

      {#each customCommands as cmd, i (i)}
        <div class="project-item">
          <div class="project-header">
            <span class="project-name">/{cmd.name}</span>
            <button class="small-btn danger" onclick={() => removeCustomCommand(i)}>Remove</button>
          </div>
          <p class="project-context-preview">
            <code>{cmd.command}</code>
            {#if cmd.description}<br/>{cmd.description}{/if}
          </p>
        </div>
      {/each}

      <div class="new-project">
        <input type="text" bind:value={newCmdName} placeholder="Command name (e.g. gitlog)" />
        <input type="text" bind:value={newCmdCommand} placeholder="Shell command (e.g. git log --oneline -10)" />
        <input type="text" bind:value={newCmdDescription} placeholder="Description (optional)" />
        <button class="small-btn accent" onclick={addCustomCommand} disabled={!newCmdName.trim() || !newCmdCommand.trim()}>
          Add Command
        </button>
      </div>
    </div>

    {#if error}
      <p class="error">{error}</p>
    {/if}

    <button class="save-btn" onclick={save}>
      {saved ? "Saved!" : "Save Settings"}
    </button>

    <div class="about-section">
      <h3>About</h3>
      <div class="about-info">
        <span>Version: <strong>{appVersion || "..."}</strong></span>
        <span>OS: <strong>{appOs || "..."}</strong> ({appArch || "..."})</span>
        <a href="https://github.com/ponack/ubuntu-claude-desktop" target="_blank" rel="noopener noreferrer">
          GitHub Repository
        </a>
      </div>
    </div>
  </div>
</div>

<style>
  .settings {
    padding: 24px 32px;
    max-width: 640px;
    margin: 0 auto;
    width: 100%;
    height: 100%;
    overflow-y: auto;
    box-sizing: border-box;
  }

  .settings-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    margin-bottom: 32px;
  }

  h2 { font-size: 20px; font-weight: 600; }

  .close-btn {
    padding: 6px 14px;
    border-radius: 8px;
    font-size: 13px;
    color: var(--text-secondary);
    border: 1px solid var(--border);
    transition: background 0.15s;
  }

  .close-btn:hover { background: var(--bg-tertiary); }

  .settings-body {
    display: flex;
    flex-direction: column;
    gap: 24px;
    padding-bottom: 40px;
  }

  .setting-group {
    display: flex;
    flex-direction: column;
    gap: 6px;
  }

  label { font-size: 13px; font-weight: 500; color: var(--text-secondary); }

  input, select {
    padding: 10px 12px;
    background: var(--bg-input);
    border: 1px solid var(--border);
    border-radius: 8px;
    font-size: 14px;
    outline: none;
    transition: border-color 0.15s;
    width: 100%;
    box-sizing: border-box;
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
    width: 100%;
    box-sizing: border-box;
  }

  .code-textarea {
    font-family: "JetBrains Mono", "Fira Code", monospace;
    font-size: 12px;
  }

  input:focus, select:focus, textarea:focus { border-color: var(--accent); }
  select { cursor: pointer; }
  option { background: var(--bg-secondary); }

  .hint { font-size: 12px; color: var(--text-muted); }
  .hint code {
    background: var(--bg-tertiary);
    padding: 1px 4px;
    border-radius: 3px;
    font-size: 11px;
  }

  .link { color: var(--accent); }
  .error { color: var(--danger); font-size: 13px; }

  .input-row { display: flex; gap: 6px; align-items: center; }
  .input-row input { flex: 1; }

  .preset-row { display: flex; gap: 4px; flex-wrap: wrap; }

  .preset-btn {
    padding: 4px 10px;
    border-radius: 6px;
    font-size: 11px;
    border: 1px solid var(--border);
    color: var(--text-secondary);
    transition: all 0.15s;
  }

  .preset-btn:hover { background: var(--bg-tertiary); }
  .preset-btn.active { background: var(--accent); color: white; border-color: var(--accent); }

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

  .save-btn:hover { background: var(--accent-hover); }

  .project-item {
    padding: 10px;
    background: var(--bg-tertiary);
    border-radius: 8px;
    display: flex;
    flex-direction: column;
    gap: 6px;
  }

  .project-header { display: flex; justify-content: space-between; align-items: center; }
  .project-name { font-weight: 500; font-size: 14px; }
  .project-context-preview { font-size: 12px; color: var(--text-muted); line-height: 1.4; }
  .project-actions { display: flex; gap: 6px; }
  .new-project { display: flex; flex-direction: column; gap: 6px; padding-top: 6px; }

  .small-btn {
    padding: 4px 12px;
    border-radius: 6px;
    font-size: 12px;
    border: 1px solid var(--border);
    color: var(--text-secondary);
    transition: background 0.15s;
    white-space: nowrap;
  }

  .small-btn:hover { background: var(--bg-tertiary); }
  .small-btn.accent { background: var(--accent); color: white; border: none; }
  .small-btn.accent:hover { background: var(--accent-hover); }
  .small-btn.accent:disabled { opacity: 0.4; cursor: not-allowed; }
  .small-btn.danger { color: var(--danger); border-color: var(--danger); }
  .small-btn.danger:hover { background: rgba(233, 69, 96, 0.1); }

  .about-section {
    margin-top: 32px;
    padding-top: 24px;
    border-top: 1px solid var(--border);
  }

  .about-section h3 {
    font-size: 14px;
    font-weight: 600;
    margin-bottom: 12px;
    color: var(--text-secondary);
  }

  .about-info {
    display: flex;
    flex-direction: column;
    gap: 6px;
    font-size: 13px;
    color: var(--text-secondary);
  }

  .about-info a {
    color: var(--accent);
    text-decoration: none;
  }

  .about-info a:hover {
    text-decoration: underline;
  }
</style>
