<script>
  import { invoke } from "@tauri-apps/api/core";
  import { listen } from "@tauri-apps/api/event";
  import { open } from "@tauri-apps/plugin-dialog";
  import { onMount, tick } from "svelte";
  import MessageBubble from "./MessageBubble.svelte";

  let { conversationId, onConversationCreated } = $props();

  let messages = $state([]);
  let inputText = $state("");
  let isStreaming = $state(false);
  let streamingMessageId = $state(null);
  let attachments = $state([]);
  let messagesContainer;

  async function loadMessages() {
    if (!conversationId) {
      messages = [];
      return;
    }
    try {
      messages = await invoke("get_messages", { conversationId });
    } catch (e) {
      console.error("Failed to load messages:", e);
    }
  }

  $effect(() => {
    conversationId;
    loadMessages();
  });

  onMount(() => {
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
      const apiAttachments = currentAttachments.map(({ path, media_type }) => ({ path, media_type }));
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
        },
      ];
      scrollToBottom();
    }
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

  async function stopGeneration() {
    try {
      await invoke("stop_generation");
    } catch (e) {
      console.error("Failed to stop:", e);
    }
  }

  function handleKeydown(e) {
    if (e.key === "Enter" && !e.shiftKey) {
      e.preventDefault();
      sendMessage();
    }
  }
</script>

<div class="chat-container">
  <div class="messages" bind:this={messagesContainer}>
    {#if messages.length === 0 && !conversationId}
      <div class="empty-state">
        <img src="/assets/logo.svg" alt="UCD" class="empty-logo" />
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
      />
    {/each}
  </div>

  <div class="input-area">
    {#if attachments.length > 0}
      <div class="attachments-preview">
        {#each attachments as att, i}
          <div class="attachment-chip">
            <span class="att-name">{att.name}</span>
            <button class="att-remove" onclick={() => removeAttachment(i)}>x</button>
          </div>
        {/each}
      </div>
    {/if}
    <div class="input-wrapper">
      <button class="attach-btn" onclick={addAttachment} disabled={isStreaming} title="Attach image">
        <svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
          <path d="M21.44 11.05l-9.19 9.19a6 6 0 01-8.49-8.49l9.19-9.19a4 4 0 015.66 5.66l-9.2 9.19a2 2 0 01-2.83-2.83l8.49-8.48"/>
        </svg>
      </button>
      <textarea
        bind:value={inputText}
        onkeydown={handleKeydown}
        placeholder="Message Claude..."
        rows="1"
        disabled={isStreaming}
      ></textarea>
      {#if isStreaming}
        <button class="stop-btn" onclick={stopGeneration}>Stop</button>
      {:else}
        <button
          class="send-btn"
          onclick={sendMessage}
          disabled={!inputText.trim() && attachments.length === 0}
        >
          Send
        </button>
      {/if}
    </div>
  </div>
</div>

<style>
  .chat-container {
    display: flex;
    flex-direction: column;
    height: 100%;
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
</style>
