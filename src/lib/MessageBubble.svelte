<script>
  import { marked } from "marked";
  import hljs from "highlight.js";
  import katex from "katex";
  import "katex/dist/katex.min.css";
  import { tick } from "svelte";

  let { role, content, isStreaming, onEdit, onRegenerate, onFork, messageId, onPreviewArtifact, onRetry } = $props();
  let messageEl;
  let isEditing = $state(false);
  let editText = $state("");

  // Configure marked to use highlight.js
  marked.setOptions({
    highlight(code, lang) {
      if (lang && hljs.getLanguage(lang)) {
        return hljs.highlight(code, { language: lang }).value;
      }
      return hljs.highlightAuto(code).value;
    },
  });

  function renderLatex(text) {
    if (!text) return "";
    // Display math: $$...$$ or \[...\]
    text = text.replace(/\$\$([\s\S]*?)\$\$/g, (_, math) => {
      try {
        return katex.renderToString(math.trim(), { displayMode: true, throwOnError: false });
      } catch { return _; }
    });
    text = text.replace(/\\\[([\s\S]*?)\\\]/g, (_, math) => {
      try {
        return katex.renderToString(math.trim(), { displayMode: true, throwOnError: false });
      } catch { return _; }
    });
    // Inline math: $...$ (not $$) or \(...\)
    text = text.replace(/(?<!\$)\$(?!\$)((?:[^$\\]|\\.)+?)\$/g, (_, math) => {
      try {
        return katex.renderToString(math.trim(), { displayMode: false, throwOnError: false });
      } catch { return _; }
    });
    text = text.replace(/\\\(([\s\S]*?)\\\)/g, (_, math) => {
      try {
        return katex.renderToString(math.trim(), { displayMode: false, throwOnError: false });
      } catch { return _; }
    });
    return text;
  }

  let renderedHtml = $derived(
    role === "error" ? content : marked.parse(renderLatex(content || ""))
  );

  function startEdit() {
    editText = content;
    isEditing = true;
  }

  function cancelEdit() {
    isEditing = false;
    editText = "";
  }

  function submitEdit() {
    if (editText.trim() && onEdit) {
      onEdit(messageId, editText.trim());
    }
    isEditing = false;
    editText = "";
  }

  function handleEditKeydown(e) {
    if (e.key === "Enter" && !e.shiftKey) {
      e.preventDefault();
      submitEdit();
    }
    if (e.key === "Escape") {
      cancelEdit();
    }
  }

  const PREVIEWABLE_LANGS = ["html", "svg"];

  function detectLanguage(block) {
    const code = block.querySelector("code");
    if (!code) return null;
    for (const cls of code.classList) {
      const match = cls.match(/^(?:language|hljs)-(\w+)$/);
      if (match) return match[1].toLowerCase();
    }
    // Fallback: detect by content
    const text = code.textContent.trim();
    if (text.startsWith("<svg")) return "svg";
    if (text.includes("<!DOCTYPE") || text.includes("<html")) return "html";
    if (text.startsWith("<") && text.endsWith(">") && text.includes("<div")) return "html";
    return null;
  }

  // Add copy buttons (and preview buttons for HTML/SVG) to code blocks after render
  async function attachCopyButtons() {
    await tick();
    if (!messageEl) return;
    const blocks = messageEl.querySelectorAll("pre");
    for (const block of blocks) {
      if (block.querySelector(".copy-btn")) continue;
      const code = block.querySelector("code");
      const codeText = code ? code.textContent : block.textContent;

      const btn = document.createElement("button");
      btn.className = "copy-btn";
      btn.textContent = "Copy";
      btn.addEventListener("click", async () => {
        await navigator.clipboard.writeText(codeText);
        btn.textContent = "Copied!";
        setTimeout(() => (btn.textContent = "Copy"), 1500);
      });
      block.style.position = "relative";
      block.appendChild(btn);

      // Add preview button for previewable languages
      const lang = detectLanguage(block);
      if (lang && PREVIEWABLE_LANGS.includes(lang) && onPreviewArtifact) {
        const previewBtn = document.createElement("button");
        previewBtn.className = "preview-btn";
        previewBtn.textContent = "Preview";
        previewBtn.addEventListener("click", () => {
          onPreviewArtifact({ code: codeText, language: lang });
        });
        block.appendChild(previewBtn);
      }
    }
  }

  $effect(() => {
    renderedHtml;
    attachCopyButtons();
  });
</script>

<div class="message" class:user={role === "user"} class:assistant={role === "assistant"} class:error={role === "error"} bind:this={messageEl} role="article" aria-label="{role === 'user' ? 'Your message' : role === 'assistant' ? 'Claude response' : 'Error message'}">
  <div class="message-header">
    {#if role === "user"}
      <span class="role-label">You</span>
    {:else if role === "assistant"}
      <span class="role-label">Claude</span>
      {#if isStreaming}
        <span class="streaming-indicator"></span>
      {/if}
    {:else if role === "error"}
      <span class="role-label error-label">Error</span>
    {/if}
  </div>

  {#if isEditing}
    <div class="edit-area">
      <textarea
        bind:value={editText}
        onkeydown={handleEditKeydown}
        rows="3"
      ></textarea>
      <div class="edit-actions">
        <button class="edit-save" onclick={submitEdit}>Save & Send</button>
        <button class="edit-cancel" onclick={cancelEdit}>Cancel</button>
      </div>
    </div>
  {:else}
    <div class="message-content">
      {#if role === "error"}
        <p class="error-text">{content}</p>
        {#if onRetry}
          <button class="retry-btn" onclick={onRetry} aria-label="Retry sending message">Retry</button>
        {/if}
      {:else}
        {@html renderedHtml}
      {/if}
    </div>

    {#if !isStreaming && role !== "error"}
      <div class="message-actions">
        {#if role === "user" && onEdit}
          <button class="action-btn" onclick={startEdit} title="Edit message" aria-label="Edit message">
            <svg width="13" height="13" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
              <path d="M11 4H4a2 2 0 00-2 2v14a2 2 0 002 2h14a2 2 0 002-2v-7"/>
              <path d="M18.5 2.5a2.121 2.121 0 013 3L12 15l-4 1 1-4 9.5-9.5z"/>
            </svg>
          </button>
        {/if}
        {#if role === "assistant" && onRegenerate}
          <button class="action-btn" onclick={() => onRegenerate(messageId)} title="Regenerate" aria-label="Regenerate response">
            <svg width="13" height="13" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
              <polyline points="23 4 23 10 17 10"/>
              <path d="M20.49 15a9 9 0 11-2.12-9.36L23 10"/>
            </svg>
          </button>
        {/if}
        {#if onFork}
          <button class="action-btn" onclick={() => onFork(messageId)} title="Fork conversation from here" aria-label="Fork conversation">
            <svg width="13" height="13" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
              <circle cx="18" cy="18" r="3"/><circle cx="6" cy="6" r="3"/><circle cx="18" cy="6" r="3"/>
              <path d="M6 9v6c0 1.66 1.34 3 3 3h3"/><line x1="18" y1="9" x2="18" y2="15"/>
            </svg>
          </button>
        {/if}
      </div>
    {/if}
  {/if}
</div>

<style>
  .message {
    max-width: 85%;
    padding: 12px 16px;
    border-radius: 12px;
    line-height: 1.6;
  }

  .message.user {
    align-self: flex-end;
    background: var(--user-bubble);
  }

  .message.assistant {
    align-self: flex-start;
    background: var(--assistant-bubble);
    border: 1px solid var(--border);
  }

  .message.error {
    align-self: center;
    background: rgba(233, 69, 96, 0.1);
    border: 1px solid var(--danger);
    max-width: 90%;
  }

  .message-header {
    display: flex;
    align-items: center;
    gap: 8px;
    margin-bottom: 4px;
  }

  .role-label {
    font-size: 12px;
    font-weight: 600;
    color: var(--text-muted);
    text-transform: uppercase;
    letter-spacing: 0.5px;
  }

  .error-label {
    color: var(--danger);
  }

  .error-text {
    color: var(--danger);
    font-size: 13px;
  }

  .retry-btn {
    margin-top: 8px;
    padding: 4px 14px;
    border-radius: 6px;
    font-size: 12px;
    background: var(--danger);
    color: white;
    transition: opacity 0.15s;
  }

  .retry-btn:hover {
    opacity: 0.85;
  }

  .streaming-indicator {
    width: 6px;
    height: 6px;
    background: var(--accent);
    border-radius: 50%;
    animation: pulse 1s infinite;
  }

  @keyframes pulse {
    0%, 100% { opacity: 1; }
    50% { opacity: 0.3; }
  }

  .message-actions {
    display: flex;
    gap: 4px;
    margin-top: 6px;
    opacity: 0;
    transition: opacity 0.15s;
  }

  .message:hover .message-actions {
    opacity: 1;
  }

  .action-btn {
    color: var(--text-muted);
    padding: 3px 6px;
    border-radius: 4px;
    transition: color 0.15s, background 0.15s;
    display: flex;
    align-items: center;
  }

  .action-btn:hover {
    color: var(--text-primary);
    background: rgba(255, 255, 255, 0.08);
  }

  .edit-area {
    display: flex;
    flex-direction: column;
    gap: 8px;
  }

  .edit-area textarea {
    background: rgba(0, 0, 0, 0.2);
    border: 1px solid var(--border);
    border-radius: 8px;
    padding: 8px;
    color: var(--text-primary);
    font-family: inherit;
    font-size: inherit;
    resize: vertical;
    outline: none;
    min-height: 60px;
  }

  .edit-area textarea:focus {
    border-color: var(--accent);
  }

  .edit-actions {
    display: flex;
    gap: 6px;
  }

  .edit-save {
    padding: 4px 12px;
    background: var(--accent);
    color: white;
    border-radius: 6px;
    font-size: 12px;
  }

  .edit-save:hover {
    background: var(--accent-hover);
  }

  .edit-cancel {
    padding: 4px 12px;
    border: 1px solid var(--border);
    border-radius: 6px;
    font-size: 12px;
    color: var(--text-secondary);
  }

  .edit-cancel:hover {
    background: var(--bg-tertiary);
  }

  .message-content :global(pre) {
    background: var(--code-bg);
    padding: 12px;
    padding-top: 36px;
    border-radius: 8px;
    overflow-x: auto;
    margin: 8px 0;
    position: relative;
  }

  .message-content :global(code) {
    font-family: "JetBrains Mono", "Fira Code", monospace;
    font-size: 13px;
  }

  .message-content :global(p code) {
    background: var(--code-inline-bg);
    padding: 2px 6px;
    border-radius: 4px;
  }

  .message-content :global(.copy-btn) {
    position: absolute;
    top: 6px;
    right: 6px;
    padding: 3px 10px;
    font-size: 11px;
    font-family: inherit;
    background: rgba(255, 255, 255, 0.1);
    color: var(--text-muted);
    border: none;
    border-radius: 4px;
    cursor: pointer;
    transition: background 0.15s, color 0.15s;
  }

  .message-content :global(.copy-btn:hover) {
    background: rgba(255, 255, 255, 0.2);
    color: var(--text-primary);
  }

  .message-content :global(.preview-btn) {
    position: absolute;
    top: 6px;
    right: 60px;
    padding: 3px 10px;
    font-size: 11px;
    font-family: inherit;
    background: rgba(233, 69, 96, 0.2);
    color: var(--accent);
    border: none;
    border-radius: 4px;
    cursor: pointer;
    transition: background 0.15s, color 0.15s;
  }

  .message-content :global(.preview-btn:hover) {
    background: rgba(233, 69, 96, 0.4);
    color: white;
  }

  .message-content :global(p) {
    margin-bottom: 8px;
  }

  .message-content :global(p:last-child) {
    margin-bottom: 0;
  }

  .message-content :global(ul), .message-content :global(ol) {
    padding-left: 20px;
    margin: 8px 0;
  }

  .message-content :global(a) {
    color: var(--accent);
    text-decoration: none;
  }

  .message-content :global(a:hover) {
    text-decoration: underline;
  }

  /* highlight.js token colors for dark theme */
  .message-content :global(.hljs-keyword) { color: #c792ea; }
  .message-content :global(.hljs-string) { color: #c3e88d; }
  .message-content :global(.hljs-number) { color: #f78c6c; }
  .message-content :global(.hljs-built_in) { color: #82aaff; }
  .message-content :global(.hljs-function) { color: #82aaff; }
  .message-content :global(.hljs-title) { color: #82aaff; }
  .message-content :global(.hljs-params) { color: #e0e0e0; }
  .message-content :global(.hljs-comment) { color: #546e7a; font-style: italic; }
  .message-content :global(.hljs-meta) { color: #ffcb6b; }
  .message-content :global(.hljs-attr) { color: #ffcb6b; }
  .message-content :global(.hljs-attribute) { color: #c792ea; }
  .message-content :global(.hljs-tag) { color: #f07178; }
  .message-content :global(.hljs-name) { color: #f07178; }
  .message-content :global(.hljs-selector-class) { color: #ffcb6b; }
  .message-content :global(.hljs-selector-id) { color: #82aaff; }
  .message-content :global(.hljs-variable) { color: #f07178; }
  .message-content :global(.hljs-type) { color: #ffcb6b; }
  .message-content :global(.hljs-literal) { color: #ff5370; }
  .message-content :global(.hljs-symbol) { color: #c792ea; }
  .message-content :global(.hljs-bullet) { color: #c3e88d; }
  .message-content :global(.hljs-link) { color: #82aaff; }
</style>
