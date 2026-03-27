<script>
  import { invoke } from "@tauri-apps/api/core";
  import { listen } from "@tauri-apps/api/event";
  import { getCurrentWindow } from "@tauri-apps/api/window";
  import { open } from "@tauri-apps/plugin-dialog";
  import { sendNotification, isPermissionGranted, requestPermission } from "@tauri-apps/plugin-notification";
  import { onMount, tick } from "svelte";
  import MessageBubble from "./MessageBubble.svelte";
  import ArtifactPanel from "./ArtifactPanel.svelte";

  let { conversationId, onConversationCreated, deepLinkText = $bindable("") } = $props();

  let messages = $state([]);
  let inputText = $state("");
  let isStreaming = $state(false);
  let streamingMessageId = $state(null);
  let attachments = $state([]);
  let artifacts = $state([]);
  let activeArtifactId = $state(null);
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
  let conversationCost = $state(0);
  let promptVariableDialog = $state(null); // { content, variables: [{name, value}] }
  let agentMode = $state(false);
  let agentSteps = $state([]); // [{step, description, status}]
  let agentRunning = $state(false);

  // URL import dialog
  let showUrlImport = $state(false);
  let importUrl = $state("");
  let importingUrl = $state(false);

  // Voice
  let ttsEnabled = $state(false);
  let ttsRate = $state(100);
  let sttEnabled = $state(false);
  let whisperModelPath = $state("");
  let isRecording = $state(false);

  // Context window limits by model/provider
  function getContextLimit(model, provider) {
    if (!model) return 128000;
    const m = model.toLowerCase();
    if (m.includes("claude")) return 200000;
    if (m.includes("gpt-4")) return 128000;
    if (m.includes("gpt-3.5")) return 16000;
    if (provider === "ollama") return 128000;
    return 128000;
  }
  let contextLimit = $derived(getContextLimit(activeModel, activeProvider));

  // Pagination state
  const PAGE_SIZE = 50;
  let totalMessageCount = $state(0);
  let loadedOffset = $state(0); // how many older messages we've loaded beyond the initial page
  let hasMoreMessages = $derived(messages.length < totalMessageCount);
  let loadingMore = $state(false);
  let loadMoreSentinel; // intersection observer target

  // Offline mode
  let isOffline = $state(!navigator.onLine);
  let offlineQueue = $state([]); // queued messages when offline

  async function loadMessages() {
    if (!conversationId) {
      messages = [];
      artifacts = [];
      activeArtifactId = null;
      currentProjectId = null;
      tokenUsage = null;
      totalMessageCount = 0;
      loadedOffset = 0;
      return;
    }
    try {
      totalMessageCount = await invoke("get_message_count", { conversationId });
      // Load the most recent PAGE_SIZE messages
      messages = await invoke("get_messages_paginated", { conversationId, limit: PAGE_SIZE, offset: 0 });
      loadedOffset = 0;
      const projId = await invoke("get_conversation_project", { conversationId });
      currentProjectId = projId || null;
      loadTokenUsage();
      loadArtifacts();
    } catch (e) {
      console.error("Failed to load messages:", e);
    }
  }

  async function loadOlderMessages() {
    if (!conversationId || loadingMore || !hasMoreMessages) return;
    loadingMore = true;
    try {
      const newOffset = loadedOffset + PAGE_SIZE;
      const older = await invoke("get_messages_paginated", { conversationId, limit: PAGE_SIZE, offset: newOffset });
      if (older.length > 0) {
        // Preserve scroll position
        const prevHeight = messagesContainer?.scrollHeight || 0;
        messages = [...older, ...messages];
        loadedOffset = newOffset;
        await tick();
        if (messagesContainer) {
          messagesContainer.scrollTop = messagesContainer.scrollHeight - prevHeight;
        }
      }
    } catch (e) {
      console.error("Failed to load older messages:", e);
    } finally {
      loadingMore = false;
    }
  }

  async function loadTokenUsage() {
    if (!conversationId) { tokenUsage = null; return; }
    try {
      tokenUsage = await invoke("get_conversation_usage", { conversationId });
    } catch (e) { tokenUsage = null; }
  }

  async function loadConversationCost() {
    if (!conversationId) { conversationCost = 0; return; }
    try {
      conversationCost = await invoke("get_conversation_cost", { conversationId });
    } catch (e) { conversationCost = 0; }
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

    // Tauri native drag-and-drop (HTML5 drag-drop doesn't work for desktop files in Tauri)
    const unlistenDragDrop = getCurrentWindow().onDragDropEvent((event) => {
      if (event.payload.type === "over") {
        isDragging = true;
      } else if (event.payload.type === "drop") {
        isDragging = false;
        if (event.payload.paths && event.payload.paths.length > 0) {
          processDroppedPaths(event.payload.paths);
        }
      } else if (event.payload.type === "leave") {
        isDragging = false;
      }
    });

    // Offline mode listeners
    const handleOnline = () => {
      isOffline = false;
      flushOfflineQueue();
    };
    const handleOffline = () => { isOffline = true; };
    window.addEventListener("online", handleOnline);
    window.addEventListener("offline", handleOffline);

    return () => {
      unlistenDragDrop.then((fn) => fn());
      window.removeEventListener("online", handleOnline);
      window.removeEventListener("offline", handleOffline);
    };
  });

  // Intersection observer for loading older messages
  $effect(() => {
    if (!loadMoreSentinel) return;
    const observer = new IntersectionObserver((entries) => {
      if (entries[0].isIntersecting && hasMoreMessages && !loadingMore) {
        loadOlderMessages();
      }
    }, { root: messagesContainer, threshold: 0.1 });
    observer.observe(loadMoreSentinel);
    return () => observer.disconnect();
  });

  async function flushOfflineQueue() {
    if (offlineQueue.length === 0) return;
    const queue = [...offlineQueue];
    offlineQueue = [];
    for (const item of queue) {
      inputText = item.text;
      attachments = item.attachments || [];
      await sendMessage();
    }
  }

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

  async function loadVoiceSettings() {
    try {
      ttsEnabled = await invoke("get_tts_enabled");
      ttsRate = await invoke("get_tts_rate");
      sttEnabled = await invoke("get_stt_enabled");
      whisperModelPath = await invoke("get_whisper_model_path");
    } catch (_) {}
  }

  async function toggleRecording() {
    if (isRecording) {
      isRecording = false;
      try {
        const transcript = await invoke("stop_recording_and_transcribe", { modelPath: whisperModelPath });
        if (transcript) {
          inputText += (inputText ? " " : "") + transcript;
        }
      } catch (e) {
        console.error("Transcription error:", e);
      }
    } else {
      try {
        await invoke("start_recording");
        isRecording = true;
      } catch (e) {
        console.error("Recording error:", e);
      }
    }
  }

  async function handleUrlImport() {
    if (!importUrl.trim()) return;
    importingUrl = true;
    try {
      const [title, content] = await invoke("import_url", { url: importUrl.trim() });
      // Save to knowledge base
      await invoke("create_knowledge_entry", {
        title,
        content,
        sourceType: "url",
        sourceUrl: importUrl.trim(),
        filePath: null,
        projectId: currentProjectId,
      });
      // Also inject into current input as context
      inputText += (inputText ? "\n\n" : "") + `[Imported from ${importUrl.trim()}]\n${content.slice(0, 2000)}${content.length > 2000 ? "\n..." : ""}`;
      showUrlImport = false;
      importUrl = "";
    } catch (e) {
      alert("Failed to import URL: " + e);
    } finally {
      importingUrl = false;
    }
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
    loadVoiceSettings();

    const unlistenUsage = listen("token-usage", () => {
      loadTokenUsage();
      loadConversationCost();
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

        // Auto-TTS when enabled
        if (ttsEnabled) {
          const lastMsg = messages.find(m => m.id === message_id);
          if (lastMsg?.content) {
            invoke("speak_text", { text: lastMsg.content, rate: ttsRate }).catch(() => {});
          }
        }

        // Agent mode: auto-continue if response contains [CONTINUE]
        if (agentMode && agentRunning) {
          const lastMsg = messages.find(m => m.id === message_id);
          const content = lastMsg?.content || "";
          if (content.includes("[CONTINUE]") && !content.includes("[DONE]")) {
            // Parse step info
            const stepMatch = content.match(/\[STEP (\d+)\/(\d+)\]/);
            if (stepMatch) {
              const current = parseInt(stepMatch[1]);
              const total = parseInt(stepMatch[2]);
              agentSteps = Array.from({length: total}, (_, i) => ({
                step: i + 1,
                status: i < current ? "done" : i === current ? "active" : "pending",
              }));
            }
            // Auto-send continuation
            setTimeout(() => {
              inputText = "Continue with the next step.";
              sendMessage();
            }, 500);
          } else {
            agentRunning = false;
            agentSteps = [];
            notifyIfBackground("Agent complete", "All steps have been completed.");
          }
        } else {
          notifyIfBackground("Response complete", "Claude has finished responding.");
        }
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

  let scrollRafId = null;
  async function scrollToBottom() {
    await tick();
    if (scrollRafId) cancelAnimationFrame(scrollRafId);
    scrollRafId = requestAnimationFrame(() => {
      if (messagesContainer) {
        messagesContainer.scrollTop = messagesContainer.scrollHeight;
      }
      scrollRafId = null;
    });
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

  function processDroppedPaths(paths) {
    const imageExts = ["png", "jpg", "jpeg", "gif", "webp"];
    const mediaTypes = {
      png: "image/png", jpg: "image/jpeg", jpeg: "image/jpeg",
      gif: "image/gif", webp: "image/webp",
    };
    for (const filePath of paths) {
      const ext = filePath.split(".").pop().toLowerCase();
      if (!imageExts.includes(ext)) continue;
      attachments = [...attachments, {
        path: filePath,
        media_type: mediaTypes[ext] || "image/png",
        name: filePath.split("/").pop(),
      }];
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

  const AGENT_PREFIX = `You are in agent mode. Break down the user's request into clear steps. For each response:
1. State which step you're on: [STEP X/Y]
2. Execute that step fully
3. End with [CONTINUE] if more steps remain, or [DONE] if all steps are complete
Be thorough in each step. Do not skip steps or combine them.`;

  async function sendMessage() {
    let text = inputText.trim();
    if ((!text && attachments.length === 0) || isStreaming) return;

    // Offline mode: queue the message
    if (isOffline) {
      offlineQueue = [...offlineQueue, { text, attachments: [...attachments] }];
      inputText = "";
      attachments = [];
      return;
    }

    // If agent mode is on and this is the first message (triggering agent), prepend instruction
    if (agentMode && !agentRunning && text !== "Continue with the next step.") {
      text = `${AGENT_PREFIX}\n\nUser request: ${text}`;
      agentRunning = true;
      agentSteps = [{ step: 1, status: "active" }];
    }

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

  async function loadArtifacts() {
    if (!conversationId) { artifacts = []; return; }
    try {
      artifacts = await invoke("get_artifacts", { conversationId });
    } catch (e) { artifacts = []; }
  }

  async function handlePreviewArtifact(artifact) {
    if (!conversationId) return;

    // Detect artifact type
    const lang = artifact.language || "text";
    let artifactType = "code";
    if (["html", "svg"].includes(lang)) artifactType = lang;
    else if (lang === "mermaid") artifactType = "mermaid";
    else if (["markdown", "md"].includes(lang)) artifactType = "markdown";
    else if (["jsx", "tsx", "react"].includes(lang)) artifactType = "react";

    // Derive a title from the first line or comment
    const firstLine = artifact.code.split("\n")[0].replace(/^\/\/\s*|^#\s*|^<!--\s*|^\s*\*\s*/, "").trim();
    const title = firstLine.length > 3 && firstLine.length < 60 ? firstLine : `${lang} artifact`;

    try {
      const id = await invoke("create_artifact", {
        conversationId,
        title,
        artifactType,
        language: lang,
        content: artifact.code,
        source: "claude",
        messageId: null,
      });
      await loadArtifacts();
      activeArtifactId = id;
    } catch (e) {
      console.error("Failed to create artifact:", e);
    }
  }

  function closeArtifact() {
    activeArtifactId = null;
  }

  function selectArtifact(id) {
    activeArtifactId = id;
  }

  async function handleIterateArtifact(artifactId, currentContent, language, instruction) {
    if (!conversationId || isStreaming) return;
    const lang = language || "text";
    inputText = `Here is the current artifact content:\n\`\`\`${lang}\n${currentContent}\n\`\`\`\n\nPlease modify it: ${instruction}`;
    await sendMessage();
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
  <div role="status" aria-live="assertive" aria-atomic="true" class="sr-only">
    {#if isStreaming}Generating response…{/if}
  </div>
  <div class="chat-main" class:has-artifact={activeArtifactId}>
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
          {@const pct = Math.min((tokenUsage.total_tokens / contextLimit) * 100, 100)}
          {@const barColor = pct > 90 ? 'var(--danger)' : pct > 70 ? '#e0a030' : 'var(--accent)'}
          <div class="token-usage-bar" title="Input: {tokenUsage.input_tokens.toLocaleString()} | Output: {tokenUsage.output_tokens.toLocaleString()} | {tokenUsage.total_tokens.toLocaleString()} / {contextLimit.toLocaleString()} tokens ({pct.toFixed(1)}%)">
            <div class="token-bar-track">
              <div class="token-bar-fill" style="width: {pct}%; background: {barColor};"></div>
            </div>
            <span class="token-bar-label">{tokenUsage.total_tokens.toLocaleString()}</span>
            {#if conversationCost > 0}
              <span class="cost-label">${conversationCost < 0.01 ? conversationCost.toFixed(4) : conversationCost.toFixed(2)}</span>
            {/if}
          </div>
        {/if}
        <div class="toolbar-actions">
          <button class="toolbar-btn" onclick={() => invoke("popout_conversation", { conversationId })} title="Open in new window" aria-label="Open in new window">
            <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
              <path d="M18 13v6a2 2 0 01-2 2H5a2 2 0 01-2-2V8a2 2 0 012-2h6"/><polyline points="15 3 21 3 21 9"/><line x1="10" y1="14" x2="21" y2="3"/>
            </svg>
          </button>
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
    {#if isOffline}
      <div class="offline-banner">
        You are offline. Messages will be queued and sent when connection is restored.
        {#if offlineQueue.length > 0}
          <span class="queue-count">({offlineQueue.length} queued)</span>
        {/if}
      </div>
    {/if}
    <div class="messages" bind:this={messagesContainer} role="log" aria-live="polite" aria-label="Chat messages">
      {#if messages.length === 0 && !conversationId}
        <div class="empty-state">
          <img src="/assets/logo.svg" alt="Linux Claude Desktop" class="empty-logo" />
          <h2>Linux Claude Desktop</h2>
          <p>Start a conversation by typing a message below.</p>
        </div>
      {/if}

      {#if hasMoreMessages}
        <div class="load-more-sentinel" bind:this={loadMoreSentinel}>
          {#if loadingMore}
            <span class="loading-indicator">Loading older messages...</span>
          {:else}
            <button class="load-more-btn" onclick={loadOlderMessages}>Load older messages</button>
          {/if}
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
          {ttsEnabled}
          {ttsRate}
        />
      {/each}
    </div>

    {#if agentRunning && agentSteps.length > 1}
      <div class="agent-progress">
        <span class="agent-label">Agent</span>
        {#each agentSteps as step}
          <div class="agent-step" class:done={step.status === "done"} class:active={step.status === "active"}>
            {step.step}
          </div>
        {/each}
      </div>
    {/if}

    <div class="input-area" class:dragging={isDragging}>
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
      <button class="attach-btn" class:agent-active={agentMode} onclick={() => (agentMode = !agentMode)} disabled={isStreaming && agentRunning} title={agentMode ? "Agent mode ON — multi-step execution" : "Enable agent mode"} aria-label="Toggle agent mode">
        <svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
          <circle cx="12" cy="12" r="3"/><path d="M12 1v4m0 14v4m-8.66-2.34l2.83-2.83m11.66-11.66l2.83-2.83M1 12h4m14 0h4m-2.34 8.66l-2.83-2.83M4.17 4.17L7 7"/>
        </svg>
      </button>
      <button class="attach-btn" onclick={() => (showPromptPicker = !showPromptPicker)} disabled={isStreaming} title="Insert a saved prompt from your library" aria-label="Prompt library">
        <svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
          <path d="M19 21l-7-5-7 5V5a2 2 0 012-2h10a2 2 0 012 2z"/>
        </svg>
      </button>
      <button class="attach-btn" onclick={() => (showUrlImport = !showUrlImport)} disabled={isStreaming} title="Import content from a URL" aria-label="Import URL">
        <svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
          <circle cx="12" cy="12" r="10"/><line x1="2" y1="12" x2="22" y2="12"/><path d="M12 2a15.3 15.3 0 014 10 15.3 15.3 0 01-4 10 15.3 15.3 0 01-4-10 15.3 15.3 0 014-10z"/>
        </svg>
      </button>
      {#if sttEnabled}
        <button
          class="attach-btn"
          class:recording={isRecording}
          onclick={toggleRecording}
          disabled={isStreaming}
          title={isRecording ? "Stop recording and transcribe" : "Start voice input"}
          aria-label={isRecording ? "Stop recording" : "Start voice input"}
          aria-pressed={isRecording}
        >
          <svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
            <path d="M12 1a3 3 0 0 0-3 3v8a3 3 0 0 0 6 0V4a3 3 0 0 0-3-3z"/>
            <path d="M19 10v2a7 7 0 0 1-14 0v-2"/>
            <line x1="12" y1="19" x2="12" y2="23"/>
            <line x1="8" y1="23" x2="16" y2="23"/>
          </svg>
        </button>
      {/if}
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

  {#if activeArtifactId}
    <ArtifactPanel
      {artifacts}
      {activeArtifactId}
      {conversationId}
      onClose={closeArtifact}
      onSelectArtifact={selectArtifact}
      onIterateWithClaude={handleIterateArtifact}
    />
  {/if}
</div>

{#if showUrlImport}
  <div class="variable-overlay" onclick={() => (showUrlImport = false)} role="dialog" aria-modal="true" aria-labelledby="url-import-title">
    <div class="variable-dialog" onclick={(e) => e.stopPropagation()}>
      <h3 id="url-import-title">Import Web Page</h3>
      <p style="font-size: 12px; color: var(--text-muted); margin: 0 0 12px;">Fetch a URL, extract text content, and save to knowledge base.</p>
      <input
        type="url"
        bind:value={importUrl}
        placeholder="https://example.com/article"
        onkeydown={(e) => e.key === "Enter" && handleUrlImport()}
        style="width: 100%; box-sizing: border-box; padding: 8px 10px; background: var(--bg-input); border: 1px solid var(--border); border-radius: 6px; font-size: 13px; color: var(--text-primary); outline: none;"
      />
      <div class="variable-actions" style="margin-top: 12px;">
        <button class="var-btn primary" onclick={handleUrlImport} disabled={!importUrl.trim() || importingUrl}>
          {importingUrl ? "Importing..." : "Import"}
        </button>
        <button class="var-btn" onclick={() => (showUrlImport = false)}>Cancel</button>
      </div>
    </div>
  </div>
{/if}

{#if promptVariableDialog}
  <div class="variable-overlay" onclick={cancelPromptVariables} role="dialog" aria-modal="true" aria-labelledby="var-dialog-title">
    <div class="variable-dialog" onclick={(e) => e.stopPropagation()}>
      <h3 id="var-dialog-title">Fill in Variables</h3>
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

  .token-usage-bar {
    display: flex;
    align-items: center;
    gap: 6px;
    cursor: default;
  }

  .token-bar-track {
    width: 60px;
    height: 6px;
    background: var(--bg-tertiary);
    border-radius: 3px;
    overflow: hidden;
  }

  .token-bar-fill {
    height: 100%;
    border-radius: 3px;
    transition: width 0.3s ease, background 0.3s ease;
    min-width: 1px;
  }

  .token-bar-label {
    font-size: 10px;
    color: var(--text-muted);
    white-space: nowrap;
  }
  .cost-label {
    font-size: 10px;
    color: var(--accent);
    white-space: nowrap;
    font-weight: 500;
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

  .agent-active {
    color: var(--accent) !important;
    background: rgba(78, 204, 163, 0.15);
  }

  .recording {
    color: var(--danger) !important;
    background: rgba(233, 69, 96, 0.15);
    animation: pulse-mic 1.2s ease-in-out infinite;
  }

  @keyframes pulse-mic {
    0%, 100% { opacity: 1; }
    50% { opacity: 0.5; }
  }

  .agent-progress {
    display: flex;
    align-items: center;
    gap: 6px;
    padding: 8px 20px;
    background: var(--bg-secondary);
    border-top: 1px solid var(--border);
  }

  .agent-label {
    font-size: 11px;
    font-weight: 600;
    color: var(--accent);
    text-transform: uppercase;
    letter-spacing: 0.5px;
    margin-right: 4px;
  }

  .agent-step {
    width: 24px;
    height: 24px;
    display: flex;
    align-items: center;
    justify-content: center;
    border-radius: 50%;
    font-size: 11px;
    font-weight: 600;
    background: var(--bg-tertiary);
    color: var(--text-muted);
    transition: all 0.2s;
  }

  .agent-step.done {
    background: var(--accent);
    color: white;
  }

  .agent-step.active {
    background: rgba(78, 204, 163, 0.3);
    color: var(--accent);
    box-shadow: 0 0 0 2px var(--accent);
  }

  .offline-banner {
    padding: 8px 20px;
    background: rgba(255, 165, 0, 0.15);
    color: #e0a030;
    font-size: 12px;
    font-weight: 500;
    text-align: center;
    border-bottom: 1px solid rgba(255, 165, 0, 0.3);
    flex-shrink: 0;
  }

  .queue-count {
    font-weight: 600;
  }

  .load-more-sentinel {
    display: flex;
    justify-content: center;
    padding: 8px;
    flex-shrink: 0;
  }

  .load-more-btn {
    padding: 6px 16px;
    border-radius: 6px;
    font-size: 12px;
    color: var(--text-muted);
    background: var(--bg-tertiary);
    transition: background 0.15s, color 0.15s;
  }

  .load-more-btn:hover {
    color: var(--text-primary);
    background: var(--bg-secondary);
  }

  .loading-indicator {
    font-size: 12px;
    color: var(--text-muted);
    animation: pulse 1s infinite;
  }
</style>
