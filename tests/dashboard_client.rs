//! The example dashboard's Python client (`examples/dashboard/server.py`) against a real daemon
//! (T-15): a client in another language reads `Version`, `Status`, `GetConfig` and a `WatchInput`
//! stream as `docs/protocol.md` describes them.
//!
//! The daemon runs with `--debug` on throwaway config and runtime folders. It needs no controller
//! and no /dev/input or /dev/uinput, so with no pad connected `WatchInput` sends `null`; this runs
//! with the ordinary tests, where `python3` exists (CI has it).

#![cfg(target_os = "linux")]

mod common;

use std::process::Command;

use common::{serial, spawn_daemon_with, temp_root, wait_for};

#[test]
fn the_dashboard_client_speaks_the_documented_protocol() {
    let version = Command::new("python3").arg("--version").output().expect("this test needs python3");
    assert!(version.status.success());
    let _serial = serial();
    let root = temp_root();
    let run = root.join("run");
    let daemon = spawn_daemon_with(&root, Some(&run), &["--debug"]);
    let socket = run.join("padwight.sock");
    wait_for("the daemon's socket", || socket.exists().then_some(()));

    let script = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/dashboard_client/check.py");
    let out = Command::new("python3").args(["-I", script]).env("XDG_RUNTIME_DIR", &run).output().expect("running python3");
    drop(daemon);
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(out.status.success(), "{stderr}\n--- daemon log ---\n{}", common::daemon_log(&root));
    assert_eq!(String::from_utf8_lossy(&out.stdout).trim(), "ok");
    let _ = std::fs::remove_dir_all(&root);
}
