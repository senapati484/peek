//! Chrome extension match pattern implementation.
//!
//! Conforms to Chrome Manifest V3 match pattern specifications:
//! `<scheme>://<host>/<path>` or `<all_urls>`.

use url::Url;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MatchPattern {
    pub raw: String,
    pub scheme: String,
    pub host: String,
    pub path_prefix: String,
    pub match_all_paths: bool,
}

impl MatchPattern {
    pub fn parse(pattern: &str) -> Option<Self> {
        let pattern = pattern.trim();
        if pattern == "<all_urls>" {
            return Some(Self {
                raw: pattern.to_string(),
                scheme: "*".to_string(),
                host: "*".to_string(),
                path_prefix: "/".to_string(),
                match_all_paths: true,
            });
        }

        let (scheme, rest) = pattern.split_once("://")?;
        let (host, path) = match rest.split_once('/') {
            Some((h, p)) => (h, format!("/{}", p)),
            None => (rest, "/*".to_string()),
        };

        let match_all_paths = path == "/*" || path == "*";
        let path_prefix = if match_all_paths {
            "/".to_string()
        } else if let Some(stripped) = path.strip_suffix('*') {
            stripped.to_string()
        } else {
            path
        };

        Some(Self {
            raw: pattern.to_string(),
            scheme: scheme.to_lowercase(),
            host: host.to_lowercase(),
            path_prefix,
            match_all_paths,
        })
    }

    pub fn matches(&self, target_url: &str) -> bool {
        let Ok(parsed) = Url::parse(target_url) else {
            return false;
        };

        // 1. Check scheme
        let url_scheme = parsed.scheme();
        if self.scheme == "*" {
            if url_scheme != "http" && url_scheme != "https" {
                return false;
            }
        } else if self.scheme != url_scheme {
            return false;
        }

        // 2. Check host
        if self.host != "*" {
            let Some(url_host) = parsed.host_str() else {
                return false;
            };
            let url_host = url_host.to_lowercase();

            if let Some(suffix) = self.host.strip_prefix("*.") {
                // "*.example.com" matches "example.com" and "sub.example.com"
                if url_host != suffix && !url_host.ends_with(&format!(".{suffix}")) {
                    return false;
                }
            } else if self.host != url_host {
                return false;
            }
        }

        // 3. Check path
        if self.match_all_paths {
            return true;
        }

        let url_path = parsed.path();
        if self.path_prefix.ends_with('/') {
            url_path.starts_with(&self.path_prefix)
        } else {
            url_path == self.path_prefix || url_path.starts_with(&format!("{}/", self.path_prefix))
        }
    }
}

pub fn matches_pattern(pattern: &str, url: &str) -> bool {
    MatchPattern::parse(pattern)
        .map(|p| p.matches(url))
        .unwrap_or(false)
}

pub fn matches_any(patterns: &[String], url: &str) -> bool {
    patterns.iter().any(|pat| matches_pattern(pat, url))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn matches_all_urls() {
        assert!(matches_pattern("<all_urls>", "https://google.com/search"));
        assert!(matches_pattern("<all_urls>", "http://example.com/"));
        assert!(!matches_pattern("<all_urls>", "chrome://settings"));
    }

    #[test]
    fn matches_wildcard_scheme_and_host() {
        assert!(matches_pattern("*://*/*", "https://news.ycombinator.com/"));
        assert!(matches_pattern("*://*/*", "http://localhost:8080/test"));
        assert!(!matches_pattern("*://*/*", "ftp://example.com/"));
    }

    #[test]
    fn matches_subdomain_wildcard() {
        assert!(matches_pattern(
            "*://*.example.com/*",
            "https://sub.example.com/path"
        ));
        assert!(matches_pattern(
            "*://*.example.com/*",
            "http://example.com/path"
        ));
        assert!(!matches_pattern(
            "*://*.example.com/*",
            "https://notexample.com/path"
        ));
    }

    #[test]
    fn matches_path_prefix() {
        assert!(matches_pattern(
            "https://github.com/rust-lang/*",
            "https://github.com/rust-lang/rust"
        ));
        assert!(!matches_pattern(
            "https://github.com/rust-lang/*",
            "https://github.com/torvalds/linux"
        ));
    }
}
