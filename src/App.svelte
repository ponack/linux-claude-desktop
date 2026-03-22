<script>
  import { invoke } from "@tauri-apps/api/core";
  import { onMount } from "svelte";
  import Sidebar from "./lib/Sidebar.svelte";
  import Chat from "./lib/Chat.svelte";
  import Settings from "./lib/Settings.svelte";

  let currentView = $state("chat");
  let activeConversationId = $state(null);
  let sidebarRefresh = $state(0);

  onMount(async () => {
    try {
      const theme = await invoke("get_theme");
      document.documentElement.setAttribute("data-theme", theme);

      // Load and apply custom CSS
      const customCss = await invoke("get_custom_css");
      if (customCss) {
        const styleEl = document.createElement("style");
        styleEl.id = "custom-css";
        styleEl.textContent = customCss;
        document.head.appendChild(styleEl);
      }
    } catch (e) {
      console.error("Failed to load theme:", e);
    }
  });

  function onSelectConversation(id) {
    activeConversationId = id;
    currentView = "chat";
  }

  function onNewChat() {
    activeConversationId = null;
    currentView = "chat";
  }

  function onConversationCreated(id) {
    activeConversationId = id;
    sidebarRefresh++;
  }

  function openSettings() {
    currentView = "settings";
  }

  function closeSettings() {
    currentView = "chat";
  }

  function handleGlobalKeydown(e) {
    // Ctrl+N: New chat
    if (e.ctrlKey && e.key === "n") {
      e.preventDefault();
      onNewChat();
    }
    // Ctrl+,: Settings
    if (e.ctrlKey && e.key === ",") {
      e.preventDefault();
      if (currentView === "settings") closeSettings();
      else openSettings();
    }
    // Ctrl+L: Focus chat input
    if (e.ctrlKey && e.key === "l") {
      e.preventDefault();
      const textarea = document.querySelector(".input-wrapper textarea");
      if (textarea) textarea.focus();
    }
    // Ctrl+K: Focus search
    if (e.ctrlKey && e.key === "k") {
      e.preventDefault();
      const search = document.querySelector(".search-box input");
      if (search) search.focus();
    }
    // Escape: Close settings or clear search
    if (e.key === "Escape") {
      if (currentView === "settings") {
        closeSettings();
      }
    }
  }
</script>

<svelte:window onkeydown={handleGlobalKeydown} />

<div class="app-layout">
  <Sidebar
    {activeConversationId}
    onSelect={onSelectConversation}
    {onNewChat}
    {openSettings}
    refreshKey={sidebarRefresh}
  />
  <main class="main-content">
    {#if currentView === "settings"}
      <Settings onClose={closeSettings} />
    {:else}
      <Chat
        conversationId={activeConversationId}
        {onConversationCreated}
      />
    {/if}
  </main>
</div>

<style>
  .app-layout {
    display: flex;
    height: 100vh;
    width: 100vw;
  }

  .main-content {
    flex: 1;
    display: flex;
    flex-direction: column;
    overflow: hidden;
  }
</style>
