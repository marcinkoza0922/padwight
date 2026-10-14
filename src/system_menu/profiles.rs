//! Switch Profile, opened from the Guide menu: the profiles of the game in front (or General's),
//! and a new one, made from a copy of the current profile, a template or one of General's.

use crate::{
    config::{Config, Feature, ProfileRef, Template, unique_name},
    menu::{ItemView, Tone},
};

/// One row of the profile list.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Row {
    Profile(usize),
    New,
}

/// One row of the new-profile page: what the profile starts from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum NewRow {
    /// A copy of the active profile.
    Copy,
    Template(Template),
    /// A copy of one of General's profiles, offered in a game.
    General(usize),
}

pub(super) fn rows(config: &Config) -> Vec<Row> {
    (0..config.active_game().profiles.len()).map(Row::Profile).chain([Row::New]).collect()
}

pub(super) fn new_rows(config: &Config) -> Vec<NewRow> {
    let general = if std::ptr::eq(config.active_game(), &config.general) { 0 } else { config.general.profiles.len() };
    [NewRow::Copy]
        .into_iter()
        .chain(Template::NEW.into_iter().map(NewRow::Template))
        .chain((0..general).map(NewRow::General))
        .collect()
}

/// A row of the profile list. The active profile is marked, and one the controllers can't play
/// (it needs a gyro, say) says what it needs.
pub(super) fn item(row: Row, config: &Config, have: Option<&[Feature]>) -> ItemView {
    let Row::Profile(i) = row else {
        return ItemView { label: "New profile…".into(), submenu: true, tone: Tone::Add, ..ItemView::plain() };
    };
    let Some(profile) = config.active_game().profiles.get(i) else { return ItemView::plain() };
    let mut label = profile.name.clone();
    if profile.name == config.active_ref().profile {
        label.push_str(" (current)");
    }
    if let Some(have) = have {
        let missing: Vec<&str> = profile.requires.iter().filter(|f| !have.contains(f)).map(|f| f.label()).collect();
        if !missing.is_empty() {
            label.push_str(&format!(" (needs {})", missing.join(", ")));
        }
    }
    ItemView { label, ..ItemView::plain() }
}

pub(super) fn new_item(row: NewRow, config: &Config) -> ItemView {
    let label = match row {
        NewRow::Copy => format!("Copy of {}", config.active_ref().profile),
        NewRow::Template(t) => t.to_string(),
        NewRow::General(i) => format!("Copy of General's {}", config.general.profiles.get(i).map_or("", |p| p.name.as_str())),
    };
    ItemView { label, ..ItemView::plain() }
}

/// The profile a row of the list switches to.
pub(super) fn target(row: Row, config: &Config) -> Option<ProfileRef> {
    let Row::Profile(i) = row else { return None };
    let profile = config.active_game().profiles.get(i)?;
    Some(ProfileRef { profile: profile.name.clone(), ..config.active_ref() })
}

/// Adds a profile made as `row` says to the active game, named so no other profile of the game
/// has its name. Returns where it is.
pub(super) fn create(row: NewRow, config: &mut Config) -> Option<ProfileRef> {
    let (base, mut profile) = match row {
        NewRow::Copy => {
            let current = config.active()?.clone();
            (current.name.clone(), current)
        }
        NewRow::Template(t) => (t.base_name().to_string(), t.make("")),
        NewRow::General(i) => {
            let general = config.general.profiles.get(i)?.clone();
            (general.name.clone(), general)
        }
    };
    let at = config.active_ref();
    let game = config.game_mut(at.game.as_deref())?;
    profile.name = unique_name(&base, |n| game.profile(n).is_some());
    let name = profile.name.clone();
    game.profiles.push(profile);
    Some(ProfileRef { profile: name, ..at })
}

/// Whether `config` has the profile `at`, so a switch to it can be kept without a save.
pub(super) fn has(config: &Config, at: &ProfileRef) -> bool {
    config.profile(at).is_some()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{Game, Profile};

    /// General's Gamepad and Desktop.
    fn two_profiles() -> Config {
        let config = Config::default();
        assert_eq!(config.general.profiles.len(), 2);
        config
    }

    #[test]
    fn the_list_has_every_profile_and_a_new_row_last() {
        let config = two_profiles();
        assert_eq!(rows(&config), vec![Row::Profile(0), Row::Profile(1), Row::New]);
        assert!(item(Row::Profile(0), &config, None).label.ends_with("(current)"));
        assert!(!item(Row::Profile(1), &config, None).label.contains("current"));
    }

    #[test]
    fn a_profile_the_controller_cant_play_says_what_it_needs() {
        let mut config = two_profiles();
        config.general.profiles[1].requires = vec![Feature::Gyro];
        assert!(item(Row::Profile(1), &config, Some(&[])).label.ends_with("(needs gyro)"));
        assert!(!item(Row::Profile(1), &config, Some(&[Feature::Gyro])).label.contains("needs"));
    }

    #[test]
    fn a_copy_gets_a_free_name_in_the_same_game() {
        let mut config = two_profiles();
        let current = config.active_ref().profile;
        let at = create(NewRow::Copy, &mut config).unwrap();
        assert_eq!(at.profile, format!("{current} 2"));
        assert_eq!(config.general.profiles.len(), 3);
        assert_eq!(create(NewRow::Template(Template::Desktop), &mut config).unwrap().profile, "Desktop 2");
        assert_eq!(create(NewRow::Template(Template::Strategy), &mut config).unwrap().profile, "Strategy");
    }

    #[test]
    fn copies_of_general_are_offered_only_in_a_game() {
        let mut config = two_profiles();
        assert!(!new_rows(&config).iter().any(|r| matches!(r, NewRow::General(_))));
        config.games.push(Game::new("Doom", vec![Profile::passthrough("Play")]));
        config.active = ProfileRef::new(Some("Doom"), "Play");
        let general: Vec<_> = new_rows(&config).into_iter().filter(|r| matches!(r, NewRow::General(_))).collect();
        assert_eq!(general.len(), 2);
        let at = create(NewRow::General(1), &mut config).unwrap();
        assert_eq!(at, ProfileRef::new(Some("Doom"), "Desktop"));
    }
}
