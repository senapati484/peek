<script lang="ts">
  import { onMount } from 'svelte';
  import {
    displayUrl,
    isBlank,
    onEvent,
    requestState,
    send,
    type BookmarkItem,
    type EdgeSide,
    type ExtensionSummary,
    type HistoryItem,
    type PeekEvent,
    type TabGroup,
    type TabInfo,
    type UserScriptSummary,
  } from './ipc';

  // Left Drawer components
  import TabsSection from './drawer/left/TabsSection.svelte';
  import SpacesSection from './drawer/left/SpacesSection.svelte';
  import HistorySection from './drawer/left/HistorySection.svelte';
  import BookmarksSection from './drawer/left/BookmarksSection.svelte';
  import NotesSection from './drawer/left/NotesSection.svelte';

  // Right Drawer components
  import SettingsGeneral from './drawer/right/SettingsGeneral.svelte';
  import SettingsThemes from './drawer/right/SettingsThemes.svelte';
  import SettingsBlocking from './drawer/right/SettingsBlocking.svelte';
  import SettingsPreview from './drawer/right/SettingsPreview.svelte';
  import SettingsProfiles from './drawer/right/SettingsProfiles.svelte';
  import SettingsExtensions from './drawer/right/SettingsExtensions.svelte';
  import SettingsUserScripts from './drawer/right/SettingsUserScripts.svelte';
  import SettingsSync from './drawer/right/SettingsSync.svelte';
  import SettingsShortcuts from './drawer/right/SettingsShortcuts.svelte';
  import SettingsAbout from './drawer/right/SettingsAbout.svelte';

  type Section =
    | 'tabs'
    | 'spaces'
    | 'history'
    | 'bookmarks'
    | 'notes'
    | 'general'
    | 'appearance'
    | 'blocking'
    | 'preview'
    | 'profiles'
    | 'extensions'
    | 'scripts'
    | 'sync'
    | 'shortcuts'
    | 'about';

  function getInitialParams() {
    const rawHash = window.location.hash.replace('#', '');
    const queryPart = rawHash.includes('?') ? rawHash.split('?')[1] : '';
    const params = new URLSearchParams(queryPart);
    const sideParam = params.get('side')?.toLowerCase();
    const sectionParam = params.get('section')?.toLowerCase();
    const validSections: Section[] = [
      'tabs',
      'spaces',
      'history',
      'bookmarks',
      'notes',
      'general',
      'appearance',
      'blocking',
      'preview',
      'profiles',
      'extensions',
      'scripts',
      'sync',
      'shortcuts',
      'about',
    ];

    let resolvedSection: Section = sideParam === 'right' ? 'general' : 'tabs';
    if (sectionParam) {
      if (sectionParam === 'settings') {
        resolvedSection = 'general';
      } else if (validSections.includes(sectionParam as Section)) {
        resolvedSection = sectionParam as Section;
      }
    }

    return {
      side: (sideParam === 'right' ? 'right' : 'left') as EdgeSide,
      section: resolvedSection,
    };
  }

  const initialParams = getInitialParams();
  let currentSide = $state<EdgeSide>(initialParams.side);
  let section = $state<Section>(initialParams.section);
  let animKey = $state(0);

  let tabs = $state<TabInfo[]>([]);
  let activeTabId = $state<number | null>(null);
  let blocked = $state(0);
  let allowedOnSite = $state(false);
  let host = $state<string | null>(null);
  let cookiesCount = $state(0);

  // Settings
  let maxLive = $state(5);
  let idleMinutes = $state(10);
  let hoverDelay = $state(150);
  let previewWidth = $state(480);
  let previewHeight = $state(320);
  let previewZoom = $state(0.75);
  let previewModifierOnly = $state(true);
  let configuredDrawerSide = $state<EdgeSide>('right');
  let edgeZone = $state(4);
  let startBlank = $state(false);
  let searchEngine = $state('Google');

  // Profiles
  let profiles = $state<string[]>(['default']);
  let activeProfile = $state('default');

  // Extensions & UserScripts
  let extensions = $state<ExtensionSummary[]>([]);
  let userScripts = $state<UserScriptSummary[]>([]);

  // Appearance & Theme
  let themeMode = $state<'system' | 'dark' | 'light'>('system');
  let accentColor = $state<'blue' | 'purple' | 'emerald' | 'amber' | 'rose'>('blue');

  // History & Bookmarks
  let historyEntries = $state<HistoryItem[]>([]);
  let bookmarksList = $state<BookmarkItem[]>([]);

  // Tab Groups
  let tabGroups = $state<TabGroup[]>([]);

  // Notes
  let noteContent = $state('');

  // Close timer
  let closeTimer: ReturnType<typeof setTimeout> | undefined;

  // Spaces State
  type SpaceId = 'personal' | 'work' | 'dev' | 'research';
  type SpaceInfo = {
    id: SpaceId;
    name: string;
    icon: string;
    color: string;
    desc: string;
  };

  const SPACES: SpaceInfo[] = [
    {
      id: 'personal',
      name: 'Personal',
      icon: '👤',
      color: '#3b82f6',
      desc: 'Daily browsing & leisure',
    },
    {
      id: 'work',
      name: 'Work',
      icon: '💼',
      color: '#a855f7',
      desc: 'Projects & company portals',
    },
    {
      id: 'dev',
      name: 'Dev',
      icon: '⚡',
      color: '#10b981',
      desc: 'GitHub, docs & localhost',
    },
    {
      id: 'research',
      name: 'Research',
      icon: '🔬',
      color: '#f59e0b',
      desc: 'Papers, articles & notes',
    },
  ];

  let activeSpace = $state<'all' | SpaceId>('all');
  let tabSpaceMap = $state<Record<number, SpaceId>>({});

  if (typeof localStorage !== 'undefined') {
    try {
      const raw = localStorage.getItem('peek_tab_spaces');
      if (raw) tabSpaceMap = JSON.parse(raw);
      const savedSpace = localStorage.getItem('peek_active_space');
      if (savedSpace) activeSpace = savedSpace as 'all' | SpaceId;
    } catch {}
  }

  function setSpace(space: 'all' | SpaceId) {
    activeSpace = space;
    if (typeof localStorage !== 'undefined') {
      localStorage.setItem('peek_active_space', space);
    }
  }

  function cycleTabSpace(tabId: number) {
    const spaceOrder: SpaceId[] = ['personal', 'work', 'dev', 'research'];
    const current = tabSpaceMap[tabId] || 'personal';
    const nextIdx = (spaceOrder.indexOf(current) + 1) % spaceOrder.length;
    const nextSpace = spaceOrder[nextIdx];
    tabSpaceMap = { ...tabSpaceMap, [tabId]: nextSpace };
    if (typeof localStorage !== 'undefined') {
      localStorage.setItem('peek_tab_spaces', JSON.stringify(tabSpaceMap));
    }
  }

  let activeTab = $derived(tabs.find((t) => t.id === activeTabId));
  let noteWords = $derived(
    noteContent.trim()
      ? noteContent.trim().split(/\s+/).filter(Boolean).length
      : 0,
  );

  function close(): void {
    send({ type: 'setDrawer', open: false });
  }

  function scheduleClose(): void {
    if (closeTimer) clearTimeout(closeTimer);
    closeTimer = setTimeout(close, 250);
  }

  function cancelClose(): void {
    if (closeTimer) {
      clearTimeout(closeTimer);
      closeTimer = undefined;
    }
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
    window.dispatchEvent(new Event('storage'));
  }

  function applyAccent(accent: 'blue' | 'purple' | 'emerald' | 'amber' | 'rose') {
    accentColor = accent;
    localStorage.setItem('peek_accent', accent);
    document.documentElement.dataset.accent = accent;
    window.dispatchEvent(new Event('storage'));
  }

  function handleEvent(event: PeekEvent): void {
    switch (event.type) {
      case 'drawerState': {
        const nextSide = (event.side as EdgeSide) || 'left';
        if (nextSide !== currentSide) {
          currentSide = nextSide;
          animKey += 1;
        }
        if (event.section) {
          const valid: Section[] = [
            'tabs', 'spaces', 'history', 'bookmarks', 'notes',
            'general', 'appearance', 'blocking', 'preview',
            'profiles', 'extensions', 'scripts', 'sync',
            'shortcuts', 'about',
          ];
          if (valid.includes(event.section as Section)) {
            section = event.section as Section;
          } else if (event.section === 'settings') {
            section = 'general';
          }
        }
        break;
      }
      case 'tabsChanged':
        tabs = event.tabs;
        activeTabId = event.active ?? (tabs.find((t) => t.active)?.id ?? null);
        break;
      case 'activeTabChanged':
        activeTabId = event.id;
        break;
      case 'historyList':
        historyEntries = event.entries;
        break;
      case 'bookmarksList':
        bookmarksList = event.bookmarks;
        break;
      case 'tabGroupsChanged':
        tabGroups = event.groups;
        break;
      case 'noteLoaded':
        noteContent = event.content;
        break;
      case 'userScriptsList':
        userScripts = event.scripts;
        break;
      case 'adblockCount':
        blocked = event.blocked;
        allowedOnSite = event.allowedOnSite;
        host = event.host;
        break;
      case 'cookiesCount':
        cookiesCount = event.count;
        break;
      case 'settings':
        maxLive = event.maxLive;
        idleMinutes = event.idleDiscardMinutes;
        hoverDelay = event.hoverDelayMs;
        configuredDrawerSide = event.drawerSide as EdgeSide;
        edgeZone = event.edgeZone;
        startBlank = event.startBlank;
        searchEngine = event.searchEngine || 'Google';
        break;
      case 'profilesList':
        profiles = event.profiles;
        activeProfile = event.active;
        break;
      case 'profileChanged':
        activeProfile = event.name;
        break;
      case 'extensionsList':
        extensions = event.extensions;
        break;
    }
  }

  onMount(() => {
    onEvent(handleEvent);
    requestState();
    send({ type: 'getHistory' });
    send({ type: 'getBookmarks' });
    send({ type: 'getNote' });
    send({ type: 'getUserScripts' });
    send({ type: 'getExtensions' });
    send({ type: 'getCookies' });

    const savedTheme = localStorage.getItem('peek_theme') as 'system' | 'dark' | 'light' | null;
    if (savedTheme) themeMode = savedTheme;
    const savedAccent = localStorage.getItem('peek_accent') as 'blue' | 'purple' | 'emerald' | 'amber' | 'rose' | null;
    if (savedAccent) accentColor = savedAccent;

    const onKey = (e: KeyboardEvent) => {
      if (e.key === 'Escape') {
        close();
      }
    };
    window.addEventListener('keydown', onKey, true);
    return () => {
      window.removeEventListener('keydown', onKey, true);
    };
  });
</script>

{#key animKey}
  <aside
    class="drawer"
    class:left={currentSide === 'left'}
    class:right={currentSide === 'right'}
    onmouseenter={cancelClose}
    onmouseleave={scheduleClose}
    aria-label="Peek drawer"
  >
    {#if currentSide === 'left'}
      <!-- Left Drawer: Dedicated Vertical Tabs, Spaces, History, Saved, and Notes -->
      <header class="tabs-header">
        <div class="brand-row">
          <div class="tabs-brand">
            <img src="/logo.png" alt="Peek" class="drawer-logo" />
            <span class="drawer-title">Workspace</span>
          </div>
          <button class="icon-close-btn" onclick={close} title="Close drawer (Esc)">
            <svg viewBox="0 0 16 16" fill="none" stroke="currentColor" class="close-icon">
              <path d="M4 4l8 8M12 4l-8 8" stroke-width="1.6" stroke-linecap="round"/>
            </svg>
          </button>
        </div>

        <nav class="left-nav-pills" aria-label="Views">
          <button
            class="left-pill"
            class:active={section === 'tabs'}
            onclick={() => (section = 'tabs')}
            type="button"
          >
            Tabs <span class="tab-count-badge">{tabs.length}</span>
          </button>
          <button
            class="left-pill"
            class:active={section === 'spaces'}
            onclick={() => (section = 'spaces')}
            type="button"
          >
            Spaces
          </button>
          <button
            class="left-pill"
            class:active={section === 'history'}
            onclick={() => { section = 'history'; send({ type: 'getHistory' }); }}
            type="button"
          >
            History
          </button>
          <button
            class="left-pill"
            class:active={section === 'bookmarks'}
            onclick={() => { section = 'bookmarks'; send({ type: 'getBookmarks' }); }}
            type="button"
          >
            Saved
          </button>
          <button
            class="left-pill"
            class:active={section === 'notes'}
            onclick={() => { section = 'notes'; send({ type: 'getNote' }); }}
            type="button"
          >
            Notes
          </button>
        </nav>
      </header>

      {#if section === 'tabs'}
        <TabsSection
          {tabs}
          {activeTabId}
          {tabGroups}
          {activeSpace}
          {tabSpaceMap}
          {SPACES}
          onSetSpace={setSpace}
          onCycleSpace={cycleTabSpace}
          onClose={close}
        />
      {:else if section === 'spaces'}
        <SpacesSection
          {SPACES}
          {tabs}
          {tabSpaceMap}
          {activeSpace}
          onSetSpace={setSpace}
          onSelectSection={(s) => (section = s)}
        />
      {:else if section === 'history'}
        <HistorySection bind:historyEntries onClose={close} />
      {:else if section === 'bookmarks'}
        <BookmarksSection bind:bookmarksList {activeTab} onClose={close} />
      {:else if section === 'notes'}
        <NotesSection bind:noteContent {activeTab} />
      {/if}

    {:else}
      <!-- Right Drawer: Settings & Tools -->
      <header class="settings-header">
        <div class="brand-row">
          <div class="tabs-brand">
            <img src="/logo.png" alt="Peek" class="drawer-logo" />
            <span class="drawer-title">Settings & Tools</span>
          </div>
          <button class="icon-close-btn" onclick={close} title="Close drawer (Esc)">
            <svg viewBox="0 0 16 16" fill="none" stroke="currentColor" class="close-icon">
              <path d="M4 4l8 8M12 4l-8 8" stroke-width="1.6" stroke-linecap="round"/>
            </svg>
          </button>
        </div>

        <nav class="settings-nav">
          <button class:on={section === 'general'} onclick={() => (section = 'general')}>General</button>
          <button class:on={section === 'appearance'} onclick={() => (section = 'appearance')}>Themes</button>
          <button class:on={section === 'blocking'} onclick={() => (section = 'blocking')}>Adblock</button>
          <button class:on={section === 'preview'} onclick={() => (section = 'preview')}>Preview</button>
          <button class:on={section === 'profiles'} onclick={() => (section = 'profiles')}>Profiles</button>
          <button class:on={section === 'extensions'} onclick={() => (section = 'extensions')}>Extensions</button>
          <button class:on={section === 'scripts'} onclick={() => { section = 'scripts'; send({ type: 'getUserScripts' }); }}>Scripts</button>
          <button class:on={section === 'sync'} onclick={() => (section = 'sync')}>Sync</button>
          <button class:on={section === 'shortcuts'} onclick={() => (section = 'shortcuts')}>Keys</button>
          <button class:on={section === 'about'} onclick={() => (section = 'about')}>About</button>
        </nav>
      </header>

      <div class="body settings-body">
        {#if section === 'general'}
          <SettingsGeneral
            {searchEngine}
            {configuredDrawerSide}
            {edgeZone}
            {startBlank}
            {maxLive}
            {idleMinutes}
            onSetting={setting}
          />
        {:else if section === 'appearance'}
          <SettingsThemes
            {themeMode}
            {accentColor}
            onApplyTheme={applyTheme}
            onApplyAccent={applyAccent}
          />
        {:else if section === 'blocking'}
          <SettingsBlocking
            {blocked}
            {host}
            {allowedOnSite}
            {cookiesCount}
          />
        {:else if section === 'preview'}
          <SettingsPreview
            bind:previewModifierOnly
            bind:hoverDelay
            bind:previewWidth
            bind:previewHeight
            bind:previewZoom
            onSetting={setting}
          />
        {:else if section === 'profiles'}
          <SettingsProfiles
            {profiles}
            {activeProfile}
          />
        {:else if section === 'extensions'}
          <SettingsExtensions {extensions} />
        {:else if section === 'scripts'}
          <SettingsUserScripts {userScripts} />
        {:else if section === 'sync'}
          <SettingsSync
            {bookmarksList}
            {tabGroups}
            {noteContent}
            {noteWords}
            onTabGroupsChange={(g) => (tabGroups = g)}
            onNoteChange={(c) => (noteContent = c)}
          />
        {:else if section === 'shortcuts'}
          <SettingsShortcuts />
        {:else if section === 'about'}
          <SettingsAbout onClose={close} />
        {/if}
      </div>

      <footer class="settings-footer">
        <div class="footer-meta">
          <span class="footer-brand">Peek</span>
          <span class="footer-version">v0.1.0</span>
          <span class="footer-dot">•</span>
          <button
            class="footer-gh-link"
            type="button"
            onclick={() => {
              send({ type: 'openUrl', url: 'https://github.com/senapati484/peek' });
              close();
            }}
            title="Open GitHub Repository"
          >
            ⭐ Star on GitHub
          </button>
        </div>
        <button class="quit-action-btn" onclick={() => send({ type: 'quit' })}>
          <svg viewBox="0 0 16 16" fill="none" stroke="currentColor" class="quit-icon">
            <path d="M8 2v5M4.93 4.93a5 5 0 106.14 0" stroke-width="1.5" stroke-linecap="round" />
          </svg>
          Quit Peek
        </button>
      </footer>
    {/if}
  </aside>
{/key}

<style>
  .drawer {
    display: flex;
    flex-direction: column;
    height: 100vh;
    background: var(--panel);
    box-shadow: var(--shadow-lg);
    animation: drawerSlide 0.2s cubic-bezier(0.16, 1, 0.3, 1) both;
    will-change: transform, opacity;
  }

  .drawer.left {
    border-inline-end: 1px solid var(--border);
    --slide-dir: -28px;
  }

  .drawer.right {
    border-inline-start: 1px solid var(--border);
    --slide-dir: 28px;
  }

  @keyframes drawerSlide {
    0% {
      opacity: 0.1;
      transform: translateX(var(--slide-dir, 28px));
    }
    100% {
      opacity: 1;
      transform: translateX(0);
    }
  }

  .tabs-header,
  .settings-header {
    padding: 12px 14px 10px;
    border-bottom: 1px solid var(--border);
    flex-shrink: 0;
    display: flex;
    flex-direction: column;
    gap: 8px;
  }

  .brand-row {
    display: flex;
    align-items: center;
    justify-content: space-between;
  }

  .tabs-brand {
    display: flex;
    align-items: center;
    gap: 8px;
  }

  .drawer-logo {
    width: 20px;
    height: 20px;
    display: block;
    user-select: none;
    -webkit-user-drag: none;
    filter: drop-shadow(0 1px 3px rgba(0, 0, 0, 0.4));
  }

  .drawer-title {
    font-size: 13.5px;
    font-weight: 600;
    color: var(--text);
    letter-spacing: -0.01em;
  }

  .icon-close-btn {
    all: unset;
    cursor: pointer;
    width: 24px;
    height: 24px;
    border-radius: 6px;
    display: flex;
    align-items: center;
    justify-content: center;
    color: var(--muted);
    transition: all 0.12s ease;
  }

  .icon-close-btn:hover {
    color: var(--text);
    background: var(--hover);
  }

  .close-icon {
    width: 14px;
    height: 14px;
  }

  .left-nav-pills {
    display: grid;
    grid-template-columns: repeat(5, 1fr);
    gap: 3px;
    padding: 3px;
    background: var(--surface-2);
    border: 1px solid var(--border-subtle);
    border-radius: 9px;
  }

  .left-pill {
    all: unset;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    gap: 4px;
    padding: 5px 2px;
    border-radius: 6px;
    font-size: 11px;
    font-weight: 550;
    color: var(--muted);
    cursor: pointer;
    transition: all 0.14s ease;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .left-pill:hover {
    color: var(--text);
    background: color-mix(in srgb, var(--surface) 60%, transparent);
  }

  .left-pill.active {
    background: var(--surface);
    color: var(--text);
    box-shadow: var(--shadow-sm);
    font-weight: 600;
  }

  .tab-count-badge {
    font-size: 9.5px;
    padding: 0 4px;
    border-radius: 999px;
    background: var(--surface-2);
    color: var(--muted);
  }

  .left-pill.active .tab-count-badge {
    color: var(--text);
  }

  .settings-nav {
    display: grid;
    grid-template-columns: repeat(5, 1fr);
    gap: 4px;
    padding: 3px;
    background: var(--surface-2);
    border: 1px solid var(--border-subtle);
    border-radius: 9px;
  }

  .settings-nav button {
    padding: 5px 2px;
    border-radius: 6px;
    color: var(--muted);
    font-size: 11px;
    font-weight: 500;
    text-align: center;
    transition: all 0.14s ease;
    border: 1px solid transparent;
    background: transparent;
    cursor: pointer;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .settings-nav button:hover {
    color: var(--text);
    background: color-mix(in srgb, var(--surface) 60%, transparent);
  }

  .settings-nav button.on {
    background: var(--surface);
    color: var(--text);
    font-weight: 600;
    border-color: var(--border);
    box-shadow: 0 1px 3px rgba(0, 0, 0, 0.2);
  }

  .body.settings-body {
    flex: 1;
    overflow-y: auto;
    padding: 12px;
  }

  .settings-footer {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 10px 14px;
    border-top: 1px solid var(--border);
    background: var(--panel);
    flex-shrink: 0;
  }

  .footer-meta {
    display: flex;
    align-items: center;
    gap: 6px;
    font-size: 11px;
    color: var(--muted);
  }

  .footer-brand {
    font-weight: 600;
    color: var(--text);
  }

  .footer-version {
    font-size: 10.5px;
    opacity: 0.8;
  }

  .footer-dot {
    color: var(--border);
    font-size: 9px;
  }

  .footer-gh-link {
    all: unset;
    cursor: pointer;
    font-size: 11px;
    font-weight: 550;
    color: var(--muted);
    transition: color 0.14s ease;
  }

  .footer-gh-link:hover {
    color: var(--accent);
  }

  .quit-action-btn {
    all: unset;
    display: inline-flex;
    align-items: center;
    gap: 6px;
    padding: 5px 11px;
    border-radius: 7px;
    background: transparent;
    border: 1px solid var(--border);
    color: var(--muted);
    font-size: 11.5px;
    font-weight: 500;
    cursor: pointer;
    transition: all 0.15s ease;
  }

  .quit-action-btn:hover {
    color: #ef4444;
    border-color: color-mix(in srgb, #ef4444 35%, var(--border));
    background: color-mix(in srgb, #ef4444 10%, transparent);
  }

  .quit-icon {
    width: 12px;
    height: 12px;
  }
</style>
