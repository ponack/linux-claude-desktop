<script>
  import { invoke } from "@tauri-apps/api/core";
  import { onMount } from "svelte";
  import Chat from "./Chat.svelte";

  let { conversationId } = $props();
  let title = $state("Chat");

  onMount(async () => {
    try {
      const theme = await invoke("get_theme");
      document.documentElement.setAttribute("data-theme", theme);

      const customCss = await invoke("get_custom_css");
      if (customCss) {
        const styleEl = document.createElement("style");
        styleEl.id = "custom-css";
        styleEl.textContent = customCss;
        document.head.appendChild(styleEl);
      }

      // Load conversation title
      const convs = await invoke("get_conversations");
      const conv = convs.find(c => c.id === conversationId);
      if (conv) title = conv.title;
    } catch (e) {
      console.error("Failed to load:", e);
    }
  });

  function onConversationCreated() {
    // No-op for popped-out windows
  }
</script>

<div class="chat-window">
  <div class="window-header">
    <span class="window-title">{title}</span>
  </div>
  <div class="window-body">
    <Chat {conversationId} {onConversationCreated} />
  </div>
</div>

<style>
  .chat-window {
    display: flex;
    flex-direction: column;
    height: 100vh;
    width: 100vw;
  }

  .window-header {
    padding: 8px 16px;
    background: var(--bg-secondary);
    border-bottom: 1px solid var(--border);
    flex-shrink: 0;
  }

  .window-title {
    font-size: 13px;
    font-weight: 600;
    color: var(--text-primary);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .window-body {
    flex: 1;
    display: flex;
    flex-direction: column;
    overflow: hidden;
  }
</style>
