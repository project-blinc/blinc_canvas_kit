use blinc_core::layer::{Point, Rect};

/// Content-space bounding box registered as an interactive region.
///
/// Register during the draw callback via `kit.hit_rect(id, rect)`.
/// Z-order matches draw order: last registered = topmost.
#[derive(Clone, Debug)]
pub struct HitRegion {
    pub id: String,
    pub rect: Rect,
}

impl HitRegion {
    pub fn new(id: impl Into<String>, rect: Rect) -> Self {
        Self {
            id: id.into(),
            rect,
        }
    }
}

/// Tracks current pointer interaction state.
#[derive(Clone, Debug, Default)]
pub struct InteractionState {
    /// Region currently under the pointer (hover).
    pub hovered: Option<String>,
    /// Region being dragged (set on POINTER_DOWN over a region).
    pub active: Option<String>,
    /// Content-space point where the current drag started.
    pub drag_start: Option<Point>,
    /// Whether a DRAG event has fired since POINTER_DOWN (distinguishes click from drag).
    pub did_drag: bool,
}

/// Event passed to click and hover callbacks.
#[derive(Clone, Debug)]
pub struct CanvasEvent {
    /// Mouse position in content-space.
    pub content_point: Point,
    /// Mouse position in screen-space.
    pub screen_point: Point,
    /// Hit region ID, or `None` if the pointer is over the background.
    pub region_id: Option<String>,
}

/// Event passed to element drag callbacks.
#[derive(Clone, Debug)]
pub struct CanvasDragEvent {
    /// Current mouse position in content-space.
    pub content_point: Point,
    /// Drag delta in content-space since last event.
    pub content_delta: Point,
    /// Current mouse position in screen-space.
    pub screen_point: Point,
    /// The region being dragged.
    pub region_id: String,
}

/// Hit-test a point against regions in reverse order (topmost first).
///
/// Returns the ID of the first region whose bounding rect contains the point.
pub fn hit_test(regions: &[HitRegion], content_point: Point) -> Option<String> {
    regions
        .iter()
        .rev()
        .find(|r| r.rect.contains(content_point))
        .map(|r| r.id.clone())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hit_test_empty() {
        assert!(hit_test(&[], Point::new(0.0, 0.0)).is_none());
    }

    #[test]
    fn test_hit_test_single_hit() {
        let regions = vec![HitRegion::new("a", Rect::new(10.0, 10.0, 50.0, 50.0))];
        assert_eq!(hit_test(&regions, Point::new(20.0, 20.0)), Some("a".into()));
    }

    #[test]
    fn test_hit_test_miss() {
        let regions = vec![HitRegion::new("a", Rect::new(10.0, 10.0, 50.0, 50.0))];
        assert!(hit_test(&regions, Point::new(100.0, 100.0)).is_none());
    }

    #[test]
    fn test_hit_test_topmost_wins() {
        let regions = vec![
            HitRegion::new("bottom", Rect::new(0.0, 0.0, 100.0, 100.0)),
            HitRegion::new("top", Rect::new(20.0, 20.0, 60.0, 60.0)),
        ];
        // Point inside both — "top" (last registered) wins
        assert_eq!(
            hit_test(&regions, Point::new(30.0, 30.0)),
            Some("top".into())
        );
    }

    #[test]
    fn test_hit_test_non_overlapping() {
        let regions = vec![
            HitRegion::new("left", Rect::new(0.0, 0.0, 50.0, 50.0)),
            HitRegion::new("right", Rect::new(100.0, 0.0, 50.0, 50.0)),
        ];
        assert_eq!(
            hit_test(&regions, Point::new(25.0, 25.0)),
            Some("left".into())
        );
        assert_eq!(
            hit_test(&regions, Point::new(125.0, 25.0)),
            Some("right".into())
        );
        assert!(hit_test(&regions, Point::new(75.0, 25.0)).is_none());
    }
}
