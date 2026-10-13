//! Leaving games to Steam Input. A game Steam starts and lists with full controller support
//! already plays well with a pad, so while it's in front Padwight lets go of the controllers and
//! Steam Input has them. A setup can take the game back (`Game::over_steam`). Leaving the game
//! (another window, or none) takes the controllers back.

use std::time::Instant;

use super::{Daemon, describe};
use crate::{config::Config, focus, ipc::WindowInfo, monitor::log};

/// Whether Steam Input should have the controllers while `window` is in front: Steam started it,
/// `full_support` says Steam lists it with full controller support, and no setup for it says to
/// use it anyway.
pub(crate) fn left_to_steam(config: &Config, window: &WindowInfo, full_support: impl Fn(&str) -> bool) -> bool {
    let Some(app) = &window.steam_app_id else { return false };
    let taken_back = focus::matching_rule(config, window).is_some_and(|(game, _)| config.game(Some(game)).is_some_and(|g| g.over_steam));
    !taken_back && full_support(app)
}

impl Daemon {
    /// Whether a scan should take the controller called `name`: remapping is on, Steam doesn't
    /// have the controllers, and the user hasn't set it aside. Every path that grabs a pad asks
    /// this, so a pad that appears while Steam has the others is left to Steam too.
    pub(super) fn should_manage(&self, name: &str) -> bool {
        self.config.enabled && !self.steam_has_pads && !self.config.ignored_devices.iter().any(|n| n == name)
    }

    /// Gives the controllers to Steam Input while `window` is in front, if it's a game to leave
    /// to Steam, and says whether it was. Any other window takes them back.
    pub(super) fn hand_to_steam(&mut self, window: &WindowInfo) -> bool {
        if !left_to_steam(&self.config, window, crate::steam_appinfo::full_support) {
            self.take_pads_back();
            return false;
        }
        if !self.steam_has_pads {
            log!("{} has full controller support in Steam, so Steam Input gets the controllers", describe(window));
            self.steam_has_pads = true;
            self.release_devices(|_| true);
            let name = if window.title.trim().is_empty() { "this game".to_owned() } else { window.title.trim().to_owned() };
            self.toast = Some(crate::info::Toast::new(vec![format!("Steam Input has the controller in {name}")], Instant::now()));
            self.broadcast_overlay();
        }
        true
    }

    /// Decides again who has the controllers for the window in front, after the config changed
    /// (a setup's `over_steam`, or its rules). The window is handled as if it had just gained
    /// focus, so taking a game back also switches to its profile.
    pub(super) fn recheck_steam_handoff(&mut self) {
        let Some(window) = self.focused.clone() else { return };
        if left_to_steam(&self.config, &window, crate::steam_appinfo::full_support) != self.steam_has_pads {
            self.window_focused(&window);
        }
    }

    /// Takes the controllers back from Steam Input, if it has them.
    pub(super) fn take_pads_back(&mut self) {
        if self.steam_has_pads {
            self.steam_has_pads = false;
            log!("taking the controllers back from Steam Input");
            self.scan();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{Game, Profile, Rule, RuleKind};

    fn window(app: Option<&str>) -> WindowInfo {
        WindowInfo { exe: "SlayTheSpire2".into(), steam_app_id: app.map(Into::into), ..Default::default() }
    }

    #[test]
    fn steam_games_with_full_support_go_to_steam_unless_a_setup_takes_them() {
        let full = |id: &str| id == "2868840";
        let mut config = Config::default();
        assert!(left_to_steam(&config, &window(Some("2868840")), full));
        assert!(!left_to_steam(&config, &window(Some("6910")), full), "partial or no support stays with Padwight");
        assert!(!left_to_steam(&config, &window(None), full), "a game Steam didn't start stays with Padwight");

        let mut game = Game::new("Slay the Spire 2", vec![Profile::passthrough("P")]);
        game.rules = vec![Rule::new(RuleKind::SteamAppId, "2868840", "P")];
        config.games.push(game);
        assert!(left_to_steam(&config, &window(Some("2868840")), full), "a setup alone doesn't take it back");
        config.games[0].over_steam = true;
        assert!(!left_to_steam(&config, &window(Some("2868840")), full));
    }
}
