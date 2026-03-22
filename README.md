<p align="center">
  <img src="assets/logo.svg" alt="Ubuntu Claude Desktop" width="180" />
</p>

<h1 align="center">Ubuntu Claude Desktop</h1>

<p align="center">
  A lightweight, native Claude AI desktop client for Ubuntu/Linux built with Tauri v2 and Svelte 5.
</p>

<p align="center">
  <img src="https://img.shields.io/badge/platform-Linux-blue?logo=linux" alt="Platform" />
  <img src="https://img.shields.io/badge/built_with-Tauri_v2-orange?logo=tauri" alt="Tauri" />
  <img src="https://img.shields.io/badge/frontend-Svelte_5-red?logo=svelte" alt="Svelte" />
  <img src="https://img.shields.io/badge/license-MIT-green" alt="License" />
</p>

---

## Why?

Anthropic's official Claude Desktop app is available for macOS and Windows, but not Linux. Ubuntu Claude Desktop fills that gap with a native, lightweight alternative that uses the Anthropic API directly.

- **~10MB** binary (vs ~150MB for Electron-based alternatives)
- **Native WebKitGTK** rendering (no bundled Chromium)
- **Low memory footprint** thanks to Tauri's Rust backend
- **Your API key, your data** — everything stays local on your machine

## Features

- Streaming chat responses in real-time
- Conversation management (create, rename, delete)
- Persistent conversation history (SQLite)
- Model selection (Opus 4.6, Sonnet 4.6, Haiku 4.5)
- Markdown rendering with syntax-highlighted code blocks
- Copy button on code blocks
- AI-generated conversation titles
- Search/filter conversations
- Custom system prompts
- Image upload with Claude Vision API
- Edit messages and regenerate responses
- Light and dark theme
- System tray integration (minimize to tray)
- LaTeX/math rendering (KaTeX)
- Keyboard shortcuts (Ctrl+N, Ctrl+K, Ctrl+,, Ctrl+L)
- Stop generation mid-stream
- Local API key storage

## Prerequisites

- **Node.js** >= 18
- **Rust** (install via [rustup](https://rustup.rs/))
- **System libraries:**

```bash
sudo apt install -y libwebkit2gtk-4.1-dev libgtk-3-dev libayatana-appindicator3-dev libssl-dev
```

## Getting Started

```bash
# Clone the repo
git clone https://github.com/ponack/ubuntu-claude-desktop.git
cd ubuntu-claude-desktop

# Install dependencies
npm install

# Run in development mode (first build takes a few minutes)
source "$HOME/.cargo/env"  # if Rust was just installed
npm run tauri dev
```

On first launch:
1. Click **Settings** in the sidebar
2. Enter your [Anthropic API key](https://console.anthropic.com/)
3. Choose your preferred model
4. Save, and start chatting

## Install (Pre-built)

Download the latest `.deb` from [Releases](https://github.com/ponack/ubuntu-claude-desktop/releases) and install:

```bash
sudo dpkg -i ubuntu-claude-desktop_*.deb
```

## Building from Source

```bash
npm run tauri build
```

This generates a `.deb` package in `src-tauri/target/release/bundle/deb/` that you can install with `dpkg -i`.

## Project Structure

```
ubuntu-claude-desktop/
├── src/                          # Svelte 5 frontend
│   ├── App.svelte                # Layout: sidebar + main area
│   ├── lib/
│   │   ├── Sidebar.svelte        # Conversation list
│   │   ├── Chat.svelte           # Message list + input + streaming
│   │   ├── MessageBubble.svelte  # Markdown rendering per message
│   │   └── Settings.svelte       # API key + model configuration
│   └── styles/global.css         # Light/dark theme CSS variables
├── src-tauri/                    # Rust backend (Tauri v2)
│   └── src/
│       ├── lib.rs                # App state + command registration
│       ├── api.rs                # Anthropic API streaming (SSE)
│       └── db.rs                 # SQLite: conversations, messages, settings
└── assets/                       # Logo and branding
```

## Roadmap

### Phase 1 — Polish ✅
- [x] Copy button on code blocks
- [x] Syntax highlighting for code
- [x] AI-generated conversation titles
- [x] Search conversations
- [x] Custom system prompts
- [x] Keyboard shortcuts

### Phase 2 — Feature Parity ✅
- [x] File and image upload (vision API)
- [x] Edit and regenerate messages
- [x] Light/dark theme toggle
- [x] System tray integration
- [x] LaTeX/math rendering

### Phase 3 — Power Features
- [ ] Artifacts (sandboxed HTML/React preview)
- [ ] MCP (Model Context Protocol) support
- [ ] Project folders with persistent context
- [ ] Export conversations (Markdown/JSON)
- [ ] Auto-update mechanism

### Phase 4 — Beyond Official
- [ ] Local model support (Ollama)
- [ ] Multi-provider support (OpenAI, etc.)
- [ ] Plugin system
- [ ] Custom CSS themes
- [ ] Prompt library/templates

## Tech Stack

| Layer | Technology |
|-------|-----------|
| Framework | [Tauri v2](https://v2.tauri.app/) |
| Frontend | [Svelte 5](https://svelte.dev/) |
| Backend | Rust |
| Database | SQLite (via rusqlite) |
| API | [Anthropic Messages API](https://docs.anthropic.com/en/api/messages) |
| Build | Vite |

## Contributing

Contributions are welcome! Feel free to open issues or submit pull requests.

## License

MIT

## Disclaimer

This is an unofficial, community-built client. It is not affiliated with or endorsed by Anthropic. "Claude" is a trademark of Anthropic.
