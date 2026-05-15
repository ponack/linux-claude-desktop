# Linux Claude Desktop — Plugin Guide

Plugins extend LCD with custom slash commands, persistent storage, notifications, and (coming soon) event hooks and custom artifact renderers. They are JavaScript ES modules that the app loads at startup.

> **Phase 14 is shipping in stages.** This document reflects what's live today (PR 1 + PR 2). Sections marked _coming in PR N_ describe API that is not yet wired up.

## Installing a plugin

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

### `invoke` whitelist

Plugins can call these Tauri commands (read-only inspection only — no mutation in PR 2):

`get_app_info`, `get_conversations`, `get_messages`, `get_provider`, `get_model`

The whitelist will grow in PR 3 once event hooks land.

### Command handler return values

When a plugin command runs from the slash-command picker, the handler can:
- Return a **string** → inserted into the chat input.
- Return `{ text, send }` → `text` is inserted; if `send` is truthy, the message is auto-sent.
- Return `undefined` → side-effect only (notify, log, etc.).

Errors thrown by the handler surface in chat as an error bubble — same UX as a failed shell command.

## Coming in later PRs

- **PR 3 — Event hooks**
  - `message:before-send` (mutable — can rewrite outgoing text, attach context)
  - `response:chunk` (observable — streaming token stream)
  - `response:complete` (observable + can append text)
  - `artifact:create`, `artifact:update`
  - `conversation:create`, `conversation:delete`

- **PR 4 — Custom artifact renderers**
  - `lcd.registerArtifactType(typeName, { mimeType, extensions, render(container, content, ctx) })`

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
