<script>
  import { makeApi } from "../lib/api.js";
  import { enqueue, getAll, remove, countForConversation } from "../lib/queue.js";
  import { tick } from "svelte";

  let { conn, conversationId, isOnline = true, onBack, onRegisterFlushCallback } = $props();

  const api = $derived(makeApi(conn));

  let conversation = $state(null);
  let messages = $state([]);
  let loading = $state(true);
  let error = $state("");
  let input = $state("");
  let sending = $state(false);
  let pendingCount = $state(0);
  $effect(() => { pendingCount = countForConversation(conversationId); });
  let msgListEl;

  // Register a callback so App.svelte can tell us when the queue flushed
  $effect(() => {
    onRegisterFlushCallback?.(() => {
      pendingCount = countForConversation(conversationId);
      load();
    });
  });

  async function load() {
    loading = true;
    error = "";
    try {
      const data = await api.getConversation(conversationId);
      conversation = { id: data.id, title: data.title, created_at: data.created_at, updated_at: data.updated_at };
      messages = data.messages || [];
      await tick();
      scrollToBottom();
    } catch (e) {
      error = e.message;
    } finally {
      loading = false;
    }
  }

  function scrollToBottom() {
    if (msgListEl) {
      msgListEl.scrollTop = msgListEl.scrollHeight;
    }
  }

  async function send() {
    const text = input.trim();
    if (!text || sending) return;
    input = "";

    // Offline: queue and show locally as pending
    if (!isOnline) {
      enqueue(conversationId, text);
      pendingCount = countForConversation(conversationId);
      messages = [...messages, {
        id: `pending_${Date.now()}`,
        role: "user",
        content: text,
        created_at: new Date().toISOString(),
        pending: true,
      }];
      await tick();
      scrollToBottom();
      return;
    }

    sending = true;
    error = "";

    const optimistic = {
      id: `opt_${Date.now()}`,
      role: "user",
      content: text,
      created_at: new Date().toISOString(),
    };
    messages = [...messages, optimistic];
    await tick();
    scrollToBottom();

    try {
      const assistantMsg = await api.sendMessage(conversationId, text);
      messages = messages.filter((m) => m.id !== optimistic.id);
      messages = [
        ...messages,
        { ...optimistic, id: optimistic.id + "_confirmed" },
        assistantMsg,
      ];
      await tick();
      scrollToBottom();
    } catch (e) {
      // Network failure mid-attempt: queue it
      messages = messages.filter((m) => m.id !== optimistic.id);
      enqueue(conversationId, text);
      pendingCount = countForConversation(conversationId);
      messages = [...messages, {
        id: `pending_${Date.now()}`,
        role: "user",
        content: text,
        created_at: new Date().toISOString(),
        pending: true,
      }];
      await tick();
      scrollToBottom();
    } finally {
      sending = false;
    }
  }

  function onKeydown(e) {
    if (e.key === "Enter" && !e.shiftKey) {
      e.preventDefault();
      send();
    }
  }

  $effect(() => {
    if (conversationId) load();
  });
</script>

<div class="screen">
  <header class="topbar">
    <button class="back-btn" onclick={onBack} aria-label="Back">
      <svg width="22" height="22" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5">
        <polyline points="15 18 9 12 15 6"/>
      </svg>
    </button>
    <h2 class="title">{conversation?.title ?? "…"}</h2>
    <div style="width:40px;"></div>
  </header>

  <div class="messages" bind:this={msgListEl}>
    {#if loading}
      <div class="state-msg">Loading…</div>
    {:else if error}
      <div class="state-msg error">{error}</div>
    {:else if messages.length === 0}
      <div class="state-msg">No messages yet. Say hello!</div>
    {:else}
      {#each messages as msg (msg.id)}
        <div class="bubble-wrap" class:user={msg.role === "user"}>
          <div class="bubble" class:user={msg.role === "user"} class:assistant={msg.role === "assistant"} class:pending={msg.pending}>
            {msg.content}
          </div>
        </div>
      {/each}
      {#if sending}
        <div class="bubble-wrap">
          <div class="bubble assistant typing">
            <span></span><span></span><span></span>
          </div>
        </div>
      {/if}
    {/if}
    {#if pendingCount > 0 && !isOnline}
      <div class="pending-notice">
        {pendingCount} message{pendingCount > 1 ? "s" : ""} queued — will send when online
      </div>
    {/if}
  </div>

  <div class="composer">
    <textarea
      bind:value={input}
      onkeydown={onKeydown}
      placeholder="Message…"
      rows="1"
      disabled={sending}
      aria-label="Message input"
    ></textarea>
    <button class="send-btn" onclick={send} disabled={!input.trim() || sending} aria-label="Send">
      <svg width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5">
        <line x1="22" y1="2" x2="11" y2="13"/><polygon points="22 2 15 22 11 13 2 9 22 2"/>
      </svg>
    </button>
  </div>
</div>

<style>
  .screen {
    display: flex;
    flex-direction: column;
    height: 100dvh;
    background: var(--bg);
  }

  .topbar {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 12px 8px 12px 4px;
    padding-top: max(12px, env(safe-area-inset-top));
    border-bottom: 1px solid var(--border);
    flex-shrink: 0;
  }

  .back-btn {
    width: 40px;
    height: 40px;
    border-radius: 10px;
    display: flex;
    align-items: center;
    justify-content: center;
    color: var(--accent);
    flex-shrink: 0;
  }

  .back-btn:active {
    background: var(--surface);
  }

  .title {
    flex: 1;
    font-size: 16px;
    font-weight: 600;
    margin: 0;
    color: var(--text);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    text-align: center;
  }

  .messages {
    flex: 1;
    overflow-y: auto;
    padding: 16px 12px;
    display: flex;
    flex-direction: column;
    gap: 8px;
    -webkit-overflow-scrolling: touch;
  }

  .state-msg {
    text-align: center;
    color: var(--muted);
    font-size: 14px;
    padding: 32px 0;
  }

  .state-msg.error {
    color: #e94560;
  }

  .bubble-wrap {
    display: flex;
    justify-content: flex-start;
  }

  .bubble-wrap.user {
    justify-content: flex-end;
  }

  .bubble {
    max-width: 80%;
    padding: 10px 14px;
    border-radius: 18px;
    font-size: 15px;
    line-height: 1.5;
    white-space: pre-wrap;
    word-break: break-word;
  }

  .bubble.user {
    background: var(--accent);
    color: #fff;
    border-bottom-right-radius: 4px;
  }

  .bubble.user.pending {
    opacity: 0.6;
  }

  .bubble.assistant {
    background: var(--surface);
    color: var(--text);
    border-bottom-left-radius: 4px;
  }

  .pending-notice {
    text-align: center;
    font-size: 12px;
    color: var(--warning, #e8a838);
    padding: 8px 0 4px;
  }

  /* Typing indicator dots */
  .bubble.typing {
    display: flex;
    gap: 4px;
    align-items: center;
    padding: 12px 16px;
  }

  .bubble.typing span {
    width: 7px;
    height: 7px;
    border-radius: 50%;
    background: var(--muted);
    animation: dot-bounce 1.2s infinite;
  }

  .bubble.typing span:nth-child(2) { animation-delay: 0.2s; }
  .bubble.typing span:nth-child(3) { animation-delay: 0.4s; }

  @keyframes dot-bounce {
    0%, 80%, 100% { transform: translateY(0); opacity: 0.4; }
    40% { transform: translateY(-5px); opacity: 1; }
  }

  .composer {
    display: flex;
    align-items: flex-end;
    gap: 8px;
    padding: 10px 12px;
    padding-bottom: max(10px, env(safe-area-inset-bottom));
    border-top: 1px solid var(--border);
    background: var(--bg);
    flex-shrink: 0;
  }

  textarea {
    flex: 1;
    padding: 10px 14px;
    background: var(--surface);
    border: 1px solid var(--border);
    border-radius: 20px;
    color: var(--text);
    font-size: 15px;
    resize: none;
    max-height: 120px;
    overflow-y: auto;
    line-height: 1.4;
    -webkit-appearance: none;
    font-family: inherit;
  }

  textarea:focus {
    outline: none;
    border-color: var(--accent);
  }

  .send-btn {
    width: 40px;
    height: 40px;
    border-radius: 50%;
    background: var(--accent);
    color: #fff;
    display: flex;
    align-items: center;
    justify-content: center;
    flex-shrink: 0;
    transition: opacity 0.15s;
  }

  .send-btn:disabled {
    opacity: 0.4;
  }

  .send-btn:not(:disabled):active {
    opacity: 0.8;
  }
</style>
