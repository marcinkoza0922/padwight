//! ENV-2 without the other programs: another process holding the controller, and a remapper's
//! duplicate virtual pad, as InputPlumber, Handheld Daemon or AntiMicroX would leave them.
//!
//! The "other program" is a stub that opens the controller's node and takes the evdev grab (what
//! InputPlumber does to hide the physical pad), and a uinput device that looks like the pad (what
//! it publishes in its place). The tests check what the daemon does about each: it must not type
//! the pad's input twice, must say once in its log that it could not take the pad, and must not
//! retry the grab in a loop.
//!
//! The real InputPlumber, Handheld Daemon and AntiMicroX are not run, so how they present the
//! pad on a ROG Ally or Bazzite stays a manual check. Needs /dev/uinput and /dev/uhid and read
//! access to /dev/input: `scripts/kernel-test-docker.sh second_grabber`.

#![cfg(target_os = "linux")]

mod common;

use std::{fs, thread, time::Duration};

use common::{
    assert_log_clean, daemon_log, hold_config, install_config,
    input::{Reader, assert_no_other_daemon, find_node},
    serial, spawn_daemon, temp_root,
    uhid::{BUTTON_SOUTH, HidPad},
    wait_for,
};
use evdev::{AbsInfo, AbsoluteAxisCode, AttributeSet, BusType, Device, EventType, InputEvent, InputId, KeyCode, UinputAbsSetup, uinput::VirtualDevice};

const NAME: &str = "Held Controller";

fn start(root: &std::path::Path) {
    assert_no_other_daemon();
    let config = root.join("config/padwight");
    fs::create_dir_all(&config).unwrap();
    install_config(&config, &hold_config());
}

/// Takes the controller's node and its grab, as a remapper that hides the physical pad does.
fn grab_like_a_remapper(name: &str) -> Device {
    let node = wait_for("the controller's node", || find_node(name));
    let mut dev = Device::open(node).unwrap();
    dev.set_nonblocking(true).unwrap();
    dev.grab().expect("the stub takes the grab first");
    dev
}

/// A uinput pad that looks like `name`, as a remapper publishes in place of the physical one.
fn duplicate_pad(name: &str) -> VirtualDevice {
    let mut keys = AttributeSet::<KeyCode>::new();
    for key in [KeyCode::BTN_SOUTH, KeyCode::BTN_EAST] {
        keys.insert(key);
    }
    let stick = AbsInfo::new(0, -32768, 32767, 16, 128, 0);
    VirtualDevice::builder()
        .unwrap()
        .name(name)
        .input_id(InputId::new(BusType::BUS_USB, 0x045e, 0x028e, 1))
        .with_keys(&keys)
        .unwrap()
        .with_absolute_axis(&UinputAbsSetup::new(AbsoluteAxisCode::ABS_X, stick))
        .unwrap()
        .with_absolute_axis(&UinputAbsSetup::new(AbsoluteAxisCode::ABS_Y, stick))
        .unwrap()
        .build()
        .unwrap()
}

fn press(dev: &mut VirtualDevice, down: bool) {
    dev.emit(&[InputEvent::new(EventType::KEY.0, KeyCode::BTN_SOUTH.0, i32::from(down))]).unwrap();
}

#[test]
#[ignore]
fn second_grabber_a_pad_held_by_another_program_is_reported_once_and_typed_zero_times() {
    let _serial = serial();
    let root = temp_root();
    start(&root);
    let mut pad = HidPad::new(NAME);
    let mut stub = grab_like_a_remapper(NAME);
    let daemon = spawn_daemon(&root, Some(&root.join("run")));
    let mut keys = Reader::open("Padwight Virtual Keyboard");

    wait_for("the daemon to say it cannot grab the pad", || daemon_log(&root).contains("cannot grab").then_some(()));
    // Press South for three seconds: the stub, which owns the grab, sees it all; Padwight types nothing.
    let mut stub_saw = 0;
    for _ in 0..60 {
        pad.report(BUTTON_SOUTH, 0, 0);
        thread::sleep(Duration::from_millis(25));
        pad.report(0, 0, 0);
        thread::sleep(Duration::from_millis(25));
        stub_saw += stub.fetch_events().map_or(0, |events| events.filter(|e| e.event_type() == EventType::KEY).count());
        keys.drain();
        assert!(keys.pressed.is_empty(), "the daemon typed input from a pad someone else holds: {:?}", keys.pressed);
    }
    assert!(stub_saw > 0, "the holder still receives the pad's input");
    let log = daemon_log(&root);
    assert_eq!(log.matches("cannot grab").count(), 1, "one line says so, and it does not retry in a loop: {log}");
    assert!(log.contains("busy"), "the line carries the kernel's reason (the grab is busy): {log}");
    assert!(!log.contains(&format!("managing {NAME}")), "not managed: {log}");
    drop(daemon);
    assert_log_clean(&root);
    let _ = fs::remove_dir_all(&root);
}

#[test]
#[ignore]
fn second_grabber_a_remappers_duplicate_pad_is_not_managed_as_a_second_pad() {
    let _serial = serial();
    let root = temp_root();
    start(&root);
    let mut pad = HidPad::new(NAME);
    let mut duplicate = duplicate_pad("Duplicate Controller");
    let daemon = spawn_daemon(&root, Some(&root.join("run")));
    let mut keys = Reader::open("Padwight Virtual Keyboard");

    // The physical pad is managed once; the uinput duplicate is another program's output and is left alone.
    wait_for("the daemon to manage the physical pad", || daemon_log(&root).contains(&format!("managing {NAME} (")).then_some(()));
    wait_for("KEY_A from the physical pad", || {
        pad.report(BUTTON_SOUTH, 0, 0);
        thread::sleep(Duration::from_millis(20));
        keys.drain();
        keys.pressed.contains(&KeyCode::KEY_A.0).then_some(())
    });
    pad.report(0, 0, 0);
    wait_for("KEY_A to be released", || {
        keys.drain();
        keys.pressed.is_empty().then_some(())
    });
    // The duplicate presses South as well: nothing is typed, so it is not read.
    for _ in 0..20 {
        press(&mut duplicate, true);
        thread::sleep(Duration::from_millis(20));
        press(&mut duplicate, false);
        keys.drain();
        assert!(keys.pressed.is_empty(), "the duplicate pad was typed: {:?}", keys.pressed);
    }
    let log = daemon_log(&root);
    assert_eq!(log.matches("managing ").count(), 1, "one pad managed: {log}");
    assert!(!log.contains("managing Duplicate Controller"), "{log}");
    drop(daemon);
    assert_log_clean(&root);
    let _ = fs::remove_dir_all(&root);
}

#[test]
#[ignore]
fn second_grabber_a_hidden_physical_pad_with_only_a_duplicate_left_is_not_taken_over() {
    // InputPlumber's whole arrangement: the physical pad is grabbed by it, and only its virtual
    // pad is left for everyone else. Padwight sees one uinput pad and one grabbed pad, and manages
    // neither, so there is no second source of input for the game to see.
    let _serial = serial();
    let root = temp_root();
    start(&root);
    let mut pad = HidPad::new(NAME);
    let _stub = grab_like_a_remapper(NAME);
    let mut duplicate = duplicate_pad("InputPlumber-like Controller");
    let daemon = spawn_daemon(&root, Some(&root.join("run")));
    let mut keys = Reader::open("Padwight Virtual Keyboard");

    wait_for("the daemon to say it cannot grab the pad", || daemon_log(&root).contains("cannot grab").then_some(()));
    for _ in 0..40 {
        pad.report(BUTTON_SOUTH, 0, 0);
        press(&mut duplicate, true);
        thread::sleep(Duration::from_millis(20));
        pad.report(0, 0, 0);
        press(&mut duplicate, false);
        thread::sleep(Duration::from_millis(20));
        keys.drain();
        assert!(keys.pressed.is_empty(), "something was typed: {:?}", keys.pressed);
    }
    let log = daemon_log(&root);
    assert!(!log.contains("managing "), "nothing managed: {log}");
    assert_eq!(log.matches("cannot grab").count(), 1, "{log}");
    drop(daemon);
    assert_log_clean(&root);
    let _ = fs::remove_dir_all(&root);
}
