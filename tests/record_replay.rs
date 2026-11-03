//! `padwight debug record` and `replay`, end to end: a virtual pad is recorded while it is moved,
//! the recording is replayed as a HID gamepad, and a daemon maps what it plays.
//!
//! Needs /dev/uinput and /dev/uhid, and read access to /dev/input, so it is opt-in:
//! `cargo test --test record_replay -- --ignored`, or `scripts/kernel-test-docker.sh record_replay`.

#![cfg(target_os = "linux")]

mod common;

use std::{
    fs,
    process::{Command, Stdio},
    thread,
    time::{Duration, Instant},
};

use common::{
    Daemon, hold_config, install_config,
    input::{Reader, assert_no_other_daemon},
    serial, spawn_daemon, temp_root,
};
use evdev::{AbsInfo, AbsoluteAxisCode, AttributeSet, BusType, EventType, InputEvent, InputId, KeyCode, UinputAbsSetup, uinput::VirtualDevice};

const BIN: &str = env!("CARGO_BIN_EXE_padwight");
const NAME: &str = "Record Test Pad";

/// A pad made through uinput (which a daemon ignores, so it is only ever recorded).
fn source_pad() -> VirtualDevice {
    let keys: AttributeSet<KeyCode> = [KeyCode::BTN_SOUTH, KeyCode::BTN_EAST].into_iter().collect();
    let axis = |code| UinputAbsSetup::new(code, AbsInfo::new(0, -32768, 32767, 16, 128, 0));
    VirtualDevice::builder()
        .unwrap()
        .name(NAME)
        .input_id(InputId::new(BusType::BUS_USB, 0x1234, 0x5678, 1))
        .with_keys(&keys)
        .unwrap()
        .with_absolute_axis(&axis(AbsoluteAxisCode::ABS_X))
        .unwrap()
        .with_absolute_axis(&axis(AbsoluteAxisCode::ABS_Y))
        .unwrap()
        .build()
        .unwrap()
}

/// Holds South for `secs` seconds, moving the stick a little every 100 ms (so each frame says South is held).
fn play_by_hand(pad: &mut VirtualDevice, secs: u64) {
    let key = |value| InputEvent::new(EventType::KEY.0, KeyCode::BTN_SOUTH.0, value);
    pad.emit(&[key(1)]).unwrap();
    for i in 0..secs * 10 {
        thread::sleep(Duration::from_millis(100));
        pad.emit(&[InputEvent::new(EventType::ABSOLUTE.0, AbsoluteAxisCode::ABS_X.0, 1000 + i as i32)]).unwrap();
    }
    pad.emit(&[key(0)]).unwrap();
}

#[test]
#[ignore]
fn record_replay_a_recorded_pad_is_mapped_by_the_daemon() {
    let _serial = serial();
    assert_no_other_daemon();
    let root = temp_root();
    let file = root.join("pad.rec");

    // Record the pad while it is held and moved by hand.
    let mut pad = source_pad();
    thread::sleep(Duration::from_millis(500));
    let recorder = Command::new(BIN).args(["debug", "record"]).arg(&file).args(["--device", NAME, "--seconds", "6"]).stderr(Stdio::piped()).spawn().unwrap();
    thread::sleep(Duration::from_millis(1000));
    play_by_hand(&mut pad, 4);
    let out = recorder.wait_with_output().unwrap();
    assert!(out.status.success(), "record exits cleanly: {}", String::from_utf8_lossy(&out.stderr));
    drop(pad);

    let text = fs::read_to_string(&file).unwrap();
    assert!(text.contains(&format!("device 0 \"{NAME}\" bus=3 vendor=0x1234 product=0x5678")), "{text}");
    assert!(text.contains("abs 0 min=-32768 max=32767 fuzz=16 flat=128"), "{text}");
    assert!(text.contains("# BTN_SOUTH") && text.contains("# ABS_X"), "{text}");
    assert!(text.lines().filter(|l| l.contains("# ABS_X")).count() >= 15, "every stick step was recorded: {text}");

    // Replay it as a HID gamepad under a daemon whose South holds KEY_A.
    let config = root.join("config/padwight");
    fs::create_dir_all(&config).unwrap();
    install_config(&config, &hold_config());
    let _daemon: Daemon = spawn_daemon(&root, Some(&root.join("run")));
    let mut keys = Reader::open("Padwight Virtual Keyboard");
    let mut replay = Command::new(BIN).args(["debug", "replay"]).arg(&file).args(["--uhid", "--lead-in", "4000", "--hold", "300", "--uniq", "padwight-replay-pad"]).stderr(Stdio::piped()).spawn().unwrap();
    let (mut saw_press, mut saw_release_after) = (false, false);
    let deadline = Instant::now() + Duration::from_secs(20);
    while replay.try_wait().unwrap().is_none() && Instant::now() < deadline {
        keys.drain();
        let held = keys.pressed.contains(&KeyCode::KEY_A.0);
        saw_press |= held;
        saw_release_after |= saw_press && !held;
        thread::sleep(Duration::from_millis(10));
    }
    keys.drain();
    if !saw_press {
        let mut err = String::new();
        std::io::Read::read_to_string(&mut replay.stderr.take().unwrap(), &mut err).unwrap();
        panic!("the replayed South press did not type KEY_A; replay said {err}; daemon said {}", common::daemon_log(&root));
    }
    assert!(saw_release_after || keys.pressed.is_empty(), "and its release let go");
    assert!(replay.wait().unwrap().success());
    let _ = fs::remove_dir_all(&root);
}
