//! Runtime shim for Manifest V3 content scripts and extension pages.
//!
//! Because system WebViews have no isolated worlds, content scripts are wrapped
//! in an IIFE closure providing scoped `chrome.*` and `browser.*` APIs
//! that bridge to Rust via `window.ipc.postMessage`.

/// Wraps content script JavaScript code in an isolated IIFE with a synthetic `chrome.*` API shim.
pub fn wrap_content_script(
    extension_id: &str,
    script_content: &str,
    nonce: &str,
) -> String {
    format!(
        r#"(function() {{
    const __PEEK_EXT_ID = "{extension_id}";
    const __PEEK_NONCE = "{nonce}";

    function postExtMessage(action, payload) {{
        try {{
            if (window.ipc && window.ipc.postMessage) {{
                window.ipc.postMessage(JSON.stringify({{
                    type: "pageEvent",
                    nonce: __PEEK_NONCE,
                    event: {{
                        extensionMessage: {{
                            extensionId: __PEEK_EXT_ID,
                            action: action,
                            payload: payload
                        }}
                    }}
                }}));
            }}
        }} catch (e) {{
            console.warn("[PeekExt] IPC error:", e);
        }}
    }}

    function createStorageArea(areaName) {{
        const storageKey = "__peek_ext_" + __PEEK_EXT_ID + "_" + areaName;
        return {{
            get: function(keys, callback) {{
                return new Promise((resolve) => {{
                    try {{
                        const raw = localStorage.getItem(storageKey) || "{{}}";
                        const parsed = JSON.parse(raw);
                        let result = {{}};

                        if (!keys) {{
                            result = parsed;
                        }} else if (typeof keys === "string") {{
                            result[keys] = parsed[keys];
                        }} else if (Array.isArray(keys)) {{
                            for (const k of keys) {{
                                if (k in parsed) result[k] = parsed[k];
                            }}
                        }} else if (typeof keys === "object") {{
                            result = Object.assign({{}}, keys);
                            for (const k of Object.keys(keys)) {{
                                if (k in parsed && parsed[k] !== undefined) {{
                                    result[k] = parsed[k];
                                }}
                            }}
                        }}

                        if (typeof callback === "function") callback(result);
                        resolve(result);
                    }} catch (e) {{
                        const empty = {{}};
                        if (typeof callback === "function") callback(empty);
                        resolve(empty);
                    }}
                }});
            }},
            set: function(items, callback) {{
                return new Promise((resolve) => {{
                    try {{
                        const raw = localStorage.getItem(storageKey) || "{{}}";
                        const parsed = JSON.parse(raw);
                        Object.assign(parsed, items);
                        localStorage.setItem(storageKey, JSON.stringify(parsed));
                        postExtMessage("storage.onChanged", {{ area: areaName, items: items }});
                        if (typeof callback === "function") callback();
                        resolve();
                    }} catch (e) {{
                        if (typeof callback === "function") callback();
                        resolve();
                    }}
                }});
            }},
            remove: function(keys, callback) {{
                return new Promise((resolve) => {{
                    try {{
                        const raw = localStorage.getItem(storageKey) || "{{}}";
                        const parsed = JSON.parse(raw);
                        const arr = Array.isArray(keys) ? keys : [keys];
                        for (const k of arr) delete parsed[k];
                        localStorage.setItem(storageKey, JSON.stringify(parsed));
                        if (typeof callback === "function") callback();
                        resolve();
                    }} catch (e) {{
                        if (typeof callback === "function") callback();
                        resolve();
                    }}
                }});
            }},
            clear: function(callback) {{
                return new Promise((resolve) => {{
                    try {{
                        localStorage.removeItem(storageKey);
                        if (typeof callback === "function") callback();
                        resolve();
                    }} catch (e) {{
                        if (typeof callback === "function") callback();
                        resolve();
                    }}
                }});
            }}
        }};
    }}

    const localStorageArea = createStorageArea("local");
    const syncStorageArea = createStorageArea("sync");

    const chrome = {{
        runtime: {{
            id: __PEEK_EXT_ID,
            lastError: null,
            getURL: function(path) {{
                const clean = path.startsWith('/') ? path.slice(1) : path;
                return "peek-extension://" + __PEEK_EXT_ID + "/" + clean;
            }},
            getManifest: function() {{
                return {{
                    manifest_version: 3,
                    name: "Peek Extension Host",
                    version: "1.0.0"
                }};
            }},
            sendMessage: function(msg, options, responseCallback) {{
                const cb = typeof options === "function" ? options : responseCallback;
                postExtMessage("runtime.sendMessage", msg);
                return new Promise((resolve) => {{
                    const res = {{ success: true }};
                    if (typeof cb === "function") {{
                        setTimeout(() => cb(res), 10);
                    }}
                    resolve(res);
                }});
            }},
            onMessage: {{
                addListener: function(fn) {{
                    window.addEventListener("peek-ext-message-" + __PEEK_EXT_ID, function(e) {{
                        if (e.detail) fn(e.detail, {{ id: __PEEK_EXT_ID }}, function() {{}});
                    }});
                }},
                removeListener: function() {{}}
            }},
            onInstalled: {{
                addListener: function(fn) {{
                    setTimeout(() => {{
                        try {{ fn({{ reason: "install" }}); }} catch (e) {{}}
                    }}, 10);
                }},
                removeListener: function() {{}}
            }},
            onStartup: {{
                addListener: function(fn) {{}},
                removeListener: function() {{}}
            }}
        }},
        storage: {{
            local: localStorageArea,
            sync: syncStorageArea,
            onChanged: {{
                addListener: function(fn) {{
                    window.addEventListener("peek-storage-changed-" + __PEEK_EXT_ID, function(e) {{
                        if (e.detail) fn(e.detail, "sync");
                    }});
                }},
                removeListener: function() {{}}
            }}
        }},
        declarativeNetRequest: {{
            updateDynamicRules: function(options, callback) {{
                postExtMessage("declarativeNetRequest.updateDynamicRules", options);
                return new Promise((resolve) => {{
                    if (typeof callback === "function") callback();
                    resolve();
                }});
            }},
            getDynamicRules: function(callback) {{
                return new Promise((resolve) => {{
                    const rules = [];
                    if (typeof callback === "function") callback(rules);
                    resolve(rules);
                }});
            }},
            updateSessionRules: function(options, callback) {{
                return new Promise((resolve) => {{
                    if (typeof callback === "function") callback();
                    resolve();
                }});
            }},
            getSessionRules: function(callback) {{
                return new Promise((resolve) => {{
                    const rules = [];
                    if (typeof callback === "function") callback(rules);
                    resolve(rules);
                }});
            }}
        }},
        action: {{
            setTitle: function(details, cb) {{ if (typeof cb === "function") cb(); return Promise.resolve(); }},
            setIcon: function(details, cb) {{ if (typeof cb === "function") cb(); return Promise.resolve(); }},
            setBadgeText: function(details, cb) {{ if (typeof cb === "function") cb(); return Promise.resolve(); }},
            setBadgeBackgroundColor: function(details, cb) {{ if (typeof cb === "function") cb(); return Promise.resolve(); }},
            openPopup: function(cb) {{ if (typeof cb === "function") cb(); return Promise.resolve(); }},
            onClicked: {{ addListener: function() {{}}, removeListener: function() {{}} }}
        }},
        tabs: {{
            query: function(queryInfo, callback) {{
                return new Promise((resolve) => {{
                    const tab = {{
                        id: 1,
                        active: true,
                        url: window.location.href,
                        title: document.title
                    }};
                    if (typeof callback === "function") callback([tab]);
                    resolve([tab]);
                }});
            }},
            sendMessage: function(tabId, message, options, responseCallback) {{
                const cb = typeof options === "function" ? options : responseCallback;
                postExtMessage("tabs.sendMessage", {{ tabId: tabId, message: message }});
                return new Promise((resolve) => {{
                    const res = {{ success: true }};
                    if (typeof cb === "function") cb(res);
                    resolve(res);
                }});
            }},
            get: function(tabId, callback) {{
                return new Promise((resolve) => {{
                    const tab = {{
                        id: tabId || 1,
                        active: true,
                        url: window.location.href,
                        title: document.title
                    }};
                    if (typeof callback === "function") callback(tab);
                    resolve(tab);
                }});
            }},
            create: function(createProperties, callback) {{
                return new Promise((resolve) => {{
                    if (createProperties && createProperties.url) {{
                        window.open(createProperties.url, "_blank");
                    }}
                    if (typeof callback === "function") callback({{}});
                    resolve({{}});
                }});
            }}
        }},
        i18n: {{
            getMessage: function(name) {{
                return name;
            }}
        }}
    }};

    const browser = chrome;

    if (typeof window !== "undefined") {{
        if (!window.chrome) window.chrome = chrome;
        if (!window.browser) window.browser = browser;
    }}

    try {{
        {script_content}
    }} catch (err) {{
        console.error("[PeekExt] Script error in " + __PEEK_EXT_ID + ":", err);
    }}
}})();"#,
        extension_id = extension_id,
        nonce = nonce,
        script_content = script_content
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wraps_content_script_safely() {
        let code = "console.log(chrome.runtime.id);";
        let wrapped = wrap_content_script("ext123", code, "nonce456");
        assert!(wrapped.contains("__PEEK_EXT_ID = \"ext123\""));
        assert!(wrapped.contains("__PEEK_NONCE = \"nonce456\""));
        assert!(wrapped.contains("console.log(chrome.runtime.id);"));
    }
}
