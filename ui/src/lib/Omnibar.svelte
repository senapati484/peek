<script lang="ts">
  import { onMount } from 'svelte';
  import {
    displayUrl,
    isBlank,
    onEvent,
    requestState,
    send,
    stripTrackingParams,
    type EdgeSide,
    type PeekEvent,
    type PinnedSite,
    type Suggestion,
    type SuggestionKind,
    type TabInfo,
  } from './ipc';
  import StartPage from './omnibar/StartPage.svelte';
  import DockedTopBar from './omnibar/DockedTopBar.svelte';
  import { cleanDomain } from './utils';

  interface CommandItem {
    id: string;
    title: string;
    action: () => void;
    shortcut?: string;
  }

  let input = $state('');
  let activeUrl = $state<string | null>(null);
  let activeTitle = $state<string | null>(null);
  let loading = $state(false);
  let canGoBack = $state(false);
  let canGoForward = $state(false);
  let suggestions = $state<Suggestion[]>([]);
  let selected = $state(-1);
  let focused = $state(false);
  let inputEl = $state<HTMLInputElement | undefined>();
  let suggestTimer: ReturnType<typeof setTimeout> | undefined;

  let pinnedSites = $state<PinnedSite[]>([]);
  let searchEngine = $state('Google');
  let activeProfile = $state('default');

  let isZenMode = $state(false);
  let zenHovered = $state(false);
  let readerModeActive = $state(false);
  let invertDarkActive = $state(false);
  let activeAudible = $state(false);

  let toastMessage = $state<string | null>(null);
  let toastTimer: ReturnType<typeof setTimeout> | undefined;

  function copyCleanLink() {
    if (!activeUrl || isBlank(activeUrl)) return;
    const clean = stripTrackingParams(activeUrl);
    try {
      navigator.clipboard?.writeText(clean);
    } catch {}
    send({ type: 'copyCleanUrl', url: activeUrl });
    showToast('Clean URL copied to clipboard');
  }

  function copyMarkdownLink() {
    if (!activeUrl || isBlank(activeUrl)) return;
    const title = activeTitle || cleanDomain(activeUrl);
    const md = `[${title}](${activeUrl})`;
    try {
      navigator.clipboard?.writeText(md);
    } catch {}
    send({ type: 'copyMarkdownLink' });
    showToast('Markdown link copied to clipboard');
  }

  let tabsList = $state<TabInfo[]>([]);
  let showTabCyclingHud = $state(false);
  let hudTabIndex = $state(1);
  let hudTabTotal = $state(1);
  let hudTabTitle = $state('');
  let hudTimer: ReturnType<typeof setTimeout> | undefined;

  let drawerOpen = $state(false);
  let drawerSide = $state<EdgeSide>('right');

  let isStartPage = $derived(
    isBlank(activeUrl ?? '') ||
    activeUrl === 'about:blank' ||
    (activeUrl?.startsWith('peek://') ?? false),
  );

  let insecure = $derived(
    !!activeUrl &&
    !isStartPage &&
    activeUrl.startsWith('http://'),
  );

  let isCurrentUrlPinned = $derived(
    !!activeUrl &&
    !isStartPage &&
    pinnedSites.some((p) => p.url === activeUrl),
  );

  function showToast(msg: string) {
    toastMessage = msg;
    if (toastTimer) clearTimeout(toastTimer);
    toastTimer = setTimeout(() => {
      toastMessage = null;
    }, 2400);
  }

  function triggerTabCycle(forward: boolean) {
    if (tabsList.length <= 1) return;
    const currentIdx = tabsList.findIndex((t) => t.active);
    const nextIdx = forward
      ? (currentIdx + 1) % tabsList.length
      : (currentIdx - 1 + tabsList.length) % tabsList.length;
    const target = tabsList[nextIdx];
    if (target) {
      send({ type: 'switchTab', id: target.id });
      hudTabIndex = nextIdx + 1;
      hudTabTotal = tabsList.length;
      hudTabTitle = target.title || displayUrl(target.url);
      showTabCyclingHud = true;
      if (hudTimer) clearTimeout(hudTimer);
      hudTimer = setTimeout(() => {
        showTabCyclingHud = false;
      }, 1600);
    }
  }

  let wheelAccumulatorX = 0;
  let lastWheelTime = 0;

  function onBarWheel(e: WheelEvent) {
    const now = Date.now();
    if (now - lastWheelTime > 400) wheelAccumulatorX = 0;
    lastWheelTime = now;
    wheelAccumulatorX += e.deltaX;
    const THRESHOLD = 35;
    if (wheelAccumulatorX > THRESHOLD) {
      triggerTabCycle(true);
      wheelAccumulatorX = 0;
    } else if (wheelAccumulatorX < -THRESHOLD) {
      triggerTabCycle(false);
      wheelAccumulatorX = 0;
    }
  }

  const COMMANDS: Suggestion[] = [
    { kind: 'command', text: '> New Tab', detail: '⌘T • Open a fresh tab', url: null, tabId: null },
    { kind: 'command', text: '> Close Tab', detail: '⌘W • Close the active tab', url: null, tabId: null },
    { kind: 'command', text: '> Reload Page', detail: '⌘R • Refresh current page', url: null, tabId: null },
    { kind: 'command', text: '> Open Tabs Drawer', detail: '⌘⇧[ • View vertical tabs list', url: null, tabId: null },
    { kind: 'command', text: '> Open Settings', detail: '⌘⇧] • Manage settings & tools', url: null, tabId: null },
    { kind: 'command', text: '> Smart Night Mode', detail: '⌘⇧D • Soft high-contrast dark theme', url: null, tabId: null },
    { kind: 'command', text: '> Toggle Reader Mode', detail: '⌘⇧R • Clean distraction-free view', url: null, tabId: null },
    { kind: 'command', text: '> Zen Mode (Ghost Canvas)', detail: '⌘⇧F • Fullscreen edge-to-edge', url: null, tabId: null },
    { kind: 'command', text: '> Copy Clean URL', detail: '⌘⇧C • Strip tracking params', url: null, tabId: null },
    { kind: 'command', text: '> Copy Markdown Link', detail: '⌘⌥C • Copy as [Title](URL)', url: null, tabId: null },
    { kind: 'command', text: '> Open Profiles', detail: 'Manage isolated browsing profiles', url: null, tabId: null },
    { kind: 'command', text: '> Open History', detail: 'Browse recent web history', url: null, tabId: null },
    { kind: 'command', text: '> Open Bookmarks', detail: 'View saved sites', url: null, tabId: null },
    { kind: 'command', text: '> Open Scratchpad', detail: 'Quick research notes', url: null, tabId: null },
    { kind: 'command', text: '> Clear Cookies & Storage', detail: 'Wipe current site local data', url: null, tabId: null },
  ];

  function runCommandAction(name: string) {
    switch (name) {
      case '> New Tab': send({ type: 'newTab' }); break;
      case '> Close Tab':
        const activeTab = tabsList.find((t) => t.active);
        if (activeTab) send({ type: 'closeTab', id: activeTab.id });
        break;
      case '> Reload Page': send({ type: 'reload' }); break;
      case '> Open Tabs Drawer': send({ type: 'setDrawer', open: true, side: 'left', section: 'tabs' }); break;
      case '> Open Settings': send({ type: 'setDrawer', open: true, side: 'right', section: 'general' }); break;
      case '> Smart Night Mode': send({ type: 'toggleInvertDark' }); break;
      case '> Toggle Reader Mode': send({ type: 'toggleReaderMode' }); break;
      case '> Zen Mode (Ghost Canvas)': send({ type: 'toggleZenMode' }); break;
      case '> Copy Clean URL': copyCleanLink(); break;
      case '> Copy Markdown Link': copyMarkdownLink(); break;
      case '> Open Profiles': send({ type: 'setDrawer', open: true, side: 'right', section: 'profiles' }); break;
      case '> Open History': send({ type: 'setDrawer', open: true, side: 'left', section: 'history' }); break;
      case '> Open Bookmarks': send({ type: 'setDrawer', open: true, side: 'left', section: 'bookmarks' }); break;
      case '> Open Scratchpad': send({ type: 'setDrawer', open: true, side: 'left', section: 'notes' }); break;
      case '> Clear Cookies & Storage': send({ type: 'clearCookies' }); break;
    }
  }

  function onInput() {
    send({ type: 'setOmnibarOpen', open: true });
    if (suggestTimer) clearTimeout(suggestTimer);
    const raw = input;
    if (!raw.trim()) {
      suggestions = [];
      return;
    }

    if (raw.startsWith('>')) {
      const query = raw.slice(1).trim().toLowerCase();
      const matched = !query
        ? COMMANDS
        : COMMANDS.filter((c) =>
            c.text.toLowerCase().includes(query) ||
            c.detail?.toLowerCase().includes(query),
          );
      suggestions = matched.slice(0, 8);
      selected = suggestions.length > 0 ? 0 : -1;
      return;
    }

    suggestTimer = setTimeout(() => {
      send({ type: 'suggest', query: raw });
    }, 80);
  }

  function accept(item: Suggestion) {
    if (item.kind === 'command') {
      runCommandAction(item.text);
      focused = false;
      suggestions = [];
      send({ type: 'setOmnibarOpen', open: false });
      send({ type: 'dismissSuggestions' });
      return;
    }

    if (item.url) {
      send({ type: 'navigate', input: item.url });
    } else {
      send({ type: 'navigate', input: item.text });
    }
    focused = false;
    suggestions = [];
    send({ type: 'setOmnibarOpen', open: false });
    send({ type: 'dismissSuggestions' });
  }

  function commit() {
    const raw = input.trim();
    if (!raw) return;

    if (selected >= 0 && selected < suggestions.length) {
      accept(suggestions[selected]);
      return;
    }

    if (raw.startsWith('>')) {
      const best = suggestions[0];
      if (best) accept(best);
      return;
    }

    send({ type: 'navigate', input: raw });
    focused = false;
    suggestions = [];
    send({ type: 'setOmnibarOpen', open: false });
    send({ type: 'dismissSuggestions' });
  }

  function onKeydown(event: KeyboardEvent) {
    if (event.key === 'ArrowDown') {
      event.preventDefault();
      if (!suggestions.length) return;
      selected = (selected + 1) % suggestions.length;
    } else if (event.key === 'ArrowUp') {
      event.preventDefault();
      if (!suggestions.length) return;
      selected = (selected - 1 + suggestions.length) % suggestions.length;
    } else if (event.key === 'Enter') {
      event.preventDefault();
      commit();
    } else if (event.key === 'Escape') {
      event.preventDefault();
      focused = false;
      suggestions = [];
      send({ type: 'setOmnibarOpen', open: false });
      send({ type: 'dismissSuggestions' });
      send({ type: 'focusPage' });
    }
  }

  function togglePinCurrent() {
    if (!activeUrl || isBlank(activeUrl)) return;
    if (isCurrentUrlPinned) {
      send({ type: 'unpinSite', url: activeUrl });
      showToast('Website unpinned from Start Page');
    } else {
      send({
        type: 'pinSite',
        url: activeUrl,
        title: activeTitle ?? undefined,
      });
      showToast('Website pinned to Start Page');
    }
  }

  function handleOpenPin(url: string) {
    send({ type: 'navigate', input: url });
  }

  function handleUnpin(url: string, e: MouseEvent) {
    e.stopPropagation();
    send({ type: 'unpinSite', url });
    showToast('Website unpinned');
  }

  function handleAddPin(url: string, title?: string) {
    send({
      type: 'pinSite',
      url,
      title: title || undefined,
    });
    showToast('Website pinned');
  }

  function triggerPinPreview(url: string, el: HTMLElement) {
    const rect = el.getBoundingClientRect();
    send({
      type: 'previewLink',
      url,
      x: Math.round(rect.left + rect.width / 2),
      y: Math.round(rect.top),
      width: 480,
      height: 320,
    });
  }

  function dismissPinPreview() {
    send({ type: 'hidePreview' });
  }

  function openDrawer(side: EdgeSide, section?: string) {
    send({ type: 'setDrawer', open: true, side, section });
  }

  function handleEvent(event: PeekEvent): void {
    switch (event.type) {
      case 'urlChanged':
        activeUrl = event.url;
        if (!focused) {
          input = isBlank(event.url) ? '' : displayUrl(event.url);
        }
        break;
      case 'titleChanged':
        activeTitle = event.title;
        break;
      case 'loading':
        loading = event.loading;
        canGoBack = event.canGoBack;
        canGoForward = event.canGoForward;
        break;
      case 'suggestionsReady':
        if (focused && !input.startsWith('>')) {
          suggestions = event.items;
          selected = suggestions.length > 0 ? 0 : -1;
        }
        break;
      case 'pinnedSitesChanged':
        pinnedSites = event.sites;
        break;
      case 'settings':
        searchEngine = event.searchEngine || 'Google';
        break;
      case 'profilesList':
        activeProfile = event.active;
        break;
      case 'profileChanged':
        activeProfile = event.name;
        break;
      case 'zenModeState':
        isZenMode = event.active;
        break;
      case 'readerModeState':
        readerModeActive = event.active;
        break;
      case 'invertDarkState':
        invertDarkActive = event.active;
        break;
      case 'toast':
        showToast(event.message);
        break;
      case 'tabsChanged':
        tabsList = event.tabs;
        const current = event.tabs.find((t) => t.id === event.active);
        if (current) {
          activeAudible = current.audible;
        }
        break;
      case 'drawerState':
        drawerOpen = event.open;
        if (event.side) drawerSide = event.side;
        break;
    }
  }

  onMount(() => {
    onEvent(handleEvent);
    requestState();
    send({ type: 'getPinnedSites' });

    const onKey = (e: KeyboardEvent) => {
      const mod = e.metaKey || e.ctrlKey;
      const key = e.key.toLowerCase();

      if (mod && !e.shiftKey && !e.altKey) {
        if (key === 'l') {
          e.preventDefault();
          focused = true;
          send({ type: 'setOmnibarOpen', open: true });
          inputEl?.focus();
          inputEl?.select();
          return;
        }
        if (key === 'k') {
          e.preventDefault();
          focused = true;
          send({ type: 'setOmnibarOpen', open: true });
          input = '>';
          onInput();
          inputEl?.focus();
          return;
        }
        if (key === 'r') {
          e.preventDefault();
          send({ type: 'reload' });
          return;
        }
        if (key === '.') {
          e.preventDefault();
          send({ type: 'toggleZenMode' });
          return;
        }
      }

      if (mod && e.shiftKey && !e.altKey) {
        if (key === 'c') {
          e.preventDefault();
          copyCleanLink();
          return;
        }
        if (key === 'p') {
          e.preventDefault();
          send({ type: 'togglePiP' });
          return;
        }
        if (key === 'd') {
          e.preventDefault();
          send({ type: 'toggleInvertDark' });
          return;
        }
        if (key === 'r') {
          e.preventDefault();
          send({ type: 'toggleReaderMode' });
          return;
        }
        if (key === 'm') {
          e.preventDefault();
          send({ type: 'toggleMuteTab' });
          return;
        }
        if (key === '[' || key === '{') {
          e.preventDefault();
          openDrawer('left', 'tabs');
          return;
        }
        if (key === ']' || key === '}') {
          e.preventDefault();
          openDrawer('right', 'general');
          return;
        }
      }

      if (mod && e.altKey && !e.shiftKey) {
        if (key === 'c') {
          e.preventDefault();
          copyMarkdownLink();
          return;
        }
      }
    };
    window.addEventListener('keydown', onKey);
    return () => {
      window.removeEventListener('keydown', onKey);
    };
  });
</script>

<div
  class="root"
  class:start-page={isStartPage}
  class:open={focused}
  role="presentation"
>
  {#if isStartPage}
    <StartPage
      {searchEngine}
      {activeProfile}
      {drawerOpen}
      {drawerSide}
      {pinnedSites}
      bind:input
      bind:focused
      {suggestions}
      {selected}
      bind:inputEl
      {onInput}
      {onKeydown}
      onFocus={() => {
        focused = true;
        send({ type: 'setOmnibarOpen', open: true });
      }}
      onAcceptSuggestion={accept}
      onOpenPin={handleOpenPin}
      onUnpin={handleUnpin}
      onAddPin={handleAddPin}
      {triggerPinPreview}
      {dismissPinPreview}
      onOpenDrawer={openDrawer}
    />
  {:else}
    <DockedTopBar
      {isZenMode}
      bind:zenHovered
      bind:focused
      {loading}
      {insecure}
      bind:input
      {activeUrl}
      bind:inputEl
      {invertDarkActive}
      {readerModeActive}
      {activeAudible}
      {isCurrentUrlPinned}
      {activeProfile}
      {toastMessage}
      {showTabCyclingHud}
      {hudTabIndex}
      {hudTabTotal}
      {hudTabTitle}
      {suggestions}
      {selected}
      {onInput}
      {onKeydown}
      onFocus={() => {
        focused = true;
        send({ type: 'setOmnibarOpen', open: true });
      }}
      {onBarWheel}
      onAcceptSuggestion={accept}
      onTogglePin={togglePinCurrent}
      onCopyCleanUrl={copyCleanLink}
      onCopyMarkdownLink={copyMarkdownLink}
      onOpenDrawer={openDrawer}
    />
  {/if}
</div>

<style>
  .root {
    width: 100%;
    height: 100%;
    position: relative;
    box-sizing: border-box;
    user-select: none;
  }

  .root.start-page {
    overflow-y: auto;
    background: var(--bg);
  }
</style>
