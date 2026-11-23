<script lang="ts">
  import { onMount } from "svelte";
  import {
    displayUrl,
    isBlank,
    onEvent,
    requestState,
    send,
    type BookmarkItem,
    type EdgeSide,
    type ExtensionSummary,
    type HistoryItem,
    type PeekEvent,
    type TabInfo,
  } from "./ipc";

  type Section =
    | "tabs"
    | "spaces"
    | "history"
    | "bookmarks"
    | "notes"
    | "general"
    | "appearance"
    | "blocking"
    | "preview"
    | "profiles"
    | "extensions"
    | "shortcuts"
    | "about";

  function getInitialParams() {
    const rawHash = window.location.hash.replace("#", "");
    const queryPart = rawHash.includes("?") ? rawHash.split("?")[1] : "";
    const params = new URLSearchParams(queryPart);
    const sideParam = params.get("side")?.toLowerCase();
    const sectionParam = params.get("section")?.toLowerCase();
    const validSections: Section[] = [
      "tabs",
      "spaces",
      "history",
      "bookmarks",
      "notes",
      "general",
      "appearance",
      "blocking",
      "preview",
      "profiles",
      "extensions",
      "shortcuts",
      "about",
    ];

    let resolvedSection: Section = sideParam === "right" ? "general" : "tabs";
    if (sectionParam) {
      if (sectionParam === "settings") {
        resolvedSection = "general";
      } else if (validSections.includes(sectionParam as Section)) {
        resolvedSection = sectionParam as Section;
      }
    }

    return {
      side: (sideParam === "right" ? "right" : "left") as EdgeSide,
      section: resolvedSection,
    };
  }

  const initialParams = getInitialParams();
  let currentSide = $state<EdgeSide>(initialParams.side);
  let section = $state<Section>(initialParams.section);
  let filterText = $state("");
  let animKey = $state(0);

  let tabs = $state<TabInfo[]>([]);
  let activeTabId = $state<number | null>(null);
  let blocked = $state(0);
  let allowedOnSite = $state(false);
  let host = $state<string | null>(null);
  let cookiesCount = $state(0);

  // Settings
  let maxLive = $state(5);
  let idleMinutes = $state(10);
  let hoverDelay = $state(150);
  let previewWidth = $state(480);
  let previewHeight = $state(320);
  let previewZoom = $state(0.75);
  let previewModifierOnly = $state(true);
  let configuredDrawerSide = $state<EdgeSide>("right");
  let edgeZone = $state(4);
  let startBlank = $state(false);
  let searchEngine = $state("Google");

  // Profiles
  let profiles = $state<string[]>(["default"]);
  let activeProfile = $state("default");
  let newProfileName = $state("");

  // Extensions
  let extensions = $state<ExtensionSummary[]>([]);
  let extensionInstallPath = $state("");

  // Appearance & Theme
  let themeMode = $state<"system" | "dark" | "light">("system");
  let accentColor = $state<"blue" | "purple" | "emerald" | "amber" | "rose">(
    "blue",
  );

  // Drag and close timer
  let dragId = $state<number | null>(null);
  let closeTimer: ReturnType<typeof setTimeout> | undefined;

  let sleepingCount = $derived(tabs.filter((t) => t.discarded).length);
  let liveCount = $derived(tabs.length - sleepingCount);
  let estimatedMemorySaved = $derived(sleepingCount * 95);

  function hibernateOthers(): void {
    send({ type: "discardOtherTabs" });
  }

  function hibernateTab(id: number): void {
    send({ type: "discardTab", id });
  }

  // Spaces State
  type SpaceId = "personal" | "work" | "dev" | "research";
  type SpaceInfo = {
    id: SpaceId;
    name: string;
    icon: string;
    color: string;
    desc: string;
  };

  const SPACES: SpaceInfo[] = [
    {
      id: "personal",
      name: "Personal",
      icon: "👤",
      color: "#3b82f6",
      desc: "Daily browsing & leisure",
    },
    {
      id: "work",
      name: "Work",
      icon: "💼",
      color: "#a855f7",
      desc: "Projects & company portals",
    },
    {
      id: "dev",
      name: "Dev",
      icon: "⚡",
      color: "#10b981",
      desc: "GitHub, docs & localhost",
    },
    {
      id: "research",
      name: "Research",
      icon: "🔬",
      color: "#f59e0b",
      desc: "Papers, articles & notes",
    },
  ];

  let activeSpace = $state<"all" | SpaceId>("all");
  let tabSpaceMap = $state<Record<number, SpaceId>>({});

  if (typeof localStorage !== "undefined") {
    try {
      const raw = localStorage.getItem("peek_tab_spaces");
      if (raw) tabSpaceMap = JSON.parse(raw);
      const savedSpace = localStorage.getItem("peek_active_space");
      if (savedSpace) activeSpace = savedSpace as "all" | SpaceId;
    } catch {}
  }

  function setSpace(space: "all" | SpaceId) {
    activeSpace = space;
    if (typeof localStorage !== "undefined") {
      localStorage.setItem("peek_active_space", space);
    }
  }

  function cycleTabSpace(tabId: number) {
    const spaceOrder: SpaceId[] = ["personal", "work", "dev", "research"];
    const current = tabSpaceMap[tabId] || "personal";
    const nextIdx = (spaceOrder.indexOf(current) + 1) % spaceOrder.length;
    const nextSpace = spaceOrder[nextIdx];
    tabSpaceMap = { ...tabSpaceMap, [tabId]: nextSpace };
    if (typeof localStorage !== "undefined") {
      localStorage.setItem("peek_tab_spaces", JSON.stringify(tabSpaceMap));
    }
  }

  // Research Scratchpad Notes State
  let noteContent = $state("");
  let copiedNotes = $state(false);
  let noteSaveTimer: ReturnType<typeof setTimeout> | undefined;

  function onNoteInput(event: Event) {
    const val = (event.target as HTMLTextAreaElement).value;
    noteContent = val;
    if (noteSaveTimer) clearTimeout(noteSaveTimer);
    noteSaveTimer = setTimeout(() => {
      send({ type: "saveNote", content: noteContent });
    }, 400);
  }

  function insertCurrentTabLink() {
    if (!activeTab || isBlank(activeTab.url)) return;
    const title = activeTab.title || activeTab.url;
    const mdLink = `\n- [${title}](${activeTab.url})\n`;
    noteContent += mdLink;
    send({ type: "saveNote", content: noteContent });
  }

  function copyNotes() {
    if (navigator.clipboard) {
      navigator.clipboard.writeText(noteContent);
      copiedNotes = true;
      setTimeout(() => {
        copiedNotes = false;
      }, 1800);
    }
  }

  let noteWords = $derived(
    noteContent.trim()
      ? noteContent.trim().split(/\s+/).filter(Boolean).length
      : 0,
  );

  let visibleTabs = $derived(
    activeSpace === "all"
      ? tabs
      : tabs.filter((t) => (tabSpaceMap[t.id] || "personal") === activeSpace),
  );

  let filteredTabs = $derived(
    filterText.trim()
      ? visibleTabs.filter((t) =>
          (t.title || t.url).toLowerCase().includes(filterText.toLowerCase()),
        )
      : visibleTabs,
  );

  let keyboardSelectedIndex = $state(-1);
  let keyboardSelectedTabId = $derived(
    keyboardSelectedIndex >= 0 && keyboardSelectedIndex < filteredTabs.length
      ? filteredTabs[keyboardSelectedIndex].id
      : null,
  );

  $effect(() => {
    // Reset selection when filter text changes
    filterText;
    keyboardSelectedIndex = -1;
  });

  $effect(() => {
    if (keyboardSelectedIndex >= 0) {
      setTimeout(() => {
        const el = document.querySelector(".tabs-list li.keyboard-selected");
        el?.scrollIntoView({ block: "nearest", behavior: "smooth" });
      }, 0);
    }
  });

  let pinnedTabs = $derived(filteredTabs.filter((t) => t.pinned));
  let regularTabs = $derived(filteredTabs.filter((t) => !t.pinned));

  // History and Bookmarks State
  let historyEntries = $state<HistoryItem[]>([]);
  let bookmarksList = $state<BookmarkItem[]>([]);

  let filteredHistory = $derived(
    filterText.trim()
      ? historyEntries.filter((h) =>
          (h.title || h.url).toLowerCase().includes(filterText.toLowerCase()),
        )
      : historyEntries,
  );

  let filteredBookmarks = $derived(
    filterText.trim()
      ? bookmarksList.filter((b) =>
          (b.title || b.url).toLowerCase().includes(filterText.toLowerCase()),
        )
      : bookmarksList,
  );

  function formatRelativeTime(ts: number): string {
    if (!ts) return "";
    const ms = ts > 10000000000 ? ts : ts * 1000;
    const diff = Date.now() - ms;
    if (diff < 60000) return "Just now";
    if (diff < 3600000) return `${Math.floor(diff / 60000)}m ago`;
    if (diff < 86400000) return `${Math.floor(diff / 3600000)}h ago`;
    const days = Math.floor(diff / 86400000);
    if (days === 1) return "Yesterday";
    if (days < 7) return `${days}d ago`;
    return new Date(ms).toLocaleDateString(undefined, {
      month: "short",
      day: "numeric",
    });
  }

  function cleanDomain(url: string): string {
    try {
      return new URL(url).hostname.replace(/^www\./, "");
    } catch {
      return url.replace(/^https?:\/\//, "").replace(/^www\./, "");
    }
  }

  let activeTab = $derived(tabs.find((t) => t.id === activeTabId));
  let isCurrentTabBookmarked = $derived(
    activeTab && !isBlank(activeTab.url)
      ? bookmarksList.some((b) => b.url === activeTab?.url)
      : false,
  );

  function toggleCurrentBookmark(): void {
    if (!activeTab || isBlank(activeTab.url)) return;
    send({
      type: "toggleBookmark",
      url: activeTab.url,
      title: activeTab.title || displayUrl(activeTab.url),
    });
  }

  function close(): void {
    send({ type: "setDrawer", open: false });
  }

  function scheduleClose(): void {
    cancelClose();
    closeTimer = setTimeout(close, 250);
  }

  function cancelClose(): void {
    if (closeTimer) {
      clearTimeout(closeTimer);
      closeTimer = undefined;
    }
  }

  function switchTo(id: number): void {
    send({ type: "switchTab", id });
    close();
  }

  function setting(key: string, value: unknown): void {
    send({ type: "setSetting", key, value });
  }

  function domainInitial(url: string): string {
    if (!url || url === "about:blank") return "✦";
    try {
      const u = new URL(url);
      const h = u.hostname.replace(/^www\./, "");
      return (h[0] || "✦").toUpperCase();
    } catch {
      return "✦";
    }
  }

  function applyTheme(mode: "system" | "dark" | "light") {
    themeMode = mode;
    localStorage.setItem("peek_theme", mode);
    if (mode === "system") {
      delete document.documentElement.dataset.theme;
    } else {
      document.documentElement.dataset.theme = mode;
    }
    window.dispatchEvent(new Event("storage"));
  }

  function applyAccent(
    accent: "blue" | "purple" | "emerald" | "amber" | "rose",
  ) {
    accentColor = accent;
    localStorage.setItem("peek_accent", accent);
    document.documentElement.dataset.accent = accent;
    window.dispatchEvent(new Event("storage"));
  }

  function handleCreateProfile() {
    const trimmed = newProfileName.trim();
    if (!trimmed) return;
    send({ type: "createProfile", name: trimmed });
    newProfileName = "";
  }

  function handleSwitchProfile(name: string) {
    if (name === activeProfile) return;
    send({ type: "switchProfile", name });
  }

  function handleDeleteProfile(name: string) {
    if (name === "default" || name === activeProfile) return;
    if (confirm(`Delete profile "${name}" and all its data?`)) {
      send({ type: "deleteProfile", name });
    }
  }

  function handleInstallExtension() {
    const path = extensionInstallPath.trim();
    if (!path) return;
    send({ type: "installExtension", path });
    extensionInstallPath = "";
  }

  function handleToggleExtension(id: string, current: boolean) {
    send({ type: "toggleExtension", id, enabled: !current });
  }

  function handleUninstallExtension(id: string) {
    if (confirm("Uninstall this extension?")) {
      send({ type: "uninstallExtension", id });
    }
  }

  function handleEvent(event: PeekEvent): void {
    switch (event.type) {
      case "tabsChanged":
        tabs = event.tabs;
        activeTabId = event.tabs.find((t) => t.active)?.id ?? null;
        break;
      case "adblockCount":
        blocked = event.blocked;
        allowedOnSite = event.allowedOnSite;
        host = event.host;
        break;
      case "settings":
        maxLive = event.maxLive;
        idleMinutes = event.idleDiscardMinutes;
        hoverDelay = event.hoverDelayMs;
        configuredDrawerSide = event.drawerSide;
        edgeZone = event.edgeZone;
        startBlank = event.startBlank;
        searchEngine = event.searchEngine;
        break;
      case "drawerState":
        if (!event.open) {
          // rust closed
        } else {
          animKey += 1;
          if (event.side) {
            currentSide = event.side;
            if (event.side === "left") {
              if (
                section !== "tabs" &&
                section !== "spaces" &&
                section !== "history" &&
                section !== "bookmarks" &&
                section !== "notes"
              ) {
                section = "tabs";
              }
            } else if (
              section === "tabs" ||
              section === "spaces" ||
              section === "history" ||
              section === "bookmarks" ||
              section === "notes"
            ) {
              section = "general";
            }
          }
          if (event.section) {
            if (event.section === "settings") {
              section = "general";
            } else {
              section = event.section as Section;
              if (section === "history") {
                currentSide = "left";
                send({ type: "getHistory" });
              } else if (section === "bookmarks") {
                currentSide = "left";
                send({ type: "getBookmarks" });
              } else if (section === "notes") {
                currentSide = "left";
                send({ type: "getNote" });
              }
            }
          }
        }
        break;
      case "noteLoaded":
        noteContent = event.content;
        break;
      case "historyList":
        historyEntries = event.entries;
        break;
      case "bookmarksList":
        bookmarksList = event.bookmarks;
        break;
      case "cookiesCount":
        cookiesCount = event.count;
        break;
      case "profilesList":
        profiles = event.profiles;
        activeProfile = event.active;
        break;
      case "extensionsList":
        extensions = event.extensions;
        break;
      default:
        break;
    }
  }

  function onDragStart(event: DragEvent, id: number): void {
    dragId = id;
    event.dataTransfer?.setData("text/plain", String(id));
  }

  function onDrop(event: DragEvent, index: number): void {
    event.preventDefault();
    if (dragId != null) {
      send({ type: "reorderTab", id: dragId, toIndex: index });
      dragId = null;
    }
  }

  onMount(() => {
    const savedTheme = localStorage.getItem("peek_theme") as
      | "system"
      | "dark"
      | "light"
      | null;
    if (savedTheme) applyTheme(savedTheme);
    const savedAccent = localStorage.getItem("peek_accent") as
      | "blue"
      | "purple"
      | "emerald"
      | "amber"
      | "rose"
      | null;
    if (savedAccent) applyAccent(savedAccent);

    onEvent(handleEvent);
    requestState();
    send({ type: "getHistory" });
    send({ type: "getBookmarks" });
    const onKey = (event: KeyboardEvent) => {
      if (event.key === "Escape") {
        event.preventDefault();
        if (filterText) {
          filterText = "";
        } else {
          close();
        }
        return;
      }
      if (currentSide === "left") {
        if (event.key === "ArrowDown") {
          event.preventDefault();
          if (filteredTabs.length > 0) {
            keyboardSelectedIndex =
              (keyboardSelectedIndex + 1) % filteredTabs.length;
          }
          return;
        }
        if (event.key === "ArrowUp") {
          event.preventDefault();
          if (filteredTabs.length > 0) {
            keyboardSelectedIndex =
              (keyboardSelectedIndex - 1 + filteredTabs.length) %
              filteredTabs.length;
          }
          return;
        }
        if (event.key === "Enter" && keyboardSelectedTabId != null) {
          event.preventDefault();
          switchTo(keyboardSelectedTabId);
          return;
        }
      }
      const mod = event.metaKey || event.ctrlKey;
      const key = event.key.toLowerCase();
      if (mod && !event.shiftKey && !event.altKey) {
        if (key === "t") {
          event.preventDefault();
          send({ type: "newTab" });
          close();
          return;
        }
        if (key === "w") {
          event.preventDefault();
          const targetId = keyboardSelectedTabId ?? activeTabId;
          if (targetId != null) {
            send({ type: "closeTab", id: targetId });
          }
          return;
        }
      }
      if (mod && event.shiftKey && !event.altKey) {
        if (key === "[" || key === "{") {
          event.preventDefault();
          if (currentSide === "left") {
            close();
          } else {
            currentSide = "left";
            section = "tabs";
          }
          return;
        }
        if (key === "]" || key === "}") {
          event.preventDefault();
          if (currentSide === "right") {
            close();
          } else {
            currentSide = "right";
            section = "general";
          }
          return;
        }
      }
    };
    window.addEventListener("keydown", onKey, true);
    return () => window.removeEventListener("keydown", onKey, true);
  });
</script>

{#key animKey}
  <aside
    class="drawer"
    class:left={currentSide === "left"}
    class:right={currentSide === "right"}
    onmouseenter={cancelClose}
    onmouseleave={scheduleClose}
    aria-label="Peek drawer"
  >
  {#if currentSide === 'left'}
    <!-- Left Drawer: Dedicated Vertical Tabs, History, and Bookmarks -->
    <header class="tabs-header">
      <div class="brand-row">
        <div class="tabs-brand">
          <img src="/logo.png" alt="Peek" class="drawer-logo" />
          <span class="drawer-title">Workspace</span>
        </div>
        <button class="icon-close-btn" onclick={close} title="Close drawer (Esc)">
          <svg viewBox="0 0 16 16" fill="none" stroke="currentColor" class="close-icon">
            <path d="M4 4l8 8M12 4l-8 8" stroke-width="1.6" stroke-linecap="round"/>
          </svg>
        </button>
      </div>

      <nav class="left-nav-pills" aria-label="Views">
        <button
          class="left-pill"
          class:active={section === 'tabs'}
          onclick={() => { section = 'tabs'; filterText = ''; }}
          type="button"
        >
          Tabs <span class="tab-count-badge">{filteredTabs.length}</span>
        </button>
        <button
          class="left-pill"
          class:active={section === 'spaces'}
          onclick={() => { section = 'spaces'; filterText = ''; }}
          type="button"
        >
          Spaces
        </button>
        <button
          class="left-pill"
          class:active={section === 'history'}
          onclick={() => { section = 'history'; filterText = ''; send({ type: 'getHistory' }); }}
          type="button"
        >
          History
        </button>
        <button
          class="left-pill"
          class:active={section === 'bookmarks'}
          onclick={() => { section = 'bookmarks'; filterText = ''; send({ type: 'getBookmarks' }); }}
          type="button"
        >
          Saved
        </button>
        <button
          class="left-pill"
          class:active={section === 'notes'}
          onclick={() => { section = 'notes'; filterText = ''; send({ type: 'getNote' }); }}
          type="button"
        >
          Notes
        </button>
      </nav>
    </header>

        <div class="filter-wrap">
          <div class="filter-box">
            <svg
              viewBox="0 0 16 16"
              fill="none"
              stroke="currentColor"
              class="filter-icon"
            >
              <circle cx="7" cy="7" r="4.5" stroke-width="1.6" />
              <path
                d="M10.5 10.5L14 14"
                stroke-width="1.6"
                stroke-linecap="round"
              />
            </svg>
            <input
              type="search"
              placeholder="Filter tabs..."
              bind:value={filterText}
              class="filter-input"
            />
            {#if filterText}
              <button class="clear-filter" onclick={() => (filterText = "")}
                >×</button
              >
            {/if}
          </div>
        </div>

        <!-- Memory Saver Dashboard -->
        <div class="memory-saver-bar">
          <div class="mem-stat-info">
            <span class="mem-icon" aria-hidden="true">☾</span>
            <span class="mem-text">
              <strong>{liveCount}</strong> live ·
              <strong>{sleepingCount}</strong>
              sleeping
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
                    title={`${tab.title || (tab.url === "about:blank" ? "New Tab" : tab.url)}\n${tab.url}`}
                  >
                    {#if tab.favicon}
                      <img
                        src={tab.favicon}
                        alt=""
                        class="favicon"
                        onerror={(e) => {
                          (e.currentTarget as HTMLImageElement).style.display =
                            "none";
                        }}
                      />
                    {:else}
                      <span class="favicon-initial"
                        >{domainInitial(tab.url)}</span
                      >
                    {/if}
                    <span class="title"
                      >{tab.title ||
                        (tab.url === "about:blank" ? "New Tab" : tab.url) ||
                        "New Tab"}</span
                    >
                    {#if tab.audible}
                      <span class="pill audio" title="Playing audio">
                        <svg
                          viewBox="0 0 16 16"
                          fill="currentColor"
                          class="pill-icon"
                        >
                          <path
                            d="M11.559 1.44a.75.75 0 0 1 .441.682v11.756a.75.75 0 0 1-1.28.53l-3.97-3.97H4.25a1.75 1.75 0 0 1-1.75-1.75V7.318c0-.966.784-1.75 1.75-1.75h2.5l3.97-3.97a.75.75 0 0 1 .839-.158z"
                          />
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
                        send({ type: "toggleMuteTab", id: tab.id });
                      }}
                    >
                      <svg
                        viewBox="0 0 16 16"
                        fill="currentColor"
                        class="tab-audio-icon"
                      >
                        <path
                          d="M11.559 1.44a.75.75 0 0 1 .441.682v11.756a.75.75 0 0 1-1.28.53l-3.97-3.97H4.25a1.75 1.75 0 0 1-1.75-1.75V7.318c0-.966.784-1.75 1.75-1.75h2.5l3.97-3.97a.75.75 0 0 1 .839-.158z"
                        />
                      </svg>
                    </button>
                  {/if}
                  <button
                    class="tab-space-tag"
                    title={`Space: ${SPACES.find((s) => s.id === (tabSpaceMap[tab.id] || "personal"))?.name} (Click to switch space)`}
                    type="button"
                    onclick={(e) => {
                      e.stopPropagation();
                      cycleTabSpace(tab.id);
                    }}
                  >
                    <span
                      class="space-indicator-dot"
                      style="background: {SPACES.find(
                        (s) => s.id === (tabSpaceMap[tab.id] || 'personal'),
                      )?.color}"
                    ></span>
                  </button>
                  <button
                    class="tab-pin-btn is-pinned"
                    title="Unpin tab"
                    type="button"
                    onclick={(e) => {
                      e.stopPropagation();
                      send({ type: "pinTab", id: tab.id, pinned: false });
                    }}
                  >
                    <svg
                      viewBox="0 0 16 16"
                      fill="currentColor"
                      stroke="currentColor"
                      class="tab-pin-icon"
                    >
                      <path
                        d="M9.828.722a.5.5 0 0 1 .354.146l4.95 4.95a.5.5 0 0 1 0 .707c-.48.48-1.072.588-1.503.588-.177 0-.335-.018-.46-.039l-3.134 3.134a5.927 5.927 0 0 1 .16 1.013c.046.702-.032 1.687-.72 2.375a.5.5 0 0 1-.707 0l-2.829-2.828-3.182 3.182c-.195.195-1.219.909-1.316.812-.097-.097.617-1.121.812-1.316l3.182-3.182-2.828-2.829a.5.5 0 0 1 0-.707c.688-.688 1.673-.767 2.375-.72a5.922 5.922 0 0 1 1.013.16l3.134-3.133a2.772 2.772 0 0 1-.04-.461c0-.43.108-1.022.589-1.503a.5.5 0 0 1 .353-.146z"
                        stroke-width="1.2"
                        stroke-linecap="round"
                        stroke-linejoin="round"
                      />
                    </svg>
                  </button>
                  <button
                    class="tab-close"
                    title="Close tab (Cmd+W)"
                    onclick={(e) => {
                      e.stopPropagation();
                      send({ type: "closeTab", id: tab.id });
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

          {#if regularTabs.length > 0}
            <ul class="tabs-list">
              {#each regularTabs as tab, idx (tab.id)}
                <li
                  class:active={tab.id === activeTabId}
                  class:discarded={tab.discarded}
                  class:keyboard-selected={tab.id === keyboardSelectedTabId}
                  draggable="true"
                  ondragstart={(e) => onDragStart(e, tab.id)}
                  ondragover={(e) => e.preventDefault()}
                  ondrop={(e) => onDrop(e, idx)}
                >
                  <button
                    class="tab-row"
                    onclick={() => switchTo(tab.id)}
                    title={`${tab.title || (tab.url === "about:blank" ? "New Tab" : tab.url)}\n${tab.url}`}
                  >
                    {#if tab.favicon}
                      <img
                        src={tab.favicon}
                        alt=""
                        class="favicon"
                        onerror={(e) => {
                          (e.currentTarget as HTMLImageElement).style.display =
                            "none";
                        }}
                      />
                    {:else}
                      <span class="favicon-initial"
                        >{domainInitial(tab.url)}</span
                      >
                    {/if}
                    <span class="title"
                      >{tab.title ||
                        (tab.url === "about:blank" ? "New Tab" : tab.url) ||
                        "New Tab"}</span
                    >
                    {#if tab.audible}
                      <span class="pill audio" title="Playing audio">
                        <svg
                          viewBox="0 0 16 16"
                          fill="currentColor"
                          class="pill-icon"
                        >
                          <path
                            d="M11.559 1.44a.75.75 0 0 1 .441.682v11.756a.75.75 0 0 1-1.28.53l-3.97-3.97H4.25a1.75 1.75 0 0 1-1.75-1.75V7.318c0-.966.784-1.75 1.75-1.75h2.5l3.97-3.97a.75.75 0 0 1 .839-.158z"
                          />
                        </svg>
                        Audio
                      </span>
                    {/if}
                    {#if tab.discarded}
                      <span
                        class="pill sleep"
                        title="Tab sleeping to save memory. Click to wake."
                      >
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
                        send({ type: "toggleMuteTab", id: tab.id });
                      }}
                    >
                      <svg
                        viewBox="0 0 16 16"
                        fill="currentColor"
                        class="tab-audio-icon"
                      >
                        <path
                          d="M11.559 1.44a.75.75 0 0 1 .441.682v11.756a.75.75 0 0 1-1.28.53l-3.97-3.97H4.25a1.75 1.75 0 0 1-1.75-1.75V7.318c0-.966.784-1.75 1.75-1.75h2.5l3.97-3.97a.75.75 0 0 1 .839-.158z"
                        />
                      </svg>
                    </button>
                  {/if}
                  <button
                    class="tab-space-tag"
                    title={`Space: ${SPACES.find((s) => s.id === (tabSpaceMap[tab.id] || "personal"))?.name} (Click to switch space)`}
                    type="button"
                    onclick={(e) => {
                      e.stopPropagation();
                      cycleTabSpace(tab.id);
                    }}
                  >
                    <span
                      class="space-indicator-dot"
                      style="background: {SPACES.find(
                        (s) => s.id === (tabSpaceMap[tab.id] || 'personal'),
                      )?.color}"
                    ></span>
                  </button>
                  <button
                    class="tab-pin-btn"
                    title="Pin tab"
                    type="button"
                    onclick={(e) => {
                      e.stopPropagation();
                      send({ type: "pinTab", id: tab.id, pinned: true });
                    }}
                  >
                    <svg
                      viewBox="0 0 16 16"
                      fill="none"
                      stroke="currentColor"
                      class="tab-pin-icon"
                    >
                      <path
                        d="M9.828.722a.5.5 0 0 1 .354.146l4.95 4.95a.5.5 0 0 1 0 .707c-.48.48-1.072.588-1.503.588-.177 0-.335-.018-.46-.039l-3.134 3.134a5.927 5.927 0 0 1 .16 1.013c.046.702-.032 1.687-.72 2.375a.5.5 0 0 1-.707 0l-2.829-2.828-3.182 3.182c-.195.195-1.219.909-1.316.812-.097-.097.617-1.121.812-1.316l3.182-3.182-2.828-2.829a.5.5 0 0 1 0-.707c.688-.688 1.673-.767 2.375-.72a5.922 5.922 0 0 1 1.013.16l3.134-3.133a2.772 2.772 0 0 1-.04-.461c0-.43.108-1.022.589-1.503a.5.5 0 0 1 .353-.146z"
                        stroke-width="1.2"
                        stroke-linecap="round"
                        stroke-linejoin="round"
                      />
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
                      send({ type: "closeTab", id: tab.id });
                    }}
                  >
                    ×
                  </button>
                </li>
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
                onclick={() => (filterText = "")}
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
              send({ type: "newTab" });
              close();
            }}
          >
            <span class="plus">+</span> New Tab <kbd>⌘ T</kbd>
          </button>
        </footer>
      {:else if section === "history"}
        <div class="filter-wrap">
          <div class="filter-box">
            <svg
              viewBox="0 0 16 16"
              fill="none"
              stroke="currentColor"
              class="filter-icon"
            >
              <circle cx="7" cy="7" r="4.5" stroke-width="1.6" />
              <path
                d="M10.5 10.5L14 14"
                stroke-width="1.6"
                stroke-linecap="round"
              />
            </svg>
            <input
              type="search"
              placeholder="Search browsing history..."
              bind:value={filterText}
              class="filter-input"
            />
            {#if filterText}
              <button class="clear-filter" onclick={() => (filterText = "")}
                >×</button
              >
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
                  : "No browsing history yet"}
              </span>
            </div>
          {:else}
            <ul class="history-list">
              {#each filteredHistory as item (item.url + ":" + item.lastVisit)}
                <li class="history-item">
                  <button
                    class="history-row"
                    type="button"
                    onclick={() => {
                      send({ type: "navigate", input: item.url });
                      close();
                    }}
                    title={`${item.title || item.url}\n${item.url}`}
                  >
                    <span class="favicon-initial"
                      >{domainInitial(item.url)}</span
                    >
                    <div class="history-info">
                      <span class="history-title"
                        >{item.title || displayUrl(item.url)}</span
                      >
                      <div class="history-sub">
                        <span class="history-host">{cleanDomain(item.url)}</span
                        >
                        {#if item.lastVisit}
                          <span class="history-dot">·</span>
                          <span class="history-time"
                            >{formatRelativeTime(item.lastVisit)}</span
                          >
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
                      send({ type: "deleteHistoryItem", url: item.url });
                      historyEntries = historyEntries.filter(
                        (h) => h.url !== item.url,
                      );
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
              if (confirm("Clear all browsing history?")) {
                send({ type: "clearHistory" });
                historyEntries = [];
              }
            }}
          >
            Clear Browsing History
          </button>
        </footer>
      {:else if section === "bookmarks"}
        <div class="filter-wrap">
          <div class="filter-box">
            <svg
              viewBox="0 0 16 16"
              fill="none"
              stroke="currentColor"
              class="filter-icon"
            >
              <circle cx="7" cy="7" r="4.5" stroke-width="1.6" />
              <path
                d="M10.5 10.5L14 14"
                stroke-width="1.6"
                stroke-linecap="round"
              />
            </svg>
            <input
              type="search"
              placeholder="Search saved bookmarks..."
              bind:value={filterText}
              class="filter-input"
            />
            {#if filterText}
              <button class="clear-filter" onclick={() => (filterText = "")}
                >×</button
              >
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
              <span class="star-icon">{isCurrentTabBookmarked ? "★" : "☆"}</span
              >
              <span
                >{isCurrentTabBookmarked
                  ? "Remove Current Tab from Saved"
                  : "Bookmark Current Tab"}</span
              >
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
                  : "No saved bookmarks yet"}
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
                      send({ type: "navigate", input: b.url });
                      close();
                    }}
                    title={`${b.title || b.url}\n${b.url}`}
                  >
                    {#if b.favicon}
                      <img
                        src={b.favicon}
                        alt=""
                        class="favicon"
                        onerror={(e) => {
                          (e.currentTarget as HTMLElement).style.display =
                            "none";
                        }}
                      />
                    {:else}
                      <span class="favicon-initial">{domainInitial(b.url)}</span
                      >
                    {/if}
                    <div class="bookmark-info">
                      <span class="bookmark-title"
                        >{b.title || displayUrl(b.url)}</span
                      >
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
                        type: "toggleBookmark",
                        url: b.url,
                        title: b.title,
                      });
                      bookmarksList = bookmarksList.filter(
                        (x) => x.id !== b.id,
                      );
                    }}
                  >
                    ×
                  </button>
                </li>
              {/each}
            </ul>
          {/if}
        </div>
      {:else if section === "spaces"}
        <div class="spaces-view-header">
          <span class="spaces-view-title">Contextual Workspaces</span>
          <span class="spaces-view-desc"
            >Switch contexts or focus on a specific project</span
          >
        </div>

        <div class="body spaces-body">
          <div class="spaces-grid">
            {#each SPACES as sp}
              {@const count = tabs.filter(
                (t) => (tabSpaceMap[t.id] || "personal") === sp.id,
              ).length}
              {@const activeInSpace = tabs.some(
                (t) => t.active && (tabSpaceMap[t.id] || "personal") === sp.id,
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
                  <span class="space-tab-count"
                    >{count} {count === 1 ? "tab" : "tabs"}</span
                  >
                  <button
                    class="space-select-btn"
                    class:active-btn={activeSpace === sp.id}
                    type="button"
                    onclick={() => {
                      setSpace(sp.id);
                      section = "tabs";
                    }}
                  >
                    {activeSpace === sp.id ? "Viewing Tabs" : "Switch Space"}
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
              setSpace("all");
              section = "tabs";
            }}
          >
            Show All Tabs Across Spaces
          </button>
        </footer>
      {:else if section === "notes"}
        <div class="scratchpad-header">
          <div class="scratchpad-meta">
            <span class="scratchpad-badge">RESEARCH SCRATCHPAD</span>
            <span class="scratchpad-count"
              >{noteWords} words · {noteContent.length} chars</span
            >
          </div>
          <div class="scratchpad-actions">
            {#if activeTab && !isBlank(activeTab.url)}
              <button
                class="scratch-action-btn"
                title="Insert link to active tab"
                type="button"
                onclick={insertCurrentTabLink}
              >
                + Link Tab
              </button>
            {/if}
            <button
              class="scratch-action-btn"
              title="Copy all notes to clipboard"
              type="button"
              onclick={copyNotes}
            >
              {copiedNotes ? "✓ Copied" : "Copy"}
            </button>
            <button
              class="scratch-action-btn danger"
              title="Clear notes"
              type="button"
              onclick={() => {
                if (confirm("Clear scratchpad notes?")) {
                  noteContent = "";
                  send({ type: "saveNote", content: "" });
                }
              }}
            >
              Clear
            </button>
          </div>
        </div>

        <div class="body scratchpad-body">
          <textarea
            class="scratchpad-editor"
            placeholder="Jot down quick thoughts, research findings, and task notes here...&#10;&#10;• Autosaved locally to your SQLite database.&#10;• Use '+ Link Tab' to insert current page as markdown.&#10;• Copy out to Notion, Slack, or Obsidian."
            bind:value={noteContent}
            oninput={onNoteInput}
            spellcheck="false"></textarea>
        </div>
      {/if}
    {:else}
      <!-- Right Drawer: Settings, Profiles, Adblock, Hover Preview, Extensions -->
      <header class="settings-header">
        <div class="brand-row">
          <div class="tabs-brand">
            <img src="/logo.png" alt="Peek" class="drawer-logo" />
            <span class="drawer-title">Settings & Tools</span>
          </div>
          <button
            class="icon-close-btn"
            onclick={close}
            title="Close drawer (Esc)"
          >
            <svg
              viewBox="0 0 16 16"
              fill="none"
              stroke="currentColor"
              class="close-icon"
            >
              <path
                d="M4 4l8 8M12 4l-8 8"
                stroke-width="1.6"
                stroke-linecap="round"
              />
            </svg>
          </button>
        </div>

        <nav class="settings-nav">
          <button
            class:on={section === "general"}
            onclick={() => (section = "general")}>General</button
          >
          <button
            class:on={section === "appearance"}
            onclick={() => (section = "appearance")}>Themes</button
          >
          <button
            class:on={section === "blocking"}
            onclick={() => (section = "blocking")}>Adblock</button
          >
          <button
            class:on={section === "preview"}
            onclick={() => (section = "preview")}>Preview</button
          >
          <button
            class:on={section === "profiles"}
            onclick={() => (section = "profiles")}>Profiles</button
          >
          <button
            class:on={section === "extensions"}
            onclick={() => (section = "extensions")}>Extensions</button
          >
          <button
            class:on={section === "shortcuts"}
            onclick={() => (section = "shortcuts")}>Keys</button
          >
          <button
            class:on={section === "about"}
            onclick={() => (section = "about")}>About</button
          >
        </nav>
      </header>

      <div class="body settings-body">
        {#if section === "general"}
          <div class="section-title">Search & Browsing</div>
          <div class="card settings-card">
            <div class="field">
              <label for="search-eng">Default Search Engine</label>
              <select
                id="search-eng"
                value={searchEngine}
                onchange={(e) =>
                  setting(
                    "searchEngine",
                    (e.currentTarget as HTMLSelectElement).value,
                  )}
              >
                <option value="Google">Google</option>
                <option value="DuckDuckGo">DuckDuckGo</option>
                <option value="Brave">Brave Search</option>
                <option value="Bing">Bing</option>
                <option value="Ecosia">Ecosia</option>
              </select>
            </div>

            <div class="field">
              <label for="side">Default Drawer Side</label>
              <select
                id="side"
                value={configuredDrawerSide}
                onchange={(e) =>
                  setting(
                    "drawerSide",
                    (e.currentTarget as HTMLSelectElement).value,
                  )}
              >
                <option value="right">Right</option>
                <option value="left">Left</option>
              </select>
            </div>

            <div class="field">
              <label for="edge">Edge Hover Zone (px)</label>
              <input
                id="edge"
                type="number"
                min="1"
                max="16"
                value={edgeZone}
                onchange={(e) =>
                  setting(
                    "edgeZone",
                    Number((e.currentTarget as HTMLInputElement).value),
                  )}
              />
            </div>

            <div class="field switch-field">
              <span>Always open new tabs with Start Page</span>
              <label class="switch">
                <input
                  type="checkbox"
                  checked={startBlank}
                  onchange={(e) =>
                    setting(
                      "startBlank",
                      (e.currentTarget as HTMLInputElement).checked,
                    )}
                />
                <span class="slider"></span>
              </label>
            </div>
          </div>

          <div class="section-title" style="margin-top: 18px;">
            Tab & Memory Budget
          </div>
          <div class="card settings-card">
            <div class="field">
              <div>
                <label for="max-live">Max Live Tabs (LRU)</label>
                <p class="field-desc">
                  Background tabs beyond this count are discarded
                </p>
              </div>
              <input
                id="max-live"
                type="number"
                min="1"
                max="20"
                value={maxLive}
                onchange={(e) =>
                  setting(
                    "maxLive",
                    Number((e.currentTarget as HTMLInputElement).value),
                  )}
              />
            </div>

            <div class="field" style="border-bottom: none;">
              <div>
                <label for="idle-time">Idle Discard (minutes)</label>
                <p class="field-desc">
                  Inactive background tabs are put to sleep
                </p>
              </div>
              <input
                id="idle-time"
                type="number"
                min="1"
                max="120"
                value={idleMinutes}
                onchange={(e) =>
                  setting(
                    "idleDiscardMinutes",
                    Number((e.currentTarget as HTMLInputElement).value),
                  )}
              />
            </div>
          </div>
        {:else if section === "appearance"}
          <div class="section-title">Color Scheme</div>
          <div class="theme-options">
            <button
              class="choice-btn"
              class:selected={themeMode === "system"}
              onclick={() => applyTheme("system")}
            >
              <span class="theme-icon">◐</span>
              Auto System
            </button>
            <button
              class="choice-btn"
              class:selected={themeMode === "dark"}
              onclick={() => applyTheme("dark")}
            >
              <span class="theme-icon">☾</span>
              Dark Mode
            </button>
            <button
              class="choice-btn"
              class:selected={themeMode === "light"}
              onclick={() => applyTheme("light")}
            >
              <span class="theme-icon">☼</span>
              Light Mode
            </button>
          </div>

          <div class="section-title" style="margin-top: 20px;">Accent Hue</div>
          <div class="card settings-card">
            <div class="accent-palette">
              <button
                class="accent-dot"
                class:selected={accentColor === "blue"}
                style="background: #3b82f6;"
                onclick={() => applyAccent("blue")}
                title="Electric Blue"
              ></button>
              <button
                class="accent-dot"
                class:selected={accentColor === "purple"}
                style="background: #8b5cf6;"
                onclick={() => applyAccent("purple")}
                title="Neon Violet"
              ></button>
              <button
                class="accent-dot"
                class:selected={accentColor === "emerald"}
                style="background: #10b981;"
                onclick={() => applyAccent("emerald")}
                title="Emerald Green"
              ></button>
              <button
                class="accent-dot"
                class:selected={accentColor === "amber"}
                style="background: #f59e0b;"
                onclick={() => applyAccent("amber")}
                title="Warm Amber"
              ></button>
              <button
                class="accent-dot"
                class:selected={accentColor === "rose"}
                style="background: #f43f5e;"
                onclick={() => applyAccent("rose")}
                title="Rose Quartz"
              ></button>
            </div>
          </div>
        {:else if section === "blocking"}
          <div class="card stat-card">
            <div class="stat-big">{blocked}</div>
            <div class="stat-label">Trackers & Ads Blocked This Session</div>
          </div>

          {#if host}
            <div class="card settings-card" style="margin-top: 14px;">
              <div class="field switch-field" style="border-bottom: none;">
                <div>
                  <span class="field-label-bold">Protection on {host}</span>
                  <p class="field-desc">
                    Adblock and tracker protection on current site
                  </p>
                </div>
                <label class="switch">
                  <input
                    type="checkbox"
                    checked={!allowedOnSite}
                    onchange={() => {
                      if (host)
                        send({
                          type: "setSiteAdblock",
                          host,
                          enabled: allowedOnSite,
                        });
                    }}
                  />
                  <span class="slider"></span>
                </label>
              </div>
            </div>
          {/if}

          <div class="section-title" style="margin-top: 18px;">
            Built-in Privacy Engine
          </div>
          <div class="card settings-card">
            <div class="filter-item">
              <div class="filter-status-dot"></div>
              <div>
                <div class="filter-name">EasyList & EasyPrivacy</div>
                <div class="filter-desc">
                  Network-level blocking & cosmetic filtering
                </div>
              </div>
            </div>
            <div class="filter-item">
              <div class="filter-status-dot"></div>
              <div>
                <div class="filter-name">uBlock Origin Filters</div>
                <div class="filter-desc">
                  Anti-malware & behavioral telemetry prevention
                </div>
              </div>
            </div>
          </div>

          <div class="card settings-card" style="margin-top: 14px;">
            <div class="field" style="border-bottom: none;">
              <div>
                <span class="field-label-bold">Cookies & Local Storage</span>
                <p class="field-desc">{cookiesCount} stored domain cookies</p>
              </div>
              <button
                class="danger-btn"
                onclick={() => send({ type: "clearCookies" })}
              >
                Clear Data
              </button>
            </div>
          </div>
        {:else if section === "preview"}
          <div class="section-title">Hover Preview & Link QuickLook</div>
          <div class="card settings-card">
            <div
              class="field"
              style="background: var(--surface-2); border-radius: 8px; padding: 10px 12px; margin-bottom: 8px;"
            >
              <div>
                <span class="field-label-bold" style="color: var(--accent);"
                  >✦ Space to Peek (Finder QuickLook)</span
                >
                <p class="field-desc">
                  Hover over any link and press <kbd
                    style="background: var(--surface); padding: 1px 4px; border-radius: 3px; border: 1px solid var(--border);"
                    >Space</kbd
                  >
                  or hold
                  <kbd
                    style="background: var(--surface); padding: 1px 4px; border-radius: 3px; border: 1px solid var(--border);"
                    >Shift</kbd
                  > to pop open an instant live card preview.
                </p>
              </div>
            </div>

            <div class="field switch-field">
              <div>
                <span class="field-label-bold">Require Modifier Key</span>
                <p class="field-desc">
                  Require ⌘ or Shift key to trigger preview on hover
                </p>
              </div>
              <label class="switch">
                <input
                  type="checkbox"
                  checked={previewModifierOnly}
                  onchange={(e) => {
                    previewModifierOnly = (e.currentTarget as HTMLInputElement)
                      .checked;
                    setting("previewModifierOnly", previewModifierOnly);
                  }}
                />
                <span class="slider"></span>
              </label>
            </div>

            <div class="field">
              <label for="h-delay">Hover Delay (ms)</label>
              <input
                id="h-delay"
                type="number"
                min="50"
                max="1500"
                step="50"
                value={hoverDelay}
                onchange={(e) => {
                  hoverDelay = Number(
                    (e.currentTarget as HTMLInputElement).value,
                  );
                  setting("hoverDelayMs", hoverDelay);
                }}
              />
            </div>

            <div class="field">
              <label for="p-width">Preview Width (px)</label>
              <input
                id="p-width"
                type="number"
                min="320"
                max="800"
                step="20"
                value={previewWidth}
                onchange={(e) => {
                  previewWidth = Number(
                    (e.currentTarget as HTMLInputElement).value,
                  );
                  setting("previewWidth", previewWidth);
                }}
              />
            </div>

            <div class="field">
              <label for="p-height">Preview Height (px)</label>
              <input
                id="p-height"
                type="number"
                min="200"
                max="600"
                step="20"
                value={previewHeight}
                onchange={(e) => {
                  previewHeight = Number(
                    (e.currentTarget as HTMLInputElement).value,
                  );
                  setting("previewHeight", previewHeight);
                }}
              />
            </div>

            <div class="field" style="border-bottom: none;">
              <label for="p-zoom">Preview Zoom Ratio</label>
              <input
                id="p-zoom"
                type="number"
                min="0.5"
                max="1.0"
                step="0.05"
                value={previewZoom}
                onchange={(e) => {
                  previewZoom = Number(
                    (e.currentTarget as HTMLInputElement).value,
                  );
                  setting("previewZoom", previewZoom);
                }}
              />
            </div>
          </div>
        {:else if section === "profiles"}
          <div class="section-title">Active Profiles</div>
          <ul class="profile-list">
            {#each profiles as p}
              <li
                class="profile-item card"
                class:active-profile={p === activeProfile}
              >
                <div class="profile-info">
                  <div class="profile-avatar">{p[0].toUpperCase()}</div>
                  <div class="profile-meta">
                    <div class="profile-name-row">
                      <span class="profile-name">{p}</span>
                      {#if p === activeProfile}
                        <span class="profile-badge">Active</span>
                      {/if}
                    </div>
                    <span class="profile-subtext">
                      {p === activeProfile
                        ? "Current workspace profile"
                        : "Isolated browsing profile"}
                    </span>
                  </div>
                </div>
                <div class="profile-actions">
                  {#if p !== activeProfile}
                    <button
                      class="switch-profile-btn"
                      onclick={() => handleSwitchProfile(p)}
                    >
                      Switch
                    </button>
                    <button
                      class="delete-profile-btn"
                      onclick={() => handleDeleteProfile(p)}
                      title="Delete profile"
                    >
                      <svg
                        viewBox="0 0 16 16"
                        fill="none"
                        stroke="currentColor"
                        class="action-icon"
                      >
                        <path
                          d="M3 4.5h10M5.5 4.5V3a1 1 0 011-1h3a1 1 0 011 1v1.5M6 7.5v5M10 7.5v5M4.5 4.5l.7 9a1.5 1.5 0 001.5 1.4h4.6a1.5 1.5 0 001.5-1.4l.7-9"
                          stroke-width="1.3"
                          stroke-linecap="round"
                          stroke-linejoin="round"
                        />
                      </svg>
                    </button>
                  {/if}
                </div>
              </li>
            {/each}
          </ul>

          <div class="card create-profile-card">
            <div class="create-profile-header">
              <span class="create-title">Create New Profile</span>
              <span class="create-desc"
                >Isolated cookies, history, tabs, and storage</span
              >
            </div>
            <div class="create-profile-input-wrap">
              <input
                type="text"
                placeholder="Profile name (e.g. Work, Research)"
                bind:value={newProfileName}
                onkeydown={(e) => {
                  if (e.key === "Enter") handleCreateProfile();
                }}
                class="create-profile-input"
              />
              <button class="create-profile-btn" onclick={handleCreateProfile}>
                Create
              </button>
            </div>
          </div>
        {:else if section === "extensions"}
          <div class="section-title">Install Chrome Extension (MV3)</div>
          <div class="card settings-card">
            <p class="field-desc" style="margin-bottom: 8px;">
              Supports Manifest V3 unpacked directories and .crx packages with
              isolated content scripts.
            </p>
            <div class="input-action-row">
              <input
                type="text"
                placeholder="/path/to/extension or extension.crx"
                bind:value={extensionInstallPath}
                class="text-input"
              />
              <button class="primary-btn" onclick={handleInstallExtension}
                >Install</button
              >
            </div>
          </div>

          <div class="section-title" style="margin-top: 18px;">
            Installed Extensions ({extensions.length})
          </div>
          {#if extensions.length === 0}
            <div class="card empty-card">
              <div class="empty-icon">🧩</div>
              <p class="note">
                No extensions installed yet. Enter an unpacked directory path
                above to install.
              </p>
            </div>
          {:else}
            <div class="extensions-list">
              {#each extensions as ext (ext.id)}
                <div class="card ext-card">
                  <div class="ext-header">
                    <div>
                      <div class="ext-title">
                        {ext.name}
                        <span class="ext-version">v{ext.version}</span>
                      </div>
                      <div class="ext-id">{ext.id}</div>
                    </div>
                    <div class="ext-controls">
                      <label class="switch">
                        <input
                          type="checkbox"
                          checked={ext.enabled}
                          onchange={() =>
                            handleToggleExtension(ext.id, ext.enabled)}
                        />
                        <span class="slider"></span>
                      </label>
                      <button
                        class="danger-icon-btn"
                        onclick={() => handleUninstallExtension(ext.id)}
                        title="Uninstall"
                      >
                        ✕
                      </button>
                    </div>
                  </div>

                  <div class="ext-badges">
                    {#if ext.fullySupported}
                      <span class="badge verified">Tier 1 Verified</span>
                    {:else}
                      <span class="badge partial">Partial MV3</span>
                    {/if}
                  </div>
                </div>
              {/each}
            </div>
          {/if}
        {:else if section === "shortcuts"}
          <div class="section-title">Navigation & Window</div>
          <div class="shortcut-list card settings-card">
            <div class="key-row">
              <kbd>⌘ L</kbd> <span>Focus address and search bar</span>
            </div>
            <div class="key-row">
              <kbd>⌘ T</kbd> <span>Open blank new tab</span>
            </div>
            <div class="key-row">
              <kbd>⌘ W</kbd> <span>Close current active tab</span>
            </div>
            <div class="key-row">
              <kbd>⌘ R</kbd> <span>Reload current page</span>
            </div>
            <div class="key-row">
              <kbd>Alt + ← / →</kbd> <span>Go back / forward in history</span>
            </div>
          </div>

          <div class="section-title" style="margin-top: 16px;">
            Peek Gestures & Signatures
          </div>
          <div class="shortcut-list card settings-card">
            <div class="key-row">
              <kbd>Space</kbd>
              <span>QuickLook preview hovered link (Finder style)</span>
            </div>
            <div class="key-row">
              <kbd>⇧ / ⌘ + Hover</kbd> <span>Trigger live link preview</span>
            </div>
            <div class="key-row">
              <kbd>Swipe Bar ⇄</kbd>
              <span>2-finger horizontal swipe top bar to switch tabs</span>
            </div>
            <div class="key-row">
              <kbd>Edge Swipe ⇥</kbd>
              <span>Swipe from trackpad left/right edges to open drawers</span>
            </div>
            <div class="key-row">
              <kbd>⌘ ⇧ [</kbd> <span>Toggle vertical tabs drawer</span>
            </div>
            <div class="key-row">
              <kbd>⌘ ⇧ ]</kbd> <span>Toggle settings & tools drawer</span>
            </div>
            <div class="key-row">
              <kbd>Esc</kbd> <span>Dismiss preview or close drawer</span>
            </div>
          </div>
        {:else if section === "about"}
          <div class="hero-brand" style="margin: 20px 0;">
            <img
              src="/logo.png"
              alt="Peek"
              class="drawer-logo"
              style="width: 48px; height: 48px;"
            />
            <h2 style="margin: 10px 0 2px; font-size: 19px; font-weight: 700;">
              Peek Browser
            </h2>
            <span style="font-size: 12px; color: var(--muted);"
              >v0.1.0 • Pure Rust Native Architecture</span
            >
          </div>
          <div class="card settings-card">
            <p class="note">
              Peek is built with zero framework bloat using direct <code
                >tao</code
              >
              and <code>wry</code> bindings, native system WebViews, Brave adblock
              engine, and an ephemeral hover preview architecture.
            </p>
          </div>
          <div class="card settings-card" style="margin-top: 12px;">
            <div class="tech-row">
              <span>Engine</span> <strong>tao + wry (WebKit/WebView2)</strong>
            </div>
            <div class="tech-row">
              <span>Database</span>
              <strong>SQLite FTS5 + Connection Pool</strong>
            </div>
            <div class="tech-row">
              <span>Adblock</span> <strong>Brave Adblock Engine (Rust)</strong>
            </div>
            <div class="tech-row">
              <span>Extensions</span> <strong>Manifest V3 Host Engine</strong>
            </div>
          </div>
        {/if}
      </div>

      <footer class="settings-footer">
        <div class="footer-meta">
          <span class="footer-brand">Peek</span>
          <span class="footer-version">v0.1.0</span>
        </div>

        <div class="section-title" style="margin-top: 16px;">Peek Gestures & Signatures</div>
        <div class="shortcut-list card settings-card">
          <div class="key-row"><kbd>Space</kbd> <span>QuickLook preview hovered link (Finder style)</span></div>
          <div class="key-row"><kbd>⇧ / ⌘ + Hover</kbd> <span>Trigger live link preview</span></div>
          <div class="key-row"><kbd>Swipe Bar ⇄</kbd> <span>2-finger horizontal swipe top bar to switch tabs</span></div>
          <div class="key-row"><kbd>Edge Swipe ⇥</kbd> <span>Swipe from trackpad left/right edges to open drawers</span></div>
          <div class="key-row"><kbd>⌘ ⇧ [</kbd> <span>Toggle vertical tabs drawer</span></div>
          <div class="key-row"><kbd>⌘ ⇧ ]</kbd> <span>Toggle settings & tools drawer</span></div>
          <div class="key-row"><kbd>Esc</kbd> <span>Dismiss preview or close drawer</span></div>
        </div>

      {:else if section === 'about'}
        <div class="about-hero">
          <img src="/logo.png" alt="Peek" class="about-logo" />
          <h2 class="about-name">Peek Browser</h2>
          <span class="about-tagline">v0.1.0 • Pure Rust Native Architecture</span>
        </div>

        <!-- Star on GitHub & Community Card -->
        <div class="card settings-card github-star-card">
          <div class="github-card-header">
            <div class="github-badge-row">
              <svg class="github-icon" viewBox="0 0 24 24" width="18" height="18" fill="currentColor">
                <path d="M12 0C5.37 0 0 5.37 0 12c0 5.31 3.435 9.795 8.205 11.385.6.105.825-.255.825-.57 0-.285-.015-1.23-.015-2.235-3.015.555-3.795-.735-4.035-1.41-.135-.345-.72-1.41-1.23-1.695-.42-.225-1.02-.78-.015-.795.945-.015 1.62.87 1.845 1.23 1.08 1.815 2.805 1.305 3.495.99.105-.78.42-1.305.765-1.605-2.67-.3-5.46-1.335-5.46-5.925 0-1.305.465-2.385 1.23-3.225-.12-.3-.54-1.53.12-3.18 0 0 1.005-.315 3.3 1.23.96-.27 1.98-.405 3-.405s2.04.135 3 .405c2.295-1.56 3.3-1.23 3.3-1.23.66 1.65.24 2.88.12 3.18.765.84 1.23 1.905 1.23 3.225 0 4.605-2.805 5.625-5.475 5.925.435.375.81 1.095.81 2.22 0 1.605-.015 2.895-.015 3.3 0 .315.225.69.825.57A12.02 12.02 0 0024 12c0-6.63-5.37-12-12-12z"/>
              </svg>
              <span class="github-title">Open Source on GitHub</span>
            </div>
            <span class="github-stars-badge">⭐ Star</span>
          </div>

          <p class="github-card-desc">
            If you like Peek, please give our repository a star on GitHub! Your support helps independent, lightweight browser innovation grow.
          </p>

          <div class="github-actions">
            <button
              class="btn-star-repo"
              type="button"
              onclick={() => {
                send({ type: 'openUrl', url: 'https://github.com/senapati484/peek' });
                close();
              }}
              title="Give Peek a star on GitHub"
            >
              <span class="star-sparkle">⭐</span>
              <span class="star-label">Star Peek on GitHub</span>
              <span class="out-arrow">↗</span>
            </button>
            <button
              class="btn-repo-issues"
              type="button"
              onclick={() => {
                send({ type: 'openUrl', url: 'https://github.com/senapati484/peek/issues' });
                close();
              }}
              title="Report an issue or suggest a feature on GitHub"
            >
              <span>Feedback & Issues</span>
              <span class="out-arrow">↗</span>
            </button>
          </div>
        </div>

        <div class="card settings-card" style="margin-top: 12px;">
          <p class="note">
            Peek is built with zero framework bloat using direct <code>tao</code> and <code>wry</code> bindings, native system WebViews, Brave adblock engine, and an ephemeral hover preview architecture.
          </p>
        </div>

        <div class="card settings-card" style="margin-top: 12px;">
          <div class="tech-row"><span>Engine</span> <strong>tao + wry (WebKit/WebView2)</strong></div>
          <div class="tech-row"><span>Database</span> <strong>SQLite FTS5 + Connection Pool</strong></div>
          <div class="tech-row"><span>Adblock</span> <strong>Brave Adblock Engine (Rust)</strong></div>
          <div class="tech-row"><span>Extensions</span> <strong>Manifest V3 Host Engine</strong></div>
          <div class="tech-row">
            <span>Repository</span>
            <button
              class="repo-inline-link"
              type="button"
              onclick={() => {
                send({ type: 'openUrl', url: 'https://github.com/senapati484/peek' });
                close();
              }}
            >
              github.com/senapati484/peek ↗
            </button>
          </div>
        </div>
      {/if}
    </div>

    <footer class="settings-footer">
      <div class="footer-meta">
        <span class="footer-brand">Peek</span>
        <span class="footer-version">v0.1.0</span>
        <span class="footer-dot">•</span>
        <button
          class="footer-gh-link"
          type="button"
          onclick={() => {
            send({ type: 'openUrl', url: 'https://github.com/senapati484/peek' });
            close();
          }}
          title="Open GitHub Repository"
        >
          ⭐ Star on GitHub
        </button>
      </div>
      <button class="quit-action-btn" onclick={() => send({ type: 'quit' })}>
        <svg viewBox="0 0 16 16" fill="none" stroke="currentColor" class="quit-icon">
          <path d="M8 2v5M4.93 4.93a5 5 0 106.14 0" stroke-width="1.5" stroke-linecap="round" />
        </svg>
        Quit Peek
      </button>
    </footer>
  {/if}
</aside>
{/key}

<style>
  .drawer {
    display: flex;
    flex-direction: column;
    height: 100vh;
    background: var(--panel);
    box-shadow: var(--shadow-lg);
    animation: drawerSlide 0.2s cubic-bezier(0.16, 1, 0.3, 1) both;
    will-change: transform, opacity;
  }

  .drawer.left {
    border-inline-end: 1px solid var(--border);
    --slide-dir: -28px;
  }

  .drawer.right {
    border-inline-start: 1px solid var(--border);
    --slide-dir: 28px;
  }

  @keyframes drawerSlide {
    0% {
      opacity: 0.1;
      transform: translateX(var(--slide-dir, 28px));
    }
    100% {
      opacity: 1;
      transform: translateX(0);
    }
  }

  /* Headers */
  .tabs-header,
  .settings-header {
    padding: 12px 14px 10px;
    border-bottom: 1px solid var(--border);
    flex-shrink: 0;
    display: flex;
    flex-direction: column;
    gap: 8px;
  }

  .brand-row {
    display: flex;
    align-items: center;
    justify-content: space-between;
  }

  .tabs-brand {
    display: flex;
    align-items: center;
    gap: 8px;
  }

  .left-nav-pills {
    display: grid;
    grid-template-columns: repeat(5, 1fr);
    gap: 3px;
    padding: 3px;
    background: var(--surface-2);
    border: 1px solid var(--border-subtle);
    border-radius: 9px;
  }

  .left-pill {
    all: unset;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    gap: 4px;
    padding: 5px 2px;
    border-radius: 6px;
    font-size: 11px;
    font-weight: 550;
    color: var(--muted);
    cursor: pointer;
    transition: all 0.14s ease;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .left-pill:hover {
    color: var(--text);
    background: color-mix(in srgb, var(--surface) 60%, transparent);
  }

  .left-pill.active {
    background: var(--surface);
    color: var(--text);
    box-shadow: var(--shadow-sm);
    font-weight: 600;
  }

  .left-pill .tab-count-badge {
    font-size: 9.5px;
    padding: 0 4px;
    border-radius: 999px;
    background: var(--surface-2);
    color: var(--muted);
  }

  .left-pill.active .tab-count-badge {
    color: var(--text);
  }

  .drawer-logo {
    width: 20px;
    height: 20px;
    display: block;
    user-select: none;
    -webkit-user-drag: none;
    filter: drop-shadow(0 1px 3px rgba(0, 0, 0, 0.4));
  }

  .drawer-title {
    font-size: 13.5px;
    font-weight: 650;
    letter-spacing: -0.01em;
  }

  .tab-count-badge {
    font-size: 10.5px;
    padding: 1px 6px;
    border-radius: 999px;
    background: var(--surface-2);
    color: var(--muted);
    font-weight: 600;
  }

  .icon-close-btn {
    width: 26px;
    height: 26px;
    border-radius: 50%;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    color: var(--muted);
    background: var(--surface-2);
    border: 1px solid var(--border-subtle);
    cursor: pointer;
    transition: all 0.15s ease;
  }

  .icon-close-btn:hover {
    color: var(--text);
    background: var(--hover);
    border-color: var(--border);
  }

  .close-icon {
    width: 12px;
    height: 12px;
  }

  /* Filter */
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
  }

  /* Settings Navigation */
  .settings-nav {
    display: grid;
    grid-template-columns: repeat(4, 1fr);
    gap: 4px;
    padding: 3px;
    background: var(--surface-2);
    border: 1px solid var(--border-subtle);
    border-radius: 9px;
  }

  .settings-nav button {
    padding: 5px 2px;
    border-radius: 6px;
    color: var(--muted);
    font-size: 11px;
    font-weight: 500;
    text-align: center;
    transition: all 0.14s ease;
    border: 1px solid transparent;
    background: transparent;
    cursor: pointer;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .settings-nav button:hover {
    color: var(--text);
    background: color-mix(in srgb, var(--surface) 60%, transparent);
  }

  .settings-nav button.on {
    background: var(--surface);
    color: var(--text);
    font-weight: 600;
    border-color: var(--border);
    box-shadow: 0 1px 3px rgba(0, 0, 0, 0.2);
  }

  /* Body */
  .body {
    flex: 1;
    overflow-y: auto;
    padding: 12px;
  }

  /* Memory Saver Bar */
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

  /* Tabs List & Pinned Subgroups */
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
    flex: 1;
    display: flex;
    align-items: center;
    gap: 8px;
    min-width: 0;
    height: 32px;
    padding: 0 8px;
    text-align: left;
    font-size: 12.5px;
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

  /* History & Bookmarks Views */
  .history-body,
  .bookmarks-body {
    flex: 1;
    overflow-y: auto;
    padding: 8px;
  }

  .history-list,
  .bookmarks-list {
    list-style: none;
    margin: 0;
    padding: 0;
    display: flex;
    flex-direction: column;
    gap: 2px;
  }

  .history-item,
  .bookmark-item {
    display: flex;
    align-items: center;
    border-radius: 8px;
    transition: background 0.12s ease;
    position: relative;
  }

  .history-item:hover,
  .bookmark-item:hover {
    background: var(--hover);
  }

  .history-row,
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

  .history-info,
  .bookmark-info {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
    gap: 2px;
  }

  .history-title,
  .bookmark-title {
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

  .history-host,
  .bookmark-host {
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

  .history-delete-btn,
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

  .history-item:hover .history-delete-btn,
  .bookmark-item:hover .bookmark-remove-btn {
    opacity: 1;
  }

  .history-delete-btn:hover,
  .bookmark-remove-btn:hover {
    background: #ef4444;
    color: #fff;
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
    transition: background 0.12s ease;
  }

  .clear-history-btn:hover {
    background: rgba(239, 68, 68, 0.12);
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

  .pill.sleep {
    background: color-mix(in srgb, var(--surface-2) 90%, transparent);
    color: var(--muted);
    border: 1px solid var(--border-subtle);
    font-size: 9px;
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
    transition:
      opacity 0.12s ease,
      background 0.12s ease,
      color 0.12s ease;
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
    transition:
      opacity 0.12s ease,
      background 0.12s ease,
      color 0.12s ease;
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
    flex: 0 0 22px;
    height: 22px;
    margin-right: 4px;
    border-radius: 5px;
    color: var(--muted);
    font-size: 14px;
    line-height: 1;
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
    transition:
      background 0.12s ease,
      border-color 0.12s ease;
  }

  .clear-filter-btn:hover {
    background: var(--hover);
    border-color: var(--accent);
  }

  /* Footers */
  .tabs-footer,
  .settings-footer {
    padding: 10px 12px;
    border-top: 1px solid var(--border);
    flex-shrink: 0;
  }

  .new-tab-btn {
    display: flex;
    align-items: center;
    justify-content: space-between;
    width: 100%;
    padding: 7px 10px;
    border: 1px dashed var(--border);
    border-radius: 8px;
    color: var(--muted);
    font-size: 12px;
    transition:
      border-color 0.15s ease,
      color 0.15s ease;
  }

  .new-tab-btn:hover {
    color: var(--text);
    border-color: var(--accent);
  }

  .plus {
    font-size: 14px;
    font-weight: 700;
  }

  /* Settings Controls */
  .section-title {
    font-size: 11.5px;
    font-weight: 650;
    color: var(--text);
    margin-bottom: 7px;
    letter-spacing: -0.01em;
  }

  .card {
    padding: 12px 14px;
    border: 1px solid var(--border);
    border-radius: var(--radius);
    background: var(--surface);
    box-shadow: var(--shadow-sm);
  }

  .stat-card {
    text-align: center;
    padding: 16px;
    background: radial-gradient(
      circle at 50% 20%,
      var(--accent-soft) 0%,
      var(--surface) 80%
    );
  }

  .stat-big {
    font-size: 36px;
    font-weight: 700;
    letter-spacing: -0.02em;
    color: var(--accent);
    line-height: 1.1;
  }

  .stat-label {
    margin-top: 4px;
    color: var(--muted);
    font-size: 11.5px;
  }

  .note {
    margin: 0;
    color: var(--muted);
    font-size: 11.5px;
    line-height: 1.45;
  }

  .field {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 12px;
    padding: 8px 0;
    border-bottom: 1px solid var(--border-subtle);
    font-size: 12px;
  }

  .field-desc {
    margin: 2px 0 0;
    font-size: 10.5px;
    color: var(--muted);
  }

  .field label {
    color: var(--text);
    font-weight: 500;
  }

  .field-label-bold {
    font-weight: 600;
    color: var(--text);
  }

  .field input[type="number"],
  .field select {
    padding: 4px 7px;
    border: 1px solid var(--border);
    border-radius: 6px;
    background: var(--panel);
    color: var(--text);
    font-size: 12px;
    text-align: right;
  }

  /* iOS/macOS Slider Switch */
  .switch {
    position: relative;
    display: inline-block;
    width: 34px;
    height: 18px;
    flex-shrink: 0;
  }

  .switch input {
    opacity: 0;
    width: 0;
    height: 0;
  }

  .slider {
    position: absolute;
    cursor: pointer;
    inset: 0;
    background-color: var(--border);
    transition: 0.18s cubic-bezier(0.16, 1, 0.3, 1);
    border-radius: 20px;
  }

  .slider:before {
    position: absolute;
    content: "";
    height: 14px;
    width: 14px;
    left: 2px;
    bottom: 2px;
    background-color: white;
    transition: 0.18s cubic-bezier(0.16, 1, 0.3, 1);
    border-radius: 50%;
    box-shadow: 0 1px 2px rgba(0, 0, 0, 0.3);
  }

  input:checked + .slider {
    background-color: var(--accent);
  }

  input:checked + .slider:before {
    transform: translateX(16px);
  }

  /* Theme Choices */
  .theme-options {
    display: flex;
    gap: 8px;
  }

  .choice-btn {
    flex: 1;
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 4px;
    padding: 8px 6px;
    border: 1px solid var(--border);
    border-radius: 8px;
    font-size: 11.5px;
    color: var(--text);
    background: var(--surface);
    transition:
      border-color 0.15s ease,
      background 0.15s ease;
  }

  .choice-btn.selected {
    border-color: var(--accent);
    background: var(--accent-soft);
    color: var(--accent);
    font-weight: 600;
  }

  .theme-icon {
    font-size: 15px;
  }

  .accent-palette {
    display: flex;
    gap: 12px;
    padding: 4px 0;
  }

  .accent-dot {
    width: 26px;
    height: 26px;
    border-radius: 50%;
    border: 2px solid transparent;
    cursor: pointer;
    transition:
      transform 0.15s ease,
      border-color 0.15s ease;
  }

  .accent-dot:hover {
    transform: scale(1.15);
  }

  .accent-dot.selected {
    border-color: var(--text);
    transform: scale(1.12);
  }

  /* Adblock & Privacy items */
  .filter-item {
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 7px 0;
    border-bottom: 1px solid var(--border-subtle);
  }

  .filter-item:last-child {
    border-bottom: none;
  }

  .filter-status-dot {
    width: 8px;
    height: 8px;
    border-radius: 50%;
    background: #10b981;
    box-shadow: 0 0 6px rgba(16, 185, 129, 0.5);
  }

  .filter-name {
    font-size: 12px;
    font-weight: 600;
    color: var(--text);
  }

  .filter-desc {
    font-size: 10.5px;
    color: var(--muted);
  }

  /* Profiles */
  .profile-list {
    list-style: none;
    margin: 0 0 12px;
    padding: 0;
    display: flex;
    flex-direction: column;
    gap: 8px;
  }

  .profile-item {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 11px 13px;
    border-radius: 12px;
    background: var(--surface);
    border: 1px solid var(--border);
    transition: all 0.15s ease;
  }

  .profile-item.active-profile {
    border-color: color-mix(in srgb, var(--accent) 45%, var(--border));
    background: color-mix(in srgb, var(--accent) 8%, var(--surface));
    box-shadow: 0 2px 8px color-mix(in srgb, var(--accent) 12%, transparent);
  }

  .profile-info {
    display: flex;
    align-items: center;
    gap: 11px;
    min-width: 0;
  }

  .profile-avatar {
    width: 34px;
    height: 34px;
    border-radius: 10px;
    background: linear-gradient(
      135deg,
      var(--accent),
      color-mix(in srgb, var(--accent) 55%, #000)
    );
    color: #ffffff;
    font-weight: 700;
    font-size: 13px;
    display: flex;
    align-items: center;
    justify-content: center;
    flex-shrink: 0;
    box-shadow: 0 2px 6px rgba(0, 0, 0, 0.25);
  }

  .profile-meta {
    display: flex;
    flex-direction: column;
    gap: 2px;
    min-width: 0;
  }

  .profile-name-row {
    display: flex;
    align-items: center;
    gap: 7px;
  }

  .profile-name {
    font-size: 13px;
    font-weight: 600;
    color: var(--text);
  }

  .profile-badge {
    font-size: 9.5px;
    color: var(--accent);
    background: color-mix(in srgb, var(--accent) 20%, transparent);
    border: 1px solid color-mix(in srgb, var(--accent) 30%, transparent);
    padding: 1px 7px;
    border-radius: 999px;
    font-weight: 650;
    letter-spacing: 0.02em;
    text-transform: uppercase;
  }

  .profile-subtext {
    font-size: 11px;
    color: var(--muted);
  }

  .profile-actions {
    display: flex;
    align-items: center;
    gap: 6px;
    flex-shrink: 0;
  }

  .switch-profile-btn {
    padding: 5px 12px;
    border-radius: 999px;
    background: var(--surface-2);
    border: 1px solid var(--border);
    font-size: 11.5px;
    font-weight: 600;
    color: var(--text);
    cursor: pointer;
    transition: all 0.15s ease;
  }

  .switch-profile-btn:hover {
    background: var(--accent);
    border-color: var(--accent);
    color: #ffffff;
  }

  .delete-profile-btn {
    width: 28px;
    height: 28px;
    border-radius: 8px;
    display: flex;
    align-items: center;
    justify-content: center;
    color: var(--muted);
    background: transparent;
    border: none;
    cursor: pointer;
    transition: all 0.15s ease;
  }

  .delete-profile-btn:hover {
    color: #ef4444;
    background: color-mix(in srgb, #ef4444 14%, transparent);
  }

  .action-icon {
    width: 14px;
    height: 14px;
  }

  .create-profile-card {
    padding: 14px;
    border-radius: 12px;
    background: var(--surface);
    border: 1px solid var(--border);
    margin-top: 10px;
  }

  .create-profile-header {
    display: flex;
    flex-direction: column;
    gap: 2px;
    margin-bottom: 10px;
  }

  .create-title {
    font-size: 12px;
    font-weight: 650;
    color: var(--text);
  }

  .create-desc {
    font-size: 10.5px;
    color: var(--muted);
  }

  .create-profile-input-wrap {
    display: flex;
    gap: 8px;
  }

  .create-profile-input {
    flex: 1;
    padding: 7px 11px;
    border-radius: 8px;
    background: var(--panel);
    border: 1px solid var(--border);
    color: var(--text);
    font-size: 12px;
    outline: none;
    transition: border-color 0.15s ease;
  }

  .create-profile-input:focus {
    border-color: var(--accent);
  }

  .create-profile-btn {
    padding: 7px 14px;
    border-radius: 8px;
    background: var(--accent);
    color: #ffffff;
    font-size: 12px;
    font-weight: 600;
    border: none;
    cursor: pointer;
    transition: filter 0.15s ease;
  }

  .create-profile-btn:hover {
    filter: brightness(1.1);
  }

  .danger-btn {
    padding: 4px 8px;
    border-radius: 5px;
    background: color-mix(in srgb, #ef4444 14%, transparent);
    color: #ef4444;
    font-size: 11px;
    font-weight: 600;
    transition: background 0.12s ease;
  }

  .danger-btn:hover {
    background: color-mix(in srgb, #ef4444 24%, transparent);
  }

  .danger-icon-btn {
    padding: 3px 6px;
    border-radius: 4px;
    color: var(--muted);
    font-size: 11px;
  }

  .danger-icon-btn:hover {
    color: #ef4444;
    background: color-mix(in srgb, #ef4444 14%, transparent);
  }

  .input-action-row {
    display: flex;
    gap: 6px;
    margin-top: 4px;
  }

  .text-input {
    flex: 1;
    padding: 5px 8px;
    border: 1px solid var(--border);
    border-radius: 6px;
    background: var(--panel);
    color: var(--text);
    font-size: 12px;
    outline: none;
  }

  .text-input:focus {
    border-color: var(--accent);
  }

  .primary-btn {
    padding: 5px 12px;
    border-radius: 6px;
    background: var(--accent);
    color: white;
    font-size: 12px;
    font-weight: 600;
    transition: opacity 0.15s ease;
  }

  .primary-btn:hover {
    opacity: 0.9;
  }

  /* Extensions */
  .extensions-list {
    display: flex;
    flex-direction: column;
    gap: 8px;
  }

  .ext-card {
    padding: 10px 12px;
  }

  .ext-header {
    display: flex;
    align-items: flex-start;
    justify-content: space-between;
  }

  .ext-title {
    font-size: 12.5px;
    font-weight: 600;
    color: var(--text);
  }

  .ext-version {
    font-size: 10.5px;
    color: var(--muted);
    font-weight: normal;
  }

  .ext-id {
    font-size: 10px;
    color: var(--muted);
    font-family: monospace;
    margin-top: 1px;
  }

  .ext-controls {
    display: flex;
    align-items: center;
    gap: 8px;
  }

  .ext-badges {
    display: flex;
    flex-wrap: wrap;
    gap: 4px;
    margin-top: 7px;
  }

  .badge {
    padding: 1px 6px;
    border-radius: 4px;
    font-size: 9.5px;
    font-weight: 600;
    text-transform: uppercase;
  }

  .badge.verified {
    background: color-mix(in srgb, #10b981 18%, transparent);
    color: #10b981;
  }

  .badge.partial {
    background: color-mix(in srgb, #f59e0b 18%, transparent);
    color: #f59e0b;
  }

  .empty-card {
    text-align: center;
    padding: 20px;
  }

  .empty-icon {
    font-size: 24px;
    margin-bottom: 6px;
  }

  /* Shortcuts */
  .shortcut-list {
    display: flex;
    flex-direction: column;
    gap: 7px;
  }

  .key-row {
    display: flex;
    align-items: center;
    justify-content: space-between;
    font-size: 11.5px;
    color: var(--text-secondary);
  }

  /* Tech Info */
  .tech-row {
    display: flex;
    justify-content: space-between;
    font-size: 11.5px;
    padding: 4px 0;
    border-bottom: 1px solid var(--border-subtle);
  }

  .tech-row:last-child {
    border-bottom: none;
  }

  .tech-row span {
    color: var(--muted);
  }

  /* Settings Footer */
  .settings-footer {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 10px 14px;
    border-top: 1px solid var(--border);
    background: var(--panel);
    flex-shrink: 0;
  }

  .footer-meta {
    display: flex;
    align-items: center;
    gap: 6px;
    font-size: 11px;
    color: var(--muted);
  }

  .footer-brand {
    font-weight: 600;
    color: var(--text);
  }

  .footer-version {
    font-size: 10.5px;
    opacity: 0.8;
  }

  .quit-action-btn {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    padding: 5px 11px;
    border-radius: 7px;
    background: transparent;
    border: 1px solid var(--border);
    color: var(--muted);
    font-size: 11.5px;
    font-weight: 500;
    cursor: pointer;
    transition: all 0.15s ease;
  }

  .quit-action-btn:hover {
    color: #ef4444;
    border-color: color-mix(in srgb, #ef4444 35%, var(--border));
    background: color-mix(in srgb, #ef4444 10%, transparent);
  }

  .quit-icon {
    width: 12px;
    height: 12px;
  }

  /* Spaces Filter Strip */
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

  /* Tab space indicator */
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

  /* Spaces View */
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

  .spaces-body {
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

  /* Scratchpad View */
  .scratchpad-header {
    padding: 12px 14px 10px;
    display: flex;
    align-items: center;
    justify-content: space-between;
    border-bottom: 1px solid var(--border-subtle);
    gap: 8px;
  }

  .scratchpad-meta {
    display: flex;
    flex-direction: column;
    gap: 1px;
  }

  .scratchpad-badge {
    font-size: 10px;
    font-weight: 700;
    letter-spacing: 0.08em;
    color: var(--accent);
  }

  .scratchpad-count {
    font-size: 11px;
    color: var(--muted);
  }

  .scratchpad-actions {
    display: flex;
    align-items: center;
    gap: 5px;
  }

  .scratch-action-btn {
    all: unset;
    cursor: pointer;
    display: inline-flex;
    align-items: center;
    padding: 3px 8px;
    border-radius: 6px;
    font-size: 11px;
    font-weight: 500;
    color: var(--text);
    background: var(--surface-2);
    border: 1px solid var(--border);
    transition: all 0.12s ease;
  }

  .scratch-action-btn:hover {
    background: var(--hover);
  }

  .scratch-action-btn.danger:hover {
    color: #ef4444;
    border-color: #ef4444;
  }

  /* GitHub Star Card in About Section */
  .about-hero {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    margin: 16px 0 18px;
    text-align: center;
  }

  .about-logo {
    width: 48px;
    height: 48px;
    display: block;
    margin-bottom: 8px;
    filter: drop-shadow(0 4px 12px rgba(0, 0, 0, 0.25));
  }

  .about-name {
    margin: 0;
    font-size: 18px;
    font-weight: 700;
    color: var(--text);
    letter-spacing: -0.02em;
  }

  .about-tagline {
    font-size: 11.5px;
    color: var(--muted);
    margin-top: 3px;
  }

  .github-star-card {
    background: linear-gradient(145deg, var(--surface), color-mix(in srgb, var(--accent) 6%, var(--surface)));
    border: 1px solid color-mix(in srgb, var(--accent) 30%, var(--border));
    padding: 14px 16px;
    border-radius: var(--radius);
    box-shadow: var(--shadow-sm);
  }

  .github-card-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    margin-bottom: 8px;
  }

  .github-badge-row {
    display: flex;
    align-items: center;
    gap: 8px;
  }

  .github-icon {
    color: var(--text);
  }

  .github-title {
    font-size: 13px;
    font-weight: 650;
    color: var(--text);
  }

  .github-stars-badge {
    font-size: 10px;
    font-weight: 700;
    color: #eab308;
    background: rgba(234, 179, 8, 0.14);
    border: 1px solid rgba(234, 179, 8, 0.28);
    padding: 2px 7px;
    border-radius: 999px;
    letter-spacing: 0.02em;
  }

  .github-card-desc {
    font-size: 12px;
    line-height: 1.5;
    color: var(--text-secondary);
    margin: 0 0 12px;
  }

  .github-actions {
    display: flex;
    flex-direction: column;
    gap: 7px;
  }

  .btn-star-repo {
    all: unset;
    display: flex;
    align-items: center;
    justify-content: center;
    gap: 8px;
    width: 100%;
    padding: 8px 12px;
    border-radius: 8px;
    background: var(--accent);
    color: #ffffff;
    font-size: 12.5px;
    font-weight: 600;
    cursor: pointer;
    box-shadow: 0 2px 8px var(--accent-glow);
    transition: all 0.16s ease;
    box-sizing: border-box;
  }

  .btn-star-repo:hover {
    filter: brightness(1.1);
    transform: translateY(-1px);
    box-shadow: 0 4px 14px var(--accent-glow);
  }

  .star-sparkle {
    font-size: 13px;
  }

  .out-arrow {
    font-size: 12px;
    opacity: 0.8;
  }

  .btn-repo-issues {
    all: unset;
    display: flex;
    align-items: center;
    justify-content: center;
    gap: 6px;
    width: 100%;
    padding: 6px 12px;
    border-radius: 8px;
    background: var(--surface-2);
    border: 1px solid var(--border);
    color: var(--text-secondary);
    font-size: 12px;
    font-weight: 500;
    cursor: pointer;
    transition: all 0.14s ease;
    box-sizing: border-box;
  }

  .btn-repo-issues:hover {
    background: var(--hover);
    color: var(--text);
  }

  .repo-inline-link {
    all: unset;
    color: var(--accent);
    cursor: pointer;
    font-size: 11.5px;
    font-weight: 550;
    transition: text-decoration 0.12s ease;
  }

  .repo-inline-link:hover {
    text-decoration: underline;
  }

  .footer-dot {
    color: var(--border);
    font-size: 9px;
  }

  .footer-gh-link {
    all: unset;
    cursor: pointer;
    font-size: 11px;
    font-weight: 550;
    color: var(--muted);
    transition: color 0.14s ease;
  }

  .footer-gh-link:hover {
    color: var(--accent);
  }

  .scratchpad-body {
    flex: 1;
    display: flex;
    flex-direction: column;
    padding: 10px;
    box-sizing: border-box;
  }

  .scratchpad-editor {
    all: unset;
    box-sizing: border-box;
    width: 100%;
    flex: 1;
    min-height: 380px;
    background: transparent;
    color: var(--text);
    font-family: -apple-system, BlinkMacSystemFont, "SF Pro Text", "Segoe UI",
      sans-serif;
    font-size: 13px;
    line-height: 1.6;
    resize: none;
    white-space: pre-wrap;
    word-break: break-word;
  }

  .scratchpad-editor::placeholder {
    color: var(--muted);
    opacity: 0.6;
    line-height: 1.5;
  }
</style>
