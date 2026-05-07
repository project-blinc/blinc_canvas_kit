use crate::viewport::CanvasViewport;

/// Pan controller that processes drag events and tracks velocity for momentum.
///
/// Velocity is tracked via exponential moving average (EMA) matching
/// the scroll physics in `blinc_layout::ScrollPhysics`.
#[derive(Clone, Debug)]
pub struct PanController {
    /// Momentum velocity in content-space px/s (X axis)
    velocity_x: f32,
    /// Momentum velocity in content-space px/s (Y axis)
    velocity_y: f32,
    /// EMA smoothing factor for velocity tracking (0.0-1.0)
    smoothing: f32,
    /// Deceleration rate in px/s²
    deceleration: f32,
    /// Whether momentum animation is active
    pub is_animating: bool,
    /// Last drag delta for velocity estimation
    last_dx: f32,
    last_dy: f32,
}

impl Default for PanController {
    fn default() -> Self {
        Self::new()
    }
}

impl PanController {
    pub fn new() -> Self {
        Self {
            velocity_x: 0.0,
            velocity_y: 0.0,
            smoothing: 0.3,
            deceleration: 1500.0,
            is_animating: false,
            last_dx: 0.0,
            last_dy: 0.0,
        }
    }

    /// Configure the EMA smoothing factor (default 0.3).
    pub fn with_smoothing(mut self, smoothing: f32) -> Self {
        self.smoothing = smoothing.clamp(0.01, 1.0);
        self
    }

    /// Configure the deceleration rate in px/s² (default 1500.0).
    pub fn with_deceleration(mut self, deceleration: f32) -> Self {
        self.deceleration = deceleration.max(0.0);
        self
    }

    /// Handle a DRAG event — apply delta to viewport, track velocity.
    ///
    /// `dx`, `dy`: drag delta in screen pixels from the EventContext.
    pub fn on_drag(&mut self, viewport: &mut CanvasViewport, dx: f32, dy: f32) {
        // Apply pan (pan_by divides by zoom internally)
        viewport.pan_by(dx - self.last_dx, dy - self.last_dy);

        // Track velocity via EMA (estimate px/frame → px/s at ~60fps)
        let instant_vx = (dx - self.last_dx) * 60.0;
        let instant_vy = (dy - self.last_dy) * 60.0;
        self.velocity_x = self.velocity_x * (1.0 - self.smoothing) + instant_vx * self.smoothing;
        self.velocity_y = self.velocity_y * (1.0 - self.smoothing) + instant_vy * self.smoothing;

        self.last_dx = dx;
        self.last_dy = dy;
        self.is_animating = false;
    }

    /// Handle DRAG_END — start momentum if velocity is sufficient.
    pub fn on_drag_end(&mut self) {
        self.last_dx = 0.0;
        self.last_dy = 0.0;

        let speed = (self.velocity_x * self.velocity_x + self.velocity_y * self.velocity_y).sqrt();
        // Only start momentum if speed exceeds threshold (50 px/s)
        self.is_animating = speed > 50.0;

        if !self.is_animating {
            self.velocity_x = 0.0;
            self.velocity_y = 0.0;
        }
    }

    /// Tick momentum deceleration. Call each frame while `is_animating`.
    ///
    /// `dt`: time delta in seconds (e.g. 1.0/60.0 for 60fps).
    ///
    /// Returns `true` if still animating.
    pub fn tick(&mut self, viewport: &mut CanvasViewport, dt: f32) -> bool {
        if !self.is_animating {
            return false;
        }

        // Apply velocity to viewport
        viewport.pan_by(self.velocity_x * dt, self.velocity_y * dt);

        // Decelerate
        let speed = (self.velocity_x * self.velocity_x + self.velocity_y * self.velocity_y).sqrt();
        if speed < 1.0 {
            self.stop();
            return false;
        }

        let decel = self.deceleration * dt;
        let factor = ((speed - decel) / speed).max(0.0);
        self.velocity_x *= factor;
        self.velocity_y *= factor;

        true
    }

    /// Stop momentum immediately.
    pub fn stop(&mut self) {
        self.velocity_x = 0.0;
        self.velocity_y = 0.0;
        self.is_animating = false;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pan_controller_default() {
        let pc = PanController::new();
        assert_eq!(pc.velocity_x, 0.0);
        assert_eq!(pc.velocity_y, 0.0);
        assert!(!pc.is_animating);
    }

    #[test]
    fn test_on_drag_applies_delta() {
        let mut pc = PanController::new();
        let mut vp = CanvasViewport::new();

        pc.on_drag(&mut vp, 10.0, 20.0);
        assert!((vp.pan_x - 10.0).abs() < 1e-3);
        assert!((vp.pan_y - 20.0).abs() < 1e-3);
    }

    #[test]
    fn test_drag_end_starts_momentum() {
        let mut pc = PanController::new();
        let mut vp = CanvasViewport::new();

        // Simulate fast drag
        pc.on_drag(&mut vp, 5.0, 0.0);
        pc.on_drag(&mut vp, 15.0, 0.0);
        pc.on_drag(&mut vp, 30.0, 0.0);
        pc.on_drag_end();

        assert!(pc.is_animating);
        assert!(pc.velocity_x.abs() > 0.0);
    }

    #[test]
    fn test_momentum_decelerates_to_zero() {
        let mut pc = PanController::new();
        let mut vp = CanvasViewport::new();

        // Simulate drag and release
        pc.on_drag(&mut vp, 10.0, 0.0);
        pc.on_drag(&mut vp, 25.0, 0.0);
        pc.on_drag_end();

        // Tick until stopped
        let dt = 1.0 / 60.0;
        let mut frames = 0;
        while pc.tick(&mut vp, dt) {
            frames += 1;
            if frames > 600 {
                // 10 seconds max
                break;
            }
        }

        assert!(!pc.is_animating);
        assert!(frames < 600, "momentum should settle within 10 seconds");
    }

    #[test]
    fn test_stop() {
        let mut pc = PanController::new();
        pc.velocity_x = 500.0;
        pc.is_animating = true;
        pc.stop();
        assert!(!pc.is_animating);
        assert_eq!(pc.velocity_x, 0.0);
    }
}
