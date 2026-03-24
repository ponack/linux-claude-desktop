<script>
  import { invoke } from "@tauri-apps/api/core";
  import { listen } from "@tauri-apps/api/event";
  import { open } from "@tauri-apps/plugin-dialog";
  import { sendNotification, isPermissionGranted, requestPermission } from "@tauri-apps/plugin-notification";
  import { onMount, tick } from "svelte";
  import MessageBubble from "./MessageBubble.svelte";
  import ArtifactPreview from "./ArtifactPreview.svelte";

  let { conversationId, onConversationCreated, deepLinkText = $bindable("") } = $props();

  let messages = $state([]);
  let inputText = $state("");
  let isStreaming = $state(false);
  let streamingMessageId = $state(null);
  let attachments = $state([]);
  let activeArtifact = $state(null);
  let projects = $state([]);
  let currentProjectId = $state(null);
  let messagesContainer;
  let chatInput;
  let prompts = $state([]);
  let showPromptPicker = $state(false);
  let customCommands = $state([]);
  let showCommandPicker = $state(false);
  let activeModel = $state("");
  let activeProvider = $state("");
  let tokenUsage = $state(null);
  let promptVariableDialog = $state(null); // { content, variables: [{name, value}] }

  async function loadMessages() {
    if (!conversationId) {
      messages = [];
      activeArtifact = null;
      currentProjectId = null;
      tokenUsage = null;
      return;
    }
    try {
      messages = await invoke("get_messages", { conversationId });
      const projId = await invoke("get_conversation_project", { conversationId });
      currentProjectId = projId || null;
      loadTokenUsage();
    } catch (e) {
      console.error("Failed to load messages:", e);
    }
  }

  async function loadTokenUsage() {
    if (!conversationId) { tokenUsage = null; return; }
    try {
      tokenUsage = await invoke("get_conversation_usage", { conversationId });
    } catch (e) { tokenUsage = null; }
  }

  async function loadProjects() {
    try {
      projects = await invoke("get_projects");
    } catch (e) {
      console.error("Failed to load projects:", e);
    }
  }

  $effect(() => {
    conversationId;
    loadMessages();
  });

  onMount(() => {
    loadProjects();
    loadPrompts();
    loadCustomCommands();
    loadActiveModel();
    if (chatInput) chatInput.focus();
  });

  $effect(() => {
    if (deepLinkText) {
      inputText = deepLinkText;
      deepLinkText = "";
      tick().then(() => sendMessage());
    }
  });

  async function notifyIfBackground(title, body) {
    if (document.hasFocus()) return;
    try {
      let granted = await isPermissionGranted();
      if (!granted) {
        const permission = await requestPermission();
        granted = permission === "granted";
      }
      if (granted) sendNotification({ title, body });
    } catch (_) {}
  }

  async function loadActiveModel() {
    try {
      activeModel = await invoke("get_model");
      activeProvider = await invoke("get_provider");
    } catch (e) { /* non-critical */ }
  }

  async function loadPrompts() {
    try { prompts = await invoke("get_prompts"); }
    catch (e) { console.error("Failed to load prompts:", e); }
  }

  async function loadCustomCommands() {
    try { customCommands = await invoke("get_custom_commands"); }
    catch (e) { customCommands = []; }
  }

  function insertPrompt(prompt) {
    showPromptPicker = false;
    const varPattern = /\{\{(\w+(?:\s+\w+)*)\}\}/g;
    const matches = [...prompt.content.matchAll(varPattern)];
    const uniqueVars = [...new Set(matches.map(m => m[1]))];

    if (uniqueVars.length > 0) {
      promptVariableDialog = {
        content: prompt.content,
        variables: uniqueVars.map(name => ({ name, value: "" })),
      };
    } else {
      inputText = prompt.content;
    }
  }

  function applyPromptVariables() {
    if (!promptVariableDialog) return;
    let result = promptVariableDialog.content;
    for (const v of promptVariableDialog.variables) {
      result = result.replaceAll(`{{${v.name}}}`, v.value);
    }
    inputText = result;
    promptVariableDialog = null;
  }

  function cancelPromptVariables() {
    promptVariableDialog = null;
  }

  let filteredCommands = $derived(
    inputText.startsWith("/")
      ? customCommands.filter(c => c.name.toLowerCase().startsWith(inputText.slice(1).toLowerCase()))
      : []
  );

  async function executeCustomCommand(cmd) {
    showCommandPicker = false;
    inputText = "";
    try {
      const output = await invoke("run_custom_command", { command: cmd.command });
      inputText = `[Output of /${cmd.name}]:\n\`\`\`\n${output.trim()}\n\`\`\`\n\nPlease analyze the above output.`;
      await sendMessage();
    } catch (e) {
      messages = [
        ...messages,
        {
          id: "error-" + Date.now(),
          role: "error",
          content: `Command /${cmd.name} failed: ${String(e)}`,
          conversation_id: conversationId,
          created_at: new Date().toISOString(),
        },
      ];
      scrollToBottom();
    }
  }

  async function handleProjectChange(e) {
    if (!conversationId) return;
    const projectId = e.target.value || null;
    try {
      await invoke("set_conversation_project", { conversationId, projectId });
      currentProjectId = projectId;
    } catch (err) {
      console.error("Failed to set project:", err);
    }
  }

  onMount(() => {
    const unlistenUsage = listen("token-usage", () => {
      loadTokenUsage();
    });

    const unlisten = listen("stream-event", (event) => {
      const { event: eventType, content, message_id } = event.payload;

      if (eventType === "delta") {
        messages = messages.map((m) =>
          m.id === message_id
            ? { ...m, content: m.content + content }
            : m
        );
        scrollToBottom();
      } else if (eventType === "done") {
        isStreaming = false;
        streamingMessageId = null;
        notifyIfBackground("Response complete", "Claude has finished responding.");
      } else if (eventType === "error") {
        isStreaming = false;
        streamingMessageId = null;
        messages = [
          ...messages,
          {
            id: "error-" + Date.now(),
            role: "error",
            content: content,
            conversation_id: conversationId,
            created_at: new Date().toISOString(),
          },
        ];
        scrollToBottom();
      }
    });

    return () => {
      unlisten.then((fn) => fn());
      unlistenUsage.then((fn) => fn());
    };
  });

  async function scrollToBottom() {
    await tick();
    if (messagesContainer) {
      messagesContainer.scrollTop = messagesContainer.scrollHeight;
    }
  }

  async function addAttachment() {
    const files = await open({
      multiple: true,
      filters: [{
        name: "Images",
        extensions: ["png", "jpg", "jpeg", "gif", "webp"],
      }],
    });
    if (!files) return;
    const fileList = Array.isArray(files) ? files : [files];
    for (const file of fileList) {
      const path = typeof file === "string" ? file : file.path;
      const ext = path.split(".").pop().toLowerCase();
      const mediaTypes = {
        png: "image/png",
        jpg: "image/jpeg",
        jpeg: "image/jpeg",
        gif: "image/gif",
        webp: "image/webp",
      };
      attachments = [...attachments, {
        path,
        media_type: mediaTypes[ext] || "image/png",
        name: path.split("/").pop(),
      }];
    }
  }

  function removeAttachment(index) {
    attachments = attachments.filter((_, i) => i !== index);
  }

  async function captureScreenshot() {
    try {
      const result = await invoke("capture_screenshot");
      attachments = [...attachments, {
        path: null,
        data: result.data,
        media_type: result.media_type,
        name: "screenshot.png",
      }];
    } catch (e) {
      console.error("Screenshot failed:", e);
    }
  }

  let isDragging = $state(false);

  function handleDragOver(e) {
    e.preventDefault();
    isDragging = true;
  }

  function handleDragLeave(e) {
    e.preventDefault();
    isDragging = false;
  }

  function handleDrop(e) {
    e.preventDefault();
    isDragging = false;
    const files = e.dataTransfer?.files;
    if (!files) return;
    processDroppedFiles(files);
  }

  function processDroppedFiles(fileList) {
    const imageExts = ["png", "jpg", "jpeg", "gif", "webp"];
    const mediaTypes = {
      png: "image/png", jpg: "image/jpeg", jpeg: "image/jpeg",
      gif: "image/gif", webp: "image/webp",
    };
    for (const file of fileList) {
      const ext = file.name.split(".").pop().toLowerCase();
      if (!imageExts.includes(ext)) continue;
      const reader = new FileReader();
      reader.onload = () => {
        attachments = [...attachments, {
          path: null,
          data: reader.result.split(",")[1],
          media_type: mediaTypes[ext] || "image/png",
          name: file.name,
        }];
      };
      reader.readAsDataURL(file);
    }
  }

  function handlePaste(e) {
    const items = e.clipboardData?.items;
    if (!items) return;
    const imageItems = [];
    for (const item of items) {
      if (item.type.startsWith("image/")) {
        imageItems.push(item);
      }
    }
    if (imageItems.length === 0) return;
    e.preventDefault();
    for (const item of imageItems) {
      const file = item.getAsFile();
      if (!file) continue;
      const reader = new FileReader();
      reader.onload = () => {
        const ext = file.type.split("/")[1] === "jpeg" ? "jpg" : file.type.split("/")[1];
        attachments = [...attachments, {
          path: null,
          data: reader.result.split(",")[1],
          media_type: file.type,
          name: `pasted-image.${ext}`,
        }];
      };
      reader.readAsDataURL(file);
    }
  }

  async function sendMessage() {
    const text = inputText.trim();
    if ((!text && attachments.length === 0) || isStreaming) return;

    inputText = "";
    const currentAttachments = [...attachments];
    attachments = [];
    let convId = conversationId;

    let isNewConversation = false;
    if (!convId) {
      try {
        const title = text.length > 40 ? text.substring(0, 40) + "..." : text || "Image conversation";
        convId = await invoke("create_conversation", { title });
        onConversationCreated(convId);
        isNewConversation = true;
      } catch (e) {
        console.error("Failed to create conversation:", e);
        return;
      }
    }

    isStreaming = true;

    try {
      const apiAttachments = currentAttachments.map(({ path, media_type, data }) => ({ path: path || null, media_type, data: data || null }));
      const assistantMsgId = await invoke("send_message", {
        conversationId: convId,
        content: text,
        attachments: apiAttachments.length > 0 ? apiAttachments : null,
      });

      streamingMessageId = assistantMsgId;
      await loadMessages();
      scrollToBottom();

      if (isNewConversation) {
        invoke("generate_title", {
          conversationId: convId,
          userMessage: text || "Shared an image",
        }).then(() => {
          onConversationCreated(convId);
        }).catch((e) => console.error("Title generation failed:", e));
      }
    } catch (e) {
      isStreaming = false;
      messages = [
        ...messages,
        {
          id: "error-" + Date.now(),
          role: "error",
          content: String(e),
          conversation_id: convId,
          created_at: new Date().toISOString(),
          retryContent: text,
        },
      ];
      scrollToBottom();
    }
  }

  async function retryMessage(errorMsg) {
    messages = messages.filter((m) => m.id !== errorMsg.id);
    inputText = errorMsg.retryContent || "";
    await sendMessage();
  }

  async function handleEdit(messageId, newContent) {
    if (!conversationId || isStreaming) return;
    try {
      // Delete this message and everything after it
      await invoke("delete_messages_from", { conversationId, messageId });
      await loadMessages();
      // Re-send with the edited content
      inputText = newContent;
      await sendMessage();
    } catch (e) {
      console.error("Edit failed:", e);
    }
  }

  async function handleRegenerate(messageId) {
    if (!conversationId || isStreaming) return;
    try {
      // Find the user message before this assistant message
      const idx = messages.findIndex((m) => m.id === messageId);
      if (idx <= 0) return;
      const userMsg = messages[idx - 1];
      if (userMsg.role !== "user") return;

      // Delete from the assistant message onwards
      await invoke("delete_messages_from", { conversationId, messageId });
      await loadMessages();

      // Re-send the original user message
      isStreaming = true;
      const assistantMsgId = await invoke("send_message", {
        conversationId,
        content: userMsg.content,
        attachments: null,
      });
      streamingMessageId = assistantMsgId;
      await loadMessages();
      scrollToBottom();
    } catch (e) {
      isStreaming = false;
      console.error("Regenerate failed:", e);
    }
  }

  async function handleFork(messageId) {
    if (!conversationId) return;
    try {
      const newConvId = await invoke("fork_conversation", { conversationId, messageId });
      onConversationCreated(newConvId);
    } catch (e) {
      console.error("Fork failed:", e);
    }
  }

  async function stopGeneration() {
    try {
      await invoke("stop_generation");
    } catch (e) {
      console.error("Failed to stop:", e);
    }
  }

  async function exportConversation(format) {
    if (!conversationId) return;
    try {
      const content = await invoke("export_conversation", {
        conversationId,
        format,
      });
      const ext = format === "json" ? "json" : "md";
      const blob = new Blob([content], { type: "text/plain" });
      const url = URL.createObjectURL(blob);
      const a = document.createElement("a");
      a.href = url;
      a.download = `conversation.${ext}`;
      a.click();
      URL.revokeObjectURL(url);
    } catch (e) {
      console.error("Export failed:", e);
    }
  }

  function handlePreviewArtifact(artifact) {
    activeArtifact = artifact;
  }

  function closeArtifact() {
    activeArtifact = null;
  }

  function handleKeydown(e) {
    if (e.key === "Enter" && !e.shiftKey) {
      e.preventDefault();
      // If command picker is showing and there's a match, execute it
      if (showCommandPicker && filteredCommands.length > 0) {
        executeCustomCommand(filteredCommands[0]);
        return;
      }
      sendMessage();
    }
    if (e.key === "Escape") {
      showCommandPicker = false;
      showPromptPicker = false;
    }
  }

  $effect(() => {
    showCommandPicker = inputText.startsWith("/") && inputText.length > 0 && filteredCommands.length > 0;
  });
</script>

<div class="chat-container">
  <div class="chat-main" class:has-artifact={activeArtifact}>
    {#if conversationId && messages.length > 0}
      <div class="chat-toolbar">
        {#if activeModel}
          <span class="model-indicator" title="{activeProvider} / {activeModel}">{activeModel}</span>
        {/if}
        {#if projects.length > 0}
          <select class="project-select" aria-label="Assign project" value={currentProjectId || ""} onchange={handleProjectChange}>
            <option value="">No project</option>
            {#each projects as project (project.id)}
              <option value={project.id}>{project.name}</option>
            {/each}
          </select>
        {/if}
        {#if tokenUsage && tokenUsage.total_tokens > 0}
          <span class="token-usage" title="Input: {tokenUsage.input_tokens.toLocaleString()} | Output: {tokenUsage.output_tokens.toLocaleString()}">
            {tokenUsage.total_tokens.toLocaleString()} tokens
          </span>
        {/if}
        <div class="toolbar-actions">
          <button class="toolbar-btn" onclick={() => exportConversation("markdown")} title="Export as Markdown" aria-label="Export as Markdown">
            <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
              <path d="M21 15v4a2 2 0 01-2 2H5a2 2 0 01-2-2v-4"/><polyline points="7 10 12 15 17 10"/><line x1="12" y1="15" x2="12" y2="3"/>
            </svg>
            .md
          </button>
          <button class="toolbar-btn" onclick={() => exportConversation("json")} title="Export as JSON" aria-label="Export as JSON">
            <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
              <path d="M21 15v4a2 2 0 01-2 2H5a2 2 0 01-2-2v-4"/><polyline points="7 10 12 15 17 10"/><line x1="12" y1="15" x2="12" y2="3"/>
            </svg>
            .json
          </button>
        </div>
      </div>
    {/if}
    <div class="messages" bind:this={messagesContainer} role="log" aria-live="polite" aria-label="Chat messages">
      {#if messages.length === 0 && !conversationId}
        <div class="empty-state">
          <img src="/assets/logo.svg" alt="Ubuntu Claude Desktop" class="empty-logo" />
          <h2>Ubuntu Claude Desktop</h2>
          <p>Start a conversation by typing a message below.</p>
        </div>
      {/if}

      {#each messages as message (message.id)}
        <MessageBubble
          role={message.role}
          content={message.content}
          messageId={message.id}
          isStreaming={isStreaming && message.id === streamingMessageId}
          onEdit={handleEdit}
          onRegenerate={handleRegenerate}
          onFork={conversationId ? handleFork : null}
          onPreviewArtifact={handlePreviewArtifact}
          onRetry={message.retryContent ? () => retryMessage(message) : null}
        />
      {/each}
    </div>

    <div class="input-area" class:dragging={isDragging}
      ondragover={handleDragOver} ondragleave={handleDragLeave} ondrop={handleDrop}>
    {#if attachments.length > 0}
      <div class="attachments-preview">
        {#each attachments as att, i}
          <div class="attachment-chip">
            <span class="att-name">{att.name}</span>
            <button class="att-remove" onclick={() => removeAttachment(i)} aria-label="Remove attachment {att.name}">x</button>
          </div>
        {/each}
      </div>
    {/if}

    {#if showCommandPicker}
      <div class="command-picker">
        {#each filteredCommands as cmd}
          <button class="command-item" onclick={() => executeCustomCommand(cmd)}>
            <span class="cmd-name">/{cmd.name}</span>
            {#if cmd.description}<span class="cmd-desc">{cmd.description}</span>{/if}
          </button>
        {/each}
      </div>
    {/if}

    {#if showPromptPicker}
      <div class="command-picker">
        {#each prompts as prompt (prompt.id)}
          <button class="command-item" onclick={() => insertPrompt(prompt)}>
            <span class="cmd-name">{prompt.name}</span>
            <span class="cmd-desc">{prompt.content.length > 60 ? prompt.content.slice(0, 60) + '...' : prompt.content}</span>
          </button>
        {/each}
        {#if prompts.length === 0}
          <div class="command-item" style="opacity: 0.5; cursor: default;">No prompts saved. Add them in Settings.</div>
        {/if}
      </div>
    {/if}

    <div class="input-wrapper">
      <button class="attach-btn" onclick={addAttachment} disabled={isStreaming} title="Attach image (PNG, JPG, GIF, WebP)" aria-label="Attach image">
        <svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
          <path d="M21.44 11.05l-9.19 9.19a6 6 0 01-8.49-8.49l9.19-9.19a4 4 0 015.66 5.66l-9.2 9.19a2 2 0 01-2.83-2.83l8.49-8.48"/>
        </svg>
      </button>
      <button class="attach-btn" onclick={captureScreenshot} disabled={isStreaming} title="Capture screenshot region" aria-label="Capture screenshot">
        <svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
          <rect x="3" y="3" width="18" height="18" rx="2" ry="2"/><circle cx="8.5" cy="8.5" r="1.5"/><polyline points="21 15 16 10 5 21"/>
        </svg>
      </button>
      <button class="attach-btn" onclick={() => (showPromptPicker = !showPromptPicker)} disabled={isStreaming} title="Insert a saved prompt from your library" aria-label="Prompt library">
        <svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
          <path d="M19 21l-7-5-7 5V5a2 2 0 012-2h10a2 2 0 012 2z"/>
        </svg>
      </button>
      <textarea
        bind:this={chatInput}
        bind:value={inputText}
        onkeydown={handleKeydown}
        onpaste={handlePaste}
        placeholder="Message Claude... (/ for commands, paste or drop images)"
        rows="1"
        disabled={isStreaming}
        aria-label="Chat message input"
      ></textarea>
      {#if isStreaming}
        <button class="stop-btn" onclick={stopGeneration} aria-label="Stop generating">Stop</button>
      {:else}
        <button
          class="send-btn"
          onclick={sendMessage}
          disabled={!inputText.trim() && attachments.length === 0}
          aria-label="Send message"
        >
          Send
        </button>
      {/if}
    </div>
  </div>
  </div>

  {#if activeArtifact}
    <ArtifactPreview artifact={activeArtifact} onClose={closeArtifact} />
  {/if}
</div>

{#if promptVariableDialog}
  <div class="variable-overlay" onclick={cancelPromptVariables} role="dialog" aria-label="Fill in prompt variables">
    <div class="variable-dialog" onclick={(e) => e.stopPropagation()}>
      <h3>Fill in Variables</h3>
      <div class="variable-list">
        {#each promptVariableDialog.variables as variable, i}
          <label class="variable-field">
            <span class="variable-name">{variable.name}</span>
            <input
              type="text"
              bind:value={promptVariableDialog.variables[i].value}
              placeholder="Enter value for {variable.name}"
              onkeydown={(e) => e.key === "Enter" && applyPromptVariables()}
            />
          </label>
        {/each}
      </div>
      <div class="variable-actions">
        <button class="var-btn primary" onclick={applyPromptVariables}>Apply</button>
        <button class="var-btn" onclick={cancelPromptVariables}>Cancel</button>
      </div>
    </div>
  </div>
{/if}

<style>
  .chat-container {
    display: flex;
    flex-direction: row;
    height: 100%;
  }

  .chat-main {
    display: flex;
    flex-direction: column;
    flex: 1;
    min-width: 0;
  }

  .chat-toolbar {
    display: flex;
    align-items: center;
    justify-content: flex-end;
    padding: 6px 20px;
    border-bottom: 1px solid var(--border);
    background: var(--bg-secondary);
    flex-shrink: 0;
  }

  .model-indicator {
    font-size: 11px;
    color: var(--text-muted);
    background: var(--bg-tertiary);
    padding: 3px 8px;
    border-radius: 4px;
    margin-right: auto;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
    max-width: 200px;
  }

  .token-usage {
    font-size: 11px;
    color: var(--text-muted);
    background: var(--bg-tertiary);
    padding: 3px 8px;
    border-radius: 4px;
    white-space: nowrap;
    cursor: default;
  }

  .toolbar-actions {
    display: flex;
    gap: 4px;
  }

  .toolbar-btn {
    display: flex;
    align-items: center;
    gap: 4px;
    padding: 4px 10px;
    border-radius: 6px;
    font-size: 12px;
    color: var(--text-muted);
    transition: color 0.15s, background 0.15s;
  }

  .toolbar-btn:hover {
    color: var(--text-primary);
    background: var(--bg-tertiary);
  }

  .project-select {
    padding: 4px 8px;
    border-radius: 6px;
    font-size: 12px;
    background: var(--bg-input);
    border: 1px solid var(--border);
    color: var(--text-secondary);
    outline: none;
    cursor: pointer;
  }

  .project-select:focus {
    border-color: var(--accent);
  }

  .messages {
    flex: 1;
    overflow-y: auto;
    padding: 20px;
    display: flex;
    flex-direction: column;
    gap: 16px;
  }

  .empty-state {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    height: 100%;
    color: var(--text-muted);
    gap: 8px;
  }

  .empty-logo {
    width: 80px;
    height: 80px;
    margin-bottom: 8px;
  }

  .empty-state h2 {
    font-size: 24px;
    font-weight: 600;
    color: var(--text-secondary);
  }

  .input-area {
    padding: 16px 20px;
    border-top: 1px solid var(--border);
    background: var(--bg-secondary);
    transition: background 0.15s;
  }

  .input-area.dragging {
    background: var(--bg-tertiary, var(--bg-secondary));
    outline: 2px dashed var(--accent);
    outline-offset: -4px;
  }

  .attachments-preview {
    display: flex;
    flex-wrap: wrap;
    gap: 6px;
    margin-bottom: 8px;
  }

  .attachment-chip {
    display: flex;
    align-items: center;
    gap: 4px;
    padding: 4px 8px;
    background: var(--bg-tertiary);
    border-radius: 6px;
    font-size: 12px;
    color: var(--text-secondary);
  }

  .att-name {
    max-width: 150px;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .att-remove {
    font-size: 11px;
    color: var(--text-muted);
    padding: 0 2px;
  }

  .att-remove:hover {
    color: var(--danger);
  }

  .input-wrapper {
    display: flex;
    align-items: flex-end;
    gap: 8px;
    background: var(--bg-input);
    border: 1px solid var(--border);
    border-radius: 12px;
    padding: 8px 12px;
  }

  .attach-btn {
    color: var(--text-muted);
    padding: 4px;
    border-radius: 6px;
    transition: color 0.15s;
    flex-shrink: 0;
    display: flex;
    align-items: center;
  }

  .attach-btn:hover:not(:disabled) {
    color: var(--accent);
  }

  .attach-btn:disabled {
    opacity: 0.3;
    cursor: not-allowed;
  }

  textarea {
    flex: 1;
    background: none;
    border: none;
    outline: none;
    resize: none;
    max-height: 150px;
    line-height: 1.5;
    padding: 4px 0;
  }

  .send-btn, .stop-btn {
    padding: 6px 16px;
    border-radius: 8px;
    font-size: 13px;
    font-weight: 500;
    transition: background 0.15s;
    flex-shrink: 0;
  }

  .send-btn {
    background: var(--accent);
    color: white;
  }

  .send-btn:hover:not(:disabled) {
    background: var(--accent-hover);
  }

  .send-btn:disabled {
    opacity: 0.4;
    cursor: not-allowed;
  }

  .stop-btn {
    background: var(--danger);
    color: white;
  }

  .stop-btn:hover {
    opacity: 0.85;
  }

  .command-picker {
    background: var(--bg-tertiary);
    border: 1px solid var(--border);
    border-radius: 8px;
    margin-bottom: 6px;
    max-height: 200px;
    overflow-y: auto;
  }

  .command-item {
    display: flex;
    flex-direction: column;
    gap: 2px;
    width: 100%;
    padding: 8px 12px;
    text-align: left;
    transition: background 0.1s;
    border-bottom: 1px solid var(--border);
  }

  .command-item:last-child {
    border-bottom: none;
  }

  .command-item:hover {
    background: var(--bg-secondary);
  }

  .cmd-name {
    font-size: 13px;
    font-weight: 500;
    color: var(--accent);
  }

  .cmd-desc {
    font-size: 11px;
    color: var(--text-muted);
  }

  .variable-overlay {
    position: fixed;
    top: 0; left: 0; right: 0; bottom: 0;
    background: rgba(0, 0, 0, 0.5);
    display: flex;
    align-items: center;
    justify-content: center;
    z-index: 2000;
  }

  .variable-dialog {
    background: var(--bg-secondary);
    border: 1px solid var(--border);
    border-radius: 12px;
    padding: 20px;
    width: 400px;
    max-width: 90vw;
    box-shadow: 0 8px 32px rgba(0, 0, 0, 0.3);
  }

  .variable-dialog h3 {
    font-size: 15px;
    font-weight: 600;
    margin-bottom: 16px;
  }

  .variable-list {
    display: flex;
    flex-direction: column;
    gap: 12px;
    margin-bottom: 16px;
  }

  .variable-field {
    display: flex;
    flex-direction: column;
    gap: 4px;
  }

  .variable-name {
    font-size: 12px;
    font-weight: 600;
    color: var(--accent);
    font-family: "JetBrains Mono", "Fira Code", monospace;
  }

  .variable-field input {
    padding: 8px 10px;
    background: var(--bg-input);
    border: 1px solid var(--border);
    border-radius: 6px;
    font-size: 13px;
    outline: none;
  }

  .variable-field input:focus {
    border-color: var(--accent);
  }

  .variable-actions {
    display: flex;
    gap: 8px;
  }

  .var-btn {
    padding: 8px 16px;
    border-radius: 8px;
    font-size: 13px;
    font-weight: 500;
    color: var(--text-secondary);
    border: 1px solid var(--border);
  }

  .var-btn:hover { background: var(--bg-tertiary); }
  .var-btn.primary { background: var(--accent); color: white; border: none; }
  .var-btn.primary:hover { background: var(--accent-hover); }
</style>
