<script>
  import { invoke } from "@tauri-apps/api/core";
  import { listen } from "@tauri-apps/api/event";
  import { onMount, onDestroy } from "svelte";
  import hljs from "./highlight.js";

  let { onClose } = $props();

  let prompt = $state("");
  let isComparing = $state(false);
  let sessionId = $state(null);
  let responses = $state([]);
  let selectedModels = $state([]);
  let availableModels = $state([]);
  let customEndpoints = $state([]);
  let pastSessions = $state([]);
  let showHistory = $state(false);
  let systemPrompt = $state("");

  let unlistenStream = null;
  let unlistenUsage = null;

  onMount(async () => {
    await loadAvailableModels();
    await loadPastSessions();

    unlistenStream = await listen("comparison-stream", (event) => {
      const { event: evtType, content, response_id, model } = event.payload;
      const idx = responses.findIndex(r => r.id === response_id);
      if (idx === -1) return;

      if (evtType === "delta") {
        responses[idx].content += content;
        responses[idx] = responses[idx]; // trigger reactivity
      } else if (evtType === "done") {
        responses[idx].streaming = false;
        responses[idx] = responses[idx];
      } else if (evtType === "error") {
        responses[idx].streaming = false;
        responses[idx].error = content;
        responses[idx] = responses[idx];
      }
    });

    unlistenUsage = await listen("comparison-usage", (event) => {
      const { response_id, input_tokens, output_tokens, latency_ms, estimated_cost } = event.payload;
      const idx = responses.findIndex(r => r.id === response_id);
      if (idx === -1) return;
      responses[idx].inputTokens = input_tokens;
      responses[idx].outputTokens = output_tokens;
      responses[idx].latencyMs = latency_ms;
      responses[idx].estimatedCost = estimated_cost;
      responses[idx] = responses[idx];
    });
  });

  onDestroy(() => {
    if (unlistenStream) unlistenStream();
    if (unlistenUsage) unlistenUsage();
  });

  async function loadAvailableModels() {
    const models = [];

    // Built-in Anthropic models
    try {
      const key = await invoke("get_api_key");
      if (key) {
        models.push(
          { provider: "anthropic", model: "claude-sonnet-4-6", label: "Claude Sonnet 4.6" },
          { provider: "anthropic", model: "claude-opus-4-6", label: "Claude Opus 4.6" },
          { provider: "anthropic", model: "claude-haiku-4-5-20251001", label: "Claude Haiku 4.5" },
        );
      }
    } catch (_) {}

    // Built-in OpenAI models
    try {
      const key = await invoke("get_openai_api_key");
      if (key) {
        models.push(
          { provider: "openai", model: "gpt-4o", label: "GPT-4o" },
          { provider: "openai", model: "gpt-4o-mini", label: "GPT-4o Mini" },
          { provider: "openai", model: "gpt-4.1", label: "GPT-4.1" },
          { provider: "openai", model: "o3", label: "o3" },
          { provider: "openai", model: "o4-mini", label: "o4-mini" },
        );
      }
    } catch (_) {}

    // Custom endpoints
    try {
      customEndpoints = await invoke("get_custom_endpoints");
      for (const ep of customEndpoints) {
        if (ep.is_enabled && ep.default_model) {
          models.push({
            provider: "custom",
            model: ep.default_model,
            label: `${ep.name}: ${ep.default_model}`,
            endpointId: ep.id,
          });
        }
      }
    } catch (_) {}

    availableModels = models;
    // Pre-select first two if available
    if (models.length >= 2) {
      selectedModels = [models[0], models[1]];
    } else if (models.length === 1) {
      selectedModels = [models[0]];
    }
  }

  async function loadPastSessions() {
    try {
      pastSessions = await invoke("get_comparison_sessions");
    } catch (_) {}
  }

  function toggleModel(m) {
    const idx = selectedModels.findIndex(s => s.provider === m.provider && s.model === m.model && s.endpointId === m.endpointId);
    if (idx >= 0) {
      selectedModels = selectedModels.filter((_, i) => i !== idx);
    } else if (selectedModels.length < 4) {
      selectedModels = [...selectedModels, m];
    }
  }

  function isSelected(m) {
    return selectedModels.some(s => s.provider === m.provider && s.model === m.model && s.endpointId === m.endpointId);
  }

  async function startComparison() {
    if (!prompt.trim() || selectedModels.length < 2) return;
    isComparing = true;
    responses = selectedModels.map(m => ({
      id: crypto.randomUUID(),
      provider: m.provider,
      model: m.model,
      label: m.label,
      content: "",
      streaming: true,
      error: null,
      inputTokens: 0,
      outputTokens: 0,
      latencyMs: 0,
      estimatedCost: 0,
      rating: null,
    }));

    try {
      const targets = selectedModels.map(m => ({
        provider: m.provider,
        model: m.model,
        endpoint_id: m.endpointId || null,
      }));
      const sid = await invoke("send_comparison", {
        prompt: prompt.trim(),
        targets,
        systemPrompt: systemPrompt.trim() || null,
      });
      sessionId = sid;
      // Re-map response IDs from the actual session
      const dbResponses = await invoke("get_comparison_responses", { sessionId: sid });
      for (let i = 0; i < dbResponses.length && i < responses.length; i++) {
        responses[i].id = dbResponses[i].id;
        responses[i] = responses[i];
      }
    } catch (e) {
      for (let r of responses) {
        r.streaming = false;
        r.error = e;
      }
      responses = [...responses];
    }
  }

  async function rateResponse(responseId, rating) {
    try {
      await invoke("rate_comparison_response", { responseId, rating });
      const idx = responses.findIndex(r => r.id === responseId);
      if (idx >= 0) {
        responses[idx].rating = rating;
        responses[idx] = responses[idx];
      }
    } catch (_) {}
  }

  async function loadSession(session) {
    showHistory = false;
    sessionId = session.id;
    prompt = session.prompt;
    isComparing = true;
    try {
      const dbResponses = await invoke("get_comparison_responses", { sessionId: session.id });
      responses = dbResponses.map(r => ({
        id: r.id,
        provider: r.provider,
        model: r.model,
        label: `${r.provider}: ${r.model}`,
        content: r.content,
        streaming: false,
        error: null,
        inputTokens: r.input_tokens,
        outputTokens: r.output_tokens,
        latencyMs: r.latency_ms,
        estimatedCost: r.estimated_cost,
        rating: r.rating,
      }));
    } catch (_) {}
  }

  function resetComparison() {
    isComparing = false;
    sessionId = null;
    responses = [];
    prompt = "";
  }

  function formatCost(cost) {
    if (cost < 0.01) return `$${cost.toFixed(4)}`;
    return `$${cost.toFixed(2)}`;
  }

  function renderMarkdown(text) {
    // Simple code block highlighting
    return text.replace(/```(\w+)?\n([\s\S]*?)```/g, (_, lang, code) => {
      try {
        const highlighted = lang ? hljs.highlight(code.trim(), { language: lang }) : hljs.highlightAuto(code.trim());
        return `<pre><code class="hljs">${highlighted.value}</code></pre>`;
      } catch (_) {
        return `<pre><code>${code.trim()}</code></pre>`;
      }
    }).replace(/\n/g, '<br>');
  }
</script>

<div class="comparison-view">
  <div class="comparison-header">
    <h2>Model Comparison</h2>
    <div class="header-actions">
      <button class="text-btn" onclick={() => { showHistory = !showHistory; }}>
        History ({pastSessions.length})
      </button>
      {#if isComparing}
        <button class="text-btn" onclick={resetComparison}>New Comparison</button>
      {/if}
      <button class="close-btn" onclick={onClose}>
        <svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><path d="M18 6L6 18M6 6l12 12"/></svg>
      </button>
    </div>
  </div>

  {#if showHistory}
    <div class="history-panel">
      {#each pastSessions as session}
        <button class="history-item" onclick={() => loadSession(session)}>
          <span class="history-prompt">{session.prompt.slice(0, 80)}{session.prompt.length > 80 ? '...' : ''}</span>
          <span class="history-date">{new Date(session.created_at).toLocaleDateString()}</span>
        </button>
      {:else}
        <p class="empty-state">No previous comparisons</p>
      {/each}
    </div>
  {/if}

  {#if !isComparing}
    <div class="comparison-setup">
      <div class="model-selector">
        <label>Select 2-4 models to compare:</label>
        <div class="model-chips">
          {#each availableModels as m}
            <button
              class="model-chip"
              class:selected={isSelected(m)}
              onclick={() => toggleModel(m)}
            >{m.label}</button>
          {/each}
        </div>
        {#if availableModels.length === 0}
          <p class="empty-state">No API keys configured. Add keys in Settings to enable comparison.</p>
        {/if}
      </div>

      <div class="prompt-section">
        <textarea
          class="comparison-prompt"
          placeholder="Enter your prompt to compare across models..."
          bind:value={prompt}
          rows="4"
          onkeydown={(e) => { if (e.key === 'Enter' && e.ctrlKey) startComparison(); }}
        ></textarea>
        <details class="system-prompt-section">
          <summary>System prompt (optional)</summary>
          <textarea
            class="system-prompt-input"
            placeholder="Optional system prompt..."
            bind:value={systemPrompt}
            rows="2"
          ></textarea>
        </details>
        <button
          class="compare-btn"
          onclick={startComparison}
          disabled={!prompt.trim() || selectedModels.length < 2}
        >Compare ({selectedModels.length} models)</button>
      </div>
    </div>
  {:else}
    <div class="comparison-prompt-display">
      <strong>Prompt:</strong> {prompt}
    </div>
    <div class="comparison-grid" style="grid-template-columns: repeat({responses.length}, 1fr);">
      {#each responses as resp}
        <div class="response-panel" class:streaming={resp.streaming}>
          <div class="panel-header">
            <span class="model-label">{resp.label || resp.model}</span>
            {#if resp.latencyMs > 0}
              <span class="latency">{(resp.latencyMs / 1000).toFixed(1)}s</span>
            {/if}
          </div>
          <div class="panel-content">
            {#if resp.error}
              <div class="panel-error">{resp.error}</div>
            {:else if resp.content}
              {@html renderMarkdown(resp.content)}
            {:else if resp.streaming}
              <div class="streaming-indicator">Generating...</div>
            {/if}
          </div>
          <div class="panel-footer">
            <div class="token-info">
              {#if resp.inputTokens > 0}
                <span>{resp.inputTokens + resp.outputTokens} tokens</span>
                <span class="cost">{formatCost(resp.estimatedCost)}</span>
              {/if}
            </div>
            <div class="rating-stars">
              {#each [1,2,3,4,5] as star}
                <button
                  class="star-btn"
                  class:active={resp.rating >= star}
                  onclick={() => rateResponse(resp.id, star)}
                  disabled={resp.streaming}
                >{resp.rating >= star ? '★' : '☆'}</button>
              {/each}
            </div>
          </div>
        </div>
      {/each}
    </div>
  {/if}
</div>

<style>
  .comparison-view {
    display: flex;
    flex-direction: column;
    height: 100%;
    padding: 16px;
    overflow: hidden;
    background: var(--bg-primary);
    color: var(--text-primary);
  }

  .comparison-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    margin-bottom: 16px;
    flex-shrink: 0;
  }

  .comparison-header h2 {
    margin: 0;
    font-size: 1.3em;
  }

  .header-actions {
    display: flex;
    gap: 8px;
    align-items: center;
  }

  .text-btn {
    background: none;
    border: 1px solid var(--border-color, #444);
    color: var(--text-secondary);
    padding: 4px 12px;
    border-radius: 6px;
    cursor: pointer;
    font-size: 0.85em;
  }
  .text-btn:hover { background: var(--bg-secondary); }

  .close-btn {
    background: none;
    border: none;
    color: var(--text-secondary);
    cursor: pointer;
    padding: 4px;
    border-radius: 4px;
  }
  .close-btn:hover { background: var(--bg-secondary); }

  .history-panel {
    max-height: 200px;
    overflow-y: auto;
    border: 1px solid var(--border-color, #444);
    border-radius: 8px;
    margin-bottom: 12px;
    flex-shrink: 0;
  }

  .history-item {
    display: flex;
    justify-content: space-between;
    width: 100%;
    padding: 8px 12px;
    border: none;
    background: none;
    color: var(--text-primary);
    cursor: pointer;
    text-align: left;
    border-bottom: 1px solid var(--border-color, #333);
  }
  .history-item:hover { background: var(--bg-secondary); }
  .history-date { color: var(--text-secondary); font-size: 0.85em; }

  .comparison-setup {
    display: flex;
    flex-direction: column;
    gap: 16px;
    max-width: 800px;
    margin: 0 auto;
    width: 100%;
  }

  .model-selector label {
    font-size: 0.9em;
    color: var(--text-secondary);
    margin-bottom: 8px;
    display: block;
  }

  .model-chips {
    display: flex;
    flex-wrap: wrap;
    gap: 8px;
  }

  .model-chip {
    padding: 6px 14px;
    border-radius: 20px;
    border: 1px solid var(--border-color, #444);
    background: var(--bg-secondary);
    color: var(--text-primary);
    cursor: pointer;
    font-size: 0.85em;
    transition: all 0.15s;
  }
  .model-chip.selected {
    background: var(--accent-color, #6366f1);
    border-color: var(--accent-color, #6366f1);
    color: white;
  }
  .model-chip:hover:not(.selected) { border-color: var(--accent-color, #6366f1); }

  .prompt-section {
    display: flex;
    flex-direction: column;
    gap: 8px;
  }

  .comparison-prompt, .system-prompt-input {
    width: 100%;
    padding: 12px;
    border: 1px solid var(--border-color, #444);
    border-radius: 8px;
    background: var(--bg-secondary);
    color: var(--text-primary);
    font-size: 0.95em;
    resize: vertical;
    font-family: inherit;
  }
  .comparison-prompt:focus, .system-prompt-input:focus {
    outline: none;
    border-color: var(--accent-color, #6366f1);
  }

  .system-prompt-section {
    font-size: 0.85em;
    color: var(--text-secondary);
  }
  .system-prompt-section summary { cursor: pointer; }

  .compare-btn {
    padding: 10px 24px;
    background: var(--accent-color, #6366f1);
    color: white;
    border: none;
    border-radius: 8px;
    font-size: 0.95em;
    cursor: pointer;
    align-self: flex-end;
  }
  .compare-btn:disabled { opacity: 0.5; cursor: not-allowed; }
  .compare-btn:hover:not(:disabled) { filter: brightness(1.1); }

  .comparison-prompt-display {
    padding: 8px 12px;
    background: var(--bg-secondary);
    border-radius: 8px;
    margin-bottom: 12px;
    font-size: 0.9em;
    flex-shrink: 0;
    max-height: 60px;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .comparison-grid {
    display: grid;
    gap: 12px;
    flex: 1;
    min-height: 0;
    overflow: hidden;
  }

  .response-panel {
    display: flex;
    flex-direction: column;
    border: 1px solid var(--border-color, #444);
    border-radius: 10px;
    overflow: hidden;
    min-height: 0;
  }
  .response-panel.streaming { border-color: var(--accent-color, #6366f1); }

  .panel-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: 8px 12px;
    background: var(--bg-secondary);
    border-bottom: 1px solid var(--border-color, #333);
    flex-shrink: 0;
  }

  .model-label { font-weight: 600; font-size: 0.85em; }
  .latency { font-size: 0.8em; color: var(--text-secondary); }

  .panel-content {
    flex: 1;
    overflow-y: auto;
    padding: 12px;
    font-size: 0.9em;
    line-height: 1.6;
  }

  .panel-content :global(pre) {
    background: var(--bg-primary);
    border-radius: 6px;
    padding: 8px;
    overflow-x: auto;
    font-size: 0.85em;
  }

  .panel-error { color: #ef4444; }

  .streaming-indicator {
    color: var(--text-secondary);
    animation: pulse 1.5s infinite;
  }

  @keyframes pulse {
    0%, 100% { opacity: 1; }
    50% { opacity: 0.4; }
  }

  .panel-footer {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: 6px 12px;
    border-top: 1px solid var(--border-color, #333);
    flex-shrink: 0;
    font-size: 0.8em;
  }

  .token-info {
    display: flex;
    gap: 8px;
    color: var(--text-secondary);
  }
  .cost { color: var(--accent-color, #6366f1); font-weight: 500; }

  .rating-stars { display: flex; gap: 2px; }
  .star-btn {
    background: none;
    border: none;
    cursor: pointer;
    font-size: 1.1em;
    color: var(--text-secondary);
    padding: 0 2px;
  }
  .star-btn.active { color: #f59e0b; }
  .star-btn:disabled { cursor: default; }

  .empty-state {
    color: var(--text-secondary);
    font-size: 0.9em;
    text-align: center;
    padding: 12px;
  }
</style>
