//! Focus tracking against stand-in compositors (T-7): Sway, Hyprland and niri, each a small socket
//! server in `tests/common/standin/`. The daemon connects, a game window gets focus, and
//! `padwight status` must show the game's profile; it must go back when focus moves away.
//!
//! The daemon sees no session bus (the bus address points nowhere) and no display, and no controller
//! is needed, so these do not need /dev/input and can run in CI. They are opt-in only because the
//! daemon makes its virtual devices through /dev/uinput:
//! `cargo nextest run --profile agent --run-ignored ignored-only compositor_`.

#![cfg(target_os = "linux")]

mod common;

use std::{
    fs,
    path::{Path, PathBuf},
};

use common::{
    Daemon, assert_log_clean, install_config, serial, spawn_daemon_env, temp_root, wait_for,
    focus::{DEFAULT_LINE, GAME_LINE, GameProcess, game_config, status, wait_for_log, wait_for_status},
    standin::{Compositor, Window, hyprland::Hyprland, niri::Niri, sway::Sway},
};

/// A desktop-like window owned by this test process, which no rule matches.
fn desktop_window() -> Window {
    Window::new(1, std::process::id(), "foot", "~")
}

/// The game's window: its pid is the stand-in game process.
fn game_window(game: &GameProcess) -> Window {
    Window::new(2, game.pid(), "padwight_game", "Fake Game")
}

struct Setup {
    root: PathBuf,
    runtime: PathBuf,
}

fn setup() -> Setup {
    let root = temp_root();
    let config = root.join("config/padwight");
    fs::create_dir_all(&config).unwrap();
    install_config(&config, &game_config());
    let runtime = root.join("run");
    fs::create_dir_all(&runtime).unwrap();
    Setup { root, runtime }
}

fn start_daemon(s: &Setup, env: &[(&str, &Path)]) -> Daemon {
    let bus = s.root.join("no-bus");
    let mut env = env.to_vec();
    // No session bus: the test must not reach the desktop it runs on.
    let address = format!("unix:path={}", bus.display());
    let address = PathBuf::from(address);
    env.push(("DBUS_SESSION_BUS_ADDRESS", &address));
    spawn_daemon_env(&s.root, Some(&s.runtime), &[], &env)
}

fn stop(mut daemon: Daemon, s: &Setup) {
    // SAFETY: `kill` only sends a signal, to a child we started and have not yet waited for.
    let rc = unsafe { libc::kill(daemon.0.id() as libc::pid_t, libc::SIGTERM) };
    assert_eq!(rc, 0, "sending SIGTERM");
    wait_for("the daemon to exit", || daemon.0.try_wait().unwrap());
    assert_log_clean(&s.root);
    let _ = fs::remove_dir_all(&s.root);
}

/// The same walk for every compositor: the game is focused before the daemon starts (so the first
/// report counts), then focus moves to the desktop, back to the game, to nothing, to the game, and
/// the game closes.
fn walk(s: &Setup, compositor: &dyn Compositor, tracker: &str, daemon: &Daemon) {
    let game = GameProcess::start(&s.root);
    let _ = daemon;
    let game_window = game_window(&game);
    let desktop = desktop_window();

    compositor.focus(&desktop);
    compositor.focus(&game_window);
    wait_for_log(&s.root, &format!("focus tracking: focused window ({tracker})"));
    wait_for_status(&s.root, &s.runtime, GAME_LINE);

    compositor.focus(&desktop);
    wait_for_status(&s.root, &s.runtime, DEFAULT_LINE);
    compositor.focus(&game_window);
    wait_for_status(&s.root, &s.runtime, GAME_LINE);
    compositor.unfocus();
    wait_for_status(&s.root, &s.runtime, DEFAULT_LINE);
    compositor.focus(&game_window);
    wait_for_status(&s.root, &s.runtime, GAME_LINE);
    compositor.close(&game_window);
    wait_for_status(&s.root, &s.runtime, DEFAULT_LINE);
    assert!(status(&s.runtime).contains(&format!("focus tracking: focused window ({tracker})")));
}

#[test]
#[ignore]
fn compositor_sway_focus_switches_the_profile() {
    let _serial = serial();
    let s = setup();
    let socket = s.runtime.join("sway-ipc.test.sock");
    let sway = Sway::start(&socket);
    let daemon = start_daemon(&s, &[("SWAYSOCK", &socket)]);
    walk(&s, &sway, "Sway", &daemon);
    stop(daemon, &s);
}

#[test]
#[ignore]
fn compositor_hyprland_focus_switches_the_profile() {
    let _serial = serial();
    let s = setup();
    let hypr = Hyprland::start(&s.runtime, "padwighttest_1_1");
    let signature = PathBuf::from(hypr.signature());
    let daemon = start_daemon(&s, &[("HYPRLAND_INSTANCE_SIGNATURE", &signature)]);
    walk(&s, &hypr, "Hyprland", &daemon);
    stop(daemon, &s);
}

#[test]
#[ignore]
fn compositor_niri_focus_switches_the_profile() {
    let _serial = serial();
    let s = setup();
    let socket = s.runtime.join("niri.test.sock");
    let niri = Niri::start(&socket);
    let daemon = start_daemon(&s, &[("NIRI_SOCKET", &socket)]);
    walk(&s, &niri, "niri", &daemon);
    stop(daemon, &s);
}
