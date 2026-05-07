//! Snap-to-grid controller for content-space coordinates.
//!
//! Rounds points to the nearest grid intersection when enabled.
//!
//! # Usage
//!
//! ```ignore
//! let kit = CanvasKit::new("editor").with_snap(25.0);
//!
//! kit.on_element_drag(|evt| {
//!     let new_pos = base_pos + evt.content_delta;
//!     element.set_pos(kit.snap_point(new_pos));
//! });
//! ```

use blinc_core::layer::Point;

/// Snap-to-grid controller.
///
/// Apply to target positions (not deltas) for clean grid alignment.
#[derive(Clone, Debug)]
pub struct SnapController {
    /// Whether snapping is enabled.
    pub enabled: bool,
    /// Grid spacing in content-space units.
    pub spacing: f32,
}

impl Default for SnapController {
    fn default() -> Self {
        Self::disabled()
    }
}

impl SnapController {
    /// Create a disabled snap controller (default spacing 10.0).
    pub fn disabled() -> Self {
        Self {
            enabled: false,
            spacing: 10.0,
        }
    }

    /// Create an enabled snap controller with the given spacing.
    pub fn new(spacing: f32) -> Self {
        Self {
            enabled: true,
            spacing: spacing.max(1.0),
        }
    }

    /// Snap a content-space point to the nearest grid intersection.
    ///
    /// Returns the point unchanged if snapping is disabled.
    pub fn snap_point(&self, pt: Point) -> Point {
        if !self.enabled || self.spacing <= 0.0 {
            return pt;
        }
        Point::new(
            (pt.x / self.spacing).round() * self.spacing,
            (pt.y / self.spacing).round() * self.spacing,
        )
    }
}

// ── Tests ───────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_snap_disabled() {
        let snap = SnapController::disabled();
        let pt = Point::new(17.3, 42.8);
        let result = snap.snap_point(pt);
        assert_eq!(result.x, 17.3);
        assert_eq!(result.y, 42.8);
    }

    #[test]
    fn test_snap_enabled() {
        let snap = SnapController::new(10.0);
        assert_eq!(
            snap.snap_point(Point::new(13.0, 27.0)),
            Point::new(10.0, 30.0)
        );
        assert_eq!(
            snap.snap_point(Point::new(15.0, 25.0)),
            Point::new(20.0, 30.0)
        );
    }

    #[test]
    fn test_snap_exact() {
        let snap = SnapController::new(25.0);
        assert_eq!(
            snap.snap_point(Point::new(50.0, 75.0)),
            Point::new(50.0, 75.0)
        );
    }

    #[test]
    fn test_snap_negative() {
        let snap = SnapController::new(10.0);
        assert_eq!(
            snap.snap_point(Point::new(-13.0, -27.0)),
            Point::new(-10.0, -30.0)
        );
    }

    #[test]
    fn test_snap_minimum_spacing() {
        let snap = SnapController::new(0.5);
        // Clamped to 1.0
        assert_eq!(snap.spacing, 1.0);
    }
}
