# Contributing to Linux Claude Desktop

Thanks for your interest in contributing! Here's how to get started.

## Development Setup

### Prerequisites

- **Node.js** >= 18
- **Rust** (install via [rustup](https://rustup.rs/))
- **System libraries:**

```bash
sudo apt install -y libwebkit2gtk-4.1-dev libgtk-3-dev libayatana-appindicator3-dev libssl-dev
```

### Getting Started

```bash
git clone https://github.com/ponack/linux-claude-desktop.git
cd linux-claude-desktop
npm install
source "$HOME/.cargo/env"  # if Rust was just installed
npm run tauri dev
```

The first build takes a few minutes while Rust compiles the backend. Subsequent builds are fast thanks to incremental compilation.

## Making Changes

1. **Fork the repo** and create a feature branch from `main`
2. **Make your changes** — keep PRs focused on a single concern
3. **Test your changes** — run `npm run tauri dev` and verify the app works
4. **Build check** — run `npx vite build` (frontend) and `cargo check --manifest-path src-tauri/Cargo.toml` (backend)
5. **Submit a PR** with a clear description of what and why

## Code Style

- **Frontend**: Svelte 5 with runes (`$state`, `$derived`, `$effect`, `$props`). No class components.
- **Backend**: Standard Rust formatting (`cargo fmt`). Use `#[tauri::command]` for frontend-callable functions.
- **CSS**: Scoped `<style>` blocks in Svelte components. Use CSS variables from `global.css` for theming.
- **Commits**: Short, descriptive messages. Use imperative mood ("Add feature" not "Added feature").

## What to Contribute

- Bug fixes (check [Issues](https://github.com/ponack/linux-claude-desktop/issues))
- Features from the [Roadmap](README.md#roadmap)
- Distro packaging (AUR, Flatpak, Snap, etc.)
- Documentation improvements
- Accessibility improvements
- Translations

## Architecture Overview

```
src/                    Svelte 5 frontend (UI, components, state)
src-tauri/src/          Rust backend (API calls, DB, system integration)
  lib.rs                App setup, command registration, tray icon
  api.rs                Multi-provider streaming (Anthropic, OpenAI, Ollama)
  db.rs                 SQLite database (conversations, settings, artifacts)
  mcp.rs                Model Context Protocol client
  dbus_service.rs       DBus interface for scripting
```

Frontend and backend communicate via Tauri's `invoke()` command system. The frontend calls Rust functions; the backend streams responses via Tauri events.

## Reporting Issues

- Use the [bug report template](https://github.com/ponack/linux-claude-desktop/issues/new?template=bug_report.yml) for bugs
- Use the [feature request template](https://github.com/ponack/linux-claude-desktop/issues/new?template=feature_request.yml) for ideas
- Check existing issues before opening a duplicate

## License

By contributing, you agree that your contributions will be licensed under the [MIT License](LICENSE).
