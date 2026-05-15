# Linux Claude Desktop — Plugin Guide

Plugins extend LCD with custom slash commands, persistent storage, notifications, event hooks, and custom artifact renderers. They are JavaScript ES modules that the app loads at startup.

> **Phase 14 is complete.** Everything documented here is live as of v0.9.5+.

## Installing a plugin

There are two ways:

### From a URL (recommended)

In **Settings → Plugins → Install from URL**, paste a direct link to a plugin `.zip` and click **Install**. The zip must contain a `manifest.json` at the root or inside a single top-level folder (GitHub release archives are fine — LCD strips the top-level folder automatically). Max 20 MB.

After install, the plugin is unpacked into the plugins directory under its manifest `id`, and the runtime reloads so it activates immediately. Reinstalling the same `id` replaces the previous version (kept plugin data is **not** wiped).

### Manually

1. Open **Settings → Plugins → Open Plugins Folder** (or `~/.local/share/linux-claude-desktop/plugins/`).
2. Copy a plugin folder into it. Each plugin lives in its own subdirectory:

   ```
   ~/.local/share/linux-claude-desktop/plugins/
     my-plugin/
       manifest.json
       index.js
   ```

3. Click **Reload Plugins** in Settings, or restart LCD.
4. Toggle the plugin **on** in Settings → Plugins.

## Uninstalling a plugin

Click the **Uninstall** button next to the plugin in Settings → Plugins. The plugin folder is deleted and its enable/disable state is cleared. Stored plugin data (anything written via `lcd.storage`) is **kept** so reinstalling later recovers it; to wipe data too, delete the rows manually via DB tools or just reinstall under a different `id`.

## Manifest schema (`manifest.json`)

```json
{
  "id": "hello-world",
  "name": "Hello World",
  "version": "0.1.0",
  "author": "Your Name",
  "description": "A short description shown in Settings.",
  "main": "index.js",
  "permissions": ["commands", "storage", "notify"],
  "hooks": []
}
```

| Field | Required | Description |
|---|---|---|
| `id` | yes | Unique slug. Used for storage namespacing and toggle state. |
| `name` | yes | Display name shown in Settings. |
| `version` | yes | Semver string. |
| `author` | no | Shown under the plugin name. |
| `description` | no | Shown in Settings. |
| `main` | no | Entry-point file relative to plugin folder. Default: `index.js`. |
| `permissions` | no | List of permission strings — see [Permissions](#permissions) below. |
| `hooks` | no | List of event hooks the plugin subscribes to (advisory; enforced in PR 3). |

## Permissions

Add the permission to the manifest's `permissions` array; otherwise the matching `lcd.*` call throws.

| Permission | Grants |
|---|---|
| `commands` | `lcd.registerCommand(...)` |
| `storage` | `lcd.storage.get/set/delete/list(...)` |
| `notify` | `lcd.notify(...)` |
| `invoke` | `lcd.invoke(name, args)` — limited to a whitelist |
| `artifacts` | `lcd.registerArtifactType(...)` |

`lcd.log` and the event bus (`on`/`off`/`emit`) require no permission.

## Plugin entry point

Your `main` file is an ES module that exports `activate` and optionally `deactivate`:

```js
export function activate(lcd) {
  lcd.log("plugin started", "host version:", lcd.version);

  lcd.registerCommand({
    name: "greet",
    description: "Inserts a friendly greeting",
    handler: () => "Hello, Claude! How are you today?",
  });
}

export function deactivate() {
  // Optional cleanup. Commands and event subscriptions are auto-removed.
}
```

## The `lcd` object

| Member | Purpose | Permission |
|---|---|---|
| `lcd.version` | Host app version string. | — |
| `lcd.pluginId` | Your plugin's manifest `id`. | — |
| `lcd.log(...args)` | Append to your plugin's console (visible in Settings). | — |
| `lcd.on(event, handler)` | Subscribe to a host or plugin event. | — |
| `lcd.off(event, handler)` | Unsubscribe. | — |
| `lcd.emit(event, payload)` | Emit an event to other plugins and the host. | — |
| `lcd.registerCommand({ name, description, handler })` | Add a slash command + command-palette entry. | `commands` |
| `lcd.storage.get(key)` | Read a JSON value. Returns `null` if missing. | `storage` |
| `lcd.storage.set(key, value)` | Persist a JSON-serialisable value. | `storage` |
| `lcd.storage.delete(key)` | Remove a key. | `storage` |
| `lcd.storage.list()` | Return all keys this plugin has stored. | `storage` |
| `lcd.notify(message, level?)` | Toast notification. `level` ∈ `info` (default), `success`, `warning`, `error`. | `notify` |
| `lcd.invoke(name, args?)` | Call a whitelisted Tauri command. | `invoke` |
| `lcd.registerArtifactType(name, { languages?, extensions?, mimeType?, render })` | Add a custom artifact renderer. See [Custom artifact renderers](#custom-artifact-renderers). | `artifacts` |

### `invoke` whitelist

Plugins can call these Tauri commands (read-only inspection only — no mutation):

`get_app_info`, `get_conversations`, `get_messages`, `get_provider`, `get_model`, `get_conversation_usage`, `get_total_usage`, `get_artifacts`, `get_artifact_content`

### Command handler return values

When a plugin command runs from the slash-command picker, the handler can:
- Return a **string** → inserted into the chat input.
- Return `{ text, send }` → `text` is inserted; if `send` is truthy, the message is auto-sent.
- Return `undefined` → side-effect only (notify, log, etc.).

Errors thrown by the handler surface in chat as an error bubble — same UX as a failed shell command.

## Event hooks

Subscribe via `lcd.on(event, handler)`. Hooks fire in the order plugins are loaded.

| Event | Kind | Payload | Notes |
|---|---|---|---|
| `message:before-send` | mutable | `{ text, conversationId, attachmentCount }` | Return `false` to cancel the send, or return `{ text: "rewritten" }` to override the outgoing text. |
| `response:chunk` | observable | `{ content, messageId, conversationId }` | Fires per streaming delta. Return values are ignored — host uses fire-and-forget so handlers don't slow the stream. |
| `response:complete` | observable | `{ text, messageId, conversationId }` | Fires once when Claude finishes streaming. `text` is the full assistant response. |
| `artifact:create` | observable | `{ id, conversationId, artifactType, language, title, source }` | `source` is `claude`, `template`, or `user_edit`. |
| `artifact:update` | observable | `{ id, source }` | Fires on edits and reverts. |
| `conversation:create` | observable | `{ id, title }` | Fires on the first user message in a fresh chat. |
| `conversation:delete` | observable | `{ id }` | |

### Handler return-value contract

- Return `false` → **cancel** the event (`message:before-send` only — observable events ignore this).
- Return `undefined` → pass through unchanged.
- Return a **partial object** → shallow-merged into the current payload. So `return { text: "new" }` works without you having to spread the rest.
- Return **any other value** → replaces the payload entirely.

If multiple plugins subscribe to the same event, they run in load order and each sees the previous handler's mutations.

## Custom artifact renderers

A plugin can render artifacts that the built-in renderers don't handle — `.excalidraw` diagrams, `.csv` tables, `.geojson` maps, anything you want.

```js
lcd.registerArtifactType("csv", {
  languages: ["csv"],          // match against artifact.language
  extensions: [".csv"],        // match against artifact.title suffix
  mimeType: "text/csv",        // informational
  render(container, content, ctx) {
    // container — an empty div the plugin owns
    // content   — the artifact text
    // ctx       — { id, conversationId, language, title, artifactType }

    const rows = content.split("\n").map((r) => r.split(","));
    container.innerHTML =
      "<table>" +
      rows.map((cells) =>
        "<tr>" + cells.map((c) => `<td>${c}</td>`).join("") + "</tr>"
      ).join("") +
      "</table>";

    // Optional: return a cleanup function that runs on unmount or
    // before re-rendering when content changes.
    return () => { /* dispose listeners, intervals, etc. */ };
  },
});
```

### Match priority

When the artifact panel decides which renderer to use, it walks every registered plugin renderer in this order. The first match wins:

1. The renderer's `typeName` equals `artifact.artifact_type`
2. `artifact.language` is in the renderer's `languages` array
3. `artifact.title` ends with one of the renderer's `extensions`

If no plugin matches, LCD falls back to its built-in renderers (Mermaid, Markdown, React, HTML/SVG, code).

### Reactivity and cleanup

The renderer is re-mounted whenever `content` or `ctx` changes. If your `render` returns a function, LCD calls it as a cleanup hook before each re-mount and on unmount — use it to dispose listeners, intervals, web workers, anything that would otherwise leak.

Errors thrown inside `render` are caught and displayed inline in the artifact panel; they do not break the rest of the app.

## Security model

Plugins are user-installed local files. The webview is already trusted, so plugins run with the same DOM access LCD itself has. Mitigations:

- The `lcd` object is the only sanctioned API. Direct Tauri `invoke()` calls from a plugin fail because the plugin is loaded via Blob URL with no access to `@tauri-apps/api`.
- Permission gating: missing manifest permission throws on first API call.
- `lcd.invoke` only forwards to a small read-only whitelist.
- File reads are constrained to the plugins directory.
- Each plugin can be disabled instantly from Settings without restarting.
- A plugin that throws on activation is isolated — its error is captured and surfaced in Settings instead of crashing the app.

If you're publishing a plugin, audit its source like you would any extension before sharing.

## Example: hello-world

See [`docs/plugins/examples/hello-world/`](plugins/examples/hello-world/) for a working plugin that demonstrates `registerCommand`, `storage`, and `notify`.
