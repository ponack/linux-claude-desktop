<script>
  import { invoke } from "@tauri-apps/api/core";
  import { diffLines } from "diff";

  let { artifactId, currentVersion, onRevert } = $props();
  let versions = $state([]);
  let selectedVersion = $state(null);
  let diffHtml = $state("");
  let compareFrom = $state(null);

  async function loadVersions() {
    if (!artifactId) return;
    try {
      versions = await invoke("get_artifact_versions", { artifactId });
    } catch (e) {
      console.error("Failed to load versions:", e);
    }
  }

  $effect(() => {
    artifactId;
    loadVersions();
    selectedVersion = null;
    diffHtml = "";
    compareFrom = null;
  });

  function showDiff(fromIdx, toIdx) {
    if (fromIdx < 0 || toIdx < 0 || fromIdx >= versions.length || toIdx >= versions.length) return;
    const oldText = versions[fromIdx].content;
    const newText = versions[toIdx].content;
    const changes = diffLines(oldText, newText);
    let html = "";
    for (const part of changes) {
      const cls = part.added ? "diff-added" : part.removed ? "diff-removed" : "diff-unchanged";
      const prefix = part.added ? "+" : part.removed ? "-" : " ";
      const lines = part.value.split("\n");
      for (let i = 0; i < lines.length; i++) {
        if (i === lines.length - 1 && lines[i] === "") continue;
        html += `<div class="${cls}"><span class="diff-prefix">${prefix}</span>${escapeHtml(lines[i])}</div>`;
      }
    }
    diffHtml = html;
  }

  function escapeHtml(str) {
    return str.replace(/&/g, "&amp;").replace(/</g, "&lt;").replace(/>/g, "&gt;");
  }

  function compareWithPrevious(idx) {
    if (idx <= 0) return;
    compareFrom = idx - 1;
    showDiff(idx - 1, idx);
  }
</script>

<div class="version-history">
  <div class="version-list">
    {#each versions as version, i (version.id)}
      <div
        class="version-item"
        class:active={selectedVersion === i}
        class:current={version.version === currentVersion}
      >
        <!-- svelte-ignore a11y_no_static_element_interactions -->
        <div class="version-header" onclick={() => { selectedVersion = i; diffHtml = ""; }}>
          <span class="version-number">v{version.version}</span>
          <span class="version-source">{version.source}</span>
          {#if version.version === currentVersion}
            <span class="version-badge">current</span>
          {/if}
        </div>
        <div class="version-meta">
          {new Date(version.created_at).toLocaleString()}
        </div>
        <div class="version-actions">
          {#if i > 0}
            <button class="ver-btn" onclick={() => compareWithPrevious(i)}>Diff</button>
          {/if}
          {#if version.version !== currentVersion}
            <button class="ver-btn revert" onclick={() => onRevert(version.content)}>Revert</button>
          {/if}
        </div>
      </div>
    {/each}
  </div>

  {#if diffHtml}
    <div class="diff-view">
      <div class="diff-header">
        <span>Comparing v{versions[compareFrom]?.version} -> v{versions[compareFrom + 1]?.version}</span>
      </div>
      <div class="diff-content">{@html diffHtml}</div>
    </div>
  {/if}

  {#if selectedVersion !== null && !diffHtml}
    <div class="version-preview">
      <div class="diff-header"><span>v{versions[selectedVersion].version} content</span></div>
      <pre class="version-code">{versions[selectedVersion].content}</pre>
    </div>
  {/if}
</div>

<style>
  .version-history {
    height: 100%;
    display: flex;
    flex-direction: column;
    overflow: hidden;
  }
  .version-list {
    flex-shrink: 0;
    max-height: 40%;
    overflow-y: auto;
    border-bottom: 1px solid var(--border);
  }
  .version-item {
    padding: 8px 12px;
    border-bottom: 1px solid var(--border);
    cursor: pointer;
    transition: background 0.1s;
  }
  .version-item:hover { background: var(--bg-tertiary); }
  .version-item.active { background: var(--bg-tertiary); }
  .version-item.current { border-left: 3px solid var(--accent); }
  .version-header {
    display: flex;
    align-items: center;
    gap: 8px;
  }
  .version-number { font-weight: 600; font-size: 13px; color: var(--text-primary); }
  .version-source {
    font-size: 11px;
    padding: 1px 6px;
    border-radius: 4px;
    background: var(--bg-tertiary);
    color: var(--text-muted);
  }
  .version-badge {
    font-size: 10px;
    padding: 1px 6px;
    border-radius: 4px;
    background: var(--accent);
    color: white;
    margin-left: auto;
  }
  .version-meta { font-size: 11px; color: var(--text-muted); margin-top: 2px; }
  .version-actions { display: flex; gap: 4px; margin-top: 4px; }
  .ver-btn {
    padding: 2px 8px;
    font-size: 11px;
    border-radius: 4px;
    color: var(--text-secondary);
    background: var(--bg-tertiary);
    border: 1px solid var(--border);
    transition: background 0.1s;
  }
  .ver-btn:hover { background: var(--bg-secondary); }
  .ver-btn.revert { color: var(--accent); border-color: var(--accent); }
  .ver-btn.revert:hover { background: rgba(78, 204, 163, 0.1); }

  .diff-view, .version-preview {
    flex: 1;
    overflow: auto;
    min-height: 0;
  }
  .diff-header {
    padding: 6px 12px;
    font-size: 12px;
    font-weight: 600;
    color: var(--text-muted);
    background: var(--bg-tertiary);
    border-bottom: 1px solid var(--border);
    position: sticky;
    top: 0;
  }
  .diff-content {
    font-family: "JetBrains Mono", "Fira Code", monospace;
    font-size: 12px;
    line-height: 1.5;
    padding: 0;
  }
  .diff-content :global(.diff-added) { background: rgba(78, 204, 163, 0.15); color: #4ecca3; }
  .diff-content :global(.diff-removed) { background: rgba(233, 69, 96, 0.15); color: #e94560; }
  .diff-content :global(.diff-unchanged) { color: var(--text-muted); }
  .diff-content :global(.diff-prefix) { display: inline-block; width: 20px; text-align: center; opacity: 0.6; user-select: none; }
  .version-code {
    margin: 0;
    padding: 12px;
    font-family: "JetBrains Mono", "Fira Code", monospace;
    font-size: 12px;
    line-height: 1.5;
    color: var(--text-primary);
    white-space: pre-wrap;
    word-break: break-word;
  }
</style>
