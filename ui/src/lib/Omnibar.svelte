<script lang="ts">
  import { onMount } from 'svelte';
  import {
    displayUrl,
    isBlank,
    onEvent,
    requestState,
    send,
    type PeekEvent,
    type Suggestion,
  } from './ipc';

  let input = $state('');
  let focused = $state(false);
  let suggestions = $state<Suggestion[]>([]);
  let selected = $state(-1);
  let activeId = $state<number | null>(null);
  let activeUrl = $state('about:blank');
  let insecure = $state(false);
  let loading = $state(false);
  let searchEngine = $state('Google');
  let suggestTimer: ReturnType<typeof setTimeout> | undefined;
  let inputEl = $state<HTMLInputElement | null>(null);

  let isStartPage = $derived(isBlank(activeUrl));

  function openBar() {
    focused = true;
    send({ type: 'setOmnibarOpen', open: true });
    setTimeout(() => {
      inputEl?.focus();
      inputEl?.select();
    }, 10);
  }

  function closeBar(focusPage = true) {
    if (suggestTimer) clearTimeout(suggestTimer);
    suggestions = [];
    selected = -1;
    if (focused) {
      focused = false;
      send({ type: 'setOmnibarOpen', open: false });
    }
    inputEl?.blur();
    if (focusPage && !isStartPage) {
      send({ type: 'focusPage' });
    }
  }

  function submit(raw: string) {
    const value = raw.trim();
    if (!value) return;
    send({ type: 'navigate', input: value });
    closeBar();
  }

  function accept(item: Suggestion) {
    if (item.kind === 'tab' && item.tabId != null) {
      send({ type: 'switchTab', id: item.tabId });
    } else if (item.url) {
      send({ type: 'navigate', input: item.url });
    } else {
      submit(item.text);
    }
    closeBar();
  }

  function label(kind: Suggestion['kind']): string {
    switch (kind) {
      case 'tab':
        return 'Tab';
      case 'history':
        return 'History';
      case 'bookmark':
        return 'Saved';
      default:
        return 'Search';
    }
  }

  function handleEvent(event: PeekEvent) {
    switch (event.type) {
      case 'tabsChanged': {
        const active = event.tabs.find((tab) => tab.active) ?? null;
        activeId = active ? active.id : null;
        if (active) {
          activeUrl = active.url;
          insecure = active.url.startsWith('http://');
          if (!focused) {
            input = isBlank(active.url) ? '' : displayUrl(active.url);
          }
        }
        break;
      }
      case 'loading':
        if (event.id === activeId) {
          loading = event.loading;
          activeUrl = event.url;
          insecure = event.url.startsWith('http://');
          if (!focused) {
            input = isBlank(event.url) ? '' : displayUrl(event.url);
          }
        }
        break;
      case 'urlChanged':
        if (event.id === activeId) {
          activeUrl = event.url;
          insecure = event.url.startsWith('http://');
          if (!focused) {
            input = isBlank(event.url) ? '' : displayUrl(event.url);
          }
        }
        break;
      case 'suggestionsReady':
        if (event.query === input) {
          suggestions = event.items.slice(0, 6);
          selected = -1;
        }
        break;
      case 'focusOmnibar':
        openBar();
        break;
      case 'drawerState':
        if (event.open) closeBar(false);
        break;
      case 'settings':
        searchEngine = event.searchEngine || 'Google';
        break;
      default:
        break;
    }
  }

  function onInput() {
    send({ type: 'setOmnibarOpen', open: true });
    if (suggestTimer) clearTimeout(suggestTimer);
    const query = input;
    if (!query.trim()) {
      suggestions = [];
      return;
    }
    suggestTimer = setTimeout(() => send({ type: 'suggest', query }), 80);
  }

  function onKeydown(event: KeyboardEvent) {
    if ((event.metaKey || event.ctrlKey) && event.key.toLowerCase() === 'l') {
      event.preventDefault();
      openBar();
      return;
    }
    if (event.key === 'Escape') {
      event.preventDefault();
      closeBar();
      return;
    }
    if (event.key === 'ArrowDown') {
      event.preventDefault();
      if (suggestions.length) {
        selected = (selected + 1) % suggestions.length;
      }
      return;
    }
    if (event.key === 'ArrowUp') {
      event.preventDefault();
      if (suggestions.length) {
        selected = (selected - 1 + suggestions.length) % suggestions.length;
      }
      return;
    }
    if (event.key === 'Tab' && suggestions.length) {
      event.preventDefault();
      selected = (selected + 1) % suggestions.length;
      return;
    }
    if (event.key === 'Enter') {
      event.preventDefault();
      const picked = selected >= 0 ? suggestions[selected] : undefined;
      if (picked) accept(picked);
      else submit(input);
    }
  }

  function onRootClick(event: MouseEvent) {
    if (event.target === event.currentTarget && !isStartPage) {
      closeBar();
    }
  }

  onMount(() => {
    onEvent(handleEvent);
    requestState();
    const onWindowKey = (event: KeyboardEvent) => {
      if ((event.metaKey || event.ctrlKey) && event.key.toLowerCase() === 'l') {
        event.preventDefault();
        openBar();
      }
    };
    window.addEventListener('keydown', onWindowKey, true);
    // Autofocus on mount if start page
    if (isStartPage) {
      setTimeout(() => inputEl?.focus(), 50);
    }
    return () => window.removeEventListener('keydown', onWindowKey, true);
  });
</script>

<div
  class="root"
  class:start-page={isStartPage}
  class:open={focused}
  onclick={onRootClick}
  role="presentation"
>
  {#if isStartPage}
    <!-- Center Start Page Layout -->
    <div class="start-center">
      <div class="hero-brand">
        <img src="/logo.png" alt="Peek Logo" class="hero-logo" />
        <h1 class="hero-title">Peek</h1>
        <p class="hero-subtitle">Hover to preview. Edge to explore.</p>
      </div>

      <div class="search-box">
        <div class="search-input-wrap">
          <svg class="search-icon" viewBox="0 0 20 20" fill="none" stroke="currentColor">
            <circle cx="9" cy="9" r="6" stroke-width="1.8" stroke-linecap="round" />
            <path d="M13.5 13.5L17.5 17.5" stroke-width="1.8" stroke-linecap="round" />
          </svg>
          <input
            bind:this={inputEl}
            bind:value={input}
            oninput={onInput}
            onkeydown={onKeydown}
            onfocus={() => {
              focused = true;
              send({ type: 'setOmnibarOpen', open: true });
            }}
            placeholder={`Search with ${searchEngine} or enter URL...`}
            spellcheck="false"
            autocomplete="off"
            autocapitalize="off"
            aria-label="Address and search"
          />
          {#if input}
            <button
              class="clear-btn"
              type="button"
              onclick={() => {
                input = '';
                suggestions = [];
                inputEl?.focus();
              }}
              aria-label="Clear input"
            >
              ×
            </button>
          {/if}
        </div>

        {#if suggestions.length}
          <div class="start-suggestions" role="listbox">
            {#each suggestions as item, index (item.kind + ':' + (item.url ?? item.text))}
              <button
                class="suggest-item"
                class:selected={index === selected}
                role="option"
                aria-selected={index === selected}
                onmousedown={(event) => {
                  event.preventDefault();
                  accept(item);
                }}
              >
                <span class="badge">{label(item.kind)}</span>
                <span class="text">{item.text}</span>
                {#if item.detail}
                  <span class="detail">{item.detail}</span>
                {/if}
              </button>
            {/each}
          </div>
        {/if}
      </div>

      <div class="shortcuts-row">
        <span class="shortcut-pill"><kbd>⌘ L</kbd> Focus search</span>
        <span class="shortcut-pill"><kbd>⌘ + Hover</kbd> Live preview</span>
        <span class="shortcut-pill"><kbd>⌘ ⇧ [</kbd> Tabs drawer</span>
        <span class="shortcut-pill"><kbd>⌘ ⇧ ]</kbd> Settings</span>
      </div>
    </div>
  {:else}
    <!-- Docked Top Bar Layout -->
    <div class="bar">
      <button
        class="logo-btn"
        title="Open Tabs (Cmd+Shift+[)"
        type="button"
        onclick={(e) => {
          e.stopPropagation();
          send({ type: 'setDrawer', open: true, side: 'left', section: 'tabs' });
        }}
      >
        <img src="/logo.png" alt="Peek" class="bar-logo" />
      </button>

      {#if insecure}
        <span class="lock" title="This page is not secure" aria-label="Not secure">!</span>
      {/if}

      <input
        bind:this={inputEl}
        bind:value={input}
        oninput={onInput}
        onkeydown={onKeydown}
        onfocus={() => {
          focused = true;
          send({ type: 'setOmnibarOpen', open: true });
        }}
        placeholder="Search or enter address"
        spellcheck="false"
        autocomplete="off"
        autocapitalize="off"
        aria-label="Address and search"
      />

      {#if loading}
        <span class="spinner" aria-hidden="true"></span>
      {/if}

      <button
        class="settings-btn"
        title="Settings (Cmd+Shift+])"
        type="button"
        onclick={(e) => {
          e.stopPropagation();
          send({ type: 'setDrawer', open: true, side: 'right', section: 'settings' });
        }}
      >
        <svg viewBox="0 0 20 20" fill="none" stroke="currentColor" class="gear-icon">
          <circle cx="10" cy="10" r="3" stroke-width="1.6" />
          <path
            d="M10 2.5V4.5M10 15.5V17.5M2.5 10H4.5M15.5 10H17.5M4.7 4.7L6.1 6.1M13.9 13.9L15.3 15.3M4.7 15.3L6.1 13.9M13.9 6.1L15.3 4.7"
            stroke-width="1.6"
            stroke-linecap="round"
          />
        </svg>
      </button>
    </div>

    {#if focused && suggestions.length}
      <div class="panel" role="listbox">
        {#each suggestions as item, index (item.kind + ':' + (item.url ?? item.text))}
          <button
            class="item"
            class:selected={index === selected}
            role="option"
            aria-selected={index === selected}
            onmousedown={(event) => {
              event.preventDefault();
              accept(item);
            }}
          >
            <span class="badge">{label(item.kind)}</span>
            <span class="text">{item.text}</span>
            {#if item.detail}
              <span class="detail">{item.detail}</span>
            {/if}
          </button>
        {/each}
      </div>
    {/if}
  {/if}
</div>

<style>
  .root {
    position: fixed;
    inset: 0;
    background: transparent;
    transition: background 0.22s ease-out;
  }

  .root.start-page {
    background: var(--bg);
    display: flex;
    align-items: center;
    justify-content: center;
  }

  /* Centered Start Page */
  .start-center {
    display: flex;
    flex-direction: column;
    align-items: center;
    width: 100%;
    max-width: 580px;
    padding: 0 20px;
    margin-top: -6vh;
    animation: fadeIn 0.24s cubic-bezier(0.2, 0, 0, 1);
  }

  @keyframes fadeIn {
    from {
      opacity: 0;
      transform: translateY(12px) scale(0.98);
    }
    to {
      opacity: 1;
      transform: translateY(0) scale(1);
    }
  }

  .hero-brand {
    display: flex;
    flex-direction: column;
    align-items: center;
    margin-bottom: 24px;
  }

  .hero-logo {
    width: 60px;
    height: 60px;
    border-radius: 14px;
    box-shadow: 0 10px 24px rgba(0, 0, 0, 0.4);
    margin-bottom: 12px;
    transition: transform 0.25s ease;
  }

  .hero-logo:hover {
    transform: scale(1.05) rotate(1deg);
  }

  .hero-title {
    margin: 0;
    font-size: 26px;
    font-weight: 700;
    letter-spacing: -0.02em;
    color: var(--text);
  }

  .hero-subtitle {
    margin: 4px 0 0;
    font-size: 13px;
    color: var(--muted);
    letter-spacing: 0.01em;
  }

  .search-box {
    width: 100%;
    position: relative;
    border-radius: 14px;
    box-shadow: 0 12px 36px rgba(0, 0, 0, 0.22);
    background: var(--panel);
    border: 1px solid var(--border);
    transition: border-color 0.15s ease, box-shadow 0.15s ease;
  }

  .search-box:focus-within {
    border-color: var(--accent);
    box-shadow: 0 14px 40px rgba(0, 0, 0, 0.28), 0 0 0 2px var(--accent-soft);
  }

  .search-input-wrap {
    display: flex;
    align-items: center;
    height: 48px;
    padding: 0 16px;
    gap: 12px;
  }

  .search-icon {
    width: 18px;
    height: 18px;
    color: var(--muted);
    flex-shrink: 0;
  }

  .search-input-wrap input {
    flex: 1;
    min-width: 0;
    height: 100%;
    border: none;
    outline: none;
    background: transparent;
    color: var(--text);
    font-size: 15px;
    letter-spacing: 0.01em;
  }

  .search-input-wrap input::placeholder {
    color: var(--muted);
  }

  .clear-btn {
    font-size: 18px;
    color: var(--muted);
    padding: 4px 8px;
    border-radius: 6px;
  }

  .clear-btn:hover {
    color: var(--text);
    background: var(--hover);
  }

  .start-suggestions {
    border-top: 1px solid var(--border);
    padding: 6px;
    max-height: 260px;
    overflow-y: auto;
  }

  .suggest-item {
    display: flex;
    align-items: center;
    gap: 10px;
    width: 100%;
    padding: 9px 12px;
    border-radius: 8px;
    text-align: left;
    font-size: 13.5px;
    cursor: pointer;
  }

  .suggest-item.selected,
  .suggest-item:hover {
    background: var(--hover);
  }

  .shortcuts-row {
    display: flex;
    align-items: center;
    justify-content: center;
    gap: 12px;
    margin-top: 22px;
    flex-wrap: wrap;
  }

  .shortcut-pill {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    font-size: 11.5px;
    color: var(--muted);
    padding: 4px 8px;
    border-radius: 6px;
    background: color-mix(in srgb, var(--surface) 60%, transparent);
    border: 1px solid color-mix(in srgb, var(--border) 60%, transparent);
  }

  kbd {
    display: inline-block;
    padding: 1px 5px;
    border-radius: 4px;
    background: var(--panel);
    border: 1px solid var(--border);
    font-family: inherit;
    font-size: 10.5px;
    font-weight: 600;
    color: var(--text);
    box-shadow: 0 1px 1px rgba(0, 0, 0, 0.08);
  }

  /* Docked Top Bar */
  .bar {
    display: flex;
    align-items: center;
    gap: 8px;
    height: 30px;
    margin: 3px 10px;
    padding: 0 11px;
    border: 1px solid var(--border);
    border-radius: 9px;
    background: var(--panel);
    box-shadow: 0 1px 3px rgba(0, 0, 0, 0.08);
    transition: transform 0.22s cubic-bezier(0.2, 0, 0, 1), border-color 0.15s ease;
    animation: dockSlide 0.22s cubic-bezier(0.2, 0, 0, 1);
  }

  @keyframes dockSlide {
    from {
      opacity: 0.6;
      transform: translateY(-8px);
    }
    to {
      opacity: 1;
      transform: translateY(0);
    }
  }

  .bar:focus-within {
    border-color: color-mix(in srgb, var(--accent) 60%, var(--border));
  }

  .logo-btn {
    all: unset;
    display: flex;
    align-items: center;
    justify-content: center;
    cursor: pointer;
    flex-shrink: 0;
    padding: 2px;
    margin-left: -3px;
    border-radius: 4px;
    transition: opacity 0.15s ease, transform 0.15s ease;
  }

  .logo-btn:hover {
    opacity: 0.85;
    transform: scale(1.08);
  }

  .bar-logo {
    width: 17px;
    height: 17px;
    border-radius: 4px;
    display: block;
    user-select: none;
    box-shadow: 0 1px 3px rgba(0, 0, 0, 0.35);
  }

  .bar input {
    flex: 1;
    min-width: 0;
    height: 100%;
    border: none;
    outline: none;
    background: transparent;
    color: var(--text);
    font-size: 13px;
    letter-spacing: 0.01em;
  }

  .bar input::placeholder {
    color: var(--muted);
  }

  .settings-btn {
    all: unset;
    display: flex;
    align-items: center;
    justify-content: center;
    cursor: pointer;
    flex-shrink: 0;
    padding: 2px;
    color: var(--muted);
    border-radius: 4px;
    transition: color 0.15s ease;
  }

  .settings-btn:hover {
    color: var(--text);
  }

  .gear-icon {
    width: 14px;
    height: 14px;
  }

  .lock {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    width: 15px;
    height: 15px;
    border-radius: 50%;
    background: color-mix(in srgb, #d97706 22%, transparent);
    color: #b45309;
    font-size: 10px;
    font-weight: 700;
  }

  .spinner {
    width: 12px;
    height: 12px;
    border: 1.5px solid var(--border);
    border-top-color: var(--accent);
    border-radius: 50%;
    animation: spin 0.7s linear infinite;
  }

  @keyframes spin {
    to {
      transform: rotate(360deg);
    }
  }

  .panel {
    max-height: 280px;
    margin: 6px 10px 0;
    padding: 4px;
    overflow-y: auto;
    border: 1px solid var(--border);
    border-radius: var(--radius);
    background: var(--panel);
    box-shadow: var(--shadow);
    animation: panelPop 0.15s cubic-bezier(0.2, 0, 0, 1);
  }

  @keyframes panelPop {
    from {
      opacity: 0;
      transform: translateY(-4px);
    }
    to {
      opacity: 1;
      transform: translateY(0);
    }
  }

  .item {
    display: flex;
    align-items: center;
    gap: 10px;
    width: 100%;
    padding: 7px 9px;
    border-radius: 7px;
    text-align: left;
    font-size: 12.5px;
  }

  .item.selected,
  .item:hover {
    background: var(--hover);
  }

  .badge {
    flex: 0 0 54px;
    color: var(--muted);
    font-size: 9.5px;
    font-weight: 600;
    letter-spacing: 0.06em;
    text-transform: uppercase;
  }

  .text {
    flex: 1 1 auto;
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .detail {
    flex: 0 1 38%;
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    color: var(--muted);
    font-size: 11px;
  }
</style>
