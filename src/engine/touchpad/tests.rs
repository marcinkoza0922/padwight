use std::time::Duration;

use super::*;
use crate::{
    config::{CurvePreset, Menu, MenuItem, MenuKind, MouseButton, OverlayStyle, Stick, TouchRegion},
    input::Axis,
};

fn profile(touchpad: TouchpadConfig) -> Profile {
    Profile { touchpad, ..Profile::passthrough("test") }
}

fn pad(motion: TouchpadMotion) -> TouchpadConfig {
    TouchpadConfig { motion, click: ButtonAction::Mouse(MouseButton::Left), right: None }
}

fn key(k: &str) -> ButtonAction {
    ButtonAction::Keys(vec![k.into()])
}

/// Drives an engine with touch events, each a few milliseconds after the last.
struct Finger<'a> {
    e: Engine,
    p: &'a Profile,
    now: Instant,
}

impl<'a> Finger<'a> {
    fn on(p: &'a Profile) -> Self {
        Finger { e: Engine::default(), p, now: Instant::now() }
    }

    fn send(&mut self, ev: TouchpadEvent) -> Vec<OutEvent> {
        self.now += Duration::from_millis(8);
        let mut out = Vec::new();
        self.e.touchpad(self.p, ev, self.now, &mut out);
        out
    }

    fn touch(&mut self, finger: u8, x: f32, y: f32) -> Vec<OutEvent> {
        self.send(TouchpadEvent::Touch { finger, x, y, height: 0.5 })
    }

    fn lift(&mut self, finger: u8) -> Vec<OutEvent> {
        self.send(TouchpadEvent::Lift { finger })
    }

    fn click(&mut self, pressed: bool) -> Vec<OutEvent> {
        self.send(TouchpadEvent::Click(pressed))
    }

    /// Moves `finger` from x0 to x1 (at height y) in `steps` reports.
    fn stroke(&mut self, finger: u8, (x0, y0): (f32, f32), (x1, y1): (f32, f32), steps: usize) -> Vec<OutEvent> {
        let mut out = self.touch(finger, x0, y0);
        for i in 1..=steps {
            let t = i as f32 / steps as f32;
            out.extend(self.touch(finger, x0 + (x1 - x0) * t, y0 + (y1 - y0) * t));
        }
        out
    }
}

fn moved(out: &[OutEvent]) -> (i32, i32) {
    out.iter().fold((0, 0), |(x, y), ev| if let OutEvent::MouseMove(dx, dy) = ev { (x + dx, y + dy) } else { (x, y) })
}

fn scrolled(out: &[OutEvent]) -> (i32, i32) {
    out.iter().fold((0, 0), |(h, v), ev| if let OutEvent::Wheel { vertical, horizontal } = ev { (h + horizontal, v + vertical) } else { (h, v) })
}

#[test]
fn movement_becomes_pointer_motion_scaled_by_speed_and_landing_is_silent() {
    let p = profile(pad(TouchpadMotion::mouse(1000.0)));
    let mut f = Finger::on(&p);
    assert!(f.touch(0, 0.5, 0.5).is_empty(), "landing is not movement");
    assert_eq!(f.touch(0, 0.625, 0.25), [OutEvent::MouseMove(125, -125)], "down the pad is measured in widths too");
    // Fractions carry over until they make a whole pixel.
    assert!(f.touch(0, 0.6254, 0.25).is_empty());
    assert_eq!(f.touch(0, 0.6262, 0.25), [OutEvent::MouseMove(1, 0)]);
    f.lift(0);
    assert!(f.touch(0, 0.1, 0.1).is_empty(), "landing again elsewhere doesn't jump");
}

#[test]
fn the_curve_slows_slow_fingers_and_speeds_up_fast_ones() {
    // A finger's pad widths/second feel like 40 times as many degrees/second of gyro.
    let curve = CurvePreset::Aggressive.speed_curve(0.025).unwrap();
    let p = profile(pad(TouchpadMotion::Mouse { speed: 1000.0, curve, tap: false, scroll: false, inertia: false }));
    let travel = |step: f32| {
        let mut f = Finger::on(&p);
        moved(&f.stroke(0, (0.1, 0.5), (0.1 + step * 20.0, 0.5), 20)).0
    };
    // 0.0008 pad widths every 8 ms is 0.1 widths/s, slow: half speed, 8 px instead of 16.
    assert!((7..=9).contains(&travel(0.0008)), "{}", travel(0.0008));
    // 0.02 every 8 ms is 2.5 widths/s, fast: double, 800 px instead of 400.
    assert!((770..=800).contains(&travel(0.02)), "{}", travel(0.02));
}

#[test]
fn a_second_finger_doesnt_move_the_pointer_unless_it_scrolls() {
    let p = profile(pad(TouchpadMotion::mouse(1000.0)));
    let mut f = Finger::on(&p);
    f.touch(0, 0.25, 0.25);
    f.touch(1, 0.6, 0.6);
    assert!(f.touch(1, 0.7, 0.6).is_empty());
    assert_eq!(moved(&f.touch(0, 0.375, 0.25)), (125, 0));

    let p = profile(pad(TouchpadMotion::Mouse { speed: 1000.0, curve: SpeedCurve::default(), tap: false, scroll: true, inertia: false }));
    let mut f = Finger::on(&p);
    f.touch(0, 0.4, 0.5);
    f.touch(1, 0.6, 0.5);
    // Both move up an eighth of the pad's width (a quarter of its height): 1.5 notches up.
    let mut out = f.touch(0, 0.4, 0.25);
    out.extend(f.touch(1, 0.6, 0.25));
    assert_eq!(moved(&out), (0, 0));
    assert_eq!(scrolled(&out), (0, 180));
}

#[test]
fn a_quick_tap_clicks_when_tapping_is_on() {
    let tapping = profile(pad(TouchpadMotion::Mouse { speed: 1000.0, curve: SpeedCurve::default(), tap: true, scroll: false, inertia: false }));
    let mut f = Finger::on(&tapping);
    f.touch(0, 0.5, 0.5);
    let out = f.lift(0);
    assert_eq!(out, [OutEvent::MouseButton(MouseButton::Left, true), OutEvent::MouseButton(MouseButton::Left, false)]);
    // A stroke isn't a tap, nor is a long touch.
    let stroke = f.stroke(0, (0.1, 0.5), (0.5, 0.5), 3);
    assert!(!stroke.iter().any(|e| matches!(e, OutEvent::MouseButton(..))));
    assert!(f.lift(0).is_empty());
    f.touch(0, 0.5, 0.5);
    f.now += Duration::from_millis(300);
    assert!(f.lift(0).is_empty());

    let plain = profile(pad(TouchpadMotion::mouse(1000.0)));
    let mut f = Finger::on(&plain);
    f.touch(0, 0.5, 0.5);
    assert!(f.lift(0).is_empty());
}

#[test]
fn a_flick_glides_on_and_slows_down_until_a_touch_stops_it() {
    let p = profile(pad(TouchpadMotion::Mouse { speed: 1000.0, curve: SpeedCurve::default(), tap: false, scroll: false, inertia: true }));
    let mut f = Finger::on(&p);
    f.stroke(0, (0.1, 0.5), (0.5, 0.5), 8);
    f.lift(0);
    assert!(f.e.needs_tick(&p));
    let tick = |e: &mut Engine| {
        let mut out = Vec::new();
        e.tick(&p, 0.05, &mut out);
        moved(&out).0
    };
    let (first, second) = (tick(&mut f.e), tick(&mut f.e));
    assert!(first > second && second > 0, "{first} then {second}");
    f.touch(0, 0.5, 0.5);
    assert!(!f.e.needs_tick(&p), "a finger landing stops the glide");
}

#[test]
fn a_relative_joystick_centers_where_the_finger_lands_and_on_lift() {
    let p = profile(pad(TouchpadMotion::Joystick { stick: Stick::Left, relative: true, radius: 0.25, deadzone: 0.0 }));
    let mut f = Finger::on(&p);
    assert!(f.touch(0, 0.5, 0.2).contains(&OutEvent::PadAxis(Axis::LeftX, 0.0)));
    // An eighth of the pad's width right is half of the radius.
    let out = f.touch(0, 0.625, 0.2);
    assert!(out.contains(&OutEvent::PadAxis(Axis::LeftX, 0.5)), "{out:?}");
    // Far past the radius is full push.
    assert!(f.touch(0, 1.0, 0.2).contains(&OutEvent::PadAxis(Axis::LeftX, 1.0)));
    assert!(f.lift(0).contains(&OutEvent::PadAxis(Axis::LeftX, 0.0)));
}

#[test]
fn an_absolute_joystick_centers_on_the_middle_and_ignores_its_deadzone() {
    let p = profile(pad(TouchpadMotion::Joystick { stick: Stick::Right, relative: false, radius: 0.2, deadzone: 0.2 }));
    let mut f = Finger::on(&p);
    assert!(f.touch(0, 0.55, 0.5).contains(&OutEvent::PadAxis(Axis::RightX, 0.0)), "inside the deadzone");
    let out = f.touch(0, 0.5, 0.0);
    assert!(out.contains(&OutEvent::PadAxis(Axis::RightY, -1.0)), "{out:?}");
}

#[test]
fn a_mouse_region_puts_the_pointer_under_the_finger_on_its_half() {
    let region = TouchpadMotion::Region { region: crate::config::MouseRegion::FULL };
    let p = profile(TouchpadConfig { right: Some(TouchRegion { motion: region, click: ButtonAction::Disabled }), ..pad(TouchpadMotion::Off) });
    let mut f = Finger::on(&p);
    assert!(f.touch(0, 0.2, 0.5).iter().all(|ev| !matches!(ev, OutEvent::MouseTo(..))), "the left half is off");
    assert_eq!(f.touch(1, 0.75, 0.25), [OutEvent::MouseTo(0.5, 0.25)]);
    assert_eq!(f.touch(1, 1.0, 1.0), [OutEvent::MouseTo(1.0, 1.0)]);
    assert!(f.lift(1).is_empty(), "lifting leaves the pointer where it is");
}

fn grid_menu() -> Menu {
    let items = ["A", "B", "C", "D"].map(|k| MenuItem::new(k, key(&format!("KEY_{k}")))).to_vec();
    Menu { name: "Grid".into(), kind: MenuKind::Grid { columns: 2 }, items, cancel: None, no_cancel: false, keep_open: false, start: 0, style: OverlayStyle::default() }
}

fn menu_pad(choose: TouchChoose) -> TouchpadConfig {
    pad(TouchpadMotion::Menu { menu: "Grid".into(), choose, overlay: true })
}

#[test]
fn a_menu_shows_while_touched_and_the_click_holds_the_item_under_the_finger() {
    let p = profile(menu_pad(TouchChoose::Click));
    let mut f = Finger::on(&p);
    f.e.set_menus(&[grid_menu()]);
    assert!(f.e.touch_menu().is_none());
    f.touch(0, 0.9, 0.1);
    assert_eq!(f.e.touch_menu().map(|m| (m.menu.as_str(), m.item)), Some(("Grid", Some(1))));
    assert!(f.e.take_touch_menu_changed());
    f.touch(0, 0.1, 0.9);
    assert_eq!(f.e.touch_menu().and_then(|m| m.item), Some(2));
    let c = crate::keycode::KeyCode::KEY_C;
    assert_eq!(f.click(true), [OutEvent::Key(c, true)]);
    // Sliding while clicked keeps the item that was pressed until the click lets go.
    f.touch(0, 0.9, 0.9);
    assert_eq!(f.click(false), [OutEvent::Key(c, false)]);
    assert!(f.lift(0).is_empty(), "the click chose; lifting doesn't");
    assert!(f.e.touch_menu().is_none());
}

#[test]
fn a_lift_menu_chooses_on_lift_unless_the_click_already_did() {
    let p = profile(menu_pad(TouchChoose::Lift));
    let mut f = Finger::on(&p);
    f.e.set_menus(&[grid_menu()]);
    f.touch(0, 0.1, 0.1);
    let a = crate::keycode::KeyCode::KEY_A;
    assert_eq!(f.lift(0), [OutEvent::Key(a, true), OutEvent::Key(a, false)]);
    f.touch(0, 0.1, 0.1);
    f.click(true);
    f.click(false);
    assert!(f.lift(0).is_empty());
}

#[test]
fn a_menu_without_its_overlay_still_picks_but_shows_nothing() {
    let p = profile(pad(TouchpadMotion::Menu { menu: "Grid".into(), choose: TouchChoose::Click, overlay: false }));
    let mut f = Finger::on(&p);
    f.e.set_menus(&[grid_menu()]);
    f.touch(0, 0.9, 0.9);
    assert!(f.e.touch_menu().is_none());
    assert_eq!(f.click(true), [OutEvent::Key(crate::keycode::KeyCode::KEY_D, true)]);
}

#[test]
fn a_quick_stroke_swipes_in_its_direction() {
    let swipe = TouchpadMotion::Swipe { up: key("KEY_W"), down: key("KEY_S"), left: ButtonAction::Disabled, right: key("KEY_D"), distance: 0.2 };
    let p = profile(pad(swipe));
    let mut f = Finger::on(&p);
    f.stroke(0, (0.2, 0.5), (0.6, 0.55), 4);
    let d = crate::keycode::KeyCode::KEY_D;
    assert_eq!(f.lift(0), [OutEvent::Key(d, true), OutEvent::Key(d, false)]);
    // Down the pad: 0.5 of its height is a quarter of its width.
    f.stroke(0, (0.5, 0.2), (0.5, 0.7), 4);
    assert_eq!(f.lift(0), [OutEvent::Key(crate::keycode::KeyCode::KEY_S, true), OutEvent::Key(crate::keycode::KeyCode::KEY_S, false)]);
    // Too short, or too slow.
    f.stroke(0, (0.5, 0.5), (0.6, 0.5), 4);
    assert!(f.lift(0).is_empty());
    f.stroke(0, (0.2, 0.5), (0.6, 0.5), 80);
    assert!(f.lift(0).is_empty());
}

#[test]
fn a_split_pad_runs_each_half_on_its_own() {
    let cfg = TouchpadConfig {
        motion: TouchpadMotion::Joystick { stick: Stick::Left, relative: false, radius: 0.2, deadzone: 0.0 },
        click: key("KEY_L"),
        right: Some(TouchRegion { motion: TouchpadMotion::mouse(1000.0), click: key("KEY_R") }),
    };
    let p = profile(cfg);
    let mut f = Finger::on(&p);
    // The left half's middle is a quarter of the way across the pad.
    assert!(f.touch(0, 0.25, 0.5).contains(&OutEvent::PadAxis(Axis::LeftX, 0.0)));
    assert!(f.touch(0, 0.5, 0.5).contains(&OutEvent::PadAxis(Axis::LeftX, 1.0)), "a finger stays on the half it landed on");
    f.touch(1, 0.75, 0.5);
    assert_eq!(moved(&f.touch(1, 0.8, 0.5)), (50, 0), "the right half moves the pointer at the same time");
    f.lift(0);
    assert_eq!(f.click(true), [OutEvent::Key(crate::keycode::KeyCode::KEY_R, true)], "the click goes by the first finger down");
    f.click(false);
}

#[test]
fn clicks_run_the_action_on_press_and_release_it_and_letting_go_of_everything_resets() {
    let p = profile(TouchpadConfig { click: ButtonAction::Mouse(MouseButton::Right), ..TouchpadConfig::default() });
    let mut f = Finger::on(&p);
    assert_eq!(f.click(true), [OutEvent::MouseButton(MouseButton::Right, true)]);
    assert_eq!(f.click(false), [OutEvent::MouseButton(MouseButton::Right, false)]);
    let p = profile(TouchpadConfig { motion: TouchpadMotion::Off, click: ButtonAction::Disabled, right: None });
    let mut f = Finger::on(&p);
    assert!(f.click(true).is_empty());
    assert!(f.touch(0, 0.5, 0.5).is_empty() && f.touch(0, 0.9, 0.9).is_empty());

    let p = profile(pad(TouchpadMotion::Joystick { stick: Stick::Left, relative: false, radius: 0.2, deadzone: 0.0 }));
    let mut f = Finger::on(&p);
    f.touch(0, 1.0, 0.5);
    let mut out = Vec::new();
    f.e.release_all(false, &mut out);
    assert!(out.contains(&OutEvent::PadAxis(Axis::LeftX, 0.0)));
    assert!(f.touch(0, 0.75, 0.5).contains(&OutEvent::PadAxis(Axis::LeftX, 0.5)), "the finger lands afresh");
}

#[test]
fn the_finger_first_down_keeps_the_pointer_when_another_lands_in_a_lower_slot() {
    let p = profile(pad(TouchpadMotion::Mouse { speed: 1000.0, curve: SpeedCurve::default(), tap: false, scroll: false, inertia: false }));
    let mut f = Finger::on(&p);
    f.touch(0, 0.2, 0.5);
    f.touch(1, 0.6, 0.5);
    f.lift(0);
    // Slot 1 is the only finger, so it moves the pointer; a new finger takes the free slot 0.
    assert_eq!(moved(&f.touch(1, 0.725, 0.5)), (125, 0));
    f.touch(0, 0.3, 0.3);
    assert_eq!(moved(&f.touch(0, 0.425, 0.3)), (0, 0), "the second finger down doesn't move the pointer");
    assert_eq!(moved(&f.touch(1, 0.85, 0.5)), (125, 0), "the first one still does");
}

#[test]
fn a_joystick_side_switched_to_another_mode_lets_its_stick_go() {
    let stick = profile(pad(TouchpadMotion::Joystick { stick: Stick::Left, relative: true, radius: 0.25, deadzone: 0.0 }));
    let mouse = profile(pad(TouchpadMotion::mouse(1000.0)));
    let mut f = Finger::on(&stick);
    f.touch(0, 0.5, 0.2);
    assert!(f.touch(0, 0.625, 0.2).contains(&OutEvent::PadAxis(Axis::LeftX, 0.5)));
    // A layer turns the pad into a mouse with the finger still down.
    f.p = &mouse;
    let out = f.touch(0, 0.63, 0.2);
    assert!(out.contains(&OutEvent::PadAxis(Axis::LeftX, 0.0)), "{out:?}");
}
