//! HW-5: the DualSense touchpad in each of its modes, through the kernel's own `hid-playstation`
//! (a uhid replica), so the daemon reads real evdev contact slots. Each test draws fingers on the
//! pad with timed reports and checks what comes out of the daemon's virtual devices. It asserts
//! the logic (a tap clicks, two fingers scroll, a stroke far and quick enough swipes, the finger
//! on the pad picks the menu item); how the times and speeds feel is for a person.
//!
//! Needs /dev/uinput and /dev/uhid, and read access to /dev/input, so it is opt-in:
//! `cargo test --test touchpad_modes -- --ignored`, or `scripts/kernel-test-docker.sh touchpad_`.

#![cfg(target_os = "linux")]

mod common;

use std::{fs, path::Path, thread, time::Duration};

use common::{
    Daemon, assert_log_clean, daemon_log,
    ds::{Snapshot, connect_dualsense, overlay_state, start, with_known_dualsense},
    hold_config,
    input::Reader,
    serial, temp_root,
    uhid::{DualSenseState, HidPad},
    wait_for,
};
use evdev::{AbsoluteAxisCode as Abs, KeyCode};

type At = (u16, u16);

/// The pad's size in the replica's units.
const W: u16 = 1920;
const H: u16 = 1080;

/// A DualSense with a daemon on it, and readers on what the daemon makes.
struct Rig {
    root: std::path::PathBuf,
    daemon: Daemon,
    pad: HidPad,
    tick: u32,
    keys: Reader,
    mouse: Reader,
    out: Reader,
}

impl Rig {
    fn new(config: &str) -> Self {
        let root = temp_root();
        let snap = Snapshot::take();
        let daemon = start(&root, &with_known_dualsense(config, 1));
        let keys = snap.open("Padwight Virtual Keyboard");
        let mouse = snap.open("Padwight Virtual Mouse");
        let out = snap.open("Padwight Virtual Pad");
        let pad = connect_dualsense(1);
        wait_for("the touchpad to be paired", || daemon_log(&root).contains("touchpad: using").then_some(()));
        let mut rig = Rig { root, daemon, pad, tick: 0, keys, mouse, out };
        rig.wait(300);
        rig.drain();
        rig
    }

    fn drain(&mut self) {
        self.keys.drain();
        self.mouse.drain();
        self.out.drain();
    }

    /// One report, then the 8 ms a real pad takes between them.
    fn frame(&mut self, f0: Option<At>, f1: Option<At>, click: bool) {
        self.tick += 1;
        let state = DualSenseState { touch: [f0, f1], touch_click: click, timestamp: common::ds::clock(self.tick), ..DualSenseState::default() };
        self.pad.send(&state.report());
        thread::sleep(Duration::from_millis(8));
        // Read as it goes: a reader that is left alone through a long stroke overflows its
        // kernel queue (SYN_DROPPED) and loses the motion.
        self.drain();
    }

    /// Reports for `ms` with the fingers where they are.
    fn hold(&mut self, ms: u64, f0: Option<At>, f1: Option<At>, click: bool) {
        for _ in 0..ms / 8 {
            self.frame(f0, f1, click);
        }
    }

    /// No finger for `ms`.
    fn wait(&mut self, ms: u64) {
        self.hold(ms, None, None, false);
    }

    /// Draws finger 0 from `a` to `b` over `ms`, with finger 1 `second` away from it if given.
    fn stroke(&mut self, a: At, b: At, ms: u64, second: Option<(i32, i32)>) {
        let steps = (ms / 8).max(1);
        for i in 0..=steps {
            let t = i as f32 / steps as f32;
            let p = (f32::from(a.0) + (f32::from(b.0) - f32::from(a.0)) * t, f32::from(a.1) + (f32::from(b.1) - f32::from(a.1)) * t);
            let f0 = (p.0 as u16, p.1 as u16);
            let f1 = second.map(|(dx, dy)| ((i32::from(f0.0) + dx).clamp(0, i32::from(W) - 1) as u16, (i32::from(f0.1) + dy).clamp(0, i32::from(H) - 1) as u16));
            self.frame(Some(f0), f1, false);
        }
    }

    /// Lets `ms` pass with no report, reading what the daemon makes as it goes (a glide sends
    /// hundreds of events a second, more than a reader left alone for a while can hold).
    fn pause(&mut self, ms: u64) {
        for _ in 0..ms / 8 {
            thread::sleep(Duration::from_millis(8));
            self.drain();
        }
    }

    /// Lifts everything, lets the daemon catch up and reads what came out.
    fn settle(&mut self) {
        self.wait(120);
        thread::sleep(Duration::from_millis(80));
        self.drain();
    }

    fn downs(&self, key: KeyCode) -> u32 {
        self.keys.downs.get(&key.0).copied().unwrap_or(0) + self.mouse.downs.get(&key.0).copied().unwrap_or(0)
    }

    fn axis(&self, code: Abs) -> i32 {
        self.out.abs.get(&code.0).copied().unwrap_or(0)
    }

    fn finish(mut self) {
        let _ = self.daemon.0.kill();
        let _ = self.daemon.0.wait();
        assert_log_clean(&self.root);
        let _ = fs::remove_dir_all(&self.root);
    }
}

/// `hold_config` with the active profile's touchpad set to `touchpad` (TOML under
/// `[general.profiles.touchpad]`), and a grid menu "Pad" of A to D in the shared menus.
fn config(touchpad: &str) -> String {
    let text = hold_config().replacen("[general.profiles.gyro]", &format!("[general.profiles.touchpad]\n{touchpad}\n\n[general.profiles.gyro]"), 1);
    let shared = "[shared]\nmacros = []\nmenus = []";
    assert!(text.contains(shared), "the default config has no shared menus");
    let mut text = text.replacen(shared, "[shared]\nmacros = []", 1);
    text.push_str("\n[[shared.menus]]\nname = \"Pad\"\nkind = { grid = { columns = 2 } }\n");
    for k in ["A", "B", "C", "D"] {
        text.push_str(&format!("[[shared.menus.items]]\nlabel = \"{k}\"\naction = {{ keys = [\"KEY_{k}\"] }}\n"));
    }
    text
}

fn quiet(root: &Path) {
    let log = daemon_log(root);
    assert!(!log.contains("touchpad: cannot take"), "{log}");
}

#[test]
#[ignore]
fn touchpad_mouse_mode_glides_taps_and_ignores_the_second_finger() {
    let _serial = serial();
    let mut rig = Rig::new(&config("click = { mouse = \"Right\" }\n[general.profiles.touchpad.motion.mouse]\nspeed = 1500.0\ntap = true"));

    // Glide: a slow drag to the right moves the pointer right, and stops when the finger lifts.
    rig.stroke((400, 500), (1400, 500), 600, None);
    rig.settle();
    assert!(rig.mouse.moved.0 > 200 && rig.mouse.moved.1.abs() < rig.mouse.moved.0 / 4, "a drag right moves the pointer right: {:?}", rig.mouse.moved);
    let after = rig.mouse.moved;
    rig.wait(300);
    rig.drain();
    assert_eq!(rig.mouse.moved, after, "without inertia the pointer stops when the finger lifts");

    // Tap: a quick touch that doesn't move runs the click action (a right click here); one
    // held too long does not.
    rig.hold(80, Some((900, 500)), None, false);
    rig.settle();
    assert_eq!(rig.downs(KeyCode::BTN_RIGHT), 1, "a tap clicks");
    rig.hold(500, Some((900, 500)), None, false);
    rig.settle();
    assert_eq!(rig.downs(KeyCode::BTN_RIGHT), 1, "a long touch is not a tap");

    // A tap that drifts far is a stroke, not a tap.
    rig.stroke((600, 500), (1200, 500), 100, None);
    rig.settle();
    assert_eq!(rig.downs(KeyCode::BTN_RIGHT), 1, "a stroke is not a tap");

    // The pad pressed down runs the click action too.
    rig.hold(300, Some((900, 500)), None, true);
    rig.settle();
    assert_eq!(rig.downs(KeyCode::BTN_RIGHT), 2, "the click presses the right button");

    // With scrolling off, two fingers moving together don't scroll.
    let wheel = rig.mouse.wheel;
    rig.stroke((600, 300), (600, 800), 400, Some((500, 0)));
    rig.settle();
    assert_eq!(rig.mouse.wheel, wheel, "two fingers don't scroll with scrolling off");

    // CR H-5: two fingers down, lift the first, put a finger down again; only the finger that
    // stayed down moves the pointer.
    let (a, b, c) = ((400, 300), (1400, 300), (800, 800));
    rig.hold(100, Some(a), Some(b), false);
    rig.hold(60, None, Some(b), false);
    rig.hold(100, Some(c), Some(b), false);
    rig.drain();
    let before = rig.mouse.moved;
    for i in 0..30u16 {
        rig.frame(Some((c.0 + i * 20, c.1)), Some(b), false);
    }
    // Not `settle`: that would lift both fingers, and the pair would land again together.
    rig.pause(80);
    assert_eq!(rig.mouse.moved, before, "the new finger doesn't move the pointer");
    let end_c = (c.0 + 29 * 20, c.1);
    for i in 0..30u16 {
        rig.frame(Some(end_c), Some((b.0 - i * 20, b.1)), false);
    }
    rig.settle();
    assert!(rig.mouse.moved.0 < before.0 - 100, "the finger that stayed moves it (left): {:?} from {before:?}", rig.mouse.moved);

    quiet(&rig.root);
    rig.finish();
}

#[test]
#[ignore]
fn touchpad_mouse_mode_scrolls_with_two_fingers_and_glides_after_a_flick() {
    let _serial = serial();
    let mut rig = Rig::new(&config("[general.profiles.touchpad.motion.mouse]\nspeed = 1500.0\nscroll = true\ninertia = true"));

    // Two fingers moving down together scroll; the pointer stays.
    let still = rig.mouse.moved;
    rig.stroke((600, 300), (600, 800), 500, Some((400, 0)));
    rig.settle();
    assert_ne!(rig.mouse.wheel.0, 0, "two fingers scroll");
    assert!((rig.mouse.moved.0 - still.0).abs() < 20 && (rig.mouse.moved.1 - still.1).abs() < 20, "the pointer stays while scrolling: {:?}", rig.mouse.moved);

    // A quick flick keeps the pointer gliding after the finger lifts.
    rig.stroke((300, 500), (1500, 500), 160, None);
    rig.wait(40);
    rig.drain();
    let at_lift = rig.mouse.moved;
    rig.pause(200);
    assert!(rig.mouse.moved.0 > at_lift.0 + 20, "the pointer glides on after the finger lifts: {at_lift:?} then {:?}", rig.mouse.moved);
    // ...and slows to a stop.
    rig.pause(3000);
    let stopped = rig.mouse.moved;
    rig.pause(500);
    assert_eq!(rig.mouse.moved, stopped, "the glide dies away");

    quiet(&rig.root);
    rig.finish();
}

#[test]
#[ignore]
fn touchpad_stick_mode_pushes_the_virtual_stick() {
    let _serial = serial();
    let mut rig = Rig::new(&config("click = \"disabled\"\n[general.profiles.touchpad.motion.joystick]\nstick = \"Left\"\nrelative = false\nradius = 0.3\ndeadzone = 0.05"));

    // The middle of the pad is centre, the edges are full deflection, and lifting recentres it.
    rig.hold(150, Some((W / 2, H / 2)), None, false);
    rig.drain();
    assert!(rig.axis(Abs::ABS_X).abs() < 3000 && rig.axis(Abs::ABS_Y).abs() < 3000, "centre is centre: {:?}", rig.out.abs);
    rig.hold(200, Some((W - 20, H / 2)), None, false);
    rig.drain();
    assert!(rig.axis(Abs::ABS_X) > 28_000, "the right edge pushes the stick right: {:?}", rig.out.abs);
    rig.hold(200, Some((20, H / 2)), None, false);
    rig.drain();
    assert!(rig.axis(Abs::ABS_X) < -28_000, "the left edge pushes it left: {:?}", rig.out.abs);
    rig.hold(200, Some((W / 2, 20)), None, false);
    rig.drain();
    assert!(rig.axis(Abs::ABS_Y) < -20_000, "the top pushes it up: {:?}", rig.out.abs);
    rig.settle();
    assert!(rig.axis(Abs::ABS_X).abs() < 1500 && rig.axis(Abs::ABS_Y).abs() < 1500, "lifting the finger centres the stick: {:?}", rig.out.abs);
    assert_eq!(rig.mouse.moved, (0, 0), "the pointer doesn't move in stick mode");

    quiet(&rig.root);
    rig.finish();
}

#[test]
#[ignore]
fn touchpad_relative_stick_mode_centres_where_the_finger_lands() {
    let _serial = serial();
    let mut rig = Rig::new(&config("click = \"disabled\"\n[general.profiles.touchpad.motion.joystick]\nstick = \"Right\"\nrelative = true\nradius = 0.25\ndeadzone = 0.05"));

    // Landing at the far right is no deflection; moving a radius (0.25 of the width) is full.
    rig.hold(150, Some((1700, 540)), None, false);
    rig.drain();
    assert!(rig.axis(Abs::ABS_RX).abs() < 3000, "landing anywhere is centre: {:?}", rig.out.abs);
    rig.hold(200, Some((1700 - 480, 540)), None, false);
    rig.drain();
    assert!(rig.axis(Abs::ABS_RX) < -28_000, "a radius to the left is full left: {:?}", rig.out.abs);
    rig.settle();
    assert!(rig.axis(Abs::ABS_RX).abs() < 1500, "lifting recentres it: {:?}", rig.out.abs);

    quiet(&rig.root);
    rig.finish();
}

#[test]
#[ignore]
fn touchpad_swipe_mode_runs_the_action_for_a_quick_long_stroke() {
    let _serial = serial();
    let swipes = "click = \"disabled\"\n[general.profiles.touchpad.motion.swipe]\nup = { keys = [\"KEY_U\"] }\ndown = { keys = [\"KEY_N\"] }\nleft = { keys = [\"KEY_L\"] }\nright = { keys = [\"KEY_D\"] }";
    let mut rig = Rig::new(&config(swipes));

    for (name, from, to, key) in [
        ("right", (500, 540), (1400, 540), KeyCode::KEY_D),
        ("left", (1400, 540), (500, 540), KeyCode::KEY_L),
        ("up", (960, 950), (960, 150), KeyCode::KEY_U),
        ("down", (960, 150), (960, 950), KeyCode::KEY_N),
    ] {
        rig.stroke(from, to, 160, None);
        rig.settle();
        assert_eq!(rig.downs(key), 1, "swipe {name} presses its key");
    }
    let total: u32 = [KeyCode::KEY_D, KeyCode::KEY_L, KeyCode::KEY_U, KeyCode::KEY_N].iter().map(|k| rig.downs(*k)).sum();

    // Too slow: the same stroke over a second is a drag, not a swipe.
    rig.stroke((500, 540), (1400, 540), 1000, None);
    rig.settle();
    // Too short: a quick stroke of a tenth of the width.
    rig.stroke((900, 540), (1090, 540), 100, None);
    rig.settle();
    let after: u32 = [KeyCode::KEY_D, KeyCode::KEY_L, KeyCode::KEY_U, KeyCode::KEY_N].iter().map(|k| rig.downs(*k)).sum();
    assert_eq!(after, total, "a slow stroke and a short one don't swipe");
    assert_eq!(rig.mouse.moved, (0, 0), "the pointer doesn't move in swipe mode");

    quiet(&rig.root);
    rig.finish();
}

#[test]
#[ignore]
fn touchpad_menu_mode_picks_the_item_under_the_finger_and_the_click_presses_it() {
    let _serial = serial();
    let mut rig = Rig::new(&config("[general.profiles.touchpad.motion.menu]\nmenu = \"Pad\""));

    // A 2 by 2 grid laid over the pad: A top left, B top right, C bottom left, D bottom right.
    for (name, at, key, item) in [("top left", (480, 270), KeyCode::KEY_A, "A"), ("bottom right", (1440, 810), KeyCode::KEY_D, "D"), ("top right", (1440, 270), KeyCode::KEY_B, "B")] {
        rig.hold(150, Some(at), None, false);
        let shown = overlay_state(&rig.root);
        assert_eq!(shown["menu"]["selected"], item, "{name}: the item under the finger is selected: {shown}");
        rig.drain();
        assert!(!rig.keys.pressed.contains(&key.0), "{name}: touching alone doesn't press it");
        rig.hold(120, Some(at), None, true);
        rig.drain();
        assert!(rig.keys.pressed.contains(&key.0), "{name}: the click holds {item}");
        rig.hold(120, Some(at), None, false);
        rig.drain();
        assert!(!rig.keys.pressed.contains(&key.0), "{name}: letting the click go releases it");
        rig.settle();
    }
    assert!(overlay_state(&rig.root)["menu"].is_null(), "the menu goes when the finger lifts");

    quiet(&rig.root);
    rig.finish();
}

#[test]
#[ignore]
fn touchpad_menu_mode_can_choose_by_lifting() {
    let _serial = serial();
    let mut rig = Rig::new(&config("[general.profiles.touchpad.motion.menu]\nmenu = \"Pad\"\nchoose = \"lift\""));

    rig.hold(150, Some((480, 810)), None, false);
    rig.drain();
    assert_eq!(rig.downs(KeyCode::KEY_C), 0, "nothing runs while the finger is down");
    rig.settle();
    assert_eq!(rig.downs(KeyCode::KEY_C), 1, "lifting runs the item under the finger (C, bottom left)");
    assert!(rig.keys.pressed.is_empty());

    quiet(&rig.root);
    rig.finish();
}

#[test]
#[ignore]
fn touchpad_split_pad_gives_each_half_its_own_mode_and_click() {
    let _serial = serial();
    // Left half: a mouse with a tap, its click is L. Right half: swipes (right is D), its click is R.
    let split = "click = { keys = [\"KEY_L\"] }\n[general.profiles.touchpad.motion.mouse]\nspeed = 1500.0\ntap = true\n\
        [general.profiles.touchpad.right]\nclick = { keys = [\"KEY_R\"] }\n\
        [general.profiles.touchpad.right.motion.swipe]\nright = { keys = [\"KEY_D\"] }";
    let mut rig = Rig::new(&config(split));

    // On the left half a drag moves the pointer, a tap and a click run the left click action.
    rig.stroke((200, 500), (800, 500), 400, None);
    rig.settle();
    assert!(rig.mouse.moved.0 > 100, "the left half is a mouse: {:?}", rig.mouse.moved);
    rig.hold(80, Some((500, 500)), None, false);
    rig.settle();
    assert_eq!(rig.downs(KeyCode::KEY_L), 1, "a tap on the left half runs its click");
    rig.hold(300, Some((500, 500)), None, true);
    rig.settle();
    assert_eq!(rig.downs(KeyCode::KEY_L), 2, "clicking on the left half runs its click");
    assert_eq!(rig.downs(KeyCode::KEY_R), 0);

    // On the right half the click is R, a swipe right is D, and the pointer stays.
    let moved = rig.mouse.moved;
    rig.hold(120, Some((1500, 500)), None, true);
    rig.settle();
    assert_eq!(rig.downs(KeyCode::KEY_R), 1, "clicking on the right half runs its click");
    rig.stroke((1100, 500), (1800, 500), 140, None);
    rig.settle();
    assert_eq!(rig.downs(KeyCode::KEY_D), 1, "a swipe on the right half runs its action");
    assert_eq!(rig.mouse.moved, moved, "the right half doesn't move the pointer");

    quiet(&rig.root);
    rig.finish();
}
