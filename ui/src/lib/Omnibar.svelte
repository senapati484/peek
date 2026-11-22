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
  let activeProfile = $state('default');
  let profileInitial = $derived((activeProfile[0] || 'D').toUpperCase());
  let suggestTimer: ReturnType<typeof setTimeout> | undefined;
  let inputEl = $state<HTMLInputElement | null>(null);

  let isStartPage = $derived(isBlank(activeUrl));

  function openBar() {
    focused = true;
    send({ type: 'setOmnibarOpen', open: true });
    if (!isStartPage && activeUrl) {
      input = activeUrl;
    }
    setTimeout(() => {
      inputEl?.focus();
      inputEl?.select();
    }, 15);
  }

  function closeBar(focusPage = true) {
    if (suggestTimer) clearTimeout(suggestTimer);
    suggestions = [];
    selected = -1;
    if (focused) {
      focused = false;
      send({ type: 'setOmnibarOpen', open: false });
    }
    input = isBlank(activeUrl) ? '' : displayUrl(activeUrl);
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
      case 'profilesList':
        activeProfile = event.active || 'default';
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
    if (isStartPage) {
      setTimeout(() => inputEl?.focus(), 60);
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
    <!-- Center Start Page Layout: Logo + Search Bar Simple -->
    <div class="start-center">
      <!-- Profile Badge in Top Corner -->
      <div class="start-profile-corner">
        <button
          class="start-profile-chip"
          title={`Active Profile: ${activeProfile} (Click to switch)`}
          type="button"
          onclick={(e) => {
            e.stopPropagation();
            send({ type: 'setDrawer', open: true, side: 'right', section: 'profiles' });
          }}
        >
          <span class="profile-avatar-dot">{profileInitial}</span>
          <span class="profile-chip-name">{activeProfile}</span>
        </button>
      </div>

      <div class="hero-brand">
        <div class="logo-wrapper">
          <img src="/logo.png" alt="Peek Logo" class="hero-logo" />
        </div>
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
                <span class="badge {item.kind}">{label(item.kind)}</span>
                <span class="text">{item.text}</span>
                {#if item.detail}
                  <span class="detail">{item.detail}</span>
                {/if}
              </button>
            {/each}
          </div>
        {/if}
      </div>
    </div>
  {:else}
    <!-- Docked Minimalist Top Bar -->
    <div class="bar-container">
      <div class="bar">
        <!-- Logo / Tabs Drawer Trigger -->
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

        <!-- Navigation Chevrons -->
        <button
          class="nav-btn"
          title="Back (Alt+Left)"
          type="button"
          onclick={(e) => {
            e.stopPropagation();
            send({ type: 'back' });
          }}
        >
          <svg viewBox="0 0 16 16" fill="none" stroke="currentColor" class="nav-icon">
            <path d="M10 3.5L5.5 8L10 12.5" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round" />
          </svg>
        </button>
        <button
          class="nav-btn"
          title="Forward (Alt+Right)"
          type="button"
          onclick={(e) => {
            e.stopPropagation();
            send({ type: 'forward' });
          }}
        >
          <svg viewBox="0 0 16 16" fill="none" stroke="currentColor" class="nav-icon">
            <path d="M6 3.5L10.5 8L6 12.5" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round" />
          </svg>
        </button>
        <button
          class="nav-btn reload-btn"
          title={loading ? "Stop Loading" : "Reload (Cmd+R)"}
          type="button"
          onclick={(e) => {
            e.stopPropagation();
            send({ type: loading ? 'stop' : 'reload' });
          }}
        >
          {#if loading}
            <span class="spinner" aria-hidden="true"></span>
          {:else}
            <svg viewBox="0 0 16 16" fill="none" stroke="currentColor" class="nav-icon">
              <path d="M13.5 8A5.5 5.5 0 1 1 12 4.1L13.5 2.5V6.5H9.5" stroke-width="1.6" stroke-linecap="round" stroke-linejoin="round" />
            </svg>
          {/if}
        </button>

        <!-- Security Lock Indicator -->
        {#if insecure}
          <span class="lock-pill insecure" title="Connection is not secure (HTTP)" aria-label="Not secure">
            <svg viewBox="0 0 16 16" fill="none" stroke="currentColor" class="lock-icon">
              <path d="M8 3.5v5M8 11.5v.5" stroke-width="1.8" stroke-linecap="round" />
              <circle cx="8" cy="8" r="6.5" stroke-width="1.4" />
            </svg>
          </span>
        {:else}
          <span class="lock-pill secure" title="Connection is secure (HTTPS)" aria-label="Secure">
            <svg viewBox="0 0 16 16" fill="none" stroke="currentColor" class="lock-icon">
              <path d="M4.5 7V4.5a3.5 3.5 0 0 1 7 0V7M3.5 7h9a1.5 1.5 0 0 1 1.5 1.5v5a1.5 1.5 0 0 1-1.5 1.5h-9A1.5 1.5 0 0 1 2 13.5v-5A1.5 1.5 0 0 1 3.5 7z" stroke-width="1.4" />
            </svg>
          </span>
        {/if}

        <!-- Main Input Field -->
        <input
          bind:this={inputEl}
          bind:value={input}
          oninput={onInput}
          onkeydown={onKeydown}
          onfocus={() => {
            focused = true;
            send({ type: 'setOmnibarOpen', open: true });
            if (activeUrl && !isBlank(activeUrl)) {
              input = activeUrl;
            }
          }}
          placeholder="Search or enter address"
          spellcheck="false"
          autocomplete="off"
          autocapitalize="off"
          aria-label="Address and search"
        />

        {#if input && focused}
          <button
            class="bar-clear-btn"
            type="button"
            onclick={() => {
              input = '';
              suggestions = [];
              inputEl?.focus();
            }}
            title="Clear"
          >
            ×
          </button>
        {/if}

        <!-- Shield / Privacy Button -->
        <button
          class="tool-btn shield-btn"
          title="Adblock & Privacy Filters"
          type="button"
          onclick={(e) => {
            e.stopPropagation();
            send({ type: 'setDrawer', open: true, side: 'right', section: 'blocking' });
          }}
        >
          <svg viewBox="0 0 16 16" fill="none" stroke="currentColor" class="tool-icon">
            <path d="M8 1.5L2.5 3.8v4.2c0 3.7 2.3 6.3 5.5 7 3.2-.7 5.5-3.3 5.5-7V3.8L8 1.5z" stroke-width="1.5" stroke-linejoin="round" />
          </svg>
        </button>

        <!-- Profile Button -->
        <button
          class="tool-btn profile-btn"
          title={`Profile: ${activeProfile} (Click to manage)`}
          type="button"
          onclick={(e) => {
            e.stopPropagation();
            send({ type: 'setDrawer', open: true, side: 'right', section: 'profiles' });
          }}
        >
          <span class="profile-avatar-pill">{profileInitial}</span>
        </button>

        <!-- Settings Button -->
        <button
          class="tool-btn settings-btn"
          title="Settings (Cmd+Shift+])"
          type="button"
          onclick={(e) => {
            e.stopPropagation();
            send({ type: 'setDrawer', open: true, side: 'right', section: 'settings' });
          }}
        >
          <svg viewBox="0 0 20 20" fill="none" stroke="currentColor" class="tool-icon gear-icon">
            <circle cx="10" cy="10" r="3" stroke-width="1.6" />
            <path
              d="M10 2.5V4.5M10 15.5V17.5M2.5 10H4.5M15.5 10H17.5M4.7 4.7L6.1 6.1M13.9 13.9L15.3 15.3M4.7 15.3L6.1 13.9M13.9 6.1L15.3 4.7"
              stroke-width="1.6"
              stroke-linecap="round"
            />
          </svg>
        </button>
      </div>

      <!-- Docked Suggestions Dropdown -->
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
              <span class="badge {item.kind}">{label(item.kind)}</span>
              <span class="text">{item.text}</span>
              {#if item.detail}
                <span class="detail">{item.detail}</span>
              {/if}
            </button>
          {/each}
        </div>
      {/if}
    </div>
  {/if}
</div>

<style>
  .root {
    width: 100vw;
    height: 100vh;
    box-sizing: border-box;
    pointer-events: none;
  }

  .root.start-page,
  .root.open {
    pointer-events: auto;
  }

  /* Center Start Page Layout */
  .start-center {
    position: relative;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    min-height: 100vh;
    padding: 32px 24px;
    background: #ffffff;
    animation: fadeIn 0.28s cubic-bezier(0.16, 1, 0.3, 1);
  }

  .start-profile-corner {
    position: absolute;
    top: 18px;
    right: 24px;
    display: flex;
    align-items: center;
    z-index: 10;
  }

  .start-profile-chip {
    all: unset;
    display: inline-flex;
    align-items: center;
    gap: 7px;
    padding: 4px 10px 4px 6px;
    border-radius: 9999px;
    background: #f1f5f9;
    border: 1px solid #e2e8f0;
    color: #334155;
    font-size: 12px;
    font-weight: 500;
    cursor: pointer;
    box-shadow: 0 1px 2px rgba(0, 0, 0, 0.04);
    transition: all 0.15s ease;
  }

  .start-profile-chip:hover {
    background: #e2e8f0;
    color: #0f172a;
    box-shadow: 0 2px 4px rgba(0, 0, 0, 0.08);
  }

  .profile-avatar-dot {
    display: flex;
    align-items: center;
    justify-content: center;
    width: 20px;
    height: 20px;
    border-radius: 50%;
    background: #3b82f6;
    color: #ffffff;
    font-size: 10.5px;
    font-weight: 700;
  }

  .profile-chip-name {
    letter-spacing: -0.01em;
  }

  @keyframes fadeIn {
    from {
      opacity: 0;
      transform: translateY(10px) scale(0.99);
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

  .logo-wrapper {
    position: relative;
    margin-bottom: 0;
  }

  .logo-wrapper::after {
    display: none;
  }

  .hero-logo {
    position: relative;
    width: 76px;
    height: 76px;
    border-radius: 18px;
    box-shadow: 0 12px 32px rgba(0, 0, 0, 0.15);
    z-index: 1;
    transition: transform 0.22s cubic-bezier(0.16, 1, 0.3, 1);
  }

  .hero-logo:hover {
    transform: scale(1.05);
  }

  .search-box {
    width: 100%;
    max-width: 580px;
    position: relative;
    border-radius: var(--radius-lg);
    background: #ffffff;
    border: 1.5px solid #e2e8f0;
    box-shadow: 0 6px 24px rgba(0, 0, 0, 0.06);
    transition: border-color 0.15s ease, box-shadow 0.15s ease;
  }

  .search-box:focus-within {
    border-color: #3b82f6;
    box-shadow: 0 8px 30px rgba(59, 130, 246, 0.15), 0 0 0 3px rgba(59, 130, 246, 0.12);
  }

  .search-input-wrap {
    display: flex;
    align-items: center;
    height: 50px;
    padding: 0 16px;
    gap: 12px;
  }

  .search-icon {
    width: 19px;
    height: 19px;
    color: var(--muted);
    flex-shrink: 0;
  }

  .search-input-wrap input {
    flex: 1;
    border: none;
    outline: none;
    background: transparent;
    color: var(--text);
    font-size: 14.5px;
    letter-spacing: 0.01em;
  }

  .search-input-wrap input::placeholder {
    color: var(--muted);
  }

  .clear-btn {
    all: unset;
    display: flex;
    align-items: center;
    justify-content: center;
    width: 24px;
    height: 24px;
    border-radius: 50%;
    color: var(--muted);
    font-size: 18px;
    cursor: pointer;
    transition: background 0.15s ease, color 0.15s ease;
  }

  .clear-btn:hover {
    background: var(--hover);
    color: var(--text);
  }

  .start-suggestions {
    border-top: 1px solid var(--border);
    padding: 6px;
    display: flex;
    flex-direction: column;
    gap: 2px;
  }

  .suggest-item {
    display: flex;
    align-items: center;
    gap: 12px;
    width: 100%;
    padding: 9px 12px;
    border-radius: 8px;
    text-align: left;
    font-size: 13.5px;
    cursor: pointer;
    transition: background 0.12s ease;
  }

  .suggest-item.selected,
  .suggest-item:hover {
    background: var(--hover);
  }

  /* Docked Top Bar: Clean Full-White Toolbar */
  .bar-container {
    width: 100%;
    height: 36px;
    background: #ffffff;
    border-bottom: 1px solid #e5e7eb;
    display: flex;
    align-items: center;
    padding: 0 8px;
    box-sizing: border-box;
    pointer-events: auto;
  }

  .bar {
    display: flex;
    align-items: center;
    gap: 6px;
    height: 28px;
    width: 100%;
    margin: 0;
    padding: 0 8px;
    border: 1px solid #e5e7eb;
    border-radius: 7px;
    background: #f9fafb;
    box-shadow: 0 1px 2px rgba(0, 0, 0, 0.04);
    transition: all 0.15s ease;
  }

  .bar:focus-within {
    background: #ffffff;
    border-color: #3b82f6;
    box-shadow: 0 0 0 2px rgba(59, 130, 246, 0.15);
  }

  .logo-btn {
    all: unset;
    display: flex;
    align-items: center;
    justify-content: center;
    cursor: pointer;
    flex-shrink: 0;
    padding: 2px;
    margin-right: 2px;
    border-radius: 4px;
    transition: transform 0.15s ease, opacity 0.15s ease;
  }

  .logo-btn:hover {
    transform: scale(1.08);
    opacity: 0.9;
  }

  .bar-logo {
    width: 16px;
    height: 16px;
    border-radius: 4px;
    display: block;
    user-select: none;
    box-shadow: 0 1px 2px rgba(0, 0, 0, 0.35);
  }

  .nav-btn {
    all: unset;
    display: flex;
    align-items: center;
    justify-content: center;
    width: 20px;
    height: 20px;
    border-radius: 4px;
    color: var(--muted);
    cursor: pointer;
    flex-shrink: 0;
    transition: background 0.12s ease, color 0.12s ease;
  }

  .nav-btn:hover {
    background: var(--hover);
    color: var(--text);
  }

  .nav-icon {
    width: 13px;
    height: 13px;
  }

  .reload-btn {
    margin-right: 2px;
  }

  .lock-pill {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    width: 16px;
    height: 16px;
    border-radius: 4px;
    flex-shrink: 0;
    color: var(--muted);
  }

  .lock-pill.secure {
    color: #10b981;
  }

  .lock-pill.insecure {
    color: #f59e0b;
  }

  .lock-icon {
    width: 12px;
    height: 12px;
  }

  .bar input {
    flex: 1;
    min-width: 0;
    height: 100%;
    border: none;
    outline: none;
    background: transparent;
    color: var(--text);
    font-size: 12.5px;
    letter-spacing: 0.01em;
    padding: 0 4px;
  }

  .bar input::placeholder {
    color: var(--muted);
  }

  .bar-clear-btn {
    all: unset;
    display: flex;
    align-items: center;
    justify-content: center;
    width: 16px;
    height: 16px;
    border-radius: 50%;
    color: var(--muted);
    font-size: 13px;
    cursor: pointer;
    flex-shrink: 0;
    margin-right: 4px;
  }

  .bar-clear-btn:hover {
    background: var(--hover);
    color: var(--text);
  }

  .tool-btn {
    all: unset;
    display: flex;
    align-items: center;
    justify-content: center;
    width: 20px;
    height: 20px;
    border-radius: 4px;
    color: var(--muted);
    cursor: pointer;
    flex-shrink: 0;
    transition: background 0.12s ease, color 0.12s ease;
  }

  .tool-btn:hover {
    background: var(--hover);
    color: var(--text);
  }

  .profile-btn {
    all: unset;
    display: flex;
    align-items: center;
    justify-content: center;
    cursor: pointer;
    padding: 1px;
    border-radius: 50%;
    transition: transform 0.15s ease, opacity 0.15s ease;
  }

  .profile-btn:hover {
    transform: scale(1.08);
    opacity: 0.9;
  }

  .profile-avatar-pill {
    display: flex;
    align-items: center;
    justify-content: center;
    width: 19px;
    height: 19px;
    border-radius: 50%;
    background: #3b82f6;
    color: #ffffff;
    font-size: 10.5px;
    font-weight: 700;
  }

  .tool-icon {
    width: 13.5px;
    height: 13.5px;
  }

  .shield-btn:hover {
    color: var(--accent);
  }

  .spinner {
    width: 11px;
    height: 11px;
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

  /* Docked Panel Dropdown */
  .panel {
    max-height: 280px;
    margin: 5px 10px 0;
    padding: 4px;
    overflow-y: auto;
    border: 1px solid var(--border);
    border-radius: var(--radius);
    background: color-mix(in srgb, var(--panel) 96%, transparent);
    backdrop-filter: blur(20px) saturate(180%);
    -webkit-backdrop-filter: blur(20px) saturate(180%);
    box-shadow: var(--shadow);
    animation: panelPop 0.16s cubic-bezier(0.16, 1, 0.3, 1);
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
    cursor: pointer;
    transition: background 0.1s ease;
  }

  .item.selected,
  .item:hover {
    background: var(--hover);
  }

  .badge {
    flex: 0 0 52px;
    color: var(--muted);
    font-size: 9.5px;
    font-weight: 600;
    letter-spacing: 0.05em;
    text-transform: uppercase;
    padding: 1px 4px;
    border-radius: 4px;
    background: var(--surface-2);
    text-align: center;
  }

  .badge.tab {
    color: var(--accent);
    background: var(--accent-soft);
  }

  .text {
    flex: 1 1 auto;
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    color: var(--text);
  }

  .detail {
    flex: 0 1 38%;
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    color: var(--muted);
    font-size: 11px;
    text-align: right;
  }
</style>
