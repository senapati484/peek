<script lang="ts">
  import { send, type EdgeSide, type Suggestion } from '../ipc';
  import { isBlank } from '../ipc';
  import SuggestionsDropdown from './SuggestionsDropdown.svelte';

  interface Props {
    isZenMode: boolean;
    zenHovered: boolean;
    focused: boolean;
    loading: boolean;
    insecure: boolean;
    input: string;
    activeUrl: string | null;
    inputEl?: HTMLInputElement;
    invertDarkActive: boolean;
    readerModeActive: boolean;
    activeAudible: boolean;
    isCurrentUrlPinned: boolean;
    activeProfile: string;
    toastMessage: string | null;
    showTabCyclingHud: boolean;
    hudTabIndex: number;
    hudTabTotal: number;
    hudTabTitle: string;
    suggestions: Suggestion[];
    selected: number;
    onInput: () => void;
    onKeydown: (e: KeyboardEvent) => void;
    onFocus: () => void;
    onBarWheel: (e: WheelEvent) => void;
    onAcceptSuggestion: (item: Suggestion) => void;
    onTogglePin: () => void;
    onCopyCleanUrl: () => void;
    onCopyMarkdownLink: () => void;
    onOpenDrawer: (side: EdgeSide, section?: string) => void;
  }

  let {
    isZenMode,
    zenHovered = $bindable(false),
    focused = $bindable(false),
    loading,
    insecure,
    input = $bindable(''),
    activeUrl,
    inputEl = $bindable(),
    invertDarkActive,
    readerModeActive,
    activeAudible,
    isCurrentUrlPinned,
    activeProfile,
    toastMessage,
    showTabCyclingHud,
    hudTabIndex,
    hudTabTotal,
    hudTabTitle,
    suggestions,
    selected,
    onInput,
    onKeydown,
    onFocus,
    onBarWheel,
    onAcceptSuggestion,
    onTogglePin,
    onCopyCleanUrl,
    onCopyMarkdownLink,
    onOpenDrawer,
  }: Props = $props();

  let zenHoverTimer: ReturnType<typeof setTimeout> | undefined;
  let profileInitial = $derived((activeProfile?.[0] || 'D').toUpperCase());
</script>

{#if isZenMode && !zenHovered && !focused}
  <div
    class="zen-hover-trigger"
    role="presentation"
    aria-hidden="true"
    onmouseenter={() => {
      zenHovered = true;
    }}
  ></div>
{/if}

<div
  class="bar-container"
  role="toolbar"
  tabindex="-1"
  aria-label="Address bar"
  class:zen-mode={isZenMode}
  class:zen-revealed={zenHovered || focused}
  onmouseenter={() => {
    if (zenHoverTimer) clearTimeout(zenHoverTimer);
    zenHovered = true;
  }}
  onmouseleave={() => {
    zenHoverTimer = setTimeout(() => {
      zenHovered = false;
    }, 280);
  }}
>
  <div class="bar" onwheel={onBarWheel} title="Swipe horizontally with 2 fingers to cycle tabs">
    <!-- Logo / Tabs Drawer Trigger -->
    <button
      class="logo-btn"
      title="Open Tabs (Cmd+Shift+[)"
      type="button"
      onclick={(e) => {
        e.stopPropagation();
        onOpenDrawer('left', 'tabs');
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
      title={loading ? 'Stop Loading' : 'Reload (Cmd+R)'}
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
        onFocus();
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
      title={isCurrentUrlPinned ? 'Unpin Website' : 'Pin Website'}
      type="button"
      onclick={(e) => {
        e.stopPropagation();
        onTogglePin();
      }}
      aria-label="Pin or Unpin Website"
    >
      <svg viewBox="0 0 16 16" fill={isCurrentUrlPinned ? 'currentColor' : 'none'} stroke="currentColor" class="tool-icon pin-icon">
        <path d="M9.828.722a.5.5 0 0 1 .354.146l4.95 4.95a.5.5 0 0 1 0 .707c-.48.48-1.072.588-1.503.588-.177 0-.335-.018-.46-.039l-3.134 3.134a5.927 5.927 0 0 1 .16 1.013c.046.702-.032 1.687-.72 2.375a.5.5 0 0 1-.707 0l-2.829-2.828-3.182 3.182c-.195.195-1.219.909-1.316.812-.097-.097.617-1.121.812-1.316l3.182-3.182-2.828-2.829a.5.5 0 0 1 0-.707c.688-.688 1.673-.767 2.375-.72a5.922 5.922 0 0 1 1.013.16l3.134-3.133a2.772 2.772 0 0 1-.04-.461c0-.43.108-1.022.589-1.503a.5.5 0 0 1 .353-.146z" stroke-width="1.2" stroke-linecap="round" stroke-linejoin="round" />
      </svg>
    </button>

    <!-- Copy Clean URL Button (Strips tracking parameters) -->
    <button
      class="tool-btn copy-clean-btn"
      title="Copy Clean URL (⌘⇧C)"
      type="button"
      onclick={(e) => {
        e.stopPropagation();
        onCopyCleanUrl();
      }}
      aria-label="Copy Clean URL"
    >
      <svg viewBox="0 0 16 16" fill="none" stroke="currentColor" class="tool-icon">
        <path d="M6.5 9.5a3 3 0 0 0 4.24 0l2-2a3 3 0 0 0-4.24-4.24l-1 1" stroke-width="1.3" stroke-linecap="round" stroke-linejoin="round"/>
        <path d="M9.5 6.5a3 3 0 0 0-4.24 0l-2 2a3 3 0 0 0 4.24 4.24l1-1" stroke-width="1.3" stroke-linecap="round" stroke-linejoin="round"/>
      </svg>
    </button>

    <!-- Copy Markdown Link Button -->
    <button
      class="tool-btn copy-md-btn"
      title="Copy Markdown Link (⌘⌥C)"
      type="button"
      onclick={(e) => {
        e.stopPropagation();
        onCopyMarkdownLink();
      }}
      aria-label="Copy Markdown Link"
    >
      <svg viewBox="0 0 16 16" fill="none" stroke="currentColor" class="tool-icon">
        <rect x="1.5" y="3" width="13" height="10" rx="1.5" stroke-width="1.2"/>
        <path d="M4 10V6l2 2.5L8 6v4M11.5 6v4M10 8.5l1.5 1.5 1.5-1.5" stroke-width="1.1" stroke-linecap="round" stroke-linejoin="round"/>
      </svg>
    </button>

    <!-- Picture-in-Picture Video -->
    <button
      class="tool-btn pip-btn"
      title="Picture-in-Picture Video (⌘⇧P)"
      type="button"
      onclick={(e) => {
        e.stopPropagation();
        send({ type: 'togglePiP' });
      }}
      aria-label="Picture-in-Picture Video"
    >
      <svg viewBox="0 0 16 16" fill="none" stroke="currentColor" class="tool-icon">
        <rect x="2" y="3" width="12" height="9" rx="1.5" stroke-width="1.2"/>
        <rect x="8" y="7" width="5" height="4" rx="1" fill="currentColor" stroke-width="0.8"/>
      </svg>
    </button>

    <!-- Smart Night Mode Toggle (Calibrated Soft Dark Theme) -->
    <button
      class="tool-btn dark-mode-btn"
      class:active-dark={invertDarkActive}
      title={invertDarkActive ? 'Night Mode Active (Click to disable)' : 'Smart Night Mode (⌘⇧D)'}
      type="button"
      onclick={(e) => {
        e.stopPropagation();
        send({ type: 'toggleInvertDark' });
      }}
      aria-label="Smart Night Mode"
    >
      <svg viewBox="0 0 16 16" fill="none" stroke="currentColor" class="tool-icon">
        <path d="M13.5 9.5A6 6 0 1 1 6.5 2.5a4.5 4.5 0 0 0 7 7z" stroke-width="1.3" stroke-linecap="round" stroke-linejoin="round"/>
      </svg>
    </button>

    <!-- Reader Mode Button -->
    <button
      class="tool-btn reader-btn"
      class:active-reader={readerModeActive}
      title="Toggle Reader Mode (⌘⇧R)"
      type="button"
      onclick={(e) => {
        e.stopPropagation();
        send({ type: 'toggleReaderMode' });
      }}
      aria-label="Reader Mode"
    >
      <svg viewBox="0 0 16 16" fill="none" stroke="currentColor" class="tool-icon">
        <path d="M2.5 3.5A2 2 0 0 1 4.5 2h7a2 2 0 0 1 2 2v9.5l-4-2-4 2V3.5z" stroke-width="1.3" stroke-linecap="round" stroke-linejoin="round"/>
        <path d="M5.5 5.5h5M5.5 8h3.5" stroke-width="1.3" stroke-linecap="round"/>
      </svg>
    </button>

    <!-- Zen Mode / Ghost Canvas Button -->
    <button
      class="tool-btn zen-btn"
      class:active-zen={isZenMode}
      title={isZenMode ? 'Exit Zen Mode (⌘.)' : 'Zen Mode / Ghost Canvas (⌘.)'}
      type="button"
      onclick={(e) => {
        e.stopPropagation();
        send({ type: 'toggleZenMode' });
      }}
      aria-label="Zen Mode"
    >
      <svg viewBox="0 0 16 16" fill="none" stroke="currentColor" class="tool-icon">
        <path d="M2.5 5.5v-3h3M13.5 5.5v-3h-3M2.5 10.5v3h3M13.5 10.5v3h-3" stroke-width="1.3" stroke-linecap="round" stroke-linejoin="round"/>
      </svg>
    </button>

    <!-- Adblock & Privacy Shield Button -->
    <button
      class="tool-btn shield-btn"
      title="Adblock & Privacy Filters"
      type="button"
      onclick={(e) => {
        e.stopPropagation();
        onOpenDrawer('right', 'blocking');
      }}
      aria-label="Adblock and Privacy Filters"
    >
      <svg viewBox="0 0 16 16" fill="none" stroke="currentColor" class="tool-icon shield-icon">
        <path d="M8 1.5L2.5 3.8v4.2c0 3.7 2.3 6.3 5.5 7 3.2-.7 5.5-3.3 5.5-7V3.8L8 1.5z" stroke-width="1.3" stroke-linejoin="round" />
        <path d="M5.5 8l2 2 3.5-3.5" stroke-width="1.3" stroke-linecap="round" stroke-linejoin="round" />
      </svg>
    </button>

    <!-- Audio Equalizer / Mute Button -->
    {#if activeAudible}
      <button
        class="tool-btn audio-btn"
        title="Mute / Unmute Audio (⌘⇧M)"
        type="button"
        onclick={(e) => {
          e.stopPropagation();
          send({ type: 'toggleMuteTab' });
        }}
        aria-label="Mute / Unmute Audio"
      >
        <span class="audio-waves" aria-hidden="true">
          <span class="wave-bar b1"></span>
          <span class="wave-bar b2"></span>
          <span class="wave-bar b3"></span>
        </span>
      </button>
    {/if}

    <!-- Profile Button (Always visible) -->
    <button
      class="tool-btn profile-btn"
      title={`Profile: ${activeProfile} (Click to manage)`}
      type="button"
      onclick={(e) => {
        e.stopPropagation();
        onOpenDrawer('right', 'profiles');
      }}
      aria-label="Manage Profiles"
    >
      <span class="profile-avatar-pill">{profileInitial}</span>
    </button>
  </div>

  <!-- Inline Bar Toast Feedback Pill (100% visible inside top bar!) -->
  {#if toastMessage}
    <div class="bar-toast-pill" role="status" aria-live="polite">
      <span class="bar-toast-dot"></span>
      <span class="bar-toast-text">{toastMessage}</span>
    </div>
  {/if}

  <!-- Inline Bar Tab Cycling HUD -->
  {#if showTabCyclingHud}
    <div class="bar-tab-hud-pill" role="status" aria-live="polite">
      <span class="tab-hud-badge">Tab {hudTabIndex} of {hudTabTotal}</span>
      <span class="tab-hud-title">{hudTabTitle}</span>
    </div>
  {/if}

  <!-- Docked Suggestions Dropdown -->
  {#if focused && suggestions.length > 0}
    <SuggestionsDropdown
      {suggestions}
      {selected}
      variant="docked"
      onAccept={onAcceptSuggestion}
    />
  {/if}
</div>

<style>
  .bar-container {
    position: relative;
    width: 100%;
    height: 100%;
    background: var(--bg);
    border-bottom: 1px solid var(--border);
    display: flex;
    align-items: center;
    padding: 0 14px;
    box-sizing: border-box;
    pointer-events: auto;
  }

  .bar {
    display: flex;
    align-items: center;
    gap: 8px;
    height: clamp(32px, calc(100% - 10px), 38px);
    width: 100%;
    margin: 0;
    padding: 0 12px;
    border: 1px solid var(--border);
    border-radius: 9999px;
    background: var(--surface);
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
    width: 24px;
    height: 24px;
    border-radius: 6px;
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
    width: 14px;
    height: 14px;
  }

  .reload-btn {
    margin-right: 2px;
  }

  .lock-pill {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    width: 18px;
    height: 18px;
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
    width: 13px;
    height: 13px;
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
    font-weight: 450;
    letter-spacing: -0.005em;
    padding: 0 6px;
  }

  .bar input::placeholder {
    color: var(--muted);
  }

  .bar-clear-btn {
    all: unset;
    display: flex;
    align-items: center;
    justify-content: center;
    width: 18px;
    height: 18px;
    border-radius: 50%;
    color: var(--muted);
    font-size: 14px;
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
    width: 26px;
    height: 26px;
    border-radius: 6px;
    color: var(--muted);
    cursor: pointer;
    flex-shrink: 0;
    transition: background 0.12s ease, color 0.12s ease, transform 0.12s ease;
  }

  .tool-btn:hover {
    background: var(--hover);
    color: var(--text);
  }

  .tool-icon {
    width: 15px;
    height: 15px;
  }

  .dark-mode-btn.active-dark {
    color: #f59e0b;
    background: rgba(245, 158, 11, 0.16);
    box-shadow: 0 0 10px rgba(245, 158, 11, 0.22);
  }

  .tool-btn.reader-btn.active-reader {
    color: #38bdf8;
    background: rgba(56, 189, 248, 0.16);
  }

  .zen-btn.active-zen {
    color: var(--accent);
    background: var(--accent-soft);
  }

  .shield-btn:hover {
    color: #10b981;
  }

  .shield-icon {
    width: 14px;
    height: 14px;
  }

  .copy-clean-btn:hover,
  .copy-md-btn:hover,
  .pip-btn:hover {
    color: var(--accent);
  }

  .audio-btn {
    padding: 0 4px;
    background: color-mix(in srgb, var(--accent) 15%, transparent);
    border-radius: 4px;
  }

  .audio-waves {
    display: inline-flex;
    align-items: flex-end;
    gap: 2px;
    height: 11px;
  }

  .wave-bar {
    width: 2px;
    background: var(--accent);
    border-radius: 1px;
    animation: waveEqualizer 0.7s ease-in-out infinite alternate;
  }

  .wave-bar.b1 { height: 4px; animation-delay: 0.1s; }
  .wave-bar.b2 { height: 11px; animation-delay: 0.25s; }
  .wave-bar.b3 { height: 7px; animation-delay: 0.18s; }

  @keyframes waveEqualizer {
    0% { height: 2px; }
    100% { height: 11px; }
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

  .zen-hover-trigger {
    position: fixed;
    top: 0;
    left: 0;
    right: 0;
    height: 12px;
    z-index: 9998;
    pointer-events: auto;
  }

  .bar-container.zen-mode {
    position: fixed;
    top: 0;
    left: 0;
    right: 0;
    z-index: 9999;
    background: transparent;
    border-bottom: none;
    transform: translateY(-100%);
    opacity: 0;
    pointer-events: none;
    transition: transform 0.22s cubic-bezier(0.16, 1, 0.3, 1), opacity 0.18s ease;
  }

  .bar-container.zen-mode.zen-revealed {
    transform: translateY(0);
    opacity: 1;
    pointer-events: auto;
  }

  .bar-container.zen-mode .bar {
    max-width: min(840px, calc(100vw - 32px));
    margin: 6px auto 0;
    border-radius: 9999px;
    border: 1px solid rgba(255, 255, 255, 0.18);
    box-shadow: 0 16px 40px rgba(0, 0, 0, 0.7), 0 0 0 1px rgba(255, 255, 255, 0.08);
    background: color-mix(in srgb, var(--panel) 92%, #000);
    backdrop-filter: blur(28px);
    -webkit-backdrop-filter: blur(28px);
  }

  .bar-toast-pill {
    position: absolute;
    left: 50%;
    top: 50%;
    transform: translate(-50%, -50%);
    height: 24px;
    max-width: min(480px, calc(100vw - 220px));
    padding: 0 12px;
    display: inline-flex;
    align-items: center;
    gap: 7px;
    background: color-mix(in srgb, var(--panel) 94%, #000);
    border: 1px solid var(--border);
    border-radius: 9999px;
    box-shadow: 0 4px 18px rgba(0, 0, 0, 0.45), 0 0 0 1px rgba(255, 255, 255, 0.08);
    backdrop-filter: blur(20px);
    -webkit-backdrop-filter: blur(20px);
    z-index: 1000;
    pointer-events: none;
    animation: barPillEnter 0.16s cubic-bezier(0.16, 1, 0.3, 1) forwards;
  }

  @keyframes barPillEnter {
    from {
      opacity: 0;
      transform: translate(-50%, calc(-50% - 5px)) scale(0.96);
    }
    to {
      opacity: 1;
      transform: translate(-50%, -50%) scale(1);
    }
  }

  .bar-toast-dot {
    width: 6px;
    height: 6px;
    border-radius: 50%;
    background: #10b981;
    box-shadow: 0 0 7px #10b981;
    flex-shrink: 0;
  }

  .bar-toast-text {
    font-size: 11.5px;
    color: var(--text);
    font-weight: 500;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .bar-tab-hud-pill {
    position: absolute;
    left: 50%;
    top: 50%;
    transform: translate(-50%, -50%);
    height: 24px;
    max-width: min(480px, calc(100vw - 220px));
    padding: 0 10px 0 4px;
    display: inline-flex;
    align-items: center;
    gap: 7px;
    background: color-mix(in srgb, var(--panel) 94%, #000);
    border: 1px solid var(--border);
    border-radius: 9999px;
    box-shadow: 0 4px 18px rgba(0, 0, 0, 0.45), 0 0 0 1px rgba(255, 255, 255, 0.08);
    backdrop-filter: blur(20px);
    -webkit-backdrop-filter: blur(20px);
    z-index: 1000;
    pointer-events: none;
    animation: barPillEnter 0.16s cubic-bezier(0.16, 1, 0.3, 1) forwards;
  }

  .tab-hud-badge {
    background: var(--surface-2);
    border: 1px solid var(--border);
    border-radius: 9999px;
    padding: 1px 7px;
    font-size: 10px;
    font-weight: 600;
    color: var(--accent);
  }

  .tab-hud-title {
    font-size: 11px;
    color: var(--text);
    font-weight: 500;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
</style>
