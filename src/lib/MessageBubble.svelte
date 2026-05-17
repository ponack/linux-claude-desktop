<script>
  import { marked } from "marked";
  import hljs from "./highlight.js";
  import katex from "katex";
  import "katex/dist/katex.min.css";
  import { invoke } from "@tauri-apps/api/core";
  import { tick } from "svelte";
  import Icon from "./Icon.svelte";

  let { role, content, isStreaming, onEdit, onRegenerate, onFork, messageId, onPreviewArtifact, onRetry, ttsEnabled = false, ttsRate = 100 } = $props();
  let messageEl;
  let isEditing = $state(false);
  let editText = $state("");

  // Annotations
  let annotations = $state([]);
  let showAnnotationInput = $state(false);
  let annotationText = $state("");
  let annotationInputEl;

  async function loadAnnotations() {
    if (!messageId) return;
    try {
      annotations = await invoke("get_message_annotations", { messageId });
    } catch { annotations = []; }
  }

  async function saveAnnotation() {
    const text = annotationText.trim();
    if (!text || !messageId) { showAnnotationInput = false; annotationText = ""; return; }
    try {
      const ann = await invoke("add_message_annotation", { messageId, content: text });
      annotations = [...annotations, ann];
    } catch (e) { console.error("Failed to save annotation:", e); }
    annotationText = "";
    showAnnotationInput = false;
  }

  async function removeAnnotation(id) {
    try {
      await invoke("delete_message_annotation", { id });
      annotations = annotations.filter(a => a.id !== id);
    } catch (e) { console.error("Failed to delete annotation:", e); }
  }

  async function openAnnotationInput() {
    showAnnotationInput = true;
    await tick();
    annotationInputEl?.focus();
  }

  function onAnnotationKeydown(e) {
    if (e.key === "Enter" && !e.shiftKey) { e.preventDefault(); saveAnnotation(); }
    if (e.key === "Escape") { showAnnotationInput = false; annotationText = ""; }
  }

  $effect(() => {
    if (messageId) loadAnnotations();
  });

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

  // Cache markdown rendering to avoid re-parsing unchanged content
  let cachedContent = "";
  let cachedHtml = "";

  let renderedHtml = $derived.by(() => {
    if (role === "error") return content;
    const raw = content || "";
    if (raw === cachedContent) return cachedHtml;
    cachedContent = raw;
    cachedHtml = marked.parse(renderLatex(raw));
    return cachedHtml;
  });

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

  const ARTIFACT_LANGS = [
    "html", "svg", "javascript", "js", "typescript", "ts", "jsx", "tsx",
    "python", "py", "rust", "rs", "css", "json", "markdown", "md", "mermaid",
    "react", "go", "java", "c", "cpp", "ruby", "php", "bash", "sh", "sql",
    "yaml", "toml", "xml",
  ];

  const RUNNABLE_LANGS = new Set([
    "python", "python3", "py",
    "javascript", "js", "node",
    "bash", "sh",
    "ruby", "rb",
  ]);

  function escapeHtml(str) {
    return str
      .replace(/&/g, "&amp;")
      .replace(/</g, "&lt;")
      .replace(/>/g, "&gt;");
  }

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
    if (/^(graph|sequenceDiagram|classDiagram|stateDiagram|erDiagram|gantt|pie|flowchart)\b/.test(text)) return "mermaid";
    if (/^import React|^export default function|^const \w+ = \(\) =>/.test(text)) return "react";
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
      btn.type = "button";
      btn.setAttribute("aria-label", "Copy code to clipboard");
      btn.textContent = "Copy";
      btn.addEventListener("click", async () => {
        await navigator.clipboard.writeText(codeText);
        btn.textContent = "Copied!";
        btn.setAttribute("aria-label", "Code copied");
        setTimeout(() => {
          btn.textContent = "Copy";
          btn.setAttribute("aria-label", "Copy code to clipboard");
        }, 1500);
      });
      block.style.position = "relative";
      block.appendChild(btn);

      // Add "Open as Artifact" button for substantial code blocks
      const lang = detectLanguage(block);
      const lineCount = codeText.split("\n").length;
      if (lang && onPreviewArtifact && (ARTIFACT_LANGS.includes(lang) || lineCount >= 10)) {
        const previewBtn = document.createElement("button");
        previewBtn.className = "preview-btn";
        previewBtn.type = "button";
        previewBtn.setAttribute("aria-label", "Open code as artifact");
        previewBtn.textContent = "Artifact";
        previewBtn.addEventListener("click", () => {
          onPreviewArtifact({ code: codeText, language: lang || "text" });
        });
        block.appendChild(previewBtn);
      }

      // Add "▶ Run" button for executable languages
      if (lang && RUNNABLE_LANGS.has(lang.toLowerCase())) {
        const hasPreviewBtn = !!block.querySelector(".preview-btn");
        const runBtn = document.createElement("button");
        runBtn.className = "run-btn";
        runBtn.type = "button";
        runBtn.setAttribute("aria-label", "Run code");
        runBtn.textContent = "▶ Run";
        runBtn.style.right = hasPreviewBtn ? "116px" : "60px";

        let outputEl = null;

        runBtn.addEventListener("click", async () => {
          runBtn.textContent = "Running…";
          runBtn.disabled = true;

          if (!outputEl) {
            outputEl = document.createElement("div");
            outputEl.className = "code-output";
            block.parentNode.insertBefore(outputEl, block.nextSibling);
          }
          outputEl.innerHTML = '<div class="output-running">Running…</div>';

          try {
            const result = await invoke("execute_code", {
              language: lang,
              code: codeText,
              timeoutSecs: 30,
            });

            let html = "";
            if (result.timed_out) {
              html = `<div class="output-timeout">⏱ Timed out after 30s</div>`;
            } else {
              if (result.stdout)
                html += `<pre class="output-stdout">${escapeHtml(result.stdout)}</pre>`;
              if (result.stderr)
                html += `<pre class="output-stderr">${escapeHtml(result.stderr)}</pre>`;
              if (!result.stdout && !result.stderr)
                html = `<div class="output-empty">No output</div>`;
              if (result.exit_code !== 0)
                html += `<div class="output-exit-code">exit ${result.exit_code}</div>`;
            }
            outputEl.innerHTML = html;
          } catch (e) {
            outputEl.innerHTML = `<pre class="output-stderr">${escapeHtml(String(e))}</pre>`;
          }

          runBtn.textContent = "▶ Run";
          runBtn.disabled = false;
        });

        block.appendChild(runBtn);
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
            <Icon name="edit" size={13} />
          </button>
        {/if}
        {#if role === "assistant" && onRegenerate}
          <button class="action-btn" onclick={() => onRegenerate(messageId)} title="Regenerate" aria-label="Regenerate response">
            <Icon name="refresh" size={13} />
          </button>
        {/if}
        {#if onFork}
          <button class="action-btn" onclick={() => onFork(messageId)} title="Fork conversation from here" aria-label="Fork conversation">
            <Icon name="fork" size={13} />
          </button>
        {/if}
        {#if role === "assistant" && ttsEnabled}
          <button class="action-btn" onclick={() => invoke("speak_text", { text: content, rate: ttsRate })} title="Read aloud" aria-label="Read message aloud">
            <Icon name="volume" size={13} />
          </button>
        {/if}
        {#if messageId}
          <button class="action-btn" onclick={openAnnotationInput} title="Add note" aria-label="Add annotation">
            <Icon name="note" size={13} />
          </button>
        {/if}
      </div>
    {/if}

    {#if annotations.length > 0 || showAnnotationInput}
      <div class="annotations">
        {#each annotations as ann (ann.id)}
          <div class="annotation">
            <span class="annotation-text">{ann.content}</span>
            <button class="annotation-delete" onclick={() => removeAnnotation(ann.id)} aria-label="Delete note" title="Delete note">
              <Icon name="close" size={10} stroke={2.5} />
            </button>
          </div>
        {/each}
        {#if showAnnotationInput}
          <div class="annotation-input-wrap">
            <input
              bind:this={annotationInputEl}
              bind:value={annotationText}
              onkeydown={onAnnotationKeydown}
              onblur={saveAnnotation}
              placeholder="Add a note… (Enter to save)"
              class="annotation-input"
              type="text"
            />
          </div>
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
    background: var(--accent-soft);
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

  .message:hover .message-actions,
  .message:focus-within .message-actions {
    opacity: 1;
  }

  /* Touch / pointer-coarse devices: always show actions */
  @media (hover: none) {
    .message-actions {
      opacity: 1;
    }
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
    padding: 3px var(--space-3);
    font-size: 11px;
    font-family: inherit;
    background: var(--accent-soft-hover);
    color: var(--accent);
    border: none;
    border-radius: var(--radius-1);
    cursor: pointer;
    transition: background 0.15s, color 0.15s;
  }

  .message-content :global(.preview-btn:hover) {
    background: var(--accent);
    color: var(--white);
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

  .message-content :global(.run-btn) {
    position: absolute;
    top: 6px;
    padding: 3px 10px;
    font-size: 11px;
    font-family: inherit;
    background: rgba(122, 162, 247, 0.15);
    color: var(--accent);
    border: none;
    border-radius: 4px;
    cursor: pointer;
    transition: background 0.15s, color 0.15s;
  }

  .message-content :global(.run-btn:hover) {
    background: rgba(122, 162, 247, 0.3);
    color: white;
  }

  .message-content :global(.run-btn:disabled) {
    opacity: 0.6;
    cursor: not-allowed;
  }

  .message-content :global(.code-output) {
    background: var(--bg-secondary);
    border: 1px solid var(--border);
    border-top: none;
    border-radius: 0 0 8px 8px;
    margin-top: -4px;
    margin-bottom: 8px;
    overflow-x: auto;
    font-family: "JetBrains Mono", "Fira Code", monospace;
    font-size: 12px;
  }

  .message-content :global(.code-output pre) {
    margin: 0;
    padding: 10px 12px;
    white-space: pre-wrap;
    word-break: break-word;
  }

  .message-content :global(.output-stdout) {
    color: var(--text-primary);
  }

  .message-content :global(.output-stderr) {
    color: var(--danger);
  }

  .message-content :global(.output-running),
  .message-content :global(.output-timeout),
  .message-content :global(.output-empty) {
    padding: 8px 12px;
    color: var(--text-muted);
    font-style: italic;
  }

  .message-content :global(.output-exit-code) {
    padding: 2px 12px 8px;
    color: var(--text-muted);
    font-size: 11px;
    font-family: "JetBrains Mono", "Fira Code", monospace;
  }

  .annotations {
    margin-top: 8px;
    display: flex;
    flex-direction: column;
    gap: 4px;
  }

  .annotation {
    display: flex;
    align-items: center;
    gap: var(--space-2);
    background: var(--warning-soft);
    border-left: 2px solid var(--warning);
    border-radius: 0 var(--radius-1) var(--radius-1) 0;
    padding: var(--space-1) var(--space-2);
    font-size: 12px;
    color: var(--text-muted);
  }

  .annotation-text {
    flex: 1;
    white-space: pre-wrap;
    word-break: break-word;
  }

  .annotation-delete {
    flex-shrink: 0;
    color: var(--text-muted);
    opacity: 0;
    padding: 2px;
    border-radius: 3px;
    transition: opacity 0.15s, color 0.15s;
  }

  .annotation:hover .annotation-delete,
  .annotation:focus-within .annotation-delete {
    opacity: 1;
  }

  .annotation-delete:hover {
    color: var(--danger);
  }

  .annotation-input-wrap {
    padding: 2px 0;
  }

  .annotation-input {
    width: 100%;
    background: var(--warning-soft);
    border: 1px solid var(--warning);
    border-radius: var(--radius-1);
    padding: var(--space-1) var(--space-2);
    font-size: 12px;
    color: var(--text-primary);
    font-family: inherit;
    outline: none;
  }

  .annotation-input::placeholder {
    color: var(--text-muted);
  }
</style>
