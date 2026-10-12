//! Tests that drive the real binary through `padwight debug`: the virtual controller's commands,
//! scripts and expectations, and, against a daemon started with `--debug`, a controller that no
//! hardware backs running through the daemon's mappings.
//!
//! The session tests need nothing but the binary and run with the rest, on Windows too. The daemon
//! tests need /dev/uinput and read access to /dev/input, so they are Linux only and opt-in like
//! the other daemon tests: `cargo test --test debug_session -- --ignored`, or
//! `scripts/kernel-test-docker.sh debug_`.

mod common;

use std::{
    fs,
    io::Write,
    path::PathBuf,
    process::{Child, Command, Stdio},
    thread,
};
#[cfg(target_os = "linux")]
use std::{os::unix::fs::PermissionsExt, time::Duration};

use common::{temp_root, wait_for};
#[cfg(target_os = "linux")]
use common::{Daemon, hold_config, install_config, input::Reader, serial, spawn_daemon, spawn_daemon_with};
use serde_json::Value;

const BIN: &str = env!("CARGO_BIN_EXE_padwight");

/// What a command printed, and whether it succeeded.
struct Reply {
    ok: bool,
    out: String,
    err: String,
}

/// One test's directory. Sessions and daemons find each other through `XDG_RUNTIME_DIR` inside
/// it, so tests never meet a session or daemon of the person running them.
struct Env {
    root: PathBuf,
}

impl Env {
    fn new() -> Self {
        let root = temp_root();
        fs::create_dir_all(root.join("run")).unwrap();
        Env { root }
    }

    fn run(&self) -> PathBuf {
        self.root.join("run")
    }

    fn command(&self, args: &[&str]) -> Command {
        let mut command = Command::new(BIN);
        command.args(args).env("XDG_RUNTIME_DIR", self.run()).env_remove("PADWIGHT_DEBUG_SOCKET").stdin(Stdio::null());
        command
    }

    fn reply(output: &std::process::Output) -> Reply {
        Reply {
            ok: output.status.success(),
            out: String::from_utf8_lossy(&output.stdout).trim().to_string(),
            err: String::from_utf8_lossy(&output.stderr).trim().to_string(),
        }
    }

    /// `padwight debug <args>`, waiting up to `wait_ms` for a session that is still starting.
    fn debug_wait(&self, wait_ms: u64, args: &[&str]) -> Reply {
        let wait = wait_ms.to_string();
        let mut full = vec!["debug", "--wait", &wait];
        full.extend_from_slice(args);
        Self::reply(&self.command(&full).output().unwrap())
    }

    fn debug(&self, args: &[&str]) -> Reply {
        self.debug_wait(5000, args)
    }

    /// A command that must succeed; returns what it printed.
    fn ok(&self, args: &[&str]) -> String {
        let reply = self.debug(args);
        assert!(reply.ok, "padwight debug {args:?} failed: {}", reply.err);
        reply.out
    }

    /// `padwight debug run`, with `script` on its standard input.
    fn script(&self, script: &str) -> Reply {
        let mut child = self.command(&["debug", "--wait", "5000", "run"]).stdin(Stdio::piped()).stdout(Stdio::piped()).stderr(Stdio::piped()).spawn().unwrap();
        child.stdin.take().unwrap().write_all(script.as_bytes()).unwrap();
        Self::reply(&child.wait_with_output().unwrap())
    }

    /// The virtual controller's input, as the session reports it.
    fn state(&self) -> Value {
        serde_json::from_str(&self.ok(&["state"])).unwrap()
    }

    /// Starts a headless session with these flags and waits until it answers.
    fn session(&self, flags: &[&str]) -> Session {
        let log = fs::File::create(self.root.join("session.log")).unwrap();
        let mut args = vec!["debug", "--headless"];
        args.extend_from_slice(flags);
        let child = self.command(&args).stdout(log.try_clone().unwrap()).stderr(log).spawn().unwrap();
        let session = Session(child);
        self.ok(&["ping"]);
        session
    }
}

impl Drop for Env {
    fn drop(&mut self) {
        if !thread::panicking() {
            let _ = fs::remove_dir_all(&self.root);
        }
    }
}

/// A running session, killed if the test fails before it has quit.
struct Session(Child);

impl Drop for Session {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

fn buttons(state: &Value) -> Vec<&str> {
    state["buttons"].as_array().unwrap().iter().map(|b| b.as_str().unwrap()).collect()
}

#[test]
fn commands_change_the_virtual_controller() {
    let env = Env::new();
    let _session = env.session(&[]);
    env.ok(&["model", "dualsense"]);
    env.ok(&["press", "a", "lb"]);
    env.ok(&["stick", "left", "0.5", "-1"]);
    env.ok(&["trigger", "rt", "0.25"]);
    let state = env.state();
    assert_eq!(state["model"], "DualSense");
    assert_eq!(state["family"], "PlayStation");
    assert_eq!(buttons(&state), ["South", "LeftBumper"]);
    assert_eq!(state["left_stick"], serde_json::json!([0.5, -1.0]));
    assert_eq!(state["right_trigger"], 0.25);
    env.ok(&["release", "south"]);
    assert_eq!(buttons(&env.state()), ["LeftBumper"]);
}

#[test]
fn a_session_starts_as_the_model_asked_for() {
    let env = Env::new();
    let _session = env.session(&["--model", "xbox-elite"]);
    assert_eq!(env.state()["model"], "XboxElite");
    assert!(env.ok(&["render"]).starts_with("<svg"));
}

#[test]
fn expectations_set_the_exit_status() {
    let env = Env::new();
    let _session = env.session(&[]);
    env.ok(&["press", "south"]);
    env.ok(&["expect", "pressed", "south"]);
    let wrong = env.debug(&["expect", "released", "south"]);
    assert!(!wrong.ok && wrong.err.contains("South"), "{}", wrong.err);
    assert!(!env.debug(&["expect", "idle"]).ok);
    assert!(!env.debug(&["expect", "model", "dualsense"]).ok);
    env.ok(&["reset"]);
    env.ok(&["expect", "idle"]);
    env.ok(&["stick", "right", "0.3", "0"]);
    env.ok(&["expect", "stick", "right", "0.3", "0"]);
    assert!(!env.debug(&["expect", "stick", "right", "0.9", "0"]).ok);
    env.ok(&["trigger", "left", "1"]);
    env.ok(&["expect", "trigger", "left", "1"]);
}

#[test]
fn scripts_run_in_order_and_stop_at_the_first_failure() {
    let env = Env::new();
    let _session = env.session(&[]);
    let script = "# a comment\npress south\n\nsleep 10\nstick left 1 0   # trailing comment\nexpect pressed south\n";
    assert!(env.script(script).ok);
    assert_eq!(buttons(&env.state()), ["South"]);

    let failing = env.script("reset\nexpect pressed east\npress start\n");
    assert!(!failing.ok && failing.err.contains("line 2"), "{}", failing.err);
    assert_eq!(buttons(&env.state()), Vec::<&str>::new(), "the line after the failure never ran");
}

#[test]
fn a_typo_in_a_script_stops_it_before_anything_is_sent() {
    let env = Env::new();
    let _session = env.session(&[]);
    let reply = env.script("press south\nbogus\n");
    assert!(!reply.ok && reply.err.contains("line 2") && reply.err.contains("bogus"), "{}", reply.err);
    assert_eq!(buttons(&env.state()), Vec::<&str>::new());
}

#[test]
fn every_model_can_be_chosen_and_has_its_own_drawing() {
    let env = Env::new();
    let models = env.ok(&["models"]);
    let names: Vec<&str> = models.lines().collect();
    assert!(names.len() >= 14 && names.contains(&"generic") && names.contains(&"dualsense"), "{names:?}");
    let _session = env.session(&[]);
    let mut drawings = Vec::new();
    for name in &names {
        env.ok(&["model", name]);
        env.ok(&["expect", "model", name]);
        let svg = env.ok(&["render"]);
        assert!(svg.starts_with("<svg") && svg.ends_with("</svg>"), "{name}");
        drawings.push(svg);
    }
    drawings.sort();
    drawings.dedup();
    assert_eq!(drawings.len(), names.len(), "two models drew the same picture");
}

#[test]
fn the_family_changes_the_button_colors() {
    let env = Env::new();
    let _session = env.session(&["--model", "generic"]);
    env.ok(&["family", "xbox"]);
    let xbox = env.ok(&["render"]);
    env.ok(&["family", "playstation"]);
    assert_ne!(xbox, env.ok(&["render"]));
    assert_eq!(env.state()["family"], "PlayStation");
}

#[test]
fn pressing_is_drawn() {
    let env = Env::new();
    let _session = env.session(&[]);
    let rest = env.ok(&["render"]);
    env.ok(&["press", "south"]);
    assert_ne!(rest, env.ok(&["render"]));
    env.ok(&["release", "south"]);
    assert_eq!(rest, env.ok(&["render"]));
}

#[test]
fn models_and_drawings_need_no_session() {
    let env = Env::new();
    let svg = env.debug_wait(0, &["svg", "xbox360"]);
    assert!(svg.ok && svg.out.starts_with("<svg"), "{}", svg.err);
    assert_ne!(svg.out, env.debug_wait(0, &["svg"]).out, "the generic drawing differs");
    let unknown = env.debug_wait(0, &["svg", "nope"]);
    assert!(!unknown.ok && unknown.err.contains("unknown model"));
    assert!(env.debug_wait(0, &["help"]).out.contains("expect pressed"));
}

#[test]
fn a_missing_session_is_reported() {
    let env = Env::new();
    let reply = env.debug_wait(0, &["state"]);
    assert!(!reply.ok && reply.err.contains("no debug session"), "{}", reply.err);
}

#[test]
fn a_bad_command_is_explained_without_a_session() {
    let env = Env::new();
    let reply = env.debug_wait(0, &["press"]);
    assert!(!reply.ok && reply.err.contains("which button"), "{}", reply.err);
}

#[test]
fn sessions_on_different_sockets_are_independent() {
    let env = Env::new();
    let _first = env.session(&[]);
    let other = env.root.join("other.sock");
    let log = fs::File::create(env.root.join("other.log")).unwrap();
    let mut second = Command::new(BIN)
        .args(["debug", "--headless", "--model", "dualsense"])
        .env("PADWIGHT_DEBUG_SOCKET", &other)
        .stdout(log.try_clone().unwrap())
        .stderr(log)
        .spawn()
        .map(Session)
        .unwrap();
    let on_other = |args: &[&str]| {
        let mut full = vec!["debug", "--wait", "5000"];
        full.extend_from_slice(args);
        Env::reply(&Command::new(BIN).args(full).env("PADWIGHT_DEBUG_SOCKET", &other).output().unwrap())
    };
    assert!(on_other(&["press", "start"]).ok);
    assert_eq!(buttons(&env.state()), Vec::<&str>::new(), "the first session didn't feel it");
    assert_eq!(env.state()["model"], serde_json::Value::Null);
    assert!(on_other(&["quit"]).ok);
    assert!(wait_for("the second session to exit", || second.0.try_wait().unwrap()).success());
    assert!(!other.exists(), "the socket is removed on the way out");
}

#[test]
fn quit_ends_the_session_and_removes_its_socket() {
    let env = Env::new();
    let mut session = env.session(&[]);
    let socket = env.run().join("padwight-debug.sock");
    assert!(socket.exists());
    env.ok(&["quit"]);
    assert!(wait_for("the session to exit", || session.0.try_wait().unwrap()).success());
    assert!(!socket.exists());
    #[cfg(target_os = "linux")]
    assert_eq!(fs::metadata(env.run()).unwrap().permissions().mode() & 0o077, 0, "the socket's folder is private");
}

#[test]
fn a_second_session_cannot_take_the_socket() {
    let env = Env::new();
    let _session = env.session(&[]);
    let again = Env::reply(&env.command(&["debug", "--headless"]).output().unwrap());
    assert!(!again.ok && again.err.contains("already running"), "{}", again.err);
    env.ok(&["ping"]);
}

#[test]
fn a_tap_holds_for_the_time_given() {
    let env = Env::new();
    let _session = env.session(&[]);
    let mut tap = env.command(&["debug", "--wait", "5000", "tap", "south", "800"]).spawn().unwrap();
    wait_for("the button to be held", || buttons(&env.state()).contains(&"South").then_some(()));
    assert!(tap.wait().unwrap().success());
    assert_eq!(buttons(&env.state()), Vec::<&str>::new(), "and let go after");
}

#[test]
fn attaching_needs_a_daemon() {
    let env = Env::new();
    let reply = Env::reply(&env.command(&["debug", "--headless", "--attach"]).output().unwrap());
    assert!(!reply.ok && reply.err.contains("daemon"), "{}", reply.err);
}

#[test]
fn live_output_needs_attaching() {
    let env = Env::new();
    let reply = Env::reply(&env.command(&["debug", "--headless", "--live"]).output().unwrap());
    assert!(!reply.ok && reply.err.contains("--live needs --attach"), "{}", reply.err);
}

#[test]
fn output_needs_attaching() {
    let env = Env::new();
    let _session = env.session(&[]);
    let reply = env.debug(&["output"]);
    assert!(!reply.ok && reply.err.contains("--attach"), "{}", reply.err);
    assert!(!env.debug(&["expect", "no-output"]).ok);
}

// What follows needs a daemon, and so /dev/uinput: Linux only.

/// A daemon started with `--debug`, running `config`, and ready for requests.
#[cfg(target_os = "linux")]
fn debug_daemon(env: &Env, config: &str) -> Daemon {
    let dir = env.root.join("config/padwight");
    fs::create_dir_all(&dir).unwrap();
    install_config(&dir, config);
    let daemon = spawn_daemon_with(&env.root, Some(&env.run()), &["--debug"]);
    wait_for("the daemon to answer", || Command::new(BIN).arg("status").env("XDG_RUNTIME_DIR", env.run()).output().ok()?.status.success().then_some(()));
    daemon
}

#[cfg(target_os = "linux")]
fn status(env: &Env) -> String {
    let out = env.command(&["status"]).output().unwrap();
    String::from_utf8_lossy(&out.stdout).into_owned()
}

/// The config with the active profile's gyro turning the mouse.
#[cfg(target_os = "linux")]
fn gyro_mouse_config() -> String {
    let text = hold_config().replacen("[general.profiles.gyro]\nmode = \"off\"", "[general.profiles.gyro]\nmode = { mouse = { sensitivity = 2.0 } }", 1);
    assert!(text.contains("sensitivity = 2.0"), "the default config's gyro is off");
    text
}

#[cfg(target_os = "linux")]
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

#[cfg(target_os = "linux")]
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

#[cfg(target_os = "linux")]
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

#[cfg(target_os = "linux")]
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

#[cfg(target_os = "linux")]
#[test]
#[ignore]
fn debug_output_stays_off_the_virtual_keyboard_without_live() {
    let _serial = serial();
    let env = Env::new();
    let _daemon = debug_daemon(&env, &hold_config());
    let mut keyboard = Reader::open_newest("Padwight Virtual Keyboard");
    let _session = env.session(&["--attach"]);
    env.ok(&["press", "south"]);
    env.ok(&["expect", "output", "key KEY_A down"]);
    thread::sleep(Duration::from_millis(200));
    keyboard.drain();
    assert!(keyboard.pressed.is_empty(), "the key was typed for real");
    env.ok(&["release", "south"]);
}

#[cfg(target_os = "linux")]
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

#[cfg(target_os = "linux")]
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

#[cfg(target_os = "linux")]
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
#[cfg(target_os = "linux")]
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

#[cfg(target_os = "linux")]
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

#[cfg(target_os = "linux")]
#[test]
#[ignore]
fn debug_live_output_reaches_the_virtual_keyboard() {
    let _serial = serial();
    let env = Env::new();
    let _daemon = debug_daemon(&env, &hold_config());
    let mut keyboard = Reader::open_newest("Padwight Virtual Keyboard");
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

#[cfg(target_os = "linux")]
#[test]
#[ignore]
fn debug_detaching_a_controller_lets_go_of_what_it_held() {
    let _serial = serial();
    let env = Env::new();
    let _daemon = debug_daemon(&env, &hold_config());
    let mut keyboard = Reader::open_newest("Padwight Virtual Keyboard");
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

#[cfg(target_os = "linux")]
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
