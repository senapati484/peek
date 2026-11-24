//! Extension registry and lifecycle manager.
//!
//! Loads extensions from the profile's `extensions/` directory,
//! installs new unpacked or `.crx` packages, and matches content scripts against visited URLs.

use std::fs;
use std::path::{Path, PathBuf};
use serde::{Deserialize, Serialize};

use crate::crx::{copy_unpacked_dir, extract_crx_or_zip};
use crate::matcher::matches_any;
use crate::shim::wrap_content_script;
use crate::{compatibility, parse_manifest, CompatibilityReport, ExtError, Manifest};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InstalledExtension {
    pub id: String,
    pub name: String,
    pub version: String,
    pub description: String,
    pub enabled: bool,
    pub dir: PathBuf,
    pub manifest: Manifest,
    pub report: CompatibilityReport,
}

#[derive(Debug, Default, Clone)]
pub struct ExtensionRegistry {
    pub extensions: Vec<InstalledExtension>,
}

impl ExtensionRegistry {
    /// Load all installed extensions from `extensions_dir`.
    pub fn load_from_dir(extensions_dir: &Path) -> Self {
        let mut extensions = Vec::new();
        if !extensions_dir.exists() {
            let _ = fs::create_dir_all(extensions_dir);
            return Self { extensions };
        }

        if let Ok(entries) = fs::read_dir(extensions_dir) {
            for entry in entries.flatten() {
                if let Ok(file_type) = entry.file_type() {
                    if file_type.is_dir() {
                        let dir = entry.path();
                        let manifest_path = dir.join("manifest.json");
                        if manifest_path.exists() {
                            if let Ok(content) = fs::read_to_string(&manifest_path) {
                                if let Ok(manifest) = parse_manifest(&content) {
                                    let id = entry.file_name().to_string_lossy().to_string();
                                    let report = compatibility(&manifest);
                                    let description = manifest.description.clone().unwrap_or_default();
                                    extensions.push(InstalledExtension {
                                        id,
                                        name: manifest.name.clone(),
                                        version: manifest.version.clone(),
                                        description,
                                        enabled: true,
                                        dir,
                                        manifest,
                                        report,
                                    });
                                }
                            }
                        }
                    }
                }
            }
        }

        Self { extensions }
    }

    /// Install an extension from an unpacked directory.
    pub fn install_from_folder(
        src_dir: &Path,
        extensions_dir: &Path,
    ) -> Result<InstalledExtension, ExtError> {
        let manifest_path = src_dir.join("manifest.json");
        if !manifest_path.exists() {
            return Err(ExtError::InvalidPackage(
                "manifest.json not found in directory".into(),
            ));
        }
        let manifest_content = fs::read_to_string(&manifest_path)?;
        let manifest = parse_manifest(&manifest_content)?;
        let report = compatibility(&manifest);

        let id = generate_extension_id(&manifest.name, &manifest.version);
        let target_dir = extensions_dir.join(&id);

        copy_unpacked_dir(src_dir, &target_dir)?;

        let description = manifest.description.clone().unwrap_or_default();
        Ok(InstalledExtension {
            id,
            name: manifest.name.clone(),
            version: manifest.version.clone(),
            description,
            enabled: true,
            dir: target_dir,
            manifest,
            report,
        })
    }

    /// Install an extension from a `.crx` or `.zip` file buffer.
    pub fn install_from_bytes(
        data: &[u8],
        extensions_dir: &Path,
    ) -> Result<InstalledExtension, ExtError> {
        let temp_dir = extensions_dir.join(".tmp_install");
        let _ = fs::remove_dir_all(&temp_dir);

        extract_crx_or_zip(data, &temp_dir)?;

        let manifest_path = temp_dir.join("manifest.json");
        if !manifest_path.exists() {
            let _ = fs::remove_dir_all(&temp_dir);
            return Err(ExtError::InvalidPackage(
                "package missing manifest.json".into(),
            ));
        }

        let manifest_content = fs::read_to_string(&manifest_path)?;
        let manifest = match parse_manifest(&manifest_content) {
            Ok(m) => m,
            Err(e) => {
                let _ = fs::remove_dir_all(&temp_dir);
                return Err(e);
            }
        };

        let id = generate_extension_id(&manifest.name, &manifest.version);
        let target_dir = extensions_dir.join(&id);
        let _ = fs::remove_dir_all(&target_dir);

        if let Err(_e) = fs::rename(&temp_dir, &target_dir) {
            // Rename might fail cross-device; fallback to copy
            copy_unpacked_dir(&temp_dir, &target_dir)?;
            let _ = fs::remove_dir_all(&temp_dir);
        }

        let report = compatibility(&manifest);
        let description = manifest.description.clone().unwrap_or_default();
        Ok(InstalledExtension {
            id,
            name: manifest.name.clone(),
            version: manifest.version.clone(),
            description,
            enabled: true,
            dir: target_dir,
            manifest,
            report,
        })
    }

    /// Retrieve all matching content script wrapped codes for a given page URL.
    pub fn matching_content_scripts(&self, url: &str, nonce: &str) -> Vec<String> {
        let mut scripts = Vec::new();
        for ext in &self.extensions {
            if !ext.enabled {
                continue;
            }
            for cs in &ext.manifest.content_scripts {
                if matches_any(&cs.matches, url) {
                    for js_file in &cs.js {
                        let script_path = ext.dir.join(js_file);
                        if let Ok(content) = fs::read_to_string(&script_path) {
                            let wrapped = wrap_content_script(&ext.id, &content, nonce);
                            scripts.push(wrapped);
                        }
                    }
                    for css_file in &cs.css {
                        let css_path = ext.dir.join(css_file);
                        if let Ok(css_content) = fs::read_to_string(&css_path) {
                            let escaped = css_content.replace('\\', "\\\\").replace('`', "\\`").replace('$', "\\$");
                            let inject_css = format!(
                                r#"(function() {{
                                    try {{
                                        var style = document.createElement("style");
                                        style.setAttribute("data-peek-ext", "{}");
                                        style.textContent = `{}`;
                                        (document.head || document.documentElement).appendChild(style);
                                    }} catch(e) {{}}
                                }})();"#,
                                ext.id, escaped
                            );
                            scripts.push(inject_css);
                        }
                    }
                }
            }
        }
        scripts
    }
}

/// Compute deterministic extension ID from name and version.
fn generate_extension_id(name: &str, version: &str) -> String {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};

    let mut hasher = DefaultHasher::new();
    name.hash(&mut hasher);
    version.hash(&mut hasher);
    let h1 = hasher.finish();

    hasher.write_u64(0x9e3779b97f4a7c15);
    let h2 = hasher.finish();

    let full_hex = format!("{:016x}{:016x}", h1, h2);
    // Convert hex characters 0..9, a..f into standard Chrome extension a..p alphabet
    full_hex
        .chars()
        .map(|c| {
            if c.is_ascii_digit() {
                ((c as u8 - b'0') + b'a') as char
            } else if c >= 'a' && c <= 'f' {
                ((c as u8 - b'a' + 10) + b'a') as char
            } else {
                'a'
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn installs_and_matches_content_scripts() {
        let tmp = tempdir().unwrap();
        let ext_dir = tmp.path().join("source_ext");
        fs::create_dir_all(&ext_dir).unwrap();

        fs::write(
            ext_dir.join("manifest.json"),
            r#"{
                "manifest_version": 3,
                "name": "Greeter",
                "version": "1.0",
                "content_scripts": [
                    { "matches": ["https://example.com/*"], "js": ["greet.js"] }
                ]
            }"#,
        )
        .unwrap();

        fs::write(ext_dir.join("greet.js"), "document.title = 'Hello';").unwrap();

        let registry_dir = tmp.path().join("registry");
        let installed = ExtensionRegistry::install_from_folder(&ext_dir, &registry_dir).unwrap();

        assert_eq!(installed.name, "Greeter");
        assert_eq!(installed.id.len(), 32);

        let registry = ExtensionRegistry::load_from_dir(&registry_dir);
        assert_eq!(registry.extensions.len(), 1);

        let scripts = registry.matching_content_scripts("https://example.com/test", "nonce123");
        assert_eq!(scripts.len(), 1);
        assert!(scripts[0].contains("document.title = 'Hello';"));
        assert!(scripts[0].contains("nonce123"));

        let none = registry.matching_content_scripts("https://other.com/", "nonce123");
        assert!(none.is_empty());
    }
}
