<script>
  import { invoke } from "@tauri-apps/api/core";
  import { listen } from "@tauri-apps/api/event";
  import { onMount, onDestroy } from "svelte";

  let { onClose } = $props();

  // State
  let task = $state("");
  let isRunning = $state(false);
  let events = $state([]);
  let latestScreenshot = $state(null);
  let availability = $state(null);
  let cuModel = $state("claude-opus-4-6");
  let confirmDialog = $state(null); // { resolve }
  let logEl;

  let unlisten = null;

  onMount(async () => {
    availability = await invoke("check_computer_use_available");
    cuModel = await invoke("get_cu_model");

    unlisten = await listen("computer-use-event", (event) => {
      const e = event.payload;
      events = [...events, { ...e, ts: Date.now() }];
      if (e.screenshot) latestScreenshot = e.screenshot;
      // Auto-scroll log
      setTimeout(() => {
        if (logEl) logEl.scrollTop = logEl.scrollHeight;
      }, 0);
      if (e.event_type === "done" || e.event_type === "error") {
        isRunning = false;
      }
    });
  });

  onDestroy(() => {
    unlisten?.then?.(fn => fn());
    if (isRunning) invoke("stop_computer_use");
  });

  async function start() {
    if (!task.trim() || isRunning) return;
    events = [];
    latestScreenshot = null;
    isRunning = true;
    try {
      await invoke("run_computer_use", { task: task.trim() });
    } catch (e) {
      events = [...events, { event_type: "error", content: String(e), ts: Date.now() }];
      isRunning = false;
    }
  }

  async function stop() {
    await invoke("stop_computer_use");
    isRunning = false;
  }

  async function saveModel() {
    await invoke("set_cu_model", { model: cuModel });
  }

  function handleKeydown(e) {
    if (e.key === "Enter" && (e.ctrlKey || e.metaKey)) start();
    if (e.key === "Escape") onClose();
  }

  function eventIcon(type) {
    switch (type) {
      case "action":     return "⚡";
      case "screenshot": return "📷";
      case "text":       return "💬";
      case "done":       return "✓";
      case "error":      return "✗";
      default:           return "·";
    }
  }

  function clearEvents() {
    events = [];
    latestScreenshot = null;
  }
</script>

<div class="cu-layout" onkeydown={handleKeydown}>
  <!-- Header -->
  <div class="cu-header">
    <div class="cu-title">
      <svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"
           stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">
        <rect x="2" y="3" width="20" height="14" rx="2"/>
        <path d="M8 21h8M12 17v4"/>
      </svg>
      <span>Computer Use</span>
      {#if isRunning}
        <span class="running-badge" aria-label="Running">Running</span>
      {/if}
    </div>
    <button class="close-btn" onclick={onClose} aria-label="Close computer use">
      <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
        <line x1="18" y1="6" x2="6" y2="18"/><line x1="6" y1="6" x2="18" y2="18"/>
      </svg>
    </button>
  </div>

  <!-- Availability warning -->
  {#if availability && !availability.available}
    <div class="cu-warning" role="alert">
      <strong>Setup required:</strong> {availability.message}
      <div class="install-hint">
        {#if !availability.xdotool}<code>sudo apt install xdotool</code>{/if}
        {#if !availability.screenshot}<code>sudo apt install scrot</code>{/if}
      </div>
    </div>
  {/if}

  <div class="cu-body">
    <!-- Left: controls + log -->
    <div class="cu-left">
      <!-- Task input -->
      <div class="task-section">
        <label class="field-label" for="cu-task">Task</label>
        <textarea
          id="cu-task"
          bind:value={task}
          placeholder="Describe what you want Claude to do on your screen…&#10;&#10;Examples:&#10;• Open a terminal and run 'ls -la'&#10;• Find the Firefox icon and launch it&#10;• Take a screenshot and describe what's on screen"
          rows="5"
          disabled={isRunning}
          aria-label="Task for Claude to perform"
        ></textarea>
        <p class="hint">Ctrl+Enter to run · Claude will take screenshots and control your desktop</p>
      </div>

      <!-- Model + controls row -->
      <div class="controls-row">
        <div class="model-field">
          <label class="field-label" for="cu-model">Model</label>
          <select id="cu-model" bind:value={cuModel} onchange={saveModel} disabled={isRunning}
                  aria-label="Model for computer use">
            <option value="claude-opus-4-6">claude-opus-4-6</option>
            <option value="claude-sonnet-4-6">claude-sonnet-4-6</option>
            <option value="claude-haiku-4-5-20251001">claude-haiku-4-5</option>
          </select>
        </div>
        <div class="btn-row">
          {#if isRunning}
            <button class="stop-btn" onclick={stop} aria-label="Stop computer use">
              <svg width="14" height="14" viewBox="0 0 24 24" fill="currentColor">
                <rect x="6" y="6" width="12" height="12"/>
              </svg>
              Stop
            </button>
          {:else}
            <button class="run-btn" onclick={start}
                    disabled={!task.trim() || (availability && !availability.available)}
                    aria-label="Run task">
              <svg width="14" height="14" viewBox="0 0 24 24" fill="currentColor">
                <polygon points="5 3 19 12 5 21 5 3"/>
              </svg>
              Run
            </button>
          {/if}
          {#if events.length > 0}
            <button class="clear-btn" onclick={clearEvents} disabled={isRunning} aria-label="Clear log">
              Clear
            </button>
          {/if}
        </div>
      </div>

      <!-- Action log -->
      <div class="log-section">
        <div class="log-header">
          <span class="field-label">Action Log</span>
          {#if events.length > 0}
            <span class="event-count">{events.length} events</span>
          {/if}
        </div>
        <div class="log" bind:this={logEl} role="log" aria-live="polite" aria-label="Computer use action log">
          {#if events.length === 0}
            <span class="log-empty">Actions will appear here…</span>
          {:else}
            {#each events as event (event.ts)}
              <div class="log-entry" class:error={event.event_type === "error"}
                   class:done={event.event_type === "done"}
                   class:action={event.event_type === "action"}>
                <span class="log-icon" aria-hidden="true">{eventIcon(event.event_type)}</span>
                <span class="log-iter">{event.iteration > 0 ? `[${event.iteration}]` : ""}</span>
                <span class="log-content">{event.content}</span>
              </div>
            {/each}
          {/if}
        </div>
      </div>
    </div>

    <!-- Right: live screenshot -->
    <div class="cu-right">
      <div class="screenshot-header field-label">
        Live Screen
        {#if isRunning}
          <span class="pulse-dot" aria-label="Updating"></span>
        {/if}
      </div>
      <div class="screenshot-panel" aria-label="Latest screenshot from Claude">
        {#if latestScreenshot}
          <img
            src="data:image/png;base64,{latestScreenshot}"
            alt="Current screen state"
            class="screenshot-img"
          />
        {:else}
          <div class="screenshot-empty">
            <svg width="40" height="40" viewBox="0 0 24 24" fill="none" stroke="currentColor"
                 stroke-width="1.5" opacity="0.3" aria-hidden="true">
              <rect x="2" y="3" width="20" height="14" rx="2"/>
              <path d="M8 21h8M12 17v4"/>
            </svg>
            <p>Screenshots will appear here as Claude works</p>
          </div>
        {/if}
      </div>
    </div>
  </div>
</div>

<style>
  .cu-layout {
    display: flex;
    flex-direction: column;
    height: 100%;
    background: var(--bg-primary);
    overflow: hidden;
  }

  /* Header */
  .cu-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 12px 20px;
    border-bottom: 1px solid var(--border);
    background: var(--bg-secondary);
    flex-shrink: 0;
  }

  .cu-title {
    display: flex;
    align-items: center;
    gap: 8px;
    font-weight: 600;
    font-size: 15px;
  }

  .running-badge {
    font-size: 10px;
    font-weight: 700;
    padding: 2px 8px;
    border-radius: 10px;
    background: rgba(233, 69, 96, 0.2);
    color: var(--danger);
    text-transform: uppercase;
    letter-spacing: 0.5px;
    animation: pulse-badge 1.4s ease-in-out infinite;
  }

  @keyframes pulse-badge {
    0%, 100% { opacity: 1; }
    50% { opacity: 0.4; }
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
  .cu-warning {
    margin: 12px 20px 0;
    padding: 10px 14px;
    border-radius: 8px;
    background: rgba(233, 69, 96, 0.1);
    border: 1px solid rgba(233, 69, 96, 0.3);
    font-size: 13px;
    color: var(--text-primary);
    flex-shrink: 0;
  }

  .install-hint {
    margin-top: 6px;
    display: flex;
    gap: 8px;
    flex-wrap: wrap;
  }

  .install-hint code {
    background: var(--code-bg);
    padding: 2px 8px;
    border-radius: 4px;
    font-size: 12px;
  }

  /* Body */
  .cu-body {
    display: grid;
    grid-template-columns: 420px 1fr;
    gap: 0;
    flex: 1;
    overflow: hidden;
  }

  /* Left panel */
  .cu-left {
    display: flex;
    flex-direction: column;
    gap: 14px;
    padding: 16px 20px;
    border-right: 1px solid var(--border);
    overflow-y: auto;
  }

  .field-label {
    font-size: 11px;
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 0.5px;
    color: var(--text-muted);
    display: block;
    margin-bottom: 6px;
  }

  .task-section textarea {
    width: 100%;
    box-sizing: border-box;
    background: var(--bg-input);
    border: 1px solid var(--border);
    border-radius: 8px;
    padding: 10px 12px;
    color: var(--text-primary);
    font-family: inherit;
    font-size: 13px;
    resize: vertical;
    line-height: 1.5;
  }

  .task-section textarea:focus {
    border-color: var(--accent);
    outline: none;
  }

  .hint {
    font-size: 11px;
    color: var(--text-muted);
    margin-top: 4px;
  }

  .controls-row {
    display: flex;
    align-items: flex-end;
    gap: 12px;
    flex-wrap: wrap;
  }

  .model-field {
    flex: 1;
    min-width: 160px;
  }

  .model-field select {
    width: 100%;
    background: var(--bg-input);
    border: 1px solid var(--border);
    border-radius: 6px;
    padding: 7px 10px;
    color: var(--text-primary);
    font-size: 12px;
    cursor: pointer;
    appearance: none;
    -webkit-appearance: none;
  }

  .btn-row {
    display: flex;
    gap: 6px;
  }

  .run-btn, .stop-btn {
    display: flex;
    align-items: center;
    gap: 6px;
    padding: 7px 16px;
    border-radius: 6px;
    font-size: 13px;
    font-weight: 600;
    cursor: pointer;
  }

  .run-btn {
    background: var(--accent);
    color: #fff;
  }

  .run-btn:hover:not(:disabled) { background: var(--accent-hover); }
  .run-btn:disabled { opacity: 0.4; cursor: not-allowed; }

  .stop-btn {
    background: rgba(233, 69, 96, 0.2);
    color: var(--danger);
    border: 1px solid rgba(233, 69, 96, 0.4);
  }

  .stop-btn:hover { background: rgba(233, 69, 96, 0.3); }

  .clear-btn {
    padding: 7px 12px;
    border: 1px solid var(--border);
    border-radius: 6px;
    font-size: 12px;
    color: var(--text-muted);
  }

  .clear-btn:hover:not(:disabled) { background: var(--bg-tertiary); color: var(--text-primary); }
  .clear-btn:disabled { opacity: 0.4; cursor: not-allowed; }

  /* Log */
  .log-section { display: flex; flex-direction: column; flex: 1; min-height: 180px; }

  .log-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    margin-bottom: 6px;
  }

  .event-count {
    font-size: 11px;
    color: var(--text-muted);
  }

  .log {
    flex: 1;
    overflow-y: auto;
    background: var(--bg-secondary);
    border: 1px solid var(--border);
    border-radius: 8px;
    padding: 8px;
    font-size: 12px;
    font-family: "JetBrains Mono", "Fira Code", monospace;
    min-height: 160px;
    max-height: 320px;
  }

  .log-empty { color: var(--text-muted); font-style: italic; }

  .log-entry {
    display: flex;
    gap: 6px;
    padding: 3px 0;
    line-height: 1.5;
    border-bottom: 1px solid rgba(255,255,255,0.04);
  }

  .log-entry:last-child { border-bottom: none; }

  .log-entry.error { color: var(--danger); }
  .log-entry.done { color: var(--success); }
  .log-entry.action { color: var(--accent); }

  .log-icon { flex-shrink: 0; width: 14px; text-align: center; }
  .log-iter { flex-shrink: 0; color: var(--text-muted); min-width: 28px; }
  .log-content { flex: 1; white-space: pre-wrap; word-break: break-word; }

  /* Right panel — screenshot */
  .cu-right {
    display: flex;
    flex-direction: column;
    padding: 16px 20px;
    overflow: hidden;
  }

  .screenshot-header {
    display: flex;
    align-items: center;
    gap: 8px;
    margin-bottom: 8px;
  }

  .pulse-dot {
    width: 7px;
    height: 7px;
    background: var(--accent);
    border-radius: 50%;
    animation: pulse-badge 1s ease-in-out infinite;
  }

  .screenshot-panel {
    flex: 1;
    border: 1px solid var(--border);
    border-radius: 8px;
    overflow: hidden;
    background: var(--bg-secondary);
    display: flex;
    align-items: center;
    justify-content: center;
  }

  .screenshot-img {
    width: 100%;
    height: 100%;
    object-fit: contain;
    display: block;
  }

  .screenshot-empty {
    text-align: center;
    color: var(--text-muted);
    padding: 40px;
  }

  .screenshot-empty p {
    font-size: 13px;
    margin-top: 12px;
  }
</style>
