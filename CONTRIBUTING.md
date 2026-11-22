# Contributing to Peek

Thank you for your interest in contributing to Peek! Peek is an open-source, minimalist browser built with Rust, native macOS WKWebView, and Svelte 5.

---

## Development Principles

When contributing code or design to Peek, keep these core principles in mind:

1. **Lightweight & Fast:** Peek avoids bloated frameworks and heavy dependencies. We run on native system webviews (`WKWebView`) rather than bundling Chromium or Electron. Keep binary size and runtime memory low.
2. **Distraction-Free UI:** Peek’s interface is designed to get out of the user's way. UI chrome exists only when needed (compact 36px bar or edge-activated drawers).
3. **Safety & Stability:** State synchronization between the host Rust application and child webviews is strictly typed and validated via cryptographic nonces.

---

## Getting Started with Development

### 1. Requirements

- macOS 12+ with Xcode Command Line Tools (`xcode-select --install`)
- Rust stable 1.80+ (`rustup default stable`)
- Node.js 18+ and npm

### 2. Workspace Setup

```bash
git clone https://github.com/senapati484/peek.git
cd peek

# Install and compile the Svelte UI
cd ui
npm install
npm run build
cd ..

# Run all workspace unit tests
cargo test --workspace

# Run Peek in debug mode
cargo run -p peek-app
```

---

## Project Structure

- `crates/peek-app`: Main application entry point, window management (`tao`), event loop, and webview layout.
- `crates/peek-engine`: Engine abstraction over `wry` and native WKWebView child instances.
- `crates/peek-core`: Pure browser domain logic (tabs, LRU discard policy, query resolvers).
- `crates/peek-adblock`: Brave adblock engine wrapper, network request filters, and cosmetic element hiding.
- `crates/peek-preview`: Link hover preview geometry, dwell timing, and bounds calculation.
- `crates/peek-ext`: Manifest V3 extension parser, compatibility checks, and content script registration.
- `crates/peek-store`: SQLite session, visit history, cookies, bookmarks, and FTS5 search.
- `crates/peek-ipc`: Nonce-authenticated JSON IPC message definitions.
- `ui/`: Svelte 5 + TypeScript interface (Omnibar, Drawers, Themes, Settings).

---

## Guidelines for Contributions

### Rust Guidelines
- Follow standard Rust naming conventions and formatting (`cargo fmt`).
- Avoid `unwrap()` or `expect()` in production event handling paths. Prefer robust error logging and graceful fallbacks.
- Keep domain logic in `peek-core` free of OS-specific or UI-specific dependencies so it remains 100% unit-testable.

### Frontend Guidelines (`ui/`)
- Written in modern Svelte 5 using runes (`$state`, `$derived`, `$props`).
- Styles must respect CSS design tokens defined in `ui/src/app.css` (`--bg`, `--surface`, `--accent`, `--text`).
- Ensure dark mode and light mode contrast standards are upheld.
- Avoid animating layout-triggering properties (like `height` or `width`); use GPU-composited properties (`transform`, `opacity`).

---

## Pull Request Checklist

Before submitting a pull request, please verify:

- [ ] All tests pass: `cargo test --workspace`
- [ ] UI builds without errors: `(cd ui && npm run build)`
- [ ] Svelte type checker passes: `(cd ui && npx svelte-check)`
- [ ] Standalone app builds cleanly: `./scripts/build_app.sh`
- [ ] Commit messages are descriptive and follow conventional commits (e.g., `feat:`, `fix:`, `docs:`).

---

## Reporting Issues

If you find a bug or have a feature suggestion, please open an issue on GitHub with:
1. A clear, descriptive title.
2. Steps to reproduce the behavior.
3. Expected vs. actual behavior.
4. macOS version and Peek version/commit hash.
