//! Hover-preview policy: hover intent, placement and lifecycle state.
//!
//! The preview itself is a native child webview (an iframe would be blocked
//! by `X-Frame-Options` / `frame-ancestors`). This crate only decides *when*
//! and *where* it appears.

use std::time::Duration;

use peek_ipc::Rect;

#[derive(Debug, Clone)]
pub struct PreviewConfig {
    /// Dwell time before a hover becomes hover intent.
    pub hover_delay: Duration,
    /// Preview size in logical pixels.
    pub width: f64,
    pub height: f64,
    /// Page scale used to render a miniature.
    pub zoom: f64,
    /// Gap between the link and the preview.
    pub gap: f64,
    /// When true, only a modifier-forced hover opens a preview.
    pub modifier_only: bool,
}

impl Default for PreviewConfig {
    fn default() -> Self {
        Self {
            hover_delay: Duration::from_millis(450),
            width: 480.0,
            height: 320.0,
            zoom: 0.6,
            gap: 8.0,
            modifier_only: true,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum PreviewState {
    Idle,
    /// Hovering, waiting for the dwell timer.
    Pending { url: String, rect: Rect },
    /// The preview webview is alive.
    Visible { url: String },
}

impl Default for PreviewState {
    fn default() -> Self {
        Self::Idle
    }
}

#[derive(Debug, Default)]
pub struct PreviewController {
    state: PreviewState,
}

#[derive(Debug, Clone, PartialEq)]
pub enum PreviewAction {
    /// Create/show the preview webview at the computed bounds.
    Show {
        url: String,
        bounds: Rect,
        zoom: f64,
    },
    /// Destroy the preview webview.
    Hide,
    /// Do nothing yet.
    None,
}

impl PreviewController {
    pub fn state(&self) -> &PreviewState {
        &self.state
    }

    pub fn is_visible(&self) -> bool {
        matches!(self.state, PreviewState::Visible { .. })
    }

    /// Record a hover event. Returns a `Show` action only when hover intent
    /// has already been satisfied (`elapsed >= hover_delay`).
    pub fn hover(
        &mut self,
        url: String,
        rect: Rect,
        elapsed: Duration,
        forced: bool,
        config: &PreviewConfig,
        window: (f64, f64),
    ) -> PreviewAction {
        let intent = forced || elapsed >= config.hover_delay;
        if config.modifier_only && !forced {
            self.state = PreviewState::Pending { url, rect };
            return PreviewAction::None;
        }
        if !intent {
            self.state = PreviewState::Pending { url, rect };
            return PreviewAction::None;
        }
        if let PreviewState::Visible { url: current } = &self.state {
            if *current == url {
                return PreviewAction::None;
            }
        }
        let bounds = place_preview(rect, config, window);
        self.state = PreviewState::Visible { url: url.clone() };
        PreviewAction::Show {
            url,
            bounds,
            zoom: config.zoom,
        }
    }

    pub fn hover_end(&mut self) -> PreviewAction {
        if matches!(self.state, PreviewState::Idle) {
            return PreviewAction::None;
        }
        self.state = PreviewState::Idle;
        PreviewAction::Hide
    }

    pub fn dismiss(&mut self) -> PreviewAction {
        self.hover_end()
    }
}

/// Place the preview below the link when there is room, otherwise above.
/// Always clamp inside the window with `gap` of margin.
pub fn place_preview(link: Rect, config: &PreviewConfig, window: (f64, f64)) -> Rect {
    let (win_w, win_h) = window;
    let width = config.width.min((win_w - 2.0 * config.gap).max(1.0));
    let height = config.height.min((win_h - 2.0 * config.gap).max(1.0));

    let below = link.y + link.height + config.gap;
    let above = link.y - height - config.gap;
    let y = if below + height <= win_h - config.gap {
        below
    } else if above >= config.gap {
        above
    } else {
        (win_h - height) / 2.0
    };

    let x = link.x + link.width / 2.0 - width / 2.0;
    let max_x = (win_w - width - config.gap).max(config.gap);
    let x = x.clamp(config.gap.min(max_x), max_x);

    Rect {
        x,
        y: y.clamp(config.gap, (win_h - height - config.gap).max(config.gap)),
        width,
        height,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rect(x: f64, y: f64) -> Rect {
        Rect {
            x,
            y,
            width: 100.0,
            height: 20.0,
        }
    }

    #[test]
    fn waits_for_hover_delay() {
        let config = PreviewConfig {
            modifier_only: false,
            ..Default::default()
        };
        let mut c = PreviewController::default();
        let action = c.hover(
            "https://a".into(),
            rect(100.0, 100.0),
            Duration::from_millis(100),
            false,
            &config,
            (1280.0, 800.0),
        );
        assert_eq!(action, PreviewAction::None);
        assert!(matches!(c.state(), PreviewState::Pending { .. }));

        let action = c.hover(
            "https://a".into(),
            rect(100.0, 100.0),
            Duration::from_millis(500),
            false,
            &config,
            (1280.0, 800.0),
        );
        assert!(matches!(action, PreviewAction::Show { .. }));
        assert!(c.is_visible());
    }

    #[test]
    fn modifier_forces_immediately() {
        let config = PreviewConfig::default();
        let mut c = PreviewController::default();
        let action = c.hover(
            "https://a".into(),
            rect(10.0, 10.0),
            Duration::ZERO,
            true,
            &config,
            (1280.0, 800.0),
        );
        assert!(matches!(action, PreviewAction::Show { .. }));
    }

    #[test]
    fn modifier_only_blocks_plain_hover() {
        let config = PreviewConfig {
            modifier_only: true,
            ..Default::default()
        };
        let mut c = PreviewController::default();
        let action = c.hover(
            "https://a".into(),
            rect(10.0, 10.0),
            Duration::from_secs(10),
            false,
            &config,
            (1280.0, 800.0),
        );
        assert_eq!(action, PreviewAction::None);
    }

    #[test]
    fn repeated_hover_on_same_url_does_not_recreate() {
        let config = PreviewConfig::default();
        let mut c = PreviewController::default();
        c.hover(
            "https://a".into(),
            rect(10.0, 10.0),
            Duration::ZERO,
            true,
            &config,
            (1280.0, 800.0),
        );
        let action = c.hover(
            "https://a".into(),
            rect(10.0, 10.0),
            Duration::ZERO,
            true,
            &config,
            (1280.0, 800.0),
        );
        assert_eq!(action, PreviewAction::None);
    }

    #[test]
    fn hover_end_hides_once() {
        let config = PreviewConfig::default();
        let mut c = PreviewController::default();
        c.hover(
            "https://a".into(),
            rect(10.0, 10.0),
            Duration::ZERO,
            true,
            &config,
            (1280.0, 800.0),
        );
        assert_eq!(c.hover_end(), PreviewAction::Hide);
        assert_eq!(c.hover_end(), PreviewAction::None);
    }

    #[test]
    fn places_below_when_room_and_above_when_not() {
        let config = PreviewConfig::default();
        let below = place_preview(rect(600.0, 100.0), &config, (1280.0, 800.0));
        assert_eq!(below.y, 128.0);
        assert_eq!(below.width, 480.0);
        assert_eq!(below.height, 320.0);

        let above = place_preview(rect(600.0, 700.0), &config, (1280.0, 800.0));
        assert_eq!(above.y, 700.0 - 320.0 - 8.0);
    }

    #[test]
    fn clamps_inside_window() {
        let config = PreviewConfig::default();
        let placed = place_preview(rect(1270.0, 100.0), &config, (1280.0, 800.0));
        assert!(placed.x + placed.width <= 1280.0 - 8.0);
        assert!(placed.x >= 8.0);

        let small = place_preview(rect(0.0, 0.0), &config, (300.0, 200.0));
        assert!(small.width <= 300.0);
        assert!(small.height <= 200.0);
        assert!(small.x >= 0.0 && small.y >= 0.0);
    }
}
