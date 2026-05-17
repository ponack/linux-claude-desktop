<script>
  import { invoke } from "@tauri-apps/api/core";
  import { onMount } from "svelte";

  let { onClose } = $props();

  // --- Catalog definition ---
  const CATALOG = [
    {
      id: "github",
      name: "GitHub",
      description: "Read issues, PRs, and files. Create issues, comments, and pull requests.",
      category: "developer",
      package: "@modelcontextprotocol/server-github",
      icon: `<path d="M9 19c-5 1.5-5-2.5-7-3m14 6v-3.87a3.37 3.37 0 0 0-.94-2.61c3.14-.35 6.44-1.54 6.44-7A5.44 5.44 0 0 0 20 4.77 5.07 5.07 0 0 0 19.91 1S18.73.65 16 2.48a13.38 13.38 0 0 0-7 0C6.27.65 5.09 1 5.09 1A5.07 5.07 0 0 0 5 4.77a5.44 5.44 0 0 0-1.5 3.78c0 5.42 3.3 6.61 6.44 7A3.37 3.37 0 0 0 9 18.13V22"/>`,
      env: [
        { key: "GITHUB_PERSONAL_ACCESS_TOKEN", label: "GitHub Personal Access Token", hint: "github.com/settings/tokens — needs repo scope", type: "password" }
      ]
    },
    {
      id: "filesystem",
      name: "Filesystem",
      description: "Read and write files in allowed directories on your local filesystem.",
      category: "developer",
      package: "@modelcontextprotocol/server-filesystem",
      icon: `<path d="M22 19a2 2 0 0 1-2 2H4a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2h5l2 3h9a2 2 0 0 1 2 2z"/>`,
      env: [],
      pathsArg: true
    },
    {
      id: "brave-search",
      name: "Brave Search",
      description: "Web and local search via the Brave Search API.",
      category: "search",
      package: "@modelcontextprotocol/server-brave-search",
      icon: `<circle cx="11" cy="11" r="8"/><line x1="21" y1="21" x2="16.65" y2="16.65"/>`,
      env: [
        { key: "BRAVE_API_KEY", label: "Brave API Key", hint: "api.search.brave.com — free tier available", type: "password" }
      ]
    },
    {
      id: "fetch",
      name: "Web Fetch",
      description: "Fetch any web page and extract its content as clean markdown.",
      category: "search",
      package: "@modelcontextprotocol/server-fetch",
      icon: `<circle cx="12" cy="12" r="10"/><line x1="2" y1="12" x2="22" y2="12"/><path d="M12 2a15.3 15.3 0 0 1 4 10 15.3 15.3 0 0 1-4 10 15.3 15.3 0 0 1-4-10 15.3 15.3 0 0 1 4-10z"/>`,
      env: []
    },
    {
      id: "memory",
      name: "Memory",
      description: "Persistent knowledge graph memory for Claude across all conversations.",
      category: "productivity",
      package: "@modelcontextprotocol/server-memory",
      icon: `<ellipse cx="12" cy="5" rx="9" ry="3"/><path d="M21 12c0 1.66-4 3-9 3s-9-1.34-9-3"/><path d="M3 5v14c0 1.66 4 3 9 3s9-1.34 9-3V5"/>`,
      env: []
    },
    {
      id: "sqlite",
      name: "SQLite",
      description: "Explore and query SQLite databases with natural language.",
      category: "data",
      package: "@modelcontextprotocol/server-sqlite",
      icon: `<ellipse cx="12" cy="5" rx="9" ry="3"/><path d="M21 12c0 1.66-4 3-9 3s-9-1.34-9-3"/><path d="M3 5v14c0 1.66 4 3 9 3s9-1.34 9-3V5"/>`,
      env: [],
      dbPathArg: true
    },
    {
      id: "postgres",
      name: "PostgreSQL",
      description: "Read-only access to PostgreSQL databases with schema inspection.",
      category: "data",
      package: "@modelcontextprotocol/server-postgres",
      icon: `<ellipse cx="12" cy="5" rx="9" ry="3"/><path d="M21 12c0 1.66-4 3-9 3s-9-1.34-9-3"/><path d="M3 5v14c0 1.66 4 3 9 3s9-1.34 9-3V5"/>`,
      env: [
        { key: "POSTGRES_URL", label: "PostgreSQL Connection URL", hint: "postgres://user:pass@host:5432/db", type: "text" }
      ]
    },
    {
      id: "puppeteer",
      name: "Puppeteer",
      description: "Browser automation: navigate pages, take screenshots, fill forms.",
      category: "browser",
      package: "@modelcontextprotocol/server-puppeteer",
      icon: `<rect x="2" y="3" width="20" height="14" rx="2"/><path d="M8 21h8M12 17v4"/>`,
      env: []
    },
    {
      id: "slack",
      name: "Slack",
      description: "Read channels and messages, post to channels, search your workspace.",
      category: "productivity",
      package: "@modelcontextprotocol/server-slack",
      icon: `<path d="M14.5 10c-.83 0-1.5-.67-1.5-1.5v-5c0-.83.67-1.5 1.5-1.5s1.5.67 1.5 1.5v5c0 .83-.67 1.5-1.5 1.5z"/><path d="M20.5 10H19V8.5c0-.83.67-1.5 1.5-1.5s1.5.67 1.5 1.5-.67 1.5-1.5 1.5z"/><path d="M9.5 14c.83 0 1.5.67 1.5 1.5v5c0 .83-.67 1.5-1.5 1.5S8 21.33 8 20.5v-5c0-.83.67-1.5 1.5-1.5z"/><path d="M3.5 14H5v1.5c0 .83-.67 1.5-1.5 1.5S2 16.33 2 15.5 2.67 14 3.5 14z"/><path d="M14 14.5c0-.83.67-1.5 1.5-1.5h5c.83 0 1.5.67 1.5 1.5s-.67 1.5-1.5 1.5h-5c-.83 0-1.5-.67-1.5-1.5z"/><path d="M15.5 19H14v1.5c0 .83.67 1.5 1.5 1.5s1.5-.67 1.5-1.5-.67-1.5-1.5-1.5z"/><path d="M10 9.5C10 8.67 9.33 8 8.5 8h-5C2.67 8 2 8.67 2 9.5S2.67 11 3.5 11h5c.83 0 1.5-.67 1.5-1.5z"/><path d="M8.5 5H10V3.5C10 2.67 9.33 2 8.5 2S7 2.67 7 3.5 7.67 5 8.5 5z"/>`,
      env: [
        { key: "SLACK_BOT_TOKEN", label: "Slack Bot Token", hint: "api.slack.com/apps — xoxb-… token", type: "password" },
        { key: "SLACK_TEAM_ID", label: "Slack Team ID", hint: "Found in workspace URL: app.slack.com/client/T…", type: "text" }
      ]
    },
    {
      id: "google-maps",
      name: "Google Maps",
      description: "Geocoding, directions, place search, and elevation queries.",
      category: "search",
      package: "@modelcontextprotocol/server-google-maps",
      icon: `<path d="M21 10c0 7-9 13-9 13s-9-6-9-13a9 9 0 0 1 18 0z"/><circle cx="12" cy="10" r="3"/>`,
      env: [
        { key: "GOOGLE_MAPS_API_KEY", label: "Google Maps API Key", hint: "console.cloud.google.com — enable Maps APIs", type: "password" }
      ]
    },
    {
      id: "sequential-thinking",
      name: "Sequential Thinking",
      description: "Dynamic tool for structured, step-by-step problem solving and analysis.",
      category: "productivity",
      package: "@modelcontextprotocol/server-sequential-thinking",
      icon: `<line x1="8" y1="6" x2="21" y2="6"/><line x1="8" y1="12" x2="21" y2="12"/><line x1="8" y1="18" x2="21" y2="18"/><line x1="3" y1="6" x2="3.01" y2="6"/><line x1="3" y1="12" x2="3.01" y2="12"/><line x1="3" y1="18" x2="3.01" y2="18"/>`,
      env: []
    },
    {
      id: "everything",
      name: "Everything",
      description: "Reference server demonstrating all MCP capabilities — useful for testing.",
      category: "developer",
      package: "@modelcontextprotocol/server-everything",
      icon: `<polygon points="12 2 15.09 8.26 22 9.27 17 14.14 18.18 21.02 12 17.77 5.82 21.02 7 14.14 2 9.27 8.91 8.26 12 2"/>`,
      env: []
    },
  ];

  const CATEGORIES = [
    { id: "all", label: "All" },
    { id: "developer", label: "Developer" },
    { id: "search", label: "Search & Web" },
    { id: "productivity", label: "Productivity" },
    { id: "data", label: "Data" },
    { id: "browser", label: "Browser" },
  ];

  // --- State ---
  let search = $state("");
  let activeCategory = $state("all");
  let nodeAvailable = $state(true);
  let installedIds = $state(new Set());
  let installTarget = $state(null);   // extension being installed
  let installValues = $state({});     // env key → value
  let installPaths = $state("");      // for pathsArg
  let installDbPath = $state("");     // for dbPathArg
  let installing = $state(false);
  let removeTarget = $state(null);    // extension being removed
  let toast = $state(null);           // { message, type }

  // --- Derived ---
  let filtered = $derived(
    CATALOG.filter(ext => {
      const catMatch = activeCategory === "all" || ext.category === activeCategory;
      const q = search.toLowerCase();
      const nameMatch = !q || ext.name.toLowerCase().includes(q) || ext.description.toLowerCase().includes(q);
      return catMatch && nameMatch;
    })
  );

  onMount(async () => {
    nodeAvailable = await invoke("check_node_available");
    await loadInstalled();
  });

  async function loadInstalled() {
    try {
      const servers = await invoke("get_mcp_servers");
      installedIds = new Set(servers.map(s => s.name));
    } catch (_) {}
  }

  function openInstall(ext) {
    installTarget = ext;
    installValues = Object.fromEntries(ext.env.map(e => [e.key, ""]));
    installPaths = "";
    installDbPath = "";
  }

  function closeInstall() {
    installTarget = null;
    installValues = {};
  }

  async function confirmInstall() {
    const ext = installTarget;
    installing = true;
    try {
      const servers = await invoke("get_mcp_servers");
      // Remove any existing entry with same id
      const filtered_servers = servers.filter(s => s.name !== ext.id);

      // Build args
      let args = ["-y", ext.package];
      if (ext.pathsArg) {
        const paths = installPaths.split(",").map(p => p.trim()).filter(Boolean);
        args = [...args, ...paths];
      }
      if (ext.dbPathArg) {
        args = [...args, installDbPath.trim()];
      }

      // Build env
      const env = {};
      for (const [k, v] of Object.entries(installValues)) {
        if (v.trim()) env[k] = v.trim();
      }

      const newServer = {
        name: ext.id,
        command: "npx",
        args,
        env,
      };

      await invoke("set_mcp_servers", { servers: [...filtered_servers, newServer] });
      await loadInstalled();
      closeInstall();
      showToast(`${ext.name} installed — restart chat to activate`, "success");
    } catch (e) {
      showToast(`Install failed: ${e}`, "error");
    } finally {
      installing = false;
    }
  }

  async function removeExtension(ext) {
    try {
      const servers = await invoke("get_mcp_servers");
      await invoke("set_mcp_servers", { servers: servers.filter(s => s.name !== ext.id) });
      await loadInstalled();
      removeTarget = null;
      showToast(`${ext.name} removed`, "success");
    } catch (e) {
      showToast(`Remove failed: ${e}`, "error");
    }
  }

  function showToast(message, type = "success") {
    toast = { message, type };
    setTimeout(() => { toast = null; }, 3500);
  }

  function installReady(ext) {
    if (ext.env.some(e => !installValues[e.key]?.trim())) return false;
    if (ext.pathsArg && !installPaths.trim()) return false;
    if (ext.dbPathArg && !installDbPath.trim()) return false;
    return true;
  }

  function handleKeydown(e) {
    if (e.key === "Escape") {
      if (installTarget) { closeInstall(); return; }
      if (removeTarget) { removeTarget = null; return; }
      onClose();
    }
  }
</script>

<!-- svelte-ignore a11y_no_static_element_interactions -->
<div class="ext-layout" onkeydown={handleKeydown}>
  <!-- Header -->
  <div class="ext-header">
    <div class="ext-title">
      <svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"
           stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">
        <path d="M21 16V8a2 2 0 0 0-1-1.73l-7-4a2 2 0 0 0-2 0l-7 4A2 2 0 0 0 3 8v8a2 2 0 0 0 1 1.73l7 4a2 2 0 0 0 2 0l7-4A2 2 0 0 0 21 16z"/>
        <polyline points="3.27 6.96 12 12.01 20.73 6.96"/><line x1="12" y1="22.08" x2="12" y2="12"/>
      </svg>
      <span>Extensions</span>
      <span class="ext-count">{CATALOG.length} available</span>
    </div>
    <button class="close-btn" onclick={onClose} aria-label="Close extensions">
      <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
        <line x1="18" y1="6" x2="6" y2="18"/><line x1="6" y1="6" x2="18" y2="18"/>
      </svg>
    </button>
  </div>

  <!-- Node warning -->
  {#if !nodeAvailable}
    <div class="ext-warning" role="alert">
      <strong>Node.js not found.</strong> Extensions require Node.js and npx.
      <code>sudo apt install nodejs npm</code>
    </div>
  {/if}

  <!-- Toolbar: search + categories -->
  <div class="ext-toolbar">
    <div class="search-wrap">
      <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"
           stroke-linecap="round" stroke-linejoin="round" class="search-icon" aria-hidden="true">
        <circle cx="11" cy="11" r="8"/><line x1="21" y1="21" x2="16.65" y2="16.65"/>
      </svg>
      <input
        class="ext-search"
        type="search"
        placeholder="Search extensions…"
        bind:value={search}
        aria-label="Search extensions"
      />
    </div>
    <div class="cat-tabs" role="tablist" aria-label="Extension categories">
      {#each CATEGORIES as cat}
        <button
          role="tab"
          aria-selected={activeCategory === cat.id}
          class="cat-tab"
          class:active={activeCategory === cat.id}
          onclick={() => activeCategory = cat.id}
        >{cat.label}</button>
      {/each}
    </div>
  </div>

  <!-- Grid -->
  <div class="ext-grid" role="list" aria-label="Available extensions">
    {#if filtered.length === 0}
      <div class="no-results">No extensions match your search.</div>
    {:else}
      {#each filtered as ext}
        {@const installed = installedIds.has(ext.id)}
        <div class="ext-card" class:installed role="listitem">
          <div class="card-header">
            <div class="card-icon" aria-hidden="true">
              <svg width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor"
                   stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round">
                {@html ext.icon}
              </svg>
            </div>
            <div class="card-meta">
              <div class="card-name">{ext.name}</div>
              <div class="card-pkg">{ext.package}</div>
            </div>
            {#if installed}
              <span class="badge-installed">Installed</span>
            {/if}
          </div>
          <p class="card-desc">{ext.description}</p>
          <div class="card-footer">
            <span class="cat-badge">{ext.category}</span>
            <div class="card-actions">
              {#if installed}
                <button class="btn-remove" onclick={() => removeTarget = ext}
                        aria-label="Remove {ext.name}">Remove</button>
              {:else}
                <button class="btn-install" onclick={() => openInstall(ext)}
                        disabled={!nodeAvailable}
                        aria-label="Install {ext.name}">
                  <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor"
                       stroke-width="2.5" aria-hidden="true">
                    <line x1="12" y1="5" x2="12" y2="19"/><line x1="5" y1="12" x2="19" y2="12"/>
                  </svg>
                  Install
                </button>
              {/if}
            </div>
          </div>
        </div>
      {/each}
    {/if}
  </div>
</div>

<!-- Install dialog -->
{#if installTarget}
  <!-- svelte-ignore a11y_click_events_have_key_events -->
  <!-- svelte-ignore a11y_no_static_element_interactions -->
  <div class="dialog-backdrop" onclick={closeInstall} role="presentation">
    <!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
    <div class="dialog" role="dialog" aria-modal="true"
         aria-labelledby="install-dialog-title"
         onclick={e => e.stopPropagation()}>
      <h3 id="install-dialog-title" class="dialog-title">Install {installTarget.name}</h3>
      <p class="dialog-pkg">{installTarget.package}</p>

      {#if installTarget.env.length === 0 && !installTarget.pathsArg && !installTarget.dbPathArg}
        <p class="dialog-note">No configuration required. This extension runs immediately via npx.</p>
      {/if}

      {#each installTarget.env as field}
        <div class="field-group">
          <label class="field-label" for="install-{field.key}">{field.label}</label>
          {#if field.type === "password"}
            <input id="install-{field.key}" type="password" class="field-input"
                   placeholder="Enter {field.label.toLowerCase()}…"
                   bind:value={installValues[field.key]} />
          {:else}
            <input id="install-{field.key}" type="text" class="field-input"
                   placeholder="Enter {field.label.toLowerCase()}…"
                   bind:value={installValues[field.key]} />
          {/if}
          {#if field.hint}
            <p class="field-hint">{field.hint}</p>
          {/if}
        </div>
      {/each}

      {#if installTarget.pathsArg}
        <div class="field-group">
          <label class="field-label" for="install-paths">Allowed Paths</label>
          <input id="install-paths" type="text" class="field-input"
                 placeholder="/home/user/projects, /tmp"
                 bind:value={installPaths} />
          <p class="field-hint">Comma-separated absolute paths Claude can read and write</p>
        </div>
      {/if}

      {#if installTarget.dbPathArg}
        <div class="field-group">
          <label class="field-label" for="install-dbpath">Database Path</label>
          <input id="install-dbpath" type="text" class="field-input"
                 placeholder="/home/user/my.db"
                 bind:value={installDbPath} />
          <p class="field-hint">Absolute path to the SQLite database file</p>
        </div>
      {/if}

      <div class="dialog-note warn">
        <svg width="13" height="13" viewBox="0 0 24 24" fill="none" stroke="currentColor"
             stroke-width="2" aria-hidden="true">
          <circle cx="12" cy="12" r="10"/><line x1="12" y1="8" x2="12" y2="12"/>
          <line x1="12" y1="16" x2="12.01" y2="16"/>
        </svg>
        npx will download and run <code>{installTarget.package}</code> from the npm registry.
        Only install extensions you trust.
      </div>

      <div class="dialog-actions">
        <button class="btn-cancel" onclick={closeInstall}>Cancel</button>
        <button class="btn-confirm" onclick={confirmInstall}
                disabled={!installReady(installTarget) || installing}>
          {installing ? "Installing…" : "Install"}
        </button>
      </div>
    </div>
  </div>
{/if}

<!-- Remove confirmation dialog -->
{#if removeTarget}
  <!-- svelte-ignore a11y_click_events_have_key_events -->
  <!-- svelte-ignore a11y_no_static_element_interactions -->
  <div class="dialog-backdrop" onclick={() => removeTarget = null} role="presentation">
    <!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
    <div class="dialog" role="dialog" aria-modal="true"
         aria-labelledby="remove-dialog-title"
         onclick={e => e.stopPropagation()}>
      <h3 id="remove-dialog-title" class="dialog-title">Remove {removeTarget.name}?</h3>
      <p class="dialog-pkg">This will remove the MCP server configuration. You can reinstall it at any time.</p>
      <div class="dialog-actions">
        <button class="btn-cancel" onclick={() => removeTarget = null}>Cancel</button>
        <button class="btn-remove-confirm" onclick={() => removeExtension(removeTarget)}>Remove</button>
      </div>
    </div>
  </div>
{/if}

<!-- Toast -->
{#if toast}
  <div class="toast" class:toast-error={toast.type === "error"} role="status" aria-live="polite">
    {toast.message}
  </div>
{/if}

<style>
  .ext-layout {
    display: flex;
    flex-direction: column;
    height: 100%;
    background: var(--bg-primary);
    overflow: hidden;
  }

  /* Header */
  .ext-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 12px 20px;
    border-bottom: 1px solid var(--border);
    background: var(--bg-secondary);
    flex-shrink: 0;
  }

  .ext-title {
    display: flex;
    align-items: center;
    gap: 8px;
    font-weight: 600;
    font-size: 15px;
  }

  .ext-count {
    font-size: 11px;
    font-weight: 400;
    color: var(--text-muted);
    background: var(--bg-tertiary);
    padding: 2px 8px;
    border-radius: 10px;
  }

  .close-btn {
    padding: 4px;
    color: var(--text-muted);
    border-radius: 4px;
    display: flex;
    align-items: center;
  }
  .close-btn:hover { color: var(--text-primary); background: var(--bg-tertiary); }

  /* Warning */
  .ext-warning {
    margin: var(--space-3) 20px 0;
    padding: 10px 14px;
    border-radius: var(--radius-3);
    background: var(--accent-soft);
    border: 1px solid var(--danger);
    font-size: 13px;
    flex-shrink: 0;
  }
  .ext-warning code {
    background: var(--code-bg);
    padding: 2px 8px;
    border-radius: 4px;
    font-size: 12px;
    margin-left: 8px;
  }

  /* Toolbar */
  .ext-toolbar {
    display: flex;
    flex-direction: column;
    gap: 10px;
    padding: 12px 20px;
    border-bottom: 1px solid var(--border);
    background: var(--bg-secondary);
    flex-shrink: 0;
  }

  .search-wrap {
    position: relative;
    display: flex;
    align-items: center;
  }

  .search-icon {
    position: absolute;
    left: 10px;
    color: var(--text-muted);
    pointer-events: none;
  }

  .ext-search {
    width: 100%;
    background: var(--bg-input);
    border: 1px solid var(--border);
    border-radius: 8px;
    padding: 7px 12px 7px 32px;
    color: var(--text-primary);
    font-size: 13px;
    box-sizing: border-box;
  }
  .ext-search:focus { border-color: var(--accent); outline: none; }

  .cat-tabs {
    display: flex;
    gap: 4px;
    flex-wrap: wrap;
  }

  .cat-tab {
    padding: 4px 12px;
    border-radius: 20px;
    font-size: 12px;
    font-weight: 500;
    color: var(--text-muted);
    background: transparent;
    border: 1px solid transparent;
    cursor: pointer;
    transition: background 0.15s, color 0.15s;
  }
  .cat-tab:hover { background: var(--bg-tertiary); color: var(--text-primary); }
  .cat-tab.active {
    background: var(--accent);
    color: #fff;
    border-color: var(--accent);
  }

  /* Grid */
  .ext-grid {
    flex: 1;
    overflow-y: auto;
    padding: 16px 20px;
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(280px, 1fr));
    gap: 14px;
    align-content: start;
  }

  .no-results {
    grid-column: 1 / -1;
    text-align: center;
    color: var(--text-muted);
    padding: 60px 0;
    font-size: 14px;
  }

  /* Card */
  .ext-card {
    background: var(--bg-secondary);
    border: 1px solid var(--border);
    border-radius: 10px;
    padding: 14px;
    display: flex;
    flex-direction: column;
    gap: 8px;
    transition: border-color 0.15s, box-shadow 0.15s;
  }
  .ext-card:hover { border-color: var(--accent); box-shadow: 0 0 0 1px var(--accent); }
  .ext-card.installed { border-color: var(--success); }
  .ext-card.installed:hover { box-shadow: 0 0 0 1px var(--success); }

  .card-header {
    display: flex;
    align-items: flex-start;
    gap: 10px;
  }

  .card-icon {
    width: 36px;
    height: 36px;
    border-radius: 8px;
    background: var(--bg-tertiary);
    display: flex;
    align-items: center;
    justify-content: center;
    flex-shrink: 0;
    color: var(--accent);
  }

  .card-meta { flex: 1; min-width: 0; }
  .card-name { font-weight: 600; font-size: 14px; }
  .card-pkg { font-size: 10px; color: var(--text-muted); font-family: monospace; white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }

  .badge-installed {
    flex-shrink: 0;
    font-size: 10px;
    font-weight: 700;
    padding: 2px 8px;
    border-radius: 10px;
    background: rgba(39, 174, 96, 0.15);
    color: var(--success);
    text-transform: uppercase;
    letter-spacing: 0.4px;
  }

  .card-desc {
    font-size: 12px;
    color: var(--text-secondary);
    line-height: 1.5;
    flex: 1;
  }

  .card-footer {
    display: flex;
    align-items: center;
    justify-content: space-between;
    margin-top: auto;
  }

  .cat-badge {
    font-size: 10px;
    text-transform: uppercase;
    letter-spacing: 0.4px;
    color: var(--text-muted);
    background: var(--bg-tertiary);
    padding: 2px 8px;
    border-radius: 10px;
  }

  .card-actions { display: flex; gap: 6px; }

  .btn-install {
    display: flex;
    align-items: center;
    gap: 5px;
    padding: 5px 12px;
    border-radius: 6px;
    font-size: 12px;
    font-weight: 600;
    background: var(--accent);
    color: #fff;
    cursor: pointer;
  }
  .btn-install:hover:not(:disabled) { background: var(--accent-hover); }
  .btn-install:disabled { opacity: 0.4; cursor: not-allowed; }

  .btn-remove {
    padding: 5px 12px;
    border-radius: 6px;
    font-size: 12px;
    font-weight: 500;
    border: 1px solid var(--border);
    color: var(--text-muted);
    cursor: pointer;
  }
  .btn-remove:hover { background: var(--accent-soft); color: var(--danger); border-color: var(--danger); }

  /* Dialogs */
  .dialog-backdrop {
    position: fixed;
    inset: 0;
    background: var(--overlay-bg);
    display: flex;
    align-items: center;
    justify-content: center;
    z-index: var(--z-dialog);
    padding: 20px;
  }

  .dialog {
    background: var(--bg-secondary);
    border: 1px solid var(--border);
    border-radius: 12px;
    padding: 24px;
    width: 100%;
    max-width: 440px;
    display: flex;
    flex-direction: column;
    gap: 14px;
  }

  .dialog-title {
    font-size: 16px;
    font-weight: 700;
    margin: 0;
  }

  .dialog-pkg {
    font-size: 12px;
    color: var(--text-muted);
    font-family: monospace;
    margin: 0;
  }

  .dialog-note {
    font-size: 12px;
    color: var(--text-secondary);
    background: var(--bg-tertiary);
    padding: 10px 12px;
    border-radius: 6px;
  }

  .dialog-note.warn {
    display: flex;
    align-items: flex-start;
    gap: 7px;
    background: rgba(255, 180, 0, 0.08);
    border: 1px solid rgba(255, 180, 0, 0.2);
    color: var(--text-secondary);
  }
  .dialog-note.warn code { background: var(--code-bg); padding: 1px 5px; border-radius: 3px; font-size: 11px; }

  .field-group { display: flex; flex-direction: column; gap: 4px; }

  .field-label {
    font-size: 11px;
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 0.5px;
    color: var(--text-muted);
  }

  .field-input {
    background: var(--bg-input);
    border: 1px solid var(--border);
    border-radius: 6px;
    padding: 8px 10px;
    color: var(--text-primary);
    font-size: 13px;
    font-family: inherit;
  }
  .field-input:focus { border-color: var(--accent); outline: none; }

  .field-hint { font-size: 11px; color: var(--text-muted); margin: 0; }

  .dialog-actions { display: flex; justify-content: flex-end; gap: 8px; margin-top: 4px; }

  .btn-cancel {
    padding: 7px 16px;
    border-radius: 6px;
    font-size: 13px;
    border: 1px solid var(--border);
    color: var(--text-secondary);
    cursor: pointer;
  }
  .btn-cancel:hover { background: var(--bg-tertiary); }

  .btn-confirm {
    padding: 7px 20px;
    border-radius: 6px;
    font-size: 13px;
    font-weight: 600;
    background: var(--accent);
    color: #fff;
    cursor: pointer;
  }
  .btn-confirm:hover:not(:disabled) { background: var(--accent-hover); }
  .btn-confirm:disabled { opacity: 0.4; cursor: not-allowed; }

  .btn-remove-confirm {
    padding: 7px 20px;
    border-radius: 6px;
    font-size: 13px;
    font-weight: 600;
    background: var(--danger);
    color: #fff;
    cursor: pointer;
  }
  .btn-remove-confirm:hover { opacity: 0.85; }

  /* Toast */
  .toast {
    position: fixed;
    bottom: 24px;
    left: 50%;
    transform: translateX(-50%);
    background: var(--bg-tertiary);
    border: 1px solid var(--border);
    color: var(--text-primary);
    font-size: 13px;
    padding: 10px 20px;
    border-radius: 20px;
    z-index: 200;
    white-space: nowrap;
    box-shadow: var(--shadow-2);
    animation: fade-in 0.2s ease;
  }
  .toast-error { background: var(--accent-soft-hover); border-color: var(--danger); color: var(--danger); }

  @keyframes fade-in {
    from { opacity: 0; transform: translateX(-50%) translateY(8px); }
    to   { opacity: 1; transform: translateX(-50%) translateY(0); }
  }
</style>
