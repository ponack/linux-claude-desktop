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
  let projects = $state([]);
  let newProjectName = $state("");
  let newProjectContext = $state("");
  let editingProject = $state(null);
  let mcpServers = $state([]);
  let newMcpName = $state("");
  let newMcpCommand = $state("");
  let newMcpArgs = $state("");

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
      await loadProjects();
      await loadMcpServers();
    } catch (e) {
      console.error("Failed to load settings:", e);
    }
  });

  async function loadMcpServers() {
    try {
      mcpServers = await invoke("get_mcp_servers");
    } catch (e) {
      console.error("Failed to load MCP servers:", e);
      mcpServers = [];
    }
  }

  async function addMcpServer() {
    if (!newMcpName.trim() || !newMcpCommand.trim()) return;
    const server = {
      name: newMcpName.trim(),
      command: newMcpCommand.trim(),
      args: newMcpArgs.trim() ? newMcpArgs.trim().split(/\s+/) : [],
      env: {},
    };
    mcpServers = [...mcpServers, server];
    await saveMcpServers();
    newMcpName = "";
    newMcpCommand = "";
    newMcpArgs = "";
  }

  async function removeMcpServer(index) {
    mcpServers = mcpServers.filter((_, i) => i !== index);
    await saveMcpServers();
  }

  async function saveMcpServers() {
    try {
      await invoke("set_mcp_servers", { servers: mcpServers });
    } catch (e) {
      error = String(e);
    }
  }

  async function loadProjects() {
    try {
      projects = await invoke("get_projects");
    } catch (e) {
      console.error("Failed to load projects:", e);
    }
  }

  async function addProject() {
    if (!newProjectName.trim()) return;
    try {
      await invoke("create_project", { name: newProjectName.trim(), context: newProjectContext.trim() });
      newProjectName = "";
      newProjectContext = "";
      await loadProjects();
    } catch (e) {
      error = String(e);
    }
  }

  async function saveProject(project) {
    try {
      await invoke("update_project", { id: project.id, name: project.name, context: project.context });
      editingProject = null;
      await loadProjects();
    } catch (e) {
      error = String(e);
    }
  }

  async function removeProject(id) {
    try {
      await invoke("delete_project", { id });
      await loadProjects();
    } catch (e) {
      error = String(e);
    }
  }

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

    <div class="setting-group">
      <label>Projects</label>
      <p class="hint">Projects inject persistent context into conversations assigned to them.</p>

      {#each projects as project (project.id)}
        <div class="project-item">
          {#if editingProject === project.id}
            <input
              type="text"
              bind:value={project.name}
              placeholder="Project name"
            />
            <textarea
              bind:value={project.context}
              placeholder="Project context/instructions..."
              rows="3"
            ></textarea>
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
        <input
          type="text"
          bind:value={newProjectName}
          placeholder="New project name"
        />
        <textarea
          bind:value={newProjectContext}
          placeholder="Project context/instructions (optional)"
          rows="2"
        ></textarea>
        <button class="small-btn accent" onclick={addProject} disabled={!newProjectName.trim()}>
          Add Project
        </button>
      </div>
    </div>

    <div class="setting-group">
      <label>MCP Servers</label>
      <p class="hint">Connect to Model Context Protocol servers for tool use. Servers communicate via stdio.</p>

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
        <input type="text" bind:value={newMcpArgs} placeholder="Arguments (space-separated, e.g. -y @modelcontextprotocol/server-filesystem /home)" />
        <button class="small-btn accent" onclick={addMcpServer} disabled={!newMcpName.trim() || !newMcpCommand.trim()}>
          Add MCP Server
        </button>
      </div>
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

  .project-item {
    padding: 10px;
    background: var(--bg-tertiary);
    border-radius: 8px;
    display: flex;
    flex-direction: column;
    gap: 6px;
  }

  .project-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
  }

  .project-name {
    font-weight: 500;
    font-size: 14px;
  }

  .project-context-preview {
    font-size: 12px;
    color: var(--text-muted);
    line-height: 1.4;
  }

  .project-actions {
    display: flex;
    gap: 6px;
  }

  .new-project {
    display: flex;
    flex-direction: column;
    gap: 6px;
    padding-top: 6px;
  }

  .small-btn {
    padding: 4px 12px;
    border-radius: 6px;
    font-size: 12px;
    border: 1px solid var(--border);
    color: var(--text-secondary);
    transition: background 0.15s;
  }

  .small-btn:hover {
    background: var(--bg-tertiary);
  }

  .small-btn.accent {
    background: var(--accent);
    color: white;
    border: none;
  }

  .small-btn.accent:hover {
    background: var(--accent-hover);
  }

  .small-btn.accent:disabled {
    opacity: 0.4;
    cursor: not-allowed;
  }

  .small-btn.danger {
    color: var(--danger);
    border-color: var(--danger);
  }

  .small-btn.danger:hover {
    background: rgba(233, 69, 96, 0.1);
  }
</style>
