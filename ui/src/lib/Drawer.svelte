<script lang="ts">
  import { onMount } from 'svelte';
  import {
    onEvent,
    requestState,
    send,
    type EdgeSide,
    type ExtensionSummary,
    type PeekEvent,
    type TabInfo,
  } from './ipc';

  type Section = 'tabs' | 'general' | 'appearance' | 'blocking' | 'preview' | 'profiles' | 'extensions' | 'shortcuts' | 'about';

  function getInitialParams() {
    const rawHash = window.location.hash.replace('#', '');
    const queryPart = rawHash.includes('?') ? rawHash.split('?')[1] : '';
    const params = new URLSearchParams(queryPart);
    const sideParam = params.get('side')?.toLowerCase();
    const sectionParam = params.get('section')?.toLowerCase();
    const validSections: Section[] = [
      'tabs', 'general', 'appearance', 'blocking', 'preview', 'profiles', 'extensions', 'shortcuts', 'about'
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
  let newProfileName = $state('');

  // Extensions
  let extensions = $state<ExtensionSummary[]>([]);
  let extensionInstallPath = $state('');

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

  function domainInitial(url: string): string {
    if (!url || url === 'about:blank') return '✦';
    try {
      const u = new URL(url);
      const h = u.hostname.replace(/^www\./, '');
      return (h[0] || '✦').toUpperCase();
    } catch {
      return '✦';
    }
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

  function handleInstallExtension() {
    const path = extensionInstallPath.trim();
    if (!path) return;
    send({ type: 'installExtension', path });
    extensionInstallPath = '';
  }

  function handleToggleExtension(id: string, current: boolean) {
    send({ type: 'toggleExtension', id, enabled: !current });
  }

  function handleUninstallExtension(id: string) {
    if (confirm('Uninstall this extension?')) {
      send({ type: 'uninstallExtension', id });
    }
  }

  function handleEvent(event: PeekEvent): void {
    switch (event.type) {
      case 'tabsChanged':
        tabs = event.tabs;
        activeTabId = event.tabs.find((t) => t.active)?.id ?? null;
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
          // rust closed
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
            if (event.section === 'settings') {
              section = 'general';
            } else {
              section = event.section as Section;
            }
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
      case 'extensionsList':
        extensions = event.extensions;
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
    const savedTheme = localStorage.getItem('peek_theme') as 'system' | 'dark' | 'light' | null;
    if (savedTheme) applyTheme(savedTheme);
    const savedAccent = localStorage.getItem('peek_accent') as 'blue' | 'purple' | 'emerald' | 'amber' | 'rose' | null;
    if (savedAccent) applyAccent(savedAccent);

    onEvent(handleEvent);
    requestState();
    const onKey = (event: KeyboardEvent) => {
      if (event.key === 'Escape') {
        event.preventDefault();
        close();
        return;
      }
      const mod = event.metaKey || event.ctrlKey;
      const key = event.key.toLowerCase();
      if (mod && !event.shiftKey && !event.altKey) {
        if (key === 't') {
          event.preventDefault();
          send({ type: 'newTab' });
          close();
          return;
        }
        if (key === 'w') {
          event.preventDefault();
          if (activeTabId != null) {
            send({ type: 'closeTab', id: activeTabId });
          }
          return;
        }
      }
      if (mod && event.shiftKey && !event.altKey) {
        if (key === '[' || key === '{') {
          event.preventDefault();
          if (currentSide === 'left') {
            close();
          } else {
            currentSide = 'left';
            section = 'tabs';
          }
          return;
        }
        if (key === ']' || key === '}') {
          event.preventDefault();
          if (currentSide === 'right') {
            close();
          } else {
            currentSide = 'right';
            section = 'general';
          }
          return;
        }
      }
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
      <button class="icon-close-btn" onclick={close} title="Close tabs (Esc)">
        <svg viewBox="0 0 16 16" fill="none" stroke="currentColor" class="close-icon">
          <path d="M4 4l8 8M12 4l-8 8" stroke-width="1.6" stroke-linecap="round"/>
        </svg>
      </button>
    </header>

    <div class="filter-wrap">
      <div class="filter-box">
        <svg viewBox="0 0 16 16" fill="none" stroke="currentColor" class="filter-icon">
          <circle cx="7" cy="7" r="4.5" stroke-width="1.6" />
          <path d="M10.5 10.5L14 14" stroke-width="1.6" stroke-linecap="round" />
        </svg>
        <input
          type="search"
          placeholder="Filter tabs..."
          bind:value={filterText}
          class="filter-input"
        />
        {#if filterText}
          <button class="clear-filter" onclick={() => filterText = ''}>×</button>
        {/if}
      </div>
    </div>

    <div class="body tabs-body">
      <ul class="tabs-list">
        {#each filteredTabs as tab, idx (tab.id)}
          <li
            class:active={tab.id === activeTabId}
            class:discarded={tab.discarded}
            draggable="true"
            ondragstart={(e) => onDragStart(e, tab.id)}
            ondragover={(e) => e.preventDefault()}
            ondrop={(e) => onDrop(e, idx)}
          >
            <button
              class="tab-row"
              onclick={() => switchTo(tab.id)}
              title={`${tab.title || (tab.url === 'about:blank' ? 'New Tab' : tab.url)}\n${tab.url}`}
            >
              {#if tab.favicon}
                <img
                  src={tab.favicon}
                  alt=""
                  class="favicon"
                  onerror={(e) => { (e.currentTarget as HTMLImageElement).style.display = 'none'; }}
                />
              {:else}
                <span class="favicon-initial">{domainInitial(tab.url)}</span>
              {/if}
              <span class="title">{tab.title || (tab.url === 'about:blank' ? 'New Tab' : tab.url) || 'New Tab'}</span>
              {#if tab.audible}
                <span class="pill audio" title="Audible tab">
                  <svg viewBox="0 0 16 16" fill="currentColor" class="pill-icon">
                    <path d="M11.559 1.44a.75.75 0 0 1 .441.682v11.756a.75.75 0 0 1-1.28.53l-3.97-3.97H4.25a1.75 1.75 0 0 1-1.75-1.75V7.318c0-.966.784-1.75 1.75-1.75h2.5l3.97-3.97a.75.75 0 0 1 .839-.158z" />
                  </svg>
                  Audio
                </span>
              {/if}
              {#if tab.discarded}
                <span class="pill sleep" title="Tab sleeping to save memory">
                  zZ
                </span>
              {/if}
            </button>
            <button
              class="tab-close"
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
          <li class="empty-state">
            <span class="empty-icon">🔍</span>
            <span class="empty-text">No tabs match "{filterText}"</span>
          </li>
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
        <button class:on={section === 'shortcuts'} onclick={() => (section = 'shortcuts')}>Keys</button>
        <button class:on={section === 'about'} onclick={() => (section = 'about')}>About</button>
      </nav>
    </header>

    <div class="body settings-body">
      {#if section === 'general'}
        <div class="section-title">Search & Browsing</div>
        <div class="card settings-card">
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
            <label for="edge">Edge Hover Zone (px)</label>
            <input
              id="edge"
              type="number"
              min="1"
              max="16"
              value={edgeZone}
              onchange={(e) => setting('edgeZone', Number((e.currentTarget as HTMLInputElement).value))}
            />
          </div>

          <div class="field switch-field">
            <span>Always open new tabs with Start Page</span>
            <label class="switch">
              <input
                type="checkbox"
                checked={startBlank}
                onchange={(e) => setting('startBlank', (e.currentTarget as HTMLInputElement).checked)}
              />
              <span class="slider"></span>
            </label>
          </div>
        </div>

        <div class="section-title" style="margin-top: 18px;">Tab & Memory Budget</div>
        <div class="card settings-card">
          <div class="field">
            <div>
              <label for="max-live">Max Live Tabs (LRU)</label>
              <p class="field-desc">Background tabs beyond this count are discarded</p>
            </div>
            <input
              id="max-live"
              type="number"
              min="1"
              max="20"
              value={maxLive}
              onchange={(e) => setting('maxLive', Number((e.currentTarget as HTMLInputElement).value))}
            />
          </div>

          <div class="field" style="border-bottom: none;">
            <div>
              <label for="idle-time">Idle Discard (minutes)</label>
              <p class="field-desc">Inactive background tabs are put to sleep</p>
            </div>
            <input
              id="idle-time"
              type="number"
              min="1"
              max="120"
              value={idleMinutes}
              onchange={(e) => setting('idleDiscardMinutes', Number((e.currentTarget as HTMLInputElement).value))}
            />
          </div>
        </div>

      {:else if section === 'appearance'}
        <div class="section-title">Color Scheme</div>
        <div class="theme-options">
          <button
            class="choice-btn"
            class:selected={themeMode === 'system'}
            onclick={() => applyTheme('system')}
          >
            <span class="theme-icon">◐</span>
            Auto System
          </button>
          <button
            class="choice-btn"
            class:selected={themeMode === 'dark'}
            onclick={() => applyTheme('dark')}
          >
            <span class="theme-icon">☾</span>
            Dark Mode
          </button>
          <button
            class="choice-btn"
            class:selected={themeMode === 'light'}
            onclick={() => applyTheme('light')}
          >
            <span class="theme-icon">☼</span>
            Light Mode
          </button>
        </div>

        <div class="section-title" style="margin-top: 20px;">Accent Hue</div>
        <div class="card settings-card">
          <div class="accent-palette">
            <button
              class="accent-dot"
              class:selected={accentColor === 'blue'}
              style="background: #3b82f6;"
              onclick={() => applyAccent('blue')}
              title="Electric Blue"
            ></button>
            <button
              class="accent-dot"
              class:selected={accentColor === 'purple'}
              style="background: #8b5cf6;"
              onclick={() => applyAccent('purple')}
              title="Neon Violet"
            ></button>
            <button
              class="accent-dot"
              class:selected={accentColor === 'emerald'}
              style="background: #10b981;"
              onclick={() => applyAccent('emerald')}
              title="Emerald Green"
            ></button>
            <button
              class="accent-dot"
              class:selected={accentColor === 'amber'}
              style="background: #f59e0b;"
              onclick={() => applyAccent('amber')}
              title="Warm Amber"
            ></button>
            <button
              class="accent-dot"
              class:selected={accentColor === 'rose'}
              style="background: #f43f5e;"
              onclick={() => applyAccent('rose')}
              title="Rose Quartz"
            ></button>
          </div>
        </div>

      {:else if section === 'blocking'}
        <div class="card stat-card">
          <div class="stat-big">{blocked}</div>
          <div class="stat-label">Trackers & Ads Blocked This Session</div>
        </div>

        {#if host}
          <div class="card settings-card" style="margin-top: 14px;">
            <div class="field switch-field" style="border-bottom: none;">
              <div>
                <span class="field-label-bold">Protection on {host}</span>
                <p class="field-desc">Adblock and tracker protection on current site</p>
              </div>
              <label class="switch">
                <input
                  type="checkbox"
                  checked={!allowedOnSite}
                  onchange={() => { if (host) send({ type: 'setSiteAdblock', host, enabled: allowedOnSite }); }}
                />
                <span class="slider"></span>
              </label>
            </div>
          </div>
        {/if}

        <div class="section-title" style="margin-top: 18px;">Built-in Privacy Engine</div>
        <div class="card settings-card">
          <div class="filter-item">
            <div class="filter-status-dot"></div>
            <div>
              <div class="filter-name">EasyList & EasyPrivacy</div>
              <div class="filter-desc">Network-level blocking & cosmetic filtering</div>
            </div>
          </div>
          <div class="filter-item">
            <div class="filter-status-dot"></div>
            <div>
              <div class="filter-name">uBlock Origin Filters</div>
              <div class="filter-desc">Anti-malware & behavioral telemetry prevention</div>
            </div>
          </div>
        </div>

        <div class="card settings-card" style="margin-top: 14px;">
          <div class="field" style="border-bottom: none;">
            <div>
              <span class="field-label-bold">Cookies & Local Storage</span>
              <p class="field-desc">{cookiesCount} stored domain cookies</p>
            </div>
            <button class="danger-btn" onclick={() => send({ type: 'clearCookies' })}>
              Clear Data
            </button>
          </div>
        </div>

      {:else if section === 'preview'}
        <div class="section-title">Hover Preview (Signature Feature)</div>
        <div class="card settings-card">
          <div class="field switch-field">
            <div>
              <span class="field-label-bold">Require ⌘ / Ctrl Key</span>
              <p class="field-desc">Hold ⌘ (macOS) or Ctrl while hovering links</p>
            </div>
            <label class="switch">
              <input
                type="checkbox"
                checked={previewModifierOnly}
                onchange={(e) => {
                  previewModifierOnly = (e.currentTarget as HTMLInputElement).checked;
                  setting('previewModifierOnly', previewModifierOnly);
                }}
              />
              <span class="slider"></span>
            </label>
          </div>

          <div class="field">
            <label for="h-delay">Hover Delay (ms)</label>
            <input
              id="h-delay"
              type="number"
              min="50"
              max="1500"
              step="50"
              value={hoverDelay}
              onchange={(e) => {
                hoverDelay = Number((e.currentTarget as HTMLInputElement).value);
                setting('hoverDelayMs', hoverDelay);
              }}
            />
          </div>

          <div class="field">
            <label for="p-width">Preview Width (px)</label>
            <input
              id="p-width"
              type="number"
              min="320"
              max="800"
              step="20"
              value={previewWidth}
              onchange={(e) => {
                previewWidth = Number((e.currentTarget as HTMLInputElement).value);
                setting('previewWidth', previewWidth);
              }}
            />
          </div>

          <div class="field">
            <label for="p-height">Preview Height (px)</label>
            <input
              id="p-height"
              type="number"
              min="200"
              max="600"
              step="20"
              value={previewHeight}
              onchange={(e) => {
                previewHeight = Number((e.currentTarget as HTMLInputElement).value);
                setting('previewHeight', previewHeight);
              }}
            />
          </div>

          <div class="field" style="border-bottom: none;">
            <label for="p-zoom">Preview Zoom Ratio</label>
            <input
              id="p-zoom"
              type="number"
              min="0.5"
              max="1.0"
              step="0.05"
              value={previewZoom}
              onchange={(e) => {
                previewZoom = Number((e.currentTarget as HTMLInputElement).value);
                setting('previewZoom', previewZoom);
              }}
            />
          </div>
        </div>

      {:else if section === 'profiles'}
        <div class="section-title">Active Profiles</div>
        <ul class="profile-list">
          {#each profiles as p}
            <li class="profile-item card" class:active-profile={p === activeProfile}>
              <div class="profile-info">
                <div class="profile-avatar">{p[0].toUpperCase()}</div>
                <div class="profile-meta">
                  <div class="profile-name-row">
                    <span class="profile-name">{p}</span>
                    {#if p === activeProfile}
                      <span class="profile-badge">Active</span>
                    {/if}
                  </div>
                  <span class="profile-subtext">
                    {p === activeProfile ? 'Current workspace profile' : 'Isolated browsing profile'}
                  </span>
                </div>
              </div>
              <div class="profile-actions">
                {#if p !== activeProfile}
                  <button class="switch-profile-btn" onclick={() => handleSwitchProfile(p)}>
                    Switch
                  </button>
                  <button
                    class="delete-profile-btn"
                    onclick={() => handleDeleteProfile(p)}
                    title="Delete profile"
                  >
                    <svg viewBox="0 0 16 16" fill="none" stroke="currentColor" class="action-icon">
                      <path d="M3 4.5h10M5.5 4.5V3a1 1 0 011-1h3a1 1 0 011 1v1.5M6 7.5v5M10 7.5v5M4.5 4.5l.7 9a1.5 1.5 0 001.5 1.4h4.6a1.5 1.5 0 001.5-1.4l.7-9" stroke-width="1.3" stroke-linecap="round" stroke-linejoin="round"/>
                    </svg>
                  </button>
                {/if}
              </div>
            </li>
          {/each}
        </ul>

        <div class="card create-profile-card">
          <div class="create-profile-header">
            <span class="create-title">Create New Profile</span>
            <span class="create-desc">Isolated cookies, history, tabs, and storage</span>
          </div>
          <div class="create-profile-input-wrap">
            <input
              type="text"
              placeholder="Profile name (e.g. Work, Research)"
              bind:value={newProfileName}
              onkeydown={(e) => { if (e.key === 'Enter') handleCreateProfile(); }}
              class="create-profile-input"
            />
            <button class="create-profile-btn" onclick={handleCreateProfile}>
              Create
            </button>
          </div>
        </div>

      {:else if section === 'extensions'}
        <div class="section-title">Install Chrome Extension (MV3)</div>
        <div class="card settings-card">
          <p class="field-desc" style="margin-bottom: 8px;">
            Supports Manifest V3 unpacked directories and .crx packages with isolated content scripts.
          </p>
          <div class="input-action-row">
            <input
              type="text"
              placeholder="/path/to/extension or extension.crx"
              bind:value={extensionInstallPath}
              class="text-input"
            />
            <button class="primary-btn" onclick={handleInstallExtension}>Install</button>
          </div>
        </div>

        <div class="section-title" style="margin-top: 18px;">Installed Extensions ({extensions.length})</div>
        {#if extensions.length === 0}
          <div class="card empty-card">
            <div class="empty-icon">🧩</div>
            <p class="note">No extensions installed yet. Enter an unpacked directory path above to install.</p>
          </div>
        {:else}
          <div class="extensions-list">
            {#each extensions as ext (ext.id)}
              <div class="card ext-card">
                <div class="ext-header">
                  <div>
                    <div class="ext-title">{ext.name} <span class="ext-version">v{ext.version}</span></div>
                    <div class="ext-id">{ext.id}</div>
                  </div>
                  <div class="ext-controls">
                    <label class="switch">
                      <input
                        type="checkbox"
                        checked={ext.enabled}
                        onchange={() => handleToggleExtension(ext.id, ext.enabled)}
                      />
                      <span class="slider"></span>
                    </label>
                    <button class="danger-icon-btn" onclick={() => handleUninstallExtension(ext.id)} title="Uninstall">
                      ✕
                    </button>
                  </div>
                </div>

                <div class="ext-badges">
                  {#if ext.fullySupported}
                    <span class="badge verified">Tier 1 Verified</span>
                  {:else}
                    <span class="badge partial">Partial MV3</span>
                  {/if}
                </div>
              </div>
            {/each}
          </div>
        {/if}

      {:else if section === 'shortcuts'}
        <div class="section-title">Navigation & Window</div>
        <div class="shortcut-list card settings-card">
          <div class="key-row"><kbd>⌘ L</kbd> <span>Focus address and search bar</span></div>
          <div class="key-row"><kbd>⌘ T</kbd> <span>Open blank new tab</span></div>
          <div class="key-row"><kbd>⌘ W</kbd> <span>Close current active tab</span></div>
          <div class="key-row"><kbd>⌘ R</kbd> <span>Reload current page</span></div>
          <div class="key-row"><kbd>Alt + ← / →</kbd> <span>Go back / forward in history</span></div>
        </div>

        <div class="section-title" style="margin-top: 16px;">Peek Signatures</div>
        <div class="shortcut-list card settings-card">
          <div class="key-row"><kbd>⌘ + Hover</kbd> <span>Trigger live link preview</span></div>
          <div class="key-row"><kbd>⌘ ⇧ [</kbd> <span>Toggle vertical tabs drawer</span></div>
          <div class="key-row"><kbd>⌘ ⇧ ]</kbd> <span>Toggle settings & tools drawer</span></div>
          <div class="key-row"><kbd>Esc</kbd> <span>Dismiss preview or close drawer</span></div>
        </div>

      {:else if section === 'about'}
        <div class="hero-brand" style="margin: 20px 0;">
          <img src="/logo.png" alt="Peek" class="drawer-logo" style="width: 48px; height: 48px;" />
          <h2 style="margin: 10px 0 2px; font-size: 19px; font-weight: 700;">Peek Browser</h2>
          <span style="font-size: 12px; color: var(--muted);">v0.1.0 • Pure Rust Native Architecture</span>
        </div>
        <div class="card settings-card">
          <p class="note">
            Peek is built with zero framework bloat using direct <code>tao</code> and <code>wry</code> bindings, native system WebViews, Brave adblock engine, and an ephemeral hover preview architecture.
          </p>
        </div>
        <div class="card settings-card" style="margin-top: 12px;">
          <div class="tech-row"><span>Engine</span> <strong>tao + wry (WebKit/WebView2)</strong></div>
          <div class="tech-row"><span>Database</span> <strong>SQLite FTS5 + Connection Pool</strong></div>
          <div class="tech-row"><span>Adblock</span> <strong>Brave Adblock Engine (Rust)</strong></div>
          <div class="tech-row"><span>Extensions</span> <strong>Manifest V3 Host Engine</strong></div>
        </div>
      {/if}
    </div>

    <footer class="settings-footer">
      <div class="footer-meta">
        <span class="footer-brand">Peek</span>
        <span class="footer-version">v0.1.0</span>
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

<style>
  .drawer {
    display: flex;
    flex-direction: column;
    height: 100vh;
    background: var(--panel);
    box-shadow: var(--shadow-lg);
    animation: drawerSlide 0.22s cubic-bezier(0.16, 1, 0.3, 1);
  }

  .drawer.left {
    border-inline-end: 1px solid var(--border);
    --slide-dir: -12px;
  }

  .drawer.right {
    border-inline-start: 1px solid var(--border);
    --slide-dir: 12px;
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
    letter-spacing: -0.01em;
  }

  .tab-count-badge {
    font-size: 10.5px;
    padding: 1px 6px;
    border-radius: 999px;
    background: var(--surface-2);
    color: var(--muted);
    font-weight: 600;
  }

  .icon-close-btn {
    width: 26px;
    height: 26px;
    border-radius: 50%;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    color: var(--muted);
    background: var(--surface-2);
    border: 1px solid var(--border-subtle);
    cursor: pointer;
    transition: all 0.15s ease;
  }

  .icon-close-btn:hover {
    color: var(--text);
    background: var(--hover);
    border-color: var(--border);
  }

  .close-icon {
    width: 12px;
    height: 12px;
  }

  /* Filter */
  .filter-wrap {
    padding: 8px 12px;
    border-bottom: 1px solid var(--border);
  }

  .filter-box {
    display: flex;
    align-items: center;
    gap: 7px;
    padding: 4px 8px;
    border: 1px solid var(--border);
    border-radius: var(--radius-sm);
    background: var(--surface);
    transition: border-color 0.15s ease;
  }

  .filter-box:focus-within {
    border-color: var(--accent);
  }

  .filter-icon {
    width: 13px;
    height: 13px;
    color: var(--muted);
    flex-shrink: 0;
  }

  .filter-input {
    flex: 1;
    border: none;
    outline: none;
    background: transparent;
    color: var(--text);
    font-size: 12px;
  }

  .clear-filter {
    color: var(--muted);
    font-size: 14px;
    line-height: 1;
    cursor: pointer;
  }

  /* Settings Navigation */
  .settings-nav {
    display: grid;
    grid-template-columns: repeat(4, 1fr);
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
    font-weight: 550;
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
    color: var(--text);
  }

  .favicon {
    flex: 0 0 15px;
    width: 15px;
    height: 15px;
    border-radius: 3px;
    object-fit: contain;
  }

  .favicon-initial {
    flex: 0 0 15px;
    width: 15px;
    height: 15px;
    border-radius: 3px;
    background: var(--surface-2);
    color: var(--muted);
    font-size: 9px;
    font-weight: 700;
    display: flex;
    align-items: center;
    justify-content: center;
  }

  .pill {
    flex: 0 0 auto;
    padding: 1px 5px;
    border-radius: 999px;
    font-size: 9.5px;
    letter-spacing: 0.04em;
    font-weight: 600;
    text-transform: uppercase;
    display: inline-flex;
    align-items: center;
    gap: 3px;
  }

  .pill-icon {
    width: 9px;
    height: 9px;
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

  .empty-state {
    display: flex;
    flex-direction: column;
    align-items: center;
    padding: 24px 12px;
    color: var(--muted);
    font-size: 12px;
    gap: 6px;
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
    transition: border-color 0.15s ease, color 0.15s ease;
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
    font-size: 11.5px;
    font-weight: 650;
    color: var(--text);
    margin-bottom: 7px;
    letter-spacing: -0.01em;
  }

  .card {
    padding: 12px 14px;
    border: 1px solid var(--border);
    border-radius: var(--radius);
    background: var(--surface);
    box-shadow: var(--shadow-sm);
  }

  .stat-card {
    text-align: center;
    padding: 16px;
    background: radial-gradient(circle at 50% 20%, var(--accent-soft) 0%, var(--surface) 80%);
  }

  .stat-big {
    font-size: 36px;
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
    border-bottom: 1px solid var(--border-subtle);
    font-size: 12px;
  }

  .field-desc {
    margin: 2px 0 0;
    font-size: 10.5px;
    color: var(--muted);
  }

  .field label {
    color: var(--text);
    font-weight: 500;
  }

  .field-label-bold {
    font-weight: 600;
    color: var(--text);
  }

  .field input[type="number"],
  .field select {
    padding: 4px 7px;
    border: 1px solid var(--border);
    border-radius: 6px;
    background: var(--panel);
    color: var(--text);
    font-size: 12px;
    text-align: right;
  }

  /* iOS/macOS Slider Switch */
  .switch {
    position: relative;
    display: inline-block;
    width: 34px;
    height: 18px;
    flex-shrink: 0;
  }

  .switch input {
    opacity: 0;
    width: 0;
    height: 0;
  }

  .slider {
    position: absolute;
    cursor: pointer;
    inset: 0;
    background-color: var(--border);
    transition: 0.18s cubic-bezier(0.16, 1, 0.3, 1);
    border-radius: 20px;
  }

  .slider:before {
    position: absolute;
    content: "";
    height: 14px;
    width: 14px;
    left: 2px;
    bottom: 2px;
    background-color: white;
    transition: 0.18s cubic-bezier(0.16, 1, 0.3, 1);
    border-radius: 50%;
    box-shadow: 0 1px 2px rgba(0, 0, 0, 0.3);
  }

  input:checked + .slider {
    background-color: var(--accent);
  }

  input:checked + .slider:before {
    transform: translateX(16px);
  }

  /* Theme Choices */
  .theme-options {
    display: flex;
    gap: 8px;
  }

  .choice-btn {
    flex: 1;
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 4px;
    padding: 8px 6px;
    border: 1px solid var(--border);
    border-radius: 8px;
    font-size: 11.5px;
    color: var(--text);
    background: var(--surface);
    transition: border-color 0.15s ease, background 0.15s ease;
  }

  .choice-btn.selected {
    border-color: var(--accent);
    background: var(--accent-soft);
    color: var(--accent);
    font-weight: 600;
  }

  .theme-icon {
    font-size: 15px;
  }

  .accent-palette {
    display: flex;
    gap: 12px;
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
    transform: scale(1.12);
  }

  /* Adblock & Privacy items */
  .filter-item {
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 7px 0;
    border-bottom: 1px solid var(--border-subtle);
  }

  .filter-item:last-child {
    border-bottom: none;
  }

  .filter-status-dot {
    width: 8px;
    height: 8px;
    border-radius: 50%;
    background: #10b981;
    box-shadow: 0 0 6px rgba(16, 185, 129, 0.5);
  }

  .filter-name {
    font-size: 12px;
    font-weight: 600;
    color: var(--text);
  }

  .filter-desc {
    font-size: 10.5px;
    color: var(--muted);
  }

  /* Profiles */
  .profile-list {
    list-style: none;
    margin: 0 0 12px;
    padding: 0;
    display: flex;
    flex-direction: column;
    gap: 8px;
  }

  .profile-item {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 11px 13px;
    border-radius: 12px;
    background: var(--surface);
    border: 1px solid var(--border);
    transition: all 0.15s ease;
  }

  .profile-item.active-profile {
    border-color: color-mix(in srgb, var(--accent) 45%, var(--border));
    background: color-mix(in srgb, var(--accent) 8%, var(--surface));
    box-shadow: 0 2px 8px color-mix(in srgb, var(--accent) 12%, transparent);
  }

  .profile-info {
    display: flex;
    align-items: center;
    gap: 11px;
    min-width: 0;
  }

  .profile-avatar {
    width: 34px;
    height: 34px;
    border-radius: 10px;
    background: linear-gradient(135deg, var(--accent), color-mix(in srgb, var(--accent) 55%, #000));
    color: #ffffff;
    font-weight: 700;
    font-size: 13px;
    display: flex;
    align-items: center;
    justify-content: center;
    flex-shrink: 0;
    box-shadow: 0 2px 6px rgba(0, 0, 0, 0.25);
  }

  .profile-meta {
    display: flex;
    flex-direction: column;
    gap: 2px;
    min-width: 0;
  }

  .profile-name-row {
    display: flex;
    align-items: center;
    gap: 7px;
  }

  .profile-name {
    font-size: 13px;
    font-weight: 600;
    color: var(--text);
  }

  .profile-badge {
    font-size: 9.5px;
    color: var(--accent);
    background: color-mix(in srgb, var(--accent) 20%, transparent);
    border: 1px solid color-mix(in srgb, var(--accent) 30%, transparent);
    padding: 1px 7px;
    border-radius: 999px;
    font-weight: 650;
    letter-spacing: 0.02em;
    text-transform: uppercase;
  }

  .profile-subtext {
    font-size: 11px;
    color: var(--muted);
  }

  .profile-actions {
    display: flex;
    align-items: center;
    gap: 6px;
    flex-shrink: 0;
  }

  .switch-profile-btn {
    padding: 5px 12px;
    border-radius: 999px;
    background: var(--surface-2);
    border: 1px solid var(--border);
    font-size: 11.5px;
    font-weight: 600;
    color: var(--text);
    cursor: pointer;
    transition: all 0.15s ease;
  }

  .switch-profile-btn:hover {
    background: var(--accent);
    border-color: var(--accent);
    color: #ffffff;
  }

  .delete-profile-btn {
    width: 28px;
    height: 28px;
    border-radius: 8px;
    display: flex;
    align-items: center;
    justify-content: center;
    color: var(--muted);
    background: transparent;
    border: none;
    cursor: pointer;
    transition: all 0.15s ease;
  }

  .delete-profile-btn:hover {
    color: #ef4444;
    background: color-mix(in srgb, #ef4444 14%, transparent);
  }

  .action-icon {
    width: 14px;
    height: 14px;
  }

  .create-profile-card {
    padding: 14px;
    border-radius: 12px;
    background: var(--surface);
    border: 1px solid var(--border);
    margin-top: 10px;
  }

  .create-profile-header {
    display: flex;
    flex-direction: column;
    gap: 2px;
    margin-bottom: 10px;
  }

  .create-title {
    font-size: 12px;
    font-weight: 650;
    color: var(--text);
  }

  .create-desc {
    font-size: 10.5px;
    color: var(--muted);
  }

  .create-profile-input-wrap {
    display: flex;
    gap: 8px;
  }

  .create-profile-input {
    flex: 1;
    padding: 7px 11px;
    border-radius: 8px;
    background: var(--panel);
    border: 1px solid var(--border);
    color: var(--text);
    font-size: 12px;
    outline: none;
    transition: border-color 0.15s ease;
  }

  .create-profile-input:focus {
    border-color: var(--accent);
  }

  .create-profile-btn {
    padding: 7px 14px;
    border-radius: 8px;
    background: var(--accent);
    color: #ffffff;
    font-size: 12px;
    font-weight: 600;
    border: none;
    cursor: pointer;
    transition: filter 0.15s ease;
  }

  .create-profile-btn:hover {
    filter: brightness(1.1);
  }


  .danger-btn {
    padding: 4px 8px;
    border-radius: 5px;
    background: color-mix(in srgb, #ef4444 14%, transparent);
    color: #ef4444;
    font-size: 11px;
    font-weight: 600;
    transition: background 0.12s ease;
  }

  .danger-btn:hover {
    background: color-mix(in srgb, #ef4444 24%, transparent);
  }

  .danger-icon-btn {
    padding: 3px 6px;
    border-radius: 4px;
    color: var(--muted);
    font-size: 11px;
  }

  .danger-icon-btn:hover {
    color: #ef4444;
    background: color-mix(in srgb, #ef4444 14%, transparent);
  }

  .input-action-row {
    display: flex;
    gap: 6px;
    margin-top: 4px;
  }

  .text-input {
    flex: 1;
    padding: 5px 8px;
    border: 1px solid var(--border);
    border-radius: 6px;
    background: var(--panel);
    color: var(--text);
    font-size: 12px;
    outline: none;
  }

  .text-input:focus {
    border-color: var(--accent);
  }

  .primary-btn {
    padding: 5px 12px;
    border-radius: 6px;
    background: var(--accent);
    color: white;
    font-size: 12px;
    font-weight: 600;
    transition: opacity 0.15s ease;
  }

  .primary-btn:hover {
    opacity: 0.9;
  }

  /* Extensions */
  .extensions-list {
    display: flex;
    flex-direction: column;
    gap: 8px;
  }

  .ext-card {
    padding: 10px 12px;
  }

  .ext-header {
    display: flex;
    align-items: flex-start;
    justify-content: space-between;
  }

  .ext-title {
    font-size: 12.5px;
    font-weight: 600;
    color: var(--text);
  }

  .ext-version {
    font-size: 10.5px;
    color: var(--muted);
    font-weight: normal;
  }

  .ext-id {
    font-size: 10px;
    color: var(--muted);
    font-family: monospace;
    margin-top: 1px;
  }

  .ext-controls {
    display: flex;
    align-items: center;
    gap: 8px;
  }

  .ext-badges {
    display: flex;
    flex-wrap: wrap;
    gap: 4px;
    margin-top: 7px;
  }

  .badge {
    padding: 1px 6px;
    border-radius: 4px;
    font-size: 9.5px;
    font-weight: 600;
    text-transform: uppercase;
  }

  .badge.verified {
    background: color-mix(in srgb, #10b981 18%, transparent);
    color: #10b981;
  }

  .badge.partial {
    background: color-mix(in srgb, #f59e0b 18%, transparent);
    color: #f59e0b;
  }



  .empty-card {
    text-align: center;
    padding: 20px;
  }

  .empty-icon {
    font-size: 24px;
    margin-bottom: 6px;
  }

  /* Shortcuts */
  .shortcut-list {
    display: flex;
    flex-direction: column;
    gap: 7px;
  }

  .key-row {
    display: flex;
    align-items: center;
    justify-content: space-between;
    font-size: 11.5px;
    color: var(--text-secondary);
  }

  /* Tech Info */
  .tech-row {
    display: flex;
    justify-content: space-between;
    font-size: 11.5px;
    padding: 4px 0;
    border-bottom: 1px solid var(--border-subtle);
  }

  .tech-row:last-child {
    border-bottom: none;
  }

  .tech-row span {
    color: var(--muted);
  }

  /* Settings Footer */
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

  .quit-action-btn {
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
