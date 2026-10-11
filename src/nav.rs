//! Moving a cursor with the controller, the same way on every overlay: the D-pad or a stick
//! steps once, then repeats while held, and a stick counts as a direction once it is pushed past
//! a point and until it comes back past a lower one.

use std::time::{Duration, Instant};

/// Holding a direction steps once, then repeats after this delay...
pub const REPEAT_DELAY: Duration = Duration::from_millis(350);
/// ...this often.
pub const REPEAT_EVERY: Duration = Duration::from_millis(100);
/// Stick deflection that counts as a direction, and where it lets go.
pub const STICK_PRESS: f32 = 0.6;
pub const STICK_RELEASE: f32 = 0.45;

/// One step: (x, y), each -1, 0 or 1; y is positive down.
pub type Dir = (i32, i32);

/// The direction of a D-pad button, if it is one.
pub fn dpad_dir(b: crate::config::Button) -> Option<Dir> {
    use crate::config::Button;
    match b {
        Button::DpadUp => Some((0, -1)),
        Button::DpadDown => Some((0, 1)),
        Button::DpadLeft => Some((-1, 0)),
        Button::DpadRight => Some((1, 0)),
        _ => None,
    }
}

/// A held direction and when it next repeats.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Repeat {
    held: Option<(Dir, Instant)>,
}

impl Repeat {
    /// `dir` was just pressed (and has stepped once): it repeats after the delay.
    pub fn start(&mut self, dir: Dir, now: Instant) {
        self.held = Some((dir, now + REPEAT_DELAY));
    }

    pub fn stop(&mut self) {
        self.held = None;
    }

    /// `dir` was let go: stops repeating if it's the one repeating.
    pub fn release(&mut self, dir: Dir) {
        if self.held.is_some_and(|(d, _)| d == dir) {
            self.held = None;
        }
    }

    /// The direction to step now, if a repeat is due. One step per call, however late, so a
    /// slow tick doesn't jump several places at once.
    pub fn due(&mut self, now: Instant) -> Option<Dir> {
        let (dir, at) = self.held?;
        if now < at {
            return None;
        }
        self.held = Some((dir, now + REPEAT_EVERY));
        Some(dir)
    }

    pub fn deadline(&self) -> Option<Instant> {
        self.held.map(|(_, at)| at)
    }
}

/// A stick read as a D-pad: the way it leans most, once past [`STICK_PRESS`], until it comes back
/// inside [`STICK_RELEASE`].
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct StickDir {
    dir: Option<Dir>,
}

impl StickDir {
    /// The stick is now at (`x`, `y`). Returns the new direction when it changed: `Some(None)`
    /// when it came back to the middle. Pass 0 for an axis the cursor doesn't move along.
    pub fn update(&mut self, (x, y): (f32, f32)) -> Option<Option<Dir>> {
        let threshold = if self.dir.is_some() { STICK_RELEASE } else { STICK_PRESS };
        let dir = if x.abs().max(y.abs()) < threshold {
            None
        } else if x.abs() > y.abs() {
            Some((x.signum() as i32, 0))
        } else {
            Some((0, y.signum() as i32))
        };
        if dir == self.dir {
            return None;
        }
        self.dir = dir;
        Some(dir)
    }

    pub fn clear(&mut self) {
        self.dir = None;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_held_direction_repeats_after_the_delay_one_step_at_a_time() {
        let t0 = Instant::now();
        let mut r = Repeat::default();
        r.start((0, 1), t0);
        assert_eq!(r.due(t0 + REPEAT_DELAY / 2), None);
        assert_eq!(r.due(t0 + REPEAT_DELAY), Some((0, 1)));
        assert_eq!(r.due(t0 + REPEAT_DELAY + REPEAT_EVERY / 2), None);
        assert_eq!(r.due(t0 + Duration::from_secs(5)), Some((0, 1)), "late: still one step");
        assert_eq!(r.due(t0 + Duration::from_secs(5)), None);
        r.release((1, 0));
        assert!(r.deadline().is_some(), "another direction's release leaves it");
        r.release((0, 1));
        assert_eq!(r.deadline(), None);
    }

    #[test]
    fn a_stick_leans_one_way_with_hysteresis() {
        let mut s = StickDir::default();
        assert_eq!(s.update((0.0, 0.3)), None, "a light push is nothing");
        assert_eq!(s.update((0.2, 0.9)), Some(Some((0, 1))));
        assert_eq!(s.update((0.0, 0.5)), None, "held inside the release margin");
        assert_eq!(s.update((0.95, 0.5)), Some(Some((1, 0))), "leaning further sideways turns it");
        assert_eq!(s.update((0.1, 0.1)), Some(None));
    }
}
