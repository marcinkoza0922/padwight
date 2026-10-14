//! Renaming a setup or one of its profiles, with everything that names it following: the setup's
//! rules, the auto-switch default, and the active profile.

use super::Config;

impl Config {
    /// Renames profile `old` of `game` (General when `None`) to `new`. False when another profile
    /// of the game has that name, `new` is empty, or nothing changed.
    pub fn rename_profile(&mut self, game: Option<&str>, old: &str, new: &str) -> bool {
        let Some(g) = self.game_mut(game) else { return false };
        if new.is_empty() || old == new || g.profiles.iter().any(|p| p.name == new) {
            return false;
        }
        let Some(profile) = g.profiles.iter_mut().find(|p| p.name == old) else { return false };
        profile.name = new.to_string();
        for rule in g.rules.iter_mut().filter(|r| r.profile == old) {
            rule.profile = new.to_string();
        }
        let named = |at: &super::ProfileRef| at.game.as_deref() == game && at.profile == old;
        if let Some(d) = self.auto_switch.default_profile.as_mut().filter(|d| named(d)) {
            d.profile = new.to_string();
        }
        if named(&self.active) {
            self.active.profile = new.to_string();
        }
        true
    }

    /// Renames setup `old` to `new`. False when another setup has that name, `new` is empty, or
    /// nothing changed.
    pub fn rename_game(&mut self, old: &str, new: &str) -> bool {
        if new.is_empty() || old == new || self.games.iter().any(|g| g.name == new) {
            return false;
        }
        let Some(game) = self.games.iter_mut().find(|g| g.name == old) else { return false };
        game.name = new.to_string();
        for at in self.auto_switch.default_profile.iter_mut().chain([&mut self.active]) {
            if at.game.as_deref() == Some(old) {
                at.game = Some(new.to_string());
            }
        }
        true
    }
}

#[cfg(test)]
mod tests {
    use crate::config::{Config, Game, Profile, ProfileRef, Rule, RuleKind};

    fn doom() -> Config {
        let mut config = Config::default();
        let mut game = Game::new("Doom", vec![Profile::passthrough("Play"), Profile::passthrough("Menus")]);
        game.rules.push(Rule::new(RuleKind::Executable, "doom.exe", "Play"));
        config.games.push(game);
        config.active = ProfileRef::new(Some("Doom"), "Play");
        config.auto_switch.default_profile = Some(ProfileRef::new(Some("Doom"), "Play"));
        config
    }

    #[test]
    fn a_renamed_profile_keeps_its_rules_and_stays_active() {
        let mut config = doom();
        assert!(!config.rename_profile(Some("Doom"), "Play", "Menus"), "taken");
        assert!(config.rename_profile(Some("Doom"), "Play", "Campaign"));
        assert_eq!(config.games[0].rules[0].profile, "Campaign");
        assert_eq!(config.active, ProfileRef::new(Some("Doom"), "Campaign"));
        assert_eq!(config.auto_switch.default_profile, Some(ProfileRef::new(Some("Doom"), "Campaign")));
    }

    #[test]
    fn a_renamed_game_stays_active() {
        let mut config = doom();
        assert!(config.rename_game("Doom", "Doom II"));
        assert_eq!(config.active, ProfileRef::new(Some("Doom II"), "Play"));
        assert_eq!(config.auto_switch.default_profile.as_ref().unwrap().game.as_deref(), Some("Doom II"));
        assert!(!config.rename_game("Doom II", ""));
    }
}
