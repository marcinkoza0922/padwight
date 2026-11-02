//! Cinnamon: `org.Cinnamon` on the bus, a fake `gsettings` for `enabled-extensions`, and, once
//! the daemon has installed and enabled the extension, the extension's own part: it owns
//! `io.github.marcinkoza0922.Padwight.Shell`, serves the overlay object (see [`super::overlay`])
//! and reports focus to the daemon. The extension's JavaScript is not run.

use std::{
    fs,
    os::unix::fs::PermissionsExt,
    path::{Path, PathBuf},
    sync::{
        Arc, Mutex, PoisonError,
        atomic::{AtomicBool, Ordering},
    },
    thread::{self, JoinHandle},
    time::Duration,
};

use zbus::blocking::Connection;

use super::{
    Compositor, Window,
    bus::{SessionBus, has_owner, report_focus},
    overlay::{Overlay, OverlayLog},
};

const UUID: &str = "padwight-focus@io.github.marcinkoza0922";
const DAEMON: &str = "io.github.marcinkoza0922.Padwight";
const LOADED_NAME: &str = "io.github.marcinkoza0922.Padwight.Shell";

pub struct Cinnamon {
    focused: Arc<Mutex<Option<Window>>>,
    out: Connection,
    log: OverlayLog,
    bin: PathBuf,
    stop: Arc<AtomicBool>,
    watcher: Option<JoinHandle<()>>,
    _shell: Connection,
}

impl Cinnamon {
    /// `dir` holds the fake `gsettings` (in `dir/bin`, put that first in the daemon's `PATH`);
    /// `data_home` is the daemon's `XDG_DATA_HOME`, where the extension gets installed. With
    /// `enabled` the extension is already in the enabled list and installed, as after a first
    /// run; without, the daemon installs and enables it, and the stand-in loads it a moment
    /// later (the daemon looks again after its ten-second watchdog).
    pub fn start(bus: &SessionBus, dir: &Path, data_home: &Path, enabled: bool) -> Self {
        let bin = dir.join("bin");
        fs::create_dir_all(&bin).unwrap();
        let script = bin.join("gsettings");
        fs::write(
            &script,
            "#!/bin/sh\nf=\"$(dirname \"$0\")/../enabled\"\ncase \"$1\" in\n  get) cat \"$f\" 2>/dev/null || echo '@as []';;\n  set) printf '%s\\n' \"$4\" > \"$f\";;\nesac\n",
        )
        .unwrap();
        fs::set_permissions(&script, fs::Permissions::from_mode(0o755)).unwrap();
        let list = dir.join("enabled");
        let extension = data_home.join("cinnamon/extensions").join(UUID);
        if enabled {
            fs::write(&list, format!("['{UUID}']\n")).unwrap();
        }

        let shell = bus.connect(Some("org.Cinnamon"));
        let out = bus.connect(None);
        let focused: Arc<Mutex<Option<Window>>> = Arc::default();
        let log = OverlayLog::default();
        let stop = Arc::new(AtomicBool::new(false));
        // Already enabled: Cinnamon loaded it at login, before the daemon started.
        let loaded = enabled.then(|| load(&bus.address, &log)).flatten();
        let watcher = {
            let (bus_address, out, focused, log, stop) = (bus.address.clone(), out.clone(), focused.clone(), log.clone(), stop.clone());
            thread::spawn(move || watch(&bus_address, &out, &list, &extension, &focused, &log, &stop, loaded))
        };
        Self { focused, out, log, bin, stop, watcher: Some(watcher), _shell: shell }
    }

    /// The directory with the fake `gsettings`: put it first in the daemon's `PATH`.
    pub fn bin_dir(&self) -> &Path {
        &self.bin
    }

    /// What the overlay process has asked of the extension.
    pub fn overlay(&self) -> &OverlayLog {
        &self.log
    }

    fn set(&self, window: Option<&Window>) {
        *self.focused.lock().unwrap_or_else(PoisonError::into_inner) = window.cloned();
        // The extension is on the bus once it is loaded.
        if has_owner(&self.out, LOADED_NAME) {
            report_focus(&self.out, window);
        }
    }
}

/// The extension, loaded: it owns its name and serves the overlay object.
fn load(address: &str, log: &OverlayLog) -> Option<Connection> {
    zbus::blocking::connection::Builder::address(address)
        .and_then(|b| b.name(LOADED_NAME))
        .and_then(|b| b.serve_at("/Overlay", Overlay::new(log.clone())))
        .and_then(zbus::blocking::connection::Builder::build)
        .ok()
}

/// Plays the extension: loads when the files are installed and the uuid is enabled, and reports
/// the focused window whenever the daemon appears on the bus.
#[expect(clippy::too_many_arguments, reason = "a test helper's thread body")]
fn watch(
    address: &str,
    out: &Connection,
    list: &Path,
    extension: &Path,
    focused: &Mutex<Option<Window>>,
    log: &OverlayLog,
    stop: &AtomicBool,
    mut loaded: Option<Connection>,
) {
    let mut daemon_seen = false;
    while !stop.load(Ordering::Relaxed) {
        thread::sleep(Duration::from_millis(50));
        if loaded.is_none() {
            let enabled = fs::read_to_string(list).is_ok_and(|text| text.contains(UUID));
            if enabled && extension.join("extension.js").exists() {
                loaded = load(address, log);
            }
        }
        let present = loaded.is_some() && has_owner(out, DAEMON);
        if present && !daemon_seen {
            report_focus(out, focused.lock().unwrap_or_else(PoisonError::into_inner).as_ref());
        }
        daemon_seen = present;
    }
}

impl Drop for Cinnamon {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
        if let Some(watcher) = self.watcher.take() {
            let _ = watcher.join();
        }
    }
}

impl Compositor for Cinnamon {
    fn focus(&self, window: &Window) {
        self.set(Some(window));
    }

    fn unfocus(&self) {
        self.set(None);
    }

    fn close(&self, _window: &Window) {
        self.set(None);
    }
}
