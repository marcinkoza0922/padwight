//! The numeric and timing substance of the FEEL checks in `docs/development/qa-plan.md`
//! (FEEL-1 to FEEL-3): response curves, deadzone, acceleration and outer-edge boost, flick stick
//! turn size, and gyro jitter and drift. These pin the arithmetic a person would otherwise have
//! to judge by feel; whether it *feels* right in a game stays a manual check.

use std::time::Instant;

use super::*;
use crate::{
    config::{DeadzoneShape, GyroConfig, GyroMode, MouseResponse, StickConfig},
    input::{Axis, InputEvent, MotionSample},
};

const FRAME: f32 = 0.004;

fn dead(size: f32, shape: DeadzoneShape) -> Deadzone {
    Deadzone { size, shape, anti: 0.0, buffer: 0.0 }
}

fn mag(p: (f32, f32)) -> f32 {
    p.0.hypot(p.1)
}

// ---- FEEL-1: response curve, deadzone, acceleration, outer-edge boost ----

#[test]
fn the_response_curve_is_monotonic_with_fixed_endpoints() {
    for curve in [0.5, 1.0, 1.5, 2.0, 3.0] {
        for boost in [0.0, 0.25, 0.5, 1.0, 2.0] {
            assert_eq!(stick::steady_speed(0.0, curve, boost), 0.0, "rest is rest (curve {curve}, boost {boost})");
            let full = stick::steady_speed(1.0, curve, boost);
            assert!((full - (1.0 + boost)).abs() < 1e-5, "full push is 1 + boost: {full} (curve {curve}, boost {boost})");
            let mut last = 0.0;
            for i in 0..=1000 {
                let speed = stick::steady_speed(i as f32 / 1000.0, curve, boost);
                assert!(speed >= last - 1e-6, "speed fell at {i}/1000: {last} -> {speed} (curve {curve}, boost {boost})");
                last = speed;
            }
        }
    }
}

#[test]
fn the_outer_boost_begins_at_the_outer_edge_and_does_not_step() {
    // Below the edge the boost changes nothing; the gain then rises continuously to 1 + boost.
    for i in 0..=900 {
        let m = i as f32 / 1000.0;
        assert_eq!(stick::steady_speed(m, 1.0, 1.0), stick::steady_speed(m, 1.0, 0.0), "at {m}");
    }
    let mut last = stick::steady_speed(0.9, 1.0, 1.0);
    for i in 901..=1000 {
        let now = stick::steady_speed(i as f32 / 1000.0, 1.0, 1.0);
        assert!(now - last < 0.05, "a step of {} at {i}/1000", now - last);
        last = now;
    }
}

#[test]
fn every_deadzone_shape_is_dead_inside_continuous_at_its_edge_and_full_at_the_rim() {
    for shape in [DeadzoneShape::Circle, DeadzoneShape::Square, DeadzoneShape::Cross] {
        for size in [0.05, 0.1, 0.2, 0.35] {
            let dz = dead(size, shape);
            assert_eq!(apply_deadzone(0.0, 0.0, dz), (0.0, 0.0), "centre rests ({shape:?} {size})");
            // Straight along an axis, where the three shapes agree.
            let inside = apply_deadzone(size * 0.999, 0.0, dz);
            assert_eq!(inside, (0.0, 0.0), "just inside is dead ({shape:?} {size})");
            let just_out = apply_deadzone(size + 0.002, 0.0, dz);
            assert!(just_out.0 > 0.0 && just_out.0 < 0.01, "no jump at the edge: {just_out:?} ({shape:?} {size})");
            let rim = apply_deadzone(1.0, 0.0, dz);
            assert!((rim.0 - 1.0).abs() < 1e-5, "full push stays full: {rim:?} ({shape:?} {size})");
            let mut last = 0.0;
            for i in 0..=1000 {
                let out = apply_deadzone(i as f32 / 1000.0, 0.0, dz).0;
                assert!(out >= last - 1e-6, "output fell at {i}/1000 ({shape:?} {size})");
                last = out;
            }
        }
    }
}

#[test]
fn a_circular_deadzone_gives_the_same_response_in_every_direction() {
    let dz = dead(0.2, DeadzoneShape::Circle);
    for degrees in (0..360).step_by(15) {
        let (s, c) = (degrees as f32).to_radians().sin_cos();
        let out = mag(apply_deadzone(0.6 * s, 0.6 * c, dz));
        assert!((out - 0.5).abs() < 1e-5, "{degrees} degrees: {out}");
    }
}

fn mouse_profile(response: MouseResponse, deadzone: f32, curve: f32) -> Profile {
    let mut p = Profile::passthrough("p");
    p.right_stick = StickConfig::new(StickAction::Mouse { speed: 1000.0, response, invert_y: false }, deadzone, curve);
    p
}

fn push(e: &mut Engine, p: &Profile, x: f32, y: f32) {
    let mut out = Vec::new();
    e.handle(p, InputEvent::Axis(Axis::RightX, x), Instant::now(), &mut out);
    e.handle(p, InputEvent::Axis(Axis::RightY, y), Instant::now(), &mut out);
}

/// Total mouse movement in pixels over `secs` of 4 ms ticks.
fn travel(e: &mut Engine, p: &Profile, secs: f32) -> (i32, i32) {
    let mut out = Vec::new();
    for _ in 0..(secs / FRAME).round() as usize {
        e.tick(p, FRAME, &mut out);
    }
    out.iter().fold((0, 0), |(x, y), o| match o {
        OutEvent::MouseMove(dx, dy) => (x + dx, y + dy),
        _ => (x, y),
    })
}

#[test]
fn a_mouse_stick_at_rest_or_inside_its_deadzone_never_moves_and_stops_ticking() {
    let p = mouse_profile(MouseResponse { accel: 1.0, outer_boost: 1.0, ..MouseResponse::default() }, 0.15, 2.0);
    let mut e = Engine::default();
    assert!(!e.needs_tick(&p));
    assert_eq!(travel(&mut e, &p, 1.0), (0, 0));
    push(&mut e, &p, 0.14, 0.0);
    assert!(!e.needs_tick(&p), "a push inside the deadzone is rest");
    assert_eq!(travel(&mut e, &p, 1.0), (0, 0));
    push(&mut e, &p, 0.5, 0.0);
    assert!(e.needs_tick(&p));
    push(&mut e, &p, 0.0, 0.0);
    assert!(!e.needs_tick(&p));
    assert_eq!(travel(&mut e, &p, 1.0), (0, 0), "nothing drifts on after letting go");
}

#[test]
fn mouse_speed_grows_with_the_push_through_deadzone_curve_and_boost() {
    let p = mouse_profile(MouseResponse { outer_boost: 0.5, ..MouseResponse::default() }, 0.1, 2.0);
    let mut last = 0;
    for step in 0..=100 {
        let mut e = Engine::default();
        push(&mut e, &p, step as f32 / 100.0, 0.0);
        let (px, _) = travel(&mut e, &p, 1.0);
        assert!(px >= last, "speed fell at {step}%: {last} -> {px}");
        // And never jumps: a 1% step in the stick is at most ~7% of full speed.
        assert!(px - last < 120, "a jump of {} px/s at {step}%", px - last);
        last = px;
    }
    // Full push is full speed times the boost: 1000 px/s * 1.5.
    assert!((1498..=1500).contains(&last), "{last}");
}

#[test]
fn acceleration_ramps_smoothly_up_to_its_ceiling_and_no_further() {
    let p = mouse_profile(MouseResponse { accel: 1.0, accel_ramp_ms: 400, ..MouseResponse::default() }, 0.0, 1.0);
    let mut e = Engine::default();
    push(&mut e, &p, 1.0, 0.0);
    let mut per_second = Vec::new();
    for _ in 0..25 {
        // 40 ms windows scaled to px/s.
        per_second.push(travel(&mut e, &p, 0.04).0 * 25);
    }
    for pair in per_second.windows(2) {
        assert!(pair[1] >= pair[0] - 25, "ramp went back: {pair:?}");
    }
    assert!(per_second[0] < 1300, "starts near plain speed: {}", per_second[0]);
    let ceiling = *per_second.last().unwrap();
    assert!((1950..=2050).contains(&ceiling), "settles at 1 + accel times the speed: {ceiling}");
}

// ---- FEEL-2: flick stick ----

fn flick_profile(full_turn_px: f32, smoothing_ms: u32) -> Profile {
    let StickAction::Flick { flick_threshold, flick_time_ms, forward_deadzone, vertical, vertical_speed, .. } = StickAction::flick() else {
        unreachable!()
    };
    let action = StickAction::Flick {
        full_turn_px,
        flick_threshold,
        flick_time_ms,
        rotate_smoothing_ms: smoothing_ms,
        forward_deadzone,
        vertical,
        vertical_speed,
    };
    let mut p = Profile::passthrough("p");
    p.right_stick = StickConfig::new(action, 0.1, 2.0);
    p
}

/// Sends the stick to `degrees` clockwise from up at full deflection, then ticks one frame.
fn aim(e: &mut Engine, p: &Profile, degrees: f32, out: &mut Vec<OutEvent>) {
    let (s, c) = degrees.to_radians().sin_cos();
    e.handle(p, InputEvent::Axis(Axis::RightX, s), Instant::now(), out);
    e.handle(p, InputEvent::Axis(Axis::RightY, -c), Instant::now(), out);
    e.tick(p, FRAME, out);
}

fn pixels(out: &[OutEvent]) -> i32 {
    out.iter().map(|o| if let OutEvent::MouseMove(dx, _) = o { *dx } else { 0 }).sum()
}

/// Flicks to `start` degrees, lets that finish, then turns the stick one whole circle (360
/// degrees in `frames` frames, clockwise or not) and returns the pixels that last part moved.
fn one_full_stick_turn(full_turn_px: f32, smoothing_ms: u32, frames: usize, clockwise: bool) -> i32 {
    let p = flick_profile(full_turn_px, smoothing_ms);
    let mut e = Engine::default();
    let mut settle = Vec::new();
    for _ in 0..100 {
        aim(&mut e, &p, 45.0, &mut settle);
    }
    let mut out = Vec::new();
    for i in 1..=frames {
        let step = 360.0 * i as f32 / frames as f32;
        aim(&mut e, &p, 45.0 + if clockwise { step } else { -step }, &mut out);
    }
    // Let go and let anything left over play out.
    let mut tail = Vec::new();
    e.handle(&p, InputEvent::Axis(Axis::RightX, 0.0), Instant::now(), &mut tail);
    e.handle(&p, InputEvent::Axis(Axis::RightY, 0.0), Instant::now(), &mut tail);
    for _ in 0..100 {
        e.tick(&p, FRAME, &mut tail);
    }
    pixels(&out) + pixels(&tail)
}

#[test]
fn one_full_stick_turn_is_exactly_one_configured_turn_size() {
    for full_turn_px in [1800.0, 3600.0, 7200.0, 12000.0] {
        for frames in [90, 360, 720] {
            let cw = one_full_stick_turn(full_turn_px, 0, frames, true);
            assert!((cw as f32 - full_turn_px).abs() <= 2.0, "clockwise, {full_turn_px} px over {frames} frames: moved {cw}");
            let ccw = one_full_stick_turn(full_turn_px, 0, frames, false);
            assert!((ccw as f32 + full_turn_px).abs() <= 2.0, "anticlockwise: moved {ccw}");
        }
    }
}

#[test]
fn rotation_smoothing_flushes_its_lag_when_the_stick_is_let_go() {
    // Smoothing delays the turn, but what the filter still holds at release is sent then, so a
    // full stick turn is one Turn size with or without smoothing.
    let exact = one_full_stick_turn(3600.0, 0, 360, true);
    let smoothed = one_full_stick_turn(3600.0, 50, 360, true);
    assert!((exact - 3600).abs() <= 2, "{exact}");
    assert!((smoothed - 3600).abs() <= 2, "smoothed turn: {smoothed}");
}

#[test]
fn a_flick_ends_cleanly_and_the_stick_stops_ticking() {
    let p = flick_profile(3600.0, 0);
    let mut e = Engine::default();
    let mut out = Vec::new();
    aim(&mut e, &p, 90.0, &mut out);
    for _ in 0..200 {
        e.tick(&p, FRAME, &mut out);
    }
    assert!((pixels(&out) - 900).abs() <= 1, "a quarter turn is a quarter of the size: {}", pixels(&out));
    let before = out.len();
    for _ in 0..500 {
        e.tick(&p, FRAME, &mut out);
    }
    assert_eq!(out.len(), before, "a held stick adds nothing once the flick is done");
    // Letting go ends it: nothing further, nothing to tick.
    e.handle(&p, InputEvent::Axis(Axis::RightX, 0.0), Instant::now(), &mut out);
    e.handle(&p, InputEvent::Axis(Axis::RightY, 0.0), Instant::now(), &mut out);
    e.tick(&p, FRAME, &mut out);
    assert!(!e.needs_tick(&p));
    let before = out.len();
    for _ in 0..500 {
        e.tick(&p, FRAME, &mut out);
    }
    assert_eq!(out.len(), before);
}

#[test]
fn the_test_turn_button_sends_exactly_the_pixels_asked_for() {
    for px in [1, 99, 100, 3600, -3600, 12345] {
        let steps = crate::output::test_turn_steps(px);
        assert_eq!(steps.len(), 100);
        assert_eq!(steps.iter().sum::<i32>(), px, "{px}");
        // Smooth: no step more than a pixel away from an even share.
        let share = px as f32 / 100.0;
        assert!(steps.iter().all(|s| (*s as f32 - share).abs() <= 1.0), "{px}: {steps:?}");
    }
}

// ---- FEEL-3: gyro jitter threshold, calibration ----

fn gyro_on(mode: GyroMode, noise_threshold: f32, auto_calibrate: bool) -> Profile {
    Profile {
        gyro: GyroConfig { enabled: true, mode, noise_threshold, auto_calibrate, ..GyroConfig::default() },
        ..Profile::passthrough("p")
    }
}

/// Mouse pixels over `seconds` of yaw at `rate` degrees/second, 250 Hz, the pad lying flat.
fn yaw_pixels(e: &mut Engine, p: &Profile, rate: f32, seconds: f32) -> i32 {
    let mut out = Vec::new();
    for _ in 0..(seconds * 250.0).round() as usize {
        e.motion(p, MotionSample { gyro: [0.0, rate, 0.0], accel: [0.0, 1.0, 0.0], dt: FRAME }, &mut out);
    }
    out.iter().map(|o| if let OutEvent::MouseMove(dx, _) = o { *dx } else { 0 }).sum()
}

#[test]
fn the_jitter_threshold_hides_drift_but_slow_real_aim_still_moves() {
    // 1 pixel per degree. A threshold of 2 deg/s scales anything slower down in proportion.
    let p = gyro_on(GyroMode::Mouse { sensitivity: 1.0 }, 2.0, false);
    let bare = gyro_on(GyroMode::Mouse { sensitivity: 1.0 }, 0.0, false);
    let drift = yaw_pixels(&mut Engine::default(), &p, 0.1, 20.0).abs();
    let undone = yaw_pixels(&mut Engine::default(), &bare, 0.1, 20.0).abs();
    assert!(undone >= 1, "without the threshold 0.1 deg/s for 20 s is {undone} px");
    assert!(drift <= 1, "with it the same drift is hidden: {drift} px");
    // Slow deliberate aim at 5 deg/s is above the threshold and keeps its full travel: 5 deg/s is
    // 5 px/s, so 4 s is 20 px.
    let slow = yaw_pixels(&mut Engine::default(), &p, 5.0, 4.0).abs();
    assert!((19..=21).contains(&slow), "{slow}");
    // And just under the threshold is scaled, not cut off: 1 deg/s at a 2 deg/s threshold is half.
    let under = yaw_pixels(&mut Engine::default(), &p, 1.0, 20.0).abs();
    assert!((8..=11).contains(&under), "{under}");
}

#[test]
fn the_jitter_scaling_never_steps_as_the_turn_gets_faster() {
    let p = gyro_on(GyroMode::Mouse { sensitivity: 1.0 }, 3.0, false);
    let mut last = 0;
    for tenth in 0..=60 {
        let px = yaw_pixels(&mut Engine::default(), &p, tenth as f32 / 10.0, 10.0).abs();
        assert!(px >= last, "travel fell at {} deg/s", tenth as f32 / 10.0);
        // 0.1 deg/s more is at most about a pixel per second more... over 10 s, 1 + rounding.
        assert!(px - last <= 3 + (tenth > 30) as i32 * 2, "a jump of {} px at {} deg/s", px - last, tenth as f32 / 10.0);
        last = px;
    }
}

#[test]
fn calibration_removes_a_synthetic_drift_and_keeps_slow_real_motion() {
    let p = gyro_on(GyroMode::Mouse { sensitivity: 1.0 }, 0.0, true);
    for drift in [0.5, 1.0, 2.0, 3.0] {
        let mut e = Engine::default();
        // Lying still on a drifting sensor: the first seconds move, then the drift is learned.
        yaw_pixels(&mut e, &p, drift, 8.0);
        let after = yaw_pixels(&mut e, &p, drift, 4.0).abs();
        assert!(after <= 1, "drift {drift} deg/s still moves {after} px in 4 s after calibrating");
        // Slow real motion of 20 deg/s on top of the same drift keeps its travel: 20 deg/s for
        // 2 s is 40 px.
        let moved = yaw_pixels(&mut e, &p, drift + 20.0, 2.0).abs();
        assert!((38..=42).contains(&moved), "drift {drift}: real slow aim moved {moved} px, expected 40");
    }
}

#[test]
fn calibration_does_not_learn_a_steady_slow_turn_as_drift() {
    // A pad that is being turned (the accelerometer disagrees with gravity) is not "set down", so
    // nothing is learned, however slow the turn is.
    let p = gyro_on(GyroMode::Mouse { sensitivity: 1.0 }, 0.0, true);
    let mut e = Engine::default();
    let mut out = Vec::new();
    let mut total = 0;
    for i in 0..(10.0 / FRAME) as usize {
        // Hand tremor on the accelerometer, well above the stillness limit, with a 3 deg/s turn.
        let wobble = if i % 2 == 0 { 0.05 } else { -0.05 };
        e.motion(&p, MotionSample { gyro: [0.0, 3.0, 0.0], accel: [wobble, 1.0, 0.0], dt: FRAME }, &mut out);
    }
    for o in &out {
        if let OutEvent::MouseMove(dx, _) = o {
            total += dx;
        }
    }
    assert!((-31..=-29).contains(&total), "30 degrees of real turning: {total} px");
}
