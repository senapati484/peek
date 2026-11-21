export type TabInfo = {
  id: number;
  url: string;
  title: string;
  favicon: string | null;
  discarded: boolean;
  loading: boolean;
  audible: boolean;
  active: boolean;
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
  | { type: 'drawerState'; open: boolean }
  | { type: 'adblockCount'; blocked: number; allowedOnSite: boolean; host: string | null }
  | { type: 'profileChanged'; name: string }
  | { type: 'cookiesCount'; count: number }
  | {
      type: 'settings';
      maxLive: number;
      idleDiscardMinutes: number;
      hoverDelayMs: number;
      drawerSide: 'left' | 'right';
      edgeZone: number;
      startBlank: boolean;
      searchEngine: string;
    }
  | { type: 'toast'; message: string };

export type PeekCommand =
  | { type: 'navigate'; input: string }
  | { type: 'newTab'; url?: string | null }
  | { type: 'closeTab'; id: number }
  | { type: 'switchTab'; id: number }
  | { type: 'reorderTab'; id: number; toIndex: number }
  | { type: 'back' }
  | { type: 'forward' }
  | { type: 'reload' }
  | { type: 'stop' }
  | { type: 'suggest'; query: string }
  | { type: 'dismissSuggestions' }
  | { type: 'requestState' }
  | { type: 'setOmnibarOpen'; open: boolean }
  | { type: 'focusPage' }
  | { type: 'setDrawer'; open: boolean }
  | { type: 'openUrl'; url: string }
  | { type: 'setSiteAdblock'; host: string; enabled: boolean }
  | { type: 'setSetting'; key: string; value: unknown }
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
  return url.replace(/^https:\/\//, '').replace(/^http:\/\//, '');
}

export function hostOf(url: string): string | null {
  try {
    return new URL(url).hostname;
  } catch {
    return null;
  }
}
