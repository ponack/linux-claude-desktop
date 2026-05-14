<script>
  import { invoke } from "@tauri-apps/api/core";
  import { listen } from "@tauri-apps/api/event";
  import { onMount, onDestroy } from "svelte";
  import { Terminal } from "@xterm/xterm";
  import { FitAddon } from "@xterm/addon-fit";
  import "@xterm/xterm/css/xterm.css";

  let { onClose, onSendToChat } = $props();

  let terminalEl = $state(null);
  let statusMsg = $state("Initializing…");
  let isAlive = $state(false);
  let sessionId = crypto.randomUUID();

  let term = null;
  let fitAddon = null;
  let unlistenOutput = null;
  let unlistenExit = null;
  let resizeObserver = null;

  function buildTheme() {
    const style = getComputedStyle(document.documentElement);
    const get = (v) => style.getPropertyValue(v).trim();
    return {
      background:   get("--bg-primary")    || "#1a1b26",
      foreground:   get("--text-primary")  || "#c0caf5",
      cursor:       get("--accent")        || "#7aa2f7",
      selectionBackground: "rgba(122,162,247,0.3)",
      black:   "#414868", red:     "#f7768e",
      green:   "#9ece6a", yellow:  "#e0af68",
      blue:    "#7aa2f7", magenta: "#bb9af7",
      cyan:    "#7dcfff", white:   "#a9b1d6",
      brightBlack:   "#414868", brightRed:     "#f7768e",
      brightGreen:   "#9ece6a", brightYellow:  "#e0af68",
      brightBlue:    "#7aa2f7", brightMagenta: "#bb9af7",
      brightCyan:    "#7dcfff", brightWhite:   "#c0caf5",
    };
  }

  onMount(async () => {
    // Set up xterm
    const fontFamily = '"JetBrains Mono", "Fira Code", "Cascadia Code", monospace';
    term = new Terminal({
      theme: buildTheme(),
      fontFamily,
      fontSize: 14,
      lineHeight: 1.2,
      cursorBlink: true,
      scrollback: 5000,
      allowProposedApi: true,
    });

    fitAddon = new FitAddon();
    term.loadAddon(fitAddon);
    term.open(terminalEl);
    fitAddon.fit();

    // Send keyboard input to backend
    term.onData((data) => {
      if (isAlive) {
        invoke("send_terminal_input", { sessionId, data }).catch(() => {});
      }
    });

    // Listen for PTY output
    unlistenOutput = await listen("terminal-output", (event) => {
      const { session_id, data } = event.payload;
      if (session_id === sessionId) {
        term.write(data);
      }
    });

    // Listen for PTY exit
    unlistenExit = await listen("terminal-exit", (event) => {
      const { session_id, code } = event.payload;
      if (session_id !== sessionId) return;
      isAlive = false;
      statusMsg = `Shell exited (code ${code})`;
      term.write(`\r\n\x1b[33m[Process exited with code ${code}]\x1b[0m\r\n`);
    });

    // Resize observer — fit terminal when container size changes
    resizeObserver = new ResizeObserver(() => {
      fitAddon.fit();
      if (isAlive) {
        invoke("resize_terminal", {
          sessionId,
          cols: term.cols,
          rows: term.rows,
        }).catch(() => {});
      }
    });
    resizeObserver.observe(terminalEl);

    // Spawn the shell
    statusMsg = "Starting shell…";
    try {
      await invoke("spawn_terminal", { sessionId });
      isAlive = true;
      statusMsg = "Connected";
      // Send initial resize to match actual terminal dimensions
      await invoke("resize_terminal", {
        sessionId,
        cols: term.cols,
        rows: term.rows,
      });
    } catch (e) {
      statusMsg = `Error: ${e}`;
      term.write(`\x1b[31mFailed to start terminal: ${e}\x1b[0m\r\n`);
    }

    term.focus();
  });

  onDestroy(() => {
    resizeObserver?.disconnect();
    unlistenOutput?.then?.((fn) => fn());
    unlistenExit?.then?.((fn) => fn());
    if (sessionId) {
      invoke("close_terminal", { sessionId }).catch(() => {});
    }
    term?.dispose();
  });

  function sendSelectionToChat() {
    const sel = term?.getSelection();
    if (sel?.trim()) {
      onSendToChat?.(sel);
    }
  }

  function handleKeydown(e) {
    if (e.key === "Escape") {
      onClose?.();
    }
  }
</script>

<svelte:window onkeydown={handleKeydown} />

<div class="terminal-view">
  <div class="terminal-header">
    <div class="terminal-header-left">
      <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" class="terminal-icon">
        <polyline points="4 17 10 11 4 5"/><line x1="12" y1="19" x2="20" y2="19"/>
      </svg>
      <span class="terminal-title">Terminal</span>
      <span class="terminal-status" class:alive={isAlive}>{statusMsg}</span>
    </div>
    <div class="terminal-header-right">
      <button
        class="header-btn"
        onclick={sendSelectionToChat}
        title="Send selection to Claude chat"
        aria-label="Send selection to Claude chat"
      >
        <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
          <path d="M21 15a2 2 0 0 1-2 2H7l-4 4V5a2 2 0 0 1 2-2h14a2 2 0 0 1 2 2z"/>
        </svg>
        Send to Claude
      </button>
      <button
        class="header-btn close-btn"
        onclick={onClose}
        title="Close terminal (Esc)"
        aria-label="Close terminal"
      >
        ✕
      </button>
    </div>
  </div>

  <div class="terminal-canvas" bind:this={terminalEl}></div>
</div>

<style>
  .terminal-view {
    display: flex;
    flex-direction: column;
    height: 100%;
    background: var(--bg-primary);
  }

  .terminal-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 10px 16px;
    border-bottom: 1px solid var(--border);
    background: var(--bg-secondary);
    flex-shrink: 0;
  }

  .terminal-header-left {
    display: flex;
    align-items: center;
    gap: 8px;
  }

  .terminal-icon {
    color: var(--accent);
  }

  .terminal-title {
    font-size: 14px;
    font-weight: 600;
    color: var(--text-primary);
  }

  .terminal-status {
    font-size: 12px;
    color: var(--text-muted);
    padding: 2px 8px;
    border-radius: 4px;
    background: var(--bg-tertiary);
  }

  .terminal-status.alive {
    color: var(--success);
    background: rgba(78, 204, 163, 0.1);
  }

  .terminal-header-right {
    display: flex;
    align-items: center;
    gap: 6px;
  }

  .header-btn {
    display: flex;
    align-items: center;
    gap: 5px;
    padding: 5px 10px;
    border-radius: 6px;
    font-size: 12px;
    color: var(--text-secondary);
    transition: background 0.15s, color 0.15s;
  }

  .header-btn:hover {
    background: var(--bg-tertiary);
    color: var(--text-primary);
  }

  .close-btn {
    font-size: 14px;
    padding: 5px 8px;
  }

  .terminal-canvas {
    flex: 1;
    min-height: 0;
    padding: 8px;
    overflow: hidden;
  }

  /* Override xterm.js defaults to fill the container */
  .terminal-canvas :global(.xterm) {
    height: 100%;
  }

  .terminal-canvas :global(.xterm-viewport) {
    border-radius: 4px;
  }
</style>
