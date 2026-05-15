// ─────────────────────────────────────────────────────────────────────────────
// Plugin host runtime
//
// PR 1: Loader, lifecycle, event bus skeleton.
// PR 2: Interactive API — registerCommand, storage, notify, invoke.
// PR 3 (planned): emit core events into the bus (message:before-send, etc.)
// PR 4 (planned): registerArtifactType for custom artifact renderers.
//
// `lcd` API surface (frozen, passed to each plugin's activate()):
//   version           string  — host app version
//   pluginId          string
//   log(...args)              — append to plugin console (Settings panel)
//   on(event, fn)             — subscribe to host/plugin events
//   off(event, fn)
//   emit(event, payload)      — emit on the bus
//   registerCommand({name, description, handler})
//                             — adds a slash command + palette entry
//   storage.get(key)          — Promise<any | null>
//   storage.set(key, value)   — Promise<void>; value JSON-serialised
//   storage.delete(key)
//   storage.list()            — Promise<string[]>
//   notify(message, level?)   — toast notification (info|success|warning|error)
//   invoke(name, args?)       — call a whitelisted Tauri command
// ─────────────────────────────────────────────────────────────────────────────

import { invoke } from "@tauri-apps/api/core";
import { showToast } from "./toast.js";

const HOST_VERSION = "0.9.4";

// Whitelist of Tauri commands a plugin may call via lcd.invoke().
// Keep this conservative — every entry here is part of the public plugin API.
const INVOKE_WHITELIST = new Set([
  "get_app_info",
  "get_conversations",
  "get_messages",
  "get_provider",
  "get_model",
  "get_conversation_usage",
  "get_total_usage",
  "get_artifacts",
  "get_artifact_content",
]);

// Plugin commands keyed by plugin id, then command name.
// Shape: Map<pluginId, Map<commandName, { description, handler }>>
const pluginCommands = new Map();

// Plugin artifact renderers keyed by plugin id, then by registered type name.
// Shape: Map<pluginId, Map<typeName, { languages, extensions, mimeType, render }>>
const pluginArtifactRenderers = new Map();

// Listeners notified whenever the command registry changes (after activate /
// deactivate / reload). Chat.svelte subscribes so its slash-command picker stays
// in sync.
const commandsChangeListeners = new Set();

export function onPluginCommandsChanged(fn) {
  commandsChangeListeners.add(fn);
  return () => commandsChangeListeners.delete(fn);
}

// Alias — same listeners fire for any registry change (commands AND renderers).
// Prefer this name in new call sites.
export const onPluginRegistryChanged = onPluginCommandsChanged;

function notifyCommandsChanged() {
  for (const fn of commandsChangeListeners) {
    try { fn(); } catch (e) { console.error("[plugins] commands listener threw:", e); }
  }
}

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
 * Emit an event to all subscribers. Returns the (possibly mutated) payload,
 * or `false` if any handler cancelled.
 *
 * Handler return values:
 *   - `false`               → cancel (subsequent handlers do NOT run; emit returns false)
 *   - `undefined`           → pass through unchanged
 *   - partial object        → shallow-merged into the current payload (handy:
 *                             `return { text: "rewritten" }` for message:before-send)
 *   - any other value       → replaces the payload entirely
 *
 * For "observable" events (response:chunk, artifact:create, etc.) the host
 * calls `emit().catch(...)` and ignores the return value; handlers can still
 * subscribe, but mutations have no effect.
 */
export async function emit(event, payload) {
  const set = subscribers.get(event);
  if (!set || set.size === 0) return payload;
  let current = payload;
  for (const sub of set) {
    try {
      const result = await sub.handler(current);
      if (result === false) return false;
      if (result === undefined) continue;
      if (
        current && typeof current === "object" && !Array.isArray(current) &&
        result && typeof result === "object" && !Array.isArray(result)
      ) {
        current = { ...current, ...result };
      } else {
        current = result;
      }
    } catch (e) {
      console.error(`[plugin ${sub.pluginId}] handler for "${event}" threw:`, e);
    }
  }
  return current;
}

function buildLcd(pluginId, logSink, permissions) {
  const has = (perm) => permissions.includes(perm);
  const requirePerm = (perm) => {
    if (!has(perm)) {
      throw new Error(`Plugin "${pluginId}" missing permission: ${perm}. Add it to manifest.json's "permissions" array.`);
    }
  };

  const storage = Object.freeze({
    async get(key) {
      requirePerm("storage");
      const raw = await invoke("plugin_storage_get", { pluginId, key: String(key) });
      if (raw == null) return null;
      try { return JSON.parse(raw); } catch { return raw; }
    },
    async set(key, value) {
      requirePerm("storage");
      const serialised = typeof value === "string" ? JSON.stringify(value) : JSON.stringify(value);
      await invoke("plugin_storage_set", { pluginId, key: String(key), value: serialised });
    },
    async delete(key) {
      requirePerm("storage");
      await invoke("plugin_storage_delete", { pluginId, key: String(key) });
    },
    async list() {
      requirePerm("storage");
      return await invoke("plugin_storage_list_keys", { pluginId });
    },
  });

  return Object.freeze({
    version: HOST_VERSION,
    pluginId,
    log: (...args) => {
      const msg = args
        .map((a) => (typeof a === "string" ? a : JSON.stringify(a)))
        .join(" ");
      logSink.push({ time: new Date().toISOString(), level: "info", msg });
      if (logSink.length > 200) logSink.splice(0, logSink.length - 200);
      console.log(`[plugin ${pluginId}]`, ...args);
    },
    on: (event, handler) => busOn(pluginId, event, handler),
    off: (event, handler) => busOff(pluginId, event, handler),
    emit: (event, payload) => emit(event, payload),
    registerCommand: ({ name, description, handler }) => {
      requirePerm("commands");
      if (!name || typeof name !== "string") throw new Error("registerCommand: name required");
      if (typeof handler !== "function") throw new Error("registerCommand: handler must be a function");
      if (!pluginCommands.has(pluginId)) pluginCommands.set(pluginId, new Map());
      pluginCommands.get(pluginId).set(name, { description: description || "", handler });
    },
    registerArtifactType: (typeName, opts = {}) => {
      requirePerm("artifacts");
      if (!typeName || typeof typeName !== "string") {
        throw new Error("registerArtifactType: typeName required");
      }
      if (typeof opts.render !== "function") {
        throw new Error("registerArtifactType: opts.render must be a function");
      }
      const entry = {
        languages: Array.isArray(opts.languages) ? opts.languages.map((s) => s.toLowerCase()) : [],
        extensions: Array.isArray(opts.extensions) ? opts.extensions.map((s) => s.toLowerCase()) : [],
        mimeType: opts.mimeType || "",
        render: opts.render,
      };
      if (!pluginArtifactRenderers.has(pluginId)) pluginArtifactRenderers.set(pluginId, new Map());
      pluginArtifactRenderers.get(pluginId).set(typeName.toLowerCase(), entry);
    },
    storage,
    notify: (message, level = "info") => {
      requirePerm("notify");
      showToast(message, level);
    },
    invoke: async (cmd, args) => {
      requirePerm("invoke");
      if (!INVOKE_WHITELIST.has(cmd)) {
        throw new Error(`lcd.invoke: "${cmd}" is not whitelisted. Allowed: ${Array.from(INVOKE_WHITELIST).join(", ")}`);
      }
      return await invoke(cmd, args || {});
    },
  });
}

async function activatePlugin(info) {
  const { manifest, main_path } = info;
  const id = manifest.id;

  if (loaded.has(id)) return;

  const logs = [];
  const permissions = Array.isArray(manifest.permissions) ? manifest.permissions : [];
  const lcd = buildLcd(id, logs, permissions);
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
  pluginCommands.delete(id);
  pluginArtifactRenderers.delete(id);
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
  notifyCommandsChanged();
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

/**
 * Flat list of every command currently registered by every active plugin.
 * Each entry: { pluginId, pluginName, name, description, handler }.
 * Consumed by Chat.svelte's slash-command picker and CommandPalette.
 */
export function getPluginCommands() {
  const out = [];
  for (const [pluginId, cmds] of pluginCommands) {
    const entry = loaded.get(pluginId);
    const pluginName = entry?.manifest?.name || pluginId;
    for (const [name, { description, handler }] of cmds) {
      out.push({ pluginId, pluginName, name, description, handler });
    }
  }
  return out;
}

/**
 * Invoke a plugin command by (pluginId, name). Returns whatever the handler
 * returns. Errors are caught and rethrown so callers can show a chat-bubble
 * error like they do for shell command failures.
 */
export async function runPluginCommand(pluginId, name, args) {
  const cmds = pluginCommands.get(pluginId);
  if (!cmds || !cmds.has(name)) {
    throw new Error(`Plugin command not found: ${pluginId}/${name}`);
  }
  const { handler } = cmds.get(name);
  return await handler(args);
}

/**
 * Find a plugin artifact renderer that matches an artifact.
 *
 * Match priority (first wins):
 *   1. Exact match against the registered typeName == artifact.artifact_type
 *   2. artifact.language in the renderer's `languages` array
 *   3. artifact.title ends with one of the renderer's `extensions`
 *
 * Returns { pluginId, pluginName, typeName, render } or null.
 */
export function resolveArtifactRenderer(artifact) {
  if (!artifact) return null;
  const t = (artifact.artifact_type || "").toLowerCase();
  const l = (artifact.language || "").toLowerCase();
  const title = (artifact.title || "").toLowerCase();

  for (const [pluginId, byType] of pluginArtifactRenderers) {
    const pluginName = loaded.get(pluginId)?.manifest?.name || pluginId;
    // 1. Match by typeName
    if (t && byType.has(t)) {
      const entry = byType.get(t);
      return { pluginId, pluginName, typeName: t, render: entry.render };
    }
    // 2. Match by language
    if (l) {
      for (const [typeName, entry] of byType) {
        if (entry.languages.includes(l)) {
          return { pluginId, pluginName, typeName, render: entry.render };
        }
      }
    }
    // 3. Match by title extension
    if (title) {
      for (const [typeName, entry] of byType) {
        if (entry.extensions.some((ext) => title.endsWith(ext))) {
          return { pluginId, pluginName, typeName, render: entry.render };
        }
      }
    }
  }
  return null;
}
