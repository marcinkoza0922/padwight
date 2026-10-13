//! Following focus: which game is in front, switching to its profile (or back to the default),
//! handing it to Steam Input, and noticing when a game starts.

use std::time::Instant;

use super::{Daemon, RECENT_WINDOWS, describe, layered, with_fallback};
use crate::{
    config::ProfileRef,
    focus::{self, FocusEvent},
    ipc::{FocusBackend, WindowInfo},
    monitor::log,
};

impl Daemon {
    pub(super) fn window_focused(&mut self, window: &WindowInfo) {
        // Editing settings while a game runs must not flip the profile.
        if focus::is_own_window(window) {
            return;
        }
        let same = |w: &WindowInfo| w.class == window.class && w.exe == window.exe && w.steam_app_id == window.steam_app_id;
        self.recent_windows.retain(|w| !same(w));
        self.recent_windows.insert(0, window.clone());
        self.recent_windows.truncate(RECENT_WINDOWS);
        self.focused = Some(window.clone());
        if self.info_refresh.is_some() {
            self.broadcast_overlay();
        }
        if self.hand_to_steam(window) {
            // Steam has the controllers, but the last game's profile no longer fits what's in front.
            if self.config.auto_switch.enabled {
                self.leave_game(&format!("{} is left to Steam Input", describe(window)));
            }
            return;
        }
        self.hint_library_game(window);

        if !self.config.auto_switch.enabled {
            return;
        }
        match focus::matching_rule(&self.config, window) {
            Some((game, rule)) => {
                let picked = ProfileRef::new(Some(game), &rule.profile);
                let target = self.config.usable_profile(picked.clone(), self.pad_features().as_deref());
                let why = format!("{} matches {}", describe(window), focus::rule_text(rule));
                self.auto_switch_to(target.clone(), &with_fallback(why, &picked, &target), true);
            }
            None => self.leave_game(&format!("{} matches no rule", describe(window))),
        }
        if let Some((game, pid)) = focus::game_launch(&self.config, window) {
            self.launched(&game, pid);
        }
    }

    pub(super) fn focus_event(&mut self, event: FocusEvent) {
        match event {
            FocusEvent::Backend(backend) => {
                // Only a change of backend starts the process scan again. The first report can
                // arrive after a scan that already switched for a running game, and clearing then
                // would leave the daemon unable to notice that game exit.
                if self.focus_backend != backend {
                    self.focus_backend = backend;
                    // Re-evaluate from scratch the next time processes are scanned.
                    self.scan_target = None;
                }
            }
            FocusEvent::Focused(window) => self.window_focused(&window),
            FocusEvent::Unfocused => self.window_unfocused(),
        }
    }

    /// Nothing has focus (the desktop, or an empty workspace), so no game's rules apply.
    pub(super) fn window_unfocused(&mut self) {
        self.focused = None;
        self.take_pads_back();
        if self.config.auto_switch.enabled {
            self.leave_game("no window has focus");
        }
    }

    /// A rule matched `game` with process `pid`: if that's a process the game hasn't had,
    /// the game has just started.
    pub(super) fn launched(&mut self, game: &str, pid: u32) {
        if self.launches.get(game) == Some(&pid) {
            return;
        }
        self.launches.insert(game.to_owned(), pid);
        self.game_started(game);
    }

    /// The active game has just started: shows its "when the game starts" info overlays and
    /// switches on toggles set to start on.
    pub(super) fn game_started(&mut self, game: &str) {
        if self.config.active_ref().game.as_deref() != Some(game) {
            return;
        }
        log!("{game} started");
        self.toast_profile();
        self.info_timers.game_started(&self.scope.info, Instant::now());
        let Some(base) = self.config.active() else { return };
        for dev in self.devices.values_mut() {
            let mut out = Vec::new();
            // A toggle starting "Next profile" would be odd; it isn't followed.
            dev.engine.start_toggles(layered(&dev.layered, base), &self.scope.menus, &mut out);
            if dev.refresh_layers(&self.config) {
                dev.engine.resync(layered(&dev.layered, base), &mut out);
            }
            dev.dispatch(&mut self.kbm, out);
        }
        self.check_info_changes();
        self.broadcast_overlay();
    }

    /// Process-scan fallback for desktops without focus tracking.
    pub(super) fn scan_processes(&mut self) {
        let no_rules = focus::rules(&self.config).next().is_none();
        if self.focus_backend != FocusBackend::ProcessScan || !self.config.auto_switch.enabled || no_rules {
            return;
        }
        let processes = focus::running_processes();
        let target = focus::profile_for_processes(&self.config, &processes);
        if target != self.scan_target {
            self.scan_target = target.clone();
            match focus::matching_process(&self.config, &processes) {
                Some((game, rule, process)) => {
                    let picked = ProfileRef::new(Some(game), &rule.profile);
                    let target = self.config.usable_profile(picked.clone(), self.pad_features().as_deref());
                    let why = format!("{} is running, which matches {}", describe(process), focus::rule_text(rule));
                    self.auto_switch_to(target.clone(), &with_fallback(why, &picked, &target), true);
                }
                None => self.leave_game("no game running"),
            }
        }
        if let Some((game, pid)) = focus::game_launch_in(&self.config, &processes) {
            self.launched(&game, pid);
        }
    }

    /// Switches to a profile a rule (or the default) picked, with a toast if `toast`. `reason`
    /// says why, for the log and the GUI.
    pub(super) fn auto_switch_to(&mut self, target: ProfileRef, reason: &str, toast: bool) {
        if target == self.config.active_ref() {
            return;
        }
        if self.config.profile(&target).is_none() {
            log!("auto-switch points to missing profile {target}");
            return;
        }
        log!("{reason} → profile {target}");
        if toast {
            self.switch_profile(target, reason);
        } else {
            self.set_profile(target, reason);
        }
    }

    /// No game has focus: a profile of a game that rules pick gives way to the default one.
    /// No toast, since alt-tabbing out of a game would show one every time.
    pub(super) fn leave_game(&mut self, reason: &str) {
        let in_game = self.config.active_ref().game.is_some_and(|g| focus::has_rules(&self.config, &g));
        if in_game && let Some(default) = self.config.auto_switch.default_profile.clone() {
            let why = format!("{reason}, so the default profile takes over");
            self.auto_switch_to(default, &why, false);
        }
    }
}
