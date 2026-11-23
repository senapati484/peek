<script lang="ts">
  import {
    send,
    type TabGroup,
    type TabInfo,
  } from '../../ipc';
  import { domainInitial } from '../../utils';

  type SpaceId = 'personal' | 'work' | 'dev' | 'research';
  type SpaceInfo = {
    id: SpaceId;
    name: string;
    icon: string;
    color: string;
    desc: string;
  };

  interface Props {
    tabs: TabInfo[];
    activeTabId: number | null;
    tabGroups: TabGroup[];
    activeSpace: 'all' | SpaceId;
    tabSpaceMap: Record<number, SpaceId>;
    SPACES: SpaceInfo[];
    onSetSpace: (s: 'all' | SpaceId) => void;
    onCycleSpace: (id: number) => void;
    onClose: () => void;
  }

  let {
    tabs,
    activeTabId,
    tabGroups,
    activeSpace,
    tabSpaceMap,
    SPACES,
    onSetSpace,
    onCycleSpace,
    onClose,
  }: Props = $props();

  let filterText = $state('');
  let keyboardSelectedIndex = $state(-1);
  let dragId = $state<number | null>(null);

  // Groups creation state
  let isCreatingGroup = $state(false);
  let newGroupName = $state('');
  let newGroupColor = $state('#3b82f6');
  const GROUP_PALETTE = [
    '#3b82f6',
    '#10b981',
    '#f59e0b',
    '#ef4444',
    '#8b5cf6',
    '#ec4899',
    '#06b6d4',
    '#64748b',
  ];

  let sleepingCount = $derived(tabs.filter((t) => t.discarded).length);
  let liveCount = $derived(tabs.length - sleepingCount);
  let estimatedMemorySaved = $derived(sleepingCount * 95);

  let visibleTabs = $derived(
    activeSpace === 'all'
      ? tabs
      : tabs.filter((t) => (tabSpaceMap[t.id] || 'personal') === activeSpace),
  );

  let filteredTabs = $derived(
    filterText.trim()
      ? visibleTabs.filter((t) =>
          (t.title || t.url).toLowerCase().includes(filterText.toLowerCase()),
        )
      : visibleTabs,
  );

  let keyboardSelectedTabId = $derived(
    keyboardSelectedIndex >= 0 && keyboardSelectedIndex < filteredTabs.length
      ? filteredTabs[keyboardSelectedIndex].id
      : null,
  );

  $effect(() => {
    filterText;
    keyboardSelectedIndex = -1;
  });

  let pinnedTabs = $derived(filteredTabs.filter((t) => t.pinned));
  let regularTabs = $derived(filteredTabs.filter((t) => !t.pinned));

  let groupedTabsMap = $derived.by(() => {
    const map: Record<string, TabInfo[]> = {};
    for (const g of tabGroups) {
      map[g.id] = [];
    }
    for (const t of regularTabs) {
      if (t.groupId && map[t.groupId]) {
        map[t.groupId].push(t);
      }
    }
    return map;
  });

  let ungroupedRegularTabs = $derived(
    regularTabs.filter((t) => !t.groupId || !tabGroups.some((g) => g.id === t.groupId)),
  );

  function switchTo(id: number): void {
    send({ type: 'switchTab', id });
    onClose();
  }

  function hibernateOthers(): void {
    send({ type: 'discardOtherTabs' });
  }

  function hibernateTab(id: number): void {
    send({ type: 'discardTab', id });
  }

  function createTabGroup() {
    const name = newGroupName.trim();
    if (!name) return;
    const id = 'grp-' + Date.now().toString(36) + '-' + Math.random().toString(36).substring(2, 6);
    const newGroup: TabGroup = {
      id,
      name,
      color: newGroupColor,
      collapsed: false,
    };
    const updated = [...tabGroups, newGroup];
    send({ type: 'saveTabGroups', groups: updated });
    newGroupName = '';
    isCreatingGroup = false;
  }

  function toggleGroupCollapse(groupId: string) {
    const updated = tabGroups.map((g) =>
      g.id === groupId ? { ...g, collapsed: !g.collapsed } : g,
    );
    send({ type: 'saveTabGroups', groups: updated });
  }

  function deleteTabGroup(groupId: string) {
    for (const t of tabs) {
      if (t.groupId === groupId) {
        send({ type: 'setTabGroup', tabId: t.id, groupId: null });
      }
    }
    const updated = tabGroups.filter((g) => g.id !== groupId);
    send({ type: 'saveTabGroups', groups: updated });
  }

  function addActiveTabToGroup(groupId: string) {
    if (activeTabId != null) {
      send({ type: 'setTabGroup', tabId: activeTabId, groupId });
    }
  }

  function removeTabFromGroup(tabId: number) {
    send({ type: 'setTabGroup', tabId, groupId: null });
  }

  function assignTabToGroup(tabId: number, groupId: string) {
    send({ type: 'setTabGroup', tabId, groupId });
  }

  function onDragStart(event: DragEvent, id: number): void {
    dragId = id;
    if (event.dataTransfer) {
      event.dataTransfer.setData('text/plain', String(id));
      event.dataTransfer.effectAllowed = 'move';
    }
  }

  function onDrop(event: DragEvent, index: number): void {
    event.preventDefault();
    if (dragId === null) return;
    send({ type: 'reorderTab', id: dragId, toIndex: index });
    dragId = null;
  }
</script>

<!-- Spaces Switcher Strip -->
<div class="spaces-filter-strip">
  <button
    class="space-filter-pill"
    class:active={activeSpace === 'all'}
    onclick={() => onSetSpace('all')}
    type="button"
  >
    All
  </button>
  {#each SPACES as sp}
    {@const spCount = tabs.filter(t => (tabSpaceMap[t.id] || 'personal') === sp.id).length}
    <button
      class="space-filter-pill"
      class:active={activeSpace === sp.id}
      style="--sp-color: {sp.color}"
      onclick={() => onSetSpace(sp.id)}
      type="button"
    >
      <span class="sp-dot" style="background: {sp.color}"></span>
      {sp.name} <span class="sp-badge">{spCount}</span>
    </button>
  {/each}
</div>

<div class="filter-wrap">
  <div class="filter-box">
    <svg viewBox="0 0 16 16" fill="none" stroke="currentColor" class="filter-icon">
      <circle cx="7" cy="7" r="4.5" stroke-width="1.6" />
      <path d="M10.5 10.5L14 14" stroke-width="1.6" stroke-linecap="round" />
    </svg>
    <input
      type="search"
      placeholder="Filter tabs..."
      bind:value={filterText}
      class="filter-input"
    />
    {#if filterText}
      <button class="clear-filter" onclick={() => (filterText = '')}>×</button>
    {/if}
  </div>
</div>

<!-- Memory Saver Dashboard -->
<div class="memory-saver-bar">
  <div class="mem-stat-info">
    <span class="mem-icon" aria-hidden="true">☾</span>
    <span class="mem-text">
      <strong>{liveCount}</strong> live ·
      <strong>{sleepingCount}</strong> sleeping
      {#if sleepingCount > 0}
        <span class="mem-saved">({estimatedMemorySaved}MB saved)</span>
      {/if}
    </span>
  </div>
  {#if liveCount > 1}
    <button
      class="hibernate-btn"
      title="Hibernate all inactive background tabs to save memory"
      onclick={hibernateOthers}
      type="button"
    >
      Hibernate Others
    </button>
  {/if}
</div>

<div class="body tabs-body">
  {#if pinnedTabs.length > 0}
    <div class="tabs-subgroup-header">
      <span class="subgroup-title">PINNED TABS</span>
      <span class="subgroup-count">{pinnedTabs.length}</span>
    </div>
    <ul class="tabs-list pinned-tabs-list">
      {#each pinnedTabs as tab (tab.id)}
        <li
          class:active={tab.id === activeTabId}
          class:discarded={tab.discarded}
          class:keyboard-selected={tab.id === keyboardSelectedTabId}
          class="is-pinned-tab"
        >
          <button
            class="tab-row"
            onclick={() => switchTo(tab.id)}
            title={`${tab.title || (tab.url === 'about:blank' ? 'New Tab' : tab.url)}\n${tab.url}`}
          >
            {#if tab.favicon}
              <img
                src={tab.favicon}
                alt=""
                class="favicon"
                onerror={(e) => {
                  (e.currentTarget as HTMLImageElement).style.display = 'none';
                }}
              />
            {:else}
              <span class="favicon-initial">{domainInitial(tab.url)}</span>
            {/if}
            <span class="title">
              {tab.title || (tab.url === 'about:blank' ? 'New Tab' : tab.url) || 'New Tab'}
            </span>
            {#if tab.audible}
              <span class="pill audio" title="Playing audio">
                <svg viewBox="0 0 16 16" fill="currentColor" class="pill-icon">
                  <path d="M11.559 1.44a.75.75 0 0 1 .441.682v11.756a.75.75 0 0 1-1.28.53l-3.97-3.97H4.25a1.75 1.75 0 0 1-1.75-1.75V7.318c0-.966.784-1.75 1.75-1.75h2.5l3.97-3.97a.75.75 0 0 1 .839-.158z"/>
                </svg>
                Audio
              </span>
            {/if}
          </button>
          {#if tab.audible}
            <button
              class="tab-audio-btn"
              title="Mute / Unmute tab audio"
              type="button"
              onclick={(e) => {
                e.stopPropagation();
                send({ type: 'toggleMuteTab', id: tab.id });
              }}
            >
              <svg viewBox="0 0 16 16" fill="currentColor" class="tab-audio-icon">
                <path d="M11.559 1.44a.75.75 0 0 1 .441.682v11.756a.75.75 0 0 1-1.28.53l-3.97-3.97H4.25a1.75 1.75 0 0 1-1.75-1.75V7.318c0-.966.784-1.75 1.75-1.75h2.5l3.97-3.97a.75.75 0 0 1 .839-.158z"/>
              </svg>
            </button>
          {/if}
          <button
            class="tab-space-tag"
            title={`Space: ${SPACES.find((s) => s.id === (tabSpaceMap[tab.id] || 'personal'))?.name} (Click to switch space)`}
            type="button"
            onclick={(e) => {
              e.stopPropagation();
              onCycleSpace(tab.id);
            }}
          >
            <span
              class="space-indicator-dot"
              style="background: {SPACES.find((s) => s.id === (tabSpaceMap[tab.id] || 'personal'))?.color}"
            ></span>
          </button>
          <button
            class="tab-pin-btn is-pinned"
            title="Unpin tab"
            type="button"
            onclick={(e) => {
              e.stopPropagation();
              send({ type: 'pinTab', id: tab.id, pinned: false });
            }}
          >
            <svg viewBox="0 0 16 16" fill="currentColor" stroke="currentColor" class="tab-pin-icon">
              <path d="M9.828.722a.5.5 0 0 1 .354.146l4.95 4.95a.5.5 0 0 1 0 .707c-.48.48-1.072.588-1.503.588-.177 0-.335-.018-.46-.039l-3.134 3.134a5.927 5.927 0 0 1 .16 1.013c.046.702-.032 1.687-.72 2.375a.5.5 0 0 1-.707 0l-2.829-2.828-3.182 3.182c-.195.195-1.219.909-1.316.812-.097-.097.617-1.121.812-1.316l3.182-3.182-2.828-2.829a.5.5 0 0 1 0-.707c.688-.688 1.673-.767 2.375-.72a5.922 5.922 0 0 1 1.013.16l3.134-3.133a2.772 2.772 0 0 1-.04-.461c0-.43.108-1.022.589-1.503a.5.5 0 0 1 .353-.146z" stroke-width="1.2" stroke-linecap="round" stroke-linejoin="round"/>
            </svg>
          </button>
          <button
            class="tab-close"
            title="Close tab (Cmd+W)"
            onclick={(e) => {
              e.stopPropagation();
              send({ type: 'closeTab', id: tab.id });
            }}
          >
            ×
          </button>
        </li>
      {/each}
    </ul>
    {#if regularTabs.length > 0}
      <div class="tabs-subgroup-header" style="margin-top: 10px;">
        <span class="subgroup-title">ALL TABS</span>
        <span class="subgroup-count">{regularTabs.length}</span>
      </div>
    {/if}
  {/if}

  {#snippet tabItem(tab: TabInfo, isGrouped: boolean, groupColor: string)}
    <li
      class:active={tab.id === activeTabId}
      class:discarded={tab.discarded}
      class:keyboard-selected={tab.id === keyboardSelectedTabId}
      class:tab-in-group={isGrouped}
      style={isGrouped ? `--group-accent: ${groupColor}` : ''}
      draggable="true"
      ondragstart={(e) => onDragStart(e, tab.id)}
      ondragover={(e) => e.preventDefault()}
      ondrop={(e) => onDrop(e, 0)}
    >
      <button
        class="tab-row"
        onclick={() => switchTo(tab.id)}
        title={`${tab.title || (tab.url === 'about:blank' ? 'New Tab' : tab.url)}\n${tab.url}`}
      >
        {#if tab.favicon}
          <img
            src={tab.favicon}
            alt=""
            class="favicon"
            onerror={(e) => {
              (e.currentTarget as HTMLImageElement).style.display = 'none';
            }}
          />
        {:else}
          <span class="favicon-initial">{domainInitial(tab.url)}</span>
        {/if}
        <span class="title">
          {tab.title || (tab.url === 'about:blank' ? 'New Tab' : tab.url) || 'New Tab'}
        </span>
        {#if tab.audible}
          <span class="pill audio" title="Playing audio">
            <svg viewBox="0 0 16 16" fill="currentColor" class="pill-icon">
              <path d="M11.559 1.44a.75.75 0 0 1 .441.682v11.756a.75.75 0 0 1-1.28.53l-3.97-3.97H4.25a1.75 1.75 0 0 1-1.75-1.75V7.318c0-.966.784-1.75 1.75-1.75h2.5l3.97-3.97a.75.75 0 0 1 .839-.158z"/>
            </svg>
            Audio
          </span>
        {/if}
        {#if tab.discarded}
          <span class="pill sleep" title="Tab sleeping to save memory. Click to wake.">
            ☾ Sleeping
          </span>
        {/if}
      </button>
      {#if tab.audible}
        <button
          class="tab-audio-btn"
          title="Mute / Unmute tab audio"
          type="button"
          onclick={(e) => {
            e.stopPropagation();
            send({ type: 'toggleMuteTab', id: tab.id });
          }}
        >
          <svg viewBox="0 0 16 16" fill="currentColor" class="tab-audio-icon">
            <path d="M11.559 1.44a.75.75 0 0 1 .441.682v11.756a.75.75 0 0 1-1.28.53l-3.97-3.97H4.25a1.75 1.75 0 0 1-1.75-1.75V7.318c0-.966.784-1.75 1.75-1.75h2.5l3.97-3.97a.75.75 0 0 1 .839-.158z"/>
          </svg>
        </button>
      {/if}
      {#if isGrouped}
        <button
          class="tab-group-btn"
          title="Remove tab from this group"
          type="button"
          onclick={(e) => {
            e.stopPropagation();
            removeTabFromGroup(tab.id);
          }}
        >
          <span class="tab-group-icon">⤺</span>
        </button>
      {:else if tabGroups.length > 0}
        <select
          class="tab-group-select"
          value=""
          title="Add to group..."
          onclick={(e) => e.stopPropagation()}
          onchange={(e) => {
            e.stopPropagation();
            const val = (e.currentTarget as HTMLSelectElement).value;
            if (val) assignTabToGroup(tab.id, val);
          }}
        >
          <option value="" disabled selected>+</option>
          {#each tabGroups as g}
            <option value={g.id}>{g.name}</option>
          {/each}
        </select>
      {/if}
      <button
        class="tab-space-tag"
        title={`Space: ${SPACES.find((s) => s.id === (tabSpaceMap[tab.id] || 'personal'))?.name} (Click to switch space)`}
        type="button"
        onclick={(e) => {
          e.stopPropagation();
          onCycleSpace(tab.id);
        }}
      >
        <span
          class="space-indicator-dot"
          style="background: {SPACES.find((s) => s.id === (tabSpaceMap[tab.id] || 'personal'))?.color}"
        ></span>
      </button>
      <button
        class="tab-pin-btn"
        title="Pin tab"
        type="button"
        onclick={(e) => {
          e.stopPropagation();
          send({ type: 'pinTab', id: tab.id, pinned: true });
        }}
      >
        <svg viewBox="0 0 16 16" fill="none" stroke="currentColor" class="tab-pin-icon">
          <path d="M9.828.722a.5.5 0 0 1 .354.146l4.95 4.95a.5.5 0 0 1 0 .707c-.48.48-1.072.588-1.503.588-.177 0-.335-.018-.46-.039l-3.134 3.134a5.927 5.927 0 0 1 .16 1.013c.046.702-.032 1.687-.72 2.375a.5.5 0 0 1-.707 0l-2.829-2.828-3.182 3.182c-.195.195-1.219.909-1.316.812-.097-.097.617-1.121.812-1.316l3.182-3.182-2.828-2.829a.5.5 0 0 1 0-.707c.688-.688 1.673-.767 2.375-.72a5.922 5.922 0 0 1 1.013.16l3.134-3.133a2.772 2.772 0 0 1-.04-.461c0-.43.108-1.022.589-1.503a.5.5 0 0 1 .353-.146z" stroke-width="1.2" stroke-linecap="round" stroke-linejoin="round"/>
        </svg>
      </button>
      {#if !tab.discarded && tab.id !== activeTabId}
        <button
          class="tab-sleep-btn"
          title="Hibernate tab to free memory"
          type="button"
          onclick={(e) => {
            e.stopPropagation();
            hibernateTab(tab.id);
          }}
        >
          ☾
        </button>
      {/if}
      <button
        class="tab-close"
        title="Close tab (Cmd+W)"
        onclick={(e) => {
          e.stopPropagation();
          send({ type: 'closeTab', id: tab.id });
        }}
      >
        ×
      </button>
    </li>
  {/snippet}

  <!-- Tab Groups Section & Toolbar -->
  <div class="tabs-subgroup-header" style="margin-top: 10px; display: flex; justify-content: space-between; align-items: center;">
    <span class="subgroup-title">TAB GROUPS</span>
    <button
      class="new-group-header-btn"
      onclick={() => (isCreatingGroup = !isCreatingGroup)}
      type="button"
      title="Create a new Tab Group"
    >
      + New Group
    </button>
  </div>

  {#if isCreatingGroup}
    <div class="card create-group-card">
      <input
        type="text"
        placeholder="Group name (e.g. Research, Project)..."
        bind:value={newGroupName}
        class="text-input sm"
        onkeydown={(e) => { if (e.key === 'Enter') createTabGroup(); }}
      />
      <div class="color-picker-row">
        {#each GROUP_PALETTE as color}
          <button
            class="color-swatch-btn"
            class:selected={newGroupColor === color}
            style="background: {color};"
            onclick={() => (newGroupColor = color)}
            type="button"
            title={color}
          ></button>
        {/each}
      </div>
      <div class="group-btn-actions">
        <button class="primary-btn sm" onclick={createTabGroup} type="button">Create</button>
        <button class="ghost-btn sm" onclick={() => (isCreatingGroup = false)} type="button">Cancel</button>
      </div>
    </div>
  {/if}

  {#each tabGroups as group (group.id)}
    {@const gTabs = groupedTabsMap[group.id] || []}
    <div class="tab-group-wrapper" style="--group-color: {group.color}">
      <div class="tab-group-header">
        <button
          class="group-toggle-btn"
          onclick={() => toggleGroupCollapse(group.id)}
          type="button"
          title={group.collapsed ? 'Expand group' : 'Collapse group'}
        >
          <span class="group-chevron" class:collapsed={group.collapsed}>▾</span>
          <span class="group-dot" style="background: {group.color}"></span>
          <span class="group-name">{group.name}</span>
          <span class="group-count">{gTabs.length}</span>
        </button>
        <div class="group-actions">
          <button
            class="group-btn"
            onclick={() => addActiveTabToGroup(group.id)}
            type="button"
            title="Add active tab to this group"
          >
            + Tab
          </button>
          <button
            class="group-btn danger"
            onclick={() => deleteTabGroup(group.id)}
            type="button"
            title="Delete group (ungroups tabs)"
          >
            ✕
          </button>
        </div>
      </div>

      {#if !group.collapsed}
        {#if gTabs.length === 0}
          <div class="group-empty-hint">No tabs in this group yet</div>
        {:else}
          <ul class="tabs-list grouped-tabs-list">
            {#each gTabs as tab (tab.id)}
              {@render tabItem(tab, true, group.color)}
            {/each}
          </ul>
        {/if}
      {/if}
    </div>
  {/each}

  {#if ungroupedRegularTabs.length > 0}
    <div class="tabs-subgroup-header" style="margin-top: 10px;">
      <span class="subgroup-title">{tabGroups.length > 0 ? 'OTHER TABS' : 'ALL TABS'}</span>
      <span class="subgroup-count">{ungroupedRegularTabs.length}</span>
    </div>
    <ul class="tabs-list">
      {#each ungroupedRegularTabs as tab (tab.id)}
        {@render tabItem(tab, false, '')}
      {/each}
    </ul>
  {/if}

  {#if filteredTabs.length === 0}
    <div class="empty-state">
      <span class="empty-icon">🔍</span>
      <span class="empty-text">No tabs match "{filterText}"</span>
      <button
        class="clear-filter-btn"
        type="button"
        onclick={() => (filterText = '')}
      >
        Clear filter (Esc)
      </button>
    </div>
  {/if}
</div>

<footer class="tabs-footer">
  <button
    class="new-tab-btn"
    onclick={() => {
      send({ type: 'newTab' });
      onClose();
    }}
  >
    <span class="plus">+</span> New Tab <kbd>⌘ T</kbd>
  </button>
</footer>

<style>

  .spaces-filter-strip {
    display: flex;
    align-items: center;
    gap: 4px;
    padding: 8px 12px 4px;
    overflow-x: auto;
    scrollbar-width: none;
    user-select: none;
  }

  .spaces-filter-strip::-webkit-scrollbar {
    display: none;
  }

  .space-filter-pill {
    all: unset;
    cursor: pointer;
    display: inline-flex;
    align-items: center;
    gap: 5px;
    padding: 3px 8px;
    border-radius: 9999px;
    font-size: 11px;
    color: var(--muted);
    background: var(--surface-2);
    border: 1px solid var(--border-subtle);
    flex-shrink: 0;
    transition: all 0.14s ease;
  }

  .space-filter-pill:hover {
    background: var(--hover);
    color: var(--text);
  }

  .space-filter-pill.active {
    background: var(--surface);
    color: var(--text);
    border-color: var(--border);
    box-shadow: var(--shadow-sm);
    font-weight: 550;
  }

  .sp-dot {
    width: 6px;
    height: 6px;
    border-radius: 50%;
  }

  .sp-badge {
    font-size: 9.5px;
    opacity: 0.7;
  }

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

  .memory-saver-bar {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 6px 14px;
    background: color-mix(in srgb, var(--surface-2) 65%, transparent);
    border-bottom: 1px solid var(--border-subtle);
    font-size: 11px;
    user-select: none;
    min-height: 28px;
    box-sizing: border-box;
  }

  .mem-stat-info {
    display: flex;
    align-items: center;
    gap: 6px;
    color: var(--text-secondary);
  }

  .mem-icon {
    font-size: 11px;
    color: var(--accent);
  }

  .mem-saved {
    color: var(--accent);
    font-weight: 500;
  }

  .hibernate-btn {
    all: unset;
    cursor: pointer;
    font-size: 10px;
    font-weight: 600;
    color: var(--accent);
    background: var(--accent-soft);
    padding: 2px 7px;
    border-radius: 4px;
    border: 1px solid color-mix(in srgb, var(--accent) 30%, transparent);
    transition: all 0.12s ease;
    white-space: nowrap;
  }

  .hibernate-btn:hover {
    background: color-mix(in srgb, var(--accent) 24%, transparent);
  }

  .body {
    flex: 1;
    overflow-y: auto;
    padding: 12px;
  }

  .tabs-subgroup-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 6px 8px 4px;
    font-size: 10px;
    font-weight: 700;
    letter-spacing: 0.06em;
    color: var(--muted);
    user-select: none;
  }

  .subgroup-title {
    text-transform: uppercase;
  }

  .subgroup-count {
    padding: 1px 5px;
    border-radius: 999px;
    background: var(--surface-2);
    font-size: 9.5px;
  }

  .tabs-list {
    list-style: none;
    margin: 0;
    padding: 0;
    display: flex;
    flex-direction: column;
    gap: 2px;
  }

  .tabs-list.pinned-tabs-list {
    margin-bottom: 8px;
  }

  .tabs-list li {
    display: flex;
    align-items: center;
    border-radius: 7px;
    position: relative;
    transition: background 0.12s ease;
  }

  .tabs-list li.is-pinned-tab {
    background: color-mix(in srgb, var(--surface-2) 65%, transparent);
    border: 1px solid var(--border-subtle);
  }

  .tabs-list li:hover {
    background: var(--hover);
  }

  .tabs-list li.active {
    background: var(--accent-soft);
    border-color: color-mix(in srgb, var(--accent) 30%, transparent);
    font-weight: 550;
  }

  .tabs-list li.discarded {
    opacity: 0.65;
  }

  .tabs-list li.keyboard-selected .tab-row {
    background: var(--accent-soft);
    border-color: var(--accent);
  }

  .tab-row {
    all: unset;
    flex: 1;
    display: flex;
    align-items: center;
    gap: 8px;
    min-width: 0;
    height: 32px;
    padding: 0 8px;
    text-align: left;
    font-size: 12.5px;
    cursor: pointer;
  }

  .title {
    flex: 1;
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    color: var(--text);
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

  .pill {
    flex: 0 0 auto;
    padding: 1px 5px;
    border-radius: 999px;
    font-size: 9.5px;
    letter-spacing: 0.04em;
    font-weight: 600;
    text-transform: uppercase;
    display: inline-flex;
    align-items: center;
    gap: 3px;
  }

  .pill-icon {
    width: 9px;
    height: 9px;
  }

  .pill.audio {
    all: unset;
    padding: 1px 5px;
    border-radius: 999px;
    font-size: 9.5px;
    letter-spacing: 0.04em;
    font-weight: 600;
    text-transform: uppercase;
    display: inline-flex;
    align-items: center;
    gap: 3px;
    background: color-mix(in srgb, var(--accent) 25%, transparent);
    color: var(--accent);
    cursor: pointer;
    transition: all 0.12s ease;
  }

  .pill.sleep {
    background: color-mix(in srgb, var(--surface-2) 90%, transparent);
    color: var(--muted);
    border: 1px solid var(--border-subtle);
    font-size: 9px;
  }

  .tab-audio-btn {
    all: unset;
    flex: 0 0 20px;
    height: 20px;
    border-radius: 4px;
    color: var(--accent);
    cursor: pointer;
    display: flex;
    align-items: center;
    justify-content: center;
    transition: all 0.12s ease;
    margin-right: 2px;
  }

  .tab-audio-btn:hover {
    background: var(--hover);
    transform: scale(1.1);
  }

  .tab-audio-icon {
    width: 12px;
    height: 12px;
  }

  .tab-space-tag {
    all: unset;
    cursor: pointer;
    display: flex;
    align-items: center;
    justify-content: center;
    width: 14px;
    height: 14px;
    border-radius: 3px;
    opacity: 0.6;
    transition: opacity 0.12s ease;
    margin-right: 2px;
  }

  .tab-space-tag:hover {
    opacity: 1;
    background: var(--hover);
  }

  .space-indicator-dot {
    width: 5.5px;
    height: 5.5px;
    border-radius: 50%;
  }

  .tab-pin-btn {
    all: unset;
    flex: 0 0 20px;
    height: 20px;
    border-radius: 4px;
    color: var(--muted);
    cursor: pointer;
    display: flex;
    align-items: center;
    justify-content: center;
    opacity: 0;
    transition: all 0.12s ease;
    margin-right: 2px;
  }

  .tab-pin-btn.is-pinned {
    opacity: 0.9;
    color: var(--accent);
  }

  .tabs-list li:hover .tab-pin-btn {
    opacity: 1;
  }

  .tab-pin-btn:hover {
    background: var(--hover);
    color: var(--accent);
  }

  .tab-pin-icon {
    width: 12px;
    height: 12px;
  }

  .tab-sleep-btn {
    all: unset;
    flex: 0 0 20px;
    height: 20px;
    border-radius: 4px;
    color: var(--muted);
    font-size: 11px;
    cursor: pointer;
    display: flex;
    align-items: center;
    justify-content: center;
    opacity: 0;
    transition: all 0.12s ease;
    margin-right: 2px;
  }

  .tabs-list li:hover .tab-sleep-btn {
    opacity: 1;
  }

  .tab-sleep-btn:hover {
    background: var(--hover);
    color: var(--accent);
  }

  .tab-close {
    all: unset;
    cursor: pointer;
    flex: 0 0 22px;
    height: 22px;
    margin-right: 4px;
    border-radius: 5px;
    color: var(--muted);
    font-size: 14px;
    line-height: 1;
    text-align: center;
    opacity: 0;
    transition: opacity 0.12s ease;
  }

  .tabs-list li:hover .tab-close {
    opacity: 1;
  }

  .tab-close:hover {
    background: color-mix(in srgb, #ef4444 18%, transparent);
    color: #ef4444;
  }

  /* Tab Groups */
  .new-group-header-btn {
    all: unset;
    font-size: 10px;
    font-weight: 600;
    color: var(--accent);
    cursor: pointer;
    padding: 2px 6px;
    border-radius: 4px;
    transition: background 0.12s ease;
  }

  .new-group-header-btn:hover {
    background: var(--accent-soft);
  }

  .create-group-card {
    margin: 4px 0 10px;
    display: flex;
    flex-direction: column;
    gap: 8px;
    background: var(--surface-2);
  }

  .color-picker-row {
    display: flex;
    gap: 6px;
    align-items: center;
  }

  .color-swatch-btn {
    all: unset;
    cursor: pointer;
    width: 18px;
    height: 18px;
    border-radius: 50%;
    border: 2px solid transparent;
    transition: transform 0.12s ease;
  }

  .color-swatch-btn.selected {
    border-color: var(--text);
    transform: scale(1.15);
  }

  .tab-group-wrapper {
    margin-bottom: 6px;
    border-radius: 8px;
    border: 1px solid color-mix(in srgb, var(--group-color) 25%, var(--border-subtle));
    background: color-mix(in srgb, var(--group-color) 4%, transparent);
    overflow: hidden;
  }

  .tab-group-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 5px 8px;
    background: color-mix(in srgb, var(--group-color) 10%, var(--surface-2));
    border-bottom: 1px solid color-mix(in srgb, var(--group-color) 15%, transparent);
  }

  .group-toggle-btn {
    all: unset;
    cursor: pointer;
    display: flex;
    align-items: center;
    gap: 6px;
    font-size: 11.5px;
    font-weight: 600;
    color: var(--text);
    flex: 1;
    min-width: 0;
  }

  .group-chevron {
    font-size: 9px;
    color: var(--muted);
    transition: transform 0.15s ease;
  }

  .group-chevron.collapsed {
    transform: rotate(-90deg);
  }

  .group-dot {
    width: 7px;
    height: 7px;
    border-radius: 50%;
    flex-shrink: 0;
  }

  .group-name {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .group-count {
    font-size: 9.5px;
    color: var(--muted);
    font-weight: normal;
  }

  .group-actions {
    display: flex;
    align-items: center;
    gap: 4px;
  }

  .group-btn {
    all: unset;
    cursor: pointer;
    font-size: 10px;
    font-weight: 550;
    color: var(--muted);
    padding: 1px 5px;
    border-radius: 4px;
    transition: all 0.12s ease;
  }

  .group-btn:hover {
    color: var(--text);
    background: var(--hover);
  }

  .group-btn.danger:hover {
    color: #ef4444;
    background: color-mix(in srgb, #ef4444 15%, transparent);
  }

  .group-empty-hint {
    padding: 8px;
    font-size: 11px;
    color: var(--muted);
    text-align: center;
    font-style: italic;
  }

  .grouped-tabs-list {
    padding: 3px;
  }

  .tab-in-group {
    border-left: 2px solid var(--group-accent);
    margin-left: 4px;
  }

  .tab-group-btn {
    all: unset;
    cursor: pointer;
    width: 16px;
    height: 16px;
    display: flex;
    align-items: center;
    justify-content: center;
    font-size: 12px;
    color: var(--muted);
    border-radius: 3px;
    margin-right: 2px;
  }

  .tab-group-btn:hover {
    background: var(--hover);
    color: var(--text);
  }

  .tab-group-select {
    padding: 1px 4px;
    font-size: 10px;
    border-radius: 4px;
    background: var(--surface-2);
    border: 1px solid var(--border-subtle);
    color: var(--muted);
    cursor: pointer;
    margin-right: 2px;
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

  .clear-filter-btn {
    all: unset;
    cursor: pointer;
    background: var(--surface-2);
    color: var(--accent);
    padding: 4px 10px;
    border-radius: 6px;
    font-size: 11px;
    font-weight: 500;
    border: 1px solid var(--border);
    transition: all 0.12s ease;
  }

  .clear-filter-btn:hover {
    background: var(--hover);
    border-color: var(--accent);
  }

  .tabs-footer {
    padding: 10px 12px;
    border-top: 1px solid var(--border);
    flex-shrink: 0;
  }

  .new-tab-btn {
    all: unset;
    display: flex;
    align-items: center;
    justify-content: space-between;
    width: 100%;
    padding: 7px 10px;
    border: 1px dashed var(--border);
    border-radius: 8px;
    color: var(--muted);
    font-size: 12px;
    box-sizing: border-box;
    cursor: pointer;
    transition: all 0.15s ease;
  }

  .new-tab-btn:hover {
    color: var(--text);
    border-color: var(--accent);
  }

  .plus {
    font-size: 14px;
    font-weight: 700;
  }
</style>
