//! Tests that drive the real binary through `padwight debug`: the virtual controller's commands,
//! scripts and expectations, and, against a daemon started with `--debug`, a controller that no
//! hardware backs running through the daemon's mappings.
//!
//! The session tests need nothing but the binary and run with the rest, on Windows too. The daemon
//! tests need /dev/uinput and read access to /dev/input, so they are Linux only and opt-in like
//! the other daemon tests: `cargo test --test debug_session -- --ignored`, or
//! `scripts/kernel-test-docker.sh debug_`. Those are in `daemon.rs`.

#[path = "../common/mod.rs"]
mod common;
#[cfg(target_os = "linux")]
mod daemon;

use std::{
    fs,
    io::Write,
    path::PathBuf,
    process::{Child, Command, Stdio},
    thread,
};
#[cfg(target_os = "linux")]
use std::os::unix::fs::PermissionsExt;

use common::{temp_root, wait_for};
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
