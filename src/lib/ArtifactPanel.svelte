<script>
  import { invoke } from "@tauri-apps/api/core";
  import { save } from "@tauri-apps/plugin-dialog";
  import CodeRenderer from "./artifacts/CodeRenderer.svelte";
  import MarkdownRenderer from "./artifacts/MarkdownRenderer.svelte";
  import MermaidRenderer from "./artifacts/MermaidRenderer.svelte";
  import HtmlRenderer from "./artifacts/HtmlRenderer.svelte";
  import ReactRenderer from "./artifacts/ReactRenderer.svelte";
  import VersionHistory from "./artifacts/VersionHistory.svelte";

  let { artifacts = [], activeArtifactId = null, conversationId, onClose, onSelectArtifact, onIterateWithClaude } = $props();

  let activeTab = $state("preview"); // preview, edit, history
  let content = $state("");
  let editingTitle = $state(false);
  let titleInput = $state("");
  let iterateInput = $state("");
  let showIterateBox = $state(false);
  let showExportMenu = $state(false);

  let activeArtifact = $derived(artifacts.find(a => a.id === activeArtifactId));

  // Load content when active artifact changes
  $effect(() => {
    if (activeArtifactId) {
      loadContent();
      activeTab = "preview";
      showIterateBox = false;
      iterateInput = "";
    }
  });

  async function loadContent() {
    if (!activeArtifactId) return;
    try {
      const c = await invoke("get_artifact_content", { artifactId: activeArtifactId });
      content = c || "";
    } catch (e) {
      console.error("Failed to load artifact content:", e);
    }
  }

  function startEditTitle() {
    titleInput = activeArtifact?.title || "";
    editingTitle = true;
  }

  async function saveTitle() {
    if (!activeArtifactId || !titleInput.trim()) { editingTitle = false; return; }
    try {
      await invoke("update_artifact_title", { artifactId: activeArtifactId, title: titleInput.trim() });
      editingTitle = false;
    } catch (e) {
      console.error("Failed to update title:", e);
    }
  }

  function handleTitleKeydown(e) {
    if (e.key === "Enter") saveTitle();
    if (e.key === "Escape") editingTitle = false;
  }

  async function handleContentChange(newContent) {
    content = newContent;
    try {
      await invoke("save_artifact_version", {
        artifactId: activeArtifactId,
        content: newContent,
        source: "user_edit",
      });
    } catch (e) {
      console.error("Failed to save version:", e);
    }
  }

  async function handleRevert(revertContent) {
    content = revertContent;
    try {
      await invoke("save_artifact_version", {
        artifactId: activeArtifactId,
        content: revertContent,
        source: "revert",
      });
      activeTab = "preview";
    } catch (e) {
      console.error("Failed to revert:", e);
    }
  }

  async function handleIterate() {
    if (!iterateInput.trim() || !onIterateWithClaude) return;
    onIterateWithClaude(activeArtifactId, content, activeArtifact?.language || "", iterateInput.trim());
    iterateInput = "";
    showIterateBox = false;
  }

  async function handleDelete() {
    if (!activeArtifactId) return;
    try {
      await invoke("delete_artifact", { artifactId: activeArtifactId });
      onClose();
    } catch (e) {
      console.error("Failed to delete artifact:", e);
    }
  }

  async function exportToFile() {
    showExportMenu = false;
    try {
      const ext = getFileExtension(activeArtifact?.language || activeArtifact?.artifact_type || "txt");
      const path = await save({
        defaultPath: `${activeArtifact?.title || "artifact"}.${ext}`,
        filters: [{ name: "All Files", extensions: ["*"] }],
      });
      if (path) {
        await invoke("save_artifact_to_file", { path, content });
      }
    } catch (e) {
      console.error("Export failed:", e);
    }
  }

  async function copyToClipboard() {
    showExportMenu = false;
    await navigator.clipboard.writeText(content);
  }

  async function openExternal() {
    showExportMenu = false;
    try {
      await invoke("open_artifact_external", {
        content,
        language: activeArtifact?.language || activeArtifact?.artifact_type || "txt",
      });
    } catch (e) {
      console.error("Open external failed:", e);
    }
  }

  function getFileExtension(lang) {
    const map = {
      javascript: "js", js: "js", typescript: "ts", ts: "ts", python: "py", py: "py",
      rust: "rs", rs: "rs", html: "html", css: "css", json: "json", markdown: "md", md: "md",
      svg: "svg", mermaid: "mmd", react: "jsx", jsx: "jsx",
    };
    return map[lang] || "txt";
  }

  function getRendererType(artifact) {
    if (!artifact) return "code";
    const t = artifact.artifact_type;
    const l = artifact.language;
    if (t === "mermaid" || l === "mermaid") return "mermaid";
    if (t === "markdown" || l === "markdown" || l === "md") return "markdown";
    if (t === "react" || l === "react" || l === "jsx" || l === "tsx") return "react";
    if (t === "html" || t === "svg" || l === "html" || l === "svg") return "html";
    return "code";
  }

  let rendererType = $derived(getRendererType(activeArtifact));
</script>

<div class="artifact-panel">
  <!-- Tab bar for multiple artifacts -->
  {#if artifacts.length > 1}
    <div class="artifact-tabs-bar">
      {#each artifacts as art (art.id)}
        <button
          class="artifact-tab"
          class:active={art.id === activeArtifactId}
          onclick={() => onSelectArtifact(art.id)}
          title={art.title}
        >
          <span class="tab-title">{art.title}</span>
          <span class="tab-type">{art.artifact_type}</span>
        </button>
      {/each}
    </div>
  {/if}

  <!-- Header -->
  <div class="artifact-header">
    <div class="header-left">
      {#if editingTitle}
        <input
          class="title-input"
          bind:value={titleInput}
          onkeydown={handleTitleKeydown}
          onblur={saveTitle}
        />
      {:else}
        <span class="artifact-title" ondblclick={startEditTitle} title="Double-click to rename">
          {activeArtifact?.title || "Untitled"}
        </span>
      {/if}
      <span class="type-badge">{activeArtifact?.artifact_type || ""}</span>
      {#if activeArtifact?.language && activeArtifact.language !== activeArtifact.artifact_type}
        <span class="lang-badge">{activeArtifact.language}</span>
      {/if}
      <span class="version-badge">v{activeArtifact?.current_version || 1}</span>
    </div>
    <div class="header-right">
      <!-- Mode tabs -->
      <div class="mode-tabs">
        <button class="mode-tab" class:active={activeTab === "preview"} onclick={() => activeTab = "preview"}>Preview</button>
        <button class="mode-tab" class:active={activeTab === "edit"} onclick={() => activeTab = "edit"}>Edit</button>
        <button class="mode-tab" class:active={activeTab === "history"} onclick={() => activeTab = "history"}>History</button>
      </div>

      <!-- Export menu -->
      <div class="export-wrapper">
        <button class="icon-btn" onclick={() => showExportMenu = !showExportMenu} title="Export">
          <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
            <path d="M21 15v4a2 2 0 01-2 2H5a2 2 0 01-2-2v-4"/><polyline points="7 10 12 15 17 10"/><line x1="12" y1="15" x2="12" y2="3"/>
          </svg>
        </button>
        {#if showExportMenu}
          <div class="export-menu">
            <button onclick={exportToFile}>Save to file</button>
            <button onclick={copyToClipboard}>Copy to clipboard</button>
            <button onclick={openExternal}>Open in editor</button>
          </div>
        {/if}
      </div>

      <!-- Iterate button -->
      <button class="icon-btn" onclick={() => showIterateBox = !showIterateBox} title="Iterate with Claude">
        <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
          <polyline points="23 4 23 10 17 10"/><path d="M20.49 15a9 9 0 11-2.12-9.36L23 10"/>
        </svg>
      </button>

      <!-- Delete button -->
      <button class="icon-btn danger" onclick={handleDelete} title="Delete artifact">
        <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
          <polyline points="3 6 5 6 21 6"/><path d="M19 6v14a2 2 0 01-2 2H7a2 2 0 01-2-2V6m3 0V4a2 2 0 012-2h4a2 2 0 012 2v2"/>
        </svg>
      </button>

      <!-- Close button -->
      <button class="icon-btn" onclick={onClose} title="Close panel">
        <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
          <line x1="18" y1="6" x2="6" y2="18"/><line x1="6" y1="6" x2="18" y2="18"/>
        </svg>
      </button>
    </div>
  </div>

  <!-- Iterate input -->
  {#if showIterateBox}
    <div class="iterate-bar">
      <input
        class="iterate-input"
        bind:value={iterateInput}
        placeholder="Tell Claude how to modify this artifact..."
        onkeydown={(e) => e.key === "Enter" && handleIterate()}
      />
      <button class="iterate-send" onclick={handleIterate} disabled={!iterateInput.trim()}>Send</button>
    </div>
  {/if}

  <!-- Body -->
  <div class="artifact-body">
    {#if activeTab === "preview"}
      {#if rendererType === "mermaid"}
        <MermaidRenderer {content} />
      {:else if rendererType === "markdown"}
        <MarkdownRenderer {content} />
      {:else if rendererType === "react"}
        <ReactRenderer {content} />
      {:else if rendererType === "html"}
        <HtmlRenderer {content} language={activeArtifact?.language || "html"} />
      {:else}
        <CodeRenderer {content} language={activeArtifact?.language || ""} />
      {/if}
    {:else if activeTab === "edit"}
      <CodeRenderer
        {content}
        language={activeArtifact?.language || ""}
        editable={true}
        onChange={handleContentChange}
      />
    {:else if activeTab === "history"}
      <VersionHistory
        artifactId={activeArtifactId}
        currentVersion={activeArtifact?.current_version || 1}
        onRevert={handleRevert}
      />
    {/if}
  </div>
</div>

<style>
  .artifact-panel {
    width: 45%;
    min-width: 360px;
    max-width: 700px;
    display: flex;
    flex-direction: column;
    border-left: 1px solid var(--border);
    background: var(--bg-secondary);
    height: 100%;
  }

  .artifact-tabs-bar {
    display: flex;
    gap: 0;
    border-bottom: 1px solid var(--border);
    overflow-x: auto;
    flex-shrink: 0;
  }
  .artifact-tab {
    padding: 6px 12px;
    font-size: 12px;
    color: var(--text-muted);
    border-bottom: 2px solid transparent;
    transition: all 0.15s;
    white-space: nowrap;
    display: flex;
    flex-direction: column;
    gap: 1px;
    min-width: 0;
  }
  .artifact-tab:hover { color: var(--text-primary); background: var(--bg-tertiary); }
  .artifact-tab.active { color: var(--text-primary); border-bottom-color: var(--accent); }
  .tab-title { font-weight: 500; overflow: hidden; text-overflow: ellipsis; max-width: 120px; }
  .tab-type { font-size: 10px; opacity: 0.6; }

  .artifact-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 6px 10px;
    border-bottom: 1px solid var(--border);
    gap: 6px;
    flex-shrink: 0;
    flex-wrap: wrap;
  }
  .header-left {
    display: flex;
    align-items: center;
    gap: 6px;
    min-width: 0;
    flex: 1;
  }
  .header-right {
    display: flex;
    align-items: center;
    gap: 4px;
    flex-shrink: 0;
  }
  .artifact-title {
    font-size: 13px;
    font-weight: 600;
    color: var(--text-primary);
    cursor: pointer;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    max-width: 160px;
  }
  .artifact-title:hover { color: var(--accent); }
  .title-input {
    font-size: 13px;
    font-weight: 600;
    background: var(--bg-input);
    border: 1px solid var(--accent);
    border-radius: 4px;
    padding: 2px 6px;
    color: var(--text-primary);
    outline: none;
    width: 150px;
  }
  .type-badge, .lang-badge, .version-badge {
    font-size: 10px;
    padding: 1px 6px;
    border-radius: 4px;
    background: var(--bg-tertiary);
    color: var(--text-muted);
    white-space: nowrap;
  }
  .version-badge { color: var(--accent); }

  .mode-tabs {
    display: flex;
    gap: 2px;
    background: var(--bg-tertiary);
    border-radius: 6px;
    padding: 2px;
  }
  .mode-tab {
    padding: 3px 8px;
    font-size: 11px;
    font-weight: 500;
    border-radius: 4px;
    color: var(--text-muted);
    transition: all 0.15s;
  }
  .mode-tab:hover { color: var(--text-primary); }
  .mode-tab.active { background: var(--accent); color: white; }

  .icon-btn {
    color: var(--text-muted);
    padding: 4px;
    border-radius: 4px;
    display: flex;
    align-items: center;
    transition: color 0.15s, background 0.15s;
  }
  .icon-btn:hover { color: var(--text-primary); background: var(--bg-tertiary); }
  .icon-btn.danger:hover { color: var(--danger); }

  .export-wrapper { position: relative; }
  .export-menu {
    position: absolute;
    top: 100%;
    right: 0;
    background: var(--bg-tertiary);
    border: 1px solid var(--border);
    border-radius: 8px;
    padding: 4px;
    z-index: 100;
    min-width: 160px;
    box-shadow: 0 4px 16px rgba(0,0,0,0.2);
  }
  .export-menu button {
    display: block;
    width: 100%;
    padding: 6px 12px;
    text-align: left;
    font-size: 12px;
    color: var(--text-secondary);
    border-radius: 4px;
    transition: background 0.1s;
  }
  .export-menu button:hover { background: var(--bg-secondary); color: var(--text-primary); }

  .iterate-bar {
    display: flex;
    gap: 6px;
    padding: 6px 10px;
    border-bottom: 1px solid var(--border);
    background: var(--bg-tertiary);
    flex-shrink: 0;
  }
  .iterate-input {
    flex: 1;
    padding: 6px 10px;
    background: var(--bg-input);
    border: 1px solid var(--border);
    border-radius: 6px;
    font-size: 12px;
    color: var(--text-primary);
    outline: none;
  }
  .iterate-input:focus { border-color: var(--accent); }
  .iterate-send {
    padding: 6px 12px;
    background: var(--accent);
    color: white;
    border-radius: 6px;
    font-size: 12px;
    font-weight: 500;
    transition: background 0.15s;
  }
  .iterate-send:hover:not(:disabled) { background: var(--accent-hover); }
  .iterate-send:disabled { opacity: 0.4; cursor: not-allowed; }

  .artifact-body {
    flex: 1;
    overflow: hidden;
    min-height: 0;
  }
</style>
