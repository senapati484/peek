use include_dir::{include_dir, Dir};

use peek_engine::{Asset, AssetProvider};

static UI_DIST: Dir<'_> = include_dir!("$CARGO_MANIFEST_DIR/../../ui/dist");

/// Serves the embedded Svelte bundle over `peek://`. Only local assets are
/// ever reachable, which is what keeps the UI webview privileged.
pub struct UiAssets;

impl AssetProvider for UiAssets {
    fn get(&self, path: &str) -> Option<Asset> {
        let rel = path.trim_start_matches('/');
        if rel.is_empty() || rel.contains("..") {
            return self.get("/index.html");
        }
        if let Some(file) = UI_DIST.get_file(rel) {
            return Some(Asset::new(mime_for(rel), file.contents()));
        }
        // Hash routes (`#bar`, `#drawer`) resolve to the same document.
        if !rel.contains('.') {
            return UI_DIST
                .get_file("index.html")
                .map(|f| Asset::new("text/html; charset=utf-8", f.contents()));
        }
        None
    }
}

/// Minimal provider for page webviews. Only the local new-tab page is
/// reachable; the privileged bundle is not exposed to remote content.
pub struct PageAssets;

impl AssetProvider for PageAssets {
    fn get(&self, path: &str) -> Option<Asset> {
        match path {
            "/blank.html" | "/" => UI_DIST
                .get_file("blank.html")
                .map(|f| Asset::new("text/html; charset=utf-8", f.contents())),
            "/logo.png" => UI_DIST
                .get_file("logo.png")
                .map(|f| Asset::new("image/png", f.contents())),
            "/favicon.png" => UI_DIST
                .get_file("favicon.png")
                .map(|f| Asset::new("image/png", f.contents())),
            _ => None,
        }
    }
}


fn mime_for(path: &str) -> &'static str {
    match path.rsplit('.').next().unwrap_or("") {
        "html" => "text/html; charset=utf-8",
        "js" | "mjs" => "text/javascript; charset=utf-8",
        "css" => "text/css; charset=utf-8",
        "json" => "application/json",
        "svg" => "image/svg+xml",
        "png" => "image/png",
        "jpg" | "jpeg" => "image/jpeg",
        "webp" => "image/webp",
        "ico" => "image/x-icon",
        "woff" => "font/woff",
        "woff2" => "font/woff2",
        "wasm" => "application/wasm",
        _ => "application/octet-stream",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn serves_index_and_blocks_traversal() {
        let assets = UiAssets;
        assert!(assets.get("/index.html").is_some());
        assert!(assets.get("/").is_some());
        assert!(assets.get("/../Cargo.toml").is_some());
        assert_eq!(assets.get("/../Cargo.toml").unwrap().mime, "text/html; charset=utf-8");
        assert!(assets.get("/missing.js").is_none());
    }

    #[test]
    fn mime_types() {
        assert_eq!(mime_for("a.css"), "text/css; charset=utf-8");
        assert_eq!(mime_for("a.wasm"), "application/wasm");
        assert_eq!(mime_for("a.unknown"), "application/octet-stream");
    }
}
