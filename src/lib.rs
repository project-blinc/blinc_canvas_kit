//! Canvas toolkit for interactive pan/zoom canvases with hit testing.
//!
//! `blinc_canvas_kit` provides viewport management (pan, zoom, coordinate
//! conversion) and element interaction (click, drag, hover) for
//! `blinc_layout::Canvas` elements. All state persists across UI rebuilds
//! via `BlincContextState`.
//!
//! # Usage
//!
//! ```ignore
//! use blinc_canvas_kit::prelude::*;
//!
//! let kit = CanvasKit::new("diagram");
//!
//! kit.on_element_click(|evt| {
//!     tracing::info!("Clicked {:?}", evt.region_id);
//! });
//!
//! kit.element(|ctx, bounds| {
//!     let r = Rect::new(100.0, 100.0, 200.0, 150.0);
//!     ctx.fill_rect(r, 8.0.into(), Brush::Solid(Color::BLUE));
//!     kit.hit_rect("my_node", r);
//! })
//! ```

pub mod background;
pub mod geometry;
pub mod grid_pass;
pub mod hit;
pub mod loading;
pub mod material;
pub mod math;
pub mod painter;
pub mod pan;
pub mod scene3d;
pub mod selection;
pub mod sketch;
pub mod snap;
pub mod spatial;
pub mod viewport;
pub mod zoom;

use std::collections::HashSet;
use std::rc::Rc;
use std::sync::Arc;

use blinc_core::draw::Stroke;
use blinc_core::events::event_types;
use blinc_core::layer::{Affine2D, Color, CornerRadius, Point, Rect};
use blinc_core::{BlincContextState, Brush, SignalId, State};
use blinc_layout::canvas::{canvas, CanvasBounds};
use blinc_layout::div::{div, Div};
pub use blinc_layout::event_handler::EventContext;

pub use background::{CanvasBackground, PatternConfig, ZoomAdaptive};
pub use geometry::Geometry;
pub use hit::{CanvasDragEvent, CanvasEvent, HitRegion, InteractionState};
pub use loading::{fit_aabb, AutoFramer};
pub use material::MaterialBuilder;
pub use painter::Painter2D;
pub use pan::PanController;
pub use scene3d::{EnvironmentData, MeshHandle, OrbitCamera, SceneKit3D};
pub use selection::{CanvasTool, MarqueeState, SelectionChangeEvent, SelectionState};
pub use sketch::{sketch, Player, Sketch, SketchContext, SketchEvents};
pub use snap::SnapController;
pub use spatial::SpatialIndex;
pub use viewport::{affine_inverse, CanvasViewport};
pub use zoom::ZoomController;

/// Prelude for convenient imports.
pub mod prelude {
    pub use crate::background::{CanvasBackground, PatternConfig, ZoomAdaptive};
    pub use crate::geometry::Geometry;
    pub use crate::hit::{CanvasDragEvent, CanvasEvent, HitRegion, InteractionState};
    pub use crate::material::MaterialBuilder;
    pub use crate::painter::Painter2D;
    pub use crate::pan::PanController;
    pub use crate::scene3d::{EnvironmentData, MeshHandle, OrbitCamera, SceneKit3D};
    pub use crate::selection::{CanvasTool, MarqueeState, SelectionChangeEvent, SelectionState};
    pub use crate::sketch::{sketch, Player, Sketch, SketchContext, SketchEvents};
    pub use crate::snap::SnapController;
    pub use crate::spatial::SpatialIndex;
    pub use crate::viewport::{affine_inverse, CanvasViewport};
    pub use crate::zoom::ZoomController;
    pub use crate::CanvasKit;
}

type EventCallback = Arc<dyn Fn(&CanvasEvent) + Send + Sync>;
type DragCallback = Arc<dyn Fn(&CanvasDragEvent) + Send + Sync>;
type SelectionCallback = Arc<dyn Fn(&SelectionChangeEvent) + Send + Sync>;
/// Subscriber that sees every raw `EventContext` routed through
/// `CanvasKit::handle_event`, in addition to whatever CanvasKit's own
/// pan / zoom / selection / hit-testing does with it. Used to bridge
/// the canvas-bounded event stream into polling input layers (e.g.
/// `blinc_input::InputState`) without the caller having to attach
/// handlers to a `Div` directly.
type AnyEventCallback = Arc<dyn Fn(&EventContext) + Send + Sync>;

/// Interactive canvas toolkit — receives events, manages viewport state,
/// and provides hit testing for canvas-drawn elements.
///
/// Create one per canvas via `CanvasKit::new("key")`. All state persists
/// across UI rebuilds via `BlincContextState`.
///
/// Two ways to use:
/// - `kit.element(render_fn)` — returns a Div with Canvas + event handlers pre-wired
/// - `kit.handler()` — returns a closure to attach to any element
#[derive(Clone)]
pub struct CanvasKit {
    viewport: State<CanvasViewport>,
    pan: State<PanController>,
    zoom_controller: ZoomController,
    hit_regions: State<SpatialIndex>,
    interaction: State<InteractionState>,
    selection: State<SelectionState>,
    tool: State<CanvasTool>,
    screen_bounds: State<(f32, f32)>,
    snap: SnapController,
    background: CanvasBackground,
    on_click_cb: Option<EventCallback>,
    on_hover_cb: Option<EventCallback>,
    on_drag_cb: Option<DragCallback>,
    on_drag_end_cb: Option<EventCallback>,
    on_selection_change_cb: Option<SelectionCallback>,
    /// Subscribers forwarded every raw `EventContext` CanvasKit sees —
    /// see [`CanvasKit::on_any_event`].
    any_event_cbs: Vec<AnyEventCallback>,
}

impl CanvasKit {
    /// Create a canvas kit with persistent state keyed by name.
    pub fn new(key: &str) -> Self {
        let ctx = BlincContextState::get();
        Self {
            viewport: ctx.use_state_keyed(&format!("{key}_vp"), CanvasViewport::new),
            pan: ctx.use_state_keyed(&format!("{key}_pan"), PanController::new),
            zoom_controller: ZoomController::new(),
            hit_regions: ctx.use_state_keyed(&format!("{key}_hit"), || SpatialIndex::new(100.0)),
            interaction: ctx.use_state_keyed(&format!("{key}_ia"), InteractionState::default),
            selection: ctx.use_state_keyed(&format!("{key}_sel"), SelectionState::new),
            tool: ctx.use_state_keyed(&format!("{key}_tool"), CanvasTool::default),
            screen_bounds: ctx.use_state_keyed(&format!("{key}_sb"), || (0.0, 0.0)),
            snap: SnapController::disabled(),
            background: CanvasBackground::None,
            on_click_cb: None,
            on_hover_cb: None,
            on_drag_cb: None,
            on_drag_end_cb: None,
            on_selection_change_cb: None,
            any_event_cbs: Vec::new(),
        }
    }

    /// Create with custom zoom controller settings.
    pub fn with_zoom_controller(mut self, zc: ZoomController) -> Self {
        self.zoom_controller = zc;
        self
    }

    /// Set the canvas background pattern (builder).
    pub fn with_background(mut self, bg: CanvasBackground) -> Self {
        self.background = bg;
        self
    }

    /// Set the canvas background pattern.
    pub fn set_background(&mut self, bg: CanvasBackground) {
        self.background = bg;
    }

    /// Set the spatial index cell size (builder). Default: 100.0.
    pub fn with_spatial_cell_size(self, cell_size: f32) -> Self {
        self.hit_regions.update(|_| SpatialIndex::new(cell_size));
        self
    }

    /// Set the canvas tool mode (builder).
    pub fn with_tool(self, tool: CanvasTool) -> Self {
        self.tool.set(tool);
        self
    }

    /// Set the canvas tool mode.
    pub fn set_tool(&self, tool: CanvasTool) {
        self.tool.set(tool);
    }

    /// Current tool mode.
    pub fn tool(&self) -> CanvasTool {
        self.tool.get()
    }

    /// Enable snap-to-grid with the given spacing (builder).
    pub fn with_snap(mut self, spacing: f32) -> Self {
        self.snap = SnapController::new(spacing);
        self
    }

    /// Set snap-to-grid enabled/disabled.
    pub fn set_snap_enabled(&mut self, enabled: bool) {
        self.snap.enabled = enabled;
    }

    /// Set snap-to-grid spacing.
    pub fn set_snap_spacing(&mut self, spacing: f32) {
        self.snap.spacing = spacing.max(1.0);
    }

    /// Whether snap-to-grid is enabled.
    pub fn snap_enabled(&self) -> bool {
        self.snap.enabled
    }

    /// Snap a content-space point to the nearest grid intersection.
    ///
    /// Returns the point unchanged if snapping is disabled.
    pub fn snap_point(&self, pt: Point) -> Point {
        self.snap.snap_point(pt)
    }

    /// Snap a content-space rect's origin to the nearest grid intersection.
    pub fn snap_rect(&self, rect: Rect) -> Rect {
        let snapped = self.snap.snap_point(Point::new(rect.x(), rect.y()));
        Rect::new(snapped.x, snapped.y, rect.width(), rect.height())
    }

    // ── Callback Builders ────────────────────────────────────────────

    /// Set callback for clicks on hit regions.
    pub fn on_element_click(&mut self, cb: impl Fn(&CanvasEvent) + Send + Sync + 'static) {
        self.on_click_cb = Some(Arc::new(cb));
    }

    /// Set callback for hover changes (region enter/leave).
    pub fn on_element_hover(&mut self, cb: impl Fn(&CanvasEvent) + Send + Sync + 'static) {
        self.on_hover_cb = Some(Arc::new(cb));
    }

    /// Set callback for dragging a hit region.
    pub fn on_element_drag(&mut self, cb: impl Fn(&CanvasDragEvent) + Send + Sync + 'static) {
        self.on_drag_cb = Some(Arc::new(cb));
    }

    /// Set callback for drag end on a hit region.
    pub fn on_element_drag_end(&mut self, cb: impl Fn(&CanvasEvent) + Send + Sync + 'static) {
        self.on_drag_end_cb = Some(Arc::new(cb));
    }

    /// Set callback for selection changes (multi-select, marquee).
    pub fn on_selection_change(
        &mut self,
        cb: impl Fn(&SelectionChangeEvent) + Send + Sync + 'static,
    ) {
        self.on_selection_change_cb = Some(Arc::new(cb));
    }

    /// Subscribe to the raw `EventContext` stream CanvasKit routes.
    ///
    /// Every event `CanvasKit::handle_event` receives — pointer down /
    /// up / move, drag / drag-end, scroll, pinch — is also forwarded to
    /// every subscriber registered here. Use this to bridge canvas
    /// input into a polling layer like `blinc_input::InputState`
    /// without having to attach `Div` handlers yourself: CanvasKit is
    /// already scoped to the canvas's bounds, so subscribers see only
    /// events the user directed at the canvas.
    ///
    /// Keyboard events are not forwarded here — Blinc's key routing
    /// is focus-based and lives outside CanvasKit. For those, attach
    /// `on_key_down` / `on_key_up` to the `Div` that wraps your canvas
    /// (e.g. via `kit.element(...)`'s return value) and forward from
    /// there.
    ///
    /// Can be called multiple times; subscribers fan out in
    /// registration order.
    pub fn on_any_event(&mut self, cb: impl Fn(&EventContext) + Send + Sync + 'static) {
        self.any_event_cbs.push(Arc::new(cb));
    }

    // ── Selection ───────────────────────────────────────────────────

    /// Current selection state.
    pub fn selection(&self) -> SelectionState {
        self.selection.get()
    }

    /// Signal ID for the selection state (for reactive deps).
    pub fn selection_signal(&self) -> SignalId {
        self.selection.signal_id()
    }

    /// Set the selection to specific IDs, firing the selection change callback.
    pub fn set_selection(&self, ids: HashSet<String>) {
        let old = self.selection.get();
        let added: HashSet<_> = ids.difference(&old.selected).cloned().collect();
        let removed: HashSet<_> = old.selected.difference(&ids).cloned().collect();

        if added.is_empty() && removed.is_empty() {
            return;
        }

        self.selection.update(|mut s| {
            s.selected = ids.clone();
            s
        });

        if let Some(ref cb) = self.on_selection_change_cb {
            cb(&SelectionChangeEvent {
                selected: ids,
                added,
                removed,
            });
        }
    }

    /// Clear the selection.
    pub fn clear_selection(&self) {
        self.set_selection(HashSet::new());
    }

    /// Check if a region is currently selected.
    pub fn is_selected(&self, id: &str) -> bool {
        self.selection.get().is_selected(id)
    }

    // ── Hit Region Registration ──────────────────────────────────────

    /// Register a content-space hit region. Call during the draw callback.
    ///
    /// Regions are tested in reverse order (last registered = topmost).
    pub fn hit_rect(&self, id: impl Into<String>, rect: Rect) {
        self.hit_regions.update(|mut index| {
            index.insert(HitRegion::new(id, rect));
            index
        });
    }

    /// Clear all hit regions. Called automatically by `element()` before each render.
    pub fn begin_frame(&self) {
        self.hit_regions.update(|mut index| {
            index.clear();
            index
        });
    }

    /// Hit-test a content-space point against registered regions.
    pub fn hit_test(&self, content_point: Point) -> Option<String> {
        self.hit_regions.get().hit_test(content_point)
    }

    /// Range query: find all region IDs intersecting a content-space rect.
    ///
    /// Used internally for marquee selection, also available for user queries.
    pub fn query_rect(&self, rect: Rect) -> HashSet<String> {
        self.hit_regions.get().query_rect(&rect)
    }

    // ── Viewport Culling ─────────────────────────────────────────────

    /// Check if a content-space rectangle is visible in the current viewport.
    ///
    /// Uses screen bounds from the most recent render. Returns `true` if
    /// screen bounds are not yet known (conservative default).
    pub fn is_visible(&self, rect: Rect) -> bool {
        let (sw, sh) = self.screen_bounds.get();
        if sw <= 0.0 || sh <= 0.0 {
            return true;
        }
        let vp = self.viewport.get();
        vp.is_visible(rect.x(), rect.y(), rect.width(), rect.height(), sw, sh)
    }

    // ── Event Processing ──────────────────────────────────────────

    /// Process an EventContext. Dispatches to hit testing, pan/zoom,
    /// selection, marquee, and callbacks.
    pub fn handle_event(&self, evt: &EventContext) {
        let screen_pt = Point::new(evt.local_x, evt.local_y);

        // Fan out to any-event subscribers before the match so they
        // see events like `DRAG_END` / `PINCH` that the match below
        // may not individually inspect. Ordering is registration-order.
        for cb in &self.any_event_cbs {
            cb(evt);
        }

        match evt.event_type {
            event_types::POINTER_DOWN => {
                self.handle_pointer_down(evt, screen_pt);
            }

            event_types::POINTER_MOVE => {
                let vp = self.viewport.get();
                let content_pt = vp.screen_to_content(screen_pt);
                let hit = self.hit_test(content_pt);

                let mut ia = self.interaction.get();
                let changed = ia.hovered != hit;
                ia.hovered = hit;
                self.interaction.set(ia);

                if changed {
                    if let Some(ref cb) = self.on_hover_cb {
                        let ia = self.interaction.get();
                        cb(&CanvasEvent {
                            content_point: content_pt,
                            screen_point: screen_pt,
                            region_id: ia.hovered.clone(),
                        });
                    }
                }
            }

            event_types::POINTER_UP => {
                self.handle_pointer_up(evt, screen_pt);
            }

            event_types::DRAG => {
                self.handle_drag(evt, screen_pt);
            }

            event_types::DRAG_END => {
                self.handle_drag_end(evt, screen_pt);
            }

            event_types::SCROLL => {
                let cursor = screen_pt;
                let zc = self.zoom_controller.clone();
                self.viewport.update(|mut vp| {
                    zc.on_scroll(&mut vp, evt.scroll_delta_y, cursor);
                    vp
                });
            }

            event_types::PINCH => {
                let center = Point::new(evt.pinch_center_x, evt.pinch_center_y);
                let zc = self.zoom_controller.clone();
                self.viewport.update(|mut vp| {
                    zc.on_pinch(&mut vp, evt.pinch_scale, center);
                    vp
                });
            }

            _ => {}
        }
    }

    fn handle_pointer_down(&self, evt: &EventContext, screen_pt: Point) {
        let vp = self.viewport.get();
        let content_pt = vp.screen_to_content(screen_pt);
        let hit = self.hit_test(content_pt);
        let tool = self.tool.get();

        if let Some(ref region_id) = hit {
            // Clicked on an element — update selection based on modifiers
            let sel = self.selection.get();

            if evt.meta || evt.ctrl {
                // Cmd/Ctrl+Click: toggle individual item
                let mut new_sel = sel.selected.clone();
                if new_sel.contains(region_id) {
                    new_sel.remove(region_id);
                } else {
                    new_sel.insert(region_id.clone());
                }
                self.set_selection(new_sel);
            } else if evt.shift {
                // Shift+Click: add to selection
                let mut new_sel = sel.selected.clone();
                new_sel.insert(region_id.clone());
                self.set_selection(new_sel);
            } else if !sel.is_selected(region_id) {
                // Plain click on unselected item: replace selection
                let mut new_sel = HashSet::new();
                new_sel.insert(region_id.clone());
                self.set_selection(new_sel);
            }
            // If plain click on already-selected item, defer to POINTER_UP
            // (allows dragging multi-selection without losing it)

            self.interaction.set(InteractionState {
                hovered: self.interaction.get().hovered,
                active: hit,
                drag_start: Some(content_pt),
                did_drag: false,
            });
        } else {
            // Clicked on background
            let should_marquee = tool == CanvasTool::Select || evt.shift;

            if should_marquee {
                // Start marquee selection
                let additive = evt.shift;
                self.selection.update(|mut s| {
                    s.marquee = Some(MarqueeState {
                        anchor: content_pt,
                        current: content_pt,
                        additive,
                        base_selection: if additive {
                            s.selected.clone()
                        } else {
                            HashSet::new()
                        },
                    });
                    s
                });
            }

            if !evt.shift && !evt.meta && !evt.ctrl && tool != CanvasTool::Select {
                // Plain background click in Pan mode: clear selection
                self.clear_selection();
            }

            self.interaction.set(InteractionState {
                hovered: self.interaction.get().hovered,
                active: None,
                drag_start: Some(content_pt),
                did_drag: false,
            });
        }
    }

    fn handle_pointer_up(&self, evt: &EventContext, screen_pt: Point) {
        let ia = self.interaction.get();
        let sel = self.selection.get();

        if ia.active.is_some() && !ia.did_drag {
            // Click (pointer down + up without drag)
            if let Some(ref cb) = self.on_click_cb {
                let vp = self.viewport.get();
                let content_pt = vp.screen_to_content(screen_pt);
                cb(&CanvasEvent {
                    content_point: content_pt,
                    screen_point: screen_pt,
                    region_id: ia.active.clone(),
                });
            }

            // Plain click on already-selected item in multi-selection:
            // narrow to just that item
            if !evt.shift && !evt.meta && !evt.ctrl {
                if let Some(ref id) = ia.active {
                    if sel.selected.len() > 1 {
                        let mut new_sel = HashSet::new();
                        new_sel.insert(id.clone());
                        self.set_selection(new_sel);
                    }
                }
            }
        }

        // Finalize marquee if active
        if sel.marquee.is_some() {
            let base = sel
                .marquee
                .as_ref()
                .map(|m| m.base_selection.clone())
                .unwrap_or_default();

            self.selection.update(|mut s| {
                s.marquee = None;
                s
            });

            // Fire selection change with final diff
            let final_sel = self.selection.get();
            if let Some(ref cb) = self.on_selection_change_cb {
                let added = final_sel.selected.difference(&base).cloned().collect();
                let removed = base.difference(&final_sel.selected).cloned().collect();
                cb(&SelectionChangeEvent {
                    selected: final_sel.selected.clone(),
                    added,
                    removed,
                });
            }
        }

        // Clear interaction state
        self.interaction.set(InteractionState {
            hovered: ia.hovered,
            active: None,
            drag_start: None,
            did_drag: false,
        });
    }

    fn handle_drag(&self, evt: &EventContext, screen_pt: Point) {
        let ia = self.interaction.get();
        let sel = self.selection.get();

        if ia.active.is_some() {
            // Element drag — move all selected elements
            let vp = self.viewport.get();
            let content_pt = vp.screen_to_content(screen_pt);
            let delta = if let Some(start) = ia.drag_start {
                Point::new(content_pt.x - start.x, content_pt.y - start.y)
            } else {
                Point::new(0.0, 0.0)
            };

            if let Some(ref cb) = self.on_drag_cb {
                // Fire callback for each selected element with the same delta
                for selected_id in &sel.selected {
                    cb(&CanvasDragEvent {
                        content_point: content_pt,
                        content_delta: delta,
                        screen_point: screen_pt,
                        region_id: selected_id.clone(),
                    });
                }
                // If the active element isn't in the selection (edge case),
                // fire for it anyway
                if let Some(ref active_id) = ia.active {
                    if !sel.selected.contains(active_id) {
                        cb(&CanvasDragEvent {
                            content_point: content_pt,
                            content_delta: delta,
                            screen_point: screen_pt,
                            region_id: active_id.clone(),
                        });
                    }
                }
            }

            // Update drag_start for incremental deltas
            self.interaction.set(InteractionState {
                hovered: ia.hovered,
                active: ia.active,
                drag_start: Some(content_pt),
                did_drag: true,
            });
        } else if sel.marquee.is_some() {
            // Marquee drag — update current point + live preview
            let vp = self.viewport.get();
            let content_pt = vp.screen_to_content(screen_pt);

            // Query spatial index for regions in marquee rect
            let marquee_anchor = sel.marquee.as_ref().map(|m| m.anchor).unwrap_or(content_pt);
            let marquee_rect = Rect::from_points(marquee_anchor, content_pt);
            let hits = self.query_rect(marquee_rect);

            self.selection.update(|mut s| {
                if let Some(ref mut m) = s.marquee {
                    m.current = content_pt;
                    if m.additive {
                        s.selected = m.base_selection.clone();
                        s.selected.extend(hits);
                    } else {
                        s.selected = hits;
                    }
                }
                s
            });
        } else {
            // Background drag in Pan mode — pan viewport
            let dx = evt.drag_delta_x;
            let dy = evt.drag_delta_y;
            let mut vp = self.viewport.get();
            let mut pan = self.pan.get();
            pan.on_drag(&mut vp, dx, dy);
            self.pan.set(pan);
            self.viewport.set(vp);
        }
    }

    fn handle_drag_end(&self, evt: &EventContext, screen_pt: Point) {
        let ia = self.interaction.get();
        if ia.active.is_some() {
            // Element drag end
            if let Some(ref cb) = self.on_drag_end_cb {
                let vp = self.viewport.get();
                let content_pt = vp.screen_to_content(screen_pt);
                cb(&CanvasEvent {
                    content_point: content_pt,
                    screen_point: screen_pt,
                    region_id: ia.active.clone(),
                });
            }
            self.interaction.set(InteractionState {
                hovered: ia.hovered,
                active: None,
                drag_start: None,
                did_drag: false,
            });
        } else {
            // Background drag end — momentum (only if not marquee)
            let sel = self.selection.get();
            if sel.marquee.is_none() {
                self.pan.update(|mut pan| {
                    pan.on_drag_end();
                    pan
                });
            }
            // Note: marquee finalization happens in POINTER_UP
            let _ = evt;
        }
    }

    /// Returns a handler closure suitable for attaching to any element.
    ///
    /// Handles POINTER_DOWN/UP/MOVE, DRAG, DRAG_END, SCROLL, and PINCH.
    pub fn handler(&self) -> Arc<dyn Fn(&EventContext) + Send + Sync + 'static> {
        let kit = self.clone();
        Arc::new(move |evt: &EventContext| {
            kit.handle_event(evt);
        })
    }

    // ── Builder ───────────────────────────────────────────────────

    /// Build a fully-wired Div containing a Canvas with all event handlers.
    ///
    /// The render callback receives `DrawContext` with the viewport transform
    /// already applied — draw in content-space coordinates directly.
    /// Call `kit.hit_rect(id, rect)` inside the callback to register
    /// interactive regions.
    pub fn element<F>(&self, render_fn: F) -> Div
    where
        F: Fn(&mut dyn blinc_core::DrawContext, CanvasBounds) + 'static,
    {
        let kit_render = self.clone();
        let render = Rc::new(render_fn);

        let h = self.handler();
        let h_drag = h.clone();
        let h_drag_end = h.clone();
        let h_scroll = h.clone();
        let h_pinch = h.clone();
        let h_ptr_down = h.clone();
        let h_ptr_up = h.clone();
        let h_ptr_move = h;

        div()
            .w_full()
            .h_full()
            .on_event(event_types::POINTER_DOWN, move |evt| h_ptr_down(evt))
            .on_event(event_types::POINTER_UP, move |evt| h_ptr_up(evt))
            .on_event(event_types::POINTER_MOVE, move |evt| h_ptr_move(evt))
            .on_drag(move |evt| h_drag(evt))
            .on_drag_end(move |evt| h_drag_end(evt))
            .on_scroll(move |evt| h_scroll(evt))
            .on_event(event_types::PINCH, move |evt| h_pinch(evt))
            .child(
                canvas(move |ctx, bounds| {
                    // Store screen bounds for viewport culling helper
                    kit_render.screen_bounds.set((bounds.width, bounds.height));

                    kit_render.begin_frame();
                    let vp = kit_render.viewport();
                    let transform = vp.transform();
                    ctx.push_transform(transform.into());

                    // Draw background pattern (behind user content)
                    kit_render
                        .background
                        .draw(ctx, &vp, bounds.width, bounds.height);

                    render(ctx, bounds);

                    // Draw marquee overlay (above user content)
                    let sel = kit_render.selection.get();
                    if let Some(ref marquee) = sel.marquee {
                        let rect = marquee.rect();
                        let stroke = Stroke::new(1.0 / vp.zoom)
                            .with_dash(vec![4.0 / vp.zoom, 3.0 / vp.zoom], 0.0);
                        ctx.stroke_rect(
                            rect,
                            CornerRadius::uniform(0.0),
                            &stroke,
                            Brush::Solid(Color::rgba(0.2, 0.5, 1.0, 0.8)),
                        );
                        ctx.fill_rect(
                            rect,
                            CornerRadius::uniform(0.0),
                            Brush::Solid(Color::rgba(0.2, 0.5, 1.0, 0.15)),
                        );
                    }

                    ctx.pop_transform();
                })
                .w_full()
                .h_full(),
            )
    }

    // ── Viewport Access ───────────────────────────────────────────

    /// Current viewport transform (content → screen).
    pub fn transform(&self) -> Affine2D {
        self.viewport.get().transform()
    }

    /// Current viewport state.
    pub fn viewport(&self) -> CanvasViewport {
        self.viewport.get()
    }

    /// Update the viewport via a mutation closure.
    pub fn update_viewport(&self, f: impl FnOnce(&mut CanvasViewport)) {
        self.viewport.update(|mut vp| {
            f(&mut vp);
            vp
        });
    }

    /// Signal ID for reactive dependency tracking (e.g. `stateful().deps()`).
    pub fn viewport_signal(&self) -> SignalId {
        self.viewport.signal_id()
    }

    /// Convert screen-space point to content-space.
    pub fn screen_to_content(&self, screen: Point) -> Point {
        self.viewport.get().screen_to_content(screen)
    }

    /// Convert content-space point to screen-space.
    pub fn content_to_screen(&self, content: Point) -> Point {
        self.viewport.get().content_to_screen(content)
    }

    /// Current interaction state (hovered/active regions).
    pub fn interaction(&self) -> InteractionState {
        self.interaction.get()
    }

    /// Signal ID for the interaction state (for reactive deps).
    pub fn interaction_signal(&self) -> SignalId {
        self.interaction.signal_id()
    }
}
