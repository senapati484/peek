export type EdgeSide = 'left' | 'right';

export type ExtensionSummary = {
  id: string;
  name: string;
  version: string;
  description: string;
  enabled: boolean;
  fullySupported: boolean;
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
};

export type PinnedSite = {
  id: number;
  url: string;
  title: string;
  favicon: string | null;
  position: number;
};

export type SuggestionKind = 'history' | 'tab' | 'bookmark' | 'search';

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
  | { type: 'toast'; message: string };

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
