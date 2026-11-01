//! Finding out about new releases (once a day) and installing one on request. See `update.rs`.

use std::{process::Stdio, thread, time::Duration};

use super::{Daemon, Msg};
use crate::{
    ipc::{Response, UpdateInfo},
    monitor::log,
    update::{self, Source},
};

/// Wait this long after start-up before the first look, so it never competes with start-up work.
const FIRST_CHECK: Duration = Duration::from_secs(120);
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
        let exe = std::env::current_exe().unwrap_or_default();
        match std::process::Command::new(exe).arg("update").stdin(Stdio::null()).spawn() {
            Ok(_) => {
                log!("installing the update");
                Response::Ok
            }
            Err(e) => Response::Error(format!("couldn't start the update: {e}")),
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
