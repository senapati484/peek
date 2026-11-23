/**
 * Shared utility functions across Peek UI components
 */

export function domainInitial(url: string): string {
  if (!url || url === 'about:blank') return '✦';
  try {
    const u = new URL(url);
    const h = u.hostname.replace(/^www\./, '');
    return (h[0] || '✦').toUpperCase();
  } catch {
    return '✦';
  }
}

export function cleanDomain(url: string): string {
  if (!url) return '';
  try {
    return new URL(url).hostname.replace(/^www\./, '');
  } catch {
    return url.replace(/^https?:\/\//, '').replace(/^www\./, '');
  }
}

export function formatRelativeTime(ts: number): string {
  if (!ts) return '';
  const ms = ts > 10000000000 ? ts : ts * 1000;
  const diff = Date.now() - ms;
  if (diff < 60000) return 'Just now';
  if (diff < 3600000) return `${Math.floor(diff / 60000)}m ago`;
  if (diff < 86400000) return `${Math.floor(diff / 3600000)}h ago`;
  const days = Math.floor(diff / 86400000);
  if (days === 1) return 'Yesterday';
  if (days < 7) return `${days}d ago`;
  return new Date(ms).toLocaleDateString(undefined, {
    month: 'short',
    day: 'numeric',
  });
}
