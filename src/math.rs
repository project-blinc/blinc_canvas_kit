//! Creative-coding math helpers.
//!
//! Conveniences for sketches: `map`, `lerp`, `constrain`, `norm`, `dist`,
//! `smoothstep`. Pure functions — no dependencies on the rest of the canvas
//! kit.
//!
//! These live here (and not in `blinc_core::math`) because today they are
//! only needed by creative-coding sketches. If wider parts of the codebase
//! start using them, promote the module down the dependency graph.

/// Linear interpolation between `a` and `b`.
///
/// `t` is not clamped; values outside `[0.0, 1.0]` extrapolate past the
/// endpoints.
#[inline]
pub fn lerp(a: f32, b: f32, t: f32) -> f32 {
    a + (b - a) * t
}

/// Re-map a value from one range to another.
///
/// Equivalent to `b0 + (v - a0) / (a1 - a0) * (b1 - b0)`. Extrapolates
/// past the output range if `v` is outside `[a0, a1]`.
#[inline]
pub fn map(v: f32, a0: f32, a1: f32, b0: f32, b1: f32) -> f32 {
    b0 + (v - a0) / (a1 - a0) * (b1 - b0)
}

/// Clamp `v` into the inclusive range `[min, max]`.
#[inline]
pub fn constrain(v: f32, min: f32, max: f32) -> f32 {
    v.max(min).min(max)
}

/// Normalize `v` from `[start, end]` to `[0.0, 1.0]`.
///
/// Does not clamp — combine with [`constrain`] if you need `[0, 1]`.
#[inline]
pub fn norm(v: f32, start: f32, end: f32) -> f32 {
    (v - start) / (end - start)
}

/// Euclidean distance between two 2D points.
#[inline]
pub fn dist(x1: f32, y1: f32, x2: f32, y2: f32) -> f32 {
    let dx = x2 - x1;
    let dy = y2 - y1;
    (dx * dx + dy * dy).sqrt()
}

/// Smooth Hermite interpolation between `edge0` and `edge1`.
///
/// Returns `0.0` when `x <= edge0`, `1.0` when `x >= edge1`, and a smooth
/// `t * t * (3 - 2t)` curve between them. Matches GLSL `smoothstep`.
#[inline]
pub fn smoothstep(edge0: f32, edge1: f32, x: f32) -> f32 {
    let t = constrain((x - edge0) / (edge1 - edge0), 0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

#[cfg(test)]
mod tests {
    use super::*;

    const EPS: f32 = 1e-6;

    #[test]
    fn lerp_endpoints_and_midpoint() {
        assert!((lerp(0.0, 10.0, 0.0) - 0.0).abs() < EPS);
        assert!((lerp(0.0, 10.0, 1.0) - 10.0).abs() < EPS);
        assert!((lerp(0.0, 10.0, 0.5) - 5.0).abs() < EPS);
    }

    #[test]
    fn lerp_extrapolates_past_unit_range() {
        assert!((lerp(0.0, 10.0, 2.0) - 20.0).abs() < EPS);
        assert!((lerp(0.0, 10.0, -1.0) + 10.0).abs() < EPS);
    }

    #[test]
    fn map_identity_and_invert() {
        assert!((map(5.0, 0.0, 10.0, 0.0, 10.0) - 5.0).abs() < EPS);
        // Inverting output range flips the value.
        assert!((map(0.0, 0.0, 10.0, 10.0, 0.0) - 10.0).abs() < EPS);
        assert!((map(10.0, 0.0, 10.0, 10.0, 0.0) - 0.0).abs() < EPS);
    }

    #[test]
    fn constrain_clamps_both_sides() {
        assert_eq!(constrain(-1.0, 0.0, 10.0), 0.0);
        assert_eq!(constrain(11.0, 0.0, 10.0), 10.0);
        assert_eq!(constrain(5.0, 0.0, 10.0), 5.0);
    }

    #[test]
    fn norm_zero_to_one() {
        assert!((norm(0.0, 0.0, 10.0) - 0.0).abs() < EPS);
        assert!((norm(10.0, 0.0, 10.0) - 1.0).abs() < EPS);
        assert!((norm(5.0, 0.0, 10.0) - 0.5).abs() < EPS);
    }

    #[test]
    fn dist_3_4_5_triangle() {
        assert!((dist(0.0, 0.0, 3.0, 4.0) - 5.0).abs() < EPS);
    }

    #[test]
    fn smoothstep_saturates() {
        assert_eq!(smoothstep(0.0, 1.0, -1.0), 0.0);
        assert_eq!(smoothstep(0.0, 1.0, 2.0), 1.0);
        // Hermite midpoint: 0.5 * 0.5 * (3 - 1) = 0.5
        assert!((smoothstep(0.0, 1.0, 0.5) - 0.5).abs() < EPS);
    }
}
