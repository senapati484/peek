export type EdgeSide = 'left' | 'right';

export type ExtensionSummary = {
  id: string;
  name: string;
  version: string;
  description: string;
  enabled: boolean;
  fullySupported: boolean;
};

export type TabGroup = {
  id: string;
  name: string;
  color: string;
  collapsed: boolean;
};

export type UserScriptSummary = {
  id: string;
  name: string;
  version: string;
  description: string;
  author: string;
  matchPatterns: string[];
  runAt: string;
  enabled: boolean;
  code: string;
};

export type TabInfo = {
  id: number;
  url: string;
  title: string;
  favicon: string | null;
  discarded: boolean;
  loading: boolean;
  audible: boolean;
  pinned: boolean;
  active: boolean;
  groupId?: string | null;
};

export type PinnedSite = {
  id: number;
  url: string;
  title: string;
  favicon: string | null;
  position: number;
};

export type HistoryItem = {
  url: string;
  title: string;
  visitCount: number;
  lastVisit: number;
};

export type BookmarkItem = {
  id: number;
  url: string;
  title: string;
  favicon: string | null;
  createdAt: number;
};

export type SuggestionKind = 'history' | 'tab' | 'bookmark' | 'search' | 'command';

export type Suggestion = {
  kind: SuggestionKind;
  text: string;
  detail: string | null;
  url: string | null;
  tabId: number | null;
};

export type PeekEvent =
  | { type: 'tabsChanged'; tabs: TabInfo[]; active: number | null }
  | { type: 'activeTabChanged'; id: number }
  | {
    type: 'loading';
    id: number;
    url: string;
    loading: boolean;
    canGoBack: boolean;
    canGoForward: boolean;
  }
  | { type: 'titleChanged'; id: number; title: string }
  | { type: 'urlChanged'; id: number; url: string }
  | { type: 'suggestionsReady'; query: string; items: Suggestion[] }
  | { type: 'focusOmnibar' }
  | { type: 'drawerState'; open: boolean; side?: EdgeSide; section?: string }
  | { type: 'adblockCount'; blocked: number; allowedOnSite: boolean; host: string | null }
  | { type: 'profileChanged'; name: string }
  | { type: 'profilesList'; profiles: string[]; active: string }
  | { type: 'cookiesCount'; count: number }
  | { type: 'extensionsList'; extensions: ExtensionSummary[] }
  | {
    type: 'settings';
    maxLive: number;
    idleDiscardMinutes: number;
    hoverDelayMs: number;
    drawerSide: EdgeSide;
    edgeZone: number;
    startBlank: boolean;
    searchEngine: string;
  }
  | { type: 'pinnedSitesChanged'; sites: PinnedSite[] }
  | { type: 'toast'; message: string }
  | { type: 'historyList'; entries: HistoryItem[] }
  | { type: 'bookmarksList'; bookmarks: BookmarkItem[] }
  | { type: 'findState'; open: boolean; query: string; count: number; index: number }
  | { type: 'readerModeState'; active: boolean }
  | { type: 'zenModeState'; active: boolean }
  | { type: 'invertDarkState'; active: boolean }
  | { type: 'noteLoaded'; content: string }
  | { type: 'zoomLevel'; percent: number }
  | { type: 'tabGroupsChanged'; groups: TabGroup[] }
  | { type: 'userScriptsList'; scripts: UserScriptSummary[] };

export type PeekCommand =
  | { type: 'navigate'; input: string }
  | { type: 'newTab'; url?: string | null }
  | { type: 'closeTab'; id: number }
  | { type: 'switchTab'; id: number }
  | { type: 'reorderTab'; id: number; toIndex: number }
  | { type: 'discardTab'; id: number }
  | { type: 'discardOtherTabs' }
  | { type: 'cycleTab'; forward: boolean }
  | { type: 'back' }
  | { type: 'forward' }
  | { type: 'reload' }
  | { type: 'stop' }
  | { type: 'suggest'; query: string }
  | { type: 'dismissSuggestions' }
  | { type: 'requestState' }
  | { type: 'setOmnibarOpen'; open: boolean }
  | { type: 'focusPage' }
  | { type: 'setDrawer'; open: boolean; side?: EdgeSide; section?: string }
  | { type: 'openUrl'; url: string }
  | { type: 'setSiteAdblock'; host: string; enabled: boolean }
  | { type: 'setSetting'; key: string; value: unknown }
  | { type: 'createProfile'; name: string }
  | { type: 'switchProfile'; name: string }
  | { type: 'deleteProfile'; name: string }
  | { type: 'installExtension'; path: string }
  | { type: 'toggleExtension'; id: string; enabled: boolean }
  | { type: 'uninstallExtension'; id: string }
  | { type: 'getExtensions' }
  | { type: 'pinSite'; url: string; title?: string | null; favicon?: string | null }
  | { type: 'unpinSite'; url: string }
  | { type: 'pinTab'; id: number; pinned: boolean }
  | { type: 'getPinnedSites' }
  | { type: 'clearCookies' }
  | { type: 'getCookies' }
  | { type: 'previewLink'; url: string; x: number; y: number; width: number; height: number }
  | { type: 'hidePreview' }
  | { type: 'copyCleanUrl'; url: string }
  | { type: 'toggleMuteTab'; id?: number }
  | { type: 'toggleReaderMode' }
  | { type: 'findInPage'; query: string; forward: boolean }
  | { type: 'closeFind' }
  | { type: 'getHistory'; query?: string }
  | { type: 'deleteHistoryItem'; url: string }
  | { type: 'clearHistory' }
  | { type: 'getBookmarks'; query?: string }
  | { type: 'toggleBookmark'; url: string; title: string }
  | { type: 'toggleZenMode' }
  | { type: 'toggleInvertDark' }
  | { type: 'togglePiP' }
  | { type: 'zoomIn' }
  | { type: 'zoomOut' }
  | { type: 'zoomReset' }
  | { type: 'copyMarkdownLink' }
  | { type: 'saveNote'; content: string }
  | { type: 'getNote' }
  | { type: 'saveTabGroups'; groups: TabGroup[] }
  | { type: 'setTabGroup'; tabId: number; groupId?: string | null }
  | { type: 'getUserScripts' }
  | { type: 'toggleUserScript'; id: string; enabled: boolean }
  | { type: 'saveUserScript'; name: string; code: string }
  | { type: 'deleteUserScript'; id: string }
  | { type: 'quit' };

type Bridge = {
  __peekSend?: (command: PeekCommand) => void;
  __PEEK_NONCE__?: string;
  __peekOnEvent?: (event: PeekEvent) => void;
};

const bridge = window as unknown as Bridge;

export function send(command: PeekCommand): void {
  bridge.__peekSend?.(command);
}

export function onEvent(handler: (event: PeekEvent) => void): void {
  bridge.__peekOnEvent = handler;
}

export function requestState(): void {
  send({ type: 'requestState' });
}

export function isBlank(url: string): boolean {
  if (!url) return true;
  return (
    url === 'about:blank' ||
    url.startsWith('peek://') ||
    url.endsWith('blank.html')
  );
}


export function displayUrl(url: string): string {
  if (isBlank(url)) return '';
  try {
    const parsed = new URL(url);
    const host = parsed.hostname.replace(/^www\./, '');
    const path = parsed.pathname === '/' ? '' : parsed.pathname;
    return host + path;
  } catch {
    return url.replace(/^https?:\/\//, '').replace(/^www\./, '');
  }
}

export function hostOf(url: string): string | null {
  try {
    return new URL(url).hostname;
  } catch {
    return null;
  }
}

const TRACKING_PREFIXES = [
  'utm_',
  'fbclid',
  'gclid',
  'gclsrc',
  'dclid',
  'si',
  'igshid',
  'mc_',
  'yclid',
  '_hs',
  'ref_',
];

const TRACKING_KEYS = new Set(['ref', 's', 't', 'feature', 'source']);

export function stripTrackingParams(rawUrl: string): string {
  try {
    const parsed = new URL(rawUrl);
    const toDelete: string[] = [];
    parsed.searchParams.forEach((_, key) => {
      const lower = key.toLowerCase();
      if (
        TRACKING_PREFIXES.some((p) => lower.startsWith(p)) ||
        TRACKING_KEYS.has(lower)
      ) {
        toDelete.push(key);
      }
    });
    for (const key of toDelete) {
      parsed.searchParams.delete(key);
    }
    return parsed.toString();
  } catch {
    return rawUrl;
  }
}
