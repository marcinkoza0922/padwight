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
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub nickname: String,
    /// `#rrggbb`, for telling pads apart.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub color: Option<String>,
    /// A player number to show for it. Only a label until slots can be pinned.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub player: Option<u8>,
    /// Pack (game name; empty for General) -> the profile this pad uses in it.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub profiles: BTreeMap<String, String>,
}

impl Config {
    pub fn controller(&self, id: &str) -> Option<&ControllerEntry> {
        self.controllers.iter().find(|c| c.id == id)
    }

    /// The profile controller `id` uses in the active game: the one its entry names, else the
    /// active profile. A controller with no entry (or an injected one, with no id) follows the
    /// active profile, as before.
    pub fn profile_for(&self, id: &str) -> Option<&Profile> {
        let game = self.active_game();
        self.controller(id)
            .and_then(|c| c.profiles.get(pack_key(self, game)))
            .and_then(|name| game.profile(name))
            .or_else(|| self.active())
    }
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
    fn entries_round_trip_through_toml() {
        let mut config = config_with(&[("Deus Ex", "Gyro aim")]);
        config.controllers[0].nickname = "Couch DualSense".into();
        config.controllers[0].player = Some(2);
        let back: Config = toml::from_str(&toml::to_string_pretty(&config).unwrap()).unwrap();
        assert_eq!(config, back);
    }
}
