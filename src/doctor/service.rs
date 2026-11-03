//! The check that the daemon is running.

use std::path::PathBuf;

use super::{Finding, Level};

pub(super) fn check_daemon() -> Finding {
    match crate::ipc::request(&crate::ipc::Request::Status) {
        Ok(_) => Finding::new(Level::Ok, "Daemon: running", ""),
        Err(_) => daemon_down(),
    }
}

fn daemon_down() -> Finding {
    let unit = home_unit_exists();
    let fix = if unit { "systemctl --user enable --now padwight" } else { "padwight daemon" };
    let detail = if unit {
        "Start it now and at every login."
    } else {
        "No padwight.service is installed for your user. Start it by hand, or install the unit (the release tarball's install.sh does)."
    };
    Finding::new(Level::Note, "Daemon: not running", detail).fix(fix.to_owned())
}

fn home_unit_exists() -> bool {
    dirs::config_dir()
        .map(|d| d.join("systemd/user"))
        .into_iter()
        .chain(["/etc/systemd/user", "/usr/lib/systemd/user"].map(PathBuf::from))
        .any(|dir| dir.join("padwight.service").exists())
}
