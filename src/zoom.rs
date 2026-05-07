use blinc_core::layer::Point;

use crate::viewport::CanvasViewport;

/// Zoom controller that processes scroll and pinch events.
#[derive(Clone, Debug)]
pub struct ZoomController {
    /// Zoom speed multiplier for scroll wheel
    pub scroll_sensitivity: f32,
    /// Zoom speed multiplier for pinch gestures
    pub pinch_sensitivity: f32,
}

impl Default for ZoomController {
    fn default() -> Self {
        Self::new()
    }
}

impl ZoomController {
    pub fn new() -> Self {
        Self {
            scroll_sensitivity: 0.001,
            pinch_sensitivity: 1.0,
        }
    }

    /// Configure scroll sensitivity (default 0.001).
    pub fn with_scroll_sensitivity(mut self, sensitivity: f32) -> Self {
        self.scroll_sensitivity = sensitivity;
        self
    }

    /// Configure pinch sensitivity (default 1.0).
    pub fn with_pinch_sensitivity(mut self, sensitivity: f32) -> Self {
        self.pinch_sensitivity = sensitivity;
        self
    }

    /// Handle SCROLL event — zoom in/out centered on cursor.
    ///
    /// `scroll_delta_y`: raw scroll delta (positive = scroll down = zoom out by convention).
    /// `cursor`: screen-space cursor position to zoom toward.
    pub fn on_scroll(&self, viewport: &mut CanvasViewport, scroll_delta_y: f32, cursor: Point) {
        // Negate so scroll-up zooms in
        let factor = 1.0 - scroll_delta_y * self.scroll_sensitivity;
        let factor = factor.clamp(0.5, 2.0); // Prevent extreme jumps from large deltas
        viewport.zoom_at(cursor, factor);
    }

    /// Handle PINCH event — zoom centered on pinch midpoint.
    ///
    /// `pinch_scale`: scale ratio delta (>1 = zoom in, <1 = zoom out).
    /// `pinch_center`: screen-space center of the pinch gesture.
    pub fn on_pinch(&self, viewport: &mut CanvasViewport, pinch_scale: f32, pinch_center: Point) {
        let factor = 1.0 + (pinch_scale - 1.0) * self.pinch_sensitivity;
        viewport.zoom_at(pinch_center, factor);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_zoom_controller_default() {
        let zc = ZoomController::new();
        assert!((zc.scroll_sensitivity - 0.001).abs() < 1e-6);
        assert!((zc.pinch_sensitivity - 1.0).abs() < 1e-6);
    }

    #[test]
    fn test_scroll_zoom_in() {
        let zc = ZoomController::new();
        let mut vp = CanvasViewport::new();
        let cursor = Point::new(400.0, 300.0);

        let zoom_before = vp.zoom;
        // Negative scroll_delta_y = scroll up = zoom in
        zc.on_scroll(&mut vp, -100.0, cursor);
        assert!(vp.zoom > zoom_before);
    }

    #[test]
    fn test_scroll_zoom_out() {
        let zc = ZoomController::new();
        let mut vp = CanvasViewport::new();
        let cursor = Point::new(400.0, 300.0);

        let zoom_before = vp.zoom;
        // Positive scroll_delta_y = scroll down = zoom out
        zc.on_scroll(&mut vp, 100.0, cursor);
        assert!(vp.zoom < zoom_before);
    }

    #[test]
    fn test_pinch_zoom_in() {
        let zc = ZoomController::new();
        let mut vp = CanvasViewport::new();
        let center = Point::new(400.0, 300.0);

        let zoom_before = vp.zoom;
        zc.on_pinch(&mut vp, 1.5, center);
        assert!(vp.zoom > zoom_before);
    }

    #[test]
    fn test_zoom_respects_bounds() {
        let zc = ZoomController::new();
        let mut vp = CanvasViewport::new();
        vp.min_zoom = 0.5;
        vp.max_zoom = 3.0;
        let cursor = Point::new(0.0, 0.0);

        // Zoom way in
        for _ in 0..100 {
            zc.on_scroll(&mut vp, -200.0, cursor);
        }
        assert!(vp.zoom <= vp.max_zoom);

        // Zoom way out
        for _ in 0..100 {
            zc.on_scroll(&mut vp, 200.0, cursor);
        }
        assert!(vp.zoom >= vp.min_zoom);
    }
}
