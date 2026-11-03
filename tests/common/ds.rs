//! Helpers for tests that use the DualSense replica (the kernel's `hid-playstation` binds it, so
//! the daemon sees the gamepad, motion and touchpad nodes a real one has), and for talking to the
//! daemon a test started.

use std::{
    fs,
    io::{BufRead, BufReader, Write},
    os::unix::net::UnixStream,
    path::{Path, PathBuf},
    process::Command,
    thread,
    time::Duration,
};

use super::{
    Daemon, daemon_log, install_config,
    input::{Reader, assert_no_other_daemon, find_node, grabbed, nodes},
    spawn_daemon,
    uhid::{DualSenseState, HidPad},
    wait_for,
};

/// The virtual devices that exist before a test starts its daemon. Other daemons on the machine
/// (a developer's, or another test run's debug session) have devices of the same names; the test
/// reads only the ones that appeared after the snapshot.
pub struct Snapshot(Vec<(&'static str, Vec<PathBuf>)>);

impl Snapshot {
    pub fn take() -> Self {
        Snapshot(["Padwight Virtual Keyboard", "Padwight Virtual Mouse", "Padwight Virtual Pad"].into_iter().map(|name| (name, nodes(name))).collect())
    }

    /// The nodes called `name` that came since the snapshot.
    pub fn mine(&self, name: &str) -> Vec<PathBuf> {
        let before = self.0.iter().find(|(n, _)| *n == name).map_or(&[][..], |(_, nodes)| nodes.as_slice());
        nodes(name).into_iter().filter(|path| !before.contains(path)).collect()
    }

    /// Opens the new node called `name`, waiting for it to appear.
    pub fn open(&self, name: &str) -> Reader {
        let before = self.0.iter().find(|(n, _)| *n == name).map_or(&[][..], |(_, nodes)| nodes.as_slice());
        Reader::open_new(name, before)
    }
}

pub const DUALSENSE: &str = "Sony Interactive Entertainment DualSense Wireless Controller";

/// Writes `config` and starts a daemon on it, with its socket under `root/run`.
pub fn start(root: &Path, config: &str) -> Daemon {
    // A daemon the previous test just killed takes a moment to lose its devices.
    for _ in 0..100 {
        if find_node("Padwight Virtual Keyboard").is_none() {
            break;
        }
        thread::sleep(Duration::from_millis(50));
    }
    assert_no_other_daemon();
    let dir = root.join("config/padwight");
    fs::create_dir_all(&dir).unwrap();
    install_config(&dir, config);
    spawn_daemon(root, Some(&root.join("run")))
}

/// `config` with the `n`th DualSense replica already known and set up to follow the active
/// profile. Without it, a second controller on the machine (another test's, a developer's) makes
/// the daemon ask how to set the new one up, and until that is answered the pad passes through.
pub fn with_known_dualsense(config: &str, n: u8) -> String {
    format!("{config}\n[[controllers]]\nid = \"02:70:61:64:00:{n:02x}\"\n")
}

/// A DualSense plugged in, once the daemon holds its gamepad node.
pub fn connect_dualsense(n: u8) -> HidPad {
    let pad = HidPad::dualsense(n);
    wait_for("the daemon to grab the DualSense", || find_node(DUALSENSE).filter(|path| grabbed(path)));
    pad
}

/// The sensor's clock (0.33 microsecond units) for the `tick`th report, 4 ms apart.
pub fn clock(tick: u32) -> u32 {
    tick * 12_000
}

/// Sends the state until `done` says the effect showed, since the kernel drops reports nobody reads.
pub fn drive(pad: &mut HidPad, what: &str, mut next: impl FnMut(u32) -> DualSenseState, mut done: impl FnMut() -> bool) {
    let mut tick = 0u32;
    wait_for(what, || {
        tick += 1;
        pad.send(&next(tick).report());
        thread::sleep(Duration::from_millis(8));
        done().then_some(())
    });
}

/// `padwight <args>` against the daemon started in `root`; fails the test if it does.
pub fn padwight(root: &Path, args: &[&str]) -> String {
    let out = Command::new(env!("CARGO_BIN_EXE_padwight"))
        .args(args)
        .env("XDG_CONFIG_HOME", root.join("config"))
        .env("XDG_RUNTIME_DIR", root.join("run"))
        .output()
        .unwrap();
    assert!(out.status.success(), "padwight {args:?}: {}\n{}", String::from_utf8_lossy(&out.stderr), daemon_log(root));
    String::from_utf8_lossy(&out.stdout).into_owned()
}

/// What `padwight debug overlay state` shows of the daemon in `root`, as JSON.
pub fn overlay_state(root: &Path) -> serde_json::Value {
    serde_json::from_str(&padwight(root, &["debug", "overlay", "state"])).expect("the overlay state is JSON")
}

/// The daemon's status as JSON, asked over its control socket the way the settings window does.
pub fn status(root: &Path) -> serde_json::Value {
    let mut stream = UnixStream::connect(root.join("run/padwight.sock")).expect("the daemon's socket");
    stream.set_read_timeout(Some(Duration::from_secs(5))).unwrap();
    stream.write_all(b"\"Status\"\n").unwrap();
    let mut line = String::new();
    BufReader::new(stream).read_line(&mut line).unwrap();
    let mut reply: serde_json::Value = serde_json::from_str(&line).expect("a status reply");
    reply["Status"].take()
}
