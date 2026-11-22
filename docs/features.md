# Peek — Features & Gestures Guide

Peek is designed with a singular focus: **eliminate browser bloat and tab overload through effortless previewing and intelligent memory management.**

---

## 1. Signature Features & Gestures

### ✦ "Space to Peek" — Finder QuickLook for the Web
On macOS, pressing `Spacebar` in Finder previews images, documents, and videos instantly without launching an application. Peek brings this paradigm to the web:

- **How it works:** Hover your cursor over any hyperlink on a webpage and tap <kbd>Space</kbd> (or hold <kbd>Shift</kbd> or <kbd>⌘</kbd>).
- **Instant Preview:** A hardware-accelerated, floating preview card appears with smooth rounded corners, rendering the target webpage live.
- **Actions:**
  - Click anywhere on the preview card to promote it to a full tab.
  - Press <kbd>Esc</kbd> or simply move the mouse away to dismiss it.
- **Why it matters:** Read Wikipedia citations, preview search results, scan newsletter links, and inspect documentation without accumulating dozens of background tabs.

---

### ⇄ Two-Finger Omnibar Swipe — Seamless Tab Switcher
Switching between active tabs usually requires clicking narrow tabs in a crowded tab bar or memorizing complex modifier combinations.

- **How it works:** Place two fingers on your trackpad and swipe horizontally across the top bar.
- **Behavior:** Peek smoothly cycles to the next (swipe right) or previous (swipe left) tab with built-in momentum protection.
- **Zero Click Required:** Glide back and forth between active research pages effortlessly.

---

### ⇥ Trackpad Edge Swipes & Proximity Drawers
Peek maximizes screen real estate by keeping tabs and toolbars tucked away until you need them:

- **Left Edge Swipe / Hover:** Swiping inward from the left edge of your trackpad (or moving your cursor to the leftmost pixels) reveals the **Vertical Tabs Drawer**.
- **Right Edge Swipe / Hover:** Swiping inward from the right edge reveals the **Settings, Tools & Profiles Drawer**.
- **Auto-Dismiss:** Moving the cursor back into the active page smoothly slides the drawers closed.

---

## 2. Intelligent Memory Saver & Tab Hibernation

Modern browsers frequently consume gigabytes of system RAM, slowing down machines and draining laptop battery. Peek enforces a strict memory budget using an automated LRU (Least-Recently-Used) architecture:

```
[ Active Tab (Live) ] ──▶ [ Inactive Tab (Live) ] ──▶ [ Exceeds Limit / Inactive ]
                                                                │
                                                                ▼
                                                    [ ☾ Sleeping Tab (0 MB RAM) ]
                                                    - Process destroyed
                                                    - URL & scroll position preserved
```

### Memory Saver Capabilities
1. **Automated Eviction:** When more than 5 live tabs are open, inactive background tabs are automatically put to sleep, dropping their WebViews entirely.
2. **Scroll Offset Restoration:** When you switch back to a sleeping tab, Peek recreates the webview and automatically scrolls back to your exact reading position (`scroll_y`).
3. **Protection Shields:** Tabs playing media/audio and tabs with unsaved form input are strictly shielded from automatic hibernation.
4. **Memory Saver Dashboard:**
   - Open the Tabs Drawer (<kbd>⌘</kbd> <kbd>⇧</kbd> <kbd>[</kbd>) to see real-time statistics: e.g. `3 live · 4 sleeping (380 MB saved)`.
   - Click **"Hibernate Others"** to immediately put all background tabs to sleep and free system memory.
   - Click the **`☾` Sleep** button on any individual inactive tab to hibernate it manually.

---

## 3. Profiles & Complete Identity Isolation

Create separate profiles for Work, Personal, Client Projects, or Research:
- **Zero Cross-Contamination:** Each profile maintains its own isolated SQLite database, cookie jar, browsing history, and WebKit local storage directory.
- **Fast Profile Switching:** Click the profile chip on the start page or in Settings to switch profiles in a single click.

---

## 4. Built-in Brave Adblock & Privacy Shields

Peek embeds the high-performance Brave Adblock Rust engine:
- **Network-Level Request Blocking:** Evaluates network requests against compiled EasyList, EasyPrivacy, and uBlock filter lists before any network traffic is initiated.
- **Cosmetic CSS Filtering:** Injects compiled stylesheets that collapse empty ad placeholder frames, sponsored banners, and sticky popups.
- **Per-Site Shields:** Click the shield badge in the settings drawer to toggle blocking on or off for specific domains.
- **Zero Telemetry:** Peek contains no trackers, telemetry pings, background beacons, or third-party analytics.

---

## 5. Keyboard & Gesture Quick Reference

| Action | Shortcut / Gesture | Description |
| :--- | :--- | :--- |
| **Focus Omnibar** | <kbd>⌘</kbd> <kbd>L</kbd> | Opens address and search bar |
| **New Tab** | <kbd>⌘</kbd> <kbd>T</kbd> | Opens a clean, blank new tab |
| **Close Tab** | <kbd>⌘</kbd> <kbd>W</kbd> | Closes the active tab |
| **Reload Page** | <kbd>⌘</kbd> <kbd>R</kbd> | Reloads current page |
| **History Navigation** | <kbd>Alt</kbd> + <kbd>←</kbd> / <kbd>→</kbd> | Go back / forward in browsing history |
| **Link QuickLook** | <kbd>Space</kbd> or <kbd>⇧</kbd> + Hover | Live preview of hovered hyperlink |
| **Cycle Tabs** | Two-Finger Horizontal Swipe on Bar | Glide forward / backward between tabs |
| **Toggle Tabs Drawer** | <kbd>⌘</kbd> <kbd>⇧</kbd> <kbd>[</kbd> (or Left Edge Swipe) | Opens vertical tabs list & memory saver |
| **Toggle Settings Drawer** | <kbd>⌘</kbd> <kbd>⇧</kbd> <kbd>]</kbd> (or Right Edge Swipe) | Opens settings, profiles, and adblock tools |
| **Dismiss / Close** | <kbd>Esc</kbd> | Closes preview popup or slide-in drawer |
| **Hibernate Background Tabs** | Click `Hibernate Others` in Tabs | Immediately frees RAM from inactive tabs |
