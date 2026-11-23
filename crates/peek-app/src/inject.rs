//! Initialization scripts injected into webviews.
//!
//! Privileged (UI) webviews get the session nonce as a global because they
//! load only local `peek://` assets. Page webviews keep the nonce inside a
//! closure so page scripts cannot reuse the bridge.

pub fn ui_script(nonce: &str) -> String {
    UI_SCRIPT.replace("__NONCE__", nonce)
}

pub fn page_script(
    nonce: &str,
    role: &str,
    hover_delay_ms: u64,
    edge: &str,
    edge_zone: f64,
) -> String {
    PAGE_SCRIPT
        .replace("__NONCE__", nonce)
        .replace("__ROLE__", role)
        .replace("__HOVER_MS__", &hover_delay_ms.to_string())
        .replace("__EDGE__", edge)
        .replace("__EDGE_ZONE__", &edge_zone.to_string())
}

const UI_SCRIPT: &str = r#"
(function () {
  "use strict";
  var NONCE = "__NONCE__";
  var VERSION = 1;
  window.__PEEK_NONCE__ = NONCE;
  var post = (window.ipc && typeof window.ipc.postMessage === "function")
    ? window.ipc.postMessage.bind(window.ipc)
    : function () {};
  window.__peekSend = function (payload) {
    try {
      post(JSON.stringify({ version: VERSION, nonce: NONCE, payload: payload }));
    } catch (e) {}
  };
  window.__peekDeliver = function (raw) {
    try {
      var env = JSON.parse(raw);
      if (env && env.version === VERSION && env.nonce === NONCE && typeof window.__peekOnEvent === "function") {
        window.__peekOnEvent(env.payload);
      }
    } catch (e) {}
  };
})();
"#;

const PAGE_SCRIPT: &str = r##"
(function () {
  "use strict";
  var NONCE = "__NONCE__";
  var ROLE = "__ROLE__";
  var HOVER_MS = __HOVER_MS__;
  var EDGE = "__EDGE__";
  var EDGE_ZONE = __EDGE_ZONE__;
  var VERSION = 1;

  var post = (window.ipc && typeof window.ipc.postMessage === "function")
    ? window.ipc.postMessage.bind(window.ipc)
    : null;

  function send(payload) {
    if (!post) return;
    try {
      post(JSON.stringify({ version: VERSION, nonce: NONCE, payload: payload }));
    } catch (e) {}
  }

  if (ROLE === "preview") {
    function mute() {
      try {
        var media = document.querySelectorAll("audio,video");
        for (var i = 0; i < media.length; i++) {
          media[i].muted = true;
          media[i].pause();
        }
      } catch (e) {}
    }
    document.addEventListener("mouseover", function () {
      send({ type: "previewEnter" });
    }, true);
    document.addEventListener("mouseout", function (ev) {
      if (!ev.relatedTarget) send({ type: "previewLeave" });
    }, true);
    document.addEventListener("click", function (ev) {
      ev.preventDefault();
      ev.stopPropagation();
      send({ type: "previewClick", url: String(location.href) });
    }, true);
    document.addEventListener("DOMContentLoaded", mute);
    setTimeout(mute, 400);
    setTimeout(mute, 1500);
    return;
  }

  try {
    if (window.PublicKeyCredential) {
      if (typeof PublicKeyCredential.isUserVerifyingPlatformAuthenticatorAvailable === "function") {
        PublicKeyCredential.isUserVerifyingPlatformAuthenticatorAvailable = function () {
          return Promise.resolve(false);
        };
      }
      if (typeof PublicKeyCredential.isConditionalMediationAvailable === "function") {
        PublicKeyCredential.isConditionalMediationAvailable = function () {
          return Promise.resolve(false);
        };
      }
    }
    if (window.navigator && window.navigator.credentials && typeof window.navigator.credentials.get === "function") {
      var origCredGet = window.navigator.credentials.get.bind(window.navigator.credentials);
      window.navigator.credentials.get = function (options) {
        if (options && options.publicKey) {
          var timeoutPromise = new Promise(function (_, reject) {
            setTimeout(function () {
              reject(new DOMException("Passkey platform authenticator not available or permitted in this browser.", "NotAllowedError"));
            }, 2500);
          });
          return Promise.race([origCredGet(options), timeoutPromise]);
        }
        return origCredGet(options);
      };
    }
  } catch (e) {}

  try {
    var AD_PATTERNS = [
      /doubleclick\.net/,
      /googlesyndication\.com/,
      /adservice\.google\./,
      /adnxs\.com/,
      /taboola\.com/,
      /outbrain\.com/,
      /amazon-adsystem\.com/,
      /scorecardresearch\.com/
    ];
    function isAdUrl(u) {
      if (!u || typeof u !== "string") return false;
      var host = (location.hostname || "").toLowerCase();
      if (host === "accounts.google.com" || host.endsWith(".accounts.google.com") || host.indexOf("myaccount.google.") >= 0) return false;
      for (var i = 0; i < AD_PATTERNS.length; i++) {
        if (AD_PATTERNS[i].test(u)) return true;
      }
      return false;
    }
    var origFetch = window.fetch;
    if (origFetch) {
      window.fetch = function (input, init) {
        var u = (typeof input === "string") ? input : (input && input.url ? input.url : "");
        if (isAdUrl(u)) {
          send({ type: "blockedResource", url: u });
          return Promise.reject(new TypeError("Blocked by Peek Adblock"));
        }
        return origFetch.apply(this, arguments);
      };
    }
    var origXhrOpen = XMLHttpRequest.prototype.open;
    if (origXhrOpen) {
      XMLHttpRequest.prototype.open = function (method, url) {
        if (isAdUrl(url)) {
          send({ type: "blockedResource", url: String(url) });
          this.__blockedByPeek = true;
          return;
        }
        return origXhrOpen.apply(this, arguments);
      };
      var origXhrSend = XMLHttpRequest.prototype.send;
      XMLHttpRequest.prototype.send = function () {
        if (this.__blockedByPeek) return;
        return origXhrSend.apply(this, arguments);
      };
    }
  } catch (e) {}

  try {
    var cookieDesc = Object.getOwnPropertyDescriptor(Document.prototype, "cookie") ||
                     Object.getOwnPropertyDescriptor(HTMLDocument.prototype, "cookie");
    if (cookieDesc && cookieDesc.set && cookieDesc.get) {
      var origSet = cookieDesc.set;
      var origGet = cookieDesc.get;
      Object.defineProperty(document, "cookie", {
        configurable: true,
        enumerable: true,
        get: function () {
          return origGet.call(document);
        },
        set: function (val) {
          origSet.call(document, val);
          try {
            send({ type: "cookieSet", cookie: String(val), url: String(location.href) });
          } catch (e) {}
        }
      });
    }
  } catch (e) {}


  document.addEventListener("keydown", function (ev) {
    var mod = ev.metaKey || ev.ctrlKey;
    var key = (ev.key || "").toLowerCase();

    if (!mod && !ev.altKey && key === "escape") {
      send({ type: "shortcut", shortcut: "escape" });
      return;
    }
    if (!mod && !ev.altKey) return;

    if (mod && !ev.shiftKey && !ev.altKey) {
      if (key === "l") { ev.preventDefault(); send({ type: "shortcut", shortcut: "focusOmnibar" }); return; }
      if (key === "t") { ev.preventDefault(); send({ type: "shortcut", shortcut: "newTab" }); return; }
      if (key === "w") { ev.preventDefault(); send({ type: "shortcut", shortcut: "closeTab" }); return; }
      if (key === "r") { ev.preventDefault(); send({ type: "shortcut", shortcut: "reload" }); return; }
      if (key === "k") { ev.preventDefault(); send({ type: "shortcut", shortcut: "openCommandPalette" }); return; }
      if (key === "f") { ev.preventDefault(); send({ type: "shortcut", shortcut: "findInPage" }); return; }
      if (key === "." || key === ">") { ev.preventDefault(); send({ type: "shortcut", shortcut: "toggleZenMode" }); return; }
      if (key === "=" || key === "+") { ev.preventDefault(); send({ type: "shortcut", shortcut: "zoomIn" }); return; }
      if (key === "-" || key === "_") { ev.preventDefault(); send({ type: "shortcut", shortcut: "zoomOut" }); return; }
      if (key === "0") { ev.preventDefault(); send({ type: "shortcut", shortcut: "zoomReset" }); return; }
      if (/^[1-9]$/.test(key)) {
        ev.preventDefault();
        send({ type: "shortcut", shortcut: { selectTab: Number(key) } });
        return;
      }
    }
    if (mod && ev.shiftKey && !ev.altKey) {
      if (key === "[" || key === "{") { ev.preventDefault(); send({ type: "shortcut", shortcut: "openTabsDrawer" }); return; }
      if (key === "]" || key === "}") { ev.preventDefault(); send({ type: "shortcut", shortcut: "openSettingsDrawer" }); return; }
      if (key === "c") { ev.preventDefault(); send({ type: "shortcut", shortcut: "copyCleanUrl" }); return; }
      if (key === "m") { ev.preventDefault(); send({ type: "shortcut", shortcut: "toggleMute" }); return; }
      if (key === "r") { ev.preventDefault(); send({ type: "shortcut", shortcut: "toggleReaderMode" }); return; }
      if (key === "d") { ev.preventDefault(); send({ type: "shortcut", shortcut: "toggleInvertDark" }); return; }
      if (key === "p") { ev.preventDefault(); send({ type: "shortcut", shortcut: "togglePiP" }); return; }
      if (key === "f") { ev.preventDefault(); send({ type: "shortcut", shortcut: "toggleZenMode" }); return; }
    }
    if (mod && ev.altKey && !ev.shiftKey) {
      if (key === "c") { ev.preventDefault(); send({ type: "shortcut", shortcut: "copyMarkdownLink" }); return; }
    }
    if (mod && key === "tab") {
      ev.preventDefault();
      send({ type: "shortcut", shortcut: ev.shiftKey ? "prevTab" : "nextTab" });
      return;
    }
    if (ev.altKey && key === "arrowleft") { ev.preventDefault(); send({ type: "shortcut", shortcut: "back" }); return; }
    if (ev.altKey && key === "arrowright") { ev.preventDefault(); send({ type: "shortcut", shortcut: "forward" }); return; }
  }, true);

  document.addEventListener("mousedown", function (ev) {
    if (ev.button === 3) { ev.preventDefault(); send({ type: "mouseNav", forward: false }); }
    if (ev.button === 4) { ev.preventDefault(); send({ type: "mouseNav", forward: true }); }
  }, true);

  var scrollScheduled = false;
  window.addEventListener("scroll", function () {
    if (scrollScheduled) return;
    scrollScheduled = true;
    requestAnimationFrame(function () {
      scrollScheduled = false;
      send({ type: "scroll", y: window.scrollY });
    });
  }, { passive: true });

  var audioTimer = null;
  function reportAudio() {
    var playing = false;
    try {
      var media = document.querySelectorAll("audio,video");
      for (var i = 0; i < media.length; i++) {
        var m = media[i];
        if (!m.paused && !m.muted && m.volume > 0) { playing = true; break; }
      }
    } catch (e) {}
    send({ type: "audioState", playing: playing });
  }
  function scheduleAudio() {
    if (audioTimer) return;
    audioTimer = setTimeout(function () { audioTimer = null; reportAudio(); }, 250);
  }
  document.addEventListener("play", scheduleAudio, true);
  document.addEventListener("pause", scheduleAudio, true);
  document.addEventListener("volumechange", scheduleAudio, true);
  document.addEventListener("DOMContentLoaded", scheduleAudio);

  var dirtySent = false;
  document.addEventListener("input", function (ev) {
    var t = ev.target;
    if (!t) return;
    var tag = (t.tagName || "").toLowerCase();
    var editing = tag === "input" || tag === "textarea" || tag === "select" || t.isContentEditable;
    if (editing && !dirtySent) {
      dirtySent = true;
      send({ type: "formDirty", dirty: true });
    }
  }, true);

  var favSent = "";
  function reportFavicon() {
    try {
      var link = document.querySelector('link[rel~="icon"]');
      if (!link || !link.href || link.href === favSent) return;
      favSent = link.href;
      send({ type: "favicon", href: link.href });
    } catch (e) {}
  }
  document.addEventListener("DOMContentLoaded", reportFavicon);
  setTimeout(reportFavicon, 1200);

  var lastReportedUrl = location.href;
  function reportUrl() {
    try {
      if (location.href && location.href !== lastReportedUrl) {
        lastReportedUrl = location.href;
        send({ type: "urlChanged", url: location.href });
      }
    } catch (e) {}
  }
  var origPushState = history.pushState;
  if (origPushState) {
    history.pushState = function () {
      var ret = origPushState.apply(this, arguments);
      reportUrl();
      return ret;
    };
  }
  var origReplaceState = history.replaceState;
  if (origReplaceState) {
    history.replaceState = function () {
      var ret = origReplaceState.apply(this, arguments);
      reportUrl();
      return ret;
    };
  }
  window.addEventListener("popstate", reportUrl);
  window.addEventListener("hashchange", reportUrl);

  var hoverTimer = null;
  var hoverAnchor = null;
  var hoverUrl = "";

  function clearHover(notify) {
    if (hoverTimer) { clearTimeout(hoverTimer); hoverTimer = null; }
    hoverAnchor = null;
    hoverUrl = "";
    if (notify) send({ type: "hoverEnd" });
  }

  function previewable(a) {
    if (!a || !a.href) return false;
    if (a.href.indexOf("http") !== 0) return false;
    var here = location.href.split("#")[0];
    if (a.href.split("#")[0] === here) return false;
    return true;
  }

  function isPreviewMod(ev) {
    var isMac = (navigator.platform || "").indexOf("Mac") >= 0;
    return (isMac ? ev.metaKey : ev.ctrlKey) || ev.shiftKey;
  }

  document.addEventListener("mouseover", function (ev) {
    var a = ev.target && ev.target.closest ? ev.target.closest("a[href]") : null;
    if (!previewable(a)) {
      if (hoverAnchor && a !== hoverAnchor) clearHover(true);
      return;
    }
    if (hoverTimer) clearTimeout(hoverTimer);
    hoverAnchor = a;
    hoverUrl = a.href;
    if (!isPreviewMod(ev)) return;
    var rect = a.getBoundingClientRect();
    hoverTimer = setTimeout(function () {
      hoverTimer = null;
      if (!hoverAnchor) return;
      send({
        type: "hover",
        url: hoverUrl,
        forced: true,
        rect: { x: rect.left, y: rect.top, width: rect.width, height: rect.height }
      });
    }, 80);
  }, true);

  document.addEventListener("keydown", function (ev) {
    if ((ev.code === "Space" || ev.key === " ") && hoverAnchor && hoverUrl && !hoverTimer) {
      var t = ev.target;
      var tag = (t && t.tagName || "").toLowerCase();
      if (tag !== "input" && tag !== "textarea" && !t.isContentEditable) {
        ev.preventDefault();
        var rect = hoverAnchor.getBoundingClientRect();
        send({
          type: "hover",
          url: hoverUrl,
          forced: true,
          rect: { x: rect.left, y: rect.top, width: rect.width, height: rect.height }
        });
        return;
      }
    }
    if (isPreviewMod(ev) && hoverAnchor && hoverUrl && !hoverTimer) {
      var rect = hoverAnchor.getBoundingClientRect();
      send({
        type: "hover",
        url: hoverUrl,
        forced: true,
        rect: { x: rect.left, y: rect.top, width: rect.width, height: rect.height }
      });
    }
  }, true);

  document.addEventListener("keyup", function (ev) {
    if (!isPreviewMod(ev)) {
      if (hoverTimer) {
        clearTimeout(hoverTimer);
        hoverTimer = null;
      }
      clearHover(true);
    }
  }, true);

  document.addEventListener("mouseout", function (ev) {
    var a = ev.target && ev.target.closest ? ev.target.closest("a[href]") : null;
    if (!a || a !== hoverAnchor) return;
    var to = ev.relatedTarget;
    if (to && a.contains(to)) return;
    clearHover(true);
  }, true);

  window.addEventListener("blur", function () { clearHover(true); });
  document.addEventListener("mousedown", function () { clearHover(true); }, true);

  var edgeTimer = null;
  var lastEdge = 0;

  function handleEdgeMove(ev) {
    if (!ev || ev.buttons !== 0) return;
    var winW = window.innerWidth;
    var docW = (document.documentElement && document.documentElement.clientWidth) || winW;
    var rightLimit = Math.min(winW, docW);
    var distRight = Math.min(winW - ev.clientX, rightLimit - ev.clientX);
    var distLeft = ev.clientX;

    var zone = Math.max(10, typeof EDGE_ZONE === "number" ? EDGE_ZONE : 10);
    var atLeft = distLeft <= zone;
    var atRight = distRight <= zone;

    if (!atLeft && !atRight) {
      if (edgeTimer) { clearTimeout(edgeTimer); edgeTimer = null; }
      return;
    }
    if (edgeTimer) return;
    var side = atLeft ? "left" : "right";
    edgeTimer = setTimeout(function () {
      edgeTimer = null;
      var now = Date.now();
      if (now - lastEdge < 350) return;
      lastEdge = now;
      send({ type: "edgeProximity", side: side, distance: 0 });
    }, 60);
  }

  window.addEventListener("mousemove", handleEdgeMove, { passive: true });

  window.addEventListener("mouseleave", function (ev) {
    if (edgeTimer) { clearTimeout(edgeTimer); edgeTimer = null; }
    if (ev && typeof ev.clientX === "number") {
      var winW = window.innerWidth;
      var docW = (document.documentElement && document.documentElement.clientWidth) || winW;
      var rightLimit = Math.min(winW, docW);
      var zone = Math.max(12, typeof EDGE_ZONE === "number" ? EDGE_ZONE + 2 : 12);
      var now = Date.now();
      if (now - lastEdge >= 350) {
        if (ev.clientX <= zone) {
          lastEdge = now;
          send({ type: "edgeProximity", side: "left", distance: 0 });
        } else if (ev.clientX >= rightLimit - zone || ev.clientX >= winW - zone) {
          lastEdge = now;
          send({ type: "edgeProximity", side: "right", distance: 0 });
        }
      }
    }
  });

  var lastWheelNav = 0;
  window.addEventListener("wheel", function (ev) {
    if (Math.abs(ev.deltaX) > 35 && Math.abs(ev.deltaY) < 25) {
      var now = Date.now();
      if (now - lastWheelNav < 700) return;
      var winW = window.innerWidth;
      var docW = (document.documentElement && document.documentElement.clientWidth) || winW;
      var rightLimit = Math.min(winW, docW);
      var atLeftEdge = ev.clientX <= 50;
      var atRightEdge = (winW - ev.clientX) <= 50 || (rightLimit - ev.clientX) <= 50;
      if (atLeftEdge && ev.deltaX < -25) {
        lastWheelNav = now;
        send({ type: "edgeProximity", side: "left", distance: 0 });
      } else if (atRightEdge && ev.deltaX > 25) {
        lastWheelNav = now;
        send({ type: "edgeProximity", side: "right", distance: 0 });
      }
    }
  }, { passive: true });

  window.__peekFind = function (query, forward) {
    if (!query) return;
    try {
      var found = window.find(query, false, !forward, true, false, false, false);
      if (!found) {
        window.find(query, false, !forward, true, false, true, false);
      }
    } catch (e) {}
  };

  window.__peekCloseFind = function () {
    var box = document.getElementById("__peek_find_box__");
    if (box) box.remove();
    var style = document.getElementById("__peek_find_style__");
    if (style) style.remove();
  };

  window.__peekShowFind = function () {
    var existing = document.getElementById("__peek_find_box__");
    if (existing) {
      var inp = existing.querySelector("input");
      if (inp) { inp.focus(); inp.select(); }
      return;
    }
    var style = document.createElement("style");
    style.id = "__peek_find_style__";
    style.textContent = `
      #__peek_find_box__ {
        position: fixed;
        top: 14px;
        right: 18px;
        z-index: 2147483646;
        display: flex;
        align-items: center;
        gap: 6px;
        padding: 5px 8px;
        background: rgba(18, 18, 20, 0.94);
        border: 1px solid rgba(255, 255, 255, 0.14);
        border-radius: 12px;
        box-shadow: 0 10px 30px rgba(0, 0, 0, 0.5), 0 0 0 1px rgba(0, 0, 0, 0.3);
        backdrop-filter: blur(16px);
        -webkit-backdrop-filter: blur(16px);
        font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", sans-serif;
      }
      #__peek_find_box__ input {
        all: unset;
        width: 170px;
        color: #ffffff;
        font-size: 13px;
        padding: 4px 6px;
        border-radius: 6px;
        background: rgba(255, 255, 255, 0.08);
      }
      #__peek_find_box__ input::placeholder {
        color: rgba(255, 255, 255, 0.4);
      }
      #__peek_find_box__ button {
        all: unset;
        cursor: pointer;
        color: rgba(255, 255, 255, 0.7);
        font-size: 12px;
        padding: 4px 7px;
        border-radius: 6px;
        display: flex;
        align-items: center;
        justify-content: center;
        transition: background 0.12s, color 0.12s;
      }
      #__peek_find_box__ button:hover {
        background: rgba(255, 255, 255, 0.14);
        color: #ffffff;
      }
    `;
    document.documentElement.appendChild(style);

    var box = document.createElement("div");
    box.id = "__peek_find_box__";
    box.innerHTML = `
      <input type="text" placeholder="Find in page..." id="__peek_find_inp__" />
      <button id="__peek_find_prev__" title="Previous (Shift+Enter)">▲</button>
      <button id="__peek_find_next__" title="Next (Enter)">▼</button>
      <button id="__peek_find_cls__" title="Close (Escape)">✕</button>
    `;
    document.documentElement.appendChild(box);

    var inp = box.querySelector("#__peek_find_inp__");
    var prev = box.querySelector("#__peek_find_prev__");
    var next = box.querySelector("#__peek_find_next__");
    var cls = box.querySelector("#__peek_find_cls__");

    inp.focus();
    inp.addEventListener("input", function () {
      window.__peekFind(inp.value, true);
    });
    inp.addEventListener("keydown", function (ev) {
      if (ev.key === "Enter") {
        ev.preventDefault();
        window.__peekFind(inp.value, !ev.shiftKey);
      } else if (ev.key === "Escape") {
        ev.preventDefault();
        window.__peekCloseFind();
      }
    });
    prev.addEventListener("click", function () { window.__peekFind(inp.value, false); });
    next.addEventListener("click", function () { window.__peekFind(inp.value, true); });
    cls.addEventListener("click", window.__peekCloseFind);
  };

  window.__peekToggleMute = function () {
    try {
      var media = document.querySelectorAll("audio, video");
      var anyUnmuted = false;
      for (var i = 0; i < media.length; i++) {
        if (!media[i].muted) { anyUnmuted = true; break; }
      }
      for (var j = 0; j < media.length; j++) {
        media[j].muted = anyUnmuted;
      }
      return anyUnmuted;
    } catch (e) {
      return false;
    }
  };

  window.__peekToggleReader = function () {
    try {
      var existing = document.getElementById("__peek_reader_view__");
      if (existing) {
        existing.remove();
        var st = document.getElementById("__peek_reader_style__");
        if (st) st.remove();
        return false;
      }
      var target = document.querySelector("article") || document.querySelector("main") || document.body;
      var clone = target.cloneNode(true);
      var bad = clone.querySelectorAll("nav, header, footer, script, style, noscript, iframe, .ads, [role='banner'], [role='navigation']");
      for (var b = 0; b < bad.length; b++) bad[b].remove();
      var words = (clone.innerText || "").trim().split(/\s+/).filter(Boolean).length;
      var readMin = Math.max(1, Math.round(words / 220));

      var style = document.createElement("style");
      style.id = "__peek_reader_style__";
      style.textContent = `
        #__peek_reader_view__ {
          position: fixed; inset: 0; z-index: 2147483647;
          background: #111113; color: #e4e4e7;
          overflow-y: auto; padding: 44px 20px 80px;
          font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", Georgia, serif;
          line-height: 1.8; font-size: 18px;
        }
        .__peek_reader_wrap { max-width: 680px; margin: 0 auto; }
        .__peek_reader_header { display: flex; align-items: center; justify-content: space-between; border-bottom: 1px solid rgba(255,255,255,0.1); padding-bottom: 14px; margin-bottom: 28px; }
        .__peek_reader_badge { background: rgba(59,130,246,0.18); color: #60a5fa; padding: 3px 10px; border-radius: 9999px; font-size: 12px; font-weight: 600; }
        .__peek_reader_meta { color: #a1a1aa; font-size: 12.5px; }
        .__peek_reader_btn { all: unset; cursor: pointer; background: rgba(255,255,255,0.08); color: #fff; padding: 4px 12px; border-radius: 7px; font-size: 12px; }
        .__peek_reader_btn:hover { background: rgba(255,255,255,0.16); }
        .__peek_reader_title { font-size: 32px; font-weight: 700; line-height: 1.25; margin-bottom: 24px; color: #f4f4f5; }
        .__peek_reader_content p { margin-bottom: 1.6em; }
        .__peek_reader_content img { max-width: 100%; height: auto; border-radius: 8px; margin: 20px 0; }
        .__peek_reader_content a { color: #60a5fa; text-decoration: underline; }
      `;
      document.documentElement.appendChild(style);

      var container = document.createElement("div");
      container.id = "__peek_reader_view__";
      container.innerHTML = `
        <div class="__peek_reader_wrap">
          <div class="__peek_reader_header">
            <span class="__peek_reader_badge">📖 Reader View</span>
            <span class="__peek_reader_meta">${words} words • ~${readMin} min read</span>
            <button class="__peek_reader_btn" onclick="window.__peekToggleReader()">✕ Exit</button>
          </div>
          <h1 class="__peek_reader_title">${document.title || ''}</h1>
          <div class="__peek_reader_content">${clone.innerHTML}</div>
        </div>
      `;
      document.documentElement.appendChild(container);
      return true;
    } catch (e) {
      return false;
    }
  };

  window.__peekCheckInvertDark = function () {
    try {
      var existing = document.getElementById("__peek_invert_dark__");
      send({ type: "invertDarkState", active: !!existing, message: "" });
    } catch (e) {}
  };

  window.__peekToggleInvertDark = function () {
    try {
      var existing = document.getElementById("__peek_invert_dark__");
      if (existing) {
        existing.remove();
        send({ type: "invertDarkState", active: false, message: "Night mode disabled" });
        return false;
      }

      function parseRgb(color) {
        if (!color || color === "transparent" || color === "inherit" || color === "rgba(0, 0, 0, 0)") return null;
        var m = color.match(/rgba?\((\d+),\s*(\d+),\s*(\d+)/i);
        if (m) {
          return { r: parseInt(m[1], 10), g: parseInt(m[2], 10), b: parseInt(m[3], 10) };
        }
        return null;
      }

      var isDark = false;
      var targets = [document.body, document.documentElement];
      var rgb = null;
      for (var i = 0; i < targets.length; i++) {
        if (!targets[i]) continue;
        var cs = window.getComputedStyle(targets[i]);
        if (cs) {
          rgb = parseRgb(cs.backgroundColor);
          if (rgb && (rgb.r > 0 || rgb.g > 0 || rgb.b > 0)) break;
        }
      }

      if (rgb) {
        var lum = 0.299 * rgb.r + 0.587 * rgb.g + 0.114 * rgb.b;
        if (lum < 65) isDark = true;
      } else {
        var csDoc = window.getComputedStyle(document.documentElement);
        if (csDoc) {
          var docRgb = parseRgb(csDoc.backgroundColor);
          if (docRgb && (0.299 * docRgb.r + 0.587 * docRgb.g + 0.114 * docRgb.b) < 65) {
            isDark = true;
          } else if (csDoc.colorScheme === "dark") {
            isDark = true;
          }
        }
      }

      var style = document.createElement("style");
      style.id = "__peek_invert_dark__";

      if (isDark) {
        // Site already has a dark theme (e.g. YouTube in dark theme). Do not invert!
        // Enforce native dark color scheme cleanly without color inversion.
        style.textContent = `
          html, body {
            color-scheme: dark !important;
          }
        `;
        document.documentElement.appendChild(style);
        send({ type: "invertDarkState", active: true, message: "Native dark theme active (site is already dark)" });
        return true;
      }

      // Light website: apply calibrated soft inversion curve and re-invert ALL media including iframes & video embeds
      style.textContent = `
        html {
          filter: invert(90%) hue-rotate(180deg) contrast(94%) !important;
          background: #121214 !important;
        }
        img, picture, video, canvas, iframe, embed, object {
          filter: invert(100%) hue-rotate(180deg) contrast(106%) !important;
        }
        picture img, iframe img {
          filter: none !important;
        }
      `;
      document.documentElement.appendChild(style);
      send({ type: "invertDarkState", active: true, message: "Night mode enabled" });
      return true;
    } catch (e) {
      return false;
    }
  };

  window.__peekTogglePiP = function () {
    try {
      var vids = Array.from(document.querySelectorAll("video"));
      var target = vids.find(function (v) { return !v.paused; }) || vids[0];
      if (!target) return false;
      if (document.pictureInPictureElement) {
        document.exitPictureInPicture();
        return false;
      } else if (target.requestPictureInPicture) {
        target.requestPictureInPicture();
        return true;
      }
    } catch (e) {
      return false;
    }
  };

  window.__peekZoomLevel = 1.0;
  window.__peekZoom = function (delta, reset) {
    try {
      if (reset) {
        window.__peekZoomLevel = 1.0;
      } else {
        window.__peekZoomLevel = Math.max(0.25, Math.min(3.0, window.__peekZoomLevel + delta));
      }
      document.body.style.zoom = window.__peekZoomLevel;
      return Math.round(window.__peekZoomLevel * 100);
    } catch (e) {
      return 100;
    }
  };
})();
"##;
