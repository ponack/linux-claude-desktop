<script>
  import { invoke } from "@tauri-apps/api/core";
  import { listen } from "@tauri-apps/api/event";
  import { getCurrentWindow } from "@tauri-apps/api/window";
  import { register, unregister } from "@tauri-apps/plugin-global-shortcut";
  import { onOpenUrl } from "@tauri-apps/plugin-deep-link";
  import { onMount, onDestroy } from "svelte";
  import Sidebar from "./lib/Sidebar.svelte";
  import Chat from "./lib/Chat.svelte";
  import Settings from "./lib/Settings.svelte";
  import CommandPalette from "./lib/CommandPalette.svelte";

  let currentView = $state("chat");
  let activeConversationId = $state(null);
  let sidebarRefresh = $state(0);
  let deepLinkText = $state("");
  let showCommandPalette = $state(false);

  const SUMMON_SHORTCUT = "Super+Shift+C";
  const QUICKASK_SHORTCUT = "Super+Shift+Q";

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
    } catch (e) {
      console.error("Failed to load theme:", e);
    }

    try {
      await register(SUMMON_SHORTCUT, async () => {
        const win = getCurrentWindow();
        const visible = await win.isVisible();
        if (visible) {
          await win.hide();
        } else {
          await win.show();
          await win.setFocus();
        }
      });
    } catch (e) {
      console.error("Failed to register global shortcut:", e);
    }

    try {
      await register(QUICKASK_SHORTCUT, () => {
        invoke("toggle_quickask");
      });
    } catch (e) {
      console.error("Failed to register quick-ask shortcut:", e);
    }

    try {
      await onOpenUrl((urls) => {
        for (const url of urls) {
          try {
            const parsed = new URL(url);
            if (parsed.pathname === "ask" || parsed.pathname === "/ask") {
              const q = parsed.searchParams.get("q")?.slice(0, 10000);
              if (q) {
                activeConversationId = null;
                currentView = "chat";
                deepLinkText = q;
              }
            }
          } catch (_) {}
        }
        getCurrentWindow().show();
        getCurrentWindow().setFocus();
      });
    } catch (e) {
      console.error("Failed to register deep link handler:", e);
    }

    // Listen for DBus ask events
    listen("dbus-ask", (event) => {
      activeConversationId = null;
      currentView = "chat";
      deepLinkText = event.payload;
    });

    // Listen for scheduled prompts
    listen("scheduled-prompt", (event) => {
      const { prompt } = event.payload;
      if (prompt) {
        activeConversationId = null;
        currentView = "chat";
        deepLinkText = prompt;
      }
    });
  });

  onDestroy(async () => {
    try { await unregister(SUMMON_SHORTCUT); } catch (_) {}
    try { await unregister(QUICKASK_SHORTCUT); } catch (_) {}
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
    // Ctrl+P: Command palette
    if (e.ctrlKey && e.key === "p") {
      e.preventDefault();
      showCommandPalette = !showCommandPalette;
      return;
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
    // Escape: Close palette, settings, or clear search
    if (e.key === "Escape") {
      if (showCommandPalette) {
        showCommandPalette = false;
      } else if (currentView === "settings") {
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
        bind:deepLinkText={deepLinkText}
      />
    {/if}
  </main>
</div>

{#if showCommandPalette}
  <CommandPalette
    onClose={() => (showCommandPalette = false)}
    onSelectConversation={onSelectConversation}
    onNewChat={onNewChat}
    onOpenSettings={openSettings}
  />
{/if}

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
    min-height: 0;
  }
</style>
