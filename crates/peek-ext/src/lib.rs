//! Manifest V3 parsing and compatibility reporting.
//!
//! System webviews cannot load Chrome extensions, so Peek runs a subset of
//! Manifest V3 in a Rust host. This crate owns the part users see first: read
//! `manifest.json` and say honestly which APIs will work.

use serde::{Deserialize, Serialize};

pub mod crx;
pub mod matcher;
pub mod registry;
pub mod shim;

pub use crx::extract_crx_or_zip;
pub use matcher::{matches_any, matches_pattern, MatchPattern};
pub use registry::{ExtensionRegistry, InstalledExtension};
pub use shim::wrap_content_script;

#[derive(Debug, thiserror::Error)]
pub enum ExtError {
    #[error("manifest.json is not valid JSON: {0}")]
    Json(#[from] serde_json::Error),
    #[error("unsupported manifest version {0}; Peek requires Manifest V3")]
    ManifestVersion(i64),
    #[error("manifest has no name")]
    MissingName,
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("zip extraction error: {0}")]
    Zip(String),
    #[error("invalid package: {0}")]
    InvalidPackage(String),
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Manifest {
    pub name: String,
    #[serde(default)]
    pub version: String,
    #[serde(default)]
    pub manifest_version: i64,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub permissions: Vec<String>,
    #[serde(default)]
    pub optional_permissions: Vec<String>,
    #[serde(default)]
    pub host_permissions: Vec<String>,
    #[serde(default)]
    pub background: Option<Background>,
    #[serde(default)]
    pub content_scripts: Vec<ContentScript>,
    #[serde(default)]
    pub action: Option<Action>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Background {
    #[serde(default)]
    pub service_worker: Option<String>,
    #[serde(default)]
    pub scripts: Vec<String>,
    #[serde(default)]
    pub r#type: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ContentScript {
    #[serde(default)]
    pub matches: Vec<String>,
    #[serde(default)]
    pub js: Vec<String>,
    #[serde(default)]
    pub css: Vec<String>,
    #[serde(default)]
    pub run_at: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Action {
    #[serde(default)]
    pub default_popup: Option<String>,
    #[serde(default)]
    pub default_title: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Support {
    /// Works in Phase 5.
    Supported,
    /// Works with limitations, from Phase 6.
    Tier2,
    /// Not possible in a system webview; reported to the user.
    Unsupported,
    /// Unknown to this build.
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ApiSupport {
    pub api: String,
    pub support: Support,
    pub note: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CompatibilityReport {
    pub name: String,
    pub version: String,
    pub manifest_version: i64,
    pub permissions: Vec<ApiSupport>,
    pub runs_background: bool,
    pub content_script_count: usize,
}

impl CompatibilityReport {
    pub fn is_fully_supported(&self) -> bool {
        !self
            .permissions
            .iter()
            .any(|p| p.support == Support::Unsupported)
    }

    pub fn unsupported(&self) -> impl Iterator<Item = &ApiSupport> {
        self.permissions
            .iter()
            .filter(|p| p.support == Support::Unsupported)
    }
}

pub fn parse_manifest(json: &str) -> Result<Manifest, ExtError> {
    let manifest: Manifest = serde_json::from_str(json)?;
    if manifest.manifest_version != 3 {
        return Err(ExtError::ManifestVersion(manifest.manifest_version));
    }
    if manifest.name.trim().is_empty() {
        return Err(ExtError::MissingName);
    }
    Ok(manifest)
}

pub fn compatibility(manifest: &Manifest) -> CompatibilityReport {
    let mut permissions: Vec<ApiSupport> = manifest
        .permissions
        .iter()
        .chain(manifest.optional_permissions.iter())
        .map(|api| classify(api))
        .collect();
    permissions.sort_by(|a, b| a.api.cmp(&b.api));
    permissions.dedup_by(|a, b| a.api == b.api);

    CompatibilityReport {
        name: manifest.name.clone(),
        version: manifest.version.clone(),
        manifest_version: manifest.manifest_version,
        permissions,
        runs_background: manifest
            .background
            .as_ref()
            .map(|b| b.service_worker.is_some() || !b.scripts.is_empty())
            .unwrap_or(false),
        content_script_count: manifest.content_scripts.len(),
    }
}

fn classify(raw: &str) -> ApiSupport {
    let api = raw
        .split_once("://")
        .map(|(head, _)| head)
        .unwrap_or(raw)
        .trim();
    let (support, note) = match api {
        "storage" => (Support::Supported, "local and sync (sync stored locally)"),
        "runtime" => (Support::Supported, "messaging, getURL, getManifest, id"),
        "i18n" => (Support::Supported, "message lookup"),
        "action" => (Support::Supported, "popup and badge"),
        "alarms" => (Support::Supported, "wake-up timers"),
        "scripting" => (Support::Supported, "executeScript, insertCSS"),
        "tabs" => (Support::Supported, "basic tab queries and events"),
        "contextMenus" => (Support::Supported, "basic menus"),
        "commands" => (Support::Supported, "keyboard commands"),
        "declarativeNetRequest" | "declarativeNetRequestWithHostAccess" => (
            Support::Supported,
            "static rulesets translated to the built-in blocker",
        ),
        "webNavigation" => (Support::Supported, "basic navigation events"),
        "cookies" => (Support::Tier2, "Phase 6"),
        "history" => (Support::Tier2, "Phase 6"),
        "bookmarks" => (Support::Tier2, "Phase 6"),
        "downloads" => (Support::Tier2, "Phase 6"),
        "notifications" => (Support::Tier2, "Phase 6"),
        "webRequest" => (
            Support::Unsupported,
            "blocking webRequest is not possible in a system webview",
        ),
        "devtools" => (Support::Unsupported, "no DevTools extension API"),
        "nativeMessaging" => (Support::Unsupported, "native messaging is disabled"),
        "debugger" => (Support::Unsupported, "no debugger API"),
        "proxy" => (Support::Unsupported, "proxy control is disabled"),
        "tabCapture" => (Support::Unsupported, "tab capture is not available"),
        _ => (Support::Unknown, "not implemented by this build"),
    };
    ApiSupport {
        api: api.to_string(),
        support,
        note: note.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const MANIFEST: &str = r#"{
        "manifest_version": 3,
        "name": "Test Extension",
        "version": "1.2.3",
        "permissions": ["storage", "tabs", "webRequest", "cookies"],
        "host_permissions": ["https://example.com/*"],
        "background": { "service_worker": "sw.js" },
        "content_scripts": [
            { "matches": ["https://example.com/*"], "js": ["cs.js"], "run_at": "document_start" }
        ],
        "action": { "default_popup": "popup.html" }
    }"#;

    #[test]
    fn parses_mv3_manifest() {
        let manifest = parse_manifest(MANIFEST).unwrap();
        assert_eq!(manifest.name, "Test Extension");
        assert_eq!(manifest.content_scripts.len(), 1);
        assert_eq!(
            manifest.background.unwrap().service_worker.as_deref(),
            Some("sw.js")
        );
    }

    #[test]
    fn rejects_mv2_and_missing_name() {
        assert!(matches!(
            parse_manifest(r#"{"manifest_version":2,"name":"x"}"#),
            Err(ExtError::ManifestVersion(2))
        ));
        assert!(matches!(
            parse_manifest(r#"{"manifest_version":3,"name":"  "}"#),
            Err(ExtError::MissingName)
        ));
    }

    #[test]
    fn builds_compatibility_report() {
        let manifest = parse_manifest(MANIFEST).unwrap();
        let report = compatibility(&manifest);
        assert_eq!(report.content_script_count, 1);
        assert!(report.runs_background);

        let storage = report.permissions.iter().find(|p| p.api == "storage").unwrap();
        assert_eq!(storage.support, Support::Supported);
        let cookies = report.permissions.iter().find(|p| p.api == "cookies").unwrap();
        assert_eq!(cookies.support, Support::Tier2);
        let webrequest = report
            .permissions
            .iter()
            .find(|p| p.api == "webRequest")
            .unwrap();
        assert_eq!(webrequest.support, Support::Unsupported);

        assert!(!report.is_fully_supported());
        assert_eq!(report.unsupported().count(), 1);
    }

    #[test]
    fn host_pattern_permissions_are_normalized() {
        let manifest = parse_manifest(
            r#"{"manifest_version":3,"name":"x","permissions":["https://a.test/*"]}"#,
        )
        .unwrap();
        let report = compatibility(&manifest);
        assert_eq!(report.permissions[0].api, "https");
        assert_eq!(report.permissions[0].support, Support::Unknown);
    }
}
