<script lang="ts">
  import type { Suggestion, SuggestionKind } from '../ipc';

  interface Props {
    suggestions: Suggestion[];
    selected: number;
    variant?: 'start' | 'docked';
    onAccept: (item: Suggestion) => void;
  }

  let {
    suggestions,
    selected,
    variant = 'docked',
    onAccept,
  }: Props = $props();

  function label(kind: SuggestionKind): string {
    switch (kind) {
      case 'search':
        return 'Search';
      case 'tab':
        return 'Tab';
      case 'history':
        return 'History';
      case 'bookmark':
        return 'Saved';
      case 'command':
        return 'Peek';
    }
  }
</script>

{#if suggestions.length > 0}
  <div
    class={variant === 'start' ? 'start-suggestions' : 'docked-suggestions'}
    role="listbox"
  >
    {#each suggestions as item, index (item.kind + ':' + (item.url ?? item.text))}
      <button
        class={variant === 'start' ? 'suggest-item' : 'item'}
        class:selected={index === selected}
        role="option"
        aria-selected={index === selected}
        onmousedown={(event) => {
          event.preventDefault();
          onAccept(item);
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

<style>
  /* Start Page Suggestions */
  .start-suggestions {
    position: absolute;
    top: calc(100% + 8px);
    left: 0;
    right: 0;
    max-height: 280px;
    overflow-y: auto;
    background: var(--surface);
    border: 1px solid var(--border);
    border-radius: var(--radius);
    box-shadow: var(--shadow-lg);
    padding: 6px;
    display: flex;
    flex-direction: column;
    gap: 3px;
    z-index: 100;
  }

  .suggest-item {
    all: unset;
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 8px 10px;
    border-radius: 7px;
    font-size: 13px;
    color: var(--text);
    cursor: pointer;
    transition: background 0.12s ease;
  }

  .suggest-item:hover,
  .suggest-item.selected {
    background: var(--hover);
  }

  /* Docked Bar Suggestions */
  .docked-suggestions {
    position: absolute;
    top: calc(100% + 4px);
    left: 0;
    right: 0;
    max-height: 280px;
    overflow-y: auto;
    background: var(--surface);
    border: 1px solid var(--border);
    border-radius: var(--radius);
    box-shadow: var(--shadow-lg);
    padding: 4px;
    display: flex;
    flex-direction: column;
    gap: 2px;
    z-index: 100;
  }

  .item {
    all: unset;
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 6px 10px;
    border-radius: 6px;
    font-size: 12.5px;
    color: var(--text);
    cursor: pointer;
    transition: background 0.12s ease;
  }

  .item:hover,
  .item.selected {
    background: var(--hover);
  }

  /* Badges */
  .badge {
    padding: 2px 6px;
    border-radius: 4px;
    font-size: 10px;
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 0.04em;
    flex-shrink: 0;
  }

  .badge.search {
    background: rgba(37, 99, 235, 0.14);
    color: var(--accent);
  }

  .badge.tab {
    background: rgba(16, 185, 129, 0.14);
    color: #10b981;
  }

  .badge.history {
    background: rgba(100, 116, 139, 0.18);
    color: var(--muted);
  }

  .badge.bookmark {
    background: rgba(168, 85, 247, 0.16);
    color: #a855f7;
  }

  .badge.command {
    background: rgba(245, 158, 11, 0.16);
    color: #f59e0b;
  }

  .badge.url {
    background: rgba(59, 130, 246, 0.16);
    color: #3b82f6;
  }

  .text {
    flex: 1;
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .detail {
    font-size: 11px;
    color: var(--muted);
    flex-shrink: 0;
    max-width: 40%;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
</style>
