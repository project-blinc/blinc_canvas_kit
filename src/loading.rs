//! Helpers for scenes that load asynchronously.
//!
//! The only things here are pieces that are genuinely annoying to
//! roll by hand — everything trivial (a "Loading…" label, a darkened
//! backdrop, a spinner) belongs in the caller's own UI code.
//!
//! - [`AutoFramer`] writes a fitted [`OrbitCamera`] back to its
//!   state signal exactly once when the scene's AABB first becomes
//!   available. Subsequent calls are no-ops so the user's drag /
//!   zoom is preserved.
//! - [`fit_aabb`] is the pure geometry the framer uses, exposed
//!   separately for callers that want to precompute a framing camera
//!   off the main thread or for non-`OrbitCamera` camera types.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use blinc_core::{State, Vec3};

use crate::scene3d::OrbitCamera;

/// One-shot helper for fitting an [`OrbitCamera`] to a scene AABB.
///
/// Hold one across rebuilds (typically inside the same struct that
/// holds the scene's [`OnceLock`](std::sync::OnceLock)-backed handle),
/// and call [`Self::apply`] every frame from the render closure with
/// the current AABB. The first call where `aabb` is `Some` writes a
/// fitted camera back through the supplied state signal; every later
/// call is a no-op. The user's drag / zoom inputs are written to the
/// same signal and naturally take over from that point.
///
/// `Clone` is cheap (one `Arc` bump) — clone into render-closure
/// captures freely.
#[derive(Clone, Default)]
pub struct AutoFramer {
    framed: Arc<AtomicBool>,
}

impl AutoFramer {
    pub fn new() -> Self {
        Self::default()
    }

    /// Has the camera been auto-framed yet? Useful when you want to
    /// gate other "first render of the scene" effects.
    pub fn is_framed(&self) -> bool {
        self.framed.load(Ordering::Acquire)
    }

    /// If the AABB is `Some` and we haven't framed yet, compute a
    /// fitted [`OrbitCamera`] and write it back through `camera`.
    /// Azimuth / elevation are preserved from the existing camera so
    /// a previously chosen viewing angle isn't reset.
    pub fn apply(&self, camera: &State<OrbitCamera>, aabb: Option<([f32; 3], [f32; 3])>) {
        if self.framed.load(Ordering::Acquire) {
            return;
        }
        let Some(aabb) = aabb else {
            return;
        };
        if self.framed.swap(true, Ordering::AcqRel) {
            return;
        }
        let (target, distance) = fit_aabb(aabb);
        let current = camera.get();
        camera.set(current.with_target(target).with_distance(distance));
    }
}

/// Compute `(target, distance)` to fit `aabb` in a 45° FOV viewport.
/// The target is the AABB center; distance is the bounding-box
/// diagonal × 1.1 (slight padding past the corners), clamped to a
/// minimum of 1.0 to avoid degenerate near-clip cases on tiny scenes.
pub fn fit_aabb(aabb: ([f32; 3], [f32; 3])) -> (Vec3, f32) {
    let (min, max) = aabb;
    let center = Vec3::new(
        (min[0] + max[0]) * 0.5,
        (min[1] + max[1]) * 0.5,
        (min[2] + max[2]) * 0.5,
    );
    let dx = max[0] - min[0];
    let dy = max[1] - min[1];
    let dz = max[2] - min[2];
    let diag = (dx * dx + dy * dy + dz * dz).sqrt();
    let distance = (diag * 1.1).max(1.0);
    (center, distance)
}
