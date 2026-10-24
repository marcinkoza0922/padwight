//! A stick moving the mouse, at the daemon's usual 250 Hz continuous output and at 1000 Hz
//! (App settings → Input): how long from pushing the stick to the first motion of the virtual
//! mouse, how many motion reports a second it sends while held, and what the daemon uses meanwhile.

use std::time::{Duration, Instant};

use evdev::{EventType, InputEvent, RelativeAxisCode};
use serde_json::{Value, json};

use super::support::{Monitor, Output, Pad, Running, latency_stats, now_ns, padwight, report, wait_managed};
use crate::common::{default_config, install_config, serial, temp_root, TIMEOUT};

const NAME: &str = "Profile Controller";
/// Pushes per rate, each from rest.
const PUSHES: usize = 100;
/// How long the stick stays pushed for the steady figures.
const HOLD: Duration = Duration::from_secs(10);

#[test]
#[ignore]
fn profile_stick_mouse() {
    let _serial = serial();
    let normal = rate(false);
    let fast = rate(true);
    report(
        "stick_mouse",
        json!({
            "what": "left stick as a mouse: push from rest to the first REL_X of the daemon's virtual mouse, then held",
            "pushes": PUSHES,
            "hold_seconds": HOLD.as_secs_f64(),
            "rate_250hz": normal,
            "rate_1000hz": fast,
        }),
    );
}

/// One daemon with the left stick as a mouse, at the usual rate or the fast one.
fn rate(fast: bool) -> Value {
    let root = temp_root();
    let config = default_config().replacen(
        "[general.profiles.left_stick.action.gamepad]\nstick = \"Left\"",
        "[general.profiles.left_stick.action.mouse]\nspeed = 2000.0",
        1,
    );
    assert!(config.contains("speed = 2000.0"), "the default config's first profile has a gamepad left stick");
    let config = format!("fast_tick = {fast}\n{config}");
    install_config(&root.join("config/padwight"), &config);
    let mut pad = Pad::new(NAME);
    let daemon = Running(padwight(&root, &["daemon"]).spawn().expect("starting the daemon"));
    wait_managed(Instant::now(), &root.join("run"), NAME, TIMEOUT).expect("the daemon grabbed the pad");
    let mut mouse = Output::open_mouse();
    let moved = |ev: &InputEvent| ev.event_type() == EventType::RELATIVE && ev.code() == RelativeAxisCode::REL_X.0;

    let mut onset = Vec::new();
    let mut missed = 0;
    for _ in 0..PUSHES {
        pad.set_x(0);
        mouse.wait(Duration::from_millis(150), |_| false, || {});
        let pushed = now_ns();
        pad.set_x(i16::MAX);
        match mouse.wait(Duration::from_secs(1), moved, || {}) {
            Some(seen) => onset.push(seen.saturating_sub(pushed)),
            None => missed += 1,
        }
    }

    // Held: the reports the mouse sends, each a SYN after motion, and the daemon's use.
    mouse.wait(Duration::from_millis(200), |_| false, || {});
    let usage = Monitor::start(daemon.pid());
    let mut reports = 0u64;
    let started = Instant::now();
    mouse.wait(HOLD, |ev| {
        reports += u64::from(ev.event_type() == EventType::SYNCHRONIZATION);
        false
    }, || {});
    let seconds = started.elapsed().as_secs_f64();
    let usage = usage.finish();
    pad.set_x(0);

    drop(daemon);
    let _ = std::fs::remove_dir_all(&root);
    json!({
        "onset": latency_stats(onset),
        "missed": missed,
        "reports_per_second": reports as f64 / seconds,
        "usage": usage,
    })
}
