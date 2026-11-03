//! A private D-Bus session bus for the stand-ins that live on D-Bus (KWin, Cinnamon), and the
//! daemon's end of it: the calls the desktop's script or extension makes when focus moves.

use std::{
    io::{BufRead, BufReader},
    path::Path,
    process::{Child, Command, Stdio},
};

use zbus::blocking::{Connection, connection::Builder};

use super::Window;

const DAEMON: &str = "io.github.marcinkoza0922.Padwight";
const FOCUS_PATH: &str = "/Focus";
const FOCUS_INTERFACE: &str = "io.github.marcinkoza0922.Padwight.Focus";

/// `dbus-daemon --session` on a socket under `dir`. Killed when dropped.
pub struct SessionBus {
    child: Child,
    pub address: String,
}

impl SessionBus {
    /// Keep `dir` short: a socket path is limited to 107 bytes.
    pub fn start(dir: &Path) -> Self {
        std::fs::create_dir_all(dir).unwrap();
        let socket = dir.join("bus.sock");
        let mut child = Command::new("dbus-daemon")
            .args(["--session", "--nofork", "--print-address=1"])
            .arg(format!("--address=unix:path={}", socket.display()))
            .stdout(Stdio::piped())
            .spawn()
            .expect("starting dbus-daemon (is it installed?)");
        let mut address = String::new();
        BufReader::new(child.stdout.take().unwrap()).read_line(&mut address).unwrap();
        Self { child, address: address.trim().to_string() }
    }

    /// A new connection to this bus, optionally owning `name`.
    pub fn connect(&self, name: Option<&str>) -> Connection {
        let mut builder = Builder::address(self.address.as_str()).expect("the bus address");
        if let Some(name) = name {
            builder = builder.name(name.to_string()).expect("a bus name");
        }
        builder.build().expect("connecting to the private bus")
    }
}

impl Drop for SessionBus {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

/// Whether anyone owns `name` on the bus `conn` is on.
pub fn has_owner(conn: &Connection, name: &str) -> bool {
    conn.call_method(Some("org.freedesktop.DBus"), "/org/freedesktop/DBus", Some("org.freedesktop.DBus"), "NameHasOwner", &(name,))
        .ok()
        .and_then(|reply| reply.body().deserialize::<bool>().ok())
        .unwrap_or(false)
}

/// What the KWin script and the Cinnamon extension send the daemon: the focused window as three
/// strings (the pid too), or that nothing has focus. Does nothing if the daemon is not on the bus.
/// Use a connection that is not serving the calls that lead here, or the reply can never arrive.
pub fn report_focus(out: &Connection, window: Option<&Window>) {
    if !has_owner(out, DAEMON) {
        return;
    }
    let _ = match window {
        Some(w) => out.call_method(Some(DAEMON), FOCUS_PATH, Some(FOCUS_INTERFACE), "WindowActivated", &(w.class.as_str(), w.pid.to_string(), w.title.as_str())),
        None => out.call_method(Some(DAEMON), FOCUS_PATH, Some(FOCUS_INTERFACE), "FocusCleared", &()),
    };
}
