//! What is remembered per physical controller: how to tell it apart, and which profile it uses
//! in each pack. Packs themselves stay shared (see `docs/development/per-controller-setups.md`).

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use super::{Config, Game, Profile};

/// The key a pack's profile choice is stored under for General, which has no name of its own.
const GENERAL_KEY: &str = "";

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ControllerEntry {
    /// See `controller_id`.
    pub id: String,
    /// The name the controller last connected under, to list it while it is away.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub name: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub nickname: String,
    /// `#rrggbb`, for telling pads apart.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub color: Option<String>,
    /// A player number to show for it, and the slot its virtual pad is made in (see `first_pad.rs`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub player: Option<u8>,
    /// Pack (game name; empty for General) -> the profile this pad uses in it.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub profiles: BTreeMap<String, String>,
    /// Hand its input to the game unchanged, whatever the profiles say.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub pass_through: bool,
    /// Seen, but the question of how to set it up hasn't been answered: it passes through until it is.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub pending: bool,
}

impl ControllerEntry {
    /// What to call it: its nickname, else the name it last connected under, else its id.
    pub fn label(&self) -> &str {
        [&self.nickname, &self.name, &self.id].into_iter().find(|s| !s.is_empty()).map_or("", |s| s.as_str())
    }
}

impl Config {
    pub fn controller(&self, id: &str) -> Option<&ControllerEntry> {
        self.controllers.iter().find(|c| c.id == id)
    }

    /// Makes `profile` (of the active game) the one controller `id` uses in it, remembering the
    /// controller if it is new.
    pub fn set_profile_for(&mut self, id: &str, profile: &str) {
        let key = pack_key(self, self.active_game()).to_string();
        let at = match self.controllers.iter().position(|c| c.id == id) {
            Some(at) => at,
            None => {
                self.controllers.push(ControllerEntry { id: id.to_string(), ..ControllerEntry::default() });
                self.controllers.len() - 1
            }
        };
        self.controllers[at].profiles.insert(key, profile.to_string());
    }

    /// Answers the new-controller question for `id`: it follows the active profile, passes
    /// through, or takes the profile choices of the controller `copy`.
    pub fn answer_new_controller(&mut self, id: &str, copy: Option<&str>, pass_through: bool) {
        let profiles = copy.and_then(|from| self.controller(from)).map(|c| c.profiles.clone()).unwrap_or_default();
        if self.controller(id).is_none() {
            self.controllers.push(ControllerEntry { id: id.to_string(), ..ControllerEntry::default() });
        }
        if let Some(entry) = self.controllers.iter_mut().find(|c| c.id == id) {
            entry.profiles = profiles;
            entry.pass_through = pass_through;
            entry.pending = false;
        }
    }

    /// The profile controller `id` uses in the active game: the one its entry names, else the
    /// active profile. A controller with no entry (or an injected one, with no id) follows the
    /// active profile, as before.
    pub fn profile_for(&self, id: &str) -> Option<&Profile> {
        let game = self.active_game();
        if self.controller(id).is_some_and(|c| c.pass_through || c.pending) {
            return Some(passthrough());
        }
        self.controller(id)
            .and_then(|c| c.profiles.get(pack_key(self, game)))
            .and_then(|name| game.profile(name))
            .or_else(|| self.active_fallback())
    }
}

/// What a pad that passes through uses.
fn passthrough() -> &'static Profile {
    static PASS: std::sync::OnceLock<Profile> = std::sync::OnceLock::new();
    PASS.get_or_init(|| Profile::passthrough("Pass through"))
}

fn pack_key<'a>(config: &Config, game: &'a Game) -> &'a str {
    if std::ptr::eq(game, &config.general) { GENERAL_KEY } else { game.name.as_str() }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn config_with(profiles: &[(&str, &str)]) -> Config {
        let mut config = Config::default();
        config.controllers.push(ControllerEntry {
            id: "pad-a".into(),
            profiles: profiles.iter().map(|(k, v)| (k.to_string(), v.to_string())).collect(),
            ..ControllerEntry::default()
        });
        config
    }

    #[test]
    fn a_controller_without_an_entry_follows_the_active_profile() {
        let config = config_with(&[]);
        let active = config.active().map(|p| p.name.clone());
        assert!(active.is_some());
        assert_eq!(config.profile_for("pad-b").map(|p| p.name.clone()), active);
        assert_eq!(config.profile_for("").map(|p| p.name.clone()), active);
    }

    #[test]
    fn its_entry_picks_the_profile_within_the_active_pack() {
        let names: Vec<String> = Config::default().general.profiles.iter().map(|p| p.name.clone()).collect();
        let other = names.iter().find(|n| **n != Config::default().active.profile).expect("a second General profile");
        let config = config_with(&[(GENERAL_KEY, other)]);
        assert_eq!(config.profile_for("pad-a").map(|p| &p.name), Some(other));
        assert_eq!(config.profile_for("pad-b").map(|p| p.name.clone()), config.active().map(|p| p.name.clone()));
    }

    #[test]
    fn a_stale_profile_name_falls_back_to_the_active_one() {
        let config = config_with(&[(GENERAL_KEY, "gone")]);
        assert_eq!(config.profile_for("pad-a").map(|p| p.name.clone()), config.active().map(|p| p.name.clone()));
    }

    #[test]
    fn while_a_controller_acts_the_active_profile_is_its_own() {
        let names: Vec<String> = Config::default().general.profiles.iter().map(|p| p.name.clone()).collect();
        let other = names.iter().find(|n| **n != Config::default().active.profile).expect("a second General profile");
        let mut config = config_with(&[(GENERAL_KEY, other)]);
        config.acting = Some("pad-a".into());
        assert_eq!(config.active().map(|p| &p.name), Some(other));
        assert_eq!(config.active_ref().profile, *other);
        config.acting = Some("pad-b".into());
        assert_eq!(config.active().map(|p| p.name.clone()), config.active_fallback().map(|p| p.name.clone()));
    }

    #[test]
    fn choosing_a_profile_for_a_controller_remembers_it() {
        let mut config = Config::default();
        let other = config.general.profiles.iter().map(|p| p.name.clone()).find(|n| *n != config.active.profile).expect("a second General profile");
        config.set_profile_for("pad-a", &other);
        config.set_profile_for("pad-a", &other);
        assert_eq!(config.controllers.len(), 1);
        assert_eq!(config.profile_for("pad-a").map(|p| p.name.clone()), Some(other));
    }

    #[test]
    fn a_pending_or_passing_controller_gets_the_passthrough_profile() {
        let mut config = config_with(&[]);
        config.controllers[0].pending = true;
        assert_eq!(config.profile_for("pad-a").map(|p| p.name.as_str()), Some("Pass through"));
        config.answer_new_controller("pad-a", None, false);
        assert_eq!(config.profile_for("pad-a").map(|p| p.name.clone()), config.active().map(|p| p.name.clone()));
        config.answer_new_controller("pad-a", None, true);
        assert_eq!(config.profile_for("pad-a").map(|p| p.name.as_str()), Some("Pass through"));
    }

    #[test]
    fn copying_takes_the_other_controllers_choices() {
        let mut config = config_with(&[("Deus Ex", "Gyro aim")]);
        config.answer_new_controller("pad-b", Some("pad-a"), false);
        assert_eq!(config.controller("pad-b").map(|c| c.profiles.clone()), config.controller("pad-a").map(|c| c.profiles.clone()));
    }

    #[test]
    fn a_controller_is_called_by_its_nickname_then_its_name() {
        let mut entry = ControllerEntry { id: "x".into(), ..ControllerEntry::default() };
        assert_eq!(entry.label(), "x");
        entry.name = "DualSense".into();
        assert_eq!(entry.label(), "DualSense");
        entry.nickname = "Couch".into();
        assert_eq!(entry.label(), "Couch");
    }

    #[test]
    fn entries_round_trip_through_toml() {
        let mut config = config_with(&[("Deus Ex", "Gyro aim")]);
        config.controllers[0].nickname = "Couch DualSense".into();
        config.controllers[0].player = Some(2);
        let back: Config = toml::from_str(&toml::to_string_pretty(&config).unwrap()).unwrap();
        assert_eq!(config, back);
    }
}
