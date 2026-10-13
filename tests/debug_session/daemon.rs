//! The tests that need a daemon started with `--debug`, and so /dev/uinput: Linux only, and
//! opt-in (see the top of `main.rs`).

use std::{fs, os::unix::fs::PermissionsExt, process::Command, thread, time::Duration};

use serde_json::Value;

use super::{BIN, Env};
use crate::common::{Daemon, hold_config, install_config, input::{Reader, nodes}, serial, spawn_daemon, spawn_daemon_with, wait_for};

/// A daemon started with `--debug`, running `config`, and ready for requests.
fn debug_daemon(env: &Env, config: &str) -> Daemon {
    let dir = env.root.join("config/padwight");
    fs::create_dir_all(&dir).unwrap();
    install_config(&dir, config);
    let daemon = spawn_daemon_with(&env.root, Some(&env.run()), &["--debug"]);
    wait_for("the daemon to answer", || Command::new(BIN).arg("status").env("XDG_RUNTIME_DIR", env.run()).output().ok()?.status.success().then_some(()));
    daemon
}

/// [`debug_daemon`], and a reader on its virtual keyboard: not on the keyboard of a daemon
/// already running on the machine, whose event number can be higher or lower.
fn debug_daemon_and_keyboard(env: &Env, config: &str) -> (Daemon, Reader) {
    const KEYBOARD: &str = "Padwight Virtual Keyboard";
    let before = nodes(KEYBOARD);
    let daemon = debug_daemon(env, config);
    (daemon, Reader::open_new(KEYBOARD, &before))
}

fn status(env: &Env) -> String {
    let out = env.command(&["status"]).output().unwrap();
    String::from_utf8_lossy(&out.stdout).into_owned()
}

/// The config with the active profile's gyro turning the mouse.
fn gyro_mouse_config() -> String {
    let text = hold_config().replacen("[general.profiles.gyro]\nmode = \"off\"", "[general.profiles.gyro]\nmode = { mouse = { sensitivity = 2.0 } }", 1);
    assert!(text.contains("sensitivity = 2.0"), "the default config's gyro is off");
    text
}

#[test]
#[ignore]
fn debug_a_normal_daemon_refuses_injected_controllers() {
    let _serial = serial();
    let env = Env::new();
    let _daemon = spawn_daemon(&env.root, Some(&env.run()));
    wait_for("the daemon to answer", || env.command(&["status"]).output().ok()?.status.success().then_some(()));
    let reply = Env::reply(&env.command(&["debug", "--headless", "--attach"]).output().unwrap());
    assert!(!reply.ok && reply.err.contains("--debug"), "{}", reply.err);
}

#[test]
#[ignore]
fn debug_identify_needs_a_debug_daemon() {
    let _serial = serial();
    let env = Env::new();
    let _daemon = spawn_daemon(&env.root, Some(&env.run()));
    wait_for("the daemon to answer", || env.command(&["status"]).output().ok()?.status.success().then_some(()));
    let _session = env.session(&[]);
    let reply = env.debug(&["identify", "dualsense"]);
    assert!(!reply.ok && reply.err.contains("--debug"), "{}", reply.err);
}

#[test]
#[ignore]
fn debug_identify_is_accepted_and_clears_back_to_choosing() {
    let _serial = serial();
    let env = Env::new();
    let _daemon = debug_daemon(&env, &hold_config());
    let _session = env.session(&[]);
    env.ok(&["identify", "dualsense"]);
    env.ok(&["identify", "auto"]);
}

#[test]
#[ignore]
fn debug_an_injected_controller_runs_through_the_active_profile() {
    let _serial = serial();
    let env = Env::new();
    let _daemon = debug_daemon(&env, &hold_config());
    let mut session = env.session(&["--attach", "--model", "dualsense"]);
    assert!(status(&env).contains("Debug controller (dualsense) [debug:0] remapping"), "{}", status(&env));

    // South is mapped to KEY_A, East to the left mouse button.
    env.ok(&["expect", "no-output"]);
    env.ok(&["press", "south"]);
    env.ok(&["expect", "output", "key KEY_A down"]);
    env.ok(&["release", "south"]);
    env.ok(&["expect", "output", "key KEY_A up"]);
    env.ok(&["output", "clear"]);
    env.ok(&["tap", "east", "30"]);
    env.ok(&["expect", "output", "mouse Left down"]);
    env.ok(&["expect", "output", "mouse Left up"]);

    // A button the profile passes through comes out as the virtual pad's.
    env.ok(&["output", "clear"]);
    env.ok(&["press", "north"]);
    env.ok(&["expect", "output", "pad North down"]);
    env.ok(&["release", "north"]);

    env.ok(&["quit"]);
    assert!(wait_for("the session to exit", || session.0.try_wait().unwrap()).success());
    assert!(!status(&env).contains("Debug controller"), "{}", status(&env));
}

#[test]
#[ignore]
fn debug_output_stays_off_the_virtual_keyboard_without_live() {
    let _serial = serial();
    let env = Env::new();
    let (_daemon, mut keyboard) = debug_daemon_and_keyboard(&env, &hold_config());
    let _session = env.session(&["--attach"]);
    env.ok(&["press", "south"]);
    env.ok(&["expect", "output", "key KEY_A down"]);
    thread::sleep(Duration::from_millis(200));
    keyboard.drain();
    assert!(keyboard.pressed.is_empty(), "the key was typed for real");
    env.ok(&["release", "south"]);
}

#[test]
#[ignore]
fn debug_sticks_and_triggers_reach_the_mappings() {
    let _serial = serial();
    let env = Env::new();
    let _daemon = debug_daemon(&env, &hold_config());
    let _session = env.session(&["--attach"]);
    env.ok(&["stick", "left", "1", "0"]);
    env.ok(&["expect", "output", "pad-axis LeftX 1.00"]);
    env.ok(&["stick", "left", "0", "0"]);
    env.ok(&["expect", "output", "pad-axis LeftX 0.00"]);
    env.ok(&["trigger", "right", "0.50"]);
    env.ok(&["expect", "output", "pad-axis RightTrigger 0.50"]);
}

#[test]
#[ignore]
fn debug_changing_the_model_keeps_what_is_held() {
    let _serial = serial();
    let env = Env::new();
    let _daemon = debug_daemon(&env, &hold_config());
    let _session = env.session(&["--attach", "--model", "xbox360"]);
    env.ok(&["press", "south"]);
    env.ok(&["model", "dualsense"]);
    assert!(status(&env).contains("Debug controller (dualsense) [debug:1]"), "{}", status(&env));
    assert!(!status(&env).contains("(xbox360)"), "the first controller is gone");
    env.ok(&["expect", "pressed", "south"]);
    env.ok(&["expect", "output", "key KEY_A down"]);
    env.ok(&["release", "south"]);
    env.ok(&["expect", "output", "key KEY_A up"]);
}

#[test]
#[ignore]
fn debug_gyro_turns_the_mouse() {
    let _serial = serial();
    let env = Env::new();
    let _daemon = debug_daemon(&env, &gyro_mouse_config());
    let _session = env.session(&["--attach"]);
    env.ok(&["gyro", "0", "90", "0", "200"]);
    let moves = env.ok(&["output"]);
    let moved: i32 = moves.lines().filter_map(|l| l.strip_prefix("mouse-move ")?.split(' ').next()?.parse::<i32>().ok()).sum();
    assert!(moved != 0, "turning at 90 degrees a second for 200 ms moved the mouse by nothing:\n{moves}");
    assert_eq!(env.state()["gyro"], Value::Null, "the pad stops turning afterwards");
}

/// The config with the active profile's touchpad split: a grid menu of A to D on the left half,
/// and swipes on the right (right is D, the click R).
fn touchpad_config() -> String {
    let touchpad = "[general.profiles.touchpad]\nclick = \"disabled\"\n\
        [general.profiles.touchpad.motion.menu]\nmenu = \"Pad\"\n\
        [general.profiles.touchpad.right]\nclick = { keys = [\"KEY_R\"] }\n\
        [general.profiles.touchpad.right.motion.swipe]\nright = { keys = [\"KEY_D\"] }\n\n";
    let text = hold_config().replacen("[general.profiles.gyro]", &format!("{touchpad}[general.profiles.gyro]"), 1);
    let shared = "[shared]\nmacros = []\nmenus = []";
    assert!(text.contains(shared), "the default config has no shared menus");
    let mut text = text.replacen(shared, "[shared]\nmacros = []", 1);
    text.push_str("\n[[shared.menus]]\nname = \"Pad\"\nkind = { grid = { columns = 2 } }\n");
    for k in ["A", "B", "C", "D"] {
        text.push_str(&format!("[[shared.menus.items]]\nlabel = \"{k}\"\naction = {{ keys = [\"KEY_{k}\"] }}\n"));
    }
    text
}

#[test]
#[ignore]
fn debug_touchpad_menus_and_swipes_reach_the_mappings() {
    let _serial = serial();
    let env = Env::new();
    let _daemon = debug_daemon(&env, &touchpad_config());
    let _session = env.session(&["--attach", "--model", "dualsense"]);
    // Top right of the left half is the grid's second item; the click holds it.
    env.ok(&["touch", "0.4", "0.1"]);
    env.ok(&["pad-click", "down"]);
    env.ok(&["expect", "output", "key KEY_B down"]);
    env.ok(&["touch", "0.1", "0.9"]);
    env.ok(&["pad-click", "up"]);
    env.ok(&["expect", "output", "key KEY_B up"]);
    env.ok(&["lift"]);
    env.ok(&["output", "clear"]);
    // A quick stroke right across the right half.
    env.ok(&["swipe", "0.6", "0.5", "0.95", "0.5", "120"]);
    env.ok(&["expect", "output", "key KEY_D down"]);
    env.ok(&["expect", "output", "key KEY_D up"]);
    // The right half's click, with a finger there.
    env.ok(&["touch", "0.8", "0.5"]);
    env.ok(&["pad-click", "30"]);
    env.ok(&["expect", "output", "key KEY_R down"]);
    env.ok(&["lift"]);
}

#[test]
#[ignore]
fn debug_live_output_reaches_the_virtual_keyboard() {
    let _serial = serial();
    let env = Env::new();
    let (_daemon, mut keyboard) = debug_daemon_and_keyboard(&env, &hold_config());
    let _session = env.session(&["--attach", "--live"]);
    env.ok(&["press", "south"]);
    wait_for("KEY_A to be down on the virtual keyboard", || {
        keyboard.drain();
        keyboard.pressed.contains(&evdev::KeyCode::KEY_A.code()).then_some(())
    });
    env.ok(&["release", "south"]);
    wait_for("KEY_A to come up again", || {
        keyboard.drain();
        keyboard.pressed.is_empty().then_some(())
    });
}

#[test]
#[ignore]
fn debug_detaching_a_controller_lets_go_of_what_it_held() {
    let _serial = serial();
    let env = Env::new();
    let (_daemon, mut keyboard) = debug_daemon_and_keyboard(&env, &hold_config());
    let mut session = env.session(&["--attach", "--live"]);
    env.ok(&["press", "south"]);
    wait_for("KEY_A down", || {
        keyboard.drain();
        keyboard.pressed.contains(&evdev::KeyCode::KEY_A.code()).then_some(())
    });
    // Killed without a word, the session leaves its controller (and the held key) behind.
    session.0.kill().unwrap();
    session.0.wait().unwrap();
    assert!(status(&env).contains("Debug controller"));
    let cleaned = Env::reply(&env.command(&["debug", "detach-all"]).output().unwrap());
    assert!(cleaned.ok, "{}", cleaned.err);
    wait_for("the key to be released", || {
        keyboard.drain();
        keyboard.pressed.is_empty().then_some(())
    });
    assert!(!status(&env).contains("Debug controller"));
}

#[test]
#[ignore]
fn debug_captures_go_to_a_private_folder_in_the_temp_directory() {
    let _serial = serial();
    let env = Env::new();
    let _daemon = debug_daemon(&env, &hold_config());
    // SAFETY: getuid cannot fail.
    let uid = unsafe { libc::getuid() };
    // The daemon's temp directory is `root/tmp` (see `spawn_daemon_with`).
    let dir = env.root.join("tmp").join(format!("padwight-debug-{uid}"));
    let meta = fs::metadata(&dir).unwrap_or_else(|e| panic!("{}: {e}", dir.display()));
    assert!(meta.is_dir());
    assert_eq!(meta.permissions().mode() & 0o077, 0, "only this user can read it");
}

#[test]
#[ignore]
fn debug_ctrl_c_detaches_the_controller_and_lets_go_of_what_it_held() {
    let _serial = serial();
    let env = Env::new();
    let (_daemon, mut keyboard) = debug_daemon_and_keyboard(&env, &hold_config());
    let mut session = env.session(&["--attach", "--live"]);
    env.ok(&["press", "south"]);
    wait_for("KEY_A down", || {
        keyboard.drain();
        keyboard.pressed.contains(&evdev::KeyCode::KEY_A.code()).then_some(())
    });
    // SAFETY: `kill` only sends a signal, to a child we started and have not yet waited for.
    assert_eq!(unsafe { libc::kill(session.0.id() as libc::pid_t, libc::SIGINT) }, 0);
    assert!(wait_for("the session to exit", || session.0.try_wait().unwrap()).success());
    wait_for("the key to be released", || {
        keyboard.drain();
        keyboard.pressed.is_empty().then_some(())
    });
    assert!(!status(&env).contains("Debug controller"), "{}", status(&env));
}

#[test]
#[ignore]
fn debug_a_second_attached_session_leaves_no_controller_behind() {
    let _serial = serial();
    let env = Env::new();
    let _daemon = debug_daemon(&env, &hold_config());
    let _session = env.session(&["--attach"]);
    let again = Env::reply(&env.command(&["debug", "--headless", "--attach"]).output().unwrap());
    assert!(!again.ok && again.err.contains("already running"), "{}", again.err);
    assert_eq!(status(&env).matches("Debug controller").count(), 1, "{}", status(&env));
}

#[test]
#[ignore]
fn debug_a_tap_the_daemon_never_got_leaves_nothing_held() {
    let _serial = serial();
    let env = Env::new();
    let mut daemon = debug_daemon(&env, &hold_config());
    let _session = env.session(&["--attach"]);
    daemon.0.kill().unwrap();
    daemon.0.wait().unwrap();
    assert!(!env.debug(&["tap", "south", "10"]).ok, "the daemon is gone");
    env.ok(&["expect", "released", "south"]);
    assert!(!env.debug(&["swipe", "0.1", "0.5", "0.9", "0.5", "10"]).ok);
    assert!(env.state().get("touches").and_then(Value::as_array).is_none_or(Vec::is_empty), "the finger was lifted");
}
