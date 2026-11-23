<script lang="ts">
  import { displayUrl, send, type HistoryItem } from '../../ipc';
  import { cleanDomain, domainInitial, formatRelativeTime } from '../../utils';

  interface Props {
    historyEntries: HistoryItem[];
    onClose: () => void;
  }

  let { historyEntries = $bindable([]), onClose }: Props = $props();

  let filterText = $state('');

  let filteredHistory = $derived(
    filterText.trim()
      ? historyEntries.filter((h) =>
          (h.title || h.url).toLowerCase().includes(filterText.toLowerCase()),
        )
      : historyEntries,
  );
</script>

<div class="filter-wrap">
  <div class="filter-box">
    <svg viewBox="0 0 16 16" fill="none" stroke="currentColor" class="filter-icon">
      <circle cx="7" cy="7" r="4.5" stroke-width="1.6" />
      <path d="M10.5 10.5L14 14" stroke-width="1.6" stroke-linecap="round" />
    </svg>
    <input
      type="search"
      placeholder="Search browsing history..."
      bind:value={filterText}
      class="filter-input"
    />
    {#if filterText}
      <button class="clear-filter" onclick={() => (filterText = '')}>×</button>
    {/if}
  </div>
</div>

<div class="body history-body">
  {#if filteredHistory.length === 0}
    <div class="empty-state">
      <span class="empty-icon">📜</span>
      <span class="empty-text">
        {filterText
          ? `No history matching "${filterText}"`
          : 'No browsing history yet'}
      </span>
    </div>
  {:else}
    <ul class="history-list">
      {#each filteredHistory as item (item.url + ':' + item.lastVisit)}
        <li class="history-item">
          <button
            class="history-row"
            type="button"
            onclick={() => {
              send({ type: 'navigate', input: item.url });
              onClose();
            }}
            title={`${item.title || item.url}\n${item.url}`}
          >
            <span class="favicon-initial">{domainInitial(item.url)}</span>
            <div class="history-info">
              <span class="history-title">{item.title || displayUrl(item.url)}</span>
              <div class="history-sub">
                <span class="history-host">{cleanDomain(item.url)}</span>
                {#if item.lastVisit}
                  <span class="history-dot">·</span>
                  <span class="history-time">{formatRelativeTime(item.lastVisit)}</span>
                {/if}
              </div>
            </div>
          </button>
          <button
            class="history-delete-btn"
            title="Remove from history"
            type="button"
            onclick={(e) => {
              e.stopPropagation();
              send({ type: 'deleteHistoryItem', url: item.url });
              historyEntries = historyEntries.filter((h) => h.url !== item.url);
            }}
          >
            ×
          </button>
        </li>
      {/each}
    </ul>
  {/if}
</div>

<footer class="tabs-footer history-footer">
  <button
    class="clear-history-btn"
    type="button"
    onclick={() => {
      if (confirm('Clear all browsing history?')) {
        send({ type: 'clearHistory' });
        historyEntries = [];
      }
    }}
  >
    Clear Browsing History
  </button>
</footer>

<style>
  .filter-wrap {
    padding: 8px 12px;
    border-bottom: 1px solid var(--border);
  }

  .filter-box {
    display: flex;
    align-items: center;
    gap: 7px;
    padding: 4px 8px;
    border: 1px solid var(--border);
    border-radius: var(--radius-sm);
    background: var(--surface);
    transition: border-color 0.15s ease;
  }

  .filter-box:focus-within {
    border-color: var(--accent);
  }

  .filter-icon {
    width: 13px;
    height: 13px;
    color: var(--muted);
    flex-shrink: 0;
  }

  .filter-input {
    flex: 1;
    border: none;
    outline: none;
    background: transparent;
    color: var(--text);
    font-size: 12px;
  }

  .clear-filter {
    color: var(--muted);
    font-size: 14px;
    line-height: 1;
    cursor: pointer;
    background: none;
    border: none;
  }

  .body.history-body {
    flex: 1;
    overflow-y: auto;
    padding: 8px;
  }

  .history-list {
    list-style: none;
    margin: 0;
    padding: 0;
    display: flex;
    flex-direction: column;
    gap: 2px;
  }

  .history-item {
    display: flex;
    align-items: center;
    border-radius: 8px;
    transition: background 0.12s ease;
    position: relative;
  }

  .history-item:hover {
    background: var(--hover);
  }

  .history-row {
    all: unset;
    flex: 1;
    min-width: 0;
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 6px 8px;
    cursor: pointer;
  }

  .favicon-initial {
    flex: 0 0 15px;
    width: 15px;
    height: 15px;
    border-radius: 3px;
    background: var(--surface-2);
    color: var(--muted);
    font-size: 9px;
    font-weight: 700;
    display: flex;
    align-items: center;
    justify-content: center;
  }

  .history-info {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
    gap: 2px;
  }

  .history-title {
    font-size: 12.5px;
    font-weight: 500;
    color: var(--text);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .history-sub {
    display: flex;
    align-items: center;
    gap: 5px;
    font-size: 11px;
    color: var(--muted);
  }

  .history-host {
    font-size: 11px;
    color: var(--muted);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .history-dot {
    opacity: 0.5;
  }

  .history-time {
    color: var(--muted);
    font-size: 10.5px;
    flex-shrink: 0;
  }

  .history-delete-btn {
    all: unset;
    opacity: 0;
    width: 22px;
    height: 22px;
    border-radius: 50%;
    display: flex;
    align-items: center;
    justify-content: center;
    color: var(--muted);
    font-size: 14px;
    cursor: pointer;
    margin-right: 6px;
    transition: all 0.14s ease;
  }

  .history-item:hover .history-delete-btn {
    opacity: 1;
  }

  .history-delete-btn:hover {
    background: #ef4444;
    color: #fff;
  }

  .empty-state {
    display: flex;
    flex-direction: column;
    align-items: center;
    padding: 32px 16px;
    color: var(--muted);
    font-size: 13px;
    gap: 8px;
    text-align: center;
  }

  .empty-icon {
    font-size: 24px;
    margin-bottom: 6px;
  }

  .empty-text {
    font-size: 12px;
  }

  .tabs-footer {
    padding: 10px 12px;
    border-top: 1px solid var(--border);
    flex-shrink: 0;
  }

  .clear-history-btn {
    all: unset;
    width: 100%;
    padding: 6px;
    border-radius: 6px;
    font-size: 12px;
    font-weight: 500;
    color: #ef4444;
    text-align: center;
    cursor: pointer;
    box-sizing: border-box;
    transition: background 0.12s ease;
  }

  .clear-history-btn:hover {
    background: rgba(239, 68, 68, 0.12);
  }
</style>
