# Peek — Implementation Plan

*A tiny, Rust-built browser with hover previews, a built-in ad blocker, Chrome-extension import and a one-bar UI. Written to be handed to AI coding agents, phase by phase.*

## 1. Product summary

Peek is a lightweight desktop browser (Windows, macOS, Linux) from one Rust codebase.

- **Low memory:** uses the operating system's own web engine instead of bundling one, keeps few tabs alive, and discards idle ones.
- **Hover preview:** hover a link (for example a search result) and a small live preview of the real page appears. Click or press Enter to open it as a tab; move away to dismiss it.
- **Built-in ad blocker:** native, not an extension.
- **Chrome extension import:** loads Manifest V3 extensions through a Rust extension host, with a compatibility report.
- **Profiles:** separate cookies, history and extensions per profile.
- **Minimal UI:** one slim bar. Everything else lives in an edge drawer revealed by dragging the cursor to the left or right edge, whichever the user chooses.
- **Customizable in-app:** themes, drawer side, hover delay, search engine, blocklists, shortcuts.

## 2. Locked decisions

### 2.1 Engine: system WebViews through `wry` + `tao`

| OS | Engine used |
| --- | --- |
| Windows | WebView2 (Chromium-based, shared runtime, installed on Windows 10/11) |
| macOS | WKWebView (WebKit) |
| Linux | WebKitGTK |

Why: nothing engine-sized is bundled, so installer size and idle memory are as low as a browser can get. Bundling Chromium (Electron or CEF) adds hundreds of MB of disk and a heavier process baseline, which defeats the main goal. There is no true "lightweight Chromium"; Chromium is only lighter when it is shared, as WebView2 is on Windows.

Use `tao` (windowing) and `wry` (webviews) directly rather than the full Tauri framework. This is a locked decision: Peek does not use the Tauri framework, because it needs none of Tauri's runtime, command system or plugins, and several child webviews in one window are easier to control through wry directly. It also keeps the binary and idle memory as small as possible. Agents must check current `wry` docs for child-webview support on all three OSes (`WebViewBuilder::build_as_child` or its current equivalent) before Phase 1.

### 2.2 Extensions: our own Rust extension host

System WebViews cannot load Chrome extensions, so Peek implements the parts of Manifest V3 that most extensions use. It will not run every extension, and the plan says so up front (section 4.6).

Fallback if compatibility proves too low: add a second engine backend (Chromium via CEF Rust bindings) behind the `Engine` trait (Phase 8). Do not build this first.

### 2.3 Language and UI stack

- **Rust everywhere** for core, tabs, storage, ad blocking, preview, extension host and updates.
- **Browser UI:** one small local webview using **Svelte + TypeScript** (no virtual DOM, tiny runtime), bundled as static assets and served through a custom `peek://` protocol. This is the only non-Rust code.
- Later optimization: replace the UI webview with a native Rust UI if its memory cost matters.

### 2.4 Name

Peek. Before launch, check trademark and domain availability; other tools and extensions already use similar names (Peekr, Peeko, Peekly).

## 3. Architecture

One Rust process owns one event loop and many webviews inside one window:

- **UI webview:** omnibar and edge drawer, loads only local `peek://` assets.
- **Tab webviews:** only the active tab, plus a few recent ones. Others are *discarded* (webview destroyed; URL, title, favicon, scroll position and a thumbnail kept).
- **Preview webview:** a single, reused child webview, created on demand and destroyed after dismissal.
- **Extension host webviews:** hidden, one per running extension background page, started lazily and suspended after idle.

### 3.1 Workspace layout

```
peek/
  Cargo.toml                (workspace)
  crates/
    peek-app/        main binary, event loop, window layout
    peek-engine/     Engine trait + wry backend (per-OS glue under cfg)
    peek-core/       tabs, sessions, navigation, discard logic
    peek-adblock/    adblock-rust wrapper, list updater, per-OS blocking glue
    peek-preview/    hover protocol, preview lifecycle
    peek-ext/        MV3 manifest, CRX importer, chrome.* shim, host
    peek-store/      SQLite (rusqlite) + settings.toml, profile data dirs
    peek-ipc/        typed IPC messages (serde), security checks
  ui/                Svelte app (omnibar + drawer)
  scripts/           injected JS (hover, edge-detect, cosmetic filters)
  tests/
```

### 3.2 IPC

Typed JSON messages (serde) over each webview's IPC channel. The UI webview may send commands (navigate, switch tab, change setting). Page webviews get only small injected scripts that can send: hover events, edge-proximity events, and extension messages. Every injected script carries a per-session nonce; Rust rejects messages without it. Page content never gets direct access to Rust commands.

## 4. Modules

### 4.1 Tabs and memory

- Keep at most **N live tabs** (default 5); least recently used tabs are discarded. Discard also after **T idle minutes** (default 10). Both are settings.
- A discarded tab shows in the drawer with its title and favicon, and reloads on selection, restoring scroll position.
- Never discard a tab that is playing audio, in a call, or has unsaved form input (detect with an injected check).
- **Acceptance:** with 10 tabs open and 5 live, the Rust process stays under a documented budget (measure and set the budget in Phase 0). A benchmark script records memory per OS in CI notes.

### 4.2 Omnibar and navigation

- URL or search in one field. Autocomplete from history, open tabs and bookmarks (SQLite FTS).
- Typing a tab title switches to that tab (the bar doubles as the tab switcher).
- Back/forward through mouse buttons, `Alt+Left/Right`, and optional swipe gestures; no visible buttons.
- Search engine configurable; default to a privacy-respecting one.
- HTTPS upgrade on by default; a small lock indicator inside the bar only for insecure pages.

### 4.3 Ad blocker

Use Brave's `adblock-rust` crate (network blocking, cosmetic filtering, uBlock Origin syntax). Serialize the compiled engine to disk so startup is fast.

Per-OS blocking glue (agents must verify each API against current docs):

- **macOS and Linux:** convert rules to the WebKit content-blocking JSON format (the crate has a conversion feature) and install them as a content rule list / user content filter.
- **Windows:** use WebView2's web-resource-request interception (through the `webview2-com` crate) and answer blocked requests from the `adblock-rust` engine.
- **Cosmetic filtering on all OSes:** inject CSS and scripts per page from the engine's per-URL cosmetic resources.
- Lists: EasyList, EasyPrivacy and uBlock filters by default; user-added lists; auto-update daily; per-site allowlist toggle in the drawer.
- **Acceptance:** blocks a known ad test page on all three OSes, and blocked-request count shows in the drawer.

### 4.4 Hover preview (signature feature)

**Why not an iframe:** many sites forbid framing (`X-Frame-Options`, CSP `frame-ancestors`). Use a separate native child webview.

Flow:

1. Injected script in every page watches `mouseover` on `<a href>` (and on search-result links). It reports `{url, rect}` to Rust after **hover intent** (default 450 ms, adjustable; cancelled if the cursor leaves).
2. Rust creates or reuses the single preview webview, sized by a setting (default about 480×320), positioned near the link and kept inside the window.
3. The preview renders the page at a reduced scale so it looks like a miniature of the real page.
4. Preview rules: media muted and autoplay blocked, scripts throttled after load, no permission prompts, no downloads; uses an **ephemeral** session by default (a setting can share the profile's cookies).
5. **Click** or **Enter** on the preview promotes it to a real tab. Moving the cursor away or pressing `Esc` destroys it.
6. Hold a modifier key (default `Shift`) to force a preview instantly; a setting can limit previews to modifier-only.

**Acceptance:** previews open within about 1 second on a normal connection, memory returns to baseline after dismissal, and only one preview exists at a time.

### 4.5 Profiles

- Each profile has its own data directory, history, bookmarks, cookies and extension set.
- Windows: separate WebView2 user-data folder per profile. macOS: a separate persistent website data store per profile (check the minimum OS version for this). Linux: separate WebKitGTK data and cache directories.
- Switch profiles from the drawer; each profile can open in its own window.

### 4.6 Chrome extension host

**Import sources:**

- An unpacked folder.
- A `.crx` file: parse the CRX3 header, unzip, validate.
- A Chrome Web Store ID: fetch the package and install it. Check the Web Store's terms of service before enabling this path for users; keep it behind a clearly labeled setting and consider offering only folder and `.crx` import at launch.

**Pipeline:** parse `manifest.json` (Manifest V3) → show requested permissions and a **compatibility report** (which APIs work, which do not) → install into the profile's extension directory.

**Content scripts:** match-pattern matcher in Rust, then inject through the webview's initialization-script mechanism at `document_start` or `document_end`. System webviews have no separate "isolated world", so each content script is wrapped in a closure and talks to Rust through a bridge object; document this limitation.

**Background logic:** run each extension's service worker script as a module in a hidden extension-host webview, with a `chrome.*` shim. This is not a real service-worker lifecycle: suspend it when idle and wake it on events.

**API support tiers:**

- **Tier 1 (Phase 5):** `storage` (local, sync as local, session), `runtime` (messaging, `getURL`, `getManifest`, `id`), `i18n`, `action` (popup, badge), `alarms`, `scripting` (`executeScript`, `insertCSS`), basic `tabs`, `contextMenus`, `commands`, `declarativeNetRequest` static rulesets (translated to the ad-block engine), basic `webNavigation`.
- **Tier 2 (Phase 6):** `cookies`, `history`, `bookmarks`, `downloads`, `notifications`, dynamic `declarativeNetRequest` rules.
- **Not supported:** blocking `webRequest`, DevTools panels, `nativeMessaging`, `debugger`, `proxy`, `tabCapture`. The compatibility report lists these when an extension needs them.

**Test corpus:** collect 20 popular extensions (password managers, dark-mode, translators, productivity) and track pass/fail per extension in a table. Because the ad blocker is built in, the most common extension need is already covered.

**Acceptance:** at least a documented list of extensions loads, runs and persists settings across restarts; unsupported ones fail with a clear message, never a crash.

### 4.7 Customization and settings

- Settings stored in `settings.toml` per profile; changes apply live.
- Themes through CSS variables: light, dark, system, plus accent color and font choice; theme files importable as JSON.
- Options: drawer side (left or right), edge hot-zone width, hover delay and size, tab limits, search engine, ad-block lists, per-site rules, keyboard shortcuts, bar position (top or bottom) and auto-hide.

### 4.8 Security and updates

- Browsers must ship security fixes fast. Because Peek uses the OS engine, engine patches arrive with OS or WebView2 updates; document minimum supported versions.
- Auto-update Peek itself with a small Rust updater crate such as self\_update (signed releases, delta updates where possible), and build installers with cargo-packager, since there is no framework bundler.
- Sandbox rules: UI webview loads only `peek://` assets; no remote content in privileged webviews; strict CSP; all IPC validated.
- Enforce HTTPS-first, block mixed content, and handle certificate errors with an interstitial.

## 5. UI specification (brief)

**Principle:** nothing on screen except the page and one slim bar.

- **The bar:** a single thin (about 36 px) rounded bar at the top, holding only the address/search field. It can auto-hide on scroll. No tab strip, no buttons, no toolbar icons.
- **The drawer:** dragging the cursor to the configured screen edge (left or right) slides in a panel over the page with sections:
  1. **Tabs** (vertical list with favicons, close, drag to reorder)
  2. **Profiles**
  3. **Extensions**
  4. **Ad blocker** (counter, site toggle)
  5. **Settings and themes**
- **Edge detection:** an injected script in each page reports pointer proximity to the edge, plus a thin always-on-top edge strip owned by the UI webview as a fallback. The drawer opens after a short dwell (about 150 ms) to avoid accidental triggers and closes when the cursor leaves or on `Esc`.
- **Keyboard first:** `Ctrl/Cmd+L` focuses the bar, `Ctrl/Cmd+K` opens a command palette, `Ctrl+Tab` cycles tabs.
- **Style:** quiet, high contrast, small type, subtle motion (under 150 ms), respects reduced-motion settings.

## 6. Phases and acceptance criteria

| Phase | Goal | Done when |
| --- | --- | --- |
| 0 Spike | Prove the layout on all 3 OSes: one window with a UI webview, a tab child webview, a preview child webview, edge hot-zone, z-order correct. Record baseline memory. | Short demo plus a written findings note per OS |
| 1 Shell | Tabs, omnibar, navigation, history, discard logic, session restore | Daily-usable basic browser |
| 2 Ad blocker | Section 4.3 | Ad test page blocked on all OSes |
| 3 Hover preview | Section 4.4 | Acceptance in 4.4 met |
| 4 Profiles and settings | Sections 4.5 and 4.7, themes, drawer side | Switch profiles; settings apply live |
| 5 Extensions Tier 1 | Importer, manifest, content scripts, background host, Tier 1 APIs, compatibility report | Test-corpus pass table started |
| 6 Extensions Tier 2 | Tier 2 APIs, store import (if allowed), polish | Pass rate documented |
| 7 Release | Installers (cargo-packager), signing, auto-update (self\_update), benchmarks, docs | Signed builds for all 3 OSes |
| 8 Optional | CEF backend behind the `Engine` trait | Only if extension compatibility needs it |

Phases 2, 3 and 4 can run in parallel after Phase 1. Phase 5 needs Phase 1 and the ad-block engine from Phase 2.

## 7. Risks

- **Engine differences:** WebKit and Chromium behave differently; test every feature on all three OSes in CI and manually.
- **Child webview z-order and focus** differ per OS: this is why Phase 0 exists. Do not start Phase 1 until it passes.
- **Extension compatibility:** partial by design; mitigate with the compatibility report and the test corpus.
- **Web Store terms:** keep store-ID import optional.
- **Security maintenance:** rely on the OS engine updates, ship Peek updates quickly, and document minimum versions.
- **Linux fragmentation:** WebKitGTK versions vary; define a minimum version.

## 8. Instructions for AI agents

**Rules for every agent:**

1. Work only inside your assigned crate or folder; do not edit other agents' areas.
2. Verify every external API against its current documentation before using it; do not guess signatures.
3. Keep dependencies minimal; each new crate needs a one-line justification in the PR.
4. Add tests and a short README section for what you built.
5. Never load remote content into the UI webview.
6. End every task by listing what is done, what is untested, and any OS-specific problems.

**Prompt template (use for each phase):**

```
Project: Peek, a lightweight Rust browser (tao + wry, system WebViews). Read PLAN.md.
Prior State: <what is already built, which crates exist, what works on which OS>
Task: <one phase or sub-task from section 6>
Scope: only touch <crate/folder>
Acceptance: <copy the acceptance criteria from the plan>
Output: code, tests, a short note on OS-specific issues.
```

**First prompt (Phase 0 spike):**

```
Create a Cargo workspace with peek-app and peek-engine. Using tao and wry, open one window containing (1) a UI child webview as a 36px top bar loading a local page, (2) a tab child webview filling the rest and loading https://example.com, (3) a preview child webview that appears at the cursor over a hovered link and is destroyed on mouse leave, (4) an edge hot-zone that slides a drawer webview in from the right when the cursor reaches the right edge. Confirm z-order and focus on Windows, macOS and Linux. Record idle memory for 1, 5 and 10 tabs. Write findings to docs/phase0.md. Verify wry child-webview APIs in current docs first.
```

**Second prompt (Phase 1):** build `peek-core` tab manager with LRU discard, session restore in SQLite, and the omnibar with history autocomplete, using the Phase 0 layout as Prior State.

## 9. Phase 1: Shell, tabs and omnibar

Phase 1 turns the Phase 0 spike into a daily-usable basic browser. Start it only after Phase 0 passes on all three OSes (z-order and focus confirmed, baseline memory recorded in `docs/phase0.md`); paste that file's findings into every prompt below as Prior State.

**Done when:** you can browse with several tabs, search or type URLs in the one bar, close and reopen the app with tabs restored, and idle tabs are discarded without losing their place.

### 9.1 Work split (three agents in parallel)

| Agent | Territory | Prompts |
| --- | --- | --- |
| Windsurf | `peek-store`, `peek-ipc` | 1.1, then 1.2 |
| Claude Code | `peek-engine`, `peek-core`, `peek-app` | 1.3, 1.5 |
| Cursor | `ui/` (Svelte omnibar) | 1.4 |

Order: 1.1 first (the message types and schema everyone depends on). Then 1.3 and 1.4 in parallel. Then 1.2 and 1.5, which need both.

### 9.2 Prompts

**1.1 Storage and IPC types (`peek-store`, `peek-ipc`)**

```
Project: Peek (tao + wry, system WebViews). Read PLAN.md. Prior State: <Phase 0 findings>.
Task: In peek-store, add rusqlite with migrations and tables: history (url, title, visit_count, last_visit), tabs (id, url, title, favicon, scroll_y, position, last_active), sessions (window layout, active tab), bookmarks, settings. Add an FTS5 index over history title and url. In peek-ipc, define serde enums for UI-to-Rust commands (Navigate, NewTab, CloseTab, SwitchTab, Back, Forward, Reload, Search) and Rust-to-UI events (TabsChanged, ActiveTabChanged, Loading, TitleChanged, SuggestionsReady). Every message carries a session nonce.
Scope: only peek-store and peek-ipc.
Acceptance: unit tests for every query and for message round-trips; no unsafe code; schema documented in docs/schema.md.
```

**1.3 Tab manager and navigation (`peek-engine`, `peek-core`)**

```
Prior State: peek-store and peek-ipc exist (1.1). Phase 0 layout works.
Task: Implement TabManager in peek-core: create, activate, close, reorder tabs, each backed by a wry child webview through the Engine trait. Hook wry handlers for navigation, page load start/finish, title changes and favicon, and push updates as IPC events. Implement LRU discard: keep at most N live tabs (default 5) and discard after T idle minutes (default 10); a discarded tab stores URL, title, favicon and scroll position and its webview is destroyed. Never discard a tab with audio or unsaved form input (detect with an injected check). Reload a discarded tab on activation and restore scroll.
Scope: peek-engine, peek-core, peek-app wiring only.
Acceptance: with 10 tabs and N=5, only 5 webviews exist; discard and restore work on all three OSes; memory numbers recorded in docs/phase1.md.
```

**1.4 Omnibar UI (`ui/`)**

```
Prior State: peek-ipc message types are fixed (1.1).
Task: Build the 36px Svelte omnibar as the only visible chrome. One input: URL or search (treat input as a URL if it has a scheme or a valid host, otherwise search with the configured engine). Show suggestions from history, open tabs and bookmarks (from SuggestionsReady events). Typing an open tab's title switches to it. Ctrl/Cmd+L focuses the bar; Esc returns focus to the page. No buttons, no tab strip. Show a small lock only on insecure pages. Keep the bundle small and avoid heavy dependencies.
Scope: only ui/.
Acceptance: works against a mock IPC layer; keyboard-only use is complete; bundle size reported in the PR.
```

**1.2 Session restore and history recording (`peek-store`, `peek-core`)**

```
Prior State: 1.1, 1.3 and 1.4 are done.
Task: Record visits into history with FTS search. On quit, save the session; on launch, restore all tabs as discarded (only the active tab loads). Add a setting to start with a blank page instead. Add HTTPS-first upgrade with a fallback prompt when the HTTPS load fails.
Scope: peek-store and peek-core.
Acceptance: close and reopen the app and the same tabs and active tab return; killing the process mid-session loses at most the last few seconds.
```

**1.5 Window behavior and shortcuts (`peek-app`)**

```
Prior State: 1.1 to 1.4 are done.
Task: Handle window resize and DPI changes for all child webviews. Add shortcuts: Ctrl/Cmd+T, W, Tab, Shift+Tab, number keys, R, and Alt+Left/Right and mouse buttons for back/forward. Handle link clicks that open new windows (target=_blank, window.open) as new tabs. Add a benchmark script that opens 10 tabs and records memory for each OS.
Scope: peek-app, scripts/.
Acceptance: all shortcuts work on the three OSes; resize and DPI changes keep layout correct; benchmark output is saved to docs/phase1.md.
```

### 9.3 Out of scope for Phase 1

Ad blocking, hover preview, profiles, themes, the edge drawer and extensions. Keep a placeholder entry point for the drawer, nothing more.
