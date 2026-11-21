//! Ad blocking configuration, engine and per-site policy for Peek.
//!
//! Wraps Brave's `adblock` crate for native network blocking, cosmetic
//! filtering, and WebKit content-blocking rule compilation.

pub mod engine;

pub use engine::{AdblockEngine, AdblockError};
use serde::{Deserialize, Serialize};

pub const DEFAULT_LISTS: &[&str] = &[
    "https://easylist.to/easylist/easylist.txt",
    "https://easylist.to/easylist/easyprivacy.txt",
    "https://raw.githubusercontent.com/uBlockOrigin/uAssets/master/filters/filters.txt",
];

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdblockConfig {
    pub enabled: bool,
    /// Rule-list URLs, refreshed daily by the updater.
    pub lists: Vec<String>,
    /// Sites where blocking is disabled (`host` only, no scheme).
    pub allowlist: Vec<String>,
    /// Cosmetic filtering (hiding leftover placeholders) on by default.
    pub cosmetic: bool,
}

impl Default for AdblockConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            lists: DEFAULT_LISTS.iter().map(|s| s.to_string()).collect(),
            allowlist: Vec::new(),
            cosmetic: true,
        }
    }
}

impl AdblockConfig {
    /// Blocking is active for `host` when enabled globally and not allowlisted.
    pub fn active_for(&self, host: &str) -> bool {
        self.enabled && !self.allowlist.iter().any(|h| h.eq_ignore_ascii_case(host))
    }

    pub fn allow(&mut self, host: &str) {
        if !self.allowlist.iter().any(|h| h.eq_ignore_ascii_case(host)) {
            self.allowlist.push(host.to_ascii_lowercase());
        }
    }

    pub fn disallow(&mut self, host: &str) {
        self.allowlist.retain(|h| !h.eq_ignore_ascii_case(host));
    }
}

/// Counters for one tab, surfaced in the drawer.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct BlockCounters {
    pub blocked: u64,
}

impl BlockCounters {
    pub fn record(&mut self, n: u64) {
        self.blocked = self.blocked.saturating_add(n);
    }

    pub fn reset(&mut self) {
        self.blocked = 0;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn allowlist_is_case_insensitive_and_idempotent() {
        let mut config = AdblockConfig::default();
        assert!(config.active_for("Example.COM"));
        config.allow("Example.COM");
        config.allow("example.com");
        assert_eq!(config.allowlist, vec!["example.com"]);
        assert!(!config.active_for("example.com"));
        assert!(config.active_for("other.com"));
        config.disallow("example.com");
        assert!(config.active_for("example.com"));
    }

    #[test]
    fn global_disable_wins() {
        let mut config = AdblockConfig::default();
        config.enabled = false;
        assert!(!config.active_for("example.com"));
    }

    #[test]
    fn counters_saturate() {
        let mut counters = BlockCounters::default();
        counters.record(5);
        counters.record(u64::MAX);
        assert_eq!(counters.blocked, u64::MAX);
        counters.reset();
        assert_eq!(counters.blocked, 0);
    }
}
