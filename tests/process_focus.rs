//! Profile switching by process, on a desktop with no focus tracker (D-partial). The daemon scans
//! the running processes for a game's rule; a stand-in game process named like the rule's
//! executable switches the profile, and its exit switches it back. The desktop trackers (KWin, Sway,
//! Hyprland) are not started here: they need a session, so they stay manual.
//!
//! Needs /dev/uinput and /dev/uhid, and read access to /dev/input, so it is opt-in:
//! `cargo test --test process_focus -- --ignored`, or `scripts/kernel-test-docker.sh process_`.

#![cfg(target_os = "linux")]

mod common;

use std::fs;

use common::{
    Daemon, assert_log_clean, install_config, serial, spawn_daemon, temp_root, wait_for,
    focus::{GAME, GAME_EXE, GAME_PROFILE, GameProcess, game_config, wait_for_log},
};

#[test]
#[ignore]
fn a_game_process_switches_the_profile_and_back() {
    let _serial = serial();
    let root = temp_root();
    let config = root.join("config/padwight");
    fs::create_dir_all(&config).unwrap();
    install_config(&config, &game_config());
    let mut daemon: Daemon = spawn_daemon(&root, Some(&root.join("run")));

    // With no session and no tracker, the daemon falls back to scanning processes.
    wait_for_log(&root, "focus tracking: running processes");

    let game = GameProcess::start(&root);
    wait_for_log(&root, &format!("which matches Executable “{GAME_EXE}” → profile {GAME} › {GAME_PROFILE}"));

    drop(game);
    wait_for_log(&root, "no game running, so the default profile takes over → profile");

    // SIGTERM is the normal stop, so the log is complete once it exits.
    // SAFETY: `kill` only sends a signal, to a child we started and have not yet waited for.
    let rc = unsafe { libc::kill(daemon.0.id() as libc::pid_t, libc::SIGTERM) };
    assert_eq!(rc, 0, "sending SIGTERM");
    wait_for("the daemon to exit", || daemon.0.try_wait().unwrap());
    assert_log_clean(&root);
    let _ = fs::remove_dir_all(&root);
}
