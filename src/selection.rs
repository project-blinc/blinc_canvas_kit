//! Multi-select and marquee selection for canvas elements.
//!
//! Provides `SelectionState` for tracking selected region IDs,
//! `MarqueeState` for active box-select drags, and `CanvasTool`
//! for switching between pan and selection modes.

use std::collections::HashSet;

use blinc_core::layer::{Point, Rect};

/// Tool mode for canvas interaction.
///
/// In `Pan` mode, background drag pans the viewport.
/// In `Select` mode, background drag draws a marquee selection rectangle.
/// Element drag always moves selected elements regardless of tool mode.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub enum CanvasTool {
    /// Background drag pans viewport. Shift+drag for marquee.
    #[default]
    Pan,
    /// Background drag draws marquee. Element drag moves selection.
    Select,
}

/// Selection state for multi-select.
#[derive(Clone, Debug, Default)]
pub struct SelectionState {
    /// Currently selected region IDs.
    pub selected: HashSet<String>,
    /// Active marquee rectangle, if a marquee drag is in progress.
    pub marquee: Option<MarqueeState>,
}

/// Active marquee (box-select) drag state.
#[derive(Clone, Debug)]
pub struct MarqueeState {
    /// Content-space anchor point (where drag started).
    pub anchor: Point,
    /// Content-space current point (where pointer is now).
    pub current: Point,
    /// Whether shift was held at marquee start (additive mode).
    pub additive: bool,
    /// Snapshot of selected IDs when marquee started (for additive mode).
    pub base_selection: HashSet<String>,
}

impl MarqueeState {
    /// The marquee rectangle (normalized: positive width/height).
    pub fn rect(&self) -> Rect {
        Rect::from_points(self.anchor, self.current)
    }
}

/// Event emitted when the selection changes.
#[derive(Clone, Debug)]
pub struct SelectionChangeEvent {
    /// The new complete set of selected region IDs.
    pub selected: HashSet<String>,
    /// IDs that were added in this change.
    pub added: HashSet<String>,
    /// IDs that were removed in this change.
    pub removed: HashSet<String>,
}

impl SelectionState {
    pub fn new() -> Self {
        Self::default()
    }

    /// Check if a region is selected.
    pub fn is_selected(&self, id: &str) -> bool {
        self.selected.contains(id)
    }

    /// Clear the entire selection.
    pub fn clear(&mut self) {
        self.selected.clear();
    }

    /// Replace selection with a single item.
    pub fn select_single(&mut self, id: String) {
        self.selected.clear();
        self.selected.insert(id);
    }

    /// Toggle a region in/out of the selection.
    pub fn toggle(&mut self, id: &str) {
        if self.selected.contains(id) {
            self.selected.remove(id);
        } else {
            self.selected.insert(id.to_string());
        }
    }

    /// Add a region to the selection.
    pub fn add(&mut self, id: String) {
        self.selected.insert(id);
    }

    /// Replace the entire selection set.
    pub fn replace(&mut self, ids: HashSet<String>) {
        self.selected = ids;
    }
}

// ── Tests ───────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_empty() {
        let sel = SelectionState::new();
        assert!(sel.selected.is_empty());
        assert!(sel.marquee.is_none());
    }

    #[test]
    fn test_select_single() {
        let mut sel = SelectionState::new();
        sel.select_single("a".into());
        assert!(sel.is_selected("a"));
        assert_eq!(sel.selected.len(), 1);

        sel.select_single("b".into());
        assert!(!sel.is_selected("a"));
        assert!(sel.is_selected("b"));
        assert_eq!(sel.selected.len(), 1);
    }

    #[test]
    fn test_toggle() {
        let mut sel = SelectionState::new();
        sel.toggle("a");
        assert!(sel.is_selected("a"));
        sel.toggle("a");
        assert!(!sel.is_selected("a"));
    }

    #[test]
    fn test_add_and_clear() {
        let mut sel = SelectionState::new();
        sel.add("a".into());
        sel.add("b".into());
        sel.add("c".into());
        assert_eq!(sel.selected.len(), 3);

        sel.clear();
        assert!(sel.selected.is_empty());
    }

    #[test]
    fn test_replace() {
        let mut sel = SelectionState::new();
        sel.add("old".into());

        let mut new_set = HashSet::new();
        new_set.insert("x".into());
        new_set.insert("y".into());
        sel.replace(new_set);

        assert!(!sel.is_selected("old"));
        assert!(sel.is_selected("x"));
        assert!(sel.is_selected("y"));
    }

    #[test]
    fn test_marquee_rect() {
        let marquee = MarqueeState {
            anchor: Point::new(100.0, 200.0),
            current: Point::new(50.0, 300.0),
            additive: false,
            base_selection: HashSet::new(),
        };
        let r = marquee.rect();
        assert!((r.x() - 50.0).abs() < 1e-6);
        assert!((r.y() - 200.0).abs() < 1e-6);
        assert!((r.width() - 50.0).abs() < 1e-6);
        assert!((r.height() - 100.0).abs() < 1e-6);
    }

    #[test]
    fn test_tool_default() {
        assert_eq!(CanvasTool::default(), CanvasTool::Pan);
    }
}
