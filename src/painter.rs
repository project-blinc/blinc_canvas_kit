//! Stateful immediate-mode drawing wrapper over `DrawContext`.
//!
//! `Painter2D` tracks a current fill and stroke so sketch code reads as:
//!
//! ```ignore
//! use blinc_canvas_kit::prelude::*;
//! use blinc_core::layer::Color;
//!
//! let mut p = Painter2D::new(ctx);
//! p.fill(Color::RED).no_stroke();
//! p.rect(10.0, 10.0, 100.0, 50.0);
//! p.stroke(Color::BLACK, 2.0);
//! p.circle(200.0, 200.0, 40.0);
//! ```
//!
//! Every call lowers to the equivalent `DrawContext` method. No new GPU
//! concepts are introduced — this layer only elides the per-call `fill` /
//! `stroke` / `corner_radius` arguments you would otherwise pass.
//!
//! # Transform stack
//!
//! `push()` / `pop()` bracket multiple `translate` / `rotate` / `scale`
//! calls. A single `pop()` undoes every transform pushed since the
//! matching `push()`:
//!
//! ```ignore
//! p.push();
//! p.translate(100.0, 100.0);
//! p.rotate(std::f32::consts::FRAC_PI_4);
//! p.scale(2.0, 2.0);
//! p.rect(-10.0, -10.0, 20.0, 20.0);   // all three transforms active
//! p.pop();                             // all three transforms undone
//! ```
//!
//! Calling `translate` / `rotate` / `scale` without a surrounding `push()`
//! still pushes onto the underlying `DrawContext` stack, but `pop()` cannot
//! undo those calls — follow the bracketed pattern for scoped transforms.

use blinc_core::draw::{Path, Stroke, Transform};
use blinc_core::layer::{Brush, Color, CornerRadius, Point, Rect};
use blinc_core::DrawContext;

/// Stateful immediate-mode 2D drawing wrapper around a [`DrawContext`].
///
/// Holds a mutable borrow of the underlying context for its lifetime; obtain
/// one via [`crate::sketch::SketchContext::painter`] inside a sketch or
/// directly via [`Painter2D::new`] inside a `kit.element(|ctx, _| ...)`
/// callback.
pub struct Painter2D<'a> {
    ctx: &'a mut dyn DrawContext,
    fill: Option<Brush>,
    stroke: Option<(Brush, f32)>,
    /// One frame per matching `push()` call. The `u32` counts how many
    /// transforms have been pushed to the underlying `DrawContext` since
    /// that `push()` — a matching `pop()` pops exactly that many.
    tx_stack: Vec<u32>,
}

impl<'a> Painter2D<'a> {
    /// Wrap a `DrawContext`. Default paint state: solid black fill, no stroke.
    pub fn new(ctx: &'a mut dyn DrawContext) -> Self {
        Self {
            ctx,
            fill: Some(Brush::Solid(Color::BLACK)),
            stroke: None,
            tx_stack: Vec::new(),
        }
    }

    /// Access the underlying `DrawContext` for features not covered by
    /// `Painter2D` — gradients, glass, custom paths with bezier segments,
    /// clip stacks, text, images, 3D.
    ///
    /// The current fill / stroke paint state on `Painter2D` is not applied
    /// to raw `DrawContext` calls.
    pub fn draw_context(&mut self) -> &mut dyn DrawContext {
        self.ctx
    }

    // ── Paint state ──────────────────────────────────────────────────────

    /// Set the current fill brush. Accepts anything convertible into
    /// `Brush` — `Color`, `GlassStyle`, `ImageBrush`, etc.
    pub fn fill(&mut self, brush: impl Into<Brush>) -> &mut Self {
        self.fill = Some(brush.into());
        self
    }

    /// Disable fill for subsequent primitives. Strokes still render.
    pub fn no_fill(&mut self) -> &mut Self {
        self.fill = None;
        self
    }

    /// Set the current stroke brush and width.
    pub fn stroke(&mut self, brush: impl Into<Brush>, width: f32) -> &mut Self {
        self.stroke = Some((brush.into(), width));
        self
    }

    /// Disable stroke for subsequent primitives. Fills still render.
    pub fn no_stroke(&mut self) -> &mut Self {
        self.stroke = None;
        self
    }

    /// Current fill brush, if any.
    pub fn current_fill(&self) -> Option<&Brush> {
        self.fill.as_ref()
    }

    /// Current stroke brush and width, if any.
    pub fn current_stroke(&self) -> Option<(&Brush, f32)> {
        self.stroke.as_ref().map(|(b, w)| (b, *w))
    }

    // ── Transform stack ──────────────────────────────────────────────────

    /// Begin a transform group. All `translate` / `rotate` / `scale` calls
    /// up to the matching `pop()` are undone by that `pop()`.
    pub fn push(&mut self) -> &mut Self {
        self.tx_stack.push(0);
        self
    }

    /// End a transform group started by `push()`, reverting every transform
    /// applied since then. No-op if there is no matching `push()`.
    pub fn pop(&mut self) -> &mut Self {
        if let Some(n) = self.tx_stack.pop() {
            for _ in 0..n {
                self.ctx.pop_transform();
            }
        }
        self
    }

    /// Translate subsequent drawing by `(x, y)`. Cumulative with prior
    /// transforms in the current push-group.
    pub fn translate(&mut self, x: f32, y: f32) -> &mut Self {
        self.apply_transform(Transform::translate(x, y))
    }

    /// Rotate subsequent drawing by `angle` radians around the current
    /// origin. Use [`Painter2D::push`] + [`Painter2D::translate`] first to
    /// rotate around a non-origin pivot.
    pub fn rotate(&mut self, angle: f32) -> &mut Self {
        self.apply_transform(Transform::rotate(angle))
    }

    /// Scale subsequent drawing by `(sx, sy)` around the current origin.
    pub fn scale(&mut self, sx: f32, sy: f32) -> &mut Self {
        self.apply_transform(Transform::scale(sx, sy))
    }

    fn apply_transform(&mut self, t: Transform) -> &mut Self {
        self.ctx.push_transform(t);
        if let Some(top) = self.tx_stack.last_mut() {
            *top += 1;
        }
        self
    }

    // ── Primitives ───────────────────────────────────────────────────────

    /// Draw an axis-aligned rectangle using the current fill and stroke.
    pub fn rect(&mut self, x: f32, y: f32, w: f32, h: f32) -> &mut Self {
        self.rounded_rect(x, y, w, h, 0.0)
    }

    /// Draw an axis-aligned rectangle with uniform corner radius `r`.
    pub fn rounded_rect(&mut self, x: f32, y: f32, w: f32, h: f32, r: f32) -> &mut Self {
        let rect = Rect::new(x, y, w, h);
        let cr = CornerRadius::uniform(r);
        if let Some(brush) = self.fill.clone() {
            self.ctx.fill_rect(rect, cr, brush);
        }
        if let Some((brush, width)) = self.stroke.clone() {
            self.ctx.stroke_rect(rect, cr, &Stroke::new(width), brush);
        }
        self
    }

    /// Draw a circle of radius `r` centered at `(x, y)`.
    pub fn circle(&mut self, x: f32, y: f32, r: f32) -> &mut Self {
        let center = Point::new(x, y);
        if let Some(brush) = self.fill.clone() {
            self.ctx.fill_circle(center, r, brush);
        }
        if let Some((brush, width)) = self.stroke.clone() {
            self.ctx
                .stroke_circle(center, r, &Stroke::new(width), brush);
        }
        self
    }

    /// Draw a straight line segment. Only the current stroke applies —
    /// lines have no fill.
    pub fn line(&mut self, x1: f32, y1: f32, x2: f32, y2: f32) -> &mut Self {
        if let Some((brush, width)) = self.stroke.clone() {
            let path = Path::line(Point::new(x1, y1), Point::new(x2, y2));
            self.ctx.stroke_path(&path, &Stroke::new(width), brush);
        }
        self
    }

    /// Fill and/or stroke an arbitrary `Path` using the current paint
    /// state. Use [`blinc_core::draw::Path`] to build bezier and multi-
    /// segment geometry.
    pub fn path(&mut self, path: &Path) -> &mut Self {
        if let Some(brush) = self.fill.clone() {
            self.ctx.fill_path(path, brush);
        }
        if let Some((brush, width)) = self.stroke.clone() {
            self.ctx.stroke_path(path, &Stroke::new(width), brush);
        }
        self
    }
}
