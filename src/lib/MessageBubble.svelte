<script>
  import { marked } from "marked";
  import hljs from "highlight.js";
  import { onMount, tick } from "svelte";

  let { role, content, isStreaming } = $props();
  let messageEl;

  // Configure marked to use highlight.js
  marked.setOptions({
    highlight(code, lang) {
      if (lang && hljs.getLanguage(lang)) {
        return hljs.highlight(code, { language: lang }).value;
      }
      return hljs.highlightAuto(code).value;
    },
  });

  let renderedHtml = $derived(
    role === "error" ? content : marked.parse(content || "")
  );

  // Add copy buttons to code blocks after render
  async function attachCopyButtons() {
    await tick();
    if (!messageEl) return;
    const blocks = messageEl.querySelectorAll("pre");
    for (const block of blocks) {
      if (block.querySelector(".copy-btn")) continue;
      const btn = document.createElement("button");
      btn.className = "copy-btn";
      btn.textContent = "Copy";
      btn.addEventListener("click", async () => {
        const code = block.querySelector("code");
        const text = code ? code.textContent : block.textContent;
        await navigator.clipboard.writeText(text);
        btn.textContent = "Copied!";
        setTimeout(() => (btn.textContent = "Copy"), 1500);
      });
      block.style.position = "relative";
      block.appendChild(btn);
    }
  }

  $effect(() => {
    renderedHtml;
    attachCopyButtons();
  });
</script>

<div class="message" class:user={role === "user"} class:assistant={role === "assistant"} class:error={role === "error"} bind:this={messageEl}>
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
  <div class="message-content">
    {#if role === "error"}
      <p class="error-text">{content}</p>
    {:else}
      {@html renderedHtml}
    {/if}
  </div>
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

  .message-content :global(pre) {
    background: rgba(0, 0, 0, 0.3);
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
    background: rgba(0, 0, 0, 0.3);
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
