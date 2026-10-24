//! Profiling only: what the engine costs per input, for every profile of every library pack, and
//! how much memory the parsed library holds. Opt-in, and run optimized:
//! `cargo test --release engine::bench -- --ignored --nocapture`.

#![expect(clippy::print_stderr, reason = "the figures are what these print")]

use std::time::{Duration, Instant};

use super::*;
use crate::input::MotionSample;

/// Inputs per profile: a stick swept round, the triggers pulled, a button pressed and let go,
/// and gyro samples, as a controller in play sends them.
const ROUNDS: usize = 2000;

/// The inputs one round sends, for a sweep position `t` (0..1).
fn round(t: f32) -> [InputEvent; 6] {
    let (sin, cos) = (t * std::f32::consts::TAU).sin_cos();
    [
        InputEvent::Axis(Axis::LeftX, cos * 0.8),
        InputEvent::Axis(Axis::LeftY, sin * 0.8),
        InputEvent::Axis(Axis::RightX, sin * 0.5),
        InputEvent::Axis(Axis::RightTrigger, (t * 2.0).fract()),
        InputEvent::Button(Button::South, t < 0.5),
        InputEvent::Button(Button::DpadUp, t > 0.25 && t < 0.75),
    ]
}

/// Nanoseconds per input (and per tick and gyro sample with them) for `profile`.
fn per_input(profile: &Profile) -> f64 {
    let mut engine = Engine::default();
    let mut out = Vec::new();
    let now = Instant::now();
    let sample = MotionSample { gyro: [20.0, -5.0, 3.0], accel: [0.0, 1.0, 0.0], dt: 0.004 };
    let started = Instant::now();
    for i in 0..ROUNDS {
        let t = i as f32 / ROUNDS as f32 * 7.0 % 1.0;
        let at = now + Duration::from_millis(i as u64 * 4);
        for ev in round(t) {
            engine.handle(profile, ev, at, &mut out);
        }
        engine.motion(profile, sample, &mut out);
        engine.tick(profile, 0.004, &mut out);
        engine.timers(profile, at, &mut out);
        out.clear();
    }
    started.elapsed().as_nanos() as f64 / (ROUNDS * 9) as f64
}

#[test]
#[ignore]
fn engine_cost_per_input_for_every_library_profile() {
    let entries = crate::library::entries();
    let profiles: Vec<(&Profile, String)> = entries
        .iter()
        .flat_map(|e| e.pack.profiles.iter().map(move |p| (p, format!("{} / {}", e.pack.pack.name, p.name))))
        .collect();
    // Twice, so the second time every code path is warm.
    profiles.iter().for_each(|(p, _)| _ = std::hint::black_box(per_input(p)));
    let mut costs: Vec<(f64, &str)> = profiles.iter().map(|(p, name)| (per_input(p), name.as_str())).collect();
    costs.sort_by(|a, b| a.0.total_cmp(&b.0));
    let at = |p: f64| costs[((costs.len() - 1) as f64 * p) as usize].0;
    eprintln!("engine, ns per input over {} profiles: p50 {:.0}, p90 {:.0}, max {:.0} ({})", costs.len(), at(0.5), at(0.9), at(1.0), costs.last().unwrap().1);
    eprintln!("passthrough: {:.0} ns per input", per_input(&Profile::passthrough("p")));
}

/// Resident memory of this process, in kB. Run its test on its own, before anything parses the
/// library: `cargo test --release memory_the_parsed -- --ignored --nocapture`.
#[cfg(target_os = "linux")]
fn rss_kb() -> (i64, i64) {
    let status = std::fs::read_to_string("/proc/self/status").unwrap_or_default();
    let field = |name: &str| status.lines().find_map(|l| l.strip_prefix(name)).and_then(|v| v.trim().trim_end_matches(" kB").parse().ok()).unwrap_or(0);
    (field("RssAnon:"), field("RssFile:"))
}

/// What grew since `before`, as `anon +N kB, file-backed +M kB` (file-backed pages are the binary's
/// own, clean and shared, which the system can take back at any time).
#[cfg(target_os = "linux")]
fn grown(before: (i64, i64)) -> String {
    let now = rss_kb();
    format!("anonymous {:+} kB, file-backed {:+} kB", now.0 - before.0, now.1 - before.1)
}

#[cfg(target_os = "linux")]
#[test]
#[ignore]
fn memory_the_parsed_library_holds() {
    let before = rss_kb();
    let started = Instant::now();
    let packs = crate::library::listings().len();
    eprintln!("library listings (what the daemon and settings window keep): {packs} packs in {:?}, resident memory {}", started.elapsed(), grown(before));
    let (before, started) = (rss_kb(), Instant::now());
    let all = crate::library::entries();
    eprintln!("every pack parsed: {:?}, resident memory {}", started.elapsed(), grown(before));
    drop(all);
}

/// The settings window compares its config with the saved one every second: what that costs
/// with every library pack added as a setup.
#[test]
#[ignore]
fn comparing_a_big_config() {
    let mut config = crate::config::Config::default();
    for entry in crate::library::entries() {
        let plan = crate::pack::plan(&config, entry.pack.clone(), true);
        crate::pack::apply(&mut config, &plan, &crate::pack::Choices::default());
    }
    let saved = config.clone();
    let started = Instant::now();
    let rounds = 100;
    for _ in 0..rounds {
        assert!(std::hint::black_box(&config) == std::hint::black_box(&saved));
    }
    eprintln!("config with {} setups: comparing takes {:?}", config.games.len(), started.elapsed() / rounds);
}
