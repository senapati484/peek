//! Typed IPC messages exchanged between the Peek UI and the Rust host.
//!
//! Every message is wrapped in an [`Envelope`] that carries a per-session
//! nonce. The host rejects any message whose nonce does not match the nonce
//! injected into that webview, which keeps ordinary page content from driving
//! privileged commands (see PLAN section 3.2).

use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};

/// Protocol revision. Bumped whenever a breaking change is made.
pub const PROTOCOL_VERSION: u32 = 1;

/// Hard cap on a single IPC payload. Prevents a compromised page from
/// allocating unbounded memory in the host process.
pub const MAX_MESSAGE_BYTES: usize = 256 * 1024;

pub type TabId = u64;

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum IpcError {
    #[error("ipc payload is {size} bytes, limit is {limit}")]
    TooLarge { size: usize, limit: usize },
    #[error("ipc payload is not valid JSON: {0}")]
    Json(String),
    #[error("ipc protocol version {got} is not supported (expected {expected})")]
    Version { got: u32, expected: u32 },
    #[error("ipc nonce does not match this webview")]
    Nonce,
}

/// A nonce-bearing wrapper around every message.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Envelope<T> {
    #[serde(default = "protocol_version")]
    pub version: u32,
    pub nonce: String,
    pub payload: T,
}

fn protocol_version() -> u32 {
    PROTOCOL_VERSION
}

impl<T> Envelope<T> {
    pub fn new(nonce: impl Into<String>, payload: T) -> Self {
        Self {
            version: PROTOCOL_VERSION,
            nonce: nonce.into(),
            payload,
        }
    }

    pub fn to_json(&self) -> Result<String, IpcError>
    where
        T: Serialize,
    {
        serde_json::to_string(self).map_err(|e| IpcError::Json(e.to_string()))
    }
}

impl<T: DeserializeOwned> Envelope<T> {
    /// Decode and validate an incoming message.
    pub fn decode(raw: &str, expected_nonce: &str) -> Result<T, IpcError> {
        if raw.len() > MAX_MESSAGE_BYTES {
            return Err(IpcError::TooLarge {
                size: raw.len(),
                limit: MAX_MESSAGE_BYTES,
            });
        }
        let envelope: Envelope<T> =
            serde_json::from_str(raw).map_err(|e| IpcError::Json(e.to_string()))?;
        if envelope.version != PROTOCOL_VERSION {
            return Err(IpcError::Version {
                got: envelope.version,
                expected: PROTOCOL_VERSION,
            });
        }
        if envelope.nonce != expected_nonce {
            return Err(IpcError::Nonce);
        }
        Ok(envelope.payload)
    }
}

/// Commands the UI webview (omnibar / drawer) may send to the host.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "camelCase", rename_all_fields = "camelCase")]
pub enum Command {
    /// Submit the omnibar: a URL, a search query, or a tab title.
    Navigate { input: String },
    NewTab { url: Option<String> },
    CloseTab { id: TabId },
    SwitchTab { id: TabId },
    ReorderTab { id: TabId, to_index: usize },
    Back,
    Forward,
    Reload,
    Stop,
    /// Ask for omnibar suggestions for `query`.
    Suggest { query: String },
    DismissSuggestions,
    /// A freshly mounted chrome webview asks for the current state.
    RequestState,
    /// The omnibar expanded (true) or collapsed (false).
    SetOmnibarOpen { open: bool },
    /// Return keyboard focus to the page.
    FocusPage,
    /// Open or close the edge drawer.
    SetDrawer {
        open: bool,
        #[serde(default)]
        side: Option<EdgeSide>,
        #[serde(default)]
        section: Option<String>,
    },
    /// Open a URL in a new tab (drawer / suggestions).
    OpenUrl { url: String },
    /// Per-site ad-block toggle (Phase 2; stored now so the UI is stable).
    SetSiteAdblock { host: String, enabled: bool },
    /// Live settings update (Phase 4).
    SetSetting {
        key: String,
        value: serde_json::Value,
    },
    /// Clear cookies and stored web data for current profile.
    ClearCookies,
    /// Request cookie count.
    GetCookies,
    /// Create a new profile with dedicated storage directory.
    CreateProfile { name: String },
    /// Switch to another profile.
    SwitchProfile { name: String },
    /// Delete a profile.
    DeleteProfile { name: String },
    /// Install an extension from a local directory or .crx file path.
    InstallExtension { path: String },
    /// Enable or disable an installed extension.
    ToggleExtension { id: String, enabled: bool },
    /// Remove an installed extension.
    UninstallExtension { id: String },
    /// Request installed extensions list.
    GetExtensions,
    Quit,
}

/// Events the host pushes to the UI webview.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "camelCase", rename_all_fields = "camelCase")]
pub enum Event {
    TabsChanged {
        tabs: Vec<TabInfo>,
        active: Option<TabId>,
    },
    ActiveTabChanged {
        id: TabId,
    },
    Loading {
        id: TabId,
        url: String,
        loading: bool,
        can_go_back: bool,
        can_go_forward: bool,
    },
    TitleChanged {
        id: TabId,
        title: String,
    },
    UrlChanged {
        id: TabId,
        url: String,
    },
    SuggestionsReady {
        query: String,
        items: Vec<Suggestion>,
    },
    /// The UI should focus the omnibar (Ctrl/Cmd+L or Ctrl/Cmd+T).
    FocusOmnibar,
    DrawerState {
        open: bool,
        #[serde(default)]
        side: Option<EdgeSide>,
        #[serde(default)]
        section: Option<String>,
    },
    AdblockCount {
        blocked: u64,
        allowed_on_site: bool,
        host: Option<String>,
    },
    ProfileChanged {
        name: String,
    },
    ProfilesList {
        profiles: Vec<String>,
        active: String,
    },
    Settings {
        max_live: usize,
        idle_discard_minutes: u64,
        hover_delay_ms: u64,
        drawer_side: EdgeSide,
        edge_zone: f64,
        start_blank: bool,
        search_engine: String,
    },
    Toast {
        message: String,
    },
    CookiesCount {
        count: usize,
    },
    ExtensionsList {
        extensions: Vec<ExtensionSummary>,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExtensionSummary {
    pub id: String,
    pub name: String,
    pub version: String,
    pub description: String,
    pub enabled: bool,
    pub fully_supported: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TabInfo {
    pub id: TabId,
    pub url: String,
    pub title: String,
    pub favicon: Option<String>,
    pub discarded: bool,
    pub loading: bool,
    pub audible: bool,
    pub active: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum SuggestionKind {
    History,
    Tab,
    Bookmark,
    Search,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Suggestion {
    pub kind: SuggestionKind,
    /// Primary line shown in the omnibar.
    pub text: String,
    /// Secondary line (usually the URL).
    pub detail: Option<String>,
    pub url: Option<String>,
    pub tab_id: Option<TabId>,
}

/// Events reported by injected scripts running inside page webviews.
///
/// Page scripts are untrusted. The host treats these as hints only and never
/// uses them to reach the network or the filesystem.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "camelCase", rename_all_fields = "camelCase")]
pub enum PageEvent {
    /// Pointer has dwelled on a link long enough to be hover intent.
    Hover {
        url: String,
        rect: Rect,
        /// `true` when a modifier forced an immediate preview.
        forced: bool,
    },
    /// Pointer left the link / preview target.
    HoverEnd,
    /// The pointer entered the preview webview (cancels pending dismissal).
    PreviewEnter,
    /// The pointer left the preview webview.
    PreviewLeave,
    /// Cursor is near a configured screen edge.
    EdgeProximity { side: EdgeSide, distance: f64 },
    /// A page started or stopped playing audio.
    AudioState { playing: bool },
    /// A form field on the page has unsaved input.
    FormDirty { dirty: bool },
    /// Page scroll position changed.
    Scroll { y: f64 },
    /// The user pressed a navigation mouse button inside the page.
    MouseNav { forward: bool },
    /// A keyboard shortcut that must be handled by the host.
    Shortcut { shortcut: Shortcut },
    /// The page's declared favicon.
    Favicon { href: String },
    /// The preview webview was clicked; promote it to a tab.
    PreviewClick { url: String },
    /// An in-page resource was blocked by the ad blocker.
    BlockedResource { url: String },
    /// A cookie was set from client script or navigation.
    CookieSet { cookie: String, url: String },
    /// Extension content-script bridge message (Phase 5).

    ExtensionMessage {
        extension_id: String,
        payload: serde_json::Value,
    },
}

/// Host-level shortcuts reported by injected page scripts.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Shortcut {
    FocusOmnibar,
    NewTab,
    CloseTab,
    NextTab,
    PrevTab,
    SelectTab(u8),
    Reload,
    Back,
    Forward,
    Escape,
    ToggleDrawer,
    OpenTabsDrawer,
    OpenSettingsDrawer,
    OpenCommandPalette,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum EdgeSide {
    Left,
    Right,
}

/// CSS-pixel rectangle relative to the page viewport.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Rect {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn command_round_trip() {
        let command = Command::ReorderTab { id: 7, to_index: 2 };
        let envelope = Envelope::new("abc123", command.clone());
        let json = envelope.to_json().unwrap();
        let decoded: Command = Envelope::decode(&json, "abc123").unwrap();
        assert_eq!(decoded, command);
    }

    #[test]
    fn event_round_trip_preserves_camel_case_tags() {
        let event = Event::TabsChanged {
            tabs: vec![TabInfo {
                id: 1,
                url: "https://example.com".into(),
                title: "Example".into(),
                favicon: None,
                discarded: false,
                loading: false,
                audible: false,
                active: true,
            }],
            active: Some(1),
        };
        let json = Envelope::new("n", event.clone()).to_json().unwrap();
        assert!(json.contains("\"type\":\"tabsChanged\""));
        let decoded: Event = Envelope::decode(&json, "n").unwrap();
        assert_eq!(decoded, event);
    }

    #[test]
    fn page_event_round_trip() {
        let event = PageEvent::Hover {
            url: "https://example.com".into(),
            rect: Rect {
                x: 1.0,
                y: 2.0,
                width: 3.0,
                height: 4.0,
            },
            forced: false,
        };
        let json = Envelope::new("n", event.clone()).to_json().unwrap();
        let decoded: PageEvent = Envelope::decode(&json, "n").unwrap();
        assert_eq!(decoded, event);
    }

    #[test]
    fn rejects_wrong_nonce() {
        let json = Envelope::new("good", Command::Reload).to_json().unwrap();
        assert_eq!(
            Envelope::<Command>::decode(&json, "bad"),
            Err(IpcError::Nonce)
        );
    }

    #[test]
    fn rejects_oversized_payload() {
        let huge = "x".repeat(MAX_MESSAGE_BYTES + 1);
        assert!(matches!(
            Envelope::<Command>::decode(&huge, "n"),
            Err(IpcError::TooLarge { .. })
        ));
    }

    #[test]
    fn rejects_unknown_version() {
        let json = r#"{"version":99,"nonce":"n","payload":{"type":"reload"}}"#;
        assert_eq!(
            Envelope::<Command>::decode(json, "n"),
            Err(IpcError::Version {
                got: 99,
                expected: PROTOCOL_VERSION
            })
        );
    }

    #[test]
    fn cookie_ipc_round_trip() {
        let cmd = Command::ClearCookies;
        let json = Envelope::new("n", cmd.clone()).to_json().unwrap();
        assert_eq!(Envelope::<Command>::decode(&json, "n").unwrap(), cmd);

        let pe = PageEvent::CookieSet {
            cookie: "foo=bar; Path=/".into(),
            url: "https://example.com/".into(),
        };
        let json_pe = Envelope::new("n", pe.clone()).to_json().unwrap();
        assert_eq!(Envelope::<PageEvent>::decode(&json_pe, "n").unwrap(), pe);

        let ev = Event::CookiesCount { count: 42 };
        let json_ev = Envelope::new("n", ev.clone()).to_json().unwrap();
        assert_eq!(Envelope::<Event>::decode(&json_ev, "n").unwrap(), ev);
    }

    #[test]
    fn missing_version_defaults_to_current() {
        let json = r#"{"nonce":"n","payload":{"type":"reload"}}"#;
        assert_eq!(Envelope::<Command>::decode(json, "n"), Ok(Command::Reload));
    }
}
