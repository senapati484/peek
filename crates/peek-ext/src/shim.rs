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

    const chrome = {{
        runtime: {{
            id: __PEEK_EXT_ID,
            getURL: function(path) {{
                const clean = path.startsWith('/') ? path.slice(1) : path;
                return "peek-extension://" + __PEEK_EXT_ID + "/" + clean;
            }},
            sendMessage: function(msg, responseCallback) {{
                postExtMessage("runtime.sendMessage", msg);
                if (typeof responseCallback === "function") {{
                    // Callback placeholder for Tier 1 async response
                    setTimeout(() => responseCallback({{ success: true }}), 10);
                }}
            }},
            onMessage: {{
                addListener: function(fn) {{
                    window.addEventListener("peek-ext-message-" + __PEEK_EXT_ID, function(e) {{
                        if (e.detail) fn(e.detail);
                    }});
                }},
                removeListener: function() {{}}
            }}
        }},
        storage: {{
            local: {{
                get: function(keys, callback) {{
                    try {{
                        const raw = localStorage.getItem("__peek_ext_" + __PEEK_EXT_ID) || "{{}}";
                        const parsed = JSON.parse(raw);
                        if (typeof callback === "function") callback(parsed);
                    }} catch (e) {{
                        if (typeof callback === "function") callback({{}});
                    }}
                }},
                set: function(items, callback) {{
                    try {{
                        const raw = localStorage.getItem("__peek_ext_" + __PEEK_EXT_ID) || "{{}}";
                        const parsed = JSON.parse(raw);
                        Object.assign(parsed, items);
                        localStorage.setItem("__peek_ext_" + __PEEK_EXT_ID, JSON.stringify(parsed));
                        if (typeof callback === "function") callback();
                    }} catch (e) {{
                        if (typeof callback === "function") callback();
                    }}
                }},
                remove: function(keys, callback) {{
                    try {{
                        const raw = localStorage.getItem("__peek_ext_" + __PEEK_EXT_ID) || "{{}}";
                        const parsed = JSON.parse(raw);
                        const arr = Array.isArray(keys) ? keys : [keys];
                        for (const k of arr) delete parsed[k];
                        localStorage.setItem("__peek_ext_" + __PEEK_EXT_ID, JSON.stringify(parsed));
                        if (typeof callback === "function") callback();
                    }} catch (e) {{
                        if (typeof callback === "function") callback();
                    }}
                }}
            }}
        }},
        i18n: {{
            getMessage: function(name) {{
                return name;
            }}
        }}
    }};

    const browser = chrome;

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
