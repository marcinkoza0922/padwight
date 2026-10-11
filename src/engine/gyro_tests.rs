//! Gyro: modes, activation, the spaces, shaping and filtering.

use std::time::Instant;

use super::*;
use crate::{
    config::{Button, CurvePreset, GyroAxes, GyroHorizontal},
    input::InputEvent,
};

fn run(e: &mut Engine, p: &Profile, ev: InputEvent) -> Vec<OutEvent> {
    let mut out = Vec::new();
    e.handle(p, ev, Instant::now(), &mut out);
    out
}

fn axis(e: &mut Engine, p: &Profile, axis: Axis, v: f32) -> Vec<OutEvent> {
    run(e, p, InputEvent::Axis(axis, v))
}

/// The last value `out` sends on the virtual pad's `axis`.
fn last_pad(out: &[OutEvent], axis: Axis) -> Option<f32> {
    out.iter().rev().find_map(|ev| match ev {
        OutEvent::PadAxis(a, v) if *a == axis => Some(*v),
        _ => None,
    })
}

fn gyro_profile(gyro: GyroConfig) -> Profile {
    Profile { gyro, ..Profile::passthrough("p") }
}

/// `seconds` of turning at constant rates (degrees/second), held flat, at 250 Hz.
#[expect(clippy::too_many_arguments, reason = "predates the size lints")]
fn turn(e: &mut Engine, p: &Profile, pitch: f32, yaw: f32, roll: f32, seconds: f32) -> Vec<OutEvent> {
    let mut out = Vec::new();
    for _ in 0..(seconds * 250.0).round() as usize {
        let sample = MotionSample { gyro: [pitch, yaw, roll], accel: [0.0, 1.0, 0.0], dt: 0.004 };
        e.motion(p, sample, &mut out);
    }
    out
}

fn mouse_total(out: &[OutEvent]) -> (i32, i32) {
    out.iter().fold((0, 0), |(x, y), ev| match ev {
        OutEvent::MouseMove(dx, dy) => (x + dx, y + dy),
        _ => (x, y),
    })
}

fn mouse_gyro(activation: GyroActivation) -> GyroConfig {
    GyroConfig { mode: GyroMode::Mouse { sensitivity: 10.0 }, activation, ..GyroConfig::default() }
}

#[test]
fn gyro_mouse_turns_degrees_into_pixels() {
    let p = gyro_profile(mouse_gyro(GyroActivation::Always));
    let mut e = Engine::default();
    // Turning left (positive yaw) 30 degrees, then tilting the front up 10 degrees.
    let (dx, dy) = mouse_total(&turn(&mut e, &p, 0.0, 60.0, 0.0, 0.5));
    assert!((-301..=-299).contains(&dx) && dy == 0, "{dx} {dy}");
    let (dx, dy) = mouse_total(&turn(&mut e, &p, 20.0, 0.0, 0.0, 0.5));
    assert!(dx == 0 && (-101..=-99).contains(&dy), "{dx} {dy}");
}

#[test]
fn gyro_ignores_jitter_below_threshold() {
    let p = gyro_profile(GyroConfig { noise_threshold: 2.0, ..mouse_gyro(GyroActivation::Always) });
    let mut e = Engine::default();
    // 0.2 deg/s of drift for 10 s would be 20 px untreated; tightening shrinks it 10x.
    let (dx, _) = mouse_total(&turn(&mut e, &p, 0.0, 0.2, 0.0, 10.0));
    assert!(dx.abs() <= 2, "{dx}");
}

#[test]
fn gyro_only_while_left_trigger_held() {
    let p = gyro_profile(mouse_gyro(GyroActivation::WhileHeld(GyroInput::LeftTrigger)));
    let mut e = Engine::default();
    assert_eq!(mouse_total(&turn(&mut e, &p, 0.0, 60.0, 0.0, 0.5)), (0, 0));
    axis(&mut e, &p, Axis::LeftTrigger, 0.8);
    assert_ne!(mouse_total(&turn(&mut e, &p, 0.0, 60.0, 0.0, 0.5)), (0, 0));
    axis(&mut e, &p, Axis::LeftTrigger, 0.1);
    assert_eq!(mouse_total(&turn(&mut e, &p, 0.0, 60.0, 0.0, 0.5)), (0, 0));
}

#[test]
fn gyro_clutch_and_toggle() {
    let clutch = gyro_profile(mouse_gyro(GyroActivation::UnlessHeld(GyroInput::Button(Button::RightStick))));
    let mut e = Engine::default();
    run(&mut e, &clutch, InputEvent::Button(Button::RightStick, true));
    assert_eq!(mouse_total(&turn(&mut e, &clutch, 0.0, 60.0, 0.0, 0.5)), (0, 0));
    run(&mut e, &clutch, InputEvent::Button(Button::RightStick, false));
    assert_ne!(mouse_total(&turn(&mut e, &clutch, 0.0, 60.0, 0.0, 0.5)), (0, 0));

    let toggle = gyro_profile(mouse_gyro(GyroActivation::Toggle(GyroInput::Button(Button::Select))));
    let mut e = Engine::default();
    let tap = |e: &mut Engine| {
        run(e, &toggle, InputEvent::Button(Button::Select, true));
        run(e, &toggle, InputEvent::Button(Button::Select, false));
    };
    assert_eq!(mouse_total(&turn(&mut e, &toggle, 0.0, 60.0, 0.0, 0.2)), (0, 0));
    tap(&mut e);
    assert_ne!(mouse_total(&turn(&mut e, &toggle, 0.0, 60.0, 0.0, 0.2)), (0, 0));
    tap(&mut e);
    assert_eq!(mouse_total(&turn(&mut e, &toggle, 0.0, 60.0, 0.0, 0.2)), (0, 0));
}

#[test]
fn gyro_stick_adds_to_physical_stick_and_clears_when_off() {
    let mut p = gyro_profile(GyroConfig {
        mode: GyroMode::Stick { stick: Stick::Right, full_rate: 100.0, anti_deadzone: 0.0 },
        activation: GyroActivation::WhileHeld(GyroInput::LeftTrigger),
        ..GyroConfig::default()
    });
    p.right_stick.deadzone = 0.0; // so the physical 0.3 passes through unchanged
    let mut e = Engine::default();
    axis(&mut e, &p, Axis::RightX, 0.3);
    axis(&mut e, &p, Axis::LeftTrigger, 1.0);
    // Turning right at 50 deg/s = half deflection, plus the physical 0.3.
    let out = turn(&mut e, &p, 0.0, -50.0, 0.0, 0.02);
    assert!(out.iter().any(|ev| matches!(ev, OutEvent::PadAxis(Axis::RightX, v) if (v - 0.8).abs() < 1e-3)), "{out:?}");
    // Letting go of LT drops the gyro part immediately.
    let out = axis(&mut e, &p, Axis::LeftTrigger, 0.0);
    assert!(out.contains(&OutEvent::PadAxis(Axis::RightX, 0.3)), "{out:?}");
}

#[test]
fn gyro_steering_follows_tilt_and_recenters() {
    let p = gyro_profile(GyroConfig {
        mode: GyroMode::Steering { stick: Stick::Left, max_angle: 45.0 },
        recenter: Some(GyroInput::Button(Button::Select)),
        ..GyroConfig::default()
    });
    let mut e = Engine::default();
    let tilt = |e: &mut Engine, degrees: f32| {
        let r = degrees.to_radians();
        let mut out = Vec::new();
        for _ in 0..100 {
            let sample = MotionSample { gyro: [0.0; 3], accel: [r.sin(), r.cos(), 0.0], dt: 0.004 };
            e.motion(&p, sample, &mut out);
        }
        out.iter().rev().find_map(|ev| match ev {
            OutEvent::PadAxis(Axis::LeftX, v) => Some(*v),
            _ => None,
        })
    };
    let x = tilt(&mut e, 22.5).unwrap();
    assert!((x - 0.5).abs() < 0.02, "half of 45 degrees: {x}");
    // Recenter at the current tilt: it now counts as straight.
    run(&mut e, &p, InputEvent::Button(Button::Select, true));
    let x = tilt(&mut e, 22.5).unwrap_or(0.5);
    assert!(x.abs() < 0.02, "{x}");
    let x = tilt(&mut e, 67.5).unwrap();
    assert!((x - 1.0).abs() < 0.02, "{x}");
}

#[test]
fn gyro_roll_and_inversion_options() {
    let p = gyro_profile(GyroConfig {
        horizontal: GyroHorizontal::Roll,
        invert_x: true,
        ..mouse_gyro(GyroActivation::Always)
    });
    let mut e = Engine::default();
    // Yaw is ignored in roll mode; inverted roll moves right.
    assert_eq!(mouse_total(&turn(&mut e, &p, 0.0, 60.0, 0.0, 0.5)), (0, 0));
    let (dx, _) = mouse_total(&turn(&mut e, &p, 0.0, 0.0, 60.0, 0.5));
    assert!(dx > 290, "{dx}");
}

/// Like [`turn`], with the controller held at `accel` (g, pointing up).
fn spin(e: &mut Engine, p: &Profile, rates: [f32; 3], accel: [f32; 3], seconds: f32) -> Vec<OutEvent> {
    let mut out = Vec::new();
    for _ in 0..(seconds * 250.0).round() as usize {
        e.motion(p, MotionSample { gyro: rates, accel, dt: 0.004 }, &mut out);
    }
    out
}

#[test]
fn the_curve_slows_slow_turns_and_speeds_up_fast_ones() {
    let curve = CurvePreset::Aggressive.speed_curve(1.0).unwrap();
    let p = gyro_profile(GyroConfig { curve, noise_threshold: 0.0, ..mouse_gyro(GyroActivation::Always) });
    let mut e = Engine::default();
    // 4 deg/s for 1 s: half speed, 20 px instead of 40.
    let (dx, _) = mouse_total(&turn(&mut e, &p, 0.0, -4.0, 0.0, 1.0));
    assert!((19..=21).contains(&dx), "{dx}");
    // 100 deg/s for 0.5 s: double, 1000 px instead of 500.
    let (dx, _) = mouse_total(&turn(&mut e, &p, 0.0, -100.0, 0.0, 0.5));
    assert!((999..=1001).contains(&dx), "{dx}");
}

#[test]
fn each_axis_has_its_own_sensitivity_and_can_be_switched_off() {
    let scaled = gyro_profile(GyroConfig { x_scale: 0.5, y_scale: 2.0, ..mouse_gyro(GyroActivation::Always) });
    let mut e = Engine::default();
    let (dx, dy) = mouse_total(&turn(&mut e, &scaled, -20.0, -20.0, 0.0, 0.5));
    assert!((49..=51).contains(&dx) && (199..=201).contains(&dy), "{dx} {dy}");

    let vertical = gyro_profile(GyroConfig { axes: GyroAxes::Vertical, ..mouse_gyro(GyroActivation::Always) });
    let mut e = Engine::default();
    let (dx, dy) = mouse_total(&turn(&mut e, &vertical, -20.0, -60.0, 0.0, 0.5));
    assert!(dx == 0 && (99..=101).contains(&dy), "{dx} {dy}");
}

#[test]
fn real_world_sensitivity_matches_the_games_full_turn() {
    let p = gyro_profile(GyroConfig { full_turn_px: Some(3600.0), ..mouse_gyro(GyroActivation::Always) });
    let p = Profile { gyro: GyroConfig { mode: GyroMode::Mouse { sensitivity: 1.0 }, ..p.gyro }, ..p };
    let mut e = Engine::default();
    // Turning 90 degrees moves a quarter of the full turn.
    let (dx, _) = mouse_total(&turn(&mut e, &p, 0.0, -90.0, 0.0, 1.0));
    assert!((899..=901).contains(&dx), "{dx}");
}

#[test]
fn player_space_turns_with_the_player_however_the_pad_is_tilted() {
    let p = gyro_profile(GyroConfig { horizontal: GyroHorizontal::PlayerSpace, ..mouse_gyro(GyroActivation::Always) });
    // Held with the face toward the player (pointing up the controller's Z), turning about the
    // player's up shows as roll in the controller's frame.
    let mut e = Engine::default();
    let (dx, _) = mouse_total(&spin(&mut e, &p, [0.0, 0.0, -60.0], [0.0, 0.0, 1.0], 0.5));
    assert!((299..=301).contains(&dx), "{dx}");
    // Flat, the same as yaw.
    let mut e = Engine::default();
    let (dx, _) = mouse_total(&spin(&mut e, &p, [0.0, -60.0, 0.0], [0.0, 1.0, 0.0], 0.5));
    assert!((299..=301).contains(&dx), "{dx}");
}

#[test]
fn world_space_reads_pitch_about_the_horizon() {
    let p = gyro_profile(GyroConfig { horizontal: GyroHorizontal::WorldSpace, ..mouse_gyro(GyroActivation::Always) });
    let mut e = Engine::default();
    // Flat, pitch is pitch and yaw is yaw.
    let (dx, dy) = mouse_total(&spin(&mut e, &p, [-20.0, -60.0, 0.0], [0.0, 1.0, 0.0], 0.5));
    assert!((299..=301).contains(&dx) && (99..=101).contains(&dy), "{dx} {dy}");
}

#[test]
fn smoothing_steadies_slow_aim_but_not_fast_turns() {
    let p = gyro_profile(GyroConfig { smoothing_threshold: 10.0, smoothing_ms: 200, noise_threshold: 0.0, ..mouse_gyro(GyroActivation::Always) });
    let mut e = Engine::default();
    // Slow turning lags: 4 deg/s for 0.2 s would be 8 px unsmoothed.
    let slow = mouse_total(&turn(&mut e, &p, 0.0, -4.0, 0.0, 0.2)).0;
    assert!(slow < 6, "{slow}");
    let mut e = Engine::default();
    let (fast, _) = mouse_total(&turn(&mut e, &p, 0.0, -60.0, 0.0, 0.5));
    assert!((299..=301).contains(&fast), "{fast}");
}

#[test]
fn pushing_the_chosen_stick_pauses_the_gyro() {
    let p = gyro_profile(GyroConfig { pause_with_stick: Some(Stick::Right), ..mouse_gyro(GyroActivation::Always) });
    let mut e = Engine::default();
    axis(&mut e, &p, Axis::RightX, 0.8);
    assert_eq!(mouse_total(&turn(&mut e, &p, 0.0, 60.0, 0.0, 0.5)), (0, 0));
    axis(&mut e, &p, Axis::RightX, 0.0);
    assert_ne!(mouse_total(&turn(&mut e, &p, 0.0, 60.0, 0.0, 0.5)), (0, 0));
}

#[test]
fn auto_calibration_learns_drift_while_the_pad_lies_still() {
    let p = gyro_profile(GyroConfig { auto_calibrate: true, noise_threshold: 0.0, ..mouse_gyro(GyroActivation::Always) });
    let mut e = Engine::default();
    // Lying still with 2 deg/s of drift: it learns it after half a second, then stops moving.
    spin(&mut e, &p, [0.0, 2.0, 0.0], [0.0, 1.0, 0.0], 6.0);
    let (dx, _) = mouse_total(&spin(&mut e, &p, [0.0, 2.0, 0.0], [0.0, 1.0, 0.0], 2.0));
    assert!(dx.abs() <= 1, "{dx}");
    // Real turning still moves.
    let (dx, _) = mouse_total(&spin(&mut e, &p, [0.0, 62.0, 0.0], [0.0, 1.0, 0.0], 0.5));
    assert!((-301..=-299).contains(&dx), "{dx}");
    // Without it, drift moves the aim.
    let plain = gyro_profile(GyroConfig { noise_threshold: 0.0, ..mouse_gyro(GyroActivation::Always) });
    let (dx, _) = mouse_total(&spin(&mut Engine::default(), &plain, [0.0, 2.0, 0.0], [0.0, 1.0, 0.0], 2.0));
    assert!(dx <= -39, "{dx}");
}

fn position_gyro(lock_at_edge: bool) -> Profile {
    let mut p = gyro_profile(GyroConfig {
        mode: GyroMode::Position { stick: Stick::Right, max_angle: 20.0, anti_deadzone: 0.0, lock_at_edge },
        noise_threshold: 0.0,
        recenter: Some(GyroInput::Button(Button::Select)),
        ..GyroConfig::default()
    });
    p.right_stick.deadzone = 0.0;
    p
}

#[test]
fn position_mode_deflects_the_stick_by_how_far_the_pad_has_turned() {
    let p = position_gyro(false);
    let mut e = Engine::default();
    // 10 degrees to the right of 20 is half deflection, and it stays there while still.
    let out = turn(&mut e, &p, 0.0, -20.0, 0.0, 0.5);
    assert!((last_pad(&out, Axis::RightX).unwrap() - 0.5).abs() < 0.01, "{out:?}");
    turn(&mut e, &p, 0.0, 0.0, 0.0, 0.5);
    assert!((e.gyro.stick.unwrap().1.0 - 0.5).abs() < 0.01);
    // Recentering counts the current direction as straight.
    run(&mut e, &p, InputEvent::Button(Button::Select, true));
    turn(&mut e, &p, 0.0, 0.0, 0.0, 0.1);
    assert!(e.gyro.stick.unwrap().1.0.abs() < 0.01);
}

#[test]
fn locked_at_the_edge_turning_back_leaves_the_edge_at_once() {
    // Turn 40 degrees right (twice full), then 10 back.
    let back = |lock: bool| {
        let p = position_gyro(lock);
        let mut e = Engine::default();
        turn(&mut e, &p, 0.0, -40.0, 0.0, 1.0);
        assert!((e.gyro.stick.unwrap().1.0 - 1.0).abs() < 0.01);
        turn(&mut e, &p, 0.0, 20.0, 0.0, 0.5);
        e.gyro.stick.unwrap().1.0
    };
    assert!((back(true) - 0.5).abs() < 0.01, "locked: off the edge by the turn back");
    assert!((back(false) - 1.0).abs() < 0.01, "unlocked: still past the edge");
}

#[test]
fn a_toggle_asks_for_rumble_when_set_to() {
    let mut cfg = mouse_gyro(GyroActivation::Toggle(GyroInput::Button(Button::Select)));
    let mut e = Engine::default();
    let p = gyro_profile(cfg.clone());
    run(&mut e, &p, InputEvent::Button(Button::Select, true));
    assert_eq!(e.take_gyro_rumble(), None);
    cfg.toggle_rumble = true;
    let p = gyro_profile(cfg);
    run(&mut e, &p, InputEvent::Button(Button::Select, false));
    run(&mut e, &p, InputEvent::Button(Button::Select, true));
    assert_eq!(e.take_gyro_rumble(), Some(false));
    assert_eq!(e.take_gyro_rumble(), None);
}
