//! Uniform-grid spatial hash for fast AABB queries.
//!
//! Used internally by `CanvasKit` for O(1) point hit testing and
//! O(cells) range queries (marquee selection). Replaces the previous
//! linear scan over `Vec<HitRegion>`.

use std::collections::{HashMap, HashSet};

use blinc_core::layer::{Point, Rect};

use crate::hit::HitRegion;

/// Uniform-grid spatial hash for canvas hit regions.
///
/// Elements are inserted by bounding box and hashed into fixed-size cells.
/// Point queries check only the relevant cell; range queries check all
/// overlapping cells with deduplication.
#[derive(Clone, Debug)]
pub struct SpatialIndex {
    cell_size: f32,
    cells: HashMap<(i32, i32), Vec<usize>>,
    regions: Vec<HitRegion>,
}

impl Default for SpatialIndex {
    fn default() -> Self {
        Self::new(100.0)
    }
}

impl SpatialIndex {
    /// Create a new spatial index with the given cell size.
    ///
    /// Cell size should be roughly the average element size in content-space.
    /// Default: 100.0.
    pub fn new(cell_size: f32) -> Self {
        Self {
            cell_size: cell_size.max(1.0),
            cells: HashMap::new(),
            regions: Vec::new(),
        }
    }

    /// Clear all entries. Called at `begin_frame()`.
    pub fn clear(&mut self) {
        self.cells.clear();
        self.regions.clear();
    }

    /// Insert a hit region into the spatial index.
    pub fn insert(&mut self, region: HitRegion) {
        let idx = self.regions.len();
        let (col_min, row_min, col_max, row_max) = self.cell_range(&region.rect);
        for col in col_min..=col_max {
            for row in row_min..=row_max {
                self.cells.entry((col, row)).or_default().push(idx);
            }
        }
        self.regions.push(region);
    }

    /// Point query: find the topmost region containing the point.
    ///
    /// Returns regions in reverse insertion order (last inserted = topmost),
    /// matching the existing `hit_test` convention.
    pub fn hit_test(&self, point: Point) -> Option<String> {
        let col = (point.x / self.cell_size).floor() as i32;
        let row = (point.y / self.cell_size).floor() as i32;

        if let Some(indices) = self.cells.get(&(col, row)) {
            // Reverse iteration: topmost (last inserted) wins
            for &idx in indices.iter().rev() {
                if self.regions[idx].rect.contains(point) {
                    return Some(self.regions[idx].id.clone());
                }
            }
        }
        None
    }

    /// Range query: find all region IDs whose bounding box intersects the query rect.
    ///
    /// Used for marquee selection.
    pub fn query_rect(&self, query: &Rect) -> HashSet<String> {
        let (col_min, row_min, col_max, row_max) = self.cell_range(query);
        let mut result = HashSet::new();
        let mut seen = HashSet::new();

        for col in col_min..=col_max {
            for row in row_min..=row_max {
                if let Some(indices) = self.cells.get(&(col, row)) {
                    for &idx in indices {
                        if seen.insert(idx) && self.regions[idx].rect.intersects(query) {
                            result.insert(self.regions[idx].id.clone());
                        }
                    }
                }
            }
        }
        result
    }

    /// Number of registered regions.
    pub fn len(&self) -> usize {
        self.regions.len()
    }

    /// Whether the index is empty.
    pub fn is_empty(&self) -> bool {
        self.regions.is_empty()
    }

    /// Compute the cell range (col_min, row_min, col_max, row_max) for a rect.
    fn cell_range(&self, rect: &Rect) -> (i32, i32, i32, i32) {
        let x_min = rect.x();
        let y_min = rect.y();
        let x_max = rect.x() + rect.width();
        let y_max = rect.y() + rect.height();
        (
            (x_min / self.cell_size).floor() as i32,
            (y_min / self.cell_size).floor() as i32,
            (x_max / self.cell_size).floor() as i32,
            (y_max / self.cell_size).floor() as i32,
        )
    }
}

// ── Tests ───────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn make_region(id: &str, x: f32, y: f32, w: f32, h: f32) -> HitRegion {
        HitRegion::new(id, Rect::new(x, y, w, h))
    }

    #[test]
    fn test_empty_index() {
        let idx = SpatialIndex::new(100.0);
        assert!(idx.is_empty());
        assert!(idx.hit_test(Point::new(50.0, 50.0)).is_none());
        assert!(idx
            .query_rect(&Rect::new(0.0, 0.0, 100.0, 100.0))
            .is_empty());
    }

    #[test]
    fn test_single_insert_and_hit() {
        let mut idx = SpatialIndex::new(100.0);
        idx.insert(make_region("a", 10.0, 10.0, 50.0, 50.0));
        assert_eq!(idx.len(), 1);
        assert_eq!(idx.hit_test(Point::new(20.0, 20.0)), Some("a".into()));
        assert!(idx.hit_test(Point::new(100.0, 100.0)).is_none());
    }

    #[test]
    fn test_topmost_wins() {
        let mut idx = SpatialIndex::new(100.0);
        idx.insert(make_region("bottom", 0.0, 0.0, 100.0, 100.0));
        idx.insert(make_region("top", 20.0, 20.0, 60.0, 60.0));
        // Point inside both — "top" (last inserted) wins
        assert_eq!(idx.hit_test(Point::new(30.0, 30.0)), Some("top".into()));
        // Point only in bottom
        assert_eq!(idx.hit_test(Point::new(5.0, 5.0)), Some("bottom".into()));
    }

    #[test]
    fn test_negative_coordinates() {
        let mut idx = SpatialIndex::new(100.0);
        idx.insert(make_region("neg", -150.0, -150.0, 100.0, 100.0));
        assert_eq!(idx.hit_test(Point::new(-100.0, -100.0)), Some("neg".into()));
        assert!(idx.hit_test(Point::new(0.0, 0.0)).is_none());
    }

    #[test]
    fn test_large_region_spans_cells() {
        let mut idx = SpatialIndex::new(50.0);
        // Region spans 4 cells (200x200 with cell_size 50)
        idx.insert(make_region("big", 0.0, 0.0, 200.0, 200.0));
        assert_eq!(idx.hit_test(Point::new(10.0, 10.0)), Some("big".into()));
        assert_eq!(idx.hit_test(Point::new(150.0, 150.0)), Some("big".into()));
    }

    #[test]
    fn test_query_rect_basic() {
        let mut idx = SpatialIndex::new(100.0);
        idx.insert(make_region("a", 10.0, 10.0, 30.0, 30.0));
        idx.insert(make_region("b", 200.0, 200.0, 30.0, 30.0));
        idx.insert(make_region("c", 50.0, 50.0, 30.0, 30.0));

        let hits = idx.query_rect(&Rect::new(0.0, 0.0, 100.0, 100.0));
        assert!(hits.contains("a"));
        assert!(hits.contains("c"));
        assert!(!hits.contains("b"));
    }

    #[test]
    fn test_query_rect_no_duplicates() {
        let mut idx = SpatialIndex::new(50.0);
        // Region spans multiple cells
        idx.insert(make_region("wide", 0.0, 0.0, 200.0, 200.0));

        let hits = idx.query_rect(&Rect::new(10.0, 10.0, 180.0, 180.0));
        assert_eq!(hits.len(), 1);
        assert!(hits.contains("wide"));
    }

    #[test]
    fn test_clear() {
        let mut idx = SpatialIndex::new(100.0);
        idx.insert(make_region("a", 10.0, 10.0, 30.0, 30.0));
        assert_eq!(idx.len(), 1);
        idx.clear();
        assert!(idx.is_empty());
        assert!(idx.hit_test(Point::new(20.0, 20.0)).is_none());
    }

    #[test]
    fn test_many_regions() {
        let mut idx = SpatialIndex::new(100.0);
        for i in 0..100 {
            let x = (i % 10) as f32 * 50.0;
            let y = (i / 10) as f32 * 50.0;
            idx.insert(make_region(&format!("r{i}"), x, y, 40.0, 40.0));
        }
        assert_eq!(idx.len(), 100);

        // Hit test specific region
        assert_eq!(idx.hit_test(Point::new(10.0, 10.0)), Some("r0".into()));
        // Miss
        assert!(idx.hit_test(Point::new(1000.0, 1000.0)).is_none());

        // Range query
        let hits = idx.query_rect(&Rect::new(0.0, 0.0, 100.0, 100.0));
        assert!(hits.contains("r0"));
        assert!(hits.contains("r1"));
    }
}
