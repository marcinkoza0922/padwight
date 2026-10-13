//! Loading takes only what saving would.

use super::behaviour::scratch_dir;
use crate::config::*;

/// Loading takes only what saving would: a setup it would refuse is left out, its folder kept.
#[test]
fn a_setup_saving_would_refuse_is_left_out_and_kept() {
    let dir = scratch_dir("refused-setup");
    let path = dir.join("config.toml");
    let mut config = Config::default();
    let combo = |action| Combo { buttons: vec![Button::LeftBumper, Button::RightBumper], action };
    let mut doom = Game::new("Doom", vec![Profile::passthrough("Play")]);
    doom.profiles[0].combos = vec![combo(ButtonAction::Screenshot), combo(ButtonAction::ToggleMedia)];
    config.games = vec![doom, Game::new("Quake", vec![Profile::passthrough("Play")])];
    config.save_to(&path).unwrap();

    let loaded = Config::load_from(&path).unwrap();
    assert_eq!(loaded.games.iter().map(|g| g.name.as_str()).collect::<Vec<_>>(), ["Quake"]);
    assert_eq!(loaded.unreadable.len(), 1);
    assert!(loaded.unreadable[0].error.contains("two combos are LB + RB"), "{}", loaded.unreadable[0].error);
    assert!(loaded.problem().is_none(), "what loaded can be saved");
    // A save keeps the folder, and it comes back once fixed: here, still left out.
    loaded.save_to(&path).unwrap();
    assert_eq!(Config::load_from(&path).unwrap().unreadable.len(), 1);

    // The daemon takes the same rule over IPC, through `problem`.
    assert!(config.problem().is_some_and(|p| p.starts_with("Doom: Profile “Play”: two combos")));
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn a_general_saving_would_refuse_is_left_out_and_holds_up_saving() {
    let dir = scratch_dir("refused-general");
    let path = dir.join("config.toml");
    let mut config = Config::default();
    config.general.profiles[0].set_button(Button::North, ButtonAction::Macro { name: "Gone".into(), repeat: false });
    config.save_to(&path).unwrap();
    let loaded = Config::load_from(&path).unwrap();
    assert_eq!(loaded.general, Config::default().general);
    assert!(loaded.unreadable[0].error.contains("missing macro"), "{}", loaded.unreadable[0].error);
    assert!(loaded.save_to(&path).is_err(), "a save would write over General");
    let _ = std::fs::remove_dir_all(&dir);
}
