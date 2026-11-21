# Peek

*A tiny, Rust-built browser with hover previews, a built-in ad blocker, Chrome extension import, and a one-bar UI.*

---

## Overview

Peek is an ultra-lightweight desktop browser for Windows, macOS, and Linux built on one Rust codebase using system WebViews (`tao` + `wry`).

* **Low Memory:** Reuses the operating system's native web engine (WKWebView on macOS, WebView2 on Windows, WebKitGTK on Linux). Discards idle tabs to keep process baseline memory low.
* **Hover Preview:** Hover any link to reveal a live, scaled miniature preview of the target page in an isolated ephemeral webview. Click or press Enter to promote it to an active tab; move away to dismiss.
* **Native Ad Blocker:** Powered by Brave's `adblock` engine (uBlock Origin / EasyList syntax) with network request blocking and per-site cosmetic DOM filtering.
* **Minimalist UI:** A single 36px omnibar and an edge-revealed slide-in drawer. Built with Svelte 5 (Runes) and compiled into a static, zero-virtual-DOM bundle served through `peek://`.
* **Profiles & Session Restore:** Isolated cookies, history with SQLite FTS5 search, bookmarks, and per-site rules.

---

## Workspace Layout

Peek is organized as a Cargo workspace with clean separation between pure browser state, engines, IPC, and UI:

```
peek/
├── Cargo.toml                # Workspace configuration
├── crates/
│   ├── peek-app/             # Main desktop binary, event loop, window layout
│   ├── peek-engine/          # Engine trait and wry backend (child webview glue)
│   ├── peek-core/            # Pure browser logic: tab manager, LRU discard, navigation
│   ├── peek-adblock/         # Brave adblock engine wrapper, WebKit rules, cosmetic filters
│   ├── peek-preview/         # Hover-preview placement geometry and dwell lifecycle
│   ├── peek-ext/             # Chrome Manifest V3 parser and compatibility reports
│   ├── peek-store/           # SQLite store (WAL mode, FTS5 history, sessions, tabs)
│   └── peek-ipc/             # Nonce-bearing typed JSON IPC protocol
├── ui/                       # Svelte 5 + TypeScript + Vite UI bundle
├── docs/                     # Architecture, schema, and OS findings
└── scripts/                  # Memory benchmarks and helper scripts
```

---

## Getting Started

### Prerequisites

* **Rust:** 1.80+ (`rustup default stable`)
* **Node.js & npm:** 18+ (for building UI assets)
* **OS Dependencies:**
  * **macOS:** Xcode command line tools (`xcode-select --install`)
  * **Linux:** `libwebkit2gtk-4.1-dev` (or `4.0`), `libgtk-3-dev`, `libsoup-3.0-dev`
  * **Windows:** WebView2 runtime (pre-installed on Windows 10/11)

### Building and Running

1. **Build the UI bundle:**
   ```bash
   cd ui
   npm install
   npm run build
   cd ..
   ```

2. **Run test suite:**
   ```bash
   cargo test
   ```

3. **Launch Peek:**
   ```bash
   cargo run -p peek-app
   ```

---

## License

MIT
