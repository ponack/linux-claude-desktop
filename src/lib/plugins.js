// ─────────────────────────────────────────────────────────────────────────────
// Plugin host runtime — Phase 14 PR 1
//
// Responsibilities:
//   • Discover installed plugins via the `list_plugins` Tauri command
//   • Load each enabled plugin's source file as an ES module (via Blob URL,
//     since dynamic import of file:// paths is blocked in the webview)
//   • Build a per-plugin `lcd` API object and call the module's `activate(lcd)`
//   • Track activation errors so the Settings UI can surface them
//
// PR 1 is intentionally narrow. The API surface exposed to plugins in this PR:
//   lcd.version    (string)       — host app version
//   lcd.log(...)                   — log to plugin console (visible in Settings)
//   lcd.on(event, handler)         — subscribe to events (no events fire yet)
//   lcd.off(event, handler)
//   lcd.emit(event, payload)       — plugin-to-plugin/host bus
//
// PR 2 will add: lcd.registerCommand, lcd.storage, lcd.notify, lcd.invoke
// PR 3 will emit core events (message:before-send, response:chunk, etc.)
// PR 4 will add: lcd.registerArtifactType
// ─────────────────────────────────────────────────────────────────────────────

import { invoke } from "@tauri-apps/api/core";

const HOST_VERSION = "0.9.4";

// Registry of loaded plugins. Each entry:
//   { id, manifest, module, lcd, logs: [], error?: string }
const loaded = new Map();

// Global event bus — shared across all plugins.
const subscribers = new Map(); // event -> Set<{ pluginId, handler }>

function busOn(pluginId, event, handler) {
  if (!subscribers.has(event)) subscribers.set(event, new Set());
  subscribers.get(event).add({ pluginId, handler });
}

function busOff(pluginId, event, handler) {
  const set = subscribers.get(event);
  if (!set) return;
  for (const sub of set) {
    if (sub.pluginId === pluginId && sub.handler === handler) {
      set.delete(sub);
      break;
    }
  }
}

function busRemoveAllForPlugin(pluginId) {
  for (const set of subscribers.values()) {
    for (const sub of set) if (sub.pluginId === pluginId) set.delete(sub);
  }
}

/**
 * Emit an event to all subscribers. Returns the (possibly mutated) payload.
 * Handlers may return a value to override the payload, or `false` to cancel.
 * Used by PR 3+ to surface lifecycle events; safe to call now (no-op if no subs).
 */
export async function emit(event, payload) {
  const set = subscribers.get(event);
  if (!set || set.size === 0) return payload;
  let current = payload;
  for (const sub of set) {
    try {
      const result = await sub.handler(current);
      if (result === false) return false;
      if (result !== undefined) current = result;
    } catch (e) {
      console.error(`[plugin ${sub.pluginId}] handler for "${event}" threw:`, e);
    }
  }
  return current;
}

function buildLcd(pluginId, logSink) {
  return Object.freeze({
    version: HOST_VERSION,
    pluginId,
    log: (...args) => {
      const msg = args
        .map((a) => (typeof a === "string" ? a : JSON.stringify(a)))
        .join(" ");
      logSink.push({ time: new Date().toISOString(), level: "info", msg });
      // Cap log buffer per plugin
      if (logSink.length > 200) logSink.splice(0, logSink.length - 200);
      // Also mirror to dev console
      console.log(`[plugin ${pluginId}]`, ...args);
    },
    on: (event, handler) => busOn(pluginId, event, handler),
    off: (event, handler) => busOff(pluginId, event, handler),
    emit: (event, payload) => emit(event, payload),
  });
}

async function activatePlugin(info) {
  const { manifest, main_path } = info;
  const id = manifest.id;

  if (loaded.has(id)) return;

  const logs = [];
  const lcd = buildLcd(id, logs);
  const entry = { id, manifest, module: null, lcd, logs, error: null };
  loaded.set(id, entry);

  try {
    const source = await invoke("read_plugin_source", { mainPath: main_path });
    // Load as ES module via Blob URL — the webview blocks file:// imports.
    const blob = new Blob([source], { type: "text/javascript" });
    const url = URL.createObjectURL(blob);
    try {
      const mod = await import(/* @vite-ignore */ url);
      entry.module = mod;
      if (typeof mod.activate === "function") {
        await mod.activate(lcd);
      }
    } finally {
      URL.revokeObjectURL(url);
    }
  } catch (e) {
    entry.error = String(e?.stack || e?.message || e);
    console.error(`[plugin ${id}] failed to activate:`, e);
  }
}

async function deactivatePlugin(id) {
  const entry = loaded.get(id);
  if (!entry) return;
  try {
    if (entry.module && typeof entry.module.deactivate === "function") {
      await entry.module.deactivate();
    }
  } catch (e) {
    console.error(`[plugin ${id}] deactivate threw:`, e);
  }
  busRemoveAllForPlugin(id);
  loaded.delete(id);
}

/**
 * Scan the plugins directory and activate every enabled plugin.
 * Called once at app startup from App.svelte's onMount.
 */
export async function initPlugins() {
  let scan;
  try {
    scan = await invoke("list_plugins");
  } catch (e) {
    console.error("[plugins] list_plugins failed:", e);
    return { plugins: [], errors: [{ path: "(host)", error: String(e) }] };
  }

  for (const info of scan.plugins) {
    if (info.enabled) await activatePlugin(info);
  }
  return scan;
}

/**
 * Re-scan and reconcile enabled state. Used by Settings → Plugins after a
 * toggle, or when the user clicks Refresh.
 */
export async function reloadPlugins() {
  for (const id of Array.from(loaded.keys())) await deactivatePlugin(id);
  return await initPlugins();
}

/**
 * For the Settings UI: return the current state of each loaded plugin,
 * including activation error (if any) and last 50 log lines.
 */
export function getLoadedState() {
  const out = {};
  for (const [id, entry] of loaded) {
    out[id] = {
      error: entry.error,
      logs: entry.logs.slice(-50),
    };
  }
  return out;
}
