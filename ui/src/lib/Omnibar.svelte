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
  let insecure = $state(false);
  let loading = $state(false);
  let suggestTimer: ReturnType<typeof setTimeout> | undefined;
  let inputEl = $state<HTMLInputElement | null>(null);

  function openBar() {
    focused = true;
    send({ type: 'setOmnibarOpen', open: true });
    inputEl?.focus();
    inputEl?.select();
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
    if (focusPage) send({ type: 'focusPage' });
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
          insecure = active.url.startsWith('http://');
          if (!focused) input = isBlank(active.url) ? '' : displayUrl(active.url);
        }
        break;
      }
      case 'loading':
        if (event.id === activeId) {
          loading = event.loading;
          insecure = event.url.startsWith('http://');
          if (!focused) input = isBlank(event.url) ? '' : displayUrl(event.url);
        }
        break;
      case 'urlChanged':
        if (event.id === activeId && !focused) {
          insecure = event.url.startsWith('http://');
          input = isBlank(event.url) ? '' : displayUrl(event.url);
        }
        break;
      case 'suggestionsReady':
        if (event.query === input) {
          suggestions = event.items;
          selected = -1;
        }
        break;
      case 'focusOmnibar':
        openBar();
        break;
      case 'drawerState':
        if (event.open) closeBar(false);
        break;
      default:
        break;
    }
  }

  function onInput() {
    if (!focused) return;
    send({ type: 'setOmnibarOpen', open: true });
    if (suggestTimer) clearTimeout(suggestTimer);
    const query = input;
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
      if (suggestions.length) selected = (selected + 1) % suggestions.length;
      return;
    }
    if (event.key === 'ArrowUp') {
      event.preventDefault();
      if (suggestions.length) selected = (selected - 1 + suggestions.length) % suggestions.length;
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
    if (event.target === event.currentTarget) closeBar();
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
    return () => window.removeEventListener('keydown', onWindowKey, true);
  });
</script>

<div class="root" class:open={focused} onclick={onRootClick} role="presentation">
  <div class="bar">
    <button
      class="logo-btn"
      title="Peek menu"
      type="button"
      onclick={(e) => {
        e.stopPropagation();
        send({ type: 'setDrawer', open: true });
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
</div>

<style>
  .root {
    position: fixed;
    inset: 0;
    background: transparent;
  }

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
    box-shadow: 0 1px 2px rgba(0, 0, 0, 0.06);
  }

  .bar:focus-within {
    border-color: color-mix(in srgb, var(--accent) 55%, var(--border));
  }

  .logo-btn {
    all: unset;
    display: flex;
    align-items: center;
    justify-content: center;
    cursor: pointer;
    flex-shrink: 0;
    padding: 0;
    margin-left: -3px;
    margin-right: -2px;
    border-radius: 4px;
    transition: opacity 0.15s ease, transform 0.15s ease;
  }

  .logo-btn:hover {
    opacity: 0.85;
    transform: scale(1.05);
  }

  .bar-logo {
    width: 17px;
    height: 17px;
    border-radius: 4px;
    display: block;
    user-select: none;
    box-shadow: 0 1px 3px rgba(0, 0, 0, 0.35);
  }

  input {

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

  input::placeholder {
    color: var(--muted);
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
    max-height: 300px;
    margin: 6px 10px 0;
    padding: 4px;
    overflow-y: auto;
    border: 1px solid var(--border);
    border-radius: var(--radius);
    background: var(--panel);
    box-shadow: var(--shadow);
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
