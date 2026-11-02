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
    focus::{DEFAULT_LINE, GAME_LINE, GameProcess, cli, game_config, status, wait_for_log, wait_for_status},
    standin::{Compositor, cosmic::Cosmic, wlroots::{OutputSpec, Wlroots}, bus::SessionBus, cinnamon::Cinnamon, kwin::Kwin, Window, hyprland::Hyprland, niri::Niri, sway::Sway},
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

/// The daemon, with `bus` as its session bus; without one it gets a dead address, so the test
/// never reaches the desktop it runs on.
fn start_daemon_on(s: &Setup, env: &[(&str, &Path)], bus: Option<&SessionBus>) -> Daemon {
    let address = PathBuf::from(bus.map_or_else(|| format!("unix:path={}", s.root.join("no-bus").display()), |b| b.address.clone()));
    let mut env = env.to_vec();
    env.push(("DBUS_SESSION_BUS_ADDRESS", &address));
    spawn_daemon_env(&s.root, Some(&s.runtime), &[], &env)
}

fn start_daemon(s: &Setup, env: &[(&str, &Path)]) -> Daemon {
    start_daemon_on(s, env, None)
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

#[test]
#[ignore]
fn compositor_kwin_focus_switches_the_profile() {
    let _serial = serial();
    let s = setup();
    let bus = SessionBus::start(&s.runtime.join("bus"));
    let kwin = Kwin::start(&bus);
    let daemon = start_daemon_on(&s, &[], Some(&bus));
    walk(&s, &kwin, "KWin", &daemon);
    assert!(kwin.script_running());
    assert_eq!(kwin.loads(), 1, "the daemon loads its script once");
    stop(daemon, &s);
}

#[test]
#[ignore]
fn compositor_cinnamon_focus_switches_the_profile() {
    let _serial = serial();
    let s = setup();
    let bus = SessionBus::start(&s.runtime.join("bus"));
    let cinnamon = Cinnamon::start(&bus, &s.root.join("cin"), &s.root.join("data"), true);
    let path = std::env::var("PATH").unwrap_or_default();
    let path = PathBuf::from(format!("{}:{path}", cinnamon.bin_dir().display()));
    let daemon = start_daemon_on(&s, &[("PATH", &path)], Some(&bus));
    walk(&s, &cinnamon, "Cinnamon", &daemon);
    stop(daemon, &s);
}

/// The protocol has no pid, so the rule matches the window class the game's window carries.
#[test]
#[ignore]
fn compositor_wlroots_focus_switches_the_profile() {
    let _serial = serial();
    let s = setup();
    let config = s.root.join("config/padwight");
    let class_rule = game_config().replace("kind = \"executable\"\nvalue = \"padwight-fake-game\"", "kind = \"window_class\"\nvalue = \"padwight_game\"");
    assert!(class_rule.contains("window_class"), "the game's rule became a class rule");
    fs::remove_dir_all(&config).unwrap();
    install_config(&config, &class_rule);
    let outputs = [
        OutputSpec { x: 0, y: 0, width: 1280, height: 800, scale: 1 },
        OutputSpec { x: 1280, y: 0, width: 1024, height: 768, scale: 1 },
    ];
    let wl = Wlroots::start(&s.runtime.join("wayland-test"), &outputs);
    let name = PathBuf::from("wayland-test");
    let daemon = start_daemon(&s, &[("WAYLAND_DISPLAY", &name)]);
    walk(&s, &wl, "wlroots", &daemon);
    stop(daemon, &s);
}

/// The overlay's surface is made on the output of the focused window (`overlay/wayland/monitors.rs`),
/// whichever of the two monitors that is. Each output gets its own daemon: in the stand-in the
/// overlay does not hide again after it closes, so one surface per run is what can be checked.
#[test]
#[ignore]
fn compositor_wlroots_overlay_follows_the_focused_windows_output() {
    let _serial = serial();
    for output in [1, 0] {
        let s = setup();
        let outputs = [
            OutputSpec { x: 0, y: 0, width: 1280, height: 800, scale: 1 },
            OutputSpec { x: 1280, y: 0, width: 1024, height: 768, scale: 1 },
        ];
        let wl = Wlroots::start(&s.runtime.join("wayland-test"), &outputs);
        wl.focus_on(&Window::new(1, std::process::id(), "foot", "~"), output);
        let name = PathBuf::from("wayland-test");
        let daemon = start_daemon(&s, &[("WAYLAND_DISPLAY", &name)]);
        wait_for_log(&s.root, "focus tracking: focused window (wlroots)");
        // Opens the keyboard overlay: the overlay process starts, and makes its surface again
        // for the first frame, now that it knows which window has focus.
        cli(&s.runtime, &["overlay-toggle"]);
        wait_for("an overlay surface made for an output", || wl.layer_surfaces().iter().any(|l| l.output.is_some()).then_some(()));
        assert_eq!(wl.layer_surfaces().last().unwrap().output, Some(output), "the overlay is on the focused window's output");
        stop(daemon, &s);
    }
}

/// The GNOME and Cinnamon overlay hands the extension a picture of the monitor's size at its
/// scale, and presents it.
#[test]
#[ignore]
fn compositor_cinnamon_overlay_hands_over_a_picture_of_the_monitors_size() {
    use common::standin::overlay::Call;

    let _serial = serial();
    let s = setup();
    let bus = SessionBus::start(&s.runtime.join("bus"));
    let cinnamon = Cinnamon::start(&bus, &s.root.join("cin"), &s.root.join("data"), true);
    cinnamon.overlay().set_monitor(1280, 720, 1.5);
    let path = std::env::var("PATH").unwrap_or_default();
    let path = PathBuf::from(format!("{}:{path}", cinnamon.bin_dir().display()));
    let daemon = start_daemon_on(&s, &[("PATH", &path)], Some(&bus));
    wait_for_log(&s.root, "focus tracking: focused window (Cinnamon)");
    cli(&s.runtime, &["overlay-toggle"]);
    wait_for("the overlay to present a frame", || cinnamon.overlay().calls().contains(&Call::Present).then_some(()));
    let calls = cinnamon.overlay().calls();
    let buffer = calls.iter().find_map(|c| match c {
        Call::SetBuffer { width, height, bytes } => Some((*width, *height, *bytes)),
        _ => None,
    });
    let (width, height, bytes) = buffer.unwrap_or_else(|| panic!("no SetBuffer in {calls:?}"));
    assert_eq!((width, height), (1920, 1080), "1280x720 logical at scale 1.5 is 1920x1080 device pixels");
    assert!(bytes >= 1920 * 1080 * 4, "the memfd holds the picture: {bytes} bytes");
    let set = calls.iter().position(|c| matches!(c, Call::SetBuffer { .. })).unwrap();
    let present = calls.iter().position(|c| *c == Call::Present).unwrap();
    assert!(set < present, "the buffer is set before the first present: {calls:?}");
    stop(daemon, &s);
}

/// COSMIC has no pid either: the rule matches the window class.
#[test]
#[ignore]
fn compositor_cosmic_focus_switches_the_profile() {
    let _serial = serial();
    let s = setup();
    let config = s.root.join("config/padwight");
    let class_rule = game_config().replace("kind = \"executable\"\nvalue = \"padwight-fake-game\"", "kind = \"window_class\"\nvalue = \"padwight_game\"");
    assert!(class_rule.contains("window_class"), "the game's rule became a class rule");
    fs::remove_dir_all(&config).unwrap();
    install_config(&config, &class_rule);
    let cosmic = Cosmic::start(&s.runtime.join("wayland-test"));
    let name = PathBuf::from("wayland-test");
    let daemon = start_daemon(&s, &[("WAYLAND_DISPLAY", &name)]);
    walk(&s, &cosmic, "COSMIC", &daemon);
    stop(daemon, &s);
}

/// FOC-3: Budgie's power dialog answers as `org.gnome.Shell` on the bus, and the desktop may call
/// itself `Budgie:GNOME`. The daemon must still use the wlroots tracker (a class rule matches), put
/// no GNOME extension in place, and log no `overlay:` error when the overlay opens.
#[test]
#[ignore]
fn compositor_budgie_uses_the_wlroots_tracker_and_installs_no_gnome_extension() {
    let _serial = serial();
    let s = setup();
    let config = s.root.join("config/padwight");
    let class_rule = game_config().replace("kind = \"executable\"\nvalue = \"padwight-fake-game\"", "kind = \"window_class\"\nvalue = \"padwight_game\"");
    fs::remove_dir_all(&config).unwrap();
    install_config(&config, &class_rule);
    let bus = SessionBus::start(&s.runtime.join("bus"));
    let _power_dialog = bus.connect(Some("org.gnome.Shell"));
    let wl = Wlroots::start(&s.runtime.join("wayland-test"), &[OutputSpec { x: 0, y: 0, width: 1280, height: 800, scale: 1 }]);
    let name = PathBuf::from("wayland-test");
    let desktop = PathBuf::from("Budgie:GNOME");
    let daemon = start_daemon_on(&s, &[("WAYLAND_DISPLAY", &name), ("XDG_CURRENT_DESKTOP", &desktop)], Some(&bus));
    walk(&s, &wl, "wlroots", &daemon);
    cli(&s.runtime, &["overlay-toggle"]);
    wait_for("an overlay surface made", || (!wl.layer_surfaces().is_empty()).then_some(()));
    assert!(!s.root.join("data/gnome-shell").exists(), "no GNOME extension was installed");
    assert!(!common::daemon_log(&s.root).contains("overlay:"), "no overlay error:\n{}", common::daemon_log(&s.root));
    stop(daemon, &s);
}
