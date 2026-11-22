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
      if (/^[1-9]$/.test(key)) {
        ev.preventDefault();
        send({ type: "shortcut", shortcut: { selectTab: Number(key) } });
        return;
      }
    }
    if (mod && ev.shiftKey && !ev.altKey) {
      if (key === "[" || key === "{") { ev.preventDefault(); send({ type: "shortcut", shortcut: "openTabsDrawer" }); return; }
      if (key === "]" || key === "}") { ev.preventDefault(); send({ type: "shortcut", shortcut: "openSettingsDrawer" }); return; }
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

  window.addEventListener("mousemove", function (ev) {
    if (ev.buttons !== 0) return;
    var atLeft = ev.clientX <= EDGE_ZONE;
    var atRight = (window.innerWidth - ev.clientX) <= EDGE_ZONE;
    if (!atLeft && !atRight) {
      if (edgeTimer) { clearTimeout(edgeTimer); edgeTimer = null; }
      return;
    }
    if (edgeTimer) return;
    var side = atLeft ? "left" : "right";
    edgeTimer = setTimeout(function () {
      edgeTimer = null;
      var now = Date.now();
      if (now - lastEdge < 800) return;
      lastEdge = now;
      send({ type: "edgeProximity", side: side, distance: 0 });
    }, 150);
  }, { passive: true });

  window.addEventListener("mouseleave", function () {
    if (edgeTimer) { clearTimeout(edgeTimer); edgeTimer = null; }
  });

  var lastWheelNav = 0;
  window.addEventListener("wheel", function (ev) {
    if (Math.abs(ev.deltaX) > 35 && Math.abs(ev.deltaY) < 25) {
      var now = Date.now();
      if (now - lastWheelNav < 700) return;
      var atLeftEdge = ev.clientX <= EDGE_ZONE + 40;
      var atRightEdge = (window.innerWidth - ev.clientX) <= EDGE_ZONE + 40;
      if (atLeftEdge && ev.deltaX < -25) {
        lastWheelNav = now;
        send({ type: "edgeProximity", side: "left", distance: 0 });
      } else if (atRightEdge && ev.deltaX > 25) {
        lastWheelNav = now;
        send({ type: "edgeProximity", side: "right", distance: 0 });
      }
    }
  }, { passive: true });
})();
"##;
