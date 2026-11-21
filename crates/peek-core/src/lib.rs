//! Pure browser state: tabs, LRU discard and omnibar resolution.
//!
//! This crate never touches `wry` or the filesystem. The app layer executes
//! the [`TabAction`]s returned here, which keeps the discard policy testable.

pub mod tabs;
pub mod url;

pub use tabs::{TabAction, TabConfig, TabManager};
pub use url::{resolve_input, upgrade_to_https, NavigateTarget, SearchEngine};

/// Milliseconds since the Unix epoch.
pub type Timestamp = i64;

pub fn now_ms() -> Timestamp {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as Timestamp)
        .unwrap_or(0)
}
