//! Tab state machine with LRU discard.
//!
//! A [`TabManager`] owns no webviews. It returns [`TabAction`]s that the app
//! layer applies to the engine, which makes every policy here unit-testable.

use std::time::Duration;

use peek_ipc::{TabId, TabInfo};
use peek_store::StoredTab;

use crate::{now_ms, Timestamp};

/// Default number of simultaneously live tab webviews.
pub const DEFAULT_MAX_LIVE: usize = 5;
/// Default idle time before a background tab is discarded.
pub const DEFAULT_IDLE_DISCARD: Duration = Duration::from_secs(10 * 60);

#[derive(Debug, Clone, PartialEq)]
pub struct Tab {
    pub id: TabId,
    pub url: String,
    pub title: String,
    pub favicon: Option<String>,
    pub scroll_y: f64,
    pub last_active: Timestamp,
    pub loading: bool,
    pub audible: bool,
    pub form_dirty: bool,
    pub pinned: bool,
    /// Whether a webview currently exists for this tab.
    pub live: bool,
}

impl Tab {
    fn new(id: TabId, url: impl Into<String>, now: Timestamp) -> Self {
        Self {
            id,
            url: url.into(),
            title: String::new(),
            favicon: None,
            scroll_y: 0.0,
            last_active: now,
            loading: false,
            audible: false,
            form_dirty: false,
            pinned: false,
            live: true,
        }
    }

    pub fn is_discardable(&self) -> bool {
        self.live && !self.pinned && !self.audible && !self.form_dirty && !self.loading
    }

    pub fn to_info(&self, active: Option<TabId>) -> TabInfo {
        TabInfo {
            id: self.id,
            url: self.url.clone(),
            title: if self.title.is_empty() {
                self.url.clone()
            } else {
                self.title.clone()
            },
            favicon: self.favicon.clone(),
            discarded: !self.live,
            loading: self.loading,
            audible: self.audible,
            pinned: self.pinned,
            active: active == Some(self.id),
        }
    }

    pub fn to_stored(&self, position: i64) -> StoredTab {
        StoredTab {
            id: self.id,
            url: self.url.clone(),
            title: self.title.clone(),
            favicon: self.favicon.clone(),
            scroll_y: self.scroll_y,
            position,
            last_active: self.last_active,
            pinned: self.pinned,
        }
    }
}

/// Instructions for the app layer, produced in execution order.
#[derive(Debug, Clone, PartialEq)]
pub enum TabAction {
    /// Create a webview for the tab and load `url`.
    Create { id: TabId, url: String },
    /// Destroy the tab's webview (discard or close).
    Destroy { id: TabId },
    /// Show or hide a live tab webview.
    Show { id: TabId, visible: bool },
    /// Load a URL in an existing webview.
    Load { id: TabId, url: String },
    /// Restore the scroll offset after a discarded tab reloads.
    Scroll { id: TabId, y: f64 },
}

#[derive(Debug, Clone)]
pub struct TabConfig {
    pub max_live: usize,
    pub idle_discard: Duration,
    pub start_blank: bool,
}

impl Default for TabConfig {
    fn default() -> Self {
        Self {
            max_live: DEFAULT_MAX_LIVE,
            idle_discard: DEFAULT_IDLE_DISCARD,
            start_blank: false,
        }
    }
}

pub struct TabManager {
    tabs: Vec<Tab>,
    active: Option<TabId>,
    next_id: TabId,
    config: TabConfig,
}

impl TabManager {
    pub fn new(config: TabConfig) -> Self {
        Self {
            tabs: Vec::new(),
            active: None,
            next_id: 1,
            config,
        }
    }

    /// Restore a session. Every tab starts discarded except the active one.
    pub fn restore(config: TabConfig, stored: Vec<StoredTab>, active: Option<TabId>) -> (Self, Vec<TabAction>) {
        let mut manager = Self::new(config);
        if stored.is_empty() {
            let (_, actions) = manager.create_tab(crate::url::blank_page());
            return (manager, actions);
        }
        let mut max_id = 0;
        for stored in stored {
            max_id = max_id.max(stored.id);
            manager.tabs.push(Tab {
                id: stored.id,
                url: stored.url,
                title: stored.title,
                favicon: stored.favicon,
                scroll_y: stored.scroll_y,
                last_active: stored.last_active,
                loading: false,
                audible: false,
                form_dirty: false,
                pinned: stored.pinned,
                live: false,
            });
        }
        manager.next_id = max_id + 1;

        let chosen = active
            .filter(|id| manager.tabs.iter().any(|t| t.id == *id))
            .or_else(|| manager.tabs.last().map(|t| t.id));
        let mut actions = Vec::new();
        if let Some(id) = chosen {
            actions = manager.activate(id);
        }
        (manager, actions)
    }

    pub fn config(&self) -> &TabConfig {
        &self.config
    }

    pub fn set_config(&mut self, config: TabConfig) {
        self.config = config;
        if self.config.max_live == 0 {
            self.config.max_live = 1;
        }
    }

    pub fn tabs(&self) -> &[Tab] {
        &self.tabs
    }

    pub fn active_id(&self) -> Option<TabId> {
        self.active
    }

    pub fn active(&self) -> Option<&Tab> {
        self.active.and_then(|id| self.tab(id))
    }

    pub fn tab(&self, id: TabId) -> Option<&Tab> {
        self.tabs.iter().find(|t| t.id == id)
    }

    pub fn live_count(&self) -> usize {
        self.tabs.iter().filter(|t| t.live).count()
    }

    /// Create a tab and make it active.
    pub fn create_tab(&mut self, url: &str) -> (TabId, Vec<TabAction>) {
        let id = self.next_id;
        self.next_id += 1;
        let tab = Tab::new(id, url, now_ms());
        self.tabs.push(tab);

        let mut actions = self.hide_active();
        self.active = Some(id);
        actions.push(TabAction::Create {
            id,
            url: url.to_string(),
        });
        actions.extend(self.enforce_live_limit());
        (id, actions)
    }

    /// Activate a tab, creating its webview again if it was discarded.
    pub fn activate(&mut self, id: TabId) -> Vec<TabAction> {
        let Some(index) = self.tabs.iter().position(|t| t.id == id) else {
            return Vec::new();
        };
        if self.active == Some(id) {
            return vec![TabAction::Show { id, visible: true }];
        }
        let mut actions = self.hide_active();
        self.active = Some(id);

        let now = now_ms();
        let tab = &mut self.tabs[index];
        tab.last_active = now;
        if tab.live {
            actions.push(TabAction::Show { id, visible: true });
        } else {
            tab.live = true;
            tab.loading = true;
            actions.push(TabAction::Create {
                id,
                url: tab.url.clone(),
            });
            if tab.scroll_y > 0.0 {
                actions.push(TabAction::Scroll {
                    id,
                    y: tab.scroll_y,
                });
            }
        }
        actions.extend(self.enforce_live_limit());
        actions
    }

    /// Close a tab. Closing the last tab opens a blank one.
    pub fn close(&mut self, id: TabId) -> Vec<TabAction> {
        let Some(index) = self.tabs.iter().position(|t| t.id == id) else {
            return Vec::new();
        };
        let was_active = self.active == Some(id);
        let tab = self.tabs.remove(index);
        let mut actions = Vec::new();
        if tab.live {
            actions.push(TabAction::Destroy { id });
        }
        if !was_active {
            return actions;
        }
        self.active = None;
        if self.tabs.is_empty() {
            let (_, created) = self.create_tab(crate::url::blank_page());
            actions.extend(created);
        } else {
            let next = self.tabs[index.min(self.tabs.len() - 1)].id;
            actions.extend(self.activate(next));
        }
        actions
    }

    /// Move a tab to a new display position.
    pub fn reorder(&mut self, id: TabId, to_index: usize) -> bool {
        let Some(from) = self.tabs.iter().position(|t| t.id == id) else {
            return false;
        };
        let to = to_index.min(self.tabs.len().saturating_sub(1));
        if from == to {
            return true;
        }
        let tab = self.tabs.remove(from);
        self.tabs.insert(to, tab);
        true
    }

    /// Discard a tab's webview while keeping its place in the session.
    pub fn discard(&mut self, id: TabId) -> Vec<TabAction> {
        if self.active == Some(id) {
            return Vec::new();
        }
        let Some(tab) = self.tabs.iter_mut().find(|t| t.id == id) else {
            return Vec::new();
        };
        if !tab.is_discardable() {
            return Vec::new();
        }
        tab.live = false;
        tab.loading = false;
        vec![TabAction::Destroy { id }]
    }

    /// Discard all inactive background tabs to save memory immediately.
    pub fn discard_others(&mut self) -> Vec<TabAction> {
        let active = self.active.unwrap_or(0);
        let victims: Vec<TabId> = self
            .tabs
            .iter()
            .filter(|t| t.id != active && t.is_discardable())
            .map(|t| t.id)
            .collect();
        let mut actions = Vec::new();
        for id in victims {
            actions.extend(self.discard(id));
        }
        actions
    }

    /// Discard least-recently-active tabs until the live limit is respected.
    pub fn enforce_live_limit(&mut self) -> Vec<TabAction> {
        let max = self.config.max_live.max(1);
        let mut actions = Vec::new();
        while self.live_count() > max {
            let Some(victim) = self
                .tabs
                .iter()
                .filter(|t| t.id != self.active.unwrap_or(0) && t.is_discardable())
                .min_by_key(|t| t.last_active)
                .map(|t| t.id)
            else {
                break;
            };
            actions.extend(self.discard(victim));
        }
        actions
    }

    /// Discard background tabs idle for longer than the configured limit.
    pub fn tick(&mut self, now: Timestamp) -> Vec<TabAction> {
        let idle_ms = self.config.idle_discard.as_millis() as Timestamp;
        let victims: Vec<TabId> = self
            .tabs
            .iter()
            .filter(|t| {
                t.id != self.active.unwrap_or(0)
                    && t.is_discardable()
                    && now.saturating_sub(t.last_active) >= idle_ms
            })
            .map(|t| t.id)
            .collect();
        let mut actions = Vec::new();
        for id in victims {
            actions.extend(self.discard(id));
        }
        actions
    }

    pub fn navigate(&mut self, id: TabId, url: &str) -> Vec<TabAction> {
        let Some(tab) = self.tabs.iter_mut().find(|t| t.id == id) else {
            return Vec::new();
        };
        tab.url = url.to_string();
        tab.loading = true;
        if !tab.live {
            tab.live = true;
            return vec![TabAction::Create {
                id,
                url: url.to_string(),
            }];
        }
        vec![TabAction::Load {
            id,
            url: url.to_string(),
        }]
    }

    pub fn set_title(&mut self, id: TabId, title: impl Into<String>) {
        if let Some(tab) = self.tabs.iter_mut().find(|t| t.id == id) {
            tab.title = title.into();
        }
    }

    pub fn set_url(&mut self, id: TabId, url: impl Into<String>) {
        if let Some(tab) = self.tabs.iter_mut().find(|t| t.id == id) {
            tab.url = url.into();
        }
    }

    pub fn set_favicon(&mut self, id: TabId, favicon: Option<String>) {
        if let Some(tab) = self.tabs.iter_mut().find(|t| t.id == id) {
            tab.favicon = favicon;
        }
    }

    pub fn set_loading(&mut self, id: TabId, loading: bool) {
        if let Some(tab) = self.tabs.iter_mut().find(|t| t.id == id) {
            tab.loading = loading;
        }
    }

    pub fn set_audible(&mut self, id: TabId, audible: bool) {
        if let Some(tab) = self.tabs.iter_mut().find(|t| t.id == id) {
            tab.audible = audible;
        }
    }

    pub fn set_form_dirty(&mut self, id: TabId, dirty: bool) {
        if let Some(tab) = self.tabs.iter_mut().find(|t| t.id == id) {
            tab.form_dirty = dirty;
        }
    }

    pub fn set_scroll(&mut self, id: TabId, y: f64) {
        if let Some(tab) = self.tabs.iter_mut().find(|t| t.id == id) {
            tab.scroll_y = y;
        }
    }

    pub fn pin_tab(&mut self, id: TabId, pinned: bool) -> bool {
        if let Some(tab) = self.tabs.iter_mut().find(|t| t.id == id) {
            tab.pinned = pinned;
            true
        } else {
            false
        }
    }

    /// Mark a tab as live after the app successfully created its webview.
    pub fn mark_live(&mut self, id: TabId) {
        if let Some(tab) = self.tabs.iter_mut().find(|t| t.id == id) {
            tab.live = true;
        }
    }

    pub fn snapshot(&self) -> Vec<TabInfo> {
        self.tabs.iter().map(|t| t.to_info(self.active)).collect()
    }

    pub fn stored_tabs(&self) -> Vec<StoredTab> {
        self.tabs
            .iter()
            .enumerate()
            .map(|(i, t)| t.to_stored(i as i64))
            .collect()
    }

    fn hide_active(&self) -> Vec<TabAction> {
        self.active
            .filter(|id| self.tab(*id).map(|t| t.live).unwrap_or(false))
            .map(|id| vec![TabAction::Show { id, visible: false }])
            .unwrap_or_default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn manager() -> TabManager {
        TabManager::new(TabConfig::default())
    }

    fn open(manager: &mut TabManager, n: usize) {
        for i in 0..n {
            manager.create_tab(&format!("https://{i}.test"));
        }
    }

    #[test]
    fn creates_and_activates_tabs() {
        let mut m = manager();
        let (id, actions) = m.create_tab("https://a.test");
        assert_eq!(id, 1);
        assert_eq!(m.active_id(), Some(1));
        assert_eq!(
            actions,
            vec![TabAction::Create {
                id: 1,
                url: "https://a.test".into()
            }]
        );
    }

    #[test]
    fn keeps_at_most_max_live_and_discards_oldest() {
        let mut m = manager();
        open(&mut m, 10);
        assert_eq!(m.live_count(), DEFAULT_MAX_LIVE);
        // The five most recent tabs stay live.
        let live: Vec<TabId> = m.tabs().iter().filter(|t| t.live).map(|t| t.id).collect();
        assert_eq!(live, vec![6, 7, 8, 9, 10]);
        assert!(m.active_id() == Some(10));
    }

    #[test]
    fn never_discards_audible_or_dirty_or_active_tabs() {
        let mut config = TabConfig::default();
        config.max_live = 5;
        let mut m = TabManager::new(config);
        open(&mut m, 5);
        m.set_audible(1, true);
        m.set_form_dirty(2, true);

        let mut tight = m.config().clone();
        tight.max_live = 1;
        m.set_config(tight);
        let actions = m.enforce_live_limit();

        // Tabs 3 and 4 are discarded; 1 (audible), 2 (dirty) and 5 (active)
        // are protected, so the limit cannot be fully met.
        assert_eq!(actions.len(), 2);
        assert_eq!(m.live_count(), 3);
        assert!(m.tab(1).unwrap().live);
        assert!(m.tab(2).unwrap().live);
        assert!(m.tab(5).unwrap().live);
        assert!(!m.tab(3).unwrap().live);
        assert!(!m.tab(4).unwrap().live);

        m.set_audible(1, false);
        m.set_form_dirty(2, false);
        m.enforce_live_limit();
        assert_eq!(m.live_count(), 1);
        assert!(m.tab(5).unwrap().live);
    }

    #[test]
    fn pin_tab_protects_from_discard() {
        let mut config = TabConfig::default();
        config.max_live = 1;
        let mut m = TabManager::new(config);
        open(&mut m, 3);
        assert!(!m.tab(1).unwrap().live);
        assert!(!m.tab(2).unwrap().live);
        assert!(m.tab(3).unwrap().live);

        // Pin tab 1 and reactivate it
        m.pin_tab(1, true);
        assert!(m.tab(1).unwrap().pinned);
        m.activate(1);
        // Both tab 1 (active) and any pinned tab will resist discard when possible
        assert!(m.tab(1).unwrap().live);

        // Now activate tab 3. Since tab 1 is pinned, tab 1 is not discardable!
        m.activate(3);
        assert!(m.tab(3).unwrap().live);
        assert!(m.tab(1).unwrap().live); // Tab 1 stays live because pinned tabs are not discardable
        assert_eq!(m.live_count(), 2);
    }

    #[test]
    fn discard_others_discards_all_inactive_tabs() {
        let mut m = manager();
        open(&mut m, 4);
        assert_eq!(m.live_count(), 4);
        assert_eq!(m.active_id(), Some(4));

        let actions = m.discard_others();
        assert_eq!(actions.len(), 3);
        assert_eq!(m.live_count(), 1);
        assert!(m.tab(4).unwrap().live);
        assert!(!m.tab(1).unwrap().live);
        assert!(!m.tab(2).unwrap().live);
        assert!(!m.tab(3).unwrap().live);
    }

    #[test]
    fn activating_a_discarded_tab_recreates_it_and_restores_scroll() {
        let mut config = TabConfig::default();
        config.max_live = 1;
        let mut m = TabManager::new(config);
        open(&mut m, 2);
        assert!(!m.tab(1).unwrap().live);
        m.set_scroll(1, 420.0);

        let actions = m.activate(1);
        assert_eq!(
            actions,
            vec![
                TabAction::Show {
                    id: 2,
                    visible: false
                },
                TabAction::Create {
                    id: 1,
                    url: "https://0.test".into()
                },
                TabAction::Scroll { id: 1, y: 420.0 },
                TabAction::Destroy { id: 2 },
            ]
        );
        assert!(m.tab(1).unwrap().live);
        assert!(!m.tab(2).unwrap().live);
    }

    #[test]
    fn idle_tick_discards_background_tabs_only() {
        let mut m = manager();
        open(&mut m, 3);
        let actions = m.tick(now_ms() + 11 * 60 * 1000);
        assert_eq!(actions.len(), 2);
        assert!(m.tab(3).unwrap().live);
        assert!(!m.tab(1).unwrap().live);
        assert!(!m.tab(2).unwrap().live);
    }

    #[test]
    fn closing_active_tab_activates_neighbour() {
        let mut m = manager();
        open(&mut m, 3);
        let actions = m.close(3);
        assert_eq!(m.active_id(), Some(2));
        assert!(actions.contains(&TabAction::Destroy { id: 3 }));
    }

    #[test]
    fn closing_last_tab_opens_blank_tab() {
        let mut m = manager();
        open(&mut m, 1);
        let actions = m.close(1);
        assert_eq!(m.tabs().len(), 1);
        assert_eq!(m.active_id(), Some(2));
        assert!(matches!(actions.last(), Some(TabAction::Create { id: 2, .. })));
    }

    #[test]
    fn reorder_moves_tab() {
        let mut m = manager();
        open(&mut m, 3);
        assert!(m.reorder(1, 2));
        let order: Vec<TabId> = m.tabs().iter().map(|t| t.id).collect();
        assert_eq!(order, vec![2, 3, 1]);
        assert!(!m.reorder(99, 0));
    }

    #[test]
    fn restore_keeps_only_active_live() {
        let mut m = manager();
        open(&mut m, 3);
        m.set_title(2, "Two");
        m.set_scroll(2, 12.0);
        let stored = m.stored_tabs();

        let (restored, actions) =
            TabManager::restore(TabConfig::default(), stored, Some(2));
        assert_eq!(restored.tabs().len(), 3);
        assert_eq!(restored.live_count(), 1);
        assert!(restored.tab(2).unwrap().live);
        assert_eq!(restored.tab(2).unwrap().title, "Two");
        assert_eq!(
            actions,
            vec![
                TabAction::Create {
                    id: 2,
                    url: "https://1.test".into()
                },
                TabAction::Scroll { id: 2, y: 12.0 }
            ]
        );
    }

    #[test]
    fn restore_without_tabs_creates_blank() {
        let (m, actions) = TabManager::restore(TabConfig::default(), Vec::new(), None);
        assert_eq!(m.tabs().len(), 1);
        assert_eq!(actions.len(), 1);
    }

    #[test]
    fn snapshot_marks_active_and_discarded() {
        let mut config = TabConfig::default();
        config.max_live = 1;
        let mut m = TabManager::new(config);
        open(&mut m, 2);
        let snap = m.snapshot();
        assert!(snap[1].active);
        assert!(snap[0].discarded);
        assert_eq!(snap[0].title, "https://0.test");
    }

    #[test]
    fn navigate_discarded_tab_creates_webview() {
        let mut config = TabConfig::default();
        config.max_live = 1;
        let mut m = TabManager::new(config);
        open(&mut m, 2);
        let actions = m.navigate(1, "https://new.test");
        assert_eq!(
            actions,
            vec![TabAction::Create {
                id: 1,
                url: "https://new.test".into()
            }]
        );
        assert_eq!(m.tab(1).unwrap().url, "https://new.test");
    }
}
