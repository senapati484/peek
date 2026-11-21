<script lang="ts">
  import { onMount } from 'svelte';
  import { onEvent, requestState, send, type PeekEvent, type TabInfo } from './ipc';

  type Section = 'tabs' | 'settings' | 'blocking' | 'profiles' | 'extensions';

  let section = $state<Section>('tabs');
  let tabs = $state<TabInfo[]>([]);
  let active = $state<number | null>(null);
  let blocked = $state(0);
  let allowedOnSite = $state(false);
  let host = $state<string | null>(null);

  let maxLive = $state(5);
  let idleMinutes = $state(10);
  let hoverDelay = $state(450);
  let drawerSide = $state<'left' | 'right'>('right');
  let edgeZone = $state(4);
  let startBlank = $state(false);
  let searchEngine = $state('DuckDuckGo');

  let dragId = $state<number | null>(null);
  let closeTimer: ReturnType<typeof setTimeout> | undefined;

  function close(): void {
    send({ type: 'setDrawer', open: false });
  }

  function scheduleClose(): void {
    cancelClose();
    closeTimer = setTimeout(close, 220);
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

  function handleEvent(event: PeekEvent): void {
    switch (event.type) {
      case 'tabsChanged':
        tabs = event.tabs;
        active = event.active;
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
        drawerSide = event.drawerSide;
        edgeZone = event.edgeZone;
        startBlank = event.startBlank;
        searchEngine = event.searchEngine;
        break;
      case 'drawerState':
        if (!event.open) send({ type: 'setDrawer', open: false });
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
    onEvent(handleEvent);
    requestState();
    const onKey = (event: KeyboardEvent) => {
      if (event.key === 'Escape') close();
    };
    window.addEventListener('keydown', onKey, true);
    return () => window.removeEventListener('keydown', onKey, true);
  });
</script>

<aside class="drawer" onmouseenter={cancelClose} onmouseleave={scheduleClose} aria-label="Peek drawer">
  <header>
    <div class="brand">
      <img src="/logo.png" alt="Peek" class="brand-logo" />
      <span>Peek</span>
    </div>
    <nav>

      <button class:on={section === 'tabs'} onclick={() => (section = 'tabs')}>Tabs</button>
      <button class:on={section === 'blocking'} onclick={() => (section = 'blocking')}>Blocking</button>
      <button class:on={section === 'settings'} onclick={() => (section = 'settings')}>Settings</button>
      <button class:on={section === 'profiles'} onclick={() => (section = 'profiles')}>Profiles</button>
      <button class:on={section === 'extensions'} onclick={() => (section = 'extensions')}>Ext</button>
    </nav>
  </header>

  <div class="body">
    {#if section === 'tabs'}
      <button class="new" onclick={() => send({ type: 'newTab' })}>
        <span class="plus">+</span> New tab
      </button>
      <ul class="tabs">
        {#each tabs as tab, index (tab.id)}
          <li
            class:active={tab.active}
            draggable="true"
            ondragstart={(event) => onDragStart(event, tab.id)}
            ondragover={(event) => event.preventDefault()}
            ondrop={(event) => onDrop(event, index)}
          >
            <button class="tab" onclick={() => switchTo(tab.id)}>
              {#if tab.favicon}
                <img class="favicon" src={tab.favicon} alt="" />
              {:else}
                <span class="favicon dot"></span>
              {/if}
              <span class="title">{tab.title || tab.url}</span>
              {#if tab.audible}<span class="pill">audio</span>{/if}
              {#if tab.discarded}<span class="pill">sleeping</span>{/if}
            </button>
            <button class="x" aria-label="Close tab" onclick={() => send({ type: 'closeTab', id: tab.id })}>
              ×
            </button>
          </li>
        {:else}
          <li class="empty">No tabs</li>
        {/each}
      </ul>
    {:else if section === 'blocking'}
      <div class="card">
        <div class="big">{blocked}</div>
        <div class="muted">requests blocked in this tab</div>
        {#if host}
          <label class="toggle">
            <input
              type="checkbox"
              checked={!allowedOnSite}
              onchange={() => send({ type: 'setSiteAdblock', host: host!, enabled: allowedOnSite })}
            />
            Block ads on {host}
          </label>
        {/if}
        <p class="note">Per-site rules are saved per profile. The filter engine arrives in Phase 2.</p>
      </div>
    {:else if section === 'settings'}
      <div class="field">
        <label for="max-live">Live tabs</label>
        <input
          id="max-live"
          type="number"
          min="1"
          max="20"
          value={maxLive}
          onchange={(event) => setting('maxLive', Number((event.currentTarget as HTMLInputElement).value))}
        />
      </div>
      <div class="field">
        <label for="idle">Sleep after (minutes)</label>
        <input
          id="idle"
          type="number"
          min="1"
          max="240"
          value={idleMinutes}
          onchange={(event) =>
            setting('idleDiscardMinutes', Number((event.currentTarget as HTMLInputElement).value))}
        />
      </div>
      <div class="field">
        <label for="hover">Hover delay (ms)</label>
        <input
          id="hover"
          type="number"
          min="0"
          max="3000"
          step="50"
          value={hoverDelay}
          onchange={(event) =>
            setting('hoverDelayMs', Number((event.currentTarget as HTMLInputElement).value))}
        />
      </div>
      <div class="field">
        <label for="side">Drawer side</label>
        <select
          id="side"
          value={drawerSide}
          onchange={(event) =>
            setting('drawerSide', (event.currentTarget as HTMLSelectElement).value)}
        >
          <option value="right">Right</option>
          <option value="left">Left</option>
        </select>
      </div>
      <div class="field">
        <label for="edge">Edge hot-zone (px)</label>
        <input
          id="edge"
          type="number"
          min="1"
          max="24"
          value={edgeZone}
          onchange={(event) => setting('edgeZone', Number((event.currentTarget as HTMLInputElement).value))}
        />
      </div>
      <label class="toggle">
        <input
          type="checkbox"
          checked={startBlank}
          onchange={(event) =>
            setting('startBlank', (event.currentTarget as HTMLInputElement).checked)}
        />
        Start with a blank page
      </label>
      <div class="field readonly">
        <span class="key">Search</span>
        <span>{searchEngine}</span>
      </div>
    {:else if section === 'profiles'}
      <div class="card">
        <div class="card-title">Default</div>
        <p class="note">Separate profiles, cookies and histories arrive in Phase 4. The data layout already reserves one directory per profile.</p>
      </div>
    {:else if section === 'extensions'}
      <div class="card">
        <div class="card-title">No extensions</div>
        <p class="note">The Manifest V3 host and importer arrive in Phase 5. Unsupported APIs will be listed in a compatibility report.</p>
      </div>
    {/if}
  </div>

  <footer>
    <button class="quit" onclick={() => send({ type: 'quit' })}>Quit Peek</button>
  </footer>
</aside>

<style>
  .drawer {
    display: flex;
    flex-direction: column;
    height: 100vh;
    background: var(--panel);
    border-inline-start: 1px solid var(--border);
    box-shadow: var(--shadow);
  }

  header {
    padding: 14px 14px 8px;
    border-bottom: 1px solid var(--border);
  }

  .brand {
    display: flex;
    align-items: center;
    gap: 8px;
    font-size: 13.5px;
    font-weight: 650;
    letter-spacing: 0.01em;
    margin-bottom: 10px;
  }

  .brand-logo {
    width: 20px;
    height: 20px;
    border-radius: 5px;
    box-shadow: 0 1px 3px rgba(0, 0, 0, 0.4);
    display: block;
    user-select: none;
  }


  nav {
    display: flex;
    gap: 2px;
    flex-wrap: wrap;
  }

  nav button {
    padding: 5px 9px;
    border-radius: 7px;
    color: var(--muted);
    font-size: 11.5px;
  }

  nav button:hover {
    background: var(--hover);
    color: var(--text);
  }

  nav button.on {
    background: var(--hover);
    color: var(--text);
    font-weight: 600;
  }

  .body {
    flex: 1;
    overflow-y: auto;
    padding: 12px;
  }

  .new {
    display: flex;
    align-items: center;
    gap: 8px;
    width: 100%;
    padding: 8px 10px;
    margin-bottom: 8px;
    border: 1px dashed var(--border);
    border-radius: 9px;
    color: var(--muted);
    font-size: 12.5px;
  }

  .new:hover {
    color: var(--text);
    border-color: var(--accent);
  }

  .plus {
    font-size: 15px;
    line-height: 1;
  }

  .tabs {
    list-style: none;
    margin: 0;
    padding: 0;
    display: flex;
    flex-direction: column;
    gap: 2px;
  }

  .tabs li {
    display: flex;
    align-items: center;
    border-radius: 8px;
  }

  .tabs li:hover {
    background: var(--hover);
  }

  .tabs li.active {
    background: color-mix(in srgb, var(--accent) 12%, transparent);
  }

  .tab {
    flex: 1;
    display: flex;
    align-items: center;
    gap: 8px;
    min-width: 0;
    padding: 7px 8px;
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
    flex: 0 0 14px;
    width: 14px;
    height: 14px;
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
    background: var(--hover);
    color: var(--muted);
    font-size: 9.5px;
    letter-spacing: 0.04em;
    text-transform: uppercase;
  }

  .x {
    flex: 0 0 24px;
    height: 24px;
    margin-right: 4px;
    border-radius: 6px;
    color: var(--muted);
    font-size: 15px;
    line-height: 1;
  }

  .x:hover {
    background: color-mix(in srgb, #ef4444 18%, transparent);
    color: #ef4444;
  }

  .empty {
    padding: 12px 8px;
    color: var(--muted);
    font-size: 12px;
  }

  .card {
    padding: 14px;
    border: 1px solid var(--border);
    border-radius: var(--radius);
    background: color-mix(in srgb, var(--hover) 45%, transparent);
  }

  .card-title {
    font-size: 12.5px;
    font-weight: 600;
    margin-bottom: 4px;
  }

  .big {
    font-size: 34px;
    font-weight: 700;
    letter-spacing: -0.02em;
    line-height: 1.1;
  }

  .muted {
    color: var(--muted);
    font-size: 11.5px;
  }

  .note {
    margin: 10px 0 0;
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
    font-size: 12.5px;
  }

  .field label,
  .field .key {
    color: var(--muted);
  }

  .field input,
  .field select {
    width: 92px;
    padding: 4px 6px;
    border: 1px solid var(--border);
    border-radius: 6px;
    background: var(--panel);
    color: var(--text);
    font-size: 12px;
    text-align: right;
  }

  .field.readonly span {
    color: var(--muted);
  }

  .toggle {
    display: flex;
    align-items: center;
    gap: 8px;
    margin-top: 10px;
    font-size: 12px;
  }

  .toggle input {
    accent-color: var(--accent);
  }

  footer {
    padding: 10px 12px 12px;
    border-top: 1px solid var(--border);
  }

  .quit {
    width: 100%;
    padding: 7px;
    border-radius: 8px;
    color: var(--muted);
    font-size: 12px;
  }

  .quit:hover {
    background: var(--hover);
    color: var(--text);
  }
</style>
