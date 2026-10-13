//! What overlay sounds cost: this process's CPU with no audio device open, with the default output
//! open and silent (what the daemon held from start-up before it learned to open it on the first
//! cue), and while cues play as fast as a cursor moves; and how long the device takes to open. Needs a sound server or ALSA device.

use std::time::{Duration, Instant};

use serde_json::json;

use super::support::report;
use crate::common::serial;

const WINDOW: Duration = Duration::from_secs(6);
/// A cue every cursor step, as `styles.rs` moves the cursor.
const STEP: Duration = Duration::from_millis(150);
const RATE: u32 = 48_000;

#[test]
#[ignore]
fn profile_sound() {
    let _serial = serial();
    let closed = cpu_over(WINDOW, || {});
    // How long opening takes, which a cue waits for when the device opens with it.
    let opens: Vec<f64> = (0..5)
        .filter_map(|_| {
            let started = Instant::now();
            let sink = rodio::DeviceSinkBuilder::open_default_sink().ok()?;
            let ms = started.elapsed().as_secs_f64() * 1000.0;
            drop(sink);
            Some(ms)
        })
        .collect();
    let Ok(sink) = rodio::DeviceSinkBuilder::open_default_sink() else {
        report("sound", json!({"skipped": "no audio device opened"}));
        return;
    };
    let open = cpu_over(WINDOW, || {});
    // A short click, like the default step sound.
    let click: Vec<f32> = (0..RATE / 25).map(|i| (i as f32 * 0.2).sin() * (-(i as f32) / 400.0).exp() * 0.2).collect();
    let mut next = Instant::now();
    let playing = cpu_over(WINDOW, || {
        if Instant::now() >= next {
            let player = rodio::Player::connect_new(sink.mixer());
            let rate = std::num::NonZero::new(RATE).unwrap();
            player.append(rodio::buffer::SamplesBuffer::new(std::num::NonZero::<u16>::MIN, rate, click.clone()));
            player.detach();
            next += STEP;
        }
    });
    drop(sink);
    report(
        "sound",
        json!({
            "what": "this process's CPU, as a share of one core, over each window",
            "device_closed_cpu_percent": closed,
            "device_open_silent_cpu_percent": open,
            "cue_every_step_cpu_percent": playing,
            "step_ms": STEP.as_millis() as u64,
            "open_ms": opens,
        }),
    );
}

/// This process's CPU over `how_long`, calling `tick` every few milliseconds.
fn cpu_over(how_long: Duration, mut tick: impl FnMut()) -> f64 {
    let started = Instant::now();
    let before = self_ticks();
    while started.elapsed() < how_long {
        tick();
        std::thread::sleep(Duration::from_millis(10));
    }
    // SAFETY: sysconf only reads a system setting.
    let per_second = unsafe { libc::sysconf(libc::_SC_CLK_TCK) } as f64;
    (self_ticks() - before) as f64 / per_second / started.elapsed().as_secs_f64() * 100.0
}

fn self_ticks() -> u64 {
    let stat = std::fs::read_to_string("/proc/self/stat").unwrap_or_default();
    let fields: Vec<&str> = stat.rfind(')').map(|i| stat[i + 2..].split_whitespace().collect()).unwrap_or_default();
    let field = |i: usize| fields.get(i).and_then(|f| f.parse::<u64>().ok()).unwrap_or(0);
    field(11) + field(12)
}
