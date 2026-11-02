//! Force Quit on a desktop without focus tracking (FOC-9, code review H-17). The daemon scans the
//! processes, so the "focused" game is the running process of the active game's rule. Holding the
//! button for two seconds ends that game; holding it again once the game is gone finds nothing to
//! end and says so. The pad is made with uhid, and South is Force Quit in every profile.
//!
//! The desktop-side half (the focused window's pid, a game in a Flatpak sandbox) is covered by
//! `scripts/sandbox-pid.sh` and `padwight debug focus`; see `tests/launcher_names.rs`.
//!
//! Needs /dev/uinput and /dev/uhid, and read access to /dev/input, so it is opt-in:
//! `cargo test --test force_quit -- --ignored`, or `scripts/kernel-test-docker.sh force_quit`.

#![cfg(target_os = "linux")]

mod common;

use std::{fs, thread, time::Duration};

use common::{
    assert_log_clean, install_config, serial, spawn_daemon, temp_root, wait_for,
    focus::{GAME, GAME_PROFILE, GameProcess, game_config, wait_for_log},
    input::{assert_no_other_daemon, find_node, grabbed},
    uhid::{BUTTON_SOUTH, HidPad},
};

const NAME: &str = "Test Controller";
/// Longer than the daemon's two-second hold.
const HOLD: Duration = Duration::from_millis(2600);

/// The game config with South as Force Quit in every profile, so it still is after the game's
/// profile gives way to the default one.
fn force_quit_config() -> String {
    let mut table: toml::Table = toml::from_str(&game_config()).unwrap();
    let mut count = 0;
    let mut bind = |profiles: &mut toml::Value| {
        for profile in profiles.as_array_mut().into_iter().flatten().filter_map(toml::Value::as_table_mut) {
            profile["buttons"].as_table_mut().unwrap().insert("South".into(), toml::Value::String("force_quit".into()));
            count += 1;
        }
    };
    bind(&mut table["general"]["profiles"]);
    for setup in table["setups"].as_array_mut().unwrap() {
        bind(&mut setup["profiles"]);
    }
    assert!(count >= 2, "the General profiles and the game's");
    table.to_string()
}

fn hold(pad: &mut HidPad) {
    // The kernel drops reports until the daemon has the device open, so send the state repeatedly.
    let start = std::time::Instant::now();
    while start.elapsed() < HOLD {
        // Nothing but the same state, as a held button on a pad with a still stick sends.
        pad.report(BUTTON_SOUTH, 0, 0);
        thread::sleep(Duration::from_millis(100));
    }
    pad.report(0, 0, 0);
}

#[test]
#[ignore]
fn force_quit_ends_the_scanned_game_then_finds_nothing() {
    let _serial = serial();
    assert_no_other_daemon();
    let root = temp_root();
    let config = root.join("config/padwight");
    fs::create_dir_all(&config).unwrap();
    install_config(&config, &force_quit_config());

    let mut pad = HidPad::new(NAME);
    let node = wait_for("the controller's event node", || find_node(NAME));
    let mut daemon = spawn_daemon(&root, Some(&root.join("run")));
    wait_for("the daemon to grab the controller", || grabbed(&node).then_some(()));
    wait_for_log(&root, "focus tracking: running processes");

    let mut game = GameProcess::start(&root);
    let pid = game.pid();
    wait_for_log(&root, &format!("→ profile {GAME} › {GAME_PROFILE}"));

    hold(&mut pad);
    wait_for_log(&root, &format!("force quitting {GAME} (pid {pid})"));
    wait_for("the game to end", || game.0.try_wait().unwrap());
    wait_for_log(&root, "no game running, so the default profile takes over");

    // Just after it closed: nothing is left to end, and nothing else is touched.
    hold(&mut pad);
    wait_for_log(&root, "force quit held, but there is nothing to force quit");
    assert_eq!(common::daemon_log(&root).matches("force quitting").count(), 1, "only the game was ended");

    // SAFETY: `kill` only sends a signal, to a child we started and have not yet waited for.
    let rc = unsafe { libc::kill(daemon.0.id() as libc::pid_t, libc::SIGTERM) };
    assert_eq!(rc, 0, "sending SIGTERM");
    wait_for("the daemon to exit", || daemon.0.try_wait().unwrap());
    assert_log_clean(&root);
    let _ = fs::remove_dir_all(&root);
}
