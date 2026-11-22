<script lang="ts">
  import { onMount } from 'svelte';
  import {
    onEvent,
    requestState,
    send,
    type EdgeSide,
    type PeekEvent,
    type TabInfo,
  } from './ipc';

  type Section = 'tabs' | 'general' | 'appearance' | 'blocking' | 'preview' | 'profiles' | 'extensions' | 'shortcuts' | 'about';

  let currentSide = $state<EdgeSide>('left');
  let section = $state<Section>('tabs');
  let filterText = $state('');

  let tabs = $state<TabInfo[]>([]);
  let activeTabId = $state<number | null>(null);
  let blocked = $state(0);
  let allowedOnSite = $state(false);
  let host = $state<string | null>(null);
  let cookiesCount = $state(0);

  // Settings
  let maxLive = $state(5);
  let idleMinutes = $state(10);
  let hoverDelay = $state(80);
  let configuredDrawerSide = $state<EdgeSide>('right');
  let edgeZone = $state(4);
  let startBlank = $state(false);
  let searchEngine = $state('Google');

  // Profiles
  let profiles = $state<string[]>(['default']);
  let activeProfile = $state('default');
  let newProfileName = $state('');

  // Appearance & Theme
  let themeMode = $state<'system' | 'dark' | 'light'>('system');
  let accentColor = $state<'blue' | 'purple' | 'emerald' | 'amber' | 'rose'>('blue');

  // Drag and close timer
  let dragId = $state<number | null>(null);
  let closeTimer: ReturnType<typeof setTimeout> | undefined;

  let filteredTabs = $derived(
    filterText.trim()
      ? tabs.filter((t) =>
          (t.title || t.url).toLowerCase().includes(filterText.toLowerCase())
        )
      : tabs
  );

  function close(): void {
    send({ type: 'setDrawer', open: false });
  }

  function scheduleClose(): void {
    cancelClose();
    closeTimer = setTimeout(close, 250);
  }

  function cancelClose(): void {
    if (closeTimer) {
      clearTimeout(closeTimer);
      closeTimer = undefined;
    }
  }

  function switchTo(id: number): void {
    send({ type: 'switchTab', id });
    close();
  }

  function setting(key: string, value: unknown): void {
    send({ type: 'setSetting', key, value });
  }

  function applyTheme(mode: 'system' | 'dark' | 'light') {
    themeMode = mode;
    localStorage.setItem('peek_theme', mode);
    if (mode === 'system') {
      delete document.documentElement.dataset.theme;
    } else {
      document.documentElement.dataset.theme = mode;
    }
  }

  function applyAccent(accent: 'blue' | 'purple' | 'emerald' | 'amber' | 'rose') {
    accentColor = accent;
    localStorage.setItem('peek_accent', accent);
    document.documentElement.dataset.accent = accent;
  }

  function handleCreateProfile() {
    const trimmed = newProfileName.trim();
    if (!trimmed) return;
    send({ type: 'createProfile', name: trimmed });
    newProfileName = '';
  }

  function handleSwitchProfile(name: string) {
    if (name === activeProfile) return;
    send({ type: 'switchProfile', name });
  }

  function handleDeleteProfile(name: string) {
    if (name === 'default' || name === activeProfile) return;
    if (confirm(`Delete profile "${name}" and all its data?`)) {
      send({ type: 'deleteProfile', name });
    }
  }

  function handleEvent(event: PeekEvent): void {
    switch (event.type) {
      case 'tabsChanged':
        tabs = event.tabs;
        activeTabId = event.active;
        break;
      case 'adblockCount':
        blocked = event.blocked;
        allowedOnSite = event.allowedOnSite;
        host = event.host;
        break;
      case 'settings':
        maxLive = event.maxLive;
        idleMinutes = event.idleDiscardMinutes;
        hoverDelay = event.hoverDelayMs;
        configuredDrawerSide = event.drawerSide;
        edgeZone = event.edgeZone;
        startBlank = event.startBlank;
        searchEngine = event.searchEngine;
        break;
      case 'drawerState':
        if (!event.open) {
          send({ type: 'setDrawer', open: false });
        } else {
          if (event.side) {
            currentSide = event.side;
            if (event.side === 'left') {
              section = 'tabs';
            } else if (section === 'tabs') {
              section = 'general';
            }
          }
          if (event.section) {
            section = event.section as Section;
          }
        }
        break;
      case 'cookiesCount':
        cookiesCount = event.count;
        break;
      case 'profilesList':
        profiles = event.profiles;
        activeProfile = event.active;
        break;
      default:
        break;
    }
  }

  function onDragStart(event: DragEvent, id: number): void {
    dragId = id;
    event.dataTransfer?.setData('text/plain', String(id));
  }

  function onDrop(event: DragEvent, index: number): void {
    event.preventDefault();
    if (dragId != null) {
      send({ type: 'reorderTab', id: dragId, toIndex: index });
      dragId = null;
    }
  }

  onMount(() => {
    // Load local theme preference
    const savedTheme = localStorage.getItem('peek_theme') as 'system' | 'dark' | 'light' | null;
    if (savedTheme) applyTheme(savedTheme);
    const savedAccent = localStorage.getItem('peek_accent') as 'blue' | 'purple' | 'emerald' | 'amber' | 'rose' | null;
    if (savedAccent) applyAccent(savedAccent);

    onEvent(handleEvent);
    requestState();
    const onKey = (event: KeyboardEvent) => {
      if (event.key === 'Escape') close();
    };
    window.addEventListener('keydown', onKey, true);
    return () => window.removeEventListener('keydown', onKey, true);
  });
</script>

<aside
  class="drawer"
  class:left={currentSide === 'left'}
  class:right={currentSide === 'right'}
  onmouseenter={cancelClose}
  onmouseleave={scheduleClose}
  aria-label="Peek drawer"
>
  {#if currentSide === 'left'}
    <!-- Left Drawer: Dedicated Vertical Tabs -->
    <header class="tabs-header">
      <div class="tabs-brand">
        <img src="/logo.png" alt="Peek" class="drawer-logo" />
        <span class="drawer-title">Tabs</span>
        <span class="tab-count-badge">{tabs.length}</span>
      </div>
      <button class="icon-btn" onclick={close} title="Close drawer (Esc)">✕</button>
    </header>

    <div class="filter-wrap">
      <input
        type="search"
        bind:value={filterText}
        placeholder="Search tabs..."
        class="filter-input"
      />
    </div>

    <div class="body tabs-body">
      <ul class="tabs-list">
        {#each filteredTabs as tab, index (tab.id)}
          <li
            class:active={tab.active}
            class:discarded={tab.discarded}
            draggable="true"
            ondragstart={(event) => onDragStart(event, tab.id)}
            ondragover={(event) => event.preventDefault()}
            ondrop={(event) => onDrop(event, index)}
          >
            <button class="tab-row" onclick={() => switchTo(tab.id)}>
              {#if tab.favicon}
                <img class="favicon" src={tab.favicon} alt="" />
              {:else}
                <span class="favicon dot"></span>
              {/if}
              <span class="title">{tab.title || tab.url}</span>
              {#if tab.audible}
                <span class="pill audio" title="Playing audio">♪</span>
              {/if}
              {#if tab.discarded}
                <span class="pill sleep" title="Sleeping (memory saved)">zzz</span>
              {/if}
            </button>
            <button
              class="tab-close"
              aria-label="Close tab"
              title="Close tab (Cmd+W)"
              onclick={(e) => {
                e.stopPropagation();
                send({ type: 'closeTab', id: tab.id });
              }}
            >
              ×
            </button>
          </li>
        {:else}
          <li class="empty">No matching tabs</li>
        {/each}
      </ul>
    </div>

    <footer class="tabs-footer">
      <button class="new-tab-btn" onclick={() => { send({ type: 'newTab' }); close(); }}>
        <span class="plus">+</span> New Tab <kbd>⌘ T</kbd>
      </button>
    </footer>

  {:else}
    <!-- Right Drawer: Settings, Profiles, Adblock, Hover Preview, Extensions -->
    <header class="settings-header">
      <div class="brand-row">
        <div class="tabs-brand">
          <img src="/logo.png" alt="Peek" class="drawer-logo" />
          <span class="drawer-title">Settings & Tools</span>
        </div>
        <button class="icon-btn" onclick={close} title="Close drawer (Esc)">✕</button>
      </div>

      <nav class="settings-nav">
        <button class:on={section === 'general'} onclick={() => (section = 'general')}>General</button>
        <button class:on={section === 'appearance'} onclick={() => (section = 'appearance')}>Themes</button>
        <button class:on={section === 'blocking'} onclick={() => (section = 'blocking')}>Adblock</button>
        <button class:on={section === 'preview'} onclick={() => (section = 'preview')}>Preview</button>
        <button class:on={section === 'profiles'} onclick={() => (section = 'profiles')}>Profiles</button>
        <button class:on={section === 'extensions'} onclick={() => (section = 'extensions')}>Ext</button>
        <button class:on={section === 'shortcuts'} onclick={() => (section = 'shortcuts')}>Keys</button>
        <button class:on={section === 'about'} onclick={() => (section = 'about')}>About</button>
      </nav>
    </header>

    <div class="body settings-body">
      {#if section === 'general'}
        <div class="section-title">General Preferences</div>
        <div class="field">
          <label for="search-eng">Default Search Engine</label>
          <select
            id="search-eng"
            value={searchEngine}
            onchange={(e) => setting('searchEngine', (e.currentTarget as HTMLSelectElement).value)}
          >
            <option value="Google">Google</option>
            <option value="DuckDuckGo">DuckDuckGo</option>
            <option value="Brave">Brave Search</option>
            <option value="Bing">Bing</option>
            <option value="Ecosia">Ecosia</option>
          </select>
        </div>

        <div class="field">
          <label for="side">Default Drawer Side</label>
          <select
            id="side"
            value={configuredDrawerSide}
            onchange={(e) => setting('drawerSide', (e.currentTarget as HTMLSelectElement).value)}
          >
            <option value="right">Right</option>
            <option value="left">Left</option>
          </select>
        </div>

        <div class="field">
          <label for="edge">Edge Hot-Zone (px)</label>
          <input
            id="edge"
            type="number"
            min="1"
            max="16"
            value={edgeZone}
            onchange={(e) => setting('edgeZone', Number((e.currentTarget as HTMLInputElement).value))}
          />
        </div>

        <label class="toggle-row">
          <input
            type="checkbox"
            checked={startBlank}
            onchange={(e) => setting('startBlank', (e.currentTarget as HTMLInputElement).checked)}
          />
          <span>Always open new tabs with centered start page</span>
        </label>

        <div class="section-title" style="margin-top: 20px;">Resource Management</div>
        <div class="field">
          <label for="max-live">Max Live Tabs (LRU)</label>
          <input
            id="max-live"
            type="number"
            min="1"
            max="20"
            value={maxLive}
            onchange={(e) => setting('maxLive', Number((e.currentTarget as HTMLInputElement).value))}
          />
        </div>
        <div class="field">
          <label for="idle">Sleep Idle Tabs (Minutes)</label>
          <input
            id="idle"
            type="number"
            min="1"
            max="120"
            value={idleMinutes}
            onchange={(e) => setting('idleDiscardMinutes', Number((e.currentTarget as HTMLInputElement).value))}
          />
        </div>

      {:else if section === 'appearance'}
        <div class="section-title">Color Theme</div>
        <div class="theme-options">
          <button
            class="choice-btn"
            class:selected={themeMode === 'system'}
            onclick={() => applyTheme('system')}
          >
            💻 System
          </button>
          <button
            class="choice-btn"
            class:selected={themeMode === 'dark'}
            onclick={() => applyTheme('dark')}
          >
            🌙 Dark
          </button>
          <button
            class="choice-btn"
            class:selected={themeMode === 'light'}
            onclick={() => applyTheme('light')}
          >
            ☀️ Light
          </button>
        </div>

        <div class="section-title" style="margin-top: 20px;">Accent Color</div>
        <div class="accent-palette">
          <button
            class="accent-dot"
            style="background: #3b82f6;"
            class:selected={accentColor === 'blue'}
            onclick={() => applyAccent('blue')}
            title="Iris Blue"
          ></button>
          <button
            class="accent-dot"
            style="background: #8b5cf6;"
            class:selected={accentColor === 'purple'}
            onclick={() => applyAccent('purple')}
            title="Electric Purple"
          ></button>
          <button
            class="accent-dot"
            style="background: #10b981;"
            class:selected={accentColor === 'emerald'}
            onclick={() => applyAccent('emerald')}
            title="Emerald Green"
          ></button>
          <button
            class="accent-dot"
            style="background: #f59e0b;"
            class:selected={accentColor === 'amber'}
            onclick={() => applyAccent('amber')}
            title="Amber Warm"
          ></button>
          <button
            class="accent-dot"
            style="background: #f43f5e;"
            class:selected={accentColor === 'rose'}
            onclick={() => applyAccent('rose')}
            title="Rose Crimson"
          ></button>
        </div>

        <div class="card" style="margin-top: 20px;">
          <div class="card-title">Theme Tokens</div>
          <p class="note">UI typography and token overrides are dynamically loaded via CSS variables.</p>
        </div>

      {:else if section === 'blocking'}
        <div class="card stat-card">
          <div class="stat-big">{blocked}</div>
          <div class="stat-label">Requests blocked in this tab</div>
        </div>

        {#if host}
          <div class="card" style="margin-top: 10px;">
            <label class="toggle-row">
              <input
                type="checkbox"
                checked={!allowedOnSite}
                onchange={() => send({ type: 'setSiteAdblock', host: host!, enabled: allowedOnSite })}
              />
              <span>Block trackers and ads on <strong>{host}</strong></span>
            </label>
          </div>
        {/if}

        <div class="card" style="margin-top: 10px;">
          <div class="card-title">Filter Engine</div>
          <p class="note">Native Brave adblock engine with uBlock Origin syntax, EasyList, EasyPrivacy, cosmetic rules and XHR blocking.</p>
        </div>

      {:else if section === 'preview'}
        <div class="section-title">Hover Preview Configuration</div>
        <p class="note" style="margin-bottom: 12px;">Hold modifier key and hover any link to peek at the page live without leaving your tab.</p>

        <div class="field">
          <label for="prev-key">Modifier Key</label>
          <select id="prev-key">
            <option value="cmd">⌘ Command (macOS)</option>
            <option value="ctrl">Ctrl</option>
            <option value="alt">⌥ Option / Alt</option>
            <option value="shift">⇧ Shift</option>
          </select>
        </div>

        <div class="field">
          <label for="prev-delay">Hover Debounce (ms)</label>
          <input
            id="prev-delay"
            type="number"
            min="20"
            max="400"
            step="10"
            value={hoverDelay}
            onchange={(e) => setting('hoverDelayMs', Number((e.currentTarget as HTMLInputElement).value))}
          />
        </div>

        <div class="card" style="margin-top: 14px;">
          <div class="card-title">Interaction Keys</div>
          <p class="note">
            • <strong>Click</strong> or <strong>Enter</strong> promotes the preview to a real tab.<br />
            • <strong>Esc</strong> or moving cursor away dismisses it.<br />
            • Audio and autoplay are completely muted.
          </p>
        </div>

      {:else if section === 'profiles'}
        <div class="section-title">Profiles & Storage</div>
        <p class="note" style="margin-bottom: 12px;">
          Each profile stores isolated cookies, history, cache and site credentials.
        </p>

        <ul class="profile-list">
          {#each profiles as prof}
            <li class="profile-item" class:active-profile={prof === activeProfile}>
              <div class="profile-info">
                <span class="profile-avatar">{prof.slice(0, 1).toUpperCase()}</span>
                <span class="profile-name">{prof}</span>
                {#if prof === activeProfile}
                  <span class="badge active-tag">Active</span>
                {/if}
              </div>
              <div class="profile-actions">
                {#if prof !== activeProfile}
                  <button class="small-btn switch-btn" onclick={() => handleSwitchProfile(prof)}>
                    Switch
                  </button>
                  {#if prof !== 'default'}
                    <button class="small-btn del-btn" onclick={() => handleDeleteProfile(prof)}>
                      ✕
                    </button>
                  {/if}
                {/if}
              </div>
            </li>
          {/each}
        </ul>

        <div class="add-profile-wrap">
          <input
            type="text"
            bind:value={newProfileName}
            placeholder="New profile name..."
            class="new-profile-input"
            onkeydown={(e) => { if (e.key === 'Enter') handleCreateProfile(); }}
          />
          <button class="add-btn" onclick={handleCreateProfile}>+ Add</button>
        </div>

        <div class="card" style="margin-top: 16px;">
          <div class="field readonly">
            <span class="key">Stored Cookies</span>
            <span>{cookiesCount}</span>
          </div>
          <button class="danger-btn" onclick={() => send({ type: 'clearCookies' })}>
            Clear Cookies & Local Storage
          </button>
        </div>

      {:else if section === 'extensions'}
        <div class="section-title">Chrome Extension Support</div>
        <p class="note" style="margin-bottom: 12px;">
          Import Manifest V3 extensions via unpacked folder or <code>.crx</code> package.
        </p>
        <div class="card">
          <div class="card-title">Manifest V3 Compatibility</div>
          <p class="note">
            • Tier 1: <code>storage</code>, <code>runtime</code>, <code>tabs</code>, <code>scripting</code>, <code>action</code>, <code>alarms</code>.<br />
            • Tier 2: <code>cookies</code>, <code>history</code>, <code>bookmarks</code>.
          </p>
        </div>

      {:else if section === 'shortcuts'}
        <div class="section-title">Keyboard Shortcuts</div>
        <div class="shortcut-list">
          <div class="key-row"><kbd>⌘ L</kbd> <span>Focus search / address</span></div>
          <div class="key-row"><kbd>⌘ T</kbd> <span>Open new tab</span></div>
          <div class="key-row"><kbd>⌘ W</kbd> <span>Close active tab</span></div>
          <div class="key-row"><kbd>⌘ ⇧ T</kbd> <span>Reopen closed tab</span></div>
          <div class="key-row"><kbd>⌘ ⇧ [</kbd> <span>Open Tabs drawer</span></div>
          <div class="key-row"><kbd>⌘ ⇧ ]</kbd> <span>Open Settings drawer</span></div>
          <div class="key-row"><kbd>⌘ + Hover</kbd> <span>Live link preview</span></div>
          <div class="key-row"><kbd>Esc</kbd> <span>Close preview / drawer</span></div>
        </div>

      {:else if section === 'about'}
        <div class="hero-brand" style="margin: 20px 0;">
          <img src="/logo.png" alt="Peek" class="drawer-logo" style="width: 44px; height: 44px;" />
          <h2 style="margin: 8px 0 2px; font-size: 18px;">Peek Browser</h2>
          <span style="font-size: 12px; color: var(--muted);">v0.1.0 • Pure Rust + WebKit/WebView2</span>
        </div>
        <div class="card">
          <p class="note">
            Peek is built with zero framework bloat using direct <code>tao</code> and <code>wry</code> bindings, native system WebViews, Brave adblock, and an ephemeral hover preview architecture.
          </p>
        </div>
      {/if}
    </div>

    <footer class="settings-footer">
      <button class="quit-btn" onclick={() => send({ type: 'quit' })}>Quit Peek</button>
    </footer>
  {/if}
</aside>

<style>
  .drawer {
    display: flex;
    flex-direction: column;
    height: 100vh;
    background: var(--panel);
    box-shadow: var(--shadow);
    animation: drawerSlide 0.22s cubic-bezier(0.2, 0, 0, 1);
  }

  .drawer.left {
    border-inline-end: 1px solid var(--border);
  }

  .drawer.right {
    border-inline-start: 1px solid var(--border);
  }

  @keyframes drawerSlide {
    from {
      opacity: 0.85;
      transform: translateX(var(--slide-dir, 12px));
    }
    to {
      opacity: 1;
      transform: translateX(0);
    }
  }

  .drawer.left {
    --slide-dir: -12px;
  }
  .drawer.right {
    --slide-dir: 12px;
  }

  /* Headers */
  .tabs-header,
  .settings-header {
    padding: 12px 14px 10px;
    border-bottom: 1px solid var(--border);
    flex-shrink: 0;
  }

  .tabs-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
  }

  .brand-row {
    display: flex;
    align-items: center;
    justify-content: space-between;
    margin-bottom: 10px;
  }

  .tabs-brand {
    display: flex;
    align-items: center;
    gap: 8px;
  }

  .drawer-logo {
    width: 20px;
    height: 20px;
    border-radius: 5px;
    box-shadow: 0 1px 3px rgba(0, 0, 0, 0.4);
    display: block;
  }

  .drawer-title {
    font-size: 13.5px;
    font-weight: 650;
    letter-spacing: 0.01em;
  }

  .tab-count-badge {
    font-size: 10.5px;
    padding: 1px 6px;
    border-radius: 999px;
    background: var(--hover);
    color: var(--muted);
    font-weight: 600;
  }

  .icon-btn {
    color: var(--muted);
    font-size: 14px;
    padding: 4px;
    border-radius: 4px;
  }

  .icon-btn:hover {
    color: var(--text);
    background: var(--hover);
  }

  /* Filter */
  .filter-wrap {
    padding: 8px 12px;
    border-bottom: 1px solid var(--border);
  }

  .filter-input {
    width: 100%;
    padding: 5px 9px;
    border: 1px solid var(--border);
    border-radius: 6px;
    background: var(--surface);
    color: var(--text);
    font-size: 12px;
    outline: none;
  }

  .filter-input:focus {
    border-color: var(--accent);
  }

  /* Settings Navigation */
  .settings-nav {
    display: flex;
    gap: 3px;
    flex-wrap: wrap;
  }

  .settings-nav button {
    padding: 4px 8px;
    border-radius: 6px;
    color: var(--muted);
    font-size: 11px;
    font-weight: 500;
  }

  .settings-nav button:hover {
    background: var(--hover);
    color: var(--text);
  }

  .settings-nav button.on {
    background: var(--hover);
    color: var(--accent);
    font-weight: 600;
  }

  /* Body */
  .body {
    flex: 1;
    overflow-y: auto;
    padding: 12px;
  }

  /* Tabs List */
  .tabs-list {
    list-style: none;
    margin: 0;
    padding: 0;
    display: flex;
    flex-direction: column;
    gap: 2px;
  }

  .tabs-list li {
    display: flex;
    align-items: center;
    border-radius: 7px;
    position: relative;
    transition: background 0.12s ease;
  }

  .tabs-list li:hover {
    background: var(--hover);
  }

  .tabs-list li.active {
    background: var(--accent-soft);
  }

  .tabs-list li.active::before {
    content: '';
    position: absolute;
    left: 0;
    top: 4px;
    bottom: 4px;
    width: 2.5px;
    border-radius: 2px;
    background: var(--accent);
  }

  .tabs-list li.discarded {
    opacity: 0.65;
  }

  .tab-row {
    flex: 1;
    display: flex;
    align-items: center;
    gap: 8px;
    min-width: 0;
    height: 32px;
    padding: 0 8px;
    text-align: left;
    font-size: 12.5px;
  }

  .title {
    flex: 1;
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .favicon {
    flex: 0 0 15px;
    width: 15px;
    height: 15px;
    border-radius: 3px;
    object-fit: contain;
  }

  .dot {
    background: var(--border);
  }

  .pill {
    flex: 0 0 auto;
    padding: 1px 5px;
    border-radius: 999px;
    font-size: 9.5px;
    letter-spacing: 0.04em;
    font-weight: 600;
    text-transform: uppercase;
  }

  .pill.audio {
    background: color-mix(in srgb, var(--accent) 25%, transparent);
    color: var(--accent);
  }

  .pill.sleep {
    background: var(--hover);
    color: var(--muted);
  }

  .tab-close {
    flex: 0 0 22px;
    height: 22px;
    margin-right: 4px;
    border-radius: 5px;
    color: var(--muted);
    font-size: 14px;
    line-height: 1;
    opacity: 0;
    transition: opacity 0.12s ease;
  }

  .tabs-list li:hover .tab-close {
    opacity: 1;
  }

  .tab-close:hover {
    background: color-mix(in srgb, #ef4444 18%, transparent);
    color: #ef4444;
  }

  .empty {
    padding: 14px 8px;
    color: var(--muted);
    font-size: 12px;
    text-align: center;
  }

  /* Footers */
  .tabs-footer,
  .settings-footer {
    padding: 10px 12px;
    border-top: 1px solid var(--border);
    flex-shrink: 0;
  }

  .new-tab-btn {
    display: flex;
    align-items: center;
    justify-content: space-between;
    width: 100%;
    padding: 7px 10px;
    border: 1px dashed var(--border);
    border-radius: 8px;
    color: var(--muted);
    font-size: 12px;
  }

  .new-tab-btn:hover {
    color: var(--text);
    border-color: var(--accent);
  }

  .plus {
    font-size: 14px;
    font-weight: 700;
  }

  /* Settings Controls */
  .section-title {
    font-size: 12px;
    font-weight: 650;
    color: var(--text);
    margin-bottom: 8px;
    letter-spacing: 0.01em;
  }

  .card {
    padding: 12px 14px;
    border: 1px solid var(--border);
    border-radius: var(--radius);
    background: var(--surface);
  }

  .stat-card {
    text-align: center;
    padding: 16px;
  }

  .stat-big {
    font-size: 34px;
    font-weight: 700;
    letter-spacing: -0.02em;
    color: var(--accent);
    line-height: 1.1;
  }

  .stat-label {
    margin-top: 4px;
    color: var(--muted);
    font-size: 11.5px;
  }

  .card-title {
    font-size: 12.5px;
    font-weight: 600;
    margin-bottom: 4px;
  }

  .note {
    margin: 0;
    color: var(--muted);
    font-size: 11.5px;
    line-height: 1.45;
  }

  .field {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 12px;
    padding: 8px 0;
    border-bottom: 1px solid var(--border);
    font-size: 12px;
  }

  .field label,
  .field .key {
    color: var(--muted);
  }

  .field input,
  .field select {
    padding: 4px 6px;
    border: 1px solid var(--border);
    border-radius: 6px;
    background: var(--panel);
    color: var(--text);
    font-size: 12px;
    text-align: right;
  }

  .toggle-row {
    display: flex;
    align-items: center;
    gap: 8px;
    margin: 10px 0;
    font-size: 12px;
    cursor: pointer;
  }

  .toggle-row input {
    accent-color: var(--accent);
  }

  /* Theme Choices */
  .theme-options {
    display: flex;
    gap: 8px;
  }

  .choice-btn {
    flex: 1;
    padding: 7px 8px;
    border: 1px solid var(--border);
    border-radius: 8px;
    font-size: 12px;
    color: var(--text);
    background: var(--surface);
  }

  .choice-btn.selected {
    border-color: var(--accent);
    background: var(--accent-soft);
    color: var(--accent);
    font-weight: 600;
  }

  .accent-palette {
    display: flex;
    gap: 10px;
    padding: 4px 0;
  }

  .accent-dot {
    width: 26px;
    height: 26px;
    border-radius: 50%;
    border: 2px solid transparent;
    cursor: pointer;
    transition: transform 0.15s ease, border-color 0.15s ease;
  }

  .accent-dot:hover {
    transform: scale(1.15);
  }

  .accent-dot.selected {
    border-color: var(--text);
    transform: scale(1.1);
  }

  /* Profiles */
  .profile-list {
    list-style: none;
    margin: 0 0 10px;
    padding: 0;
    display: flex;
    flex-direction: column;
    gap: 4px;
  }

  .profile-item {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 6px 8px;
    border-radius: 7px;
    background: var(--surface);
    border: 1px solid var(--border);
  }

  .profile-item.active-profile {
    border-color: var(--accent);
    background: var(--accent-soft);
  }

  .profile-info {
    display: flex;
    align-items: center;
    gap: 8px;
  }

  .profile-avatar {
    width: 20px;
    height: 20px;
    border-radius: 50%;
    background: var(--accent);
    color: #fff;
    font-size: 10px;
    font-weight: 700;
    display: flex;
    align-items: center;
    justify-content: center;
  }

  .profile-name {
    font-size: 12px;
    font-weight: 500;
  }

  .active-tag {
    font-size: 9px;
    padding: 1px 4px;
    border-radius: 4px;
    background: var(--accent);
    color: #fff;
    font-weight: 600;
  }

  .profile-actions {
    display: flex;
    gap: 4px;
  }

  .small-btn {
    padding: 2px 7px;
    border-radius: 5px;
    font-size: 11px;
    border: 1px solid var(--border);
    background: var(--panel);
  }

  .switch-btn:hover {
    border-color: var(--accent);
    color: var(--accent);
  }

  .del-btn:hover {
    border-color: #ef4444;
    color: #ef4444;
  }

  .add-profile-wrap {
    display: flex;
    gap: 6px;
    margin-top: 8px;
  }

  .new-profile-input {
    flex: 1;
    padding: 5px 8px;
    border: 1px solid var(--border);
    border-radius: 6px;
    font-size: 12px;
    background: var(--surface);
    color: var(--text);
  }

  .add-btn {
    padding: 5px 10px;
    border-radius: 6px;
    background: var(--accent);
    color: #fff;
    font-size: 12px;
    font-weight: 500;
  }

  .danger-btn {
    width: 100%;
    margin-top: 10px;
    padding: 6px 10px;
    border: 1px solid var(--border);
    border-radius: 6px;
    background: var(--hover);
    color: var(--text);
    font-size: 11.5px;
    cursor: pointer;
  }

  .danger-btn:hover {
    border-color: #ef4444;
    color: #ef4444;
  }

  /* Shortcuts List */
  .shortcut-list {
    display: flex;
    flex-direction: column;
    gap: 8px;
  }

  .key-row {
    display: flex;
    align-items: center;
    justify-content: space-between;
    font-size: 12px;
  }

  .key-row span {
    color: var(--muted);
  }

  kbd {
    display: inline-block;
    padding: 1px 5px;
    border-radius: 4px;
    background: var(--surface);
    border: 1px solid var(--border);
    font-family: inherit;
    font-size: 10.5px;
    font-weight: 600;
    color: var(--text);
  }

  .quit-btn {
    width: 100%;
    padding: 7px;
    border-radius: 7px;
    color: var(--muted);
    font-size: 12px;
    border: 1px solid var(--border);
  }

  .quit-btn:hover {
    background: var(--hover);
    color: #ef4444;
    border-color: #ef4444;
  }
</style>
