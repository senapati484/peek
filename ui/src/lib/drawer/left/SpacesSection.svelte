<script lang="ts">
  import type { TabInfo } from '../../ipc';

  type SpaceId = 'personal' | 'work' | 'dev' | 'research';
  type SpaceInfo = {
    id: SpaceId;
    name: string;
    icon: string;
    color: string;
    desc: string;
  };

  interface Props {
    SPACES: SpaceInfo[];
    tabs: TabInfo[];
    tabSpaceMap: Record<number, SpaceId>;
    activeSpace: 'all' | SpaceId;
    onSetSpace: (s: 'all' | SpaceId) => void;
    onSelectSection: (section: 'tabs') => void;
  }

  let {
    SPACES,
    tabs,
    tabSpaceMap,
    activeSpace,
    onSetSpace,
    onSelectSection,
  }: Props = $props();
</script>

<div class="spaces-view-header">
  <span class="spaces-view-title">Contextual Workspaces</span>
  <span class="spaces-view-desc">Switch contexts or focus on a specific project</span>
</div>

<div class="body spaces-body">
  <div class="spaces-grid">
    {#each SPACES as sp}
      {@const count = tabs.filter(
        (t) => (tabSpaceMap[t.id] || 'personal') === sp.id,
      ).length}
      {@const activeInSpace = tabs.some(
        (t) => t.active && (tabSpaceMap[t.id] || 'personal') === sp.id,
      )}
      <div class="space-card" class:selected={activeSpace === sp.id}>
        <div class="space-card-header">
          <span
            class="space-icon-box"
            style="background: {sp.color}18; color: {sp.color}"
          >
            {sp.icon}
          </span>
          <div class="space-card-text">
            <div class="space-name-row">
              <span class="space-card-name">{sp.name}</span>
              {#if activeSpace === sp.id}
                <span class="space-current-badge">Active</span>
              {/if}
              {#if activeInSpace}
                <span
                  class="space-live-indicator"
                  title="Current tab belongs here"
                ></span>
              {/if}
            </div>
            <span class="space-card-desc">{sp.desc}</span>
          </div>
        </div>

        <div class="space-card-footer">
          <span class="space-tab-count">{count} {count === 1 ? 'tab' : 'tabs'}</span>
          <button
            class="space-select-btn"
            class:active-btn={activeSpace === sp.id}
            type="button"
            onclick={() => {
              onSetSpace(sp.id);
              onSelectSection('tabs');
            }}
          >
            {activeSpace === sp.id ? 'Viewing Tabs' : 'Switch Space'}
          </button>
        </div>
      </div>
    {/each}
  </div>
</div>

<footer class="drawer-footer">
  <button
    class="clear-all-btn"
    type="button"
    onclick={() => {
      onSetSpace('all');
      onSelectSection('tabs');
    }}
  >
    Show All Tabs Across Spaces
  </button>
</footer>

<style>
  .spaces-view-header {
    padding: 14px 16px 8px;
    display: flex;
    flex-direction: column;
    gap: 2px;
    border-bottom: 1px solid var(--border-subtle);
  }

  .spaces-view-title {
    font-size: 13.5px;
    font-weight: 600;
    color: var(--text);
  }

  .spaces-view-desc {
    font-size: 11.5px;
    color: var(--muted);
  }

  .body.spaces-body {
    flex: 1;
    padding: 12px;
    overflow-y: auto;
  }

  .spaces-grid {
    display: flex;
    flex-direction: column;
    gap: 10px;
  }

  .space-card {
    border: 1px solid var(--border);
    border-radius: 12px;
    background: var(--surface);
    padding: 12px;
    display: flex;
    flex-direction: column;
    gap: 10px;
    transition: all 0.15s ease;
  }

  .space-card:hover {
    border-color: rgba(255, 255, 255, 0.16);
    background: var(--hover);
  }

  .space-card.selected {
    border-color: var(--accent);
    box-shadow: 0 0 0 1px var(--accent-soft);
  }

  .space-card-header {
    display: flex;
    align-items: flex-start;
    gap: 10px;
  }

  .space-icon-box {
    width: 32px;
    height: 32px;
    border-radius: 8px;
    display: flex;
    align-items: center;
    justify-content: center;
    font-size: 15px;
    flex-shrink: 0;
  }

  .space-card-text {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
    gap: 2px;
  }

  .space-name-row {
    display: flex;
    align-items: center;
    gap: 6px;
  }

  .space-card-name {
    font-size: 13px;
    font-weight: 600;
    color: var(--text);
  }

  .space-current-badge {
    background: var(--accent-soft);
    color: var(--accent);
    font-size: 9.5px;
    font-weight: 600;
    text-transform: uppercase;
    padding: 1px 5px;
    border-radius: 9999px;
  }

  .space-live-indicator {
    width: 6px;
    height: 6px;
    border-radius: 50%;
    background: #10b981;
  }

  .space-card-desc {
    font-size: 11px;
    color: var(--muted);
  }

  .space-card-footer {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding-top: 4px;
    border-top: 1px solid var(--border-subtle);
  }

  .space-tab-count {
    font-size: 11px;
    color: var(--muted);
  }

  .space-select-btn {
    all: unset;
    cursor: pointer;
    padding: 3px 10px;
    border-radius: 6px;
    font-size: 11px;
    font-weight: 500;
    color: var(--text);
    background: rgba(255, 255, 255, 0.06);
    border: 1px solid rgba(255, 255, 255, 0.08);
    transition: all 0.12s ease;
  }

  .space-select-btn:hover {
    background: rgba(255, 255, 255, 0.12);
  }

  .space-select-btn.active-btn {
    background: var(--accent);
    color: #ffffff;
    border-color: var(--accent);
  }

  .drawer-footer {
    padding: 10px 12px;
    border-top: 1px solid var(--border);
    flex-shrink: 0;
  }

  .clear-all-btn {
    all: unset;
    cursor: pointer;
    display: block;
    width: 100%;
    text-align: center;
    padding: 6px;
    border-radius: 6px;
    font-size: 11.5px;
    color: var(--accent);
    background: var(--accent-soft);
    font-weight: 500;
    box-sizing: border-box;
    transition: all 0.12s ease;
  }

  .clear-all-btn:hover {
    background: color-mix(in srgb, var(--accent) 25%, transparent);
  }
</style>
