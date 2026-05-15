# Linux Claude Desktop — Plugin Guide

Plugins extend LCD with custom commands, renderers, and event hooks. They are JavaScript ES modules that the app loads at startup.

> **Phase 14 is shipping in stages.** This document reflects what's live today (PR 1). Sections marked _coming in PR N_ describe API that is not yet wired up.

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
  "permissions": [],
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
| `permissions` | no | List of permission strings (enforced in PR 2). |
| `hooks` | no | List of event hooks the plugin subscribes to (advisory; enforced in PR 3). |

## Plugin entry point

Your `main` file is an ES module that exports `activate` and optionally `deactivate`:

```js
export function activate(lcd) {
  lcd.log("plugin started", "host version:", lcd.version);

  // Subscribe to events (no events fire yet — coming in PR 3).
  lcd.on("response:complete", (payload) => {
    lcd.log("Claude replied:", payload.text.length, "chars");
  });
}

export function deactivate() {
  // Optional cleanup. Event subscriptions are auto-removed.
}
```

## The `lcd` object (PR 1 surface)

| Member | Description |
|---|---|
| `lcd.version` | Host app version string. |
| `lcd.pluginId` | Your plugin's `id` from the manifest. |
| `lcd.log(...args)` | Append to your plugin's console (visible in Settings → Plugins). |
| `lcd.on(event, handler)` | Subscribe to a host or plugin event. |
| `lcd.off(event, handler)` | Unsubscribe. |
| `lcd.emit(event, payload)` | Emit an event to other plugins and the host. |

## Coming in later PRs

- **PR 2 — Interactive API**
  - `lcd.registerCommand(name, { description, handler })` — adds a slash command + command-palette entry.
  - `lcd.storage.get/set/delete/list(key)` — per-plugin namespaced key/value persistence.
  - `lcd.notify(level, message)` — toast notification.
  - `lcd.invoke(allowedCommand, args)` — call a whitelisted Tauri command.

- **PR 3 — Event hooks** (the host will emit these into the bus)
  - `message:before-send` (mutable — can rewrite outgoing text, attach context).
  - `response:chunk` (observable — streaming token stream).
  - `response:complete` (observable + can append text).
  - `artifact:create`, `artifact:update`.
  - `conversation:create`, `conversation:delete`.

- **PR 4 — Custom artifact renderers**
  - `lcd.registerArtifactType(typeName, { mimeType, extensions, render(container, content, ctx) })`.

## Security model

Plugins are user-installed local files. The webview is already trusted, so plugins run with the same DOM access LCD itself has. Mitigations:

- The `lcd` object is the only sanctioned API — direct Tauri `invoke()` calls from a plugin will fail because the plugin is loaded via Blob URL with no access to `@tauri-apps/api`.
- File reads are constrained to the plugins directory.
- Each plugin can be disabled instantly from Settings without restarting.
- A plugin that throws on activation is isolated — its error is captured and surfaced in Settings instead of crashing the app.

If you're publishing a plugin, audit its source like you would any extension before sharing.

## Example: hello-world

See [`docs/plugins/examples/hello-world/`](plugins/examples/hello-world/) for a minimal working plugin.
