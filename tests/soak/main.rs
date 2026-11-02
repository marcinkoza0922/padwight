//! Soak and chaos run (QA tooling T-6): a `--debug` daemon, driven for a set time by seeded random
//! steps: input on one to four uhid controllers, hot-plug, profile switches and the Guide mode,
//! `reload`, `SetConfig`, overlays opening and closing, the overlay process killed, stopped and
//! continued, and a hook for fake focus changes. Every few minutes it lets everything go and checks
//! that nothing is held in the kernel, the log has no panic, the daemon answers, and its memory,
//! descriptors and threads are within a bound of where they started.
//!
//! The same seed gives the same steps. A time-based run differs only in how many steps fit; set
//! `PADWIGHT_SOAK_STEPS` to replay a failure to its exact step.
//!
//! Needs /dev/uhid and /dev/uinput and read access to /dev/input: `scripts/soak.sh` runs it in a
//! container. Settings (environment):
//!
//! - `PADWIGHT_SOAK_SECONDS` (default 60): how long to run.
//! - `PADWIGHT_SOAK_STEPS`: run exactly this many steps instead.
//! - `PADWIGHT_SOAK_SEED`: a number to replay; by default one is made from the clock and printed.
//! - `PADWIGHT_SOAK_CHECK_SECONDS` (default 120): time between invariant checks.
//! - `PADWIGHT_SOAK_CHECK_STEPS`: check every this many steps instead (for step-count runs).
//! - `PADWIGHT_SOAK_PADS` (default 4): most controllers plugged at once, 1 to 4.
//! - `PADWIGHT_SOAK_RSS_KB`, `PADWIGHT_SOAK_FDS`, `PADWIGHT_SOAK_THREADS`: how far above the start
//!   values the daemon may grow (defaults 40000, 8, 6).
//! - `PADWIGHT_SOAK_FOCUS_HOOK`: a command run as a step with a window class from
//!   `PADWIGHT_SOAK_FOCUS_CLASSES` (comma separated; default `game-a,game-b,none`) as its argument.
//!   T-7's stand-in compositors plug in here.
//! - `PADWIGHT_SOAK_OUT`: where `soak.csv`, `steps.log` and the daemon log are kept.

#![cfg(target_os = "linux")]
#![expect(clippy::print_stderr, reason = "the run reports its progress on stderr")]

#[path = "../common/mod.rs"]
mod common;

mod probe;
mod steps;

use std::{
    fs::{self, File},
    io::Write,
    path::PathBuf,
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

use common::{input::assert_no_other_daemon, install_config, serial, spawn_daemon_with, temp_root};
use probe::{Bounds, Sample, check_invariants};
use steps::{Rng, World};

fn env_u64(name: &str) -> Option<u64> {
    std::env::var(name).ok()?.parse().ok()
}

/// The soak config: the default one with the Guide mode on North, so a held North is a mode
/// switch, a key on West and the mouse on East, and the pad's other buttons passed through.
fn soak_config() -> String {
    let text = common::default_config();
    let text = text.replacen("[general.profiles.buttons.North]\ngamepad = \"North\"", "[general.profiles.buttons.North]\nmode = \"Guide\"", 1);
    let text = text.replacen("[general.profiles.buttons.West]\ngamepad = \"West\"", "[general.profiles.buttons.West]\nkeys = [\"KEY_B\"]", 1);
    let text = text.replacen("[general.profiles.buttons.East]\ngamepad = \"East\"", "[general.profiles.buttons.East]\nmouse = \"Left\"", 1);
    assert!(text.contains("mode = \"Guide\"") && text.contains("KEY_B") && text.contains("mouse = \"Left\""), "the default config has North, West and East as passthrough");
    text
}

#[test]
#[ignore]
fn soak_run() {
    let _serial = serial();
    assert_no_other_daemon();
    let seed = env_u64("PADWIGHT_SOAK_SEED").unwrap_or_else(|| SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos() as u64 & 0xffff_ffff);
    let seconds = env_u64("PADWIGHT_SOAK_SECONDS").unwrap_or(60);
    let max_steps = env_u64("PADWIGHT_SOAK_STEPS");
    let check_every = Duration::from_secs(env_u64("PADWIGHT_SOAK_CHECK_SECONDS").unwrap_or(120));
    let check_steps = env_u64("PADWIGHT_SOAK_CHECK_STEPS");
    let pads = env_u64("PADWIGHT_SOAK_PADS").unwrap_or(4).clamp(1, 4) as usize;
    let bounds = Bounds {
        rss_kb: env_u64("PADWIGHT_SOAK_RSS_KB").unwrap_or(40_000),
        fds: env_u64("PADWIGHT_SOAK_FDS").unwrap_or(8),
        threads: env_u64("PADWIGHT_SOAK_THREADS").unwrap_or(6),
    };
    let out: PathBuf = std::env::var_os("PADWIGHT_SOAK_OUT").map_or_else(|| std::env::temp_dir().join("padwight-soak"), PathBuf::from);
    fs::create_dir_all(&out).unwrap();
    let length = max_steps.map_or(format!("{seconds} s"), |n| format!("{n} steps"));
    let every = check_steps.map_or(format!("{} s", check_every.as_secs()), |n| format!("{n} steps"));
    eprintln!("soak: seed {seed}, {length}, check every {every}, up to {pads} pads");

    let root = temp_root();
    let config = root.join("config/padwight");
    fs::create_dir_all(&config).unwrap();
    install_config(&config, &soak_config());
    let runtime = root.join("run");
    let mut daemon = spawn_daemon_with(&root, Some(&runtime), &["--debug"]);
    let _ = fs::set_permissions(&runtime, std::os::unix::fs::PermissionsExt::from_mode(0o700));
    let pid = daemon.0.id();
    common::wait_for("the daemon's socket", || probe::request(&runtime, "\"Status\"").map(|_| ()));

    let mut world = World::new(root.clone(), runtime, pid, pads);
    let mut rng = Rng::new(seed);
    let mut steps_log = File::create(out.join("steps.log")).unwrap();
    let mut csv = File::create(out.join("soak.csv")).unwrap();
    writeln!(csv, "elapsed_s,steps,rss_kb,fds,threads,overlay_processes").unwrap();

    // One pad plugged and settled, so the start values include a working controller.
    world.plug(0);
    std::thread::sleep(Duration::from_secs(3));
    let start = Sample::take(pid).expect("the daemon is running");
    eprintln!("soak: start rss {} kB, {} fds, {} threads", start.rss_kb, start.fds, start.threads);
    writeln!(csv, "0,0,{},{},{},0", start.rss_kb, start.fds, start.threads).unwrap();

    let began = Instant::now();
    let mut last_check = Instant::now();
    let mut count = 0u64;
    let failure = loop {
        let done = max_steps.map_or_else(|| began.elapsed() >= Duration::from_secs(seconds), |n| count >= n);
        let due = check_steps.map_or_else(|| last_check.elapsed() >= check_every, |n| count > 0 && count % n == 0);
        if done || due {
            match check_invariants(&mut world, &mut daemon.0, start, bounds) {
                Ok(s) => {
                    eprintln!("soak: check at {} s, step {count}: ok (rss {} kB, {} fds, {} threads, {} overlay processes)", began.elapsed().as_secs(), s.rss_kb, s.fds, s.threads, s.overlays);
                    writeln!(csv, "{},{count},{},{},{},{}", began.elapsed().as_secs(), s.rss_kb, s.fds, s.threads, s.overlays).unwrap();
                }
                Err(why) => break Some(why),
            }
            last_check = Instant::now();
            if done {
                break None;
            }
        }
        let step = world.step(&mut rng);
        writeln!(steps_log, "{count} {step}").unwrap();
        count += 1;
    };
    world.finish();
    let log_copy = out.join("daemon.log");
    let _ = fs::copy(root.join("daemon.log"), &log_copy);
    let _ = daemon.0.kill();
    let _ = daemon.0.wait();
    if let Some(why) = failure {
        eprintln!("soak: FAILED after {count} steps: {why}\nsoak: replay with PADWIGHT_SOAK_SEED={seed} PADWIGHT_SOAK_STEPS={count}; the daemon log is {}", log_copy.display());
        panic!("soak invariant broken: {why}");
    }
    eprintln!("soak: passed, {count} steps in {} s, seed {seed}", began.elapsed().as_secs());
    let _ = fs::remove_dir_all(&root);
}
