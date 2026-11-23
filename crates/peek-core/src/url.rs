//! Omnibar input resolution: URL vs. search vs. switch-to-tab.

use serde::{Deserialize, Serialize};

use crate::tabs::Tab;

/// A configurable search engine. `{query}` is replaced with the
/// percent-encoded query.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SearchEngine {
    pub name: String,
    pub template: String,
}

impl Default for SearchEngine {
    fn default() -> Self {
        Self::google()
    }
}

impl SearchEngine {
    pub fn google() -> Self {
        Self {
            name: "Google".into(),
            template: "https://www.google.com/search?q={query}".into(),
        }
    }

    pub fn duckduckgo() -> Self {
        Self {
            name: "DuckDuckGo".into(),
            template: "https://duckduckgo.com/?q={query}".into(),
        }
    }


    pub fn search_url(&self, query: &str) -> String {
        let encoded: String = url::form_urlencoded::byte_serialize(query.as_bytes()).collect();
        self.template.replace("{query}", &encoded)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NavigateTarget {
    /// Load this URL directly.
    Url(String),
    /// Run a search with the configured engine.
    Search(String),
    /// Focus an already-open tab (the omnibar doubles as a tab switcher).
    SwitchToTab(u64),
}

/// Decide what the omnibar should do with `input`.
pub fn resolve_input(input: &str, tabs: &[Tab], https_first: bool) -> NavigateTarget {
    let input = input.trim();
    if input.is_empty() {
        return NavigateTarget::Url(blank_page().into());
    }

    // 1. An exact open-tab title match switches tabs.
    if let Some(tab) = tabs
        .iter()
        .find(|t| !t.title.is_empty() && t.title.eq_ignore_ascii_case(input))
    {
        return NavigateTarget::SwitchToTab(tab.id);
    }

    // 2. Explicit scheme.
    if let Some(scheme) = explicit_scheme(input) {
        let url = if https_first && scheme == "http" {
            upgrade_to_https(input).unwrap_or_else(|| input.to_string())
        } else {
            input.to_string()
        };
        return NavigateTarget::Url(url);
    }

    // 3. Looks like a host: example.com, localhost:3000, 192.168.0.1/path.
    //    Bare hosts default to HTTPS; an explicit http:// scheme is handled above.
    if looks_like_host(input) {
        return NavigateTarget::Url(format!("https://{input}"));
    }

    // 4. Otherwise search.
    NavigateTarget::Search(input.to_string())
}

/// `http://x` -> `https://x`. Returns `None` when the URL is not http.
pub fn upgrade_to_https(url: &str) -> Option<String> {
    let parsed = url::Url::parse(url).ok()?;
    if parsed.scheme() != "http" {
        return None;
    }
    let mut upgraded = parsed;
    upgraded.set_scheme("https").ok()?;
    Some(upgraded.into())
}

pub fn blank_page() -> &'static str {
    "peek://localhost/blank.html"
}

/// Strip known tracking query parameters (e.g. `utm_*`, `fbclid`, `gclid`, `si`, `igshid`, `ref`).
pub fn strip_tracking_params(raw_url: &str) -> String {
    if let Ok(mut parsed) = url::Url::parse(raw_url) {
        let tracking_prefixes = [
            "utm_", "fbclid", "gclid", "gclsrc", "dclid", "si", "igshid", "mc_", "yclid", "_hs", "ref_",
        ];
        let exact_keys = ["ref", "s", "t", "feature"];

        let filtered: Vec<(String, String)> = parsed
            .query_pairs()
            .filter(|(k, _)| {
                let lower = k.to_lowercase();
                !exact_keys.contains(&lower.as_str())
                    && !tracking_prefixes.iter().any(|prefix| lower.starts_with(prefix))
            })
            .map(|(k, v)| (k.into_owned(), v.into_owned()))
            .collect();

        if filtered.is_empty() {
            parsed.set_query(None);
        } else {
            let mut serializer = url::form_urlencoded::Serializer::new(String::new());
            for (k, v) in filtered {
                serializer.append_pair(&k, &v);
            }
            parsed.set_query(Some(&serializer.finish()));
        }
        parsed.to_string()
    } else {
        raw_url.to_string()
    }
}

pub fn is_blank(url: &str) -> bool {
    let u = url.trim();
    u.is_empty() || u == "about:blank" || u == blank_page() || u.ends_with("/blank.html")
}

fn explicit_scheme(input: &str) -> Option<String> {
    let (scheme, _) = input.split_once(':')?;
    if scheme.len() < 2 || scheme.len() > 32 {
        return None;
    }
    if !scheme
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || c == '+' || c == '-' || c == '.')
    {
        return None;
    }
    if !scheme.chars().next()?.is_ascii_alphabetic() {
        return None;
    }
    // Avoid treating "localhost:3000" as scheme "localhost".
    if !input[scheme.len() + 1..].starts_with("//")
        && !matches!(scheme, "about" | "file" | "data" | "view-source" | "mailto")
    {
        return None;
    }
    Some(scheme.to_ascii_lowercase())
}

fn looks_like_host(input: &str) -> bool {
    if input.chars().any(|c| c.is_whitespace()) {
        return false;
    }
    let authority = input.split(['/', '?', '#']).next().unwrap_or(input);
    let host = authority.rsplit('@').next().unwrap_or(authority);
    let host = host.split(':').next().unwrap_or(host);

    if host.eq_ignore_ascii_case("localhost") {
        return true;
    }
    if host.parse::<std::net::IpAddr>().is_ok() {
        return true;
    }
    // A dotted name whose last label is at least two alphabetic characters.
    match host.rsplit_once('.') {
        Some((_, tld)) => {
            tld.len() >= 2 && tld.chars().all(|c| c.is_ascii_alphabetic())
        }
        None => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tabs::{Tab, TabConfig, TabManager};

    fn tab(id: u64, title: &str) -> Tab {
        Tab {
            id,
            url: format!("https://{id}.test"),
            title: title.into(),
            favicon: None,
            scroll_y: 0.0,
            last_active: 0,
            loading: false,
            audible: false,
            form_dirty: false,
            pinned: false,
            group_id: None,
            live: true,
        }
    }

    #[test]
    fn url_with_scheme_is_loaded() {
        assert_eq!(
            resolve_input("https://example.com/a", &[], true),
            NavigateTarget::Url("https://example.com/a".into())
        );
    }

    #[test]
    fn http_is_upgraded_when_https_first() {
        assert_eq!(
            resolve_input("http://example.com/a?b=1", &[], true),
            NavigateTarget::Url("https://example.com/a?b=1".into())
        );
        assert_eq!(
            resolve_input("http://example.com", &[], false),
            NavigateTarget::Url("http://example.com".into())
        );
    }

    #[test]
    fn bare_host_gets_https() {
        assert_eq!(
            resolve_input("example.com/path", &[], true),
            NavigateTarget::Url("https://example.com/path".into())
        );
        assert_eq!(
            resolve_input("localhost:3000", &[], true),
            NavigateTarget::Url("https://localhost:3000".into())
        );
        assert_eq!(
            resolve_input("192.168.1.4", &[], true),
            NavigateTarget::Url("https://192.168.1.4".into())
        );
    }

    #[test]
    fn free_text_becomes_a_search() {
        let engine = SearchEngine::default();
        assert_eq!(
            resolve_input("rust borrow checker", &[], true),
            NavigateTarget::Search("rust borrow checker".into())
        );
        assert_eq!(
            engine.search_url("a b&c"),
            "https://www.google.com/search?q=a+b%26c"
        );
    }

    #[test]
    fn tab_title_switches_tabs() {
        let tabs = vec![tab(1, "Hacker News"), tab(2, "Rust")];
        assert_eq!(
            resolve_input("hacker news", &tabs, true),
            NavigateTarget::SwitchToTab(1)
        );
    }

    #[test]
    fn empty_input_is_blank_page() {
        assert_eq!(
            resolve_input("   ", &[], true),
            NavigateTarget::Url(blank_page().into())
        );
    }

    #[test]
    fn special_schemes_are_kept() {
        for input in ["about:blank", "file:///tmp/x.html", "view-source:https://a"] {
            match resolve_input(input, &[], true) {
                NavigateTarget::Url(u) => assert_eq!(u, input),
                other => panic!("{input} resolved to {other:?}"),
            }
        }
    }

    #[test]
    fn manager_snapshot_exposes_titles_for_switching() {
        let mut manager = TabManager::new(TabConfig::default());
        manager.create_tab("https://a.test");
        manager.set_title(1, "Alpha");
        assert_eq!(
            resolve_input("alpha", &manager.tabs(), true),
            NavigateTarget::SwitchToTab(1)
        );
    }

    #[test]
    fn test_strip_tracking_params() {
        let dirty = "https://youtube.com/watch?v=123&si=trackingId&utm_source=twitter";
        let clean = strip_tracking_params(dirty);
        assert_eq!(clean, "https://youtube.com/watch?v=123");

        let clean_already = "https://example.com/page?id=42";
        assert_eq!(strip_tracking_params(clean_already), clean_already);
    }
}
