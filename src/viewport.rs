use blinc_core::layer::{Affine2D, Point};

/// Canvas viewport state: pan offset + zoom level.
///
/// The viewport defines a content-space → screen-space mapping:
///   screen = scale(zoom) * translate(pan) * content
///
/// `screen_to_content()` inverts this for hit testing.
#[derive(Clone, Debug)]
pub struct CanvasViewport {
    /// Pan offset in content-space pixels
    pub pan_x: f32,
    pub pan_y: f32,
    /// Zoom level (1.0 = 100%, 2.0 = 200%)
    pub zoom: f32,
    /// Minimum zoom level
    pub min_zoom: f32,
    /// Maximum zoom level
    pub max_zoom: f32,
}

impl Default for CanvasViewport {
    fn default() -> Self {
        Self::new()
    }
}

impl CanvasViewport {
    pub fn new() -> Self {
        Self {
            pan_x: 0.0,
            pan_y: 0.0,
            zoom: 1.0,
            min_zoom: 0.1,
            max_zoom: 10.0,
        }
    }

    /// The combined affine transform: content-space → screen-space.
    ///
    /// Applies pan translation then zoom scaling.
    pub fn transform(&self) -> Affine2D {
        // scale(zoom) * translate(pan)
        let translate = Affine2D::translation(self.pan_x, self.pan_y);
        let scale = Affine2D::scale(self.zoom, self.zoom);
        scale.then(&translate)
    }

    /// Inverse transform: screen-space → content-space.
    ///
    /// Returns identity if the transform is singular (zoom ≈ 0).
    pub fn inverse_transform(&self) -> Affine2D {
        affine_inverse(&self.transform()).unwrap_or(Affine2D::IDENTITY)
    }

    /// Convert a screen-space point to content-space.
    pub fn screen_to_content(&self, screen: Point) -> Point {
        self.inverse_transform().transform_point(screen)
    }

    /// Convert a content-space point to screen-space.
    pub fn content_to_screen(&self, content: Point) -> Point {
        self.transform().transform_point(content)
    }

    /// Pan by a delta in screen pixels.
    ///
    /// The delta is divided by zoom so panning feels consistent at all zoom levels.
    pub fn pan_by(&mut self, dx: f32, dy: f32) {
        self.pan_x += dx / self.zoom;
        self.pan_y += dy / self.zoom;
    }

    /// Zoom centered on a screen-space point.
    ///
    /// Adjusts pan so the point under the cursor stays fixed in content-space.
    pub fn zoom_at(&mut self, screen_point: Point, factor: f32) {
        // Content point under cursor before zoom
        let content_before = self.screen_to_content(screen_point);

        // Apply zoom
        self.zoom = (self.zoom * factor).clamp(self.min_zoom, self.max_zoom);

        // Content point under cursor after zoom (with old pan)
        let content_after = self.screen_to_content(screen_point);

        // Adjust pan to keep content_before at the same screen position
        self.pan_x += content_after.x - content_before.x;
        self.pan_y += content_after.y - content_before.y;
    }

    /// Set zoom level, clamped to [min_zoom, max_zoom].
    pub fn set_zoom(&mut self, zoom: f32) {
        self.zoom = zoom.clamp(self.min_zoom, self.max_zoom);
    }

    /// Reset to identity (pan=0, zoom=1).
    pub fn reset(&mut self) {
        self.pan_x = 0.0;
        self.pan_y = 0.0;
        self.zoom = 1.0;
    }

    /// Check if a content-space rectangle is visible within the screen bounds.
    pub fn is_visible(
        &self,
        content_x: f32,
        content_y: f32,
        content_w: f32,
        content_h: f32,
        screen_w: f32,
        screen_h: f32,
    ) -> bool {
        let tl = self.content_to_screen(Point::new(content_x, content_y));
        let br = self.content_to_screen(Point::new(content_x + content_w, content_y + content_h));
        // AABB overlap test
        br.x > 0.0 && tl.x < screen_w && br.y > 0.0 && tl.y < screen_h
    }
}

/// Compute the inverse of an affine transform.
///
/// Returns `None` if the matrix is singular (determinant ≈ 0).
pub fn affine_inverse(affine: &Affine2D) -> Option<Affine2D> {
    let [a, b, c, d, tx, ty] = affine.elements;
    let det = a * d - c * b;
    if det.abs() < 1e-10 {
        return None;
    }
    let inv_det = 1.0 / det;
    Some(Affine2D {
        elements: [
            d * inv_det,
            -b * inv_det,
            -c * inv_det,
            a * inv_det,
            (c * ty - d * tx) * inv_det,
            (b * tx - a * ty) * inv_det,
        ],
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_affine_inverse_identity() {
        let inv = affine_inverse(&Affine2D::IDENTITY).unwrap();
        assert!((inv.elements[0] - 1.0).abs() < 1e-6);
        assert!((inv.elements[3] - 1.0).abs() < 1e-6);
    }

    #[test]
    fn test_affine_inverse_roundtrip() {
        let t = Affine2D::translation(50.0, -30.0);
        let s = Affine2D::scale(2.0, 0.5);
        let combined = s.then(&t);
        let inv = affine_inverse(&combined).unwrap();
        let result = combined.then(&inv);
        for i in 0..6 {
            let expected = Affine2D::IDENTITY.elements[i];
            assert!(
                (result.elements[i] - expected).abs() < 1e-4,
                "element {i}: {} != {}",
                result.elements[i],
                expected
            );
        }
    }

    #[test]
    fn test_affine_inverse_singular() {
        let singular = Affine2D {
            elements: [0.0, 0.0, 0.0, 0.0, 0.0, 0.0],
        };
        assert!(affine_inverse(&singular).is_none());
    }

    #[test]
    fn test_viewport_default() {
        let vp = CanvasViewport::new();
        assert_eq!(vp.pan_x, 0.0);
        assert_eq!(vp.pan_y, 0.0);
        assert_eq!(vp.zoom, 1.0);
    }

    #[test]
    fn test_coordinate_roundtrip() {
        let mut vp = CanvasViewport::new();
        vp.pan_x = 100.0;
        vp.pan_y = -50.0;
        vp.zoom = 2.0;

        let original = Point::new(42.0, 73.0);
        let screen = vp.content_to_screen(original);
        let back = vp.screen_to_content(screen);

        assert!((back.x - original.x).abs() < 1e-3);
        assert!((back.y - original.y).abs() < 1e-3);
    }

    #[test]
    fn test_zoom_at_keeps_point_fixed() {
        let mut vp = CanvasViewport::new();
        vp.pan_x = 50.0;
        vp.pan_y = 30.0;
        vp.zoom = 1.5;

        let cursor = Point::new(200.0, 150.0);
        let content_before = vp.screen_to_content(cursor);

        vp.zoom_at(cursor, 1.5);

        let content_after = vp.screen_to_content(cursor);
        assert!((content_after.x - content_before.x).abs() < 1e-2);
        assert!((content_after.y - content_before.y).abs() < 1e-2);
    }

    #[test]
    fn test_zoom_clamp() {
        let mut vp = CanvasViewport::new();
        vp.set_zoom(100.0);
        assert_eq!(vp.zoom, vp.max_zoom);

        vp.set_zoom(0.001);
        assert_eq!(vp.zoom, vp.min_zoom);
    }

    #[test]
    fn test_pan_by() {
        let mut vp = CanvasViewport::new();
        vp.zoom = 2.0;
        vp.pan_by(10.0, 20.0);
        // dx/zoom = 10/2 = 5, dy/zoom = 20/2 = 10
        assert!((vp.pan_x - 5.0).abs() < 1e-6);
        assert!((vp.pan_y - 10.0).abs() < 1e-6);
    }

    #[test]
    fn test_reset() {
        let mut vp = CanvasViewport::new();
        vp.pan_x = 100.0;
        vp.pan_y = -50.0;
        vp.zoom = 3.0;
        vp.reset();
        assert_eq!(vp.pan_x, 0.0);
        assert_eq!(vp.pan_y, 0.0);
        assert_eq!(vp.zoom, 1.0);
    }

    #[test]
    fn test_is_visible() {
        let vp = CanvasViewport::new();
        // Content rect at (100,100) 200x200, screen 800x600
        assert!(vp.is_visible(100.0, 100.0, 200.0, 200.0, 800.0, 600.0));
        // Content rect far off-screen
        assert!(!vp.is_visible(1000.0, 1000.0, 50.0, 50.0, 800.0, 600.0));
    }
}
