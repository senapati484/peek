//! Native UserScript / Tampermonkey engine for Peek.
//!
//! Parses standard `// ==UserScript==` header blocks, matches target URLs,
//! and wraps scripts in an isolated execution sandbox with `GM_*` shims.

use std::path::{Path, PathBuf};
use serde::{Deserialize, Serialize};

use crate::matcher::matches_pattern;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct UserScript {
    pub id: String,
    pub name: String,
    pub version: String,
    pub description: String,
    pub author: String,
    pub match_patterns: Vec<String>,
    pub include_patterns: Vec<String>,
    pub exclude_patterns: Vec<String>,
    pub run_at: String,
    pub grants: Vec<String>,
    pub enabled: bool,
    pub code: String,
    #[serde(skip)]
    pub file_path: Option<PathBuf>,
}

impl UserScript {
    /// Parse a `.user.js` script from source code.
    pub fn parse(source: &str, file_path: Option<PathBuf>) -> Result<Self, String> {
        let mut name = String::new();
        let mut version = "1.0".to_string();
        let mut description = String::new();
        let mut author = String::new();
        let mut match_patterns = Vec::new();
        let mut include_patterns = Vec::new();
        let mut exclude_patterns = Vec::new();
        let mut run_at = "document-end".to_string();
        let mut grants = Vec::new();

        let mut in_header = false;

        for line in source.lines() {
            let trimmed = line.trim();
            if trimmed.contains("==UserScript==") {
                in_header = true;
                continue;
            }
            if trimmed.contains("==/UserScript==") {
                break;
            }
            if in_header {
                if let Some(rest) = trimmed.strip_prefix("//") {
                    let directive = rest.trim();
                    if let Some(val) = directive.strip_prefix("@name") {
                        name = val.trim().to_string();
                    } else if let Some(val) = directive.strip_prefix("@version") {
                        version = val.trim().to_string();
                    } else if let Some(val) = directive.strip_prefix("@description") {
                        description = val.trim().to_string();
                    } else if let Some(val) = directive.strip_prefix("@author") {
                        author = val.trim().to_string();
                    } else if let Some(val) = directive.strip_prefix("@match") {
                        let pat = val.trim().to_string();
                        if !pat.is_empty() {
                            match_patterns.push(pat);
                        }
                    } else if let Some(val) = directive.strip_prefix("@include") {
                        let pat = val.trim().to_string();
                        if !pat.is_empty() {
                            include_patterns.push(pat);
                        }
                    } else if let Some(val) = directive.strip_prefix("@exclude") {
                        let pat = val.trim().to_string();
                        if !pat.is_empty() {
                            exclude_patterns.push(pat);
                        }
                    } else if let Some(val) = directive.strip_prefix("@run-at") {
                        let r = val.trim().to_lowercase();
                        if r == "document-start" || r == "document-end" || r == "document-idle" {
                            run_at = r;
                        }
                    } else if let Some(val) = directive.strip_prefix("@grant") {
                        let g = val.trim().to_string();
                        if !g.is_empty() {
                            grants.push(g);
                        }
                    }
                }
            }
        }

        if name.is_empty() {
            if let Some(path) = &file_path {
                name = path
                    .file_stem()
                    .and_then(|s| s.to_str())
                    .unwrap_or("Untitled UserScript")
                    .trim_end_matches(".user")
                    .to_string();
            } else {
                name = "Untitled UserScript".to_string();
            }
        }

        if match_patterns.is_empty() && include_patterns.is_empty() {
            match_patterns.push("*://*/*".to_string());
        }

        let slug = name
            .to_lowercase()
            .chars()
            .map(|c| if c.is_alphanumeric() { c } else { '-' })
            .collect::<String>();
        let id = format!("{}-{}", slug.trim_matches('-'), crc32_simple(name.as_bytes()));

        Ok(Self {
            id,
            name,
            version,
            description,
            author,
            match_patterns,
            include_patterns,
            exclude_patterns,
            run_at,
            grants,
            enabled: true,
            code: source.to_string(),
            file_path,
        })
    }

    /// Check if this script applies to the given page URL.
    pub fn matches_url(&self, page_url: &str) -> bool {
        if !self.enabled {
            return false;
        }

        // Check exclusions first
        for pat in &self.exclude_patterns {
            if matches_pattern(pat, page_url) || simple_wildcard_match(pat, page_url) {
                return false;
            }
        }

        // Check match patterns
        for pat in &self.match_patterns {
            if matches_pattern(pat, page_url) {
                return true;
            }
        }

        // Check include patterns
        for pat in &self.include_patterns {
            if matches_pattern(pat, page_url) || simple_wildcard_match(pat, page_url) {
                return true;
            }
        }

        false
    }

    /// Generate wrapped JavaScript containing GM_* API shims and isolated scope.
    pub fn generate_injected_js(&self) -> String {
        let script_id_escaped = serde_json::to_string(&self.id).unwrap_or_default();
        let script_name_escaped = serde_json::to_string(&self.name).unwrap_or_default();
        let script_version_escaped = serde_json::to_string(&self.version).unwrap_or_default();

        format!(
            r#"(function() {{
  try {{
    var __peek_script_id = {script_id_escaped};
    var __peek_script_name = {script_name_escaped};
    var __peek_script_ver = {script_version_escaped};

    var GM_info = {{
      script: {{
        name: __peek_script_name,
        version: __peek_script_ver,
        namespace: "peek://userscripts"
      }}
    }};

    function GM_addStyle(css) {{
      var style = document.createElement("style");
      style.textContent = css;
      (document.head || document.documentElement).appendChild(style);
      return style;
    }}

    function GM_setValue(key, val) {{
      try {{
        localStorage.setItem("__peek_gm_" + __peek_script_id + "_" + key, JSON.stringify(val));
      }} catch (e) {{}}
    }}

    function GM_getValue(key, defVal) {{
      try {{
        var item = localStorage.getItem("__peek_gm_" + __peek_script_id + "_" + key);
        return item !== null ? JSON.parse(item) : defVal;
      }} catch (e) {{
        return defVal;
      }}
    }}

    function GM_deleteValue(key) {{
      try {{
        localStorage.removeItem("__peek_gm_" + __peek_script_id + "_" + key);
      }} catch (e) {{}}
    }}

    function GM_listValues() {{
      var keys = [];
      var prefix = "__peek_gm_" + __peek_script_id + "_";
      for (var i = 0; i < localStorage.length; i++) {{
        var k = localStorage.key(i);
        if (k && k.indexOf(prefix) === 0) {{
          keys.push(k.substring(prefix.length));
        }}
      }}
      return keys;
    }}

    function GM_log(msg) {{
      console.log("[UserScript: " + __peek_script_name + "]", msg);
    }}

    var unsafeWindow = window;

    // --- UserScript Source Begin ---
{code}
    // --- UserScript Source End ---
  }} catch (err) {{
    console.error("[Peek UserScript Error in " + {script_name_escaped} + "]:", err);
  }}
}})();"#,
            script_id_escaped = script_id_escaped,
            script_name_escaped = script_name_escaped,
            script_version_escaped = script_version_escaped,
            code = self.code
        )
    }
}

#[derive(Debug, Clone, Default)]
pub struct UserScriptRegistry {
    pub scripts: Vec<UserScript>,
    pub dir: PathBuf,
}

impl UserScriptRegistry {
    /// Load all `.user.js` files from the specified directory.
    pub fn load_from_dir(dir: &Path) -> Self {
        let _ = std::fs::create_dir_all(dir);
        let mut scripts = Vec::new();

        if let Ok(entries) = std::fs::read_dir(dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_file() {
                    let file_name = path.file_name().and_then(|s| s.to_str()).unwrap_or("");
                    if file_name.ends_with(".user.js") || file_name.ends_with(".js") {
                        if let Ok(content) = std::fs::read_to_string(&path) {
                            if let Ok(script) = UserScript::parse(&content, Some(path)) {
                                scripts.push(script);
                            }
                        }
                    }
                }
            }
        }

        Self {
            scripts,
            dir: dir.to_path_buf(),
        }
    }

    /// Return all active scripts matching a target URL and lifecycle phase.
    pub fn matching_scripts(&self, url: &str, run_at: &str) -> Vec<&UserScript> {
        self.scripts
            .iter()
            .filter(|s| s.enabled && (s.run_at == run_at || run_at == "any") && s.matches_url(url))
            .collect()
    }

    /// Save or update a userscript.
    pub fn save_script(&mut self, name: &str, code: &str) -> std::io::Result<UserScript> {
        let _ = std::fs::create_dir_all(&self.dir);
        let script = UserScript::parse(code, None).unwrap_or_else(|_| UserScript {
            id: format!("custom-{}", crc32_simple(name.as_bytes())),
            name: name.to_string(),
            version: "1.0".to_string(),
            description: String::new(),
            author: String::new(),
            match_patterns: vec!["*://*/*".to_string()],
            include_patterns: Vec::new(),
            exclude_patterns: Vec::new(),
            run_at: "document-end".to_string(),
            grants: Vec::new(),
            enabled: true,
            code: code.to_string(),
            file_path: None,
        });

        let file_name = format!("{}.user.js", script.id);
        let file_path = self.dir.join(file_name);
        std::fs::write(&file_path, code)?;

        let mut script_with_path = script;
        script_with_path.file_path = Some(file_path);

        if let Some(pos) = self.scripts.iter().position(|s| s.id == script_with_path.id) {
            self.scripts[pos] = script_with_path.clone();
        } else {
            self.scripts.push(script_with_path.clone());
        }

        Ok(script_with_path)
    }

    /// Toggle enabled status for a script.
    pub fn toggle_script(&mut self, id: &str, enabled: bool) -> bool {
        if let Some(script) = self.scripts.iter_mut().find(|s| s.id == id) {
            script.enabled = enabled;
            true
        } else {
            false
        }
    }

    /// Delete a script from disk and registry.
    pub fn delete_script(&mut self, id: &str) -> bool {
        if let Some(pos) = self.scripts.iter().position(|s| s.id == id) {
            let script = self.scripts.remove(pos);
            if let Some(path) = script.file_path {
                let _ = std::fs::remove_file(path);
            }
            true
        } else {
            false
        }
    }
}

fn simple_wildcard_match(pattern: &str, text: &str) -> bool {
    if pattern == "*" || pattern == "*://*/*" {
        return true;
    }
    if let Some(prefix) = pattern.strip_suffix('*') {
        text.starts_with(prefix)
    } else {
        text == pattern
    }
}

fn crc32_simple(bytes: &[u8]) -> u32 {
    let mut crc = 0xFFFFFFFFu32;
    for &b in bytes {
        crc ^= b as u32;
        for _ in 0..8 {
            let mask = -( (crc & 1) as i32 ) as u32;
            crc = (crc >> 1) ^ (0xEDB88320 & mask);
        }
    }
    !crc
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_metadata_block() {
        let code = r#"// ==UserScript==
// @name         Dark Theme Enhancer
// @version      2.1.0
// @description  Adds dark theme to example
// @author       Peek Hacker
// @match        https://github.com/*
// @include      https://*.github.io/*
// @exclude      https://github.com/settings/*
// @run-at       document-start
// @grant        GM_addStyle
// @grant        GM_setValue
// ==/UserScript==

GM_addStyle("body { background: #111 !important; }");
"#;

        let script = UserScript::parse(code, None).expect("parse failed");
        assert_eq!(script.name, "Dark Theme Enhancer");
        assert_eq!(script.version, "2.1.0");
        assert_eq!(script.description, "Adds dark theme to example");
        assert_eq!(script.author, "Peek Hacker");
        assert_eq!(script.run_at, "document-start");
        assert_eq!(script.grants, vec!["GM_addStyle", "GM_setValue"]);

        assert!(script.matches_url("https://github.com/rust-lang/rust"));
        assert!(script.matches_url("https://example.github.io/blog"));
        assert!(!script.matches_url("https://github.com/settings/profile"));
        assert!(!script.matches_url("https://google.com"));

        let injected = script.generate_injected_js();
        assert!(injected.contains("GM_addStyle"));
        assert!(injected.contains("GM_setValue"));
        assert!(injected.contains("Dark Theme Enhancer"));
    }
}
