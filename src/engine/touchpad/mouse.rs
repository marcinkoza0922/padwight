//! The touchpad's pointer and scroll output: finger movement into pixels and wheel units, and
//! the glide after a flick.

use super::super::{Engine, WHEEL_UNITS_PER_NOTCH, take_whole};
use crate::output::OutEvent;

/// Time constant (seconds) of the glide slowing down, and the speed (pixels/second) it stops at.
const GLIDE_DECAY: f32 = 0.35;
const GLIDE_STOP: f32 = 30.0;
/// Notches scrolled by two fingers moving one pad width.
const SCROLL_NOTCHES_PER_WIDTH: f32 = 12.0;

impl Engine {
    /// Moves the pointer by `delta` pad widths at `px_per_width`.
    pub(super) fn point_with(&mut self, (dx, dy): (f32, f32), px_per_width: f32, out: &mut Vec<OutEvent>) {
        let (x, y) = take_whole(&mut self.touch.pointer_acc, dx * px_per_width, dy * px_per_width);
        if (x, y) != (0, 0) {
            out.push(OutEvent::MouseMove(x, y));
        }
    }

    /// Scrolls for one of `fingers` fingers moving by `delta` pad widths: together they scroll by
    /// how far they move on average. Fingers moving up scroll up.
    pub(super) fn scroll_with(&mut self, (dx, dy): (f32, f32), fingers: usize, out: &mut Vec<OutEvent>) {
        let units = SCROLL_NOTCHES_PER_WIDTH * WHEEL_UNITS_PER_NOTCH / fingers as f32;
        let (h, v) = take_whole(&mut self.touch.scroll_acc, dx * units, -dy * units);
        if (h, v) != (0, 0) {
            out.push(OutEvent::Wheel { vertical: v, horizontal: h });
        }
    }

    /// Moves the pointer on after a flick, slowing down until it stops.
    pub(in crate::engine) fn tick_touch(&mut self, dt: f32, out: &mut Vec<OutEvent>) {
        let Some((vx, vy)) = self.touch.glide else { return };
        self.point_with((vx * dt, vy * dt), 1.0, out);
        let keep = (-dt / GLIDE_DECAY).exp();
        let (vx, vy) = (vx * keep, vy * keep);
        self.touch.glide = (vx.hypot(vy) >= GLIDE_STOP).then_some((vx, vy));
    }
}
