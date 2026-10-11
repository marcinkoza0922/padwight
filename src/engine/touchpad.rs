//! Touchpad input: the click runs the profile's touchpad action like a button press, and finger
//! movement drives the mouse pointer.

use std::time::Instant;

use crate::{
    config::{Profile, SpeedCurve, TouchpadMotion},
    input::TouchpadEvent,
    output::OutEvent,
};

use super::{Engine, Source, take_whole};

impl Engine {
    /// Handles one touchpad event.
    pub(super) fn touchpad(&mut self, profile: &Profile, ev: TouchpadEvent, now: Instant, out: &mut Vec<OutEvent>) {
        match ev {
            TouchpadEvent::Click(pressed) => self.digital(&Source::Touchpad, &profile.touchpad.click, pressed, out),
            TouchpadEvent::Move(dx, dy) => {
                if let TouchpadMotion::Mouse { speed, curve } = profile.touchpad.motion {
                    let speed = speed * self.finger_gain(&curve, dx.hypot(dy), now);
                    let (x, y) = take_whole(&mut self.touchpad_acc, dx * speed, dy * speed);
                    if (x, y) != (0, 0) {
                        out.push(OutEvent::MouseMove(x, y));
                    }
                }
            }
        }
    }
}

/// A finger movement report's usual interval (seconds), assumed for the first one of a stroke.
const REPORT_INTERVAL: f32 = 0.008;
/// A pause this long (seconds) starts a new stroke.
const STROKE_GAP: f32 = 0.1;
/// Time constant (seconds) smoothing the finger's speed, so one uneven report doesn't jolt it.
const SPEED_SMOOTHING: f32 = 0.03;

impl Engine {
    /// The touchpad curve's gain for a movement of `distance` pad widths at `now`.
    fn finger_gain(&mut self, curve: &SpeedCurve, distance: f32, now: Instant) -> f32 {
        if curve.is_flat() {
            return 1.0;
        }
        let (last, smoothed) = self.touchpad_speed;
        let dt = last.map(|t| now.duration_since(t).as_secs_f32()).filter(|dt| *dt < STROKE_GAP);
        let speed = distance / dt.unwrap_or(REPORT_INTERVAL).max(1e-3);
        let smoothed = match dt {
            Some(dt) => smoothed + (speed - smoothed) * (1.0 - (-dt / SPEED_SMOOTHING).exp()),
            None => speed,
        };
        self.touchpad_speed = (Some(now), smoothed);
        curve.gain(smoothed)
    }
}

#[cfg(test)]
mod tests {
    use crate::config::{ButtonAction, MouseButton, TouchpadConfig};

    use super::*;

    fn profile(touchpad: TouchpadConfig) -> Profile {
        Profile { touchpad, ..Profile::passthrough("test") }
    }

    fn run(e: &mut Engine, p: &Profile, ev: TouchpadEvent) -> Vec<OutEvent> {
        let mut out = Vec::new();
        e.touchpad(p, ev, Instant::now(), &mut out);
        out
    }

    #[test]
    fn movement_becomes_pointer_motion_scaled_by_speed() {
        let p = profile(TouchpadConfig { motion: TouchpadMotion::Mouse { speed: 1000.0, curve: SpeedCurve::default() }, ..TouchpadConfig::default() });
        let mut e = Engine::default();
        assert_eq!(run(&mut e, &p, TouchpadEvent::Move(0.1, -0.05)), vec![OutEvent::MouseMove(100, -50)]);
        // Fractions carry over until they make a whole pixel.
        assert!(run(&mut e, &p, TouchpadEvent::Move(0.0004, 0.0)).is_empty());
        assert_eq!(run(&mut e, &p, TouchpadEvent::Move(0.0006, 0.0)), vec![OutEvent::MouseMove(1, 0)]);
    }

    #[test]
    fn the_curve_slows_slow_fingers_and_speeds_up_fast_ones() {
        // A finger's pad widths/second feel like 40 times as many degrees/second of gyro.
        let curve = crate::config::CurvePreset::Aggressive.speed_curve(0.025).unwrap();
        let p = profile(TouchpadConfig { motion: TouchpadMotion::Mouse { speed: 1000.0, curve }, ..TouchpadConfig::default() });
        let stroke = |step: f32| {
            let mut e = Engine::default();
            let t0 = Instant::now();
            let mut out = Vec::new();
            for i in 0..20 {
                e.touchpad(&p, TouchpadEvent::Move(step, 0.0), t0 + std::time::Duration::from_millis(8 * i), &mut out);
            }
            out.iter().map(|ev| if let OutEvent::MouseMove(x, _) = ev { *x } else { 0 }).sum::<i32>()
        };
        // 0.0008 pad widths every 8 ms is 0.1 widths/s, slow: half speed, 8 px instead of 16.
        assert!((7..=9).contains(&stroke(0.0008)), "{}", stroke(0.0008));
        // 0.02 every 8 ms is 2.5 widths/s, fast: double, 800 px instead of 400.
        assert!((790..=800).contains(&stroke(0.02)), "{}", stroke(0.02));
    }

    #[test]
    fn motion_off_moves_nothing() {
        let p = profile(TouchpadConfig { motion: TouchpadMotion::Off, ..TouchpadConfig::default() });
        assert!(run(&mut Engine::default(), &p, TouchpadEvent::Move(0.5, 0.5)).is_empty());
    }

    #[test]
    fn click_runs_its_action_on_press_and_releases_it() {
        let p = profile(TouchpadConfig { click: ButtonAction::Mouse(MouseButton::Right), ..TouchpadConfig::default() });
        let mut e = Engine::default();
        assert_eq!(run(&mut e, &p, TouchpadEvent::Click(true)), vec![OutEvent::MouseButton(MouseButton::Right, true)]);
        assert_eq!(run(&mut e, &p, TouchpadEvent::Click(false)), vec![OutEvent::MouseButton(MouseButton::Right, false)]);
    }

    #[test]
    fn a_disabled_click_does_nothing() {
        let p = profile(TouchpadConfig { click: ButtonAction::Disabled, ..TouchpadConfig::default() });
        let mut e = Engine::default();
        assert!(run(&mut e, &p, TouchpadEvent::Click(true)).is_empty());
    }
}
