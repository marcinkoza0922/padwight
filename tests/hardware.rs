//! The hardware checks of the QA plan that a replica controller can answer (HW-1, 3, 8, 10, 11),
//! run against the real daemon with pads made through uhid. A DualSense replica goes through the
//! kernel's `hid-playstation`, so the daemon reads the nodes a real one has; the plain pads are
//! generic HID gamepads.
//!
//! What a replica can't tell: whether a real pad's buttons and axes arrive as the replica's do
//! (the driver's mapping is checked here, the controller's own firmware is not), Bluetooth, and
//! how anything feels. Those stay with a person.
//!
//! Needs /dev/uinput and /dev/uhid, and read access to /dev/input, so it is opt-in:
//! `cargo test --test hardware -- --ignored`, or `scripts/kernel-test-docker.sh hw_`.

#![cfg(target_os = "linux")]

mod common;

use std::{
    fs,
    os::unix::fs::PermissionsExt,
    path::Path,
    process::{Child, Command, Stdio},
    thread,
    time::{Duration, Instant},
};

use common::{
    Daemon, TIMEOUT, assert_log_clean, daemon_log,
    ds::{DUALSENSE, Snapshot, clock, connect_dualsense, drive, overlay_state, start, status, with_known_dualsense},
    hold_config,
    input::{Reader, assert_no_other_daemon, find_node, grabbed},
    serial, spawn_daemon, temp_root,
    uhid::{BUTTON_EAST, BUTTON_SOUTH, DualSenseState, HidPad},
    wait_for,
};
use evdev::{AbsoluteAxisCode as Abs, KeyCode};

fn stop(mut daemon: Daemon, root: &Path) {
    let _ = daemon.0.kill();
    let _ = daemon.0.wait();
    assert_log_clean(root);
    let _ = fs::remove_dir_all(root);
}

const PAD: &str = "Padwight Virtual Pad";
const KEYBOARD: &str = "Padwight Virtual Keyboard";
const MOUSE: &str = "Padwight Virtual Mouse";

// ---------------------------------------------------------------------------------------------
// HW-1: every button, stick and trigger gives the right output, and the sticks and triggers rest
// at center.
// ---------------------------------------------------------------------------------------------

/// A DualSense reporting `state` until `check` holds on the virtual pad, then at rest until the
/// pad shows nothing pressed and every axis back at rest.
fn probe(pad: &mut HidPad, out: &mut Reader, what: &str, state: &DualSenseState, mut check: impl FnMut(&Reader) -> bool) {
    drive(pad, what, |_| state.clone(), || {
        out.drain();
        check(out)
    });
    drive(pad, &format!("{what}: back to rest"), |_| DualSenseState::default(), || {
        out.drain();
        let axis = |code: Abs| out.abs.get(&code.0).copied().unwrap_or(0);
        out.pressed.is_empty()
            && axis(Abs::ABS_X).abs() < 1500
            && axis(Abs::ABS_Y).abs() < 1500
            && axis(Abs::ABS_RX).abs() < 1500
            && axis(Abs::ABS_RY).abs() < 1500
            && axis(Abs::ABS_Z) == 0
            && axis(Abs::ABS_RZ) == 0
            && axis(Abs::ABS_HAT0X) == 0
            && axis(Abs::ABS_HAT0Y) == 0
    });
}

fn axis(out: &Reader, code: Abs) -> i32 {
    out.abs.get(&code.0).copied().unwrap_or(0)
}

#[test]
#[ignore]
fn hw_1_dualsense_passthrough_gives_every_button_stick_and_trigger() {
    let _serial = serial();
    let root = temp_root();
    let snap = Snapshot::take();
    let daemon = start(&root, &with_known_dualsense(&common::default_config(), 1));
    let mut out = snap.open(PAD);
    let mut pad = connect_dualsense(1);

    // Face buttons are positional: cross is South. The virtual pad is an Xbox 360 pad, whose
    // driver names the north and west buttons the other way round (see `north_code`).
    type Set = fn(&mut DualSenseState);
    let buttons: [(&str, Set, KeyCode); 10] = [
        ("cross", |s| s.cross = true, KeyCode::BTN_SOUTH),
        ("circle", |s| s.circle = true, KeyCode::BTN_EAST),
        ("triangle", |s| s.triangle = true, KeyCode::BTN_WEST),
        ("square", |s| s.square = true, KeyCode::BTN_NORTH),
        ("L1", |s| s.shoulders |= 0x01, KeyCode::BTN_TL),
        ("R1", |s| s.shoulders |= 0x02, KeyCode::BTN_TR),
        ("create", |s| s.shoulders |= 0x10, KeyCode::BTN_SELECT),
        ("options", |s| s.shoulders |= 0x20, KeyCode::BTN_START),
        ("L3", |s| s.shoulders |= 0x40, KeyCode::BTN_THUMBL),
        ("R3", |s| s.shoulders |= 0x80, KeyCode::BTN_THUMBR),
    ];
    for (name, set, expect) in buttons {
        let mut state = DualSenseState::default();
        set(&mut state);
        probe(&mut pad, &mut out, name, &state, |o| o.pressed.contains(&expect.0));
    }

    // The d-pad: the hat counts clockwise from up.
    for (name, hat, x, y) in [("up", 0u8, 0, -1), ("right", 2, 1, 0), ("down", 4, 0, 1), ("left", 6, -1, 0)] {
        let state = DualSenseState { hat, ..DualSenseState::default() };
        probe(&mut pad, &mut out, &format!("d-pad {name}"), &state, |o| axis(o, Abs::ABS_HAT0X) == x && axis(o, Abs::ABS_HAT0Y) == y);
    }

    // Sticks: full right and full down read near the ends of the range; centre reads zero.
    for (name, sticks, code, high) in [
        ("left stick right", [255, 128, 128, 128], Abs::ABS_X, true),
        ("left stick left", [0, 128, 128, 128], Abs::ABS_X, false),
        ("left stick down", [128, 255, 128, 128], Abs::ABS_Y, true),
        ("left stick up", [128, 0, 128, 128], Abs::ABS_Y, false),
        ("right stick right", [128, 128, 255, 128], Abs::ABS_RX, true),
        ("right stick left", [128, 128, 0, 128], Abs::ABS_RX, false),
        ("right stick down", [128, 128, 128, 255], Abs::ABS_RY, true),
        ("right stick up", [128, 128, 128, 0], Abs::ABS_RY, false),
    ] {
        let state = DualSenseState { sticks, ..DualSenseState::default() };
        probe(&mut pad, &mut out, name, &state, |o| if high { axis(o, code) > 30_000 } else { axis(o, code) < -30_000 });
    }
    // Half way is about half.
    let state = DualSenseState { sticks: [128 + 64, 128, 128, 128], ..DualSenseState::default() };
    probe(&mut pad, &mut out, "left stick half right", &state, |o| (8_000..24_000).contains(&axis(o, Abs::ABS_X)));

    // Triggers run 0..255 and rest at 0.
    let state = DualSenseState { triggers: [255, 0], ..DualSenseState::default() };
    probe(&mut pad, &mut out, "left trigger", &state, |o| axis(o, Abs::ABS_Z) >= 250 && axis(o, Abs::ABS_RZ) == 0);
    let state = DualSenseState { triggers: [0, 255], ..DualSenseState::default() };
    probe(&mut pad, &mut out, "right trigger", &state, |o| axis(o, Abs::ABS_RZ) >= 250 && axis(o, Abs::ABS_Z) == 0);
    let state = DualSenseState { triggers: [128, 0], ..DualSenseState::default() };
    probe(&mut pad, &mut out, "left trigger half", &state, |o| (100..160).contains(&axis(o, Abs::ABS_Z)));

    drop(pad);
    stop(daemon, &root);
}

#[test]
#[ignore]
fn hw_1_dualsense_remapped_profile_changes_only_what_it_remaps() {
    let _serial = serial();
    let root = temp_root();
    // Cross holds A on the keyboard and circle the left mouse button (see `hold_config`).
    let snap = Snapshot::take();
    let daemon = start(&root, &with_known_dualsense(&hold_config(), 1));
    let mut keys = snap.open(KEYBOARD);
    let mut mouse = snap.open(MOUSE);
    let mut out = snap.open(PAD);
    let mut pad = connect_dualsense(1);

    let cross = DualSenseState { cross: true, ..DualSenseState::default() };
    drive(&mut pad, "cross to hold KEY_A", |_| cross.clone(), || {
        keys.drain();
        keys.pressed.contains(&KeyCode::KEY_A.0)
    });
    out.drain();
    assert!(!out.pressed.contains(&KeyCode::BTN_SOUTH.0), "a remapped button doesn't also reach the virtual pad");
    let circle = DualSenseState { circle: true, ..DualSenseState::default() };
    drive(&mut pad, "circle to hold the left mouse button", |_| circle.clone(), || {
        mouse.drain();
        mouse.pressed.contains(&KeyCode::BTN_LEFT.0)
    });
    // Rest: both let go. A button that wasn't remapped still passes through, and a stick rests.
    drive(&mut pad, "rest", |_| DualSenseState::default(), || {
        keys.drain();
        mouse.drain();
        keys.pressed.is_empty() && mouse.pressed.is_empty()
    });
    let triangle = DualSenseState { triangle: true, sticks: [255, 128, 128, 128], ..DualSenseState::default() };
    drive(&mut pad, "triangle and the left stick to pass through", |_| triangle.clone(), || {
        out.drain();
        out.pressed.contains(&KeyCode::BTN_WEST.0) && axis(&out, Abs::ABS_X) > 30_000
    });
    drive(&mut pad, "rest", |_| DualSenseState::default(), || {
        out.drain();
        out.pressed.is_empty() && axis(&out, Abs::ABS_X).abs() < 1500
    });
    drop(pad);
    stop(daemon, &root);
}

// ---------------------------------------------------------------------------------------------
// HW-3: two controllers at once, each with its own setup. Unplug the first.
// ---------------------------------------------------------------------------------------------

const ID_A: &str = "02:70:61:64:00:a1";
const ID_B: &str = "02:70:61:64:00:a2";

/// `hold_config`, with each of two controllers (by id) pointed at its own profile of General:
/// the first at Gamepad, where South holds KEY_A, the second at Desktop, where South holds the
/// left mouse button.
fn two_pad_config() -> String {
    let mut text = hold_config();
    for (id, profile) in [(ID_A, "Gamepad"), (ID_B, "Desktop")] {
        text.push_str(&format!("\n[[controllers]]\nid = \"{id}\"\n[controllers.profiles]\n\"\" = \"{profile}\"\n"));
    }
    text
}

/// Holds South on `pad` until `done`.
fn hold_south(pad: &mut HidPad, what: &str, mut done: impl FnMut() -> bool) {
    wait_for(what, || {
        pad.report(BUTTON_SOUTH, 0, 0);
        thread::sleep(Duration::from_millis(20));
        done().then_some(())
    });
}

#[test]
#[ignore]
fn hw_3_two_pads_follow_their_own_setups_and_the_second_survives_the_first_leaving() {
    let _serial = serial();
    let root = temp_root();
    let snap = Snapshot::take();
    let daemon = start(&root, &two_pad_config());
    let mut keys = snap.open(KEYBOARD);
    let mut mouse = snap.open(MOUSE);
    let connect = |name: &str, uniq: &str| {
        let pad = HidPad::with_uniq(name, uniq);
        wait_for("the daemon to grab the pad", || find_node(name).filter(|path| grabbed(path)));
        pad
    };
    let mut a = connect("Pad A", ID_A);
    let mut b = connect("Pad B", ID_B);
    wait_for("both pads to be managed", || {
        let log = daemon_log(&root);
        (log.contains("managing Pad A (") && log.contains("managing Pad B (")).then_some(())
    });

    // Each pad follows its own profile: A's South is a key, B's is a mouse button.
    hold_south(&mut a, "pad A's South to hold KEY_A", || {
        keys.drain();
        mouse.drain();
        keys.pressed.contains(&KeyCode::KEY_A.0)
    });
    assert!(!mouse.pressed.contains(&KeyCode::BTN_LEFT.0), "pad A doesn't use pad B's profile");
    hold_south(&mut b, "pad B's South to hold the left button", || {
        keys.drain();
        mouse.drain();
        mouse.pressed.contains(&KeyCode::BTN_LEFT.0)
    });
    assert!(keys.pressed.contains(&KeyCode::KEY_A.0), "both are held at once");

    // Unplug the first while both are held. Its key comes up at once; the second's stays.
    drop(a);
    wait_for("pad A's node to go", || find_node("Pad A").is_none().then_some(()));
    wait_for("KEY_A to be released", || {
        keys.drain();
        mouse.drain();
        keys.pressed.is_empty().then_some(())
    });
    assert!(mouse.pressed.contains(&KeyCode::BTN_LEFT.0), "pad B keeps holding its button");
    assert!(daemon_log(&root).contains("device gone: Pad A"));

    // The second pad still works: let go, press again.
    b.report(0, 0, 0);
    wait_for("pad B's button to come up", || {
        mouse.drain();
        mouse.pressed.is_empty().then_some(())
    });
    hold_south(&mut b, "pad B to press again", || {
        mouse.drain();
        mouse.pressed.contains(&KeyCode::BTN_LEFT.0)
    });
    b.report(0, 0, 0);
    wait_for("nothing held", || {
        keys.drain();
        mouse.drain();
        (keys.pressed.is_empty() && mouse.pressed.is_empty()).then_some(())
    });

    // The first comes back and gets its own profile again, not the survivor's.
    let mut a = connect("Pad A", ID_A);
    hold_south(&mut a, "the returned pad A to hold KEY_A", || {
        keys.drain();
        mouse.drain();
        keys.pressed.contains(&KeyCode::KEY_A.0)
    });
    assert!(mouse.pressed.is_empty(), "the returned pad uses its own profile");
    drop((a, b));
    stop(daemon, &root);
}

// ---------------------------------------------------------------------------------------------
// HW-8: a program reading the pad's raw HID node (Steam does), with and without the udev rule.
// ---------------------------------------------------------------------------------------------

/// A program called `steam` with `node` open, like Steam reading a pad's raw HID node.
fn fake_steam(root: &Path, node: &Path) -> Child {
    let exe = root.join("steam");
    fs::copy("/bin/sleep", &exe).expect("copying sleep");
    Command::new("sh")
        .arg("-c")
        .arg(format!("exec 3<>{}; exec {} 60", node.display(), exe.display()))
        .stdin(Stdio::null())
        .spawn()
        .unwrap()
}

/// The hidraw node of the DualSense replica.
fn hidraw_of_dualsense() -> std::path::PathBuf {
    let gamepad = find_node(DUALSENSE).expect("the DualSense node");
    let event = gamepad.file_name().unwrap().to_string_lossy().to_string();
    let hidraw = fs::read_dir(format!("/sys/class/input/{event}/device/device/hidraw")).expect("the replica has a raw HID node").flatten().next().unwrap();
    Path::new("/dev").join(hidraw.file_name())
}

#[test]
#[ignore]
fn hw_8_a_program_reading_the_raw_node_is_warned_about_unless_the_udev_rule_hides_it() {
    let _serial = serial();
    let root = temp_root();
    let daemon = start(&root, &with_known_dualsense(&common::default_config(), 1));
    let pad = connect_dualsense(1);
    let node = hidraw_of_dualsense();

    // Without the rule the node is open to its users (the daemon here is root, so mode alone
    // can't stop a program, as it can't stop root in the rule's real setup either).
    let mut steam = fake_steam(&root, &node);
    let deadline = Instant::now() + Duration::from_secs(20);
    let readers = loop {
        let readers = status(&root)["raw_readers"].clone();
        if readers.as_array().is_some_and(|r| !r.is_empty()) {
            break readers;
        }
        assert!(Instant::now() < deadline, "the daemon never found the program reading the raw node: {}", daemon_log(&root));
        thread::sleep(Duration::from_millis(500));
    };
    assert_eq!(readers[0]["programs"][0], "steam", "{readers}");
    let log = daemon_log(&root);
    assert!(log.contains("Steam also reads") && log.contains("install dist/72-padwight-hidraw.rules"), "the log names the program and the fix: {log}");
    // The toast for whoever is in a fullscreen game.
    let toast = overlay_state(&root).to_string();
    assert!(toast.contains("Steam reads") && toast.contains("past Padwight"), "the toast says who reads the pad: {toast}");
    let _ = steam.kill();
    let _ = steam.wait();

    // With the rule: the node is mode 0000, which the daemon's scan skips. The rule also keeps
    // a program from opening it, so none has it open; the scan reports nothing.
    fs::set_permissions(&node, fs::Permissions::from_mode(0o000)).unwrap();
    let before = daemon_log(&root).matches("also reads").count();
    let mut steam = fake_steam(&root, &node);
    // A scan runs every few seconds; wait for two.
    thread::sleep(Duration::from_secs(14));
    assert!(status(&root)["raw_readers"].as_array().unwrap().is_empty(), "no warning with the rule in place: {}", status(&root));
    assert_eq!(daemon_log(&root).matches("also reads").count(), before, "nothing new in the log");
    let _ = steam.kill();
    let _ = steam.wait();
    drop(pad);
    stop(daemon, &root);
}

// ---------------------------------------------------------------------------------------------
// HW-10: `padwight disable; padwight enable` in one line, per family.
// ---------------------------------------------------------------------------------------------

/// Runs `padwight disable; padwight enable` as one shell line, the way a user types it.
fn disable_enable_in_one_line(root: &Path) {
    let line = format!("{bin} disable; {bin} enable", bin = env!("CARGO_BIN_EXE_padwight"));
    let status = Command::new("sh")
        .arg("-c")
        .arg(line)
        .env("XDG_CONFIG_HOME", root.join("config"))
        .env("XDG_RUNTIME_DIR", root.join("run"))
        .status()
        .unwrap();
    assert!(status.success());
}

#[test]
#[ignore]
fn hw_10_dualsense_is_taken_back_at_once_and_its_touchpad_still_drives_the_profile() {
    let _serial = serial();
    let root = temp_root();
    // Gyro on, so the motion node is paired too; the profile's touchpad is a laptop's.
    let config = common::default_config().replacen("[general.profiles.gyro]\nenabled = false", "[general.profiles.gyro]\nenabled = true", 1);
    let snap = Snapshot::take();
    let daemon = start(&root, &with_known_dualsense(&config, 1));
    let mut mouse = snap.open(MOUSE);
    let mut keys = snap.open(KEYBOARD);
    let mut pad = connect_dualsense(1);
    wait_for("the touchpad to be paired", || daemon_log(&root).contains("touchpad: using").then_some(()));
    let touchpad = format!("{DUALSENSE} Touchpad");

    for round in 1..=3 {
        disable_enable_in_one_line(&root);
        let log = daemon_log(&root);
        assert!(!log.contains("cannot grab Sony"), "round {round}: the pad was still held by the old reader: {log}");
        assert!(!log.contains("touchpad: cannot take"), "round {round}: the touchpad could not be taken: {log}");
        assert_eq!(log.matches("managing Sony").count(), round + 1, "round {round}: the pad was taken again: {log}");
        let node = find_node(&touchpad).expect("the touchpad node");
        assert!(grabbed(&node), "round {round}: the desktop still sees the touchpad");

        // A finger drawn across the pad moves the pointer through the profile; the desktop
        // pointer (the touchpad node itself) is held by the daemon, so only the virtual mouse moves.
        let before = mouse.moved;
        drive(&mut pad, "a finger to move the pointer", |t| DualSenseState { touch: [Some((200 + (t % 100) as u16 * 10, 500)), None], timestamp: clock(t), ..DualSenseState::default() }, || {
            mouse.drain();
            mouse.moved != before
        });
        drive(&mut pad, "the finger to lift", |t| DualSenseState { timestamp: clock(t), ..DualSenseState::default() }, || {
            mouse.drain();
            true
        });
        // And the buttons work.
        let cross = DualSenseState { cross: true, ..DualSenseState::default() };
        let mut out = snap.open(PAD);
        drive(&mut pad, "cross to pass through", |_| cross.clone(), || {
            out.drain();
            out.pressed.contains(&KeyCode::BTN_SOUTH.0)
        });
        keys.drain();
        drive(&mut pad, "rest", |_| DualSenseState::default(), || {
            out.drain();
            out.pressed.is_empty()
        });
    }
    drop(pad);
    stop(daemon, &root);
}

#[test]
#[ignore]
fn hw_10_generic_pad_is_taken_back_at_once_after_disable_and_enable_in_one_line() {
    let _serial = serial();
    let root = temp_root();
    let snap = Snapshot::take();
    let daemon = start(&root, &hold_config());
    let mut keys = snap.open(KEYBOARD);
    let mut mouse = snap.open(MOUSE);
    let mut pad = HidPad::new("Test Controller");
    wait_for("the daemon to grab the pad", || find_node("Test Controller").filter(|path| grabbed(path)));
    for round in 1..=3 {
        disable_enable_in_one_line(&root);
        let log = daemon_log(&root);
        assert!(!log.contains("cannot grab Test Controller"), "round {round}: {log}");
        assert_eq!(log.matches("managing Test Controller (").count(), round + 1, "round {round}: {log}");
        wait_for("South and East to work again", || {
            pad.report(BUTTON_SOUTH | BUTTON_EAST, 0, 0);
            thread::sleep(Duration::from_millis(20));
            keys.drain();
            mouse.drain();
            (keys.pressed.contains(&KeyCode::KEY_A.0) && mouse.pressed.contains(&KeyCode::BTN_LEFT.0)).then_some(())
        });
        pad.report(0, 0, 0);
        wait_for("both let go", || {
            keys.drain();
            mouse.drain();
            (keys.pressed.is_empty() && mouse.pressed.is_empty()).then_some(())
        });
    }
    drop(pad);
    stop(daemon, &root);
}

// ---------------------------------------------------------------------------------------------
// HW-11: the daemon restarted (SIGTERM, as systemctl restart sends) or killed (SIGKILL) mid-macro,
// mid-layer and with a menu open: it comes back, releases everything and doesn't grab twice.
// ---------------------------------------------------------------------------------------------

/// `hold_config` with a macro on square that holds KEY_M for five seconds, a list menu on
/// triangle, and a layer on L1 in which circle holds KEY_H.
fn restart_config() -> String {
    let text = hold_config()
        .replacen("rules = []\nmacros = []\nmenus = []\ninfo_overlays = []", "rules = []\ninfo_overlays = []", 1)
        .replacen("[general.profiles.buttons.North]\ngamepad = \"North\"", "[general.profiles.buttons.North]\nopen_menu = \"List\"", 1)
        .replacen("[general.profiles.buttons.West]\ngamepad = \"West\"", "[general.profiles.buttons.West]\nmacro = { name = \"Hold\" }", 1)
        .replacen("[general.profiles.buttons.LeftBumper]\ngamepad = \"LeftBumper\"", "[general.profiles.buttons.LeftBumper]\nlayer = \"Hold\"", 1);
    assert!(text.contains("open_menu = \"List\"") && text.contains("macro = { name"), "the default config has North and West as passthrough");
    let mut text = text;
    text.push_str("\n[[general.macros]]\nname = \"Hold\"\nsteps = [{ press = { keys = [\"KEY_M\"] } }, { wait = 5000 }, { release = { keys = [\"KEY_M\"] } }]\n");
    text.push_str("\n[[general.layers]]\nname = \"Hold\"\n[general.layers.buttons.East]\nkeys = [\"KEY_H\"]\n");
    text.push_str("\n[[general.menus]]\nname = \"List\"\nkind = \"list\"\n");
    for k in ["A", "B"] {
        text.push_str(&format!("[[general.menus.items]]\nlabel = \"{k}\"\naction = {{ keys = [\"KEY_{k}\"] }}\n"));
    }
    text
}

/// Whether the menu "List" is showing.
fn menu_up(root: &Path) -> bool {
    overlay_state(root)["menu"]["title"] == "List"
}

#[derive(Clone, Copy, Debug)]
enum Moment {
    Macro,
    Layer,
    Menu,
}

#[derive(Clone, Copy, Debug)]
enum Signal {
    Term,
    Kill,
}

fn restart_case(moment: Moment, signal: Signal) {
    let _serial = serial();
    let root = temp_root();
    let snap = Snapshot::take();
    let mut daemon = start(&root, &with_known_dualsense(&restart_config(), 1));
    let mut keys = snap.open(KEYBOARD);
    let mut mouse = snap.open(MOUSE);
    let mut pad = connect_dualsense(1);
    let pad_node = find_node(DUALSENSE).unwrap();

    // Get into the moment, and stay in it: the pad keeps reporting what it held.
    match moment {
        Moment::Macro => {
            let square = DualSenseState { square: true, ..DualSenseState::default() };
            drive(&mut pad, "the macro to hold KEY_M", |_| square.clone(), || {
                keys.drain();
                keys.pressed.contains(&KeyCode::KEY_M.0)
            });
            pad.send(&DualSenseState::default().report()); // the macro goes on after the button is let go
        }
        Moment::Layer => {
            // L1 first, so the layer is up before circle goes down; then both stay held.
            let bumper = DualSenseState { shoulders: 0x01, ..DualSenseState::default() };
            let both = DualSenseState { shoulders: 0x01, circle: true, ..DualSenseState::default() };
            drive(&mut pad, "the layer to hold KEY_H", |t| if t < 25 { bumper.clone() } else { both.clone() }, || {
                keys.drain();
                keys.pressed.contains(&KeyCode::KEY_H.0)
            });
        }
        Moment::Menu => {
            let press = DualSenseState { triangle: true, ..DualSenseState::default() };
            drive(&mut pad, "the menu to open", |_| press.clone(), || menu_up(&root));
            pad.send(&DualSenseState::default().report());
            assert!(menu_up(&root), "the menu stays open: {}", overlay_state(&root));
        }
    }

    // Stop it the way a restart does. SIGTERM must release the keys before the daemon is gone.
    let old_keys = snap.mine(KEYBOARD);
    match signal {
        Signal::Kill => daemon.0.kill().unwrap(),
        // SAFETY: `kill` only sends a signal, to a child we started and have not yet waited for.
        Signal::Term => assert_eq!(unsafe { libc::kill(daemon.0.id() as libc::pid_t, libc::SIGTERM) }, 0),
    }
    let deadline = Instant::now() + TIMEOUT;
    let mut released_before_exit = false;
    loop {
        keys.drain();
        mouse.drain();
        released_before_exit |= keys.pressed.is_empty() && mouse.pressed.is_empty();
        if daemon.0.try_wait().unwrap().is_some() {
            break;
        }
        assert!(Instant::now() < deadline, "{moment:?}/{signal:?}: the daemon didn't exit");
        thread::sleep(Duration::from_millis(5));
    }
    if matches!(signal, Signal::Term) {
        assert!(released_before_exit, "{moment:?}/SIGTERM: keys still held when the daemon exited: {:?}", keys.pressed);
    }
    wait_for("the old virtual devices to go", || (keys.drain() && mouse.drain()).then_some(()));
    wait_for("the daemon's devices to disappear", || (snap.mine(KEYBOARD).is_empty() && snap.mine(MOUSE).is_empty() && snap.mine(PAD).is_empty()).then_some(()));
    assert!(old_keys.len() == 1);
    assert!(!grabbed(&pad_node), "{moment:?}/{signal:?}: the pad is still grabbed after the daemon is gone");
    assert_log_clean(&root);

    // It comes back (after SIGKILL, over its stale socket) and takes the pad once.
    let mut daemon = spawn_daemon(&root, Some(&root.join("run")));
    wait_for("the daemon to grab the pad again", || grabbed(&pad_node).then_some(()));
    assert_eq!(snap.mine(KEYBOARD).len(), 1, "one virtual keyboard, not two");
    let mut keys = snap.open(KEYBOARD);
    let mut mouse = snap.open(MOUSE);
    // The pad lets go of whatever it held, and nothing stays pressed on the new devices. The
    // five-second macro of the old daemon must not have come back with it.
    drive(&mut pad, "the pad to rest", |_| DualSenseState::default(), || {
        keys.drain();
        mouse.drain();
        keys.pressed.is_empty() && mouse.pressed.is_empty()
    });
    thread::sleep(Duration::from_millis(700));
    keys.drain();
    assert!(keys.pressed.is_empty(), "{moment:?}/{signal:?}: keys held after the restart: {:?}", keys.pressed);
    assert!(!keys.downs.contains_key(&KeyCode::KEY_M.0), "{moment:?}/{signal:?}: the old macro came back");
    assert!(!menu_up(&root), "{moment:?}/{signal:?}: the menu is still up: {}", overlay_state(&root));

    // And the pad is usable: circle holds the left mouse button.
    let circle = DualSenseState { circle: true, ..DualSenseState::default() };
    drive(&mut pad, "circle to work after the restart", |_| circle.clone(), || {
        mouse.drain();
        mouse.pressed.contains(&KeyCode::BTN_LEFT.0)
    });
    drive(&mut pad, "rest", |_| DualSenseState::default(), || {
        mouse.drain();
        mouse.pressed.is_empty()
    });
    let log = daemon_log(&root);
    assert_eq!(log.matches("managing Sony").count(), 1, "{moment:?}/{signal:?}: the pad was taken once: {log}");
    assert!(!log.contains("cannot grab Sony"), "{log}");
    assert!(daemon.0.try_wait().unwrap().is_none());
    let _ = daemon.0.kill();
    let _ = daemon.0.wait();
    drop(pad);
    assert_log_clean(&root);
    let _ = fs::remove_dir_all(&root);
    assert_no_other_daemon();
}

#[test]
#[ignore]
fn hw_11_restart_mid_macro_with_sigterm() {
    restart_case(Moment::Macro, Signal::Term);
}

#[test]
#[ignore]
fn hw_11_restart_mid_macro_with_sigkill() {
    restart_case(Moment::Macro, Signal::Kill);
}

#[test]
#[ignore]
fn hw_11_restart_mid_layer_with_sigterm() {
    restart_case(Moment::Layer, Signal::Term);
}

#[test]
#[ignore]
fn hw_11_restart_mid_layer_with_sigkill() {
    restart_case(Moment::Layer, Signal::Kill);
}

#[test]
#[ignore]
fn hw_11_restart_with_a_menu_open_with_sigterm() {
    restart_case(Moment::Menu, Signal::Term);
}

#[test]
#[ignore]
fn hw_11_restart_with_a_menu_open_with_sigkill() {
    restart_case(Moment::Menu, Signal::Kill);
}
