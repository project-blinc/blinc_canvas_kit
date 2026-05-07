//! Viewport-aware infinite canvas background patterns.
//!
//! Renders only the visible portion of the pattern — extends infinitely
//! in all directions including negative coordinates.
//!
//! # Usage
//!
//! ```ignore
//! let mut kit = CanvasKit::new("graph");
//! kit.set_background(CanvasBackground::dots().with_spacing(50.0));
//! ```

use blinc_core::draw::{Path, Stroke};
use blinc_core::layer::{Color, CornerRadius, Point, Rect};
use blinc_core::{Brush, DrawContext};

use crate::viewport::CanvasViewport;

/// Background pattern for an infinite canvas.
///
/// Set via `kit.set_background()` or `kit.with_background()`.
/// Drawn automatically by `kit.element()` before the user's render callback.
#[derive(Clone, Debug)]
pub enum CanvasBackground {
    /// No background pattern.
    None,
    /// Dot grid at regular spacing.
    Dots(PatternConfig),
    /// Horizontal + vertical lines.
    Grid(PatternConfig),
    /// Diagonal lines at ±45°.
    Crosshatch(PatternConfig),
}

/// Configuration for a background pattern.
#[derive(Clone, Debug)]
pub struct PatternConfig {
    /// Spacing between pattern elements in content-space pixels.
    pub spacing: f32,
    /// Color of the pattern elements.
    pub color: Color,
    /// Dot diameter (Dots) or line width (Grid/Crosshatch).
    pub size: f32,
    /// Zoom-adaptive level-of-detail.
    pub zoom_adaptive: Option<ZoomAdaptive>,
}

/// When zoomed far out, reduce pattern density to maintain performance.
#[derive(Clone, Debug)]
pub struct ZoomAdaptive {
    /// Below this zoom level, switch to coarse pattern.
    pub zoom_threshold: f32,
    /// Show every Nth element at coarse level.
    pub coarse_factor: u32,
}

// ── Constructors & Builders ─────────────────────────────────────────────

impl CanvasBackground {
    /// Dot grid with defaults: spacing=50, light gray, size=2.
    pub fn dots() -> Self {
        Self::Dots(PatternConfig {
            spacing: 50.0,
            color: Color::rgba(0.25, 0.25, 0.3, 0.5),
            size: 2.0,
            zoom_adaptive: None,
        })
    }

    /// Line grid with defaults: spacing=50, light gray, size=1.
    pub fn grid() -> Self {
        Self::Grid(PatternConfig {
            spacing: 50.0,
            color: Color::rgba(0.25, 0.25, 0.3, 0.3),
            size: 1.0,
            zoom_adaptive: None,
        })
    }

    /// Crosshatch with defaults: spacing=40, light gray, size=1.
    pub fn crosshatch() -> Self {
        Self::Crosshatch(PatternConfig {
            spacing: 40.0,
            color: Color::rgba(0.25, 0.25, 0.3, 0.25),
            size: 1.0,
            zoom_adaptive: None,
        })
    }

    /// Customize spacing.
    pub fn with_spacing(mut self, spacing: f32) -> Self {
        if let Some(c) = self.config_mut() {
            c.spacing = spacing;
        }
        self
    }

    /// Customize color.
    pub fn with_color(mut self, color: Color) -> Self {
        if let Some(c) = self.config_mut() {
            c.color = color;
        }
        self
    }

    /// Customize dot size or line width.
    pub fn with_size(mut self, size: f32) -> Self {
        if let Some(c) = self.config_mut() {
            c.size = size;
        }
        self
    }

    /// Enable zoom-adaptive rendering.
    ///
    /// Below `threshold` zoom, show every `coarse_factor`-th element.
    pub fn with_zoom_adaptive(mut self, threshold: f32, coarse_factor: u32) -> Self {
        if let Some(c) = self.config_mut() {
            c.zoom_adaptive = Some(ZoomAdaptive {
                zoom_threshold: threshold,
                coarse_factor: coarse_factor.max(1),
            });
        }
        self
    }

    fn config_mut(&mut self) -> Option<&mut PatternConfig> {
        match self {
            Self::None => None,
            Self::Dots(c) | Self::Grid(c) | Self::Crosshatch(c) => Some(c),
        }
    }
}

// ── Rendering ───────────────────────────────────────────────────────────

/// Max primitives per pattern per frame (safety cap).
const MAX_PRIMITIVES: usize = 10_000;

impl CanvasBackground {
    /// Draw the background pattern for the visible viewport region.
    ///
    /// Called with the viewport transform already pushed on DrawContext.
    /// `screen_w` / `screen_h` come from `CanvasBounds`.
    pub fn draw(
        &self,
        ctx: &mut dyn DrawContext,
        viewport: &CanvasViewport,
        screen_w: f32,
        screen_h: f32,
    ) {
        match self {
            Self::None => {}
            Self::Dots(config) => draw_dots(ctx, viewport, screen_w, screen_h, config),
            Self::Grid(config) => draw_grid_lines(ctx, viewport, screen_w, screen_h, config),
            Self::Crosshatch(config) => draw_crosshatch(ctx, viewport, screen_w, screen_h, config),
        }
    }
}

// ── Viewport math ───────────────────────────────────────────────────────

/// Visible content-space rectangle from viewport + screen size.
/// Returns (left, top, right, bottom).
fn visible_content_rect(vp: &CanvasViewport, screen_w: f32, screen_h: f32) -> (f32, f32, f32, f32) {
    let tl = vp.screen_to_content(Point::new(0.0, 0.0));
    let br = vp.screen_to_content(Point::new(screen_w, screen_h));
    (tl.x, tl.y, br.x, br.y)
}

/// Compute start, end, step for iterating grid cells within a range.
fn cell_range(content_min: f32, content_max: f32, spacing: f32, coarse: u32) -> (f32, f32, f32) {
    let step = spacing * coarse as f32;
    let first = (content_min / step).floor() * step;
    let last = (content_max / step).ceil() * step;
    (first, last, step)
}

fn effective_coarse(config: &PatternConfig, zoom: f32) -> u32 {
    match &config.zoom_adaptive {
        Some(za) if zoom < za.zoom_threshold => za.coarse_factor,
        _ => 1,
    }
}

// ── Dot pattern ─────────────────────────────────────────────────────────

fn draw_dots(
    ctx: &mut dyn DrawContext,
    viewport: &CanvasViewport,
    screen_w: f32,
    screen_h: f32,
    config: &PatternConfig,
) {
    let (left, top, right, bottom) = visible_content_rect(viewport, screen_w, screen_h);
    let coarse = effective_coarse(config, viewport.zoom);
    let (first_x, last_x, step) = cell_range(left, right, config.spacing, coarse);
    let (first_y, last_y, _) = cell_range(top, bottom, config.spacing, coarse);

    let brush = Brush::Solid(config.color);
    let half = config.size / 2.0;
    let radius = CornerRadius::uniform(half);

    let mut count = 0;
    let mut x = first_x;
    while x <= last_x {
        let mut y = first_y;
        while y <= last_y {
            if count >= MAX_PRIMITIVES {
                return;
            }
            ctx.fill_rect(
                Rect::new(x - half, y - half, config.size, config.size),
                radius,
                brush.clone(),
            );
            count += 1;
            y += step;
        }
        x += step;
    }
}

// ── Grid line pattern ───────────────────────────────────────────────────

fn draw_grid_lines(
    ctx: &mut dyn DrawContext,
    viewport: &CanvasViewport,
    screen_w: f32,
    screen_h: f32,
    config: &PatternConfig,
) {
    let (left, top, right, bottom) = visible_content_rect(viewport, screen_w, screen_h);
    let coarse = effective_coarse(config, viewport.zoom);
    let (first_x, last_x, step) = cell_range(left, right, config.spacing, coarse);
    let (first_y, last_y, _) = cell_range(top, bottom, config.spacing, coarse);

    let brush = Brush::Solid(config.color);
    let lw = config.size;
    let half = lw / 2.0;
    let height = bottom - top;
    let width = right - left;

    // Vertical lines
    let mut x = first_x;
    while x <= last_x {
        ctx.fill_rect(
            Rect::new(x - half, top, lw, height),
            CornerRadius::uniform(0.0),
            brush.clone(),
        );
        x += step;
    }

    // Horizontal lines
    let mut y = first_y;
    while y <= last_y {
        ctx.fill_rect(
            Rect::new(left, y - half, width, lw),
            CornerRadius::uniform(0.0),
            brush.clone(),
        );
        y += step;
    }
}

// ── Crosshatch pattern ──────────────────────────────────────────────────

fn draw_crosshatch(
    ctx: &mut dyn DrawContext,
    viewport: &CanvasViewport,
    screen_w: f32,
    screen_h: f32,
    config: &PatternConfig,
) {
    let (left, top, right, bottom) = visible_content_rect(viewport, screen_w, screen_h);
    let coarse = effective_coarse(config, viewport.zoom);
    let step = config.spacing * coarse as f32;

    let brush = Brush::Solid(config.color);
    let stroke = Stroke::new(config.size);

    let mut count = 0;

    // +45° diagonals: y = x - d, parameterized by d = x - y
    let d_min = ((left - bottom) / step).floor() * step;
    let d_max = ((right - top) / step).ceil() * step;

    let mut d = d_min;
    while d <= d_max {
        if count >= MAX_PRIMITIVES {
            return;
        }
        // Clip line y = x - d to visible rect
        let x0 = left.max(d + top);
        let y0 = x0 - d;
        let x1 = right.min(d + bottom);
        let y1 = x1 - d;

        if x0 < x1 {
            let path = Path::new().move_to(x0, y0).line_to(x1, y1);
            ctx.stroke_path(&path, &stroke, brush.clone());
            count += 1;
        }
        d += step;
    }

    // -45° diagonals: y = -x + d, parameterized by d = x + y
    let d_min = ((left + top) / step).floor() * step;
    let d_max = ((right + bottom) / step).ceil() * step;

    let mut d = d_min;
    while d <= d_max {
        if count >= MAX_PRIMITIVES {
            return;
        }
        // Clip line y = -x + d to visible rect
        let x0 = left.max(d - bottom);
        let y0 = -x0 + d;
        let x1 = right.min(d - top);
        let y1 = -x1 + d;

        if x0 < x1 {
            let path = Path::new().move_to(x0, y0).line_to(x1, y1);
            ctx.stroke_path(&path, &stroke, brush.clone());
            count += 1;
        }
        d += step;
    }
}

// ── Tests ───────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_visible_content_rect_identity() {
        let vp = CanvasViewport::new(); // pan=0, zoom=1
        let (l, t, r, b) = visible_content_rect(&vp, 800.0, 600.0);
        assert!((l - 0.0).abs() < 1e-3);
        assert!((t - 0.0).abs() < 1e-3);
        assert!((r - 800.0).abs() < 1e-3);
        assert!((b - 600.0).abs() < 1e-3);
    }

    #[test]
    fn test_visible_content_rect_zoomed() {
        let mut vp = CanvasViewport::new();
        vp.zoom = 2.0; // 2x zoom: visible area halved
        let (l, t, r, b) = visible_content_rect(&vp, 800.0, 600.0);
        assert!((r - l - 400.0).abs() < 1e-2);
        assert!((b - t - 300.0).abs() < 1e-2);
    }

    #[test]
    fn test_visible_content_rect_panned() {
        let mut vp = CanvasViewport::new();
        vp.pan_x = 100.0; // panned right in content space
        vp.pan_y = -50.0;
        let (l, t, _r, _b) = visible_content_rect(&vp, 800.0, 600.0);
        // screen(0,0) → content(-100, 50) (inverted pan)
        assert!((l - (-100.0)).abs() < 1e-2);
        assert!((t - 50.0).abs() < 1e-2);
    }

    #[test]
    fn test_cell_range_positive() {
        let (first, last, step) = cell_range(75.0, 250.0, 50.0, 1);
        assert_eq!(step, 50.0);
        assert_eq!(first, 50.0); // floor(75/50)*50
        assert_eq!(last, 250.0); // ceil(250/50)*50
    }

    #[test]
    fn test_cell_range_negative() {
        let (first, last, step) = cell_range(-120.0, -30.0, 50.0, 1);
        assert_eq!(step, 50.0);
        assert_eq!(first, -150.0); // floor(-120/50)*50 = floor(-2.4)*50 = -3*50
        assert_eq!(last, 0.0); // ceil(-30/50)*50 = ceil(-0.6)*50 = 0*50
    }

    #[test]
    fn test_cell_range_coarse() {
        let (first, last, step) = cell_range(0.0, 500.0, 50.0, 5);
        assert_eq!(step, 250.0); // 50*5
        assert_eq!(first, 0.0);
        assert_eq!(last, 500.0);
    }

    #[test]
    fn test_effective_coarse_below_threshold() {
        let config = PatternConfig {
            spacing: 50.0,
            color: Color::rgba(1.0, 1.0, 1.0, 1.0),
            size: 2.0,
            zoom_adaptive: Some(ZoomAdaptive {
                zoom_threshold: 0.5,
                coarse_factor: 5,
            }),
        };
        assert_eq!(effective_coarse(&config, 0.3), 5);
        assert_eq!(effective_coarse(&config, 0.5), 1); // at threshold = fine
        assert_eq!(effective_coarse(&config, 1.0), 1);
    }

    #[test]
    fn test_effective_coarse_no_adaptive() {
        let config = PatternConfig {
            spacing: 50.0,
            color: Color::rgba(1.0, 1.0, 1.0, 1.0),
            size: 2.0,
            zoom_adaptive: None,
        };
        assert_eq!(effective_coarse(&config, 0.1), 1);
    }

    #[test]
    fn test_builder_chaining() {
        let bg = CanvasBackground::dots()
            .with_spacing(30.0)
            .with_color(Color::rgba(1.0, 0.0, 0.0, 1.0))
            .with_size(4.0)
            .with_zoom_adaptive(0.3, 5);

        match bg {
            CanvasBackground::Dots(c) => {
                assert_eq!(c.spacing, 30.0);
                assert_eq!(c.size, 4.0);
                assert!(c.zoom_adaptive.is_some());
                let za = c.zoom_adaptive.unwrap();
                assert_eq!(za.zoom_threshold, 0.3);
                assert_eq!(za.coarse_factor, 5);
            }
            _ => panic!("Expected Dots"),
        }
    }

    #[test]
    fn test_none_builder_noop() {
        let bg = CanvasBackground::None.with_spacing(30.0).with_size(5.0);
        assert!(matches!(bg, CanvasBackground::None));
    }
}
