//! The `Engine` trait and its `wry` backend.
//!
//! Peek uses the operating system web engine (WKWebView / WebView2 /
//! WebKitGTK) through `wry`. All webviews are children of a single `tao`
//! window; this crate is the only place that knows about `wry` types.

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

pub use wry::cookie;
#[cfg(any(target_os = "macos", target_os = "ios"))]
use wry::WebViewBuilderExtDarwin;
use tao::window::Window;
use wry::dpi::{LogicalPosition, LogicalSize};
use wry::{NewWindowResponse, PageLoadEvent, Rect, WebContext, WebView, WebViewBuilder};

pub type WebViewId = u64;

#[derive(Debug, thiserror::Error)]
pub enum EngineError {
    #[error("webview error: {0}")]
    Wry(#[from] wry::Error),
    #[error("webview {0} does not exist")]
    Missing(WebViewId),
}

pub type Result<T> = std::result::Result<T, EngineError>;

/// A rectangle in logical (DPI-independent) pixels, relative to the window.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Bounds {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}

impl Bounds {
    pub fn new(x: f64, y: f64, width: f64, height: f64) -> Self {
        Self {
            x,
            y,
            width: width.max(0.0),
            height: height.max(0.0),
        }
    }

    fn to_wry(self) -> Rect {
        Rect {
            position: LogicalPosition::new(self.x, self.y).into(),
            size: LogicalSize::new(self.width, self.height).into(),
        }
    }
}

/// A file served over the `peek://` scheme.
#[derive(Debug, Clone)]
pub struct Asset {
    pub mime: &'static str,
    pub body: Vec<u8>,
}

impl Asset {
    pub fn new(mime: &'static str, body: impl Into<Vec<u8>>) -> Self {
        Self {
            mime,
            body: body.into(),
        }
    }
}

/// Supplies local assets to privileged (`peek://`) webviews.
pub trait AssetProvider: Send + Sync + 'static {
    /// `path` always begins with `/`. Returns `None` for a missing asset.
    fn get(&self, path: &str) -> Option<Asset>;
}

impl<F> AssetProvider for F
where
    F: Fn(&str) -> Option<Asset> + Send + Sync + 'static,
{
    fn get(&self, path: &str) -> Option<Asset> {
        self(path)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LoadEvent {
    Started,
    Finished,
}

/// Callbacks fired by webviews. Implementations must be cheap and must not
/// block: they run on the event-loop thread.
pub trait EngineEvents: Send + Sync + 'static {
    fn title_changed(&self, id: WebViewId, title: String);
    fn url_changed(&self, id: WebViewId, url: String);
    fn page_load(&self, id: WebViewId, event: LoadEvent, url: String);
    /// Raw `window.ipc.postMessage` payload from injected scripts.
    fn ipc(&self, id: WebViewId, message: String);
    /// A page requested a new window (`target=_blank`, `window.open`).
    /// The host decides whether to open a tab; the OS window is always denied.
    fn new_window_requested(&self, id: WebViewId, url: String);
}

pub struct WebViewOptions {
    pub bounds: Bounds,
    pub visible: bool,
    pub transparent: bool,
    /// Allow media to autoplay without a user gesture. Off for previews.
    pub autoplay: bool,
    /// Ephemeral cookie/store session (preview default).
    pub incognito: bool,
    pub devtools: bool,
    /// macOS: clicking an inactive window also clicks the webview.
    pub accept_first_mouse: bool,
    pub initialization_scripts: Vec<String>,
    pub url: Option<String>,
    pub html: Option<String>,
    /// When set, registers the `peek` scheme backed by this provider.
    pub assets: Option<Arc<dyn AssetProvider>>,
    pub zoom: Option<f64>,
    pub user_agent: Option<String>,
    pub data_directory: Option<PathBuf>,
    pub data_store_identifier: Option<[u8; 16]>,
    pub corner_radius: Option<f64>,
    pub border_width: Option<f64>,
    pub border_color: Option<[f64; 4]>,
}

impl Default for WebViewOptions {
    fn default() -> Self {
        Self {
            bounds: Bounds::new(0.0, 0.0, 100.0, 100.0),
            visible: true,
            transparent: false,
            autoplay: false,
            incognito: false,
            devtools: cfg!(debug_assertions),
            accept_first_mouse: false,
            initialization_scripts: Vec::new(),
            url: None,
            html: None,
            assets: None,
            zoom: None,
            user_agent: None,
            data_directory: None,
            data_store_identifier: None,
            corner_radius: None,
            border_width: None,
            border_color: None,
        }
    }
}

/// Abstraction over a browser engine so a second backend (CEF) can be added
/// later without touching `peek-core`.
pub trait Engine {
    fn create(&mut self, window: &Window, options: WebViewOptions) -> Result<WebViewId>;
    fn destroy(&mut self, id: WebViewId);
    fn exists(&self, id: WebViewId) -> bool;
    fn count(&self) -> usize;
    fn set_bounds(&self, id: WebViewId, bounds: Bounds) -> Result<()>;
    fn set_visible(&self, id: WebViewId, visible: bool) -> Result<()>;
    fn load_url(&self, id: WebViewId, url: &str) -> Result<()>;
    fn reload(&self, id: WebViewId) -> Result<()>;
    fn go_back(&self, id: WebViewId) -> Result<()>;
    fn go_forward(&self, id: WebViewId) -> Result<()>;
    fn can_go_back(&self, id: WebViewId) -> bool;
    fn can_go_forward(&self, id: WebViewId) -> bool;
    fn url(&self, id: WebViewId) -> Option<String>;
    fn evaluate(&self, id: WebViewId, js: &str) -> Result<()>;
    fn focus(&self, id: WebViewId) -> Result<()>;
    fn set_zoom(&self, id: WebViewId, zoom: f64) -> Result<()>;
    fn cookies(&self, id: WebViewId) -> Result<Vec<cookie::Cookie<'static>>>;
    fn set_cookie(&self, id: WebViewId, cookie: &cookie::Cookie<'_>) -> Result<()>;
}

pub struct WryEngine {
    next_id: WebViewId,
    views: HashMap<WebViewId, WebView>,
    web_contexts: HashMap<PathBuf, WebContext>,
    events: Arc<dyn EngineEvents>,
}

impl WryEngine {
    pub fn new(events: Arc<dyn EngineEvents>) -> Self {
        Self {
            next_id: 1,
            views: HashMap::new(),
            web_contexts: HashMap::new(),
            events,
        }
    }

    fn view(&self, id: WebViewId) -> Result<&WebView> {
        self.views.get(&id).ok_or(EngineError::Missing(id))
    }
}

impl Engine for WryEngine {
    fn create(&mut self, window: &Window, options: WebViewOptions) -> Result<WebViewId> {
        let id = self.next_id;
        self.next_id += 1;

        let builder = if let Some(ref data_dir) = options.data_directory {
            let context = self
                .web_contexts
                .entry(data_dir.clone())
                .or_insert_with(|| WebContext::new(Some(data_dir.clone())));
            WebViewBuilder::new_with_web_context(context)
        } else {
            WebViewBuilder::new()
        };

        let mut builder = builder
            .with_bounds(options.bounds.to_wry())
            .with_visible(options.visible)
            .with_transparent(options.transparent)
            .with_autoplay(options.autoplay)
            .with_incognito(options.incognito)
            .with_devtools(options.devtools)
            .with_accept_first_mouse(options.accept_first_mouse)
            .with_general_autofill_enabled(true)
            .with_back_forward_navigation_gestures(true);

        #[cfg(any(target_os = "macos", target_os = "ios"))]
        if let Some(store_id) = options.data_store_identifier {
            if !options.incognito {
                builder = builder.with_data_store_identifier(store_id);
            }
        }

        if let Some(ref ua) = options.user_agent {
            builder = builder.with_user_agent(ua);
        }

        for script in options.initialization_scripts {
            builder = builder.with_initialization_script(script);
        }

        if let Some(assets) = options.assets {
            builder = builder.with_custom_protocol("peek".into(), move |_id, request| {
                let path = request.uri().path();
                let path = if path.is_empty() || path == "/" {
                    "/index.html"
                } else {
                    path
                };
                match assets.get(path) {
                    Some(asset) => wry::http::Response::builder()
                        .status(200)
                        .header(wry::http::header::CONTENT_TYPE, asset.mime)
                        .header(wry::http::header::CACHE_CONTROL, "no-store")
                        .body(std::borrow::Cow::Owned(asset.body))
                        .unwrap_or_else(|_| empty_response(500)),
                    None => empty_response(404),
                }
            });
        }

        let ipc_events = self.events.clone();
        let title_events = self.events.clone();
        let load_events = self.events.clone();
        let nav_events = self.events.clone();
        let window_events = self.events.clone();
        builder = builder
            .with_ipc_handler(move |request| ipc_events.ipc(id, request.body().clone()))
            .with_document_title_changed_handler(move |title| title_events.title_changed(id, title))
            .with_on_page_load_handler(move |event, url| {
                let event = match event {
                    PageLoadEvent::Started => LoadEvent::Started,
                    PageLoadEvent::Finished => LoadEvent::Finished,
                };
                load_events.page_load(id, event, url);
            })
            .with_navigation_handler(move |url| {
                nav_events.url_changed(id, url);
                true
            })
            .with_new_window_req_handler(move |url, _features| {
                window_events.new_window_requested(id, url);
                NewWindowResponse::Deny
            });

        if let Some(html) = options.html {
            builder = builder.with_html(html);
        } else if let Some(url) = options.url {
            builder = builder.with_url(url);
        }

        let webview = builder.build_as_child(window)?;
        if let Some(zoom) = options.zoom {
            webview.zoom(zoom)?;
        }

        #[cfg(target_os = "macos")]
        if let Some(radius) = options.corner_radius {
            use objc2::msg_send;
            use wry::WebViewExtMacOS;
            let wk = webview.webview();
            unsafe {
                let () = msg_send![&*wk, setWantsLayer: true];
                let layer: *mut objc2::runtime::AnyObject = msg_send![&*wk, layer];
                if !layer.is_null() {
                    let () = msg_send![layer, setCornerRadius: radius];
                    let () = msg_send![layer, setMasksToBounds: true];
                    if let Some(width) = options.border_width {
                        let () = msg_send![layer, setBorderWidth: width];
                        let rgba = options.border_color.unwrap_or([0.35, 0.5, 0.95, 0.45]);
                        let color: *mut objc2::runtime::AnyObject = msg_send![
                            objc2::class!(NSColor),
                            colorWithRed: rgba[0],
                            green: rgba[1],
                            blue: rgba[2],
                            alpha: rgba[3]
                        ];
                        if !color.is_null() {
                            let cg_color: *mut objc2::runtime::AnyObject = msg_send![color, CGColor];
                            let () = msg_send![layer, setBorderColor: cg_color];
                        }
                    }
                }
            }
        }

        self.views.insert(id, webview);
        Ok(id)
    }

    fn destroy(&mut self, id: WebViewId) {
        self.views.remove(&id);
    }

    fn exists(&self, id: WebViewId) -> bool {
        self.views.contains_key(&id)
    }

    fn count(&self) -> usize {
        self.views.len()
    }

    fn set_bounds(&self, id: WebViewId, bounds: Bounds) -> Result<()> {
        self.view(id)?.set_bounds(bounds.to_wry())?;
        Ok(())
    }

    fn set_visible(&self, id: WebViewId, visible: bool) -> Result<()> {
        self.view(id)?.set_visible(visible)?;
        Ok(())
    }

    fn load_url(&self, id: WebViewId, url: &str) -> Result<()> {
        self.view(id)?.load_url(url)?;
        Ok(())
    }

    fn reload(&self, id: WebViewId) -> Result<()> {
        self.view(id)?.reload()?;
        Ok(())
    }

    fn go_back(&self, id: WebViewId) -> Result<()> {
        self.view(id)?.go_back()?;
        Ok(())
    }

    fn go_forward(&self, id: WebViewId) -> Result<()> {
        self.view(id)?.go_forward()?;
        Ok(())
    }

    fn can_go_back(&self, id: WebViewId) -> bool {
        self.views
            .get(&id)
            .and_then(|v| v.can_go_back().ok())
            .unwrap_or(false)
    }

    fn can_go_forward(&self, id: WebViewId) -> bool {
        self.views
            .get(&id)
            .and_then(|v| v.can_go_forward().ok())
            .unwrap_or(false)
    }

    fn url(&self, id: WebViewId) -> Option<String> {
        self.views.get(&id).and_then(|v| v.url().ok())
    }

    fn evaluate(&self, id: WebViewId, js: &str) -> Result<()> {
        self.view(id)?.evaluate_script(js)?;
        Ok(())
    }

    fn focus(&self, id: WebViewId) -> Result<()> {
        self.view(id)?.focus()?;
        Ok(())
    }

    fn set_zoom(&self, id: WebViewId, zoom: f64) -> Result<()> {
        self.view(id)?.zoom(zoom)?;
        Ok(())
    }

    fn cookies(&self, id: WebViewId) -> Result<Vec<cookie::Cookie<'static>>> {
        Ok(self.view(id)?.cookies().unwrap_or_default())
    }

    fn set_cookie(&self, id: WebViewId, cookie: &cookie::Cookie<'_>) -> Result<()> {
        let _ = self.view(id)?.set_cookie(cookie);
        Ok(())
    }
}

fn empty_response(status: u16) -> wry::http::Response<std::borrow::Cow<'static, [u8]>> {
    wry::http::Response::builder()
        .status(status)
        .body(std::borrow::Cow::Borrowed(&[][..]))
        .expect("static empty response is valid")
}
