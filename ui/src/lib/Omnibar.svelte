<script lang="ts">
  import { onMount } from 'svelte';
  import {
    displayUrl,
    isBlank,
    onEvent,
    requestState,
    send,
    type EdgeSide,
    type PeekEvent,
    type PinnedSite,
    type Suggestion,
  } from './ipc';

  let input = $state('');
  let focused = $state(false);
  let suggestions = $state<Suggestion[]>([]);
  let selected = $state(-1);
  let originalQuery = $state('');
  let activeId = $state<number | null>(null);
  let activeUrl = $state('about:blank');
  let activeTitle = $state('');
  let activeFavicon = $state<string | null>(null);
  let insecure = $state(false);
  let loading = $state(false);
  let searchEngine = $state('Google');
  let activeProfile = $state('default');
  let profileInitial = $derived((activeProfile[0] || 'D').toUpperCase());
  let drawerOpen = $state(false);
  let suggestTimer: ReturnType<typeof setTimeout> | undefined;
  let inputEl = $state<HTMLInputElement | null>(null);

  // Pinned Websites State
  let pinnedSites = $state<PinnedSite[]>([]);
  let showAddPinModal = $state(false);
  let newPinUrl = $state('');
  let newPinTitle = $state('');

  let isStartPage = $derived(isBlank(activeUrl));

  let isCurrentUrlPinned = $derived(
    !isBlank(activeUrl) &&
      pinnedSites.some((p) => {
        try {
          const pHost = new URL(p.url).hostname.replace(/^www\./, '').toLowerCase();
          const cHost = new URL(activeUrl).hostname.replace(/^www\./, '').toLowerCase();
          const pPath = new URL(p.url).pathname.replace(/\/$/, '');
          const cPath = new URL(activeUrl).pathname.replace(/\/$/, '');
          return pHost === cHost && pPath === cPath;
        } catch {
          return p.url === activeUrl;
        }
      })
  );

  let lastBarCycle = 0;
  function onBarWheel(event: WheelEvent) {
    if (Math.abs(event.deltaX) > 20 && Math.abs(event.deltaY) < 18) {
      const now = Date.now();
      if (now - lastBarCycle < 240) return;
      lastBarCycle = now;
      send({ type: 'cycleTab', forward: event.deltaX > 0 });
    }
  }

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
          activeTitle = active.title;
          activeFavicon = active.favicon;
          insecure = active.url.startsWith('http://');
          if (!focused) {
            input = isBlank(active.url) ? '' : displayUrl(active.url);
          }
          if (isBlank(active.url)) {
            setTimeout(() => {
              inputEl?.focus();
              inputEl?.select();
            }, 30);
          }
        }
        break;
      }
      case 'titleChanged':
        if (event.id === activeId) {
          activeTitle = event.title;
        }
        break;
      case 'pinnedSitesChanged':
        pinnedSites = event.sites;
        break;
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
        drawerOpen = event.open;
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

  function togglePinCurrent() {
    if (isBlank(activeUrl)) return;
    if (isCurrentUrlPinned) {
      send({ type: 'unpinSite', url: activeUrl });
    } else {
      send({
        type: 'pinSite',
        url: activeUrl,
        title: activeTitle || displayUrl(activeUrl),
        favicon: activeFavicon || null,
      });
    }
  }

  function handleOpenPin(url: string) {
    send({ type: 'navigate', input: url });
  }

  function handleUnpin(url: string, e: MouseEvent) {
    e.stopPropagation();
    send({ type: 'unpinSite', url });
  }

  function handleAddPinSubmit() {
    const raw = newPinUrl.trim();
    if (!raw) return;
    const url = raw.includes('://') ? raw : `https://${raw}`;
    send({
      type: 'pinSite',
      url,
      title: newPinTitle.trim() || undefined,
    });
    newPinUrl = '';
    newPinTitle = '';
    showAddPinModal = false;
  }

  function domainInitial(url: string): string {
    if (!url || isBlank(url)) return '✦';
    try {
      const h = new URL(url).hostname.replace(/^www\./, '');
      return (h[0] || '✦').toUpperCase();
    } catch {
      return '✦';
    }
  }

  function cleanDomain(url: string): string {
    try {
      return new URL(url).hostname.replace(/^www\./, '');
    } catch {
      return url.replace(/^https?:\/\//, '').replace(/^www\./, '');
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
    if ((event.metaKey || event.ctrlKey) && event.key.toLowerCase() === 'a') {
      inputEl?.select();
      return;
    }
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
        if (selected === -1) {
          originalQuery = input;
        }
        selected = (selected + 1) % suggestions.length;
        const item = suggestions[selected];
        if (item) {
          input = item.url ?? item.text;
        }
      }
      return;
    }
    if (event.key === 'ArrowUp') {
      event.preventDefault();
      if (suggestions.length) {
        if (selected <= 0) {
          selected = -1;
          input = originalQuery;
        } else {
          selected = selected - 1;
          const item = suggestions[selected];
          if (item) {
            input = item.url ?? item.text;
          }
        }
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
    send({ type: 'getPinnedSites' });

    const syncTheme = () => {
      const savedTheme = localStorage.getItem('peek_theme');
      if (savedTheme === 'dark') {
        document.documentElement.dataset.theme = 'dark';
      } else if (savedTheme === 'light') {
        document.documentElement.dataset.theme = 'light';
      } else {
        delete document.documentElement.dataset.theme;
      }
      const savedAccent = localStorage.getItem('peek_accent');
      if (savedAccent) {
        document.documentElement.dataset.accent = savedAccent;
      }
    };
    syncTheme();
    window.addEventListener('storage', syncTheme);

    const onWindowKey = (event: KeyboardEvent) => {
      const mod = event.metaKey || event.ctrlKey;
      const key = event.key.toLowerCase();

      if (mod && !event.shiftKey && !event.altKey) {
        if (key === 'l') {
          event.preventDefault();
          openBar();
          return;
        }
        if (key === 't') {
          event.preventDefault();
          send({ type: 'newTab' });
          return;
        }
        if (key === 'w') {
          event.preventDefault();
          if (activeId != null) {
            send({ type: 'closeTab', id: activeId });
          }
          return;
        }
        if (key === 'r') {
          event.preventDefault();
          send({ type: 'reload' });
          return;
        }
      }

      if (mod && event.shiftKey && !event.altKey) {
        if (key === '[' || key === '{') {
          event.preventDefault();
          send({ type: 'setDrawer', open: !drawerOpen, side: 'left', section: 'tabs' });
          return;
        }
        if (key === ']' || key === '}') {
          event.preventDefault();
          send({ type: 'setDrawer', open: !drawerOpen, side: 'right', section: 'settings' });
          return;
        }
      }

      if (event.key === 'Escape' && !isStartPage && focused) {
        event.preventDefault();
        closeBar();
        return;
      }
    };
    window.addEventListener('keydown', onWindowKey, true);

    let edgeTimer: ReturnType<typeof setTimeout> | null = null;
    let lastEdge = 0;

    const onWindowMouseMove = (ev: MouseEvent) => {
      if (!isStartPage || drawerOpen || ev.buttons !== 0) return;
      const zone = 10;
      const atLeft = ev.clientX <= zone;
      const atRight = window.innerWidth - ev.clientX <= zone;
      if (!atLeft && !atRight) {
        if (edgeTimer) {
          clearTimeout(edgeTimer);
          edgeTimer = null;
        }
        return;
      }
      if (edgeTimer) return;
      const side: EdgeSide = atLeft ? 'left' : 'right';
      const section = atLeft ? 'tabs' : 'settings';
      edgeTimer = setTimeout(() => {
        edgeTimer = null;
        const now = Date.now();
        if (now - lastEdge < 800) return;
        lastEdge = now;
        send({ type: 'setDrawer', open: true, side, section });
      }, 140);
    };

    const onWindowMouseLeave = () => {
      if (edgeTimer) {
        clearTimeout(edgeTimer);
        edgeTimer = null;
      }
    };

    window.addEventListener('mousemove', onWindowMouseMove, { passive: true });
    window.addEventListener('mouseleave', onWindowMouseLeave);

    if (isStartPage) {
      setTimeout(() => inputEl?.focus(), 60);
    }
    return () => {
      window.removeEventListener('keydown', onWindowKey, true);
      window.removeEventListener('storage', syncTheme);
      window.removeEventListener('mousemove', onWindowMouseMove);
      window.removeEventListener('mouseleave', onWindowMouseLeave);
      if (edgeTimer) clearTimeout(edgeTimer);
    };
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
    <!-- Center Start Page Layout: Responsive Search Bar + Clean Corner Affordances -->
    <div class="start-center">
      {#if !drawerOpen}
        <!-- Left Corner: Quick Tabs Affordance -->
        <div class="start-corner left">
          <button
            class="start-corner-pill"
            title="Open Tabs (or hover left edge) • ⌘⇧["
            type="button"
            onclick={(e) => {
              e.stopPropagation();
              send({ type: 'setDrawer', open: true, side: 'left', section: 'tabs' });
            }}
          >
            <svg viewBox="0 0 16 16" fill="none" stroke="currentColor" class="corner-icon">
              <rect x="2" y="2.5" width="12" height="11" rx="2" stroke-width="1.3" />
              <path d="M6 2.5v11" stroke-width="1.3" />
            </svg>
            <span class="corner-text">Tabs</span>
            <kbd class="corner-kbd">⌘⇧[</kbd>
          </button>
        </div>

        <!-- Right Corner: Quick Settings & Profile Affordances -->
        <div class="start-corner right">
          <button
            class="start-corner-pill"
            title="Open Settings & Tools (or hover right edge) • ⌘⇧]"
            type="button"
            onclick={(e) => {
              e.stopPropagation();
              send({ type: 'setDrawer', open: true, side: 'right', section: 'settings' });
            }}
          >
            <svg viewBox="0 0 16 16" fill="none" stroke="currentColor" class="corner-icon">
              <circle cx="8" cy="8" r="2.5" stroke-width="1.3" />
              <path d="M8 1.5v1.8M8 12.7v1.8M1.5 8h1.8M12.7 8h1.8M3.4 3.4l1.3 1.3M11.3 11.3l1.3 1.3M3.4 12.6l1.3-1.3M11.3 4.7l1.3-1.3" stroke-width="1.3" stroke-linecap="round" />
            </svg>
            <span class="corner-text">Settings</span>
            <kbd class="corner-kbd">⌘⇧]</kbd>
          </button>

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

        <!-- Left & Right Edge Touchpoint Indicators -->
        <button
          class="edge-hover-indicator left"
          title="Hover left edge to view Tabs"
          type="button"
          onclick={(e) => {
            e.stopPropagation();
            send({ type: 'setDrawer', open: true, side: 'left', section: 'tabs' });
          }}
        >
          <span class="edge-indicator-bar"></span>
        </button>

        <button
          class="edge-hover-indicator right"
          title="Hover right edge to view Settings"
          type="button"
          onclick={(e) => {
            e.stopPropagation();
            send({ type: 'setDrawer', open: true, side: 'right', section: 'settings' });
          }}
        >
          <span class="edge-indicator-bar"></span>
        </button>
      {/if}

      <div class="hero-brand">
        <div class="logo-wrapper">
          <img src="/logo.png" alt="Peek Logo" class="hero-logo" />
        </div>
      </div>

      <div class="search-box">
        <div class="search-input-wrap">
          <svg class="search-icon" viewBox="0 0 20 20" fill="none" stroke="currentColor">
            <circle cx="9" cy="9" r="5.5" stroke-width="1.7" stroke-linecap="round" />
            <path d="M13 13L17 17" stroke-width="1.7" stroke-linecap="round" />
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
            placeholder=""
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

      <!-- Pinned Websites Speed Dial Grid -->
      <div class="pinned-section">
        <div class="pinned-grid">
          {#each pinnedSites as site (site.id)}
            <div class="pin-card-wrap">
              <button
                class="pin-card"
                type="button"
                title={`${site.title || cleanDomain(site.url)}\n${site.url}\n(Space to Peek)`}
                data-preview-url={site.url}
                onclick={() => handleOpenPin(site.url)}
              >
                <div class="pin-icon-box">
                  {#if site.favicon}
                    <img
                      src={site.favicon}
                      alt=""
                      class="pin-favicon"
                      onerror={(e) => { (e.currentTarget as HTMLElement).style.display = 'none'; }}
                    />
                  {:else}
                    <span class="pin-initial">{domainInitial(site.url)}</span>
                  {/if}
                </div>
                <span class="pin-title">{site.title || cleanDomain(site.url)}</span>
              </button>
              <button
                class="pin-remove-btn"
                type="button"
                title={`Unpin ${site.title || site.url}`}
                onclick={(e) => handleUnpin(site.url, e)}
                aria-label={`Unpin ${site.title || site.url}`}
              >
                ×
              </button>
            </div>
          {/each}

          <!-- Add Pin Button -->
          <button
            class="pin-card add-pin-card"
            type="button"
            title="Pin a website"
            onclick={() => { showAddPinModal = true; }}
          >
            <div class="pin-icon-box add-box">
              <svg viewBox="0 0 16 16" fill="none" stroke="currentColor" class="add-icon">
                <path d="M8 3.5v9M3.5 8h9" stroke-width="1.8" stroke-linecap="round"/>
              </svg>
            </div>
            <span class="pin-title">Add Pin</span>
          </button>
        </div>
      </div>

      {#if showAddPinModal}
        <div
          class="pin-modal-backdrop"
          onclick={(e) => { if (e.target === e.currentTarget) showAddPinModal = false; }}
          role="presentation"
        >
          <div
            class="pin-modal"
            role="dialog"
            aria-modal="true"
            aria-labelledby="modal-title"
            tabindex="-1"
            onkeydown={(e) => { if (e.key === 'Escape') showAddPinModal = false; }}
          >
            <div class="modal-header">
              <h3 id="modal-title" class="modal-title">Pin Website</h3>
              <button class="modal-close-btn" type="button" onclick={() => { showAddPinModal = false; }}>×</button>
            </div>
            <form onsubmit={(e) => { e.preventDefault(); handleAddPinSubmit(); }} class="modal-form">
              <label class="modal-label">
                <span>Website URL</span>
                <input
                  type="text"
                  placeholder="https://example.com"
                  bind:value={newPinUrl}
                  class="modal-input"
                  required
                />
              </label>
              <label class="modal-label">
                <span>Display Name (optional)</span>
                <input
                  type="text"
                  placeholder="Example"
                  bind:value={newPinTitle}
                  class="modal-input"
                />
              </label>
              <div class="modal-actions">
                <button type="button" class="btn secondary" onclick={() => { showAddPinModal = false; }}>Cancel</button>
                <button type="submit" class="btn primary">Pin Website</button>
              </div>
            </form>
          </div>
        </div>
      {/if}

      <!-- Minimalist Keyboard & Gesture Affordances -->
      <div class="start-hints-row" aria-label="Shortcuts and Gestures">
        <span class="start-hint-pill">
          <kbd class="start-kbd">⌘</kbd><kbd class="start-kbd">L</kbd> Search
        </span>
        <span class="start-hint-dot">•</span>
        <span class="start-hint-pill" title="Press Space or hold Shift while hovering any link to preview without opening a tab">
          <kbd class="start-kbd">Space</kbd> Link Peek
        </span>
        <span class="start-hint-dot">•</span>
        <span class="start-hint-pill" title="Swipe two fingers horizontally on the top bar to switch tabs">
          <span class="start-hint-icon">⇄</span> Swipe to Switch
        </span>
        <span class="start-hint-dot">•</span>
        <span class="start-hint-pill" title="Swipe from left/right screen edges to open drawers">
          <span class="start-hint-icon">⇥</span> Edge Drawers
        </span>
      </div>
    </div>
  {:else}
    <!-- Docked Minimalist Top Bar -->
    <div class="bar-container">
      <div class="bar" onwheel={onBarWheel} title="Swipe horizontally with 2 fingers to cycle tabs">
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

        <!-- Pin Website Button -->
        <button
          class="tool-btn pin-btn"
          class:pinned={isCurrentUrlPinned}
          title={isCurrentUrlPinned ? "Unpin Website" : "Pin Website"}
          type="button"
          onclick={(e) => {
            e.stopPropagation();
            togglePinCurrent();
          }}
        >
          <svg viewBox="0 0 16 16" fill={isCurrentUrlPinned ? "currentColor" : "none"} stroke="currentColor" class="tool-icon pin-icon">
            <path d="M9.828.722a.5.5 0 0 1 .354.146l4.95 4.95a.5.5 0 0 1 0 .707c-.48.48-1.072.588-1.503.588-.177 0-.335-.018-.46-.039l-3.134 3.134a5.927 5.927 0 0 1 .16 1.013c.046.702-.032 1.687-.72 2.375a.5.5 0 0 1-.707 0l-2.829-2.828-3.182 3.182c-.195.195-1.219.909-1.316.812-.097-.097.617-1.121.812-1.316l3.182-3.182-2.828-2.829a.5.5 0 0 1 0-.707c.688-.688 1.673-.767 2.375-.72a5.922 5.922 0 0 1 1.013.16l3.134-3.133a2.772 2.772 0 0 1-.04-.461c0-.43.108-1.022.589-1.503a.5.5 0 0 1 .353-.146z" stroke-width="1.2" stroke-linecap="round" stroke-linejoin="round" />
          </svg>
        </button>

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
    background: var(--bg);
    animation: fadeIn 0.28s cubic-bezier(0.16, 1, 0.3, 1);
  }

  /* Corner Affordances: Tabs (Left) and Settings & Profile (Right) */
  .start-corner {
    position: absolute;
    top: 20px;
    display: flex;
    align-items: center;
    gap: 8px;
    z-index: 20;
    animation: fadeIn 0.3s cubic-bezier(0.16, 1, 0.3, 1);
  }

  .start-corner.left {
    left: 24px;
  }

  .start-corner.right {
    right: 24px;
  }

  .start-corner-pill {
    all: unset;
    display: inline-flex;
    align-items: center;
    gap: 7px;
    padding: 5px 12px 5px 10px;
    border-radius: 9999px;
    background: color-mix(in srgb, var(--surface) 80%, transparent);
    border: 1px solid var(--border);
    color: var(--muted);
    font-size: 12px;
    font-weight: 500;
    cursor: pointer;
    box-shadow: 0 2px 8px rgba(0, 0, 0, 0.25);
    backdrop-filter: blur(16px);
    -webkit-backdrop-filter: blur(16px);
    transition: all 0.16s ease;
  }

  .start-corner-pill:hover {
    background: var(--hover);
    color: var(--text);
    border-color: color-mix(in srgb, var(--accent) 50%, var(--border));
    transform: translateY(-1px);
    box-shadow: 0 4px 12px rgba(0, 0, 0, 0.35);
  }

  .corner-icon {
    width: 14px;
    height: 14px;
    color: var(--muted);
    transition: color 0.15s ease;
  }

  .start-corner-pill:hover .corner-icon {
    color: var(--accent);
  }

  .corner-text {
    letter-spacing: -0.01em;
  }

  .corner-kbd {
    all: unset;
    font-size: 10px;
    font-weight: 600;
    font-family: inherit;
    padding: 1px 5px;
    border-radius: 4px;
    background: var(--surface-2);
    color: var(--muted);
    border: 1px solid var(--border-subtle);
    margin-left: 2px;
    letter-spacing: 0.02em;
  }

  .start-corner-pill:hover .corner-kbd {
    color: var(--text);
    border-color: var(--border);
  }

  /* Edge touchpoint indicators */
  .edge-hover-indicator {
    all: unset;
    position: fixed;
    top: 50%;
    transform: translateY(-50%);
    width: 14px;
    height: 72px;
    display: flex;
    align-items: center;
    justify-content: center;
    cursor: pointer;
    z-index: 15;
    opacity: 0.4;
    transition: opacity 0.2s ease, transform 0.2s cubic-bezier(0.16, 1, 0.3, 1);
  }

  .edge-hover-indicator.left {
    left: 2px;
  }

  .edge-hover-indicator.right {
    right: 2px;
  }

  .edge-hover-indicator:hover {
    opacity: 1;
    transform: translateY(-50%) scale(1.3);
  }

  .edge-indicator-bar {
    width: 3.5px;
    height: 100%;
    border-radius: 9999px;
    background: var(--border);
    transition: background 0.2s ease, box-shadow 0.2s ease;
  }

  .edge-hover-indicator:hover .edge-indicator-bar {
    background: var(--accent);
    box-shadow: 0 0 10px var(--accent-glow);
  }

  .start-profile-chip {
    all: unset;
    display: inline-flex;
    align-items: center;
    gap: 7px;
    padding: 4px 11px 4px 5px;
    border-radius: 9999px;
    background: color-mix(in srgb, var(--panel) 75%, transparent);
    border: 1px solid var(--border);
    color: var(--text-secondary);
    font-size: 12px;
    font-weight: 500;
    cursor: pointer;
    box-shadow: var(--shadow-sm);
    backdrop-filter: blur(12px);
    -webkit-backdrop-filter: blur(12px);
    transition: all 0.15s ease;
  }

  .start-profile-chip:hover {
    background: var(--hover);
    color: var(--text);
    border-color: var(--accent);
  }

  .profile-avatar-dot {
    display: flex;
    align-items: center;
    justify-content: center;
    width: 20px;
    height: 20px;
    border-radius: 50%;
    background: var(--accent);
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
      transform: translateY(8px) scale(0.99);
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
    margin-bottom: 22px;
  }

  .logo-wrapper {
    position: relative;
    margin-bottom: 0;
  }

  .hero-logo {
    position: relative;
    width: 72px;
    height: 72px;
    display: block;
    user-select: none;
    -webkit-user-drag: none;
    filter: drop-shadow(0 14px 28px rgba(0, 0, 0, 0.45)) drop-shadow(0 0 24px var(--accent-glow));
    z-index: 1;
    transition: transform 0.22s cubic-bezier(0.16, 1, 0.3, 1);
  }

  .hero-logo:hover {
    transform: scale(1.06);
  }

  /* Sleek, Prominent Responsive Search Box */
  .search-box {
    width: 100%;
    max-width: min(620px, calc(100vw - 48px));
    position: relative;
    border-radius: 9999px;
    background: color-mix(in srgb, var(--surface) 85%, transparent);
    border: 1px solid var(--border);
    box-shadow: 0 10px 32px rgba(0, 0, 0, 0.45), 0 2px 8px rgba(0, 0, 0, 0.25), inset 0 1px 0 rgba(255, 255, 255, 0.06);
    backdrop-filter: blur(24px);
    -webkit-backdrop-filter: blur(24px);
    transition: all 0.2s cubic-bezier(0.16, 1, 0.3, 1);
  }

  .search-box:focus-within {
    border-color: color-mix(in srgb, var(--accent) 65%, var(--border));
    box-shadow: 0 0 0 3px var(--accent-soft), 0 14px 40px rgba(0, 0, 0, 0.6), var(--shadow);
    background: var(--panel);
    transform: translateY(-1px);
  }

  .search-input-wrap {
    display: flex;
    align-items: center;
    height: 48px;
    padding: 0 18px;
    gap: 12px;
    border-radius: 9999px;
  }

  .search-icon {
    width: 17px;
    height: 17px;
    color: var(--muted);
    flex-shrink: 0;
    transition: color 0.15s ease;
  }

  .search-box:focus-within .search-icon {
    color: var(--accent);
  }

  .search-input-wrap input {
    flex: 1;
    min-width: 0;
    border: none;
    outline: none;
    background: transparent;
    color: var(--text);
    font-size: 15px;
    font-weight: 450;
    letter-spacing: -0.01em;
  }

  .clear-btn {
    all: unset;
    display: flex;
    align-items: center;
    justify-content: center;
    width: 22px;
    height: 22px;
    border-radius: 50%;
    color: var(--muted);
    font-size: 16px;
    line-height: 1;
    cursor: pointer;
    transition: all 0.15s ease;
  }

  .clear-btn:hover {
    background: var(--hover);
    color: var(--text);
  }

  .start-suggestions {
    position: absolute;
    top: calc(100% + 10px);
    left: 0;
    right: 0;
    border: 1px solid var(--border);
    border-radius: 18px;
    background: var(--panel);
    box-shadow: 0 16px 48px rgba(0, 0, 0, 0.65), 0 4px 16px rgba(0, 0, 0, 0.4);
    backdrop-filter: blur(28px);
    -webkit-backdrop-filter: blur(28px);
    padding: 8px;
    display: flex;
    flex-direction: column;
    gap: 3px;
    z-index: 50;
    animation: dropdownFade 0.18s cubic-bezier(0.16, 1, 0.3, 1);
  }

  @keyframes dropdownFade {
    from {
      opacity: 0;
      transform: translateY(-6px) scale(0.995);
    }
    to {
      opacity: 1;
      transform: translateY(0) scale(1);
    }
  }

  .suggest-item {
    display: flex;
    align-items: center;
    gap: 12px;
    width: 100%;
    padding: 9px 13px;
    border-radius: 10px;
    text-align: left;
    font-size: 13.5px;
    cursor: pointer;
    transition: background 0.12s ease;
    border: none;
    background: transparent;
  }

  .suggest-item.selected,
  .suggest-item:hover {
    background: var(--hover);
  }

  /* Pinned Websites Speed Dial Section */
  .pinned-section {
    width: 100%;
    max-width: min(620px, calc(100vw - 48px));
    margin-top: 24px;
    margin-bottom: 6px;
    display: flex;
    justify-content: center;
    user-select: none;
    animation: fadeIn 0.32s cubic-bezier(0.16, 1, 0.3, 1);
  }

  .pinned-grid {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(84px, 1fr));
    gap: 12px 8px;
    width: 100%;
    justify-content: center;
  }

  .pin-card-wrap {
    position: relative;
    display: flex;
    flex-direction: column;
    align-items: center;
  }

  .pin-card {
    all: unset;
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 8px;
    width: 100%;
    cursor: pointer;
    padding: 8px 4px 6px;
    border-radius: 12px;
    transition: transform 0.16s cubic-bezier(0.16, 1, 0.3, 1), background 0.16s ease;
  }

  .pin-card:hover {
    background: color-mix(in srgb, var(--surface) 60%, transparent);
    transform: translateY(-2px);
  }

  .pin-icon-box {
    width: 48px;
    height: 48px;
    border-radius: 13px;
    display: flex;
    align-items: center;
    justify-content: center;
    background: color-mix(in srgb, var(--surface) 85%, transparent);
    border: 1px solid var(--border);
    box-shadow: 0 4px 14px rgba(0, 0, 0, 0.22), inset 0 1px 0 rgba(255, 255, 255, 0.08);
    backdrop-filter: blur(14px);
    -webkit-backdrop-filter: blur(14px);
    transition: all 0.18s cubic-bezier(0.16, 1, 0.3, 1);
  }

  .pin-card:hover .pin-icon-box {
    box-shadow: 0 8px 24px rgba(0, 0, 0, 0.32), 0 0 0 1px var(--accent-glow);
    border-color: color-mix(in srgb, var(--accent) 55%, var(--border));
    transform: scale(1.05);
  }

  .pin-favicon {
    width: 24px;
    height: 24px;
    border-radius: 4px;
    object-fit: contain;
  }

  .pin-initial {
    font-size: 17px;
    font-weight: 600;
    color: var(--accent);
  }

  .pin-title {
    font-size: 11.5px;
    font-weight: 500;
    color: var(--text-secondary);
    max-width: 80px;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
    text-align: center;
    letter-spacing: -0.01em;
    transition: color 0.14s ease;
  }

  .pin-card:hover .pin-title {
    color: var(--text);
  }

  .pin-remove-btn {
    all: unset;
    position: absolute;
    top: 2px;
    right: 12px;
    width: 18px;
    height: 18px;
    border-radius: 50%;
    background: color-mix(in srgb, var(--panel) 90%, black);
    border: 1px solid var(--border);
    color: var(--muted);
    font-size: 13px;
    line-height: 1;
    display: flex;
    align-items: center;
    justify-content: center;
    cursor: pointer;
    opacity: 0;
    transform: scale(0.85);
    transition: all 0.15s ease;
    z-index: 5;
  }

  .pin-card-wrap:hover .pin-remove-btn {
    opacity: 1;
    transform: scale(1);
  }

  .pin-remove-btn:hover {
    background: #ef4444;
    border-color: #ef4444;
    color: #ffffff;
    box-shadow: 0 2px 8px rgba(239, 68, 68, 0.4);
  }

  .add-pin-card:hover .add-box {
    background: var(--accent-soft);
    border-color: var(--accent);
  }

  .add-icon {
    width: 18px;
    height: 18px;
    color: var(--muted);
    transition: color 0.15s ease;
  }

  .add-pin-card:hover .add-icon {
    color: var(--accent);
  }

  /* Add Pin Modal */
  .pin-modal-backdrop {
    position: fixed;
    inset: 0;
    background: rgba(0, 0, 0, 0.55);
    backdrop-filter: blur(8px);
    -webkit-backdrop-filter: blur(8px);
    display: flex;
    align-items: center;
    justify-content: center;
    z-index: 100;
    animation: fadeIn 0.16s ease;
  }

  .pin-modal {
    width: 100%;
    max-width: 360px;
    background: var(--panel);
    border: 1px solid var(--border);
    border-radius: var(--radius-lg);
    box-shadow: var(--shadow-lg), 0 0 0 1px rgba(255, 255, 255, 0.05);
    padding: 20px 22px;
    animation: dropdownFade 0.2s cubic-bezier(0.16, 1, 0.3, 1);
  }

  .modal-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    margin-bottom: 16px;
  }

  .modal-title {
    font-size: 15px;
    font-weight: 600;
    color: var(--text);
    margin: 0;
  }

  .modal-close-btn {
    all: unset;
    cursor: pointer;
    font-size: 18px;
    line-height: 1;
    color: var(--muted);
    border-radius: 4px;
    padding: 2px 6px;
    transition: color 0.12s ease;
  }

  .modal-close-btn:hover {
    color: var(--text);
  }

  .modal-form {
    display: flex;
    flex-direction: column;
    gap: 14px;
  }

  .modal-label {
    display: flex;
    flex-direction: column;
    gap: 6px;
    font-size: 12px;
    font-weight: 500;
    color: var(--text-secondary);
  }

  .modal-input {
    all: unset;
    box-sizing: border-box;
    width: 100%;
    padding: 8px 12px;
    border-radius: 8px;
    background: var(--surface-2);
    border: 1px solid var(--border);
    color: var(--text);
    font-size: 13px;
    transition: border-color 0.14s ease, box-shadow 0.14s ease;
  }

  .modal-input:focus {
    border-color: var(--accent);
    box-shadow: 0 0 0 2px var(--accent-soft);
  }

  .modal-actions {
    display: flex;
    align-items: center;
    justify-content: flex-end;
    gap: 10px;
    margin-top: 6px;
  }

  .modal-actions .btn {
    all: unset;
    cursor: pointer;
    padding: 6px 14px;
    border-radius: 8px;
    font-size: 12.5px;
    font-weight: 550;
    transition: all 0.14s ease;
  }

  .modal-actions .btn.secondary {
    color: var(--text-secondary);
    background: var(--surface-2);
    border: 1px solid var(--border);
  }

  .modal-actions .btn.secondary:hover {
    background: var(--hover);
    color: var(--text);
  }

  .modal-actions .btn.primary {
    background: var(--accent);
    color: #ffffff;
    box-shadow: 0 2px 8px var(--accent-glow);
  }

  .modal-actions .btn.primary:hover {
    filter: brightness(1.1);
    transform: translateY(-1px);
  }

  /* Minimalist Keyboard & Gesture Affordances */
  .start-hints-row {
    display: flex;
    align-items: center;
    justify-content: center;
    flex-wrap: wrap;
    gap: 8px;
    margin-top: 14px;
    user-select: none;
    pointer-events: auto;
  }

  .start-hint-pill {
    display: inline-flex;
    align-items: center;
    gap: 5px;
    font-size: 11.5px;
    color: var(--muted);
    letter-spacing: -0.01em;
    padding: 3px 6px;
    border-radius: 6px;
    transition: color 0.15s ease;
  }

  .start-hint-pill:hover {
    color: var(--text-secondary);
  }

  .start-kbd {
    font-family: -apple-system, BlinkMacSystemFont, "SF Pro Text", inherit;
    font-size: 10px;
    font-weight: 500;
    color: var(--text-secondary);
    background: var(--surface-2);
    border: 1px solid var(--border);
    border-radius: 4px;
    padding: 1px 4.5px;
    box-shadow: 0 1px 2px rgba(0, 0, 0, 0.2);
    line-height: 1.2;
  }

  .start-hint-dot {
    font-size: 10px;
    color: var(--border);
    user-select: none;
  }

  .start-hint-icon {
    font-size: 12px;
    opacity: 0.8;
  }

  /* Docked Top Bar: Clean Full-Width Toolbar */
  .bar-container {
    position: relative;
    width: 100%;
    height: 36px;
    background: var(--surface);
    border-bottom: 1px solid var(--border-subtle);
    display: flex;
    align-items: center;
    padding: 0 10px;
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
    padding: 0 10px;
    border: 1px solid var(--border);
    border-radius: 9999px;
    background: var(--surface-2);
    box-shadow: var(--shadow-sm);
    transition: all 0.15s ease;
  }

  .bar:focus-within {
    background: var(--panel);
    border-color: var(--accent);
    box-shadow: 0 0 0 2px var(--accent-soft);
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
    width: 17px;
    height: 17px;
    display: block;
    user-select: none;
    -webkit-user-drag: none;
    filter: drop-shadow(0 1px 2px rgba(0, 0, 0, 0.4));
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
    transition: background 0.12s ease, color 0.12s ease, transform 0.12s ease;
  }

  .tool-btn:hover {
    background: var(--hover);
    color: var(--text);
  }

  .pin-btn.pinned {
    color: var(--accent);
    background: var(--accent-soft);
  }

  .pin-btn.pinned:hover {
    color: var(--accent);
    filter: brightness(1.15);
  }

  .pin-icon {
    width: 13px;
    height: 13px;
    transition: transform 0.14s ease;
  }

  .pin-btn:hover .pin-icon {
    transform: scale(1.1);
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
    position: absolute;
    top: 38px;
    left: 10px;
    right: 10px;
    max-height: 280px;
    margin: 0;
    padding: 4px;
    overflow-y: auto;
    border: 1px solid var(--border);
    border-radius: var(--radius);
    background: color-mix(in srgb, var(--panel) 96%, transparent);
    backdrop-filter: blur(20px) saturate(180%);
    -webkit-backdrop-filter: blur(20px) saturate(180%);
    box-shadow: 0 12px 32px rgba(0, 0, 0, 0.45), 0 2px 6px rgba(0, 0, 0, 0.2);
    z-index: 1000;
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
    flex: 0 0 auto;
    min-width: 52px;
    color: var(--muted);
    font-size: 9.5px;
    font-weight: 600;
    letter-spacing: 0.04em;
    text-transform: uppercase;
    padding: 2px 6px;
    border-radius: 5px;
    background: var(--surface-2);
    text-align: center;
  }

  .badge.tab {
    color: var(--accent);
    background: color-mix(in srgb, var(--accent) 18%, transparent);
  }

  .badge.bookmark {
    color: #a855f7;
    background: color-mix(in srgb, #a855f7 18%, transparent);
  }

  .badge.history {
    color: var(--muted);
    background: var(--surface-2);
  }

  .badge.search {
    color: #10b981;
    background: color-mix(in srgb, #10b981 18%, transparent);
  }

  .text {
    flex: 1 1 auto;
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    color: var(--text);
    font-weight: 450;
  }

  .detail {
    flex: 0 1 auto;
    max-width: 260px;
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    color: var(--muted);
    font-size: 11.5px;
    text-align: right;
    margin-left: auto;
  }
</style>
