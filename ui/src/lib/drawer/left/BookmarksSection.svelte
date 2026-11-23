<script lang="ts">
  import { displayUrl, isBlank, send, type BookmarkItem, type TabInfo } from '../../ipc';
  import { cleanDomain, domainInitial } from '../../utils';

  interface Props {
    bookmarksList: BookmarkItem[];
    activeTab: TabInfo | undefined;
    onClose: () => void;
  }

  let { bookmarksList = $bindable([]), activeTab, onClose }: Props = $props();

  let filterText = $state('');

  let filteredBookmarks = $derived(
    filterText.trim()
      ? bookmarksList.filter((b) =>
          (b.title || b.url).toLowerCase().includes(filterText.toLowerCase()),
        )
      : bookmarksList,
  );

  let isCurrentTabBookmarked = $derived(
    activeTab && !isBlank(activeTab.url)
      ? bookmarksList.some((b) => b.url === activeTab?.url)
      : false,
  );

  function toggleCurrentBookmark(): void {
    if (!activeTab || isBlank(activeTab.url)) return;
    send({
      type: 'toggleBookmark',
      url: activeTab.url,
      title: activeTab.title || displayUrl(activeTab.url),
    });
  }
</script>

<div class="filter-wrap">
  <div class="filter-box">
    <svg viewBox="0 0 16 16" fill="none" stroke="currentColor" class="filter-icon">
      <circle cx="7" cy="7" r="4.5" stroke-width="1.6" />
      <path d="M10.5 10.5L14 14" stroke-width="1.6" stroke-linecap="round" />
    </svg>
    <input
      type="search"
      placeholder="Search saved bookmarks..."
      bind:value={filterText}
      class="filter-input"
    />
    {#if filterText}
      <button class="clear-filter" onclick={() => (filterText = '')}>×</button>
    {/if}
  </div>
</div>

<!-- Quick Bookmark Active Tab Action -->
{#if activeTab && !isBlank(activeTab.url)}
  <div class="bookmark-action-bar">
    <button
      class="bookmark-toggle-btn"
      class:is-saved={isCurrentTabBookmarked}
      type="button"
      onclick={toggleCurrentBookmark}
    >
      <span class="star-icon">{isCurrentTabBookmarked ? '★' : '☆'}</span>
      <span>
        {isCurrentTabBookmarked
          ? 'Remove Current Tab from Saved'
          : 'Bookmark Current Tab'}
      </span>
    </button>
  </div>
{/if}

<div class="body bookmarks-body">
  {#if filteredBookmarks.length === 0}
    <div class="empty-state">
      <span class="empty-icon">★</span>
      <span class="empty-text">
        {filterText
          ? `No bookmarks matching "${filterText}"`
          : 'No saved bookmarks yet'}
      </span>
    </div>
  {:else}
    <ul class="bookmarks-list">
      {#each filteredBookmarks as b (b.id)}
        <li class="bookmark-item">
          <button
            class="bookmark-row"
            type="button"
            onclick={() => {
              send({ type: 'navigate', input: b.url });
              onClose();
            }}
            title={`${b.title || b.url}\n${b.url}`}
          >
            {#if b.favicon}
              <img
                src={b.favicon}
                alt=""
                class="favicon"
                onerror={(e) => {
                  (e.currentTarget as HTMLElement).style.display = 'none';
                }}
              />
            {:else}
              <span class="favicon-initial">{domainInitial(b.url)}</span>
            {/if}
            <div class="bookmark-info">
              <span class="bookmark-title">{b.title || displayUrl(b.url)}</span>
              <span class="bookmark-host">{cleanDomain(b.url)}</span>
            </div>
          </button>
          <button
            class="bookmark-remove-btn"
            title="Remove bookmark"
            type="button"
            onclick={(e) => {
              e.stopPropagation();
              send({
                type: 'toggleBookmark',
                url: b.url,
                title: b.title,
              });
              bookmarksList = bookmarksList.filter((x) => x.id !== b.id);
            }}
          >
            ×
          </button>
        </li>
      {/each}
    </ul>
  {/if}
</div>

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

  .bookmark-action-bar {
    padding: 8px 12px;
    border-bottom: 1px solid var(--border-subtle);
  }

  .bookmark-toggle-btn {
    all: unset;
    display: flex;
    align-items: center;
    gap: 8px;
    width: 100%;
    padding: 6px 10px;
    border-radius: 7px;
    background: var(--surface-2);
    border: 1px solid var(--border);
    color: var(--text-secondary);
    font-size: 12px;
    font-weight: 550;
    cursor: pointer;
    transition: all 0.15s ease;
    box-sizing: border-box;
  }

  .bookmark-toggle-btn:hover {
    background: var(--hover);
    color: var(--text);
  }

  .bookmark-toggle-btn.is-saved {
    color: #a855f7;
    border-color: rgba(168, 85, 247, 0.4);
    background: rgba(168, 85, 247, 0.1);
  }

  .star-icon {
    font-size: 13px;
  }

  .body.bookmarks-body {
    flex: 1;
    overflow-y: auto;
    padding: 8px;
  }

  .bookmarks-list {
    list-style: none;
    margin: 0;
    padding: 0;
    display: flex;
    flex-direction: column;
    gap: 2px;
  }

  .bookmark-item {
    display: flex;
    align-items: center;
    border-radius: 8px;
    transition: background 0.12s ease;
    position: relative;
  }

  .bookmark-item:hover {
    background: var(--hover);
  }

  .bookmark-row {
    all: unset;
    flex: 1;
    min-width: 0;
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 6px 8px;
    cursor: pointer;
  }

  .favicon {
    flex: 0 0 15px;
    width: 15px;
    height: 15px;
    border-radius: 3px;
    object-fit: contain;
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

  .bookmark-info {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
    gap: 2px;
  }

  .bookmark-title {
    font-size: 12.5px;
    font-weight: 500;
    color: var(--text);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .bookmark-host {
    font-size: 11px;
    color: var(--muted);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .bookmark-remove-btn {
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

  .bookmark-item:hover .bookmark-remove-btn {
    opacity: 1;
  }

  .bookmark-remove-btn:hover {
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
</style>
