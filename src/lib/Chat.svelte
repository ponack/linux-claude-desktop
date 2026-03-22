<script>
  import { invoke } from "@tauri-apps/api/core";
  import { listen } from "@tauri-apps/api/event";
  import { onMount, tick } from "svelte";
  import MessageBubble from "./MessageBubble.svelte";

  let { conversationId, onConversationCreated } = $props();

  let messages = $state([]);
  let inputText = $state("");
  let isStreaming = $state(false);
  let streamingMessageId = $state(null);
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
        // Show error as a system message
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

  async function sendMessage() {
    const text = inputText.trim();
    if (!text || isStreaming) return;

    inputText = "";
    let convId = conversationId;

    // Create conversation if needed
    let isNewConversation = false;
    if (!convId) {
      try {
        const title = text.length > 40 ? text.substring(0, 40) + "..." : text;
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
      const assistantMsgId = await invoke("send_message", {
        conversationId: convId,
        content: text,
      });

      streamingMessageId = assistantMsgId;

      // Reload messages to get the saved user message + placeholder
      await loadMessages();
      scrollToBottom();

      // Generate AI title for new conversations (fire and forget)
      if (isNewConversation) {
        invoke("generate_title", {
          conversationId: convId,
          userMessage: text,
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
        <h2>Ubuntu Claude Desktop</h2>
        <p>Start a conversation by typing a message below.</p>
      </div>
    {/if}

    {#each messages as message (message.id)}
      <MessageBubble
        role={message.role}
        content={message.content}
        isStreaming={isStreaming && message.id === streamingMessageId}
      />
    {/each}
  </div>

  <div class="input-area">
    <div class="input-wrapper">
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
          disabled={!inputText.trim()}
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

  .input-wrapper {
    display: flex;
    align-items: flex-end;
    gap: 8px;
    background: var(--bg-input);
    border: 1px solid var(--border);
    border-radius: 12px;
    padding: 8px 12px;
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
