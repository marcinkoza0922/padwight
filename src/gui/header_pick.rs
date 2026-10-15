//! The header's active-profile pickers: a setup, then one of its profiles, both comboboxes.

use iced::widget::combo_box;

use super::Message;
use crate::config::{Config, ProfileRef};

/// Name the setup list shows for General, which has no name of its own.
const GENERAL: &str = "General";

pub(super) struct HeaderPick {
    setups: combo_box::State<String>,
    profiles: combo_box::State<String>,
    /// What the states were built from, to rebuild them only when it changes.
    setup_names: Vec<String>,
    profile_names: Vec<String>,
    /// The active profile's setup and name, as shown.
    setup: String,
    profile: String,
}

impl HeaderPick {
    pub(super) fn new() -> Self {
        HeaderPick {
            setups: combo_box::State::new(Vec::new()),
            profiles: combo_box::State::new(Vec::new()),
            setup_names: Vec::new(),
            profile_names: Vec::new(),
            setup: String::new(),
            profile: String::new(),
        }
    }

    /// Follows the saved config's setups, profiles and active profile.
    pub(super) fn sync(&mut self, saved: &Config) {
        let setup_names: Vec<String> = saved.all_games().map(|(key, _)| key.unwrap_or(GENERAL).to_string()).collect();
        let at = &saved.active;
        let profile_names: Vec<String> = saved.game(at.game.as_deref()).map(|g| g.profiles.iter().map(|p| p.name.clone()).collect()).unwrap_or_default();
        if setup_names != self.setup_names {
            self.setups = combo_box::State::new(setup_names.clone());
            self.setup_names = setup_names;
        }
        if profile_names != self.profile_names {
            self.profiles = combo_box::State::new(profile_names.clone());
            self.profile_names = profile_names;
        }
        self.setup = at.game.clone().unwrap_or_else(|| GENERAL.to_string());
        self.profile.clone_from(&at.profile);
    }

    pub(super) fn setup_box(&self) -> combo_box::ComboBox<'_, String, Message> {
        combo_box(&self.setups, "Setup", Some(&self.setup), Message::PickSetup)
    }

    pub(super) fn profile_box(&self) -> combo_box::ComboBox<'_, String, Message> {
        combo_box(&self.profiles, "Profile", Some(&self.profile), Message::PickProfile)
    }
}

/// The first profile of a setup picked by name.
pub(super) fn first_of(saved: &Config, setup: &str) -> Option<ProfileRef> {
    let key = (setup != GENERAL).then_some(setup);
    let game = saved.game(key)?;
    Some(ProfileRef::new(key, &game.profiles.first()?.name))
}
