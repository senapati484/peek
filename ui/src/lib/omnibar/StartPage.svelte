<script lang="ts">
  import type { EdgeSide, PinnedSite, Suggestion } from '../ipc';
  import { cleanDomain, domainInitial } from '../utils';
  import AddPinModal from './AddPinModal.svelte';
  import SuggestionsDropdown from './SuggestionsDropdown.svelte';

  interface Props {
    searchEngine: string;
    activeProfile: string;
    drawerOpen: boolean;
    drawerSide: EdgeSide;
    pinnedSites: PinnedSite[];
    input: string;
    focused: boolean;
    suggestions: Suggestion[];
    selected: number;
    inputEl?: HTMLInputElement;
    onInput: () => void;
    onKeydown: (e: KeyboardEvent) => void;
    onFocus: () => void;
    onAcceptSuggestion: (item: Suggestion) => void;
    onOpenPin: (url: string) => void;
    onUnpin: (url: string, e: MouseEvent) => void;
    onAddPin: (url: string, title?: string) => void;
    triggerPinPreview: (url: string, el: HTMLElement) => void;
    dismissPinPreview: () => void;
    onOpenDrawer: (side: EdgeSide, section?: string) => void;
  }

  let {
    searchEngine,
    activeProfile,
    drawerOpen,
    drawerSide,
    pinnedSites,
    input = $bindable(''),
    focused = $bindable(false),
    suggestions,
    selected,
    inputEl = $bindable(),
    onInput,
    onKeydown,
    onFocus,
    onAcceptSuggestion,
    onOpenPin,
    onUnpin,
    onAddPin,
    triggerPinPreview,
    dismissPinPreview,
    onOpenDrawer,
  }: Props = $props();

  let showAddPinModal = $state(false);
  let profileInitial = $derived((activeProfile?.[0] || 'D').toUpperCase());
</script>

<div class="start-center">
  <!-- Left Corner: Quick Tabs Affordance (visible unless left drawer is open) -->
  {#if !drawerOpen || drawerSide !== 'left'}
    <div class="start-corner left">
      <button
        class="start-corner-pill"
        title="Open Tabs (or hover left edge) • ⌘⇧["
        type="button"
        onclick={(e) => {
          e.stopPropagation();
          onOpenDrawer('left', 'tabs');
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
  {/if}

  <!-- Right Corner: Quick Settings & Profile Affordances (visible unless right drawer is open) -->
  {#if !drawerOpen || drawerSide !== 'right'}
    <div class="start-corner right">
      <button
        class="start-corner-pill"
        title="Open Settings & Tools (or hover right edge) • ⌘⇧]"
        type="button"
        onclick={(e) => {
          e.stopPropagation();
          onOpenDrawer('right', 'general');
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
          onOpenDrawer('right', 'profiles');
        }}
      >
        <span class="profile-avatar-dot">{profileInitial}</span>
        <span class="profile-chip-name">{activeProfile}</span>
      </button>
    </div>
  {/if}

  <!-- Left & Right Edge Touchpoint Indicators -->
  {#if !drawerOpen || drawerSide !== 'left'}
    <button
      class="edge-hover-indicator left"
      title="Hover or click left edge to view Tabs (⌘⇧[)"
      type="button"
      onclick={(e) => {
        e.stopPropagation();
        onOpenDrawer('left', 'tabs');
      }}
      onmouseenter={() => {
        onOpenDrawer('left', 'tabs');
      }}
      aria-label="Open Tabs Drawer"
    >
      <span class="edge-indicator-bar"></span>
    </button>
  {/if}

  {#if !drawerOpen || drawerSide !== 'right'}
    <button
      class="edge-hover-indicator right"
      title="Hover or click right edge to view Settings (⌘⇧])"
      type="button"
      onclick={(e) => {
        e.stopPropagation();
        onOpenDrawer('right', 'general');
      }}
      onmouseenter={() => {
        onOpenDrawer('right', 'general');
      }}
      aria-label="Open Settings Drawer"
    >
      <span class="edge-indicator-bar"></span>
    </button>
  {/if}

  <div class="hero-brand">
    <div class="hero-ambient-glow" aria-hidden="true"></div>
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
        onfocus={onFocus}
        placeholder={`Search ${searchEngine} or enter address...`}
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
            inputEl?.focus();
          }}
          aria-label="Clear input"
        >
          ×
        </button>
      {/if}
    </div>

    {#if suggestions.length > 0}
      <SuggestionsDropdown
        {suggestions}
        {selected}
        variant="start"
        onAccept={onAcceptSuggestion}
      />
    {/if}
  </div>

  <!-- Only show Pinned Sites when user is NOT actively typing and suggestions are closed -->
  {#if !input.trim() && !suggestions.length}
    <div class="pinned-section">
      <div class="pinned-grid">
        {#each pinnedSites as site (site.id)}
          <div class="pin-card-wrap">
            <button
              class="pin-card"
              type="button"
              title={`${site.title || cleanDomain(site.url)}\n${site.url}\n(Space to Peek)`}
              data-preview-url={site.url}
              onclick={() => onOpenPin(site.url)}
              onmouseenter={(e) => {
                if (e.shiftKey) triggerPinPreview(site.url, e.currentTarget);
              }}
              onmouseleave={() => {
                dismissPinPreview();
              }}
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
              onclick={(e) => onUnpin(site.url, e)}
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

    <!-- Minimalist Keyboard & Gesture Affordances -->
    <div class="start-hints-row" aria-label="Shortcuts and Gestures">
      <span class="start-hint-pill" title="Press ⌘K or type > for Command Palette">
        <kbd class="start-kbd">⌘</kbd><kbd class="start-kbd">K</kbd> Commands
      </span>
      <span class="start-hint-dot">•</span>
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
  {/if}

  <AddPinModal
    isOpen={showAddPinModal}
    onClose={() => { showAddPinModal = false; }}
    onSubmit={onAddPin}
  />
</div>

<style>
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
    background: var(--surface);
    border: 1px solid var(--border);
    color: var(--muted);
    font-size: 12px;
    font-weight: 500;
    cursor: pointer;
    box-shadow: var(--shadow-sm);
    backdrop-filter: blur(16px);
    -webkit-backdrop-filter: blur(16px);
    transition: all 0.16s ease;
  }

  .start-corner-pill:hover {
    background: var(--hover);
    color: var(--text);
    border-color: color-mix(in srgb, var(--accent) 50%, var(--border));
    transform: translateY(-1px);
    box-shadow: var(--shadow);
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
    z-index: 25;
    opacity: 0.45;
    transition: opacity 0.2s ease, transform 0.2s cubic-bezier(0.16, 1, 0.3, 1);
  }

  .edge-hover-indicator.left {
    left: 4px;
  }

  .edge-hover-indicator.right {
    right: 4px;
  }

  .edge-hover-indicator:hover {
    opacity: 1;
    transform: translateY(-50%) scale(1.2);
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
    position: relative;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    margin-bottom: 24px;
  }

  .hero-ambient-glow {
    position: absolute;
    width: 260px;
    height: 260px;
    border-radius: 50%;
    background: radial-gradient(circle, var(--accent-glow, rgba(59, 130, 246, 0.22)) 0%, transparent 70%);
    filter: blur(48px);
    pointer-events: none;
    z-index: 0;
    opacity: 0.65;
    animation: ambientPulse 6s ease-in-out infinite alternate;
  }

  @keyframes ambientPulse {
    0% { transform: scale(0.9); opacity: 0.45; }
    100% { transform: scale(1.15); opacity: 0.75; }
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

  .search-box {
    width: 100%;
    max-width: min(620px, calc(100vw - 48px));
    position: relative;
    z-index: 100;
    border-radius: 9999px;
    background: var(--panel);
    border: 1px solid var(--border);
    box-shadow: var(--shadow-lg), 0 2px 8px rgba(15, 23, 42, 0.04);
    backdrop-filter: blur(24px);
    -webkit-backdrop-filter: blur(24px);
    transition: all 0.2s cubic-bezier(0.16, 1, 0.3, 1);
  }

  .search-box:focus-within {
    border-color: var(--accent);
    box-shadow: 0 0 0 3px var(--accent-soft), var(--shadow-lg);
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

  .pin-card:focus-visible {
    outline: 2px solid var(--accent);
    outline-offset: 2px;
    background: color-mix(in srgb, var(--surface) 80%, transparent);
    transform: translateY(-2px);
  }

  .pin-icon-box {
    width: 48px;
    height: 48px;
    border-radius: 13px;
    display: flex;
    align-items: center;
    justify-content: center;
    background: var(--surface);
    border: 1px solid var(--border);
    box-shadow: var(--shadow-sm);
    transition: all 0.18s cubic-bezier(0.16, 1, 0.3, 1);
  }

  .pin-card:hover .pin-icon-box {
    box-shadow: var(--shadow), 0 0 0 1px var(--accent-glow);
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

  .start-hints-row {
    display: flex;
    align-items: center;
    justify-content: center;
    flex-wrap: wrap;
    gap: 8px;
    margin-top: 24px;
    user-select: none;
    pointer-events: auto;
    animation: fadeIn 0.3s cubic-bezier(0.16, 1, 0.3, 1);
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
    font-weight: 600;
    color: var(--text-secondary);
    background: var(--surface-2);
    border: 1px solid var(--border);
    border-radius: 4px;
    padding: 1.5px 5.5px;
    box-shadow: var(--shadow-sm);
    line-height: 1.2;
  }

  .start-hint-dot {
    font-size: 10px;
    color: var(--border);
    user-select: none;
  }

  .start-hint-icon {
    font-size: 12px;
    opacity: 0.85;
  }
</style>
