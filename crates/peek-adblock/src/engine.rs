//! Native Adblock engine implementation using Brave's `adblock` crate.
//!
//! Provides network blocking, WebKit content-blocking rule compilation,
//! cosmetic CSS generation, and serialized filter caching.

use adblock::content_blocking::{CbRule, CbRuleEquivalent};
use adblock::lists::{parse_filters, FilterSet, ParseOptions};
use adblock::request::Request;
use adblock::Engine;


#[derive(Debug, thiserror::Error)]
pub enum AdblockError {
    #[error("engine deserialization error")]
    Deserialize,
    #[error("invalid request parameters: {0}")]
    Request(String),
    #[error("json serialization error: {0}")]
    Json(#[from] serde_json::Error),
}

/// A wrapper around Brave's `adblock::Engine` with helpers for browser embedding.
pub struct AdblockEngine {
    inner: Engine,
}

impl AdblockEngine {
    /// Create a new engine from a raw rule-list string (uBlock/EasyList syntax).
    pub fn new(rules_text: &str) -> Self {
        let inner = Engine::new_with_list_text(rules_text);
        Self { inner }
    }

    /// Create an engine from a collection of rule lines.
    pub fn from_rules<I, S>(rules: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: AsRef<str>,
    {
        let mut filter_set = FilterSet::new(true);
        for rule in rules {
            filter_set.add_filter_list(rule.as_ref().to_string(), ParseOptions::default());
        }
        let inner = Engine::new_with_filter_set(filter_set);
        Self { inner }
    }

    /// Serialize the engine's compiled state to bytes for fast startup caching.
    pub fn serialize(&self) -> Vec<u8> {
        self.inner.serialize()
    }

    /// Restore the engine from serialized bytes.
    pub fn from_serialized(bytes: &[u8]) -> Result<Self, AdblockError> {
        let mut inner = Engine::default();
        inner.deserialize(bytes).map_err(|_| AdblockError::Deserialize)?;
        Ok(Self { inner })
    }

    /// Check if a network request should be blocked.
    ///
    /// Returns `true` if the request matches a blocking filter and is not exceptioned.
    pub fn should_block(&self, url: &str, source_url: &str, request_type: &str, method: &str) -> bool {
        let Ok(req) = Request::new(url, source_url, request_type, method) else {
            return false;
        };
        let result = self.inner.check_network_request(&req);
        result.filter.is_some() && result.exception.is_none()
    }

    /// Generate cosmetic CSS and injected scripts for `url`.
    ///
    /// Elements matching the generated CSS should be hidden (`display: none !important`).
    pub fn cosmetic_resources(&self, url: &str) -> (Option<String>, Option<String>) {
        let resources = self.inner.url_cosmetic_resources(url);
        let css = if !resources.hide_selectors.is_empty() {
            let mut selectors: Vec<_> = resources.hide_selectors.into_iter().collect();
            selectors.sort();
            let rules: Vec<String> = selectors
                .into_iter()
                .map(|s| format!("{s} {{ display: none !important; }}"))
                .collect();
            Some(rules.join("\n"))
        } else {
            None
        };

        let script = if !resources.injected_script.is_empty() {
            Some(resources.injected_script)
        } else {
            None
        };

        (css, script)
    }

    /// Convert rule text into WebKit's native JSON content-blocking format.
    ///
    /// Supported on macOS (WKWebView) and Linux (WebKitGTK).
    pub fn to_webkit_content_blocking(rules_text: &str) -> Result<String, AdblockError> {
        let lines: Vec<&str> = rules_text.lines().collect();
        let (network_filters, cosmetic_filters) = parse_filters(lines, true, ParseOptions::default());
        let mut cb_rules = Vec::new();

        for network_filter in network_filters {
            if let Ok(equiv) = CbRuleEquivalent::try_from(network_filter) {
                for rule in equiv {
                    cb_rules.push(rule);
                }
            }
        }

        for cosmetic_filter in cosmetic_filters {
            if let Ok(rule) = CbRule::try_from(cosmetic_filter) {
                cb_rules.push(rule);
            }
        }


        let json = serde_json::to_string(&cb_rules)?;
        Ok(json)
    }

}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn blocks_matching_network_request() {
        let rules = "||doubleclick.net^$third-party\n@@||doubleclick.net/allowed.js";
        let engine = AdblockEngine::new(rules);

        // Blocked
        assert!(engine.should_block(
            "https://doubleclick.net/ad.js",
            "https://example.com",
            "script",
            "GET"
        ));

        // Exceptioned
        assert!(!engine.should_block(
            "https://doubleclick.net/allowed.js",
            "https://example.com",
            "script",
            "GET"
        ));

        // Different domain
        assert!(!engine.should_block(
            "https://example.com/main.js",
            "https://example.com",
            "script",
            "GET"
        ));
    }

    #[test]
    fn serialization_roundtrip() {
        let rules = "||ads.example.com^";
        let engine = AdblockEngine::new(rules);
        let bytes = engine.serialize();
        assert!(!bytes.is_empty());

        let restored = AdblockEngine::from_serialized(&bytes).unwrap();
        assert!(restored.should_block(
            "https://ads.example.com/tracker.js",
            "https://other.com",
            "script",
            "GET"
        ));
    }

    #[test]
    fn cosmetic_selectors_generated() {
        let rules = "example.com##.ad-banner\nexample.com##div#sponsored-post";
        let engine = AdblockEngine::new(rules);

        let (css, _) = engine.cosmetic_resources("https://example.com/article");
        assert!(css.is_some());
        let css = css.unwrap();
        assert!(css.contains(".ad-banner { display: none !important; }"));
        assert!(css.contains("div#sponsored-post { display: none !important; }"));
    }

    #[test]
    fn compiles_webkit_content_blocking_json() {
        let rules = "||badtracker.com^$third-party";
        let json = AdblockEngine::to_webkit_content_blocking(rules).unwrap();
        assert!(json.contains("badtracker"));
        assert!(json.contains("\"type\":\"block\""));
        assert!(json.contains("third-party"));
    }


}
