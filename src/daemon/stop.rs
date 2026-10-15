//! Ending the daemon: a signal or the tray's Quit ends it at once, unless the settings window has
//! unsaved edits. Then the daemon waits, and the window asks about them (focused, over whatever it
//! shows) before the daemon ends. Cancelling the question keeps the daemon running.

use std::time::{Duration, Instant};

use super::Daemon;
use crate::monitor::log;

/// How long a settings window may go without reporting its unsaved edits before it counts as
/// gone (closed or crashed). It reports on every poll, about once a second, so this leaves room
/// for a slow reply.
const WINDOW_SILENT: Duration = Duration::from_secs(3);

/// Whether a report of unsaved edits made at `reported` still holds the daemon at `now`.
fn held(reported: Option<Instant>, now: Instant) -> bool {
    reported.is_some_and(|at| now.saturating_duration_since(at) < WINDOW_SILENT)
}

impl Daemon {
    /// A stop was asked for (a signal, or the tray's Quit): ends the daemon now, or once the
    /// settings window has dealt with its unsaved edits.
    pub(super) fn stop(&mut self) {
        if !held(self.unsaved_at, Instant::now()) {
            self.shut_down();
        }
        if !self.stop_asked {
            log!("waiting for the settings window to save or discard its unsaved changes");
        }
        self.stop_asked = true;
    }

    /// Ends the daemon once a stop that was asked for is no longer held up by unsaved edits.
    pub(super) fn finish_stop(&mut self) {
        if self.stop_asked && !held(self.unsaved_at, Instant::now()) {
            self.shut_down();
        }
    }

    /// The settings window's answer to the question: the daemon keeps running.
    pub(super) fn cancel_stop(&mut self) {
        if self.stop_asked {
            log!("stop cancelled: the settings window keeps its unsaved changes");
        }
        self.stop_asked = false;
    }

    /// Records whether the settings window has unsaved edits (`Request::Unsaved`).
    pub(super) fn report_unsaved(&mut self, unsaved: bool) {
        self.unsaved_at = unsaved.then(Instant::now);
    }

    /// When a stop that was asked for needs checking again: when the window's last report goes
    /// stale, which is what ends it if the window has gone.
    pub(super) fn stop_deadline(&self) -> Option<Instant> {
        if self.stop_asked { self.unsaved_at.map(|at| at + WINDOW_SILENT) } else { None }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unsaved_edits_hold_the_daemon_only_while_the_window_reports_them() {
        let now = Instant::now();
        assert!(!held(None, now), "no window has unsaved edits");
        assert!(held(Some(now), now), "a fresh report holds it");
        assert!(held(Some(now - Duration::from_secs(1)), now), "a report a second old still holds it");
        assert!(!held(Some(now - WINDOW_SILENT), now), "a window that went silent doesn't hold it");
    }
}
