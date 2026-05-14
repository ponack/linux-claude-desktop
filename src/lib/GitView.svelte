<script>
  import { invoke } from "@tauri-apps/api/core";
  import { open as openDialog } from "@tauri-apps/plugin-dialog";
  import { onMount } from "svelte";

  let { onClose, onSendToChat } = $props();

  // ── Repo state ────────────────────────────────────────────────────────────
  let savedRepos = $state([]);
  let repoPath = $state("");
  let repoInfo = $state(null); // { branch, remote_url, ahead, behind, is_git_repo }

  // ── File status state ─────────────────────────────────────────────────────
  let stagedFiles = $state([]);
  let unstagedFiles = $state([]);
  let untrackedFiles = $state([]);
  let selectedFiles = $state(new Set()); // paths of checked items

  // ── Diff / Log ────────────────────────────────────────────────────────────
  let activeTab = $state("diff"); // "diff" | "log"
  let diffContent = $state("");
  let diffFile = $state(null); // { path, staged }
  let commits = $state([]);
  let loadingDiff = $state(false);
  let loadingLog = $state(false);

  // ── Commit form ───────────────────────────────────────────────────────────
  let commitMessage = $state("");
  let committing = $state(false);
  let commitError = $state("");

  // ── Push / Pull ───────────────────────────────────────────────────────────
  let pushPullStatus = $state(""); // "", "pushing", "pulling", "done", "error"
  let pushPullMsg = $state("");

  // ── Git availability ──────────────────────────────────────────────────────
  let gitAvailable = $state(true);

  onMount(async () => {
    const avail = await invoke("check_git_available").catch(() => ({ available: false }));
    gitAvailable = avail.available;
    if (!gitAvailable) return;

    savedRepos = await invoke("get_git_repos").catch(() => []);

    const defaultRepo = await invoke("get_git_default_repo").catch(() => "");
    if (defaultRepo) {
      await openRepo(defaultRepo);
    } else if (savedRepos.length > 0) {
      await openRepo(savedRepos[0].path);
    }
  });

  async function pickRepo() {
    const selected = await openDialog({ directory: true, title: "Select Git Repository" });
    if (!selected) return;
    await openRepo(selected);
  }

  async function openRepo(path) {
    repoPath = path;
    const info = await invoke("git_repo_info", { path }).catch(() => null);
    repoInfo = info;
    if (!info?.is_git_repo) return;

    // Persist in saved repos
    const name = path.split("/").filter(Boolean).pop() || path;
    await invoke("add_git_repo", { path, name }).catch(() => {});
    await invoke("touch_git_repo", { path }).catch(() => {});
    savedRepos = await invoke("get_git_repos").catch(() => []);

    await refreshStatus();
    if (activeTab === "log") await loadLog();
  }

  async function refreshStatus() {
    if (!repoPath) return;
    const files = await invoke("git_status", { path: repoPath }).catch(() => []);
    stagedFiles = files.filter((f) => f.staged);
    unstagedFiles = files.filter((f) => !f.staged && f.status !== "untracked");
    untrackedFiles = files.filter((f) => f.status === "untracked");
    repoInfo = await invoke("git_repo_info", { path: repoPath }).catch(() => repoInfo);
  }

  async function loadLog() {
    if (!repoPath) return;
    loadingLog = true;
    commits = await invoke("git_log", { path: repoPath, limit: 50 }).catch(() => []);
    loadingLog = false;
  }

  async function showDiff(file, staged) {
    if (!repoPath) return;
    diffFile = { path: file, staged };
    loadingDiff = true;
    activeTab = "diff";
    diffContent = await invoke("git_diff", { path: repoPath, file, staged }).catch((e) => `Error: ${e}`);
    loadingDiff = false;
  }

  async function stageFile(file) {
    await invoke("git_stage", { path: repoPath, files: [file] }).catch(() => {});
    await refreshStatus();
  }

  async function unstageFile(file) {
    await invoke("git_unstage", { path: repoPath, files: [file] }).catch(() => {});
    await refreshStatus();
  }

  async function stageAll() {
    await invoke("git_stage", { path: repoPath, files: [] }).catch(() => {});
    await refreshStatus();
  }

  async function unstageAll() {
    await invoke("git_unstage", { path: repoPath, files: [] }).catch(() => {});
    await refreshStatus();
  }

  async function doCommit() {
    if (!commitMessage.trim()) return;
    committing = true;
    commitError = "";
    try {
      const result = await invoke("git_commit", { path: repoPath, message: commitMessage.trim() });
      commitMessage = "";
      await refreshStatus();
    } catch (e) {
      commitError = String(e);
    } finally {
      committing = false;
    }
  }

  async function doPush() {
    pushPullStatus = "pushing";
    pushPullMsg = "";
    try {
      const out = await invoke("git_push", { path: repoPath });
      pushPullMsg = out.trim() || "Push successful";
      pushPullStatus = "done";
      await refreshStatus();
    } catch (e) {
      pushPullMsg = String(e);
      pushPullStatus = "error";
    }
  }

  async function doPull() {
    pushPullStatus = "pulling";
    pushPullMsg = "";
    try {
      const out = await invoke("git_pull", { path: repoPath });
      pushPullMsg = out.trim() || "Pull successful";
      pushPullStatus = "done";
      await refreshStatus();
      if (activeTab === "log") await loadLog();
    } catch (e) {
      pushPullMsg = String(e);
      pushPullStatus = "error";
    }
  }

  async function switchTab(tab) {
    activeTab = tab;
    if (tab === "log" && commits.length === 0) await loadLog();
  }

  function sendDiffToChat() {
    if (!diffContent) return;
    const text = diffFile
      ? `Git diff for \`${diffFile.path}\`${diffFile.staged ? " (staged)" : ""}:\n\`\`\`diff\n${diffContent}\n\`\`\``
      : `Git diff:\n\`\`\`diff\n${diffContent}\n\`\`\``;
    onSendToChat?.(text);
  }

  async function sendFullDiffToChat() {
    const full = await invoke("git_diff", { path: repoPath, file: null, staged: false }).catch(() => "");
    const staged = await invoke("git_diff", { path: repoPath, file: null, staged: true }).catch(() => "");
    const combined = [staged && `# Staged changes\n\`\`\`diff\n${staged}\n\`\`\``, full && `# Unstaged changes\n\`\`\`diff\n${full}\n\`\`\``].filter(Boolean).join("\n\n");
    if (combined) onSendToChat?.(combined);
  }

  function statusIcon(status) {
    switch (status) {
      case "modified": return "M";
      case "added": return "A";
      case "deleted": return "D";
      case "renamed": return "R";
      case "copied": return "C";
      case "untracked": return "?";
      default: return "·";
    }
  }

  function statusClass(status) {
    switch (status) {
      case "added":
      case "untracked": return "s-added";
      case "deleted": return "s-deleted";
      case "renamed":
      case "copied": return "s-renamed";
      default: return "s-modified";
    }
  }

  function handleKeydown(e) {
    if (e.key === "Escape") onClose?.();
  }
</script>

<svelte:window onkeydown={handleKeydown} />

<div class="git-view">
  <!-- Header -->
  <div class="git-header">
    <div class="git-header-left">
      <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" class="git-icon">
        <circle cx="18" cy="18" r="3"/><circle cx="6" cy="6" r="3"/><circle cx="6" cy="18" r="3"/>
        <path d="M6 9v6M15.4 6.4A8 8 0 0 1 21 13v2"/>
      </svg>
      <span class="git-title">Git</span>
      {#if repoInfo?.is_git_repo}
        <span class="branch-badge">
          <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
            <line x1="6" y1="3" x2="6" y2="15"/><circle cx="18" cy="6" r="3"/><circle cx="6" cy="18" r="3"/>
            <path d="M18 9a9 9 0 0 1-9 9"/>
          </svg>
          {repoInfo.branch}
        </span>
        {#if repoInfo.ahead > 0 || repoInfo.behind > 0}
          <span class="sync-badge">
            {#if repoInfo.ahead > 0}↑{repoInfo.ahead}{/if}
            {#if repoInfo.behind > 0}↓{repoInfo.behind}{/if}
          </span>
        {/if}
      {/if}
    </div>
    <div class="git-header-right">
      {#if repoInfo?.is_git_repo}
        <button class="header-btn" onclick={sendFullDiffToChat} title="Send all changes to Claude">
          <svg width="13" height="13" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
            <path d="M21 15a2 2 0 0 1-2 2H7l-4 4V5a2 2 0 0 1 2-2h14a2 2 0 0 1 2 2z"/>
          </svg>
          Ask Claude
        </button>
        <button class="header-btn" onclick={refreshStatus} title="Refresh">
          <svg width="13" height="13" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
            <polyline points="23 4 23 10 17 10"/><polyline points="1 20 1 14 7 14"/>
            <path d="M3.51 9a9 9 0 0 1 14.85-3.36L23 10M1 14l4.64 4.36A9 9 0 0 0 20.49 15"/>
          </svg>
        </button>
      {/if}
      <button class="header-btn close-btn" onclick={onClose} title="Close (Esc)">✕</button>
    </div>
  </div>

  {#if !gitAvailable}
    <div class="git-empty">
      <p>git is not installed. Run <code>sudo apt install git</code> to enable this feature.</p>
    </div>
  {:else}
    <!-- Repo picker bar -->
    <div class="repo-bar">
      {#if savedRepos.length > 0}
        <select class="repo-select" value={repoPath} onchange={(e) => openRepo(e.target.value)}>
          <option value="" disabled>Select a repository…</option>
          {#each savedRepos as repo (repo.id)}
            <option value={repo.path}>{repo.name} — {repo.path}</option>
          {/each}
        </select>
      {:else}
        <span class="repo-placeholder">No repositories opened yet</span>
      {/if}
      <button class="repo-open-btn" onclick={pickRepo}>
        <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
          <path d="M22 19a2 2 0 0 1-2 2H4a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2h5l2 3h9a2 2 0 0 1 2 2z"/>
        </svg>
        Open Repo
      </button>
    </div>

    {#if !repoPath}
      <div class="git-empty">
        <p>Open a git repository to get started.</p>
      </div>
    {:else if repoInfo && !repoInfo.is_git_repo}
      <div class="git-empty">
        <p><strong>{repoPath}</strong> is not a git repository.</p>
      </div>
    {:else if repoInfo?.is_git_repo}
      <div class="git-body">
        <!-- Left panel: files + commit -->
        <div class="left-panel">
          <!-- Staged files -->
          <div class="file-section">
            <div class="section-header">
              <span>Staged ({stagedFiles.length})</span>
              {#if stagedFiles.length > 0}
                <button class="action-link" onclick={unstageAll}>Unstage all</button>
              {/if}
            </div>
            {#if stagedFiles.length === 0}
              <div class="empty-section">Nothing staged</div>
            {:else}
              {#each stagedFiles as f (f.path + "-staged")}
                <div
                  class="file-row"
                  class:active={diffFile?.path === f.path && diffFile?.staged}
                  onclick={() => showDiff(f.path, true)}
                  role="button"
                  tabindex="0"
                  onkeydown={(e) => e.key === "Enter" && showDiff(f.path, true)}
                >
                  <span class="status-badge {statusClass(f.status)}">{statusIcon(f.status)}</span>
                  <span class="file-name" title={f.path}>{f.path}</span>
                  <button
                    class="file-action-btn"
                    onclick={(e) => { e.stopPropagation(); unstageFile(f.path); }}
                    title="Unstage"
                  >−</button>
                </div>
              {/each}
            {/if}
          </div>

          <!-- Unstaged files -->
          <div class="file-section">
            <div class="section-header">
              <span>Changes ({unstagedFiles.length + untrackedFiles.length})</span>
              {#if unstagedFiles.length + untrackedFiles.length > 0}
                <button class="action-link" onclick={stageAll}>Stage all</button>
              {/if}
            </div>
            {#if unstagedFiles.length === 0 && untrackedFiles.length === 0}
              <div class="empty-section">Working tree clean</div>
            {:else}
              {#each unstagedFiles as f (f.path + "-unstaged")}
                <div
                  class="file-row"
                  class:active={diffFile?.path === f.path && !diffFile?.staged}
                  onclick={() => showDiff(f.path, false)}
                  role="button"
                  tabindex="0"
                  onkeydown={(e) => e.key === "Enter" && showDiff(f.path, false)}
                >
                  <span class="status-badge {statusClass(f.status)}">{statusIcon(f.status)}</span>
                  <span class="file-name" title={f.path}>{f.path}</span>
                  <button
                    class="file-action-btn stage-btn"
                    onclick={(e) => { e.stopPropagation(); stageFile(f.path); }}
                    title="Stage"
                  >+</button>
                </div>
              {/each}
              {#each untrackedFiles as f (f.path + "-untracked")}
                <div
                  class="file-row"
                  role="button"
                  tabindex="0"
                  onkeydown={() => {}}
                >
                  <span class="status-badge s-added">{statusIcon(f.status)}</span>
                  <span class="file-name" title={f.path}>{f.path}</span>
                  <button
                    class="file-action-btn stage-btn"
                    onclick={(e) => { e.stopPropagation(); stageFile(f.path); }}
                    title="Stage"
                  >+</button>
                </div>
              {/each}
            {/if}
          </div>

          <!-- Commit form -->
          <div class="commit-section">
            <textarea
              class="commit-input"
              placeholder="Commit message…"
              bind:value={commitMessage}
              rows="3"
              onkeydown={(e) => {
                if (e.key === "Enter" && (e.ctrlKey || e.metaKey)) {
                  e.preventDefault();
                  doCommit();
                }
              }}
            ></textarea>
            {#if commitError}
              <div class="commit-error">{commitError}</div>
            {/if}
            <div class="commit-actions">
              <button
                class="commit-btn"
                disabled={!commitMessage.trim() || committing || stagedFiles.length === 0}
                onclick={doCommit}
              >
                {committing ? "Committing…" : "Commit"}
              </button>
              <button
                class="sync-btn"
                onclick={doPull}
                disabled={pushPullStatus === "pushing" || pushPullStatus === "pulling"}
                title="Pull"
              >
                <svg width="13" height="13" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                  <polyline points="1 4 1 10 7 10"/><path d="M3.51 15a9 9 0 1 0 .49-4"/>
                </svg>
                Pull
              </button>
              <button
                class="sync-btn"
                onclick={doPush}
                disabled={pushPullStatus === "pushing" || pushPullStatus === "pulling"}
                title="Push"
              >
                <svg width="13" height="13" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                  <polyline points="23 4 23 10 17 10"/><path d="M20.49 9A9 9 0 1 1 3.51 15"/>
                </svg>
                Push
              </button>
            </div>
            {#if pushPullMsg}
              <div class="push-pull-msg" class:error={pushPullStatus === "error"}>{pushPullMsg}</div>
            {/if}
          </div>
        </div>

        <!-- Right panel: diff / log -->
        <div class="right-panel">
          <div class="panel-tabs">
            <button
              class="panel-tab"
              class:active={activeTab === "diff"}
              onclick={() => switchTab("diff")}
            >Diff</button>
            <button
              class="panel-tab"
              class:active={activeTab === "log"}
              onclick={() => switchTab("log")}
            >Log</button>
            {#if activeTab === "diff" && diffContent}
              <button class="panel-tab-action" onclick={sendDiffToChat}>
                Ask Claude
              </button>
            {/if}
          </div>

          {#if activeTab === "diff"}
            {#if loadingDiff}
              <div class="panel-loading">Loading diff…</div>
            {:else if !diffContent && !diffFile}
              <div class="panel-empty">Click a file to view its diff.</div>
            {:else if !diffContent}
              <div class="panel-empty">No diff available.</div>
            {:else}
              <div class="diff-view">
                <pre class="diff-pre">{@html renderDiff(diffContent)}</pre>
              </div>
            {/if}
          {:else}
            {#if loadingLog}
              <div class="panel-loading">Loading log…</div>
            {:else if commits.length === 0}
              <div class="panel-empty">No commits yet.</div>
            {:else}
              <div class="log-list">
                {#each commits as c (c.hash)}
                  <div class="log-row">
                    <span class="log-hash">{c.short_hash}</span>
                    <span class="log-msg" title={c.message}>{c.message}</span>
                    <span class="log-meta">{c.author} · {c.date}</span>
                  </div>
                {/each}
              </div>
            {/if}
          {/if}
        </div>
      </div>
    {/if}
  {/if}
</div>

<script context="module">
  function renderDiff(text) {
    return text
      .split("\n")
      .map((line) => {
        const escaped = line
          .replace(/&/g, "&amp;")
          .replace(/</g, "&lt;")
          .replace(/>/g, "&gt;");
        if (line.startsWith("+") && !line.startsWith("+++")) {
          return `<span class="diff-add">${escaped}</span>`;
        }
        if (line.startsWith("-") && !line.startsWith("---")) {
          return `<span class="diff-del">${escaped}</span>`;
        }
        if (line.startsWith("@@")) {
          return `<span class="diff-hunk">${escaped}</span>`;
        }
        if (line.startsWith("diff ") || line.startsWith("index ") || line.startsWith("--- ") || line.startsWith("+++ ")) {
          return `<span class="diff-meta">${escaped}</span>`;
        }
        return `<span>${escaped}</span>`;
      })
      .join("\n");
  }
</script>

<style>
  .git-view {
    display: flex;
    flex-direction: column;
    height: 100%;
    background: var(--bg-primary);
    overflow: hidden;
  }

  /* Header */
  .git-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 10px 16px;
    border-bottom: 1px solid var(--border);
    background: var(--bg-secondary);
    flex-shrink: 0;
  }

  .git-header-left {
    display: flex;
    align-items: center;
    gap: 8px;
  }

  .git-icon { color: var(--accent); }

  .git-title {
    font-size: 14px;
    font-weight: 600;
    color: var(--text-primary);
  }

  .branch-badge {
    display: flex;
    align-items: center;
    gap: 4px;
    padding: 2px 8px;
    border-radius: 4px;
    background: var(--bg-tertiary);
    font-size: 12px;
    color: var(--text-secondary);
    font-family: "JetBrains Mono", "Fira Code", monospace;
  }

  .sync-badge {
    font-size: 11px;
    color: var(--text-muted);
    padding: 2px 6px;
    border-radius: 4px;
    background: var(--bg-tertiary);
  }

  .git-header-right {
    display: flex;
    align-items: center;
    gap: 6px;
  }

  .header-btn {
    display: flex;
    align-items: center;
    gap: 5px;
    padding: 5px 10px;
    border-radius: 6px;
    font-size: 12px;
    color: var(--text-secondary);
    transition: background 0.15s, color 0.15s;
  }

  .header-btn:hover {
    background: var(--bg-tertiary);
    color: var(--text-primary);
  }

  .close-btn { font-size: 14px; padding: 5px 8px; }

  /* Repo bar */
  .repo-bar {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 8px 16px;
    border-bottom: 1px solid var(--border);
    background: var(--bg-secondary);
    flex-shrink: 0;
  }

  .repo-select {
    flex: 1;
    padding: 6px 8px;
    background: var(--bg-input);
    border: 1px solid var(--border);
    border-radius: 6px;
    font-size: 13px;
    color: var(--text-primary);
    min-width: 0;
  }

  .repo-placeholder {
    flex: 1;
    font-size: 13px;
    color: var(--text-muted);
  }

  .repo-open-btn {
    display: flex;
    align-items: center;
    gap: 5px;
    padding: 6px 12px;
    border-radius: 6px;
    font-size: 12px;
    background: var(--bg-tertiary);
    color: var(--text-secondary);
    white-space: nowrap;
    transition: background 0.15s, color 0.15s;
  }

  .repo-open-btn:hover {
    background: var(--accent);
    color: white;
  }

  /* Empty states */
  .git-empty {
    flex: 1;
    display: flex;
    align-items: center;
    justify-content: center;
    color: var(--text-muted);
    font-size: 14px;
    padding: 40px;
    text-align: center;
  }

  /* Body */
  .git-body {
    flex: 1;
    display: flex;
    min-height: 0;
    overflow: hidden;
  }

  /* Left panel */
  .left-panel {
    width: 280px;
    min-width: 220px;
    max-width: 320px;
    display: flex;
    flex-direction: column;
    border-right: 1px solid var(--border);
    overflow: hidden;
  }

  .file-section {
    flex: 1;
    display: flex;
    flex-direction: column;
    min-height: 60px;
    border-bottom: 1px solid var(--border);
    overflow: hidden;
  }

  .section-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 6px 10px;
    font-size: 11px;
    font-weight: 600;
    color: var(--text-muted);
    text-transform: uppercase;
    letter-spacing: 0.5px;
    background: var(--bg-secondary);
    flex-shrink: 0;
  }

  .action-link {
    font-size: 11px;
    color: var(--accent);
    text-transform: none;
    font-weight: normal;
    letter-spacing: 0;
  }

  .action-link:hover { text-decoration: underline; }

  .empty-section {
    padding: 8px 10px;
    font-size: 12px;
    color: var(--text-muted);
  }

  .file-row {
    display: flex;
    align-items: center;
    gap: 6px;
    padding: 4px 8px;
    cursor: pointer;
    border-radius: 4px;
    margin: 1px 4px;
    transition: background 0.1s;
  }

  .file-row:hover { background: var(--bg-tertiary); }
  .file-row.active { background: var(--bg-tertiary); }

  .file-row:hover .file-action-btn { opacity: 1; }

  .status-badge {
    font-size: 10px;
    font-weight: 700;
    width: 14px;
    text-align: center;
    font-family: "JetBrains Mono", monospace;
    flex-shrink: 0;
  }

  .s-modified { color: var(--warning, #e0af68); }
  .s-added    { color: var(--success, #9ece6a); }
  .s-deleted  { color: var(--danger, #f7768e); }
  .s-renamed  { color: var(--accent, #7aa2f7); }

  .file-name {
    flex: 1;
    font-size: 12px;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    color: var(--text-secondary);
    font-family: "JetBrains Mono", "Fira Code", monospace;
  }

  .file-action-btn {
    opacity: 0;
    font-size: 14px;
    font-weight: 600;
    color: var(--text-muted);
    width: 18px;
    height: 18px;
    display: flex;
    align-items: center;
    justify-content: center;
    border-radius: 3px;
    flex-shrink: 0;
    transition: background 0.1s, color 0.1s, opacity 0.1s;
  }

  .file-action-btn:hover { background: var(--bg-primary); color: var(--text-primary); }
  .file-action-btn.stage-btn:hover { color: var(--success, #9ece6a); }

  /* Commit section */
  .commit-section {
    padding: 10px;
    border-top: 1px solid var(--border);
    flex-shrink: 0;
    background: var(--bg-secondary);
  }

  .commit-input {
    width: 100%;
    padding: 8px;
    background: var(--bg-input);
    border: 1px solid var(--border);
    border-radius: 6px;
    font-size: 12px;
    color: var(--text-primary);
    resize: none;
    font-family: inherit;
    box-sizing: border-box;
    transition: border-color 0.15s;
  }

  .commit-input:focus {
    outline: none;
    border-color: var(--accent);
  }

  .commit-error {
    font-size: 11px;
    color: var(--danger);
    margin-top: 4px;
    white-space: pre-wrap;
  }

  .commit-actions {
    display: flex;
    gap: 6px;
    margin-top: 8px;
  }

  .commit-btn {
    flex: 1;
    padding: 7px 10px;
    background: var(--accent);
    color: white;
    border-radius: 6px;
    font-size: 12px;
    font-weight: 500;
    transition: background 0.15s, opacity 0.15s;
  }

  .commit-btn:hover:not(:disabled) { background: var(--accent-hover); }
  .commit-btn:disabled { opacity: 0.5; cursor: not-allowed; }

  .sync-btn {
    display: flex;
    align-items: center;
    gap: 4px;
    padding: 7px 10px;
    border-radius: 6px;
    font-size: 12px;
    color: var(--text-secondary);
    background: var(--bg-tertiary);
    transition: background 0.15s, color 0.15s;
  }

  .sync-btn:hover:not(:disabled) { background: var(--bg-primary); color: var(--text-primary); }
  .sync-btn:disabled { opacity: 0.5; cursor: not-allowed; }

  .push-pull-msg {
    margin-top: 6px;
    font-size: 11px;
    color: var(--success);
    white-space: pre-wrap;
    max-height: 60px;
    overflow-y: auto;
  }

  .push-pull-msg.error { color: var(--danger); }

  /* Right panel */
  .right-panel {
    flex: 1;
    display: flex;
    flex-direction: column;
    min-width: 0;
    overflow: hidden;
  }

  .panel-tabs {
    display: flex;
    align-items: center;
    gap: 0;
    border-bottom: 1px solid var(--border);
    background: var(--bg-secondary);
    padding: 0 8px;
    flex-shrink: 0;
  }

  .panel-tab {
    padding: 8px 14px;
    font-size: 13px;
    color: var(--text-muted);
    border-bottom: 2px solid transparent;
    margin-bottom: -1px;
    transition: color 0.15s, border-color 0.15s;
  }

  .panel-tab.active {
    color: var(--text-primary);
    border-bottom-color: var(--accent);
  }

  .panel-tab:hover:not(.active) { color: var(--text-secondary); }

  .panel-tab-action {
    margin-left: auto;
    padding: 4px 10px;
    font-size: 12px;
    color: var(--accent);
    border-radius: 5px;
    transition: background 0.15s;
  }

  .panel-tab-action:hover { background: var(--bg-tertiary); }

  .panel-loading,
  .panel-empty {
    flex: 1;
    display: flex;
    align-items: center;
    justify-content: center;
    font-size: 13px;
    color: var(--text-muted);
    padding: 40px;
  }

  /* Diff view */
  .diff-view {
    flex: 1;
    overflow: auto;
    padding: 8px 0;
  }

  .diff-pre {
    font-family: "JetBrains Mono", "Fira Code", monospace;
    font-size: 12px;
    line-height: 1.6;
    margin: 0;
    padding: 0 8px;
    white-space: pre;
    color: var(--text-secondary);
  }

  .diff-pre :global(.diff-add) { color: var(--success, #9ece6a); display: block; }
  .diff-pre :global(.diff-del) { color: var(--danger, #f7768e); display: block; }
  .diff-pre :global(.diff-hunk) { color: var(--accent, #7aa2f7); display: block; }
  .diff-pre :global(.diff-meta) { color: var(--text-muted); display: block; }
  .diff-pre :global(span) { display: block; }

  /* Log list */
  .log-list {
    flex: 1;
    overflow-y: auto;
    padding: 4px 0;
  }

  .log-row {
    display: grid;
    grid-template-columns: 60px 1fr;
    grid-template-rows: auto auto;
    gap: 0 8px;
    padding: 6px 12px;
    border-bottom: 1px solid var(--border);
    transition: background 0.1s;
  }

  .log-row:hover { background: var(--bg-secondary); }

  .log-hash {
    font-family: "JetBrains Mono", monospace;
    font-size: 11px;
    color: var(--accent);
    grid-row: 1 / 3;
    align-self: center;
  }

  .log-msg {
    font-size: 13px;
    color: var(--text-primary);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .log-meta {
    font-size: 11px;
    color: var(--text-muted);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
</style>
