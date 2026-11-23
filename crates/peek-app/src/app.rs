use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use peek_core::tabs::{TabAction, TabConfig, TabManager};
use peek_core::url::{resolve_input, NavigateTarget, SearchEngine};
use peek_core::now_ms;
use peek_adblock::{AdblockConfig, AdblockEngine, BlockCounters};
use peek_ext::{ExtensionRegistry, UserScriptRegistry};
use peek_engine::{
    Bounds, Engine, EngineEvents, LoadEvent, WebViewId, WebViewOptions, WryEngine,
};
use peek_ipc::{
    BookmarkItem, Command, EdgeSide, Envelope, Event, ExtensionSummary, HistoryItem, PageEvent,
    Shortcut, Suggestion, SuggestionKind, TabGroup, TabId, UserScriptSummary,
};
use peek_preview::{PreviewAction, PreviewConfig, PreviewController};
use peek_store::{Session, Store};
use tao::event::Event as TaoEvent;
use tao::event::WindowEvent;
use tao::event_loop::{ControlFlow, EventLoopProxy};
use tao::window::Window;
use url::Url;

use crate::assets::{PageAssets, UiAssets};
use crate::inject;

pub const BAR_HEIGHT: f64 = 42.0;
pub const BAR_HEIGHT_FULLSCREEN: f64 = 50.0;

pub fn default_user_agent() -> &'static str {
    #[cfg(target_os = "windows")]
    {
        "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/131.0.0.0 Safari/537.36 Edg/131.0.0.0"
    }
    #[cfg(target_os = "linux")]
    {
        "Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/605.1.15 (KHTML, like Gecko) Version/18.0 Safari/605.1.15"
    }
    #[cfg(not(any(target_os = "windows", target_os = "linux")))]
    {
        "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/605.1.15 (KHTML, like Gecko) Version/18.3 Safari/605.1.15"
    }
}

const PREVIEW_HIDE_GRACE: Duration = Duration::from_millis(300);
const TICK_INTERVAL: Duration = Duration::from_millis(250);
const IDLE_TICK_EVERY: u64 = 240;
const OMNIBAR_PANEL_HEIGHT: f64 = 340.0;

#[derive(Debug, Clone)]
pub struct Settings {
    pub max_live: usize,
    pub idle_discard_minutes: u64,
    pub search_engine: SearchEngine,
    pub hover_delay_ms: u64,
    pub preview: PreviewConfig,
    pub drawer_side: EdgeSide,
    pub edge_zone: f64,
    pub start_blank: bool,
}

impl Settings {
    fn load(store: &Store) -> Self {
        let defaults = PreviewConfig::default();
        let search_engine = store
            .get_setting_or("searchEngine", SearchEngine::default())
            .unwrap_or_default();
        Self {
            max_live: store.get_setting_or("maxLive", 5usize).unwrap_or(5).max(1),
            idle_discard_minutes: store
                .get_setting_or("idleDiscardMinutes", 10u64)
                .unwrap_or(10),
            search_engine,
            hover_delay_ms: store
                .get_setting_or("hoverDelayMs", defaults.hover_delay.as_millis() as u64)
                .unwrap_or(450),
            preview: PreviewConfig {
                hover_delay: Duration::from_millis(
                    store
                        .get_setting_or("hoverDelayMs", defaults.hover_delay.as_millis() as u64)
                        .unwrap_or(450),
                ),
                width: store
                    .get_setting_or("previewWidth", defaults.width)
                    .unwrap_or(defaults.width),
                height: store
                    .get_setting_or("previewHeight", defaults.height)
                    .unwrap_or(defaults.height),
                zoom: store
                    .get_setting_or("previewZoom", defaults.zoom)
                    .unwrap_or(defaults.zoom),
                gap: defaults.gap,
                modifier_only: store
                    .get_setting_or("previewModifierOnly", true)
                    .unwrap_or(true),
            },
            drawer_side: store
                .get_setting_or("drawerSide", EdgeSide::Right)
                .unwrap_or(EdgeSide::Right),
            edge_zone: store.get_setting_or("edgeZone", 10.0).unwrap_or(10.0),
            start_blank: store.get_setting_or("startBlank", false).unwrap_or(false),
        }
    }

    fn tab_config(&self) -> TabConfig {
        TabConfig {
            max_live: self.max_live,
            idle_discard: Duration::from_secs(self.idle_discard_minutes * 60),
            start_blank: self.start_blank,
        }
    }
}

pub enum UserEvent {
    Title { view: WebViewId, title: String },
    Url { view: WebViewId, url: String },
    Load { view: WebViewId, event: LoadEvent, url: String },
    Ipc { view: WebViewId, message: String },
    NewWindow { url: String },
    Tick,
}

struct ProxyEvents {
    proxy: EventLoopProxy<UserEvent>,
}

impl ProxyEvents {
    fn send(&self, event: UserEvent) {
        let _ = self.proxy.send_event(event);
    }
}

impl EngineEvents for ProxyEvents {
    fn title_changed(&self, view: WebViewId, title: String) {
        self.send(UserEvent::Title { view, title });
    }

    fn url_changed(&self, view: WebViewId, url: String) {
        self.send(UserEvent::Url { view, url });
    }

    fn page_load(&self, view: WebViewId, event: LoadEvent, url: String) {
        self.send(UserEvent::Load { view, event, url });
    }

    fn ipc(&self, view: WebViewId, message: String) {
        self.send(UserEvent::Ipc { view, message });
    }

    fn new_window_requested(&self, _view: WebViewId, url: String) {
        self.send(UserEvent::NewWindow { url });
    }
}

fn is_subframe_or_widget_url(url: &str) -> bool {
    let lower = url.to_ascii_lowercase();
    lower.contains("ogs.google.com")
        || lower.contains("/widget/account")
        || lower.contains("accounts.google.com/widget")
        || lower.contains("accounts.google.com/gsi/")
}

pub struct App {
    window: Window,
    engine: WryEngine,
    tabs: TabManager,
    store: Store,
    settings: Settings,
    proxy: EventLoopProxy<UserEvent>,
    ui_nonce: String,
    page_nonce: String,
    ui_id: Option<WebViewId>,
    drawer_id: Option<WebViewId>,
    preview_id: Option<WebViewId>,
    preview: PreviewController,
    preview_hide_at: Option<Instant>,
    tab_views: HashMap<TabId, WebViewId>,
    view_tabs: HashMap<WebViewId, TabId>,
    drawer_open: bool,
    omnibar_open: bool,
    window_size: (f64, f64),
    ticks: u64,
    running: Arc<AtomicBool>,
    adblock: Arc<AdblockEngine>,
    adblock_config: AdblockConfig,
    tab_counters: HashMap<TabId, BlockCounters>,
    #[allow(dead_code)]
    profile_dir: std::path::PathBuf,
    profile_web_data_dir: std::path::PathBuf,
    profile_uuid: [u8; 16],
    current_drawer_side: EdgeSide,
    current_profile_name: String,
    extensions: ExtensionRegistry,
    userscripts: UserScriptRegistry,
    is_zen_mode: bool,
}

impl App {
    pub fn new(
        window: Window,
        proxy: EventLoopProxy<UserEvent>,
        store: Store,
        profile_dir: std::path::PathBuf,
        current_profile_name: String,
    ) -> anyhow::Result<Self> {
        let settings = Settings::load(&store);
        let engine = WryEngine::new(Arc::new(ProxyEvents {
            proxy: proxy.clone(),
        }));
        let size = window.inner_size();
        let scale = window.scale_factor();
        let window_size = (
            (size.width as f64 / scale).max(1.0),
            (size.height as f64 / scale).max(1.0),
        );
        let tabs = TabManager::new(settings.tab_config());
        let adblock_config = AdblockConfig::default();
        let default_rules = "||doubleclick.net^\n||googlesyndication.com^\n||adservice.google.com^\n||adnxs.com^\n||taboola.com^\n||outbrain.com^\n||scorecardresearch.com^\n||facebook.com/tr/^\n||amazon-adsystem.com^\n##.ad-banner\n##.adsbygoogle\n##[id^=\"google_ads_\"]\n##div.ad-container";
        let adblock = Arc::new(AdblockEngine::new(default_rules));
        let tab_counters = HashMap::new();

        let profile_uuid = match store.get_setting("profileUuid") {
            Ok(Some(hex_str)) if hex_str.len() == 32 => {
                let mut bytes = [0u8; 16];
                let mut valid = true;
                for i in 0..16 {
                    if let Ok(b) = u8::from_str_radix(&hex_str[i * 2..i * 2 + 2], 16) {
                        bytes[i] = b;
                    } else {
                        valid = false;
                        break;
                    }
                }
                if valid {
                    bytes
                } else {
                    derive_profile_uuid(&current_profile_name)
                }
            }
            _ => {
                let uuid = derive_profile_uuid(&current_profile_name);
                let hex_str = uuid.iter().map(|b| format!("{b:02x}")).collect::<String>();
                let _ = store.set_setting("profileUuid", &hex_str);
                uuid
            }
        };
        let profile_web_data_dir = profile_dir.join("web_data");
        let _ = std::fs::create_dir_all(&profile_web_data_dir);
        let extensions_dir = profile_dir.join("extensions");
        let _ = std::fs::create_dir_all(&extensions_dir);
        let extensions = ExtensionRegistry::load_from_dir(&extensions_dir);
        let userscripts_dir = profile_dir.join("userscripts");
        let _ = std::fs::create_dir_all(&userscripts_dir);
        let userscripts = UserScriptRegistry::load_from_dir(&userscripts_dir);

        let current_drawer_side = settings.drawer_side;
        Ok(Self {
            window,
            engine,
            tabs,
            store,
            settings,
            proxy,
            ui_nonce: random_nonce(),
            page_nonce: random_nonce(),
            ui_id: None,
            drawer_id: None,
            preview_id: None,
            preview: PreviewController::default(),
            preview_hide_at: None,
            tab_views: HashMap::new(),
            view_tabs: HashMap::new(),
            drawer_open: false,
            omnibar_open: false,
            window_size,
            ticks: 0,
            running: Arc::new(AtomicBool::new(true)),
            adblock,
            adblock_config,
            tab_counters,
            profile_dir,
            profile_web_data_dir,
            profile_uuid,
            current_drawer_side,
            current_profile_name,
            extensions,
            userscripts,
            is_zen_mode: false,
        })
    }

    pub fn start(&mut self) -> anyhow::Result<()> {
        self.create_chrome()?;
        let _ = self.create_drawer_prewarm();

        self.restore_tabs_from_store()?;

        self.layout();
        self.window.set_visible(true);
        #[cfg(target_os = "macos")]
        crate::set_app_icon();
        let _ = self.store.seed_default_bookmarks(now_ms());
        self.spawn_ticker();
        self.push_tabs();
        self.send_pinned_sites();
        self.send_profiles_list();
        self.send_tab_groups();
        self.send_userscripts_list();
        Ok(())
    }

    fn restore_tabs_from_store(&mut self) -> anyhow::Result<()> {
        let (stored, active) = if self.settings.start_blank {
            (Vec::new(), None)
        } else {
            let stored = self.store.load_tabs().unwrap_or_default();
            let mut stored: Vec<peek_store::StoredTab> = stored
                .into_iter()
                .filter(|t| !t.url.contains("ogs.google.com") && !t.url.contains("/widget/account"))
                .collect();

            // Self-heal any tabs that had their url saved as about:blank but have a real title
            for tab in &mut stored {
                if (peek_core::url::is_blank(&tab.url) || tab.url.is_empty()) && !tab.title.is_empty() {
                    if let Ok(entries) = self.store.search_history(&tab.title, 1) {
                        if let Some(entry) = entries.first() {
                            tab.url = entry.url.clone();
                        }
                    }
                }
            }

            let session = self.store.load_session().unwrap_or(None);
            let active = session.as_ref().and_then(|s| s.active_tab);
            (stored, active)
        };

        let (manager, actions) = TabManager::restore(
            self.settings.tab_config(),
            stored,
            active,
        );
        self.tabs = manager;
        self.apply(actions);
        Ok(())
    }

    pub fn handle(&mut self, event: TaoEvent<'_, UserEvent>, control_flow: &mut ControlFlow) {
        match event {
            TaoEvent::WindowEvent { event, .. } => match event {
                WindowEvent::CloseRequested => {
                    self.shutdown();
                    *control_flow = ControlFlow::Exit;
                }
                WindowEvent::Resized(_) | WindowEvent::ScaleFactorChanged { .. } => {
                    self.layout();
                }
                WindowEvent::Focused(true) => {
                    if let Some(view) = self.active_view() {
                        let _ = self.engine.focus(view);
                    }
                }
                _ => {}
            },
            TaoEvent::UserEvent(event) => self.handle_user_event(event),
            TaoEvent::LoopDestroyed => {
                self.shutdown();
            }
            _ => {}
        }
    }

    fn handle_user_event(&mut self, event: UserEvent) {
        match event {
            UserEvent::Title { view, title } => {
                if let Some(tab) = self.view_tabs.get(&view).copied() {
                    self.tabs.set_title(tab, &title);
                    self.send(Event::TitleChanged {
                        id: tab,
                        title: title.clone(),
                    });
                    self.push_tabs();
                }
            }
            UserEvent::Url { view, url } => {
                if let Some(tab) = self.view_tabs.get(&view).copied() {
                    let clean_url = if let Some(top) = self.engine.url(view) {
                        if !top.is_empty() && !peek_core::url::is_blank(&top) {
                            top
                        } else {
                            url
                        }
                    } else {
                        url
                    };
                    if is_subframe_or_widget_url(&clean_url) {
                        return;
                    }
                    // Prevent intermediate WebKit about:blank events from wiping out real tab URLs
                    if peek_core::url::is_blank(&clean_url) {
                        if let Some(existing) = self.tabs.tab(tab) {
                            if !peek_core::url::is_blank(&existing.url) {
                                return;
                            }
                        }
                    }
                    self.tabs.set_url(tab, &clean_url);
                    self.send(Event::UrlChanged {
                        id: tab,
                        url: clean_url.clone(),
                    });
                    self.send_loading(tab);
                    self.push_tabs();
                    if Some(tab) == self.tabs.active_id() && !peek_core::url::is_blank(&clean_url) {
                        if let Some(id) = self.ui_id {
                            let _ = self.engine.set_bounds(id, self.chrome_bounds());
                        }
                        let _ = self.engine.set_visible(view, true);
                        let _ = self.engine.focus(view);
                    }
                }
            }
            UserEvent::Load { view, event, url } => self.on_load(view, event, url),
            UserEvent::Ipc { view, message } => self.on_ipc(view, &message),
            UserEvent::NewWindow { url } => {
                let trimmed = url.trim();
                if trimmed.contains("ogs.google.com")
                    || trimmed.contains("/widget/account")
                    || trimmed.contains("accounts.google.com/widget")
                {
                    return;
                }
                let target_url = if trimmed.is_empty() {
                    peek_core::url::blank_page().to_string()
                } else {
                    trimmed.to_string()
                };
                self.new_tab(&target_url);
            }
            UserEvent::Tick => self.on_tick(),
        }
    }

    fn on_load(&mut self, view: WebViewId, event: LoadEvent, url: String) {
        let Some(tab) = self.view_tabs.get(&view).copied() else {
            debug_log(&format!("load {view} (chrome) {event:?} {url}"));
            return;
        };
        debug_log(&format!("load tab {tab} {event:?} {url}"));
        match event {
            LoadEvent::Started => {
                self.tabs.set_loading(tab, true);
                let cand_url = if let Some(top) = self.engine.url(view) {
                    if !top.is_empty() && !peek_core::url::is_blank(&top) {
                        top
                    } else {
                        url
                    }
                } else {
                    url
                };
                if !cand_url.is_empty() && !peek_core::url::is_blank(&cand_url) && !is_subframe_or_widget_url(&cand_url) {
                    self.tabs.set_url(tab, &cand_url);
                    self.send(Event::UrlChanged {
                        id: tab,
                        url: cand_url.clone(),
                    });
                    if Some(tab) == self.tabs.active_id() {
                        if let Some(id) = self.ui_id {
                            let bounds = self.chrome_bounds();
                            let _ = self.engine.set_bounds(id, bounds);
                        }
                        let _ = self.engine.set_visible(view, true);
                        let _ = self.engine.focus(view);
                    }
                }
                self.send_loading(tab);
                self.push_tabs();
                self.send_cookies_count();
            }
            LoadEvent::Finished => {
                self.tabs.set_loading(tab, false);
                let top_url = self.engine.url(view).unwrap_or_default();
                let final_url = if !top_url.is_empty() && !peek_core::url::is_blank(&top_url) && !is_subframe_or_widget_url(&top_url) {
                    top_url
                } else if !url.is_empty() && !peek_core::url::is_blank(&url) && !is_subframe_or_widget_url(&url) {
                    url
                } else {
                    self.tabs
                        .tab(tab)
                        .map(|t| t.url.clone())
                        .unwrap_or_else(|| peek_core::url::blank_page().to_string())
                };

                if !final_url.is_empty() && !peek_core::url::is_blank(&final_url) && !is_subframe_or_widget_url(&final_url) {
                    self.tabs.set_url(tab, &final_url);
                    self.send(Event::UrlChanged {
                        id: tab,
                        url: final_url.clone(),
                    });
                }

                let title = self
                    .tabs
                    .tab(tab)
                    .map(|t| t.title.clone())
                    .unwrap_or_default();
                if final_url.starts_with("http://") || final_url.starts_with("https://") {
                    let _ = self.store.record_visit(&final_url, &title, now_ms());
                    self.apply_cosmetic_filters(view, &final_url);
                    for script in self
                        .extensions
                        .matching_content_scripts(&final_url, &self.page_nonce)
                    {
                        let _ = self.engine.evaluate(view, &script);
                    }
                    for script in self.userscripts.matching_scripts(&final_url, "document-end") {
                        let _ = self.engine.evaluate(view, &script.generate_injected_js());
                    }
                    for script in self.userscripts.matching_scripts(&final_url, "document-idle") {
                        let _ = self.engine.evaluate(view, &script.generate_injected_js());
                    }
                }

                if Some(tab) == self.tabs.active_id() {
                    let is_blank = peek_core::url::is_blank(&final_url);
                    if let Some(id) = self.ui_id {
                        let bounds = self.chrome_bounds();
                        let _ = self.engine.set_bounds(id, bounds);
                    }
                    let _ = self.engine.set_visible(view, !is_blank);
                    if !is_blank {
                        let _ = self.engine.focus(view);
                    }
                }

                // Restore scroll offset if tab was restored from hibernation
                if let Some(t) = self.tabs.tab(tab) {
                    if let Some(ref fav) = t.favicon {
                        if let Ok(true) = self.store.is_bookmarked(&final_url) {
                            let _ = self.store.update_bookmark_favicon(&final_url, fav);
                            self.send_pinned_sites();
                        }
                    }
                    if t.scroll_y > 0.0 {
                        let y = t.scroll_y;
                        let _ = self.engine.evaluate(
                            view,
                            &format!(
                                "(function(){{ var pos = {y}; setTimeout(function(){{ window.scrollTo(0, pos); }}, 50); setTimeout(function(){{ window.scrollTo(0, pos); }}, 200); }})();"
                            ),
                        );
                    }
                }

                self.send_loading(tab);
                self.push_tabs();
                self.send_cookies_count();
            }
        }
    }

    fn on_ipc(&mut self, view: WebViewId, raw: &str) {
        if Some(view) == self.ui_id || Some(view) == self.drawer_id {
            if let Ok(command) = Envelope::<Command>::decode(raw, &self.ui_nonce) {
                self.handle_command(command);
            }
            return;
        }

        let tab = self.view_tabs.get(&view).copied();
        if tab.is_some() || Some(view) == self.preview_id {
            if let Ok(event) = Envelope::<PageEvent>::decode(raw, &self.page_nonce) {
                self.handle_page_event(view, tab, event);
            }
        }
    }

    fn handle_command(&mut self, command: Command) {
        match command {
            Command::Navigate { input } => self.navigate_from_input(&input),
            Command::NewTab { url } => {
                let url = url
                    .filter(|u| !u.trim().is_empty())
                    .unwrap_or_else(|| peek_core::url::blank_page().to_string());
                self.new_tab(&url);
            }
            Command::CloseTab { id } => {
                let actions = self.tabs.close(id);
                self.apply(actions);
                self.push_tabs();
            }
            Command::SwitchTab { id } => {
                let actions = self.tabs.activate(id);
                self.apply(actions);
                self.push_tabs();
            }
            Command::ReorderTab { id, to_index } => {
                if self.tabs.reorder(id, to_index) {
                    self.push_tabs();
                }
            }
            Command::DiscardTab { id } => {
                let actions = self.tabs.discard(id);
                self.apply(actions);
                self.push_tabs();
            }
            Command::DiscardOtherTabs => {
                let actions = self.tabs.discard_others();
                let count = actions.len();
                self.apply(actions);
                self.push_tabs();
                self.send(Event::Toast {
                    message: format!("Hibernated {count} inactive tabs to free memory"),
                });
            }
            Command::CycleTab { forward } => self.cycle_tab(forward),
            Command::Back => {
                if let Some(view) = self.active_view() {
                    let _ = self.engine.go_back(view);
                }
            }
            Command::Forward => {
                if let Some(view) = self.active_view() {
                    let _ = self.engine.go_forward(view);
                }
            }
            Command::Reload => {
                if let Some(view) = self.active_view() {
                    let _ = self.engine.reload(view);
                }
            }
            Command::Stop => {
                if let Some(view) = self.active_view() {
                    let _ = self.engine.evaluate(view, "window.stop()");
                }
            }
            Command::Suggest { query } => {
                let items = self.suggestions(&query);
                self.send(Event::SuggestionsReady { query, items });
            }
            Command::DismissSuggestions => {}
            Command::RequestState => self.send_state(),
            Command::SetOmnibarOpen { open } => {
                if self.omnibar_open != open {
                    self.omnibar_open = open;
                    if let Some(id) = self.ui_id {
                        let bounds = self.chrome_bounds();
                        let _ = self.engine.set_bounds(id, bounds);
                    }
                    self.ensure_layer_order();
                    if !open {
                        if let Some(view) = self.active_view() {
                            let _ = self.engine.focus(view);
                        }
                    }
                }
            }
            Command::FocusPage => {
                self.omnibar_open = false;
                if let Some(id) = self.ui_id {
                    let bounds = self.chrome_bounds();
                    let _ = self.engine.set_bounds(id, bounds);
                }
                if let Some(view) = self.active_view() {
                    let _ = self.engine.focus(view);
                }
            }
            Command::SetDrawer { open, side, section } => {
                if open {
                    let s = side.unwrap_or(self.settings.drawer_side);
                    self.open_drawer_on_side(s, section.as_deref());
                } else {
                    self.close_drawer();
                }
            }
            Command::OpenUrl { url } => {
                self.new_tab(&url);
            }
            Command::SetSiteAdblock { host, enabled } => {
                let _ = self.store.set_site_adblock(&host, enabled);
                self.send_adblock_state();
            }
            Command::SetSetting { key, value } => self.apply_setting(&key, value),
            Command::ClearCookies => {
                let _ = self.store.clear_cookies();
                self.send(Event::CookiesCount { count: 0 });
                self.send(Event::Toast {
                    message: "Cookies and storage cleared".into(),
                });
            }
            Command::GetCookies => {
                self.send_cookies_count();
            }
            Command::CreateProfile { name } => self.create_profile(&name),
            Command::SwitchProfile { name } => self.switch_profile(&name),
            Command::DeleteProfile { name } => self.delete_profile(&name),
            Command::InstallExtension { path } => self.install_extension_path(&path),
            Command::ToggleExtension { id, enabled } => self.toggle_extension(&id, enabled),
            Command::UninstallExtension { id } => self.uninstall_extension(&id),
            Command::GetExtensions => self.send_extensions_list(),
            Command::PinSite { url, title, favicon } => {
                let clean_url = url.trim();
                if !clean_url.is_empty() && !peek_core::url::is_blank(clean_url) {
                    let t = title.unwrap_or_default();
                    let _ = self.store.add_bookmark(clean_url, &t, favicon.as_deref(), now_ms());
                    self.send_pinned_sites();
                    self.send(Event::Toast {
                        message: format!("Pinned {}", if t.is_empty() { clean_url } else { &t }),
                    });
                }
            }
            Command::UnpinSite { url } => {
                let clean_url = url.trim();
                if let Ok(removed) = self.store.remove_bookmark_by_url(clean_url) {
                    if removed > 0 {
                        self.send_pinned_sites();
                        self.send(Event::Toast {
                            message: "Removed pin".into(),
                        });
                    }
                }
            }
            Command::PinTab { id, pinned } => {
                if self.tabs.pin_tab(id, pinned) {
                    self.push_tabs();
                    self.send(Event::Toast {
                        message: if pinned { "Tab pinned".into() } else { "Tab unpinned".into() },
                    });
                }
            }
            Command::GetPinnedSites => {
                self.send_pinned_sites();
            }
            Command::PreviewLink { url, x, y, width, height } => {
                let (win_w, win_h) = self.window_size;
                let config = self.settings.preview.clone();
                let target_w = (config.width as f64).min((win_w - 40.0).max(200.0));
                let target_h = (config.height as f64).min((win_h - 40.0).max(150.0));
                let placed_x = (x + width / 2.0 - target_w / 2.0).clamp(20.0, (win_w - target_w - 20.0).max(20.0));
                let placed_y = if y + height + target_h + 20.0 <= win_h {
                    y + height + 10.0
                } else {
                    (y - target_h - 10.0).max(20.0)
                };
                let placed = Bounds::new(placed_x, placed_y, target_w, target_h);
                self.show_preview(&url, placed, config.zoom);
            }
            Command::HidePreview => {
                self.hide_preview();
            }
            Command::CopyCleanUrl { url } => {
                let clean = peek_core::url::strip_tracking_params(&url);
                if let Some(ui_id) = self.ui_id {
                    let escaped = serde_json::to_string(&clean).unwrap_or_default();
                    let js = format!("if (navigator.clipboard) navigator.clipboard.writeText({escaped});");
                    let _ = self.engine.evaluate(ui_id, &js);
                }
                self.send(Event::Toast {
                    message: "Clean URL copied".into(),
                });
            }
            Command::ToggleMuteTab { id } => {
                let target = id.or_else(|| self.tabs.active_id());
                if let Some(tab_id) = target {
                    if let Some(view) = self.tab_views.get(&tab_id).copied() {
                        let _ = self.engine.evaluate(view, "if (window.__peekToggleMute) window.__peekToggleMute();");
                        self.send(Event::Toast {
                            message: "Toggled audio mute".into(),
                        });
                    }
                }
            }
            Command::ToggleReaderMode => {
                if let Some(view) = self.active_view() {
                    let _ = self.engine.evaluate(view, "if (window.__peekToggleReader) window.__peekToggleReader();");
                }
            }
            Command::FindInPage { query, forward } => {
                if let Some(view) = self.active_view() {
                    let escaped = serde_json::to_string(&query).unwrap_or_default();
                    let js = format!("if (window.__peekFind) window.__peekFind({escaped}, {forward});");
                    let _ = self.engine.evaluate(view, &js);
                }
            }
            Command::CloseFind => {
                if let Some(view) = self.active_view() {
                    let _ = self.engine.evaluate(view, "if (window.__peekCloseFind) window.__peekCloseFind();");
                }
            }
            Command::GetHistory { query } => {
                if let Some(q) = query.filter(|s| !s.trim().is_empty()) {
                    let list = self.store.search_history(&q, 100).unwrap_or_default();
                    let entries = list.into_iter().map(|h| HistoryItem {
                        url: h.url,
                        title: h.title,
                        visit_count: h.visit_count,
                        last_visit: h.last_visit,
                    }).collect();
                    self.send(Event::HistoryList { entries });
                } else {
                    self.send_history_list();
                }
            }
            Command::DeleteHistoryItem { url } => {
                let _ = self.store.delete_history_url(&url);
                self.send_history_list();
                self.send(Event::Toast { message: "History item deleted".into() });
            }
            Command::ClearHistory => {
                let _ = self.store.clear_history();
                self.send(Event::HistoryList { entries: Vec::new() });
                self.send(Event::Toast { message: "Browsing history cleared".into() });
            }
            Command::GetBookmarks { query } => {
                if let Some(q) = query.filter(|s| !s.trim().is_empty()) {
                    let list = self.store.search_bookmarks(&q, 100).unwrap_or_default();
                    let bookmarks = list.into_iter().map(|b| BookmarkItem {
                        id: b.id,
                        url: b.url,
                        title: b.title,
                        favicon: b.favicon,
                        created_at: b.created_at,
                    }).collect();
                    self.send(Event::BookmarksList { bookmarks });
                } else {
                    self.send_bookmarks_list();
                }
            }
            Command::ToggleBookmark { url, title } => {
                let clean = url.trim();
                if !clean.is_empty() && !peek_core::url::is_blank(clean) {
                    if let Ok(is_bm) = self.store.is_bookmarked(clean) {
                        if is_bm {
                            let _ = self.store.remove_bookmark_by_url(clean);
                            self.send(Event::Toast { message: "Removed bookmark".into() });
                        } else {
                            let _ = self.store.add_bookmark(clean, &title, None, now_ms());
                            self.send(Event::Toast { message: "Saved to bookmarks".into() });
                        }
                        self.send_pinned_sites();
                        let list = self.store.bookmarks().unwrap_or_default();
                        let bookmarks = list.into_iter().map(|b| BookmarkItem {
                            id: b.id,
                            url: b.url,
                            title: b.title,
                            favicon: b.favicon,
                            created_at: b.created_at,
                        }).collect();
                        self.send(Event::BookmarksList { bookmarks });
                    }
                }
            }
            Command::ToggleZenMode => {
                self.is_zen_mode = !self.is_zen_mode;
                self.layout();
                self.send(Event::ZenModeState { active: self.is_zen_mode });
                self.send(Event::Toast {
                    message: if self.is_zen_mode {
                        "Zen Mode enabled (⌘.)".into()
                    } else {
                        "Zen Mode disabled".into()
                    },
                });
            }
            Command::ToggleInvertDark => {
                if let Some(view) = self.active_view() {
                    let _ = self.engine.evaluate(
                        view,
                        "if (window.__peekToggleInvertDark) window.__peekToggleInvertDark();",
                    );
                }
            }
            Command::TogglePiP => {
                if let Some(view) = self.active_view() {
                    let _ = self.engine.evaluate(view, "if (window.__peekTogglePiP) window.__peekTogglePiP();");
                }
            }
            Command::ZoomIn => {
                if let Some(view) = self.active_view() {
                    let _ = self.engine.evaluate(view, "if (window.__peekZoom) window.__peekZoom(0.1, false);");
                }
            }
            Command::ZoomOut => {
                if let Some(view) = self.active_view() {
                    let _ = self.engine.evaluate(view, "if (window.__peekZoom) window.__peekZoom(-0.1, false);");
                }
            }
            Command::ZoomReset => {
                if let Some(view) = self.active_view() {
                    let _ = self.engine.evaluate(view, "if (window.__peekZoom) window.__peekZoom(0.0, true);");
                }
            }
            Command::CopyMarkdownLink => {
                if let Some(tab) = self.tabs.active() {
                    let title = if tab.title.trim().is_empty() { &tab.url } else { &tab.title };
                    let clean = peek_core::url::strip_tracking_params(&tab.url);
                    let md = format!("[{title}]({clean})");
                    if let Some(ui_id) = self.ui_id {
                        let escaped = serde_json::to_string(&md).unwrap_or_default();
                        let js = format!("if (navigator.clipboard) navigator.clipboard.writeText({escaped});");
                        let _ = self.engine.evaluate(ui_id, &js);
                    }
                    self.send(Event::Toast {
                        message: "Copied Markdown link (⌘⌥C)".into(),
                    });
                }
            }
            Command::SaveNote { content } => {
                let _ = self.store.set_setting("scratchpad_notes", &content);
            }
            Command::GetNote => {
                self.send_note();
            }
            Command::SaveTabGroups { groups } => {
                let _ = self.store.set_setting_json("tab_groups", &groups);
                self.send(Event::TabGroupsChanged { groups });
            }
            Command::SetTabGroup { tab_id, group_id } => {
                self.tabs.set_group(tab_id, group_id);
                self.push_tabs();
            }
            Command::GetUserScripts => {
                self.send_userscripts_list();
            }
            Command::ToggleUserScript { id, enabled } => {
                self.userscripts.toggle_script(&id, enabled);
                self.send_userscripts_list();
            }
            Command::SaveUserScript { name, code } => {
                if let Ok(_saved) = self.userscripts.save_script(&name, &code) {
                    self.send_userscripts_list();
                    self.send(Event::Toast {
                        message: format!("Saved UserScript '{name}'"),
                    });
                }
            }
            Command::DeleteUserScript { id } => {
                if self.userscripts.delete_script(&id) {
                    self.send_userscripts_list();
                    self.send(Event::Toast {
                        message: "UserScript deleted".into(),
                    });
                }
            }
            Command::Quit => {
                self.shutdown();
                std::process::exit(0);
            }
        }
    }

    fn handle_page_event(&mut self, view: WebViewId, tab: Option<TabId>, event: PageEvent) {
        match event {
            PageEvent::Hover { url, rect, forced } => {
                let (w, h) = self.window_size;
                let config = self.settings.preview.clone();
                let elapsed = config.hover_delay;
                let bar_h = self.bar_height();
                let page_height = (h - bar_h).max(1.0);
                match self
                    .preview
                    .hover(url, rect, elapsed, forced, &config, (w, page_height))
                {
                    PreviewAction::Show { url, bounds, zoom } => {
                        self.preview_hide_at = None;
                        let placed = Bounds::new(
                            bounds.x,
                            bounds.y + bar_h,
                            bounds.width,
                            bounds.height,
                        );
                        self.show_preview(&url, placed, zoom);
                    }
                    PreviewAction::Hide => self.hide_preview(),
                    PreviewAction::None => {}
                }
            }
            PageEvent::HoverEnd => self.schedule_preview_hide(),
            PageEvent::PreviewEnter => self.preview_hide_at = None,
            PageEvent::PreviewLeave => self.schedule_preview_hide(),
            PageEvent::PreviewClick { url } => {
                self.hide_preview();
                self.new_tab(&url);
            }
            PageEvent::EdgeProximity { side, .. } => {
                let section = match side {
                    EdgeSide::Left => "tabs",
                    EdgeSide::Right => "settings",
                };
                self.open_drawer_on_side(side, Some(section));
            }
            PageEvent::AudioState { playing } => {
                if let Some(tab) = tab {
                    self.tabs.set_audible(tab, playing);
                    self.push_tabs();
                }
            }
            PageEvent::FormDirty { dirty } => {
                if let Some(tab) = tab {
                    self.tabs.set_form_dirty(tab, dirty);
                }
            }
            PageEvent::Scroll { y } => {
                if let Some(tab) = tab {
                    self.tabs.set_scroll(tab, y);
                }
            }
            PageEvent::MouseNav { forward } => {
                if let Some(tab) = tab {
                    if let Some(view) = self.tab_views.get(&tab).copied() {
                        let _ = if forward {
                            self.engine.go_forward(view)
                        } else {
                            self.engine.go_back(view)
                        };
                    }
                }
            }
            PageEvent::Shortcut { shortcut } => self.handle_shortcut(shortcut),
            PageEvent::Favicon { href } => {
                if let Some(tab) = tab {
                    self.tabs.set_favicon(tab, Some(href.clone()));
                    if let Some(t) = self.tabs.tab(tab) {
                        if let Ok(true) = self.store.is_bookmarked(&t.url) {
                            let _ = self.store.update_bookmark_favicon(&t.url, &href);
                            self.send_pinned_sites();
                        }
                    }
                    self.push_tabs();
                }
            }
            PageEvent::BlockedResource { .. } => {
                if let Some(tab) = tab {
                    self.tab_counters.entry(tab).or_default().record(1);
                    self.send_adblock_state();
                }
            }
            PageEvent::CookieSet { cookie, url } => {
                if let Some(stored) = parse_cookie_string(&cookie, &url) {
                    let _ = self.store.save_cookie(&stored);
                    self.send_cookies_count();
                }
            }
            PageEvent::UrlChanged { url } => {
                if let Some(tab) = tab {
                    let clean_url = if is_subframe_or_widget_url(&url) {
                        String::new()
                    } else {
                        url
                    };
                    if !clean_url.is_empty() && !peek_core::url::is_blank(&clean_url) {
                        self.tabs.set_url(tab, &clean_url);
                        self.send(Event::UrlChanged {
                            id: tab,
                            url: clean_url.clone(),
                        });
                        self.push_tabs();
                    }
                }
            }
            PageEvent::InvertDarkState { active, message } => {
                self.send(Event::InvertDarkState { active });
                if !message.trim().is_empty() {
                    self.send(Event::Toast { message });
                }
            }
            PageEvent::ExtensionMessage { .. } => {}

        }
        let _ = view;
    }

    fn handle_shortcut(&mut self, shortcut: Shortcut) {
        match shortcut {
            Shortcut::FocusOmnibar => {
                self.close_drawer();
                self.send(Event::FocusOmnibar);
            }
            Shortcut::NewTab => {
                self.new_tab(peek_core::url::blank_page());
                self.send(Event::FocusOmnibar);
            }
            Shortcut::CloseTab => {
                if let Some(id) = self.tabs.active_id() {
                    let actions = self.tabs.close(id);
                    self.apply(actions);
                    self.push_tabs();
                }
            }
            Shortcut::NextTab => self.cycle_tab(true),
            Shortcut::PrevTab => self.cycle_tab(false),
            Shortcut::SelectTab(n) => {
                let ids: Vec<TabId> = self.tabs.tabs().iter().map(|t| t.id).collect();
                if let Some(id) = ids.get((n as usize).saturating_sub(1)).copied() {
                    let actions = self.tabs.activate(id);
                    self.apply(actions);
                    self.push_tabs();
                }
            }
            Shortcut::Reload => {
                if let Some(view) = self.active_view() {
                    let _ = self.engine.reload(view);
                }
            }
            Shortcut::Back => {
                if let Some(view) = self.active_view() {
                    let _ = self.engine.go_back(view);
                }
            }
            Shortcut::Forward => {
                if let Some(view) = self.active_view() {
                    let _ = self.engine.go_forward(view);
                }
            }
            Shortcut::Escape => {
                self.hide_preview();
                self.close_drawer();
            }
            Shortcut::ToggleDrawer => {
                if self.drawer_open {
                    self.close_drawer();
                } else {
                    self.open_drawer_on_side(self.settings.drawer_side, None);
                }
            }
            Shortcut::OpenTabsDrawer => self.open_drawer_on_side(EdgeSide::Left, Some("tabs")),
            Shortcut::OpenSettingsDrawer => self.open_drawer_on_side(EdgeSide::Right, Some("settings")),
            Shortcut::OpenCommandPalette => self.send(Event::FocusOmnibar),
            Shortcut::CopyCleanUrl => {
                if let Some(tab) = self.tabs.active() {
                    let clean = peek_core::url::strip_tracking_params(&tab.url);
                    if let Some(ui_id) = self.ui_id {
                        let escaped = serde_json::to_string(&clean).unwrap_or_default();
                        let js = format!("if (navigator.clipboard) navigator.clipboard.writeText({escaped});");
                        let _ = self.engine.evaluate(ui_id, &js);
                    }
                    self.send(Event::Toast {
                        message: "Clean URL copied".into(),
                    });
                }
            }
            Shortcut::FindInPage => {
                if let Some(view) = self.active_view() {
                    let _ = self.engine.evaluate(view, "if (window.__peekShowFind) window.__peekShowFind();");
                }
            }
            Shortcut::ToggleReaderMode => {
                if let Some(view) = self.active_view() {
                    let _ = self.engine.evaluate(view, "if (window.__peekToggleReader) window.__peekToggleReader();");
                }
            }
            Shortcut::ToggleMute => {
                if let Some(view) = self.active_view() {
                    let _ = self.engine.evaluate(view, "if (window.__peekToggleMute) window.__peekToggleMute();");
                    self.send(Event::Toast { message: "Toggled audio mute".into() });
                }
            }
            Shortcut::ToggleZenMode => self.handle_command(Command::ToggleZenMode),
            Shortcut::ToggleInvertDark => self.handle_command(Command::ToggleInvertDark),
            Shortcut::TogglePiP => self.handle_command(Command::TogglePiP),
            Shortcut::ZoomIn => self.handle_command(Command::ZoomIn),
            Shortcut::ZoomOut => self.handle_command(Command::ZoomOut),
            Shortcut::ZoomReset => self.handle_command(Command::ZoomReset),
            Shortcut::CopyMarkdownLink => self.handle_command(Command::CopyMarkdownLink),
        }
    }

    fn apply_setting(&mut self, key: &str, value: serde_json::Value) {
        if self.store.set_setting_json(key, &value).is_err() {
            return;
        }
        match key {
            "maxLive" => {
                if let Some(n) = value.as_u64() {
                    self.settings.max_live = (n as usize).max(1);
                    self.tabs.set_config(self.settings.tab_config());
                    let actions = self.tabs.enforce_live_limit();
                    self.apply(actions);
                    self.push_tabs();
                }
            }
            "idleDiscardMinutes" => {
                if let Some(n) = value.as_u64() {
                    self.settings.idle_discard_minutes = n;
                    self.tabs.set_config(self.settings.tab_config());
                }
            }
            "searchEngine" => {
                if let Ok(engine) = serde_json::from_value::<SearchEngine>(value) {
                    self.settings.search_engine = engine;
                }
            }
            "hoverDelayMs" => {
                if let Some(n) = value.as_u64() {
                    self.settings.hover_delay_ms = n;
                    self.settings.preview.hover_delay = Duration::from_millis(n);
                }
            }
            "drawerSide" => {
                if let Ok(side) = serde_json::from_value::<EdgeSide>(value) {
                    self.settings.drawer_side = side;
                }
            }
            "edgeZone" => {
                if let Some(n) = value.as_f64() {
                    self.settings.edge_zone = n.max(1.0);
                }
            }
            "previewModifierOnly" => {
                if let Some(flag) = value.as_bool() {
                    self.settings.preview.modifier_only = flag;
                }
            }
            "startBlank" => {
                if let Some(flag) = value.as_bool() {
                    self.settings.start_blank = flag;
                }
            }
            _ => {}
        }
        self.send_settings();
        self.send(Event::Toast {
            message: "Setting saved".into(),
        });
    }

    fn navigate_from_input(&mut self, input: &str) {
        let target = resolve_input(input, self.tabs.tabs(), true);
        match target {
            NavigateTarget::Url(url) => self.navigate_active(&url),
            NavigateTarget::Search(query) => {
                let url = self.settings.search_engine.search_url(&query);
                self.navigate_active(&url);
            }
            NavigateTarget::SwitchToTab(id) => {
                let actions = self.tabs.activate(id);
                self.apply(actions);
                self.push_tabs();
            }
        }
    }

    fn navigate_active(&mut self, url: &str) {
        let Some(id) = self.tabs.active_id() else {
            self.new_tab(url);
            return;
        };
        let actions = self.tabs.navigate(id, url);
        self.apply(actions);
        self.push_tabs();
    }

    fn new_tab(&mut self, url: &str) -> TabId {
        let (id, actions) = self.tabs.create_tab(url);
        self.apply(actions);
        self.push_tabs();
        id
    }

    fn cycle_tab(&mut self, forward: bool) {
        let ids: Vec<TabId> = self.tabs.tabs().iter().map(|t| t.id).collect();
        if ids.len() < 2 {
            return;
        }
        let active = self.tabs.active_id();
        let index = ids.iter().position(|id| Some(*id) == active).unwrap_or(0);
        let next = if forward {
            (index + 1) % ids.len()
        } else {
            (index + ids.len() - 1) % ids.len()
        };
        let actions = self.tabs.activate(ids[next]);
        self.apply(actions);
        self.push_tabs();
    }

    fn apply(&mut self, actions: Vec<TabAction>) {
        for action in actions {
            match action {
                TabAction::Create { id, url } => {
                    self.close_drawer();
                    self.hide_preview();
                    if let Err(error) = self.create_tab_view(id, &url) {
                        eprintln!("peek: failed to create tab webview {id}: {error}");
                    }
                }
                TabAction::Destroy { id } => self.destroy_tab_view(id),
                TabAction::Show { id, visible } => {
                    if let Some(view) = self.tab_views.get(&id).copied() {
                        if visible {
                            if let Some(t) = self.tabs.tab(id) {
                                if peek_core::url::is_blank(&t.url) {
                                    if let Some(live_url) = self.engine.url(view) {
                                        if !live_url.is_empty() && !peek_core::url::is_blank(&live_url) {
                                            self.tabs.set_url(id, &live_url);
                                        }
                                    }
                                }
                            }
                        }
                        let is_blank = self
                            .tabs
                            .tab(id)
                            .map(|t| peek_core::url::is_blank(&t.url))
                            .unwrap_or(true);
                        let should_show = visible && !is_blank;
                        let _ = self.engine.set_visible(view, should_show);
                        if should_show {
                            let _ = self.engine.set_bounds(view, self.page_bounds());
                            let _ = self.engine.focus(view);
                            let _ = self.engine.evaluate(
                                view,
                                "if (window.__peekCheckInvertDark) window.__peekCheckInvertDark();",
                            );
                        }
                    }
                }
                TabAction::Load { id, url } => {
                    if let Some(view) = self.tab_views.get(&id).copied() {
                        let _ = self.engine.load_url(view, &url);
                        let is_blank = peek_core::url::is_blank(&url);
                        let is_active = self.tabs.active_id() == Some(id);
                        let should_show = is_active && !is_blank;
                        let _ = self.engine.set_visible(view, should_show);
                        if should_show {
                            let _ = self.engine.focus(view);
                        }
                    }
                }
                TabAction::Scroll { id, y } => {
                    if let Some(view) = self.tab_views.get(&id).copied() {
                        let _ = self
                            .engine
                            .evaluate(view, &format!("window.scrollTo(0,{y});"));
                    }
                }
            }
        }
        if let Some(id) = self.ui_id {
            let bounds = self.chrome_bounds();
            let _ = self.engine.set_bounds(id, bounds);
            if self.is_active_blank() {
                let _ = self.engine.set_visible(id, true);
                let _ = self.engine.focus(id);
            }
        }
        self.ensure_layer_order();
    }

    fn create_tab_view(&mut self, id: TabId, url: &str) -> anyhow::Result<()> {
        if self.tab_views.contains_key(&id) {
            return Ok(());
        }
        let script = inject::page_script(
            &self.page_nonce,
            "tab",
            self.settings.hover_delay_ms,
            match self.settings.drawer_side {
                EdgeSide::Left => "left",
                EdgeSide::Right => "right",
            },
            self.settings.edge_zone,
        );
        let is_blank = peek_core::url::is_blank(url);
        let mut init_scripts = vec![script];
        for us in self.userscripts.matching_scripts(url, "document-start") {
            init_scripts.push(us.generate_injected_js());
        }
        let data_store_identifier = if self.current_profile_name == "default" {
            None
        } else {
            Some(self.profile_uuid)
        };
        let view = self.engine.create(
            &self.window,
            WebViewOptions {
                bounds: self.page_bounds(),
                url: Some(url.to_string()),
                assets: Some(Arc::new(PageAssets)),
                visible: !is_blank,
                initialization_scripts: init_scripts,
                accept_first_mouse: true,
                data_directory: Some(self.profile_web_data_dir.clone()),
                data_store_identifier,
                incognito: false,
                user_agent: Some(default_user_agent().to_string()),
                ..Default::default()
            },
        )?;
        debug_log(&format!("created tab {id} as view {view}"));
        self.tab_views.insert(id, view);
        self.view_tabs.insert(view, id);
        self.tabs.mark_live(id);
        self.ensure_layer_order();
        if !is_blank {
            let _ = self.engine.focus(view);
        }
        Ok(())
    }

    fn destroy_tab_view(&mut self, id: TabId) {
        self.tab_counters.remove(&id);
        if let Some(view) = self.tab_views.remove(&id) {
            self.view_tabs.remove(&view);
            self.engine.destroy(view);
        }
    }


    fn active_view(&self) -> Option<WebViewId> {
        self.tabs
            .active_id()
            .and_then(|id| self.tab_views.get(&id).copied())
    }

    fn create_chrome(&mut self) -> anyhow::Result<()> {
        let id = self.engine.create(
            &self.window,
            WebViewOptions {
                bounds: self.chrome_bounds(),
                url: Some("peek://localhost/index.html#bar".into()),
                assets: Some(Arc::new(UiAssets)),
                transparent: true,
                initialization_scripts: vec![inject::ui_script(&self.ui_nonce)],
                accept_first_mouse: true,
                ..Default::default()
            },
        )?;
        self.ui_id = Some(id);
        Ok(())
    }

    fn create_drawer_prewarm(&mut self) -> anyhow::Result<()> {
        if self.drawer_id.is_some() {
            return Ok(());
        }
        let bounds = self.drawer_bounds_for_side(EdgeSide::Left);
        let script = inject::ui_script(&self.ui_nonce);
        let url = "peek://localhost/index.html#drawer?side=left&section=tabs".to_string();
        let id = self.engine.create(
            &self.window,
            WebViewOptions {
                bounds,
                url: Some(url),
                assets: Some(Arc::new(UiAssets)),
                transparent: true,
                initialization_scripts: vec![script],
                visible: false,
                accept_first_mouse: true,
                ..Default::default()
            },
        )?;
        self.drawer_id = Some(id);
        Ok(())
    }

    fn ensure_layer_order(&self) {
        if let Some(view) = self.active_view() {
            let _ = self.engine.send_to_back(view);
        }
        if let Some(ui_id) = self.ui_id {
            let _ = self.engine.bring_to_front(ui_id);
        }
        if self.drawer_open {
            if let Some(drawer_id) = self.drawer_id {
                let _ = self.engine.bring_to_front(drawer_id);
            }
        }
        if let Some(preview_id) = self.preview_id {
            let _ = self.engine.bring_to_front(preview_id);
        }
    }

    fn open_drawer_on_side(&mut self, side: EdgeSide, section: Option<&str>) {
        self.current_drawer_side = side;
        let bounds = self.drawer_bounds_for_side(side);
        let section_param = section.unwrap_or(match side {
            EdgeSide::Left => "tabs",
            EdgeSide::Right => "settings",
        });

        self.omnibar_open = false;
        if let Some(id) = self.ui_id {
            let _ = self.engine.set_bounds(id, self.chrome_bounds());
        }

        if let Some(id) = self.drawer_id {
            let _ = self.engine.set_bounds(id, bounds);
            let _ = self.engine.set_visible(id, true);
            self.drawer_open = true;
            self.ensure_layer_order();
            let _ = self.engine.focus(id);
            self.send(Event::DrawerState {
                open: true,
                side: Some(side),
                section: Some(section_param.to_string()),
            });
            self.push_tabs();
            self.send_profiles_list();
            self.send_extensions_list();
            self.send_tab_groups();
            self.send_userscripts_list();
            return;
        }

        let script = inject::ui_script(&self.ui_nonce);
        let url = format!(
            "peek://localhost/index.html#drawer?side={:?}&section={}",
            side, section_param
        )
        .to_lowercase();
        match self.engine.create(
            &self.window,
            WebViewOptions {
                bounds,
                url: Some(url),
                assets: Some(Arc::new(UiAssets)),
                transparent: true,
                initialization_scripts: vec![script],
                visible: true,
                accept_first_mouse: true,
                ..Default::default()
            },
        ) {
            Ok(id) => {
                self.drawer_id = Some(id);
                self.drawer_open = true;
                self.ensure_layer_order();
                let _ = self.engine.focus(id);
                self.send(Event::DrawerState {
                    open: true,
                    side: Some(side),
                    section: Some(section_param.to_string()),
                });
                self.push_tabs();
                self.send_profiles_list();
                self.send_extensions_list();
                self.send_tab_groups();
                self.send_userscripts_list();
            }
            Err(error) => eprintln!("peek: failed to open drawer: {error}"),
        }
    }

    fn close_drawer(&mut self) {
        if let Some(id) = self.drawer_id {
            let _ = self.engine.set_visible(id, false);
        }
        if self.drawer_open {
            self.drawer_open = false;
            self.send(Event::DrawerState {
                open: false,
                side: Some(self.current_drawer_side),
                section: None,
            });
        }
        self.ensure_layer_order();
        if self.is_active_blank() {
            if let Some(id) = self.ui_id {
                let _ = self.engine.focus(id);
            }
        } else if let Some(view) = self.active_view() {
            let _ = self.engine.focus(view);
        }
    }

    fn show_preview(&mut self, url: &str, bounds: Bounds, zoom: f64) {
        if let Some(id) = self.preview_id {
            let _ = self.engine.set_bounds(id, bounds);
            let _ = self.engine.set_visible(id, true);
            let _ = self.engine.bring_to_front(id);
            let _ = self.engine.load_url(id, url);
            return;
        }
        let script = inject::page_script(
            &self.page_nonce,
            "preview",
            self.settings.hover_delay_ms,
            "right",
            self.settings.edge_zone,
        );
        match self.engine.create(
            &self.window,
            WebViewOptions {
                bounds,
                url: Some(url.to_string()),
                visible: true,
                autoplay: false,
                incognito: true,
                devtools: false,
                initialization_scripts: vec![script],
                zoom: Some(zoom),
                user_agent: Some(default_user_agent().to_string()),
                corner_radius: Some(14.0),
                border_width: Some(1.5),
                border_color: Some([0.35, 0.55, 0.95, 0.65]),
                ..Default::default()
            },
        ) {
            Ok(id) => {
                self.preview_id = Some(id);
                let _ = self.engine.bring_to_front(id);
                let _ = self.engine.focus(id);
            }
            Err(error) => {
                eprintln!("peek: failed to create preview: {error}");
                self.preview = PreviewController::default();
            }
        }
    }

    fn schedule_preview_hide(&mut self) {
        if self.preview_id.is_some() {
            self.preview_hide_at = Some(Instant::now() + PREVIEW_HIDE_GRACE);
        }
    }

    fn hide_preview(&mut self) {
        if let Some(id) = self.preview_id.take() {
            self.engine.destroy(id);
        }
        self.preview = PreviewController::default();
        self.preview_hide_at = None;
    }

    fn on_tick(&mut self) {
        self.ticks = self.ticks.wrapping_add(1);
        if let Some(deadline) = self.preview_hide_at {
            if Instant::now() >= deadline {
                self.hide_preview();
            }
        }
        if self.ticks % IDLE_TICK_EVERY == 0 {
            let actions = self.tabs.tick(now_ms());
            if !actions.is_empty() {
                self.apply(actions);
                self.push_tabs();
            } else {
                self.save_session();
            }
        }
    }

    fn layout(&mut self) {
        let size = self.window.inner_size();
        let scale = self.window.scale_factor().max(0.1);
        let w = (size.width as f64 / scale).max(1.0);
        let h = (size.height as f64 / scale).max(1.0);
        self.window_size = (w, h);

        if let Some(id) = self.ui_id {
            let _ = self.engine.set_bounds(id, self.chrome_bounds());
        }
        let active_id = self.tabs.active_id();
        let is_blank = self.is_active_blank();
        let page = self.page_bounds();
        for (tab_id, view) in &self.tab_views {
            let _ = self.engine.set_bounds(*view, page);
            let should_show = Some(*tab_id) == active_id && !is_blank;
            let _ = self.engine.set_visible(*view, should_show);
            if should_show {
                let _ = self.engine.focus(*view);
            }
        }
        if let Some(id) = self.drawer_id {
            let _ = self.engine.set_bounds(id, self.drawer_bounds());
        }
        self.hide_preview();
        self.ensure_layer_order();
        if is_blank {
            if let Some(id) = self.ui_id {
                let _ = self.engine.focus(id);
            }
        }
    }

    fn is_active_blank(&self) -> bool {
        self.tabs
            .active()
            .map(|t| peek_core::url::is_blank(&t.url))
            .unwrap_or(true)
    }

    pub fn bar_height(&self) -> f64 {
        if self.window.fullscreen().is_some() {
            BAR_HEIGHT_FULLSCREEN
        } else {
            BAR_HEIGHT
        }
    }

    fn chrome_bounds(&self) -> Bounds {
        let (w, h) = self.window_size;
        let bar_h = self.bar_height();
        if self.is_active_blank() {
            Bounds::new(0.0, 0.0, w, h)
        } else if self.is_zen_mode {
            let height = if self.omnibar_open {
                bar_h + OMNIBAR_PANEL_HEIGHT
            } else {
                bar_h
            };
            Bounds::new(0.0, 0.0, w, height)
        } else {
            let height = bar_h
                + if self.omnibar_open {
                    OMNIBAR_PANEL_HEIGHT
                } else {
                    0.0
                };
            Bounds::new(0.0, 0.0, w, height)
        }
    }

    fn page_bounds(&self) -> Bounds {
        let (w, h) = self.window_size;
        let bar_h = self.bar_height();
        if self.is_zen_mode {
            Bounds::new(0.0, 0.0, w, h)
        } else {
            Bounds::new(0.0, bar_h, w, (h - bar_h).max(0.0))
        }
    }

    fn drawer_bounds_for_side(&self, side: EdgeSide) -> Bounds {
        let (w, h) = self.window_size;
        let width = match side {
            EdgeSide::Left => 320.0f64,
            EdgeSide::Right => 360.0f64,
        }
        .min(w);
        let x = match side {
            EdgeSide::Left => 0.0,
            EdgeSide::Right => (w - width).max(0.0),
        };
        Bounds::new(x, 0.0, width, h)
    }

    fn drawer_bounds(&self) -> Bounds {
        self.drawer_bounds_for_side(self.current_drawer_side)
    }

    fn profiles_list(&self) -> Vec<String> {
        let mut profiles = vec!["default".to_string()];
        if let Some(parent) = self.profile_dir.parent() {
            if let Ok(entries) = std::fs::read_dir(parent) {
                for entry in entries.flatten() {
                    if let Ok(file_type) = entry.file_type() {
                        if file_type.is_dir() {
                            let name = entry.file_name().to_string_lossy().to_string();
                            if name != "default" && !name.starts_with('.') {
                                profiles.push(name);
                            }
                        }
                    }
                }
            }
        }
        profiles.sort();
        profiles.dedup();
        profiles
    }

    fn send_profiles_list(&self) {
        self.send(Event::ProfilesList {
            profiles: self.profiles_list(),
            active: self.current_profile_name.clone(),
        });
    }

    fn create_profile(&mut self, name: &str) {
        let clean = name.trim();
        if clean.is_empty()
            || clean.contains('/')
            || clean.contains('\\')
            || clean.starts_with('.')
        {
            return;
        }
        let parent = match self.profile_dir.parent() {
            Some(p) => p.to_path_buf(),
            None => return,
        };
        let new_dir = parent.join(clean);
        let _ = std::fs::create_dir_all(&new_dir);
        let _ = std::fs::create_dir_all(new_dir.join("web_data"));
        let _ = std::fs::create_dir_all(new_dir.join("extensions"));
        let db_path = new_dir.join("peek.db");
        if let Ok(store) = Store::open(&db_path) {
            let uuid = derive_profile_uuid(clean);
            let hex_str = uuid.iter().map(|b| format!("{b:02x}")).collect::<String>();
            let _ = store.set_setting("profileUuid", &hex_str);
            let _ = store.seed_default_bookmarks(now_ms());
        }
        self.send_profiles_list();
        self.send(Event::Toast {
            message: format!("Profile '{clean}' created"),
        });
    }

    fn switch_profile(&mut self, name: &str) {
        let clean = name.trim();
        if clean.is_empty() || clean == self.current_profile_name {
            return;
        }
        let parent = match self.profile_dir.parent() {
            Some(p) => p.to_path_buf(),
            None => return,
        };
        let target_dir = parent.join(clean);
        if !target_dir.exists() {
            return;
        }

        // 1. Save current session and state to old profile store
        self.save_session();

        // 2. Destroy old tab views and preview
        for (_tab_id, view_id) in self.tab_views.drain() {
            self.engine.destroy(view_id);
        }
        self.view_tabs.clear();
        if let Some(pid) = self.preview_id.take() {
            self.engine.destroy(pid);
        }
        self.preview = PreviewController::default();
        self.preview_hide_at = None;

        // 3. Persist active profile name to disk
        let active_file = parent.join("active_profile");
        let _ = std::fs::write(&active_file, clean);

        // 4. Open new profile database and directories
        let db_path = target_dir.join("peek.db");
        let new_store = match Store::open(&db_path) {
            Ok(s) => s,
            Err(error) => {
                eprintln!("peek: failed to open profile db at {}: {error}", db_path.display());
                return;
            }
        };

        self.store = new_store;
        self.current_profile_name = clean.to_string();
        self.profile_dir = target_dir.clone();
        self.profile_web_data_dir = target_dir.join("web_data");
        let _ = std::fs::create_dir_all(&self.profile_web_data_dir);

        // 5. Derive or load profile UUID for cookie & cache partition
        let uuid = match self.store.get_setting("profileUuid") {
            Ok(Some(hex_str)) if hex_str.len() == 32 => {
                let mut bytes = [0u8; 16];
                let mut valid = true;
                for i in 0..16 {
                    if let Ok(b) = u8::from_str_radix(&hex_str[i * 2..i * 2 + 2], 16) {
                        bytes[i] = b;
                    } else {
                        valid = false;
                        break;
                    }
                }
                if valid {
                    bytes
                } else {
                    let u = derive_profile_uuid(clean);
                    let hex_str = u.iter().map(|b| format!("{b:02x}")).collect::<String>();
                    let _ = self.store.set_setting("profileUuid", &hex_str);
                    u
                }
            }
            _ => {
                let u = derive_profile_uuid(clean);
                let hex_str = u.iter().map(|b| format!("{b:02x}")).collect::<String>();
                let _ = self.store.set_setting("profileUuid", &hex_str);
                u
            }
        };
        self.profile_uuid = uuid;

        // 6. Reload profile settings and extensions
        self.settings = Settings::load(&self.store);
        self.current_drawer_side = self.settings.drawer_side;
        let extensions_dir = self.profile_dir.join("extensions");
        let _ = std::fs::create_dir_all(&extensions_dir);
        self.extensions = ExtensionRegistry::load_from_dir(&extensions_dir);
        let userscripts_dir = self.profile_dir.join("userscripts");
        let _ = std::fs::create_dir_all(&userscripts_dir);
        self.userscripts = UserScriptRegistry::load_from_dir(&userscripts_dir);
        let _ = self.store.seed_default_bookmarks(now_ms());

        // 7. Restore tabs from the new profile store
        if let Err(error) = self.restore_tabs_from_store() {
            eprintln!("peek: error restoring tabs for profile '{clean}': {error}");
        }

        // 8. Relayout and sync UI state via IPC
        self.layout();
        self.send(Event::ProfileChanged {
            name: clean.to_string(),
        });
        self.send_profiles_list();
        self.push_tabs();
        self.send_pinned_sites();
        self.send_cookies_count();
        self.send_extensions_list();
        self.send_tab_groups();
        self.send_userscripts_list();
        self.send_bookmarks_list();
        self.send_history_list();
        self.send_note();
        self.send_settings();
        self.send(Event::Toast {
            message: format!("Switched to profile '{clean}'"),
        });
    }

    fn delete_profile(&mut self, name: &str) {
        let clean = name.trim();
        if clean.is_empty() || clean == "default" || clean == self.current_profile_name {
            return;
        }
        let parent = match self.profile_dir.parent() {
            Some(p) => p.to_path_buf(),
            None => return,
        };
        let target = parent.join(clean);
        let _ = std::fs::remove_dir_all(target);
        self.send_profiles_list();
        self.send(Event::Toast {
            message: format!("Profile '{clean}' deleted"),
        });
    }

    fn send_extensions_list(&self) {
        let extensions = self
            .extensions
            .extensions
            .iter()
            .map(|e| ExtensionSummary {
                id: e.id.clone(),
                name: e.name.clone(),
                version: e.version.clone(),
                description: e.description.clone(),
                enabled: e.enabled,
                fully_supported: e.report.is_fully_supported(),
            })
            .collect();
        self.send(Event::ExtensionsList { extensions });
    }

    fn install_extension_path(&mut self, path: &str) {
        let p = std::path::Path::new(path);
        if !p.exists() {
            self.send(Event::Toast {
                message: format!("Extension path not found: {path}"),
            });
            return;
        }

        let extensions_dir = self.profile_dir.join("extensions");
        let result = if p.is_dir() {
            peek_ext::ExtensionRegistry::install_from_folder(p, &extensions_dir)
        } else if let Ok(bytes) = std::fs::read(p) {
            peek_ext::ExtensionRegistry::install_from_bytes(&bytes, &extensions_dir)
        } else {
            self.send(Event::Toast {
                message: "Could not read extension file".into(),
            });
            return;
        };

        match result {
            Ok(ext) => {
                let name = ext.name.clone();
                self.extensions.extensions.retain(|e| e.id != ext.id);
                self.extensions.extensions.push(ext);
                self.send(Event::Toast {
                    message: format!("Installed extension: {name}"),
                });
                self.send_extensions_list();
            }
            Err(err) => {
                self.send(Event::Toast {
                    message: format!("Failed to install: {err}"),
                });
            }
        }
    }

    fn toggle_extension(&mut self, id: &str, enabled: bool) {
        if let Some(ext) = self.extensions.extensions.iter_mut().find(|e| e.id == id) {
            ext.enabled = enabled;
            self.send_extensions_list();
        }
    }

    fn uninstall_extension(&mut self, id: &str) {
        if let Some(pos) = self.extensions.extensions.iter().position(|e| e.id == id) {
            let ext = self.extensions.extensions.remove(pos);
            let _ = std::fs::remove_dir_all(&ext.dir);
            self.send(Event::Toast {
                message: format!("Removed {}", ext.name),
            });
            self.send_extensions_list();
        }
    }

    fn suggestions(&self, query: &str) -> Vec<Suggestion> {
        let mut items: Vec<Suggestion> = Vec::new();
        let q = query.trim();
        if q.is_empty() {
            for entry in self.store.recent_history(5).unwrap_or_default() {
                items.push(history_suggestion(&entry.url, &entry.title));
            }
            return items;
        }

        let needle = q.to_lowercase();
        for tab in self.tabs.tabs() {
            if tab.title.to_lowercase().contains(&needle) {
                items.push(Suggestion {
                    kind: SuggestionKind::Tab,
                    text: tab.title.clone(),
                    detail: Some(tab.url.clone()),
                    url: Some(tab.url.clone()),
                    tab_id: Some(tab.id),
                });
            }
        }
        for bookmark in self.store.search_bookmarks(q, 4).unwrap_or_default() {
            items.push(Suggestion {
                kind: SuggestionKind::Bookmark,
                text: if bookmark.title.is_empty() {
                    bookmark.url.clone()
                } else {
                    bookmark.title
                },
                detail: Some(bookmark.url.clone()),
                url: Some(bookmark.url),
                tab_id: None,
            });
        }
        for entry in self.store.search_history(q, 5).unwrap_or_default() {
            items.push(history_suggestion(&entry.url, &entry.title));
        }
        items.push(Suggestion {
            kind: SuggestionKind::Search,
            text: q.to_string(),
            detail: Some(self.settings.search_engine.name.clone()),
            url: Some(self.settings.search_engine.search_url(q)),
            tab_id: None,
        });

        let mut seen = std::collections::HashSet::new();
        items.retain(|item| match &item.url {
            Some(url) => seen.insert(url.clone()),
            None => true,
        });
        items.truncate(9);
        items
    }

    fn send_state(&self) {
        self.send(Event::TabsChanged {
            tabs: self.tabs.snapshot(),
            active: self.tabs.active_id(),
        });
        self.send(Event::DrawerState {
            open: self.drawer_open,
            side: Some(self.current_drawer_side),
            section: None,
        });
        self.send_adblock_state();
        self.send_settings();
        self.send_cookies_count();
        self.send_profiles_list();
        self.send_extensions_list();
        self.send_pinned_sites();
    }

    fn send_pinned_sites(&self) {
        let sites: Vec<peek_ipc::PinnedSite> = self
            .store
            .bookmarks()
            .unwrap_or_default()
            .into_iter()
            .map(|b| peek_ipc::PinnedSite {
                id: b.id,
                url: b.url,
                title: b.title,
                favicon: b.favicon,
                position: b.position,
            })
            .collect();
        self.send(Event::PinnedSitesChanged { sites });
    }

    fn send_cookies_count(&self) {
        let count = self.store.cookie_count().unwrap_or(0);
        self.send(Event::CookiesCount { count });
    }

    fn send_settings(&self) {
        self.send(Event::Settings {
            max_live: self.settings.max_live,
            idle_discard_minutes: self.settings.idle_discard_minutes,
            hover_delay_ms: self.settings.hover_delay_ms,
            drawer_side: self.settings.drawer_side,
            edge_zone: self.settings.edge_zone,
            start_blank: self.settings.start_blank,
            search_engine: self.settings.search_engine.name.clone(),
        });
    }

    fn send_bookmarks_list(&self) {
        let list = self.store.bookmarks().unwrap_or_default();
        let bookmarks = list
            .into_iter()
            .map(|b| BookmarkItem {
                id: b.id,
                url: b.url,
                title: b.title,
                favicon: b.favicon,
                created_at: b.created_at,
            })
            .collect();
        self.send(Event::BookmarksList { bookmarks });
    }

    fn send_history_list(&self) {
        let list = self.store.recent_history(100).unwrap_or_default();
        let entries = list
            .into_iter()
            .map(|h| HistoryItem {
                url: h.url,
                title: h.title,
                visit_count: h.visit_count,
                last_visit: h.last_visit,
            })
            .collect();
        self.send(Event::HistoryList { entries });
    }

    fn send_note(&self) {
        let content = self
            .store
            .get_setting("scratchpad_notes")
            .unwrap_or_default()
            .unwrap_or_default();
        self.send(Event::NoteLoaded { content });
    }

    fn send_tab_groups(&self) {
        let groups: Vec<TabGroup> = self
            .store
            .get_setting_or("tab_groups", Vec::new())
            .unwrap_or_default();
        self.send(Event::TabGroupsChanged { groups });
    }

    fn send_userscripts_list(&self) {
        let summaries: Vec<UserScriptSummary> = self
            .userscripts
            .scripts
            .iter()
            .map(|s| UserScriptSummary {
                id: s.id.clone(),
                name: s.name.clone(),
                version: s.version.clone(),
                description: s.description.clone(),
                author: s.author.clone(),
                match_patterns: s.match_patterns.clone(),
                run_at: s.run_at.clone(),
                enabled: s.enabled,
                code: s.code.clone(),
            })
            .collect();
        self.send(Event::UserScriptsList { scripts: summaries });
    }

    fn send_adblock_state(&self) {
        let host = self
            .tabs
            .active()
            .and_then(|tab| Url::parse(&tab.url).ok())
            .and_then(|url| url.host_str().map(|h| h.to_string()));
        let active_id = self.tabs.active_id();
        let blocked = active_id
            .and_then(|id| self.tab_counters.get(&id))
            .map(|c| c.blocked)
            .unwrap_or(0);
        let allowed = host
            .as_ref()
            .map(|host| self.store.site_adblock_enabled(host).ok().flatten() == Some(false))
            .unwrap_or(false);
        self.send(Event::AdblockCount {
            blocked,
            allowed_on_site: allowed,
            host,
        });
    }

    fn apply_cosmetic_filters(&self, view: WebViewId, tab_url: &str) {
        if !self.adblock_config.enabled || !self.adblock_config.cosmetic {
            return;
        }
        let Ok(parsed) = Url::parse(tab_url) else {
            return;
        };
        let Some(host) = parsed.host_str() else {
            return;
        };
        if self.store.site_adblock_enabled(host).ok().flatten() == Some(false) {
            return;
        }
        let (css, script) = self.adblock.cosmetic_resources(tab_url);
        if let Some(css) = css {
            let js = format!(
                "(function(){{var s=document.getElementById('__peek_adblock__');if(!s){{s=document.createElement('style');s.id='__peek_adblock__';(document.head||document.documentElement).appendChild(s);}}s.textContent={:?};}})()",
                css
            );
            let _ = self.engine.evaluate(view, &js);
        }
        if let Some(script) = script {
            let _ = self.engine.evaluate(view, &script);
        }
    }


    fn send_loading(&self, tab: TabId) {
        let Some(info) = self.tabs.tab(tab) else {
            return;
        };
        let view = self.tab_views.get(&tab).copied();
        self.send(Event::Loading {
            id: tab,
            url: info.url.clone(),
            loading: info.loading,
            can_go_back: view.map(|v| self.engine.can_go_back(v)).unwrap_or(false),
            can_go_forward: view.map(|v| self.engine.can_go_forward(v)).unwrap_or(false),
        });
    }

    fn push_tabs(&mut self) {
        self.send(Event::TabsChanged {
            tabs: self.tabs.snapshot(),
            active: self.tabs.active_id(),
        });
        self.save_session();
    }

    fn save_session(&mut self) {
        let tabs = self.tabs.stored_tabs();
        if let Err(error) = self.store.save_tabs(&tabs) {
            eprintln!("peek: failed to save tabs: {error}");
            return;
        }
        let (w, h) = self.window_size;
        let session = Session {
            active_tab: self.tabs.active_id(),
            window_layout: format!("{{\"width\":{w},\"height\":{h}}}"),
            created_at: now_ms(),
        };
        if let Err(error) = self.store.save_session(&session) {
            eprintln!("peek: failed to save session: {error}");
        }
    }

    fn send(&self, event: Event) {
        for target in [self.ui_id, self.drawer_id].into_iter().flatten() {
            let envelope = Envelope::new(self.ui_nonce.as_str(), event.clone());
            let Ok(json) = envelope.to_json() else {
                continue;
            };
            let Ok(literal) = serde_json::to_string(&json) else {
                continue;
            };
            let _ = self.engine.evaluate(
                target,
                &format!("window.__peekDeliver&&window.__peekDeliver({literal})"),
            );
        }
    }

    fn shutdown(&mut self) {
        self.running.store(false, Ordering::Relaxed);
        self.save_session();
        if let Some(parent) = self.profile_dir.parent() {
            let active_file = parent.join("active_profile");
            let _ = std::fs::write(&active_file, &self.current_profile_name);
        }
        if let Some(id) = self.drawer_id.take() {
            self.engine.destroy(id);
        }
    }

    fn spawn_ticker(&mut self) {
        let proxy = self.proxy.clone();
        let running = self.running.clone();
        thread::spawn(move || {
            while running.load(Ordering::Relaxed) {
                thread::sleep(TICK_INTERVAL);
                if proxy.send_event(UserEvent::Tick).is_err() {
                    break;
                }
            }
        });
    }
}

fn history_suggestion(url: &str, title: &str) -> Suggestion {
    Suggestion {
        kind: SuggestionKind::History,
        text: if title.is_empty() {
            url.to_string()
        } else {
            title.to_string()
        },
        detail: Some(url.to_string()),
        url: Some(url.to_string()),
        tab_id: None,
    }
}

fn debug_log(message: &str) {
    if std::env::var_os("PEEK_DEBUG").is_some() {
        eprintln!("peek: {message}");
    }
}

fn random_nonce() -> String {
    use std::collections::hash_map::RandomState;
    use std::hash::{BuildHasher, Hasher};

    let mut out = String::with_capacity(32);
    for _ in 0..2 {
        let mut hasher = RandomState::new().build_hasher();
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0);
        hasher.write_u128(nanos);
        hasher.write_usize(&out as *const _ as usize);
        out.push_str(&format!("{:016x}", hasher.finish()));
    }
    out
}

pub fn derive_profile_uuid(profile_name: &str) -> [u8; 16] {
    let mut hash = [0u8; 16];
    let seed = format!("dev.peek.profile.{}", profile_name);
    for (i, b) in seed.as_bytes().iter().enumerate() {
        hash[i % 16] ^= b.wrapping_mul((i as u8).wrapping_add(1));
    }
    // Set UUID version 4 and variant bits
    hash[6] = (hash[6] & 0x0f) | 0x40;
    hash[8] = (hash[8] & 0x3f) | 0x80;
    hash
}

fn parse_cookie_string(raw: &str, page_url: &str) -> Option<peek_store::StoredCookie> {
    let mut parts = raw.split(';');
    let first = parts.next()?.trim();
    if first.is_empty() {
        return None;
    }
    let (name, value) = match first.find('=') {
        Some(pos) => (&first[..pos], &first[pos + 1..]),
        None => (first, ""),
    };
    if name.is_empty() {
        return None;
    }

    let default_domain = Url::parse(page_url)
        .ok()
        .and_then(|u| u.host_str().map(|h| h.to_string()))
        .unwrap_or_else(|| "localhost".to_string());

    let mut domain = default_domain;
    let mut path = "/".to_string();
    let mut secure = false;
    let http_only = false;
    let mut same_site = None;
    let mut expires_at = None;

    for part in parts {
        let trimmed = part.trim();
        let (k, v) = match trimmed.find('=') {
            Some(pos) => (trimmed[..pos].trim(), trimmed[pos + 1..].trim()),
            None => (trimmed, ""),
        };
        let k_lower = k.to_ascii_lowercase();
        match k_lower.as_str() {
            "domain" => {
                if !v.is_empty() {
                    domain = v.trim_start_matches('.').to_string();
                }
            }
            "path" => {
                if !v.is_empty() {
                    path = v.to_string();
                }
            }
            "secure" => {
                secure = true;
            }
            "samesite" => {
                if !v.is_empty() {
                    same_site = Some(v.to_string());
                }
            }
            "max-age" => {
                if let Ok(secs) = v.parse::<i64>() {
                    expires_at = Some(now_ms() + secs * 1000);
                }
            }
            _ => {}
        }
    }

    Some(peek_store::StoredCookie {
        domain,
        name: name.to_string(),
        path,
        value: value.to_string(),
        expires_at,
        secure,
        http_only,
        same_site,
        created_at: now_ms(),
    })
}

