//! Controllers made with uhid that the kernel's real drivers bind (T-3 step 1): the DualSense goes
//! through `hid-playstation`, which makes the gamepad, motion-sensor and touchpad nodes a real one
//! has. The daemon then reads them the way it reads a real controller's.
//!
//! Needs /dev/uinput and /dev/uhid, read access to /dev/input, and the driver's module on the
//! host (the kernel loads it when the device appears), so it is opt-in:
//! `scripts/kernel-test-docker.sh replica_`.

#![cfg(target_os = "linux")]

mod common;

use std::{fs, path::Path, thread, time::Duration};

use common::{
    Daemon, assert_log_clean, daemon_log, default_config, install_config,
    input::{Reader, assert_no_other_daemon, find_node, grabbed},
    serial, spawn_daemon, temp_root,
    uhid::{DualSenseState, HidPad},
    wait_for,
};
use evdev::{Device, KeyCode, PropType};

const DUALSENSE: &str = "Sony Interactive Entertainment DualSense Wireless Controller";

/// The default config with gyro on in the active profile (turning moves the mouse).
fn gyro_config() -> String {
    default_config().replacen("[general.profiles.gyro]\nenabled = false", "[general.profiles.gyro]\nenabled = true", 1)
}

fn start(root: &Path, config: &str) -> Daemon {
    assert_no_other_daemon();
    let dir = root.join("config/padwight");
    fs::create_dir_all(&dir).unwrap();
    install_config(&dir, config);
    spawn_daemon(root, Some(&root.join("run")))
}

/// Sends the state until `done` says the effect showed, since the kernel drops reports nobody reads.
fn drive(pad: &mut HidPad, what: &str, mut next: impl FnMut(u32) -> DualSenseState, mut done: impl FnMut() -> bool) {
    let mut tick = 0u32;
    wait_for(what, || {
        tick += 1;
        pad.send(&next(tick).report());
        thread::sleep(Duration::from_millis(8));
        done().then_some(())
    });
}

/// The sensor's clock (0.33 microsecond units) for the `tick`th report, 4 ms apart.
fn clock(tick: u32) -> u32 {
    tick * 12_000
}

#[test]
#[ignore]
fn replica_dualsense_gets_the_real_drivers_nodes() {
    let _serial = serial();
    assert_no_other_daemon();
    let pad = HidPad::dualsense(1);
    let gamepad = wait_for("the DualSense gamepad node", || find_node(DUALSENSE));
    let motion = wait_for("the motion node", || find_node(&format!("{DUALSENSE} Motion Sensors")));
    let touchpad = wait_for("the touchpad node", || find_node(&format!("{DUALSENSE} Touchpad")));

    let dev = Device::open(&gamepad).unwrap();
    assert_eq!((dev.input_id().vendor(), dev.input_id().product()), (0x054c, 0x0ce6));
    assert_eq!(dev.unique_name(), Some("02:70:61:64:00:01"), "the driver reports the pad's address as its unique id");
    assert!(dev.supported_keys().unwrap().contains(KeyCode::BTN_SOUTH));
    assert!(Device::open(&motion).unwrap().properties().contains(PropType::ACCELEROMETER), "the sensors are marked as a motion sensor");
    assert!(Device::open(&touchpad).unwrap().supported_keys().unwrap().contains(KeyCode::BTN_LEFT));
    // The driver that bound it is hid-playstation, not hid-generic.
    let node = gamepad.file_name().unwrap().to_string_lossy().to_string();
    let driver = fs::canonicalize(format!("/sys/class/input/{node}/device/device/driver")).unwrap();
    assert_eq!(driver.file_name().unwrap(), "playstation");
    drop(pad);
}

#[test]
#[ignore]
fn replica_dualsense_gyro_and_touchpad_reach_the_engine() {
    let _serial = serial();
    let root = temp_root();
    let mut daemon = start(&root, &gyro_config());
    let mut mouse = Reader::open("Padwight Virtual Mouse");
    let mut pad = HidPad::dualsense(1);
    wait_for("the daemon to grab the DualSense", || find_node(DUALSENSE).filter(|path| grabbed(path)));
    wait_for("the daemon to pair the motion sensor and touchpad", || {
        let log = daemon_log(&root);
        (log.contains("gyro: using") && log.contains("touchpad: using")).then_some(())
    });

    // Turning the controller (yaw) moves the pointer.
    drive(&mut pad, "the pointer to move with the gyro", |t| DualSenseState { gyro: [0, 6000, 0], timestamp: clock(t), ..DualSenseState::default() }, || {
        mouse.drain();
        mouse.moved.0 != 0
    });

    // A finger drawn across the pad moves the pointer, and pressing the pad down clicks.
    let before = mouse.moved;
    drive(&mut pad, "the pointer to move with a finger", |t| DualSenseState { touch: [Some((200 + (t % 100) as u16 * 10, 500)), None], timestamp: clock(t), ..DualSenseState::default() }, || {
        mouse.drain();
        mouse.moved != before
    });
    drive(&mut pad, "the touchpad click to press the left button", |t| DualSenseState { touch_click: true, timestamp: clock(t), ..DualSenseState::default() }, || {
        mouse.drain();
        mouse.pressed.contains(&KeyCode::BTN_LEFT.0)
    });

    assert!(daemon.0.try_wait().unwrap().is_none());
    let _ = daemon.0.kill();
    let _ = daemon.0.wait();
    assert_log_clean(&root);
    let _ = fs::remove_dir_all(&root);
}

/// HW-4: the motion node can appear after the pad is already managed (a udev rule applied late,
/// or the kernel making the nodes one after another). The daemon pairs it whenever it shows up.
/// The nodes come a few microseconds apart, so which order the daemon sees them in is up to the
/// race; the test plugs the pad in several times and checks that gyro is paired and works each time.
#[test]
#[ignore]
fn replica_dualsense_motion_node_arriving_after_the_pad_is_paired() {
    let _serial = serial();
    let root = temp_root();
    let mut daemon = start(&root, &gyro_config());
    let mut mouse = Reader::open("Padwight Virtual Mouse");
    for round in 1..=4u8 {
        let mut pad = HidPad::dualsense(round);
        wait_for("the daemon to grab the DualSense", || find_node(DUALSENSE).filter(|path| grabbed(path)));
        wait_for("the sensor to be paired", || (daemon_log(&root).matches("gyro: using").count() == usize::from(round)).then_some(()));
        let before = mouse.moved;
        drive(&mut pad, "gyro to work after the pairing", |t| DualSenseState { gyro: [0, 6000, 0], timestamp: clock(t), ..DualSenseState::default() }, || {
            mouse.drain();
            mouse.moved != before
        });
        drop(pad);
        wait_for("the DualSense node to go", || find_node(DUALSENSE).is_none().then_some(()));
    }
    let log = daemon_log(&root);
    assert_eq!(log.matches("managing Sony").count(), 4, "{log}");
    let _ = daemon.0.kill();
    let _ = daemon.0.wait();
    assert_log_clean(&root);
    let _ = fs::remove_dir_all(&root);
}
