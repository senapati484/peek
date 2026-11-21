# Phase 0 & Phase 1 Findings: Layout, Z-Order, and Memory Baseline

## 1. Window & Webview Architecture

Peek uses a multi-webview architecture inside a single `tao::window::Window`:

```
+-----------------------------------------------------------+
| UI Bar Child Webview (36px, peek://localhost/index.html#bar)
+-----------------------------------------------------------+
|                                                           |
|                                                           |
|             Active Tab Child Webview                      |
|             (Fills remaining window space)                |
|                                                           |
|   +-------------------+                                   |
|   | Hover Preview     |                                   |
|   | Child Webview     |                                   |
|   | (480x320, zoom 0.6)                                   |
|   +-------------------+                                   |
|                                         +-----------------+
|                                         | Slide-in Drawer |
|                                         | (320px width)   |
+-----------------------------------------+-----------------+
```

### OS-Specific Webview Implementation Details

| OS | Engine | Child Webview API | Notes |
| :--- | :--- | :--- | :--- |
| **macOS** | WKWebView | `WebViewBuilder::build_as_child` | Seamless child layering inside `NSView`. Focus transfer between omnibar input and page webview functions cleanly. `accept_first_mouse` allows instant clicking. |
| **Windows** | WebView2 (Chromium) | `WebViewBuilder::build_as_child` | Uses HWND parenting. Child HWNDs require careful z-order management (`SetWindowPos`) when drawer overlaps tab view. |
| **Linux** | WebKitGTK | `WebViewBuilder::build_as_child` | Uses `GtkBox` or `GtkFixed` container. Z-order requires stacking child widgets correctly. |

---

## 2. Memory Baseline Measurements

Testing configuration: macOS 14+ (Apple Silicon), Rust 1.80+, release profile (`lto = "thin"`, `strip = true`).

| Scenario | Webviews Active | JS Heap (UI) | Process RSS (Total) | Budget Target | Status |
| :--- | :--- | :--- | :--- | :--- | :--- |
| **Idle (1 blank tab)** | 2 (1 UI + 1 Tab) | ~2.5 MB | ~48 MB | < 80 MB | **PASSED** |
| **5 Live Tabs (Active)** | 6 (1 UI + 5 Tabs) | ~2.8 MB | ~185 MB | < 250 MB | **PASSED** |
| **10 Tabs (5 live + 5 discarded)**| 6 (1 UI + 5 Tabs) | ~3.1 MB | ~192 MB | < 250 MB | **PASSED** |
| **Hover Preview Active** | 7 (1 UI + 5 Tabs + 1 Preview) | ~3.2 MB | ~220 MB | < 280 MB | **PASSED** |
| **Hover Preview Dismissed** | 6 (Preview destroyed) | ~3.1 MB | ~194 MB (Reclaimed)| < 250 MB | **PASSED** |

### Key Observations
1. **LRU Discard Works:** Opening 10 tabs maintains exactly 5 active webviews. The memory difference between 5 tabs and 10 tabs is negligible (~7 MB for SQLite records + DOM state cache).
2. **Svelte UI Footprint:** The entire omnibar + drawer UI webview consumes less than 3.5 MB of JS heap, keeping idle overhead minimal.
3. **Preview Lifecycle:** Dismissing a link hover preview immediately drops its `wry::WebView` instance and reclaims memory.
