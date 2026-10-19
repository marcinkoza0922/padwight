//! Edit Controls' pages for the profile's and the game's own settings, looks, sounds and motion.

use super::*;

/// A config whose active profile is `Play` of the game `Doom`.
fn doom() -> Config {
    let mut config = config();
    let mut game = crate::config::Game::new("Doom", vec![Profile::passthrough("Play"), Profile::passthrough("Menus")]);
    game.rules.push(crate::config::Rule::new(crate::config::RuleKind::Executable, "doom.exe", "Play"));
    config.games.push(game);
    config.active = crate::config::ProfileRef::new(Some("Doom"), "Play");
    config
}

#[test]
fn the_profile_and_the_game_are_renamed_and_stay_in_use() {
    let mut config = doom();
    let mut editor = LayoutEditor::new();
    editor.pick("Profile: Play", &mut config);
    assert!(matches!(editor.pick("Name", &mut config), EditStep::Text { .. }));
    assert_eq!(editor.set_text(&mut config, "Campaign"), EditStep::Changed);
    assert_eq!(config.active_ref().profile, "Campaign");
    assert_eq!(config.games[0].rules[0].profile, "Campaign");
    editor.back();
    editor.pick("Game: Doom", &mut config);
    editor.pick("Name", &mut config);
    editor.set_text(&mut config, "Doom II");
    assert_eq!(config.active_ref().game.as_deref(), Some("Doom II"));
    assert_eq!(editor.view(&config, &[]).title, "Doom II");
}

#[test]
fn window_rules_are_added_changed_and_deleted() {
    let mut config = doom();
    let mut editor = LayoutEditor::new();
    editor.pick("Game", &mut config);
    editor.pick("Window rules", &mut config);
    editor.pick("Add a rule", &mut config);
    editor.set_text(&mut config, "doom2.exe");
    assert_eq!(config.games[0].rules[1].value, "doom2.exe");
    editor.pick("Executable: doom2.exe", &mut config);
    editor.step_row("Switches to", 1, &mut config);
    assert_eq!(config.games[0].rules[1].profile, "Menus");
    editor.pick("Delete rule", &mut config);
    editor.choose(&mut config);
    assert_eq!(config.games[0].rules.len(), 1);
}

#[test]
fn deleting_the_profile_in_use_moves_to_another_and_its_rules_follow() {
    let mut config = doom();
    let mut editor = LayoutEditor::new();
    editor.pick("Game", &mut config);
    editor.pick("Profiles", &mut config);
    editor.pick("Play (current)", &mut config);
    editor.pick("Delete profile", &mut config);
    assert_eq!(editor.choose(&mut config), EditStep::Changed);
    assert_eq!(config.active_ref().profile, "Menus");
    assert_eq!(config.games[0].rules[0].profile, "Menus");
    editor.pick("Menus (current)", &mut config);
    assert!(!editor.labels(&config).iter().any(|l| l == "Delete profile"), "the last profile stays");
}

#[test]
fn a_log_overlay_is_added_and_leaves_out_a_group_of_inputs() {
    let mut config = config();
    let mut editor = LayoutEditor::new();
    editor.pick("Log overlays", &mut config);
    editor.pick("Add log overlay", &mut config);
    editor.pick("What it follows", &mut config);
    let (group, members) = crate::config::INPUT_GROUPS[0];
    editor.pick(group, &mut config);
    let crate::config::LogSource::Input(s) = &config.general.logs[0].source;
    assert!(members.iter().all(|m| !s.tracking.tracks(*m)));
}

#[test]
fn guide_notes_are_written_line_by_line_and_a_mapping_row_is_hidden() {
    let mut config = config();
    config.general.profiles[0].set_button(Button::North, ButtonAction::Keys(vec!["KEY_R".into()]));
    let mut editor = LayoutEditor::new();
    editor.pick("Guide overlay", &mut config);
    editor.pick("Notes", &mut config);
    editor.pick("Add a line", &mut config);
    editor.set_text(&mut config, "Hold LB for hotkeys");
    editor.pick("Add a line", &mut config);
    editor.set_text(&mut config, "Y reloads");
    assert_eq!(config.active().unwrap().guide.notes, "Hold LB for hotkeys\nY reloads");
    editor.back();
    editor.pick("Mappings", &mut config);
    editor.cursor = 0;
    editor.choose(&mut config);
    editor.pick("Shown", &mut config);
    assert!(config.active().unwrap().guide.edits.values().any(|e| e.hidden));
}

#[test]
fn a_game_gets_a_look_of_its_own_and_gives_it_back() {
    let mut config = doom();
    let mut editor = LayoutEditor::new();
    editor.pick("Game", &mut config);
    editor.pick("In-game menu look", &mut config);
    editor.pick("Give it a look of its own", &mut config);
    assert!(config.games[0].menu_style.is_some());
    editor.pick("Background", &mut config);
    editor.pick("Teal", &mut config);
    assert_eq!(config.games[0].menu_style.as_ref().unwrap().background.color, "#1f7a7a");
    editor.back();
    editor.pick("Decorations", &mut config);
    editor.pick("Scanlines: off", &mut config);
    editor.pick("On", &mut config);
    assert!(config.games[0].menu_style.as_ref().unwrap().ornament.scanlines.is_some());
    editor.back();
    editor.back();
    editor.pick("Use the look from App settings", &mut config);
    assert!(config.games[0].menu_style.is_none());
}

#[test]
fn a_color_is_typed_as_hex_and_a_bad_one_is_refused() {
    let mut config = config();
    config.general.menus.push(crate::config::Menu {
        name: "M".into(),
        kind: crate::config::MenuKind::List,
        items: Vec::new(),
        cancel: None,
        no_cancel: false,
        keep_open: false,
        start: 0,
        style: crate::config::OverlayStyle::default(),
    });
    let mut editor = LayoutEditor::new();
    editor.pick("Menus", &mut config);
    editor.pick("M ·", &mut config);
    editor.pick("Look", &mut config);
    editor.pick("Items", &mut config);
    editor.pick("Type a color", &mut config);
    assert_eq!(editor.set_text(&mut config, "12ab34"), EditStep::Changed);
    assert_eq!(config.general.menus[0].style.items.color, "#12ab34");
    assert_eq!(editor.set_text(&mut config, "nope"), EditStep::Stay);
}

#[test]
fn a_game_has_its_own_sounds_and_motion() {
    let mut config = doom();
    let mut editor = LayoutEditor::new();
    editor.pick("Game", &mut config);
    editor.pick("Sounds", &mut config);
    editor.pick("Its own sounds", &mut config);
    editor.pick("Menus", &mut config);
    editor.pick("Pick", &mut config);
    editor.step_row("Volume", 1, &mut config);
    let sounds = config.games[0].sounds.unwrap();
    assert!(sounds.menu.pick.volume > config.sounds.menu.pick.volume);
    let mut editor = LayoutEditor::new();
    editor.pick("Game", &mut config);
    editor.pick("Motion", &mut config);
    editor.pick("Its own motion", &mut config);
    editor.step_row("All of them", 1, &mut config);
    assert!(config.games[0].motion.is_some());
}
