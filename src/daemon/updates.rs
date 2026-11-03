//! Finding out about new releases (once a day) and installing one on request. See `update.rs`.

use std::{
    process::Stdio,
    sync::atomic::{AtomicBool, Ordering},
    thread,
    time::Duration,
};

use super::{Daemon, Msg};
use crate::{
    ipc::{Response, UpdateInfo},
    monitor::log,
    update::{self, Source},
};

/// Wait this long after start-up before the first look, so it never competes with start-up work.
const FIRST_CHECK: Duration = Duration::from_secs(120);
/// An update is running: another request waits for it.
static INSTALLING: AtomicBool = AtomicBool::new(false);
const EVERY: Duration = Duration::from_secs(24 * 60 * 60);

impl Daemon {
    /// Looks on GitHub a little after start and then daily, on a thread of its own. Whether to look
    /// is read from the config each time, so the setting takes effect without a restart.
    pub(super) fn start_update_checks(&self) {
        let tx = self.tx.clone();
        thread::spawn(move || {
            thread::sleep(FIRST_CHECK);
            loop {
                if let Some(version) = checked() {
                    let _ = tx.send(Msg::UpdateFound(version));
                }
                thread::sleep(EVERY);
            }
        });
    }

    pub(super) fn update_found(&mut self, version: String) {
        if !self.config.check_updates {
            return;
        }
        log!("padwight {version} is available (running {})", update::CURRENT);
        self.update = Some(version);
        self.sync_tray();
    }

    /// The newer release found, if the setting is on, and how to get it.
    pub(super) fn update_info(&self) -> Option<UpdateInfo> {
        let version = self.update.clone().filter(|_| self.config.check_updates)?;
        let source = Source::detect();
        Some(UpdateInfo { version, self_updating: source.self_updating(), advice: source.advice().to_owned() })
    }

    /// Runs `padwight update` as a child. It ends with a restart of the service, and so of us.
    pub(super) fn install_update(&mut self) -> Response {
        if !Source::detect().self_updating() {
            return Response::Error(format!("not updated: {}", Source::detect().advice()));
        }
        // A replaced binary reads back as "<path> (deleted)": the new one is at the path.
        let exe = std::env::current_exe().unwrap_or_default();
        let exe = std::path::PathBuf::from(exe.to_string_lossy().trim_end_matches(" (deleted)"));
        if INSTALLING.swap(true, Ordering::SeqCst) {
            return Response::Error("an update is already running".into());
        }
        match std::process::Command::new(exe).arg("update").stdin(Stdio::null()).spawn() {
            Ok(mut child) => {
                log!("installing the update");
                // Reaped here, so a failed update doesn't linger as a zombie, and says so in the log.
                thread::spawn(move || {
                    match child.wait() {
                        Ok(status) if !status.success() => log!("the update failed ({status}); run `padwight update` to see why"),
                        Err(e) => log!("couldn't wait for the update: {e}"),
                        Ok(_) => {}
                    }
                    INSTALLING.store(false, Ordering::SeqCst);
                });
                Response::Ok
            }
            Err(e) => {
                INSTALLING.store(false, Ordering::SeqCst);
                Response::Error(format!("couldn't start the update: {e}"))
            }
        }
    }
}

/// A newer version, when the setting allows looking and GitHub has one.
fn checked() -> Option<String> {
    // The config is the daemon's; the file is what the setting was saved to.
    if !crate::config::Config::load().is_ok_and(|c| c.check_updates) {
        return None;
    }
    match update::available() {
        Ok(found) => found,
        Err(e) => {
            log!("update check failed: {e:#}");
            None
        }
    }
}
