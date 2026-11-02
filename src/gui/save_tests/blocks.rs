//! Each Save & apply check (GUI-5), through the window: the problem is made by editing, the save
//! is refused with that check's words and writes nothing, the edits stay in the window, and
//! undoing the problem lets the very same save through.

use super::super::checks::{Names, section_has_problem};
use super::*;
use crate::gui::items::QuickChoice;

/// Makes `problem` in a window on a saved config, and checks the save is refused with `words`,
/// leaves the files as they were and keeps the edit; then `fix` and the save goes through.
/// `tab` is the profile tab that should be flagged, if the problem is in a profile.
#[track_caller]
fn blocks(mut rig: Rig, problem: impl FnOnce(&mut Rig), words: &str, tab: Option<ProfileTab>, fix: impl FnOnce(&mut Rig)) {
    let before = rig.files();
    problem(&mut rig);
    let refused = rig.save().expect_err("the save should have been refused");
    assert!(refused.contains(words), "refused with {refused:?}, expected {words:?}");
    assert_eq!(rig.files(), before, "a refused save wrote something");
    assert_ne!(rig.app.config, rig.app.saved, "the refused edit is still in the window");
    if let Some(tab) = tab {
        let config = &rig.app.config;
        assert!(config.all_games().any(|(_, g)| game_flagged(config, g, tab)), "tab {tab:?} is not flagged");
    }
    fix(&mut rig);
    rig.save().unwrap_or_else(|e| panic!("fixed, but still refused: {e}"));
    rig.assert_round_trip();
}

fn game_flagged(config: &Config, game: &Game, tab: ProfileTab) -> bool {
    let names = Names::for_game(config, game);
    game.profiles.iter().any(|p| section_has_problem(p, tab, &names.for_gyro(p.gyro.enabled)))
}

#[test]
fn an_unknown_key_blocks_the_save() {
    blocks(
        Rig::new(),
        |r| r.send(Message::SetAction(Target::Button(Button::South), keys("KEY_NOPE"))),
        "unknown key",
        Some(ProfileTab::Buttons),
        |r| r.send(Message::Undo),
    );
}

#[test]
fn a_missing_macro_blocks_the_save() {
    blocks(
        Rig::new(),
        |r| r.send(Message::SetAction(Target::Button(Button::South), ButtonAction::Macro { name: "Gone".into(), repeat: false })),
        "missing macro",
        Some(ProfileTab::Buttons),
        |r| r.send(Message::Undo),
    );
}

#[test]
fn an_incomplete_combo_blocks_the_save() {
    blocks(
        Rig::new(),
        |r| {
            r.send(Message::AddCombo);
            r.send(Message::RemoveComboButton(0, Button::RightBumper));
        },
        "at least two buttons",
        Some(ProfileTab::Combos),
        |r| r.send(Message::AddComboButton(0, Button::RightBumper)),
    );
}

#[test]
fn two_combos_of_the_same_buttons_block_the_save() {
    blocks(
        Rig::new(),
        |r| {
            r.send(Message::AddCombo);
            r.send(Message::AddCombo);
            let p = &mut r.app.config.general.profiles[0];
            p.combos[1].buttons = p.combos[0].buttons.iter().rev().copied().collect();
        },
        "two combos are",
        Some(ProfileTab::Combos),
        |r| r.app.config.general.profiles[0].combos.truncate(1),
    );
}

#[test]
fn an_unnamed_macro_blocks_the_save() {
    blocks(Rig::new(), |r| {
        r.send(Message::NewMacro);
        r.send(Message::RenameMacro(0, String::new()));
    }, "A macro needs a name", None, |r| r.send(Message::RenameMacro(0, "Heal".into())));
}

#[test]
fn a_macro_with_a_missing_macro_in_it_blocks_the_save() {
    blocks(
        Rig::new(),
        |r| {
            r.send(Message::NewMacro);
            r.send(Message::AddMacroStep(0, StepKind::Wait));
            r.app.config.general.macros[0].steps.push(MacroStep::Press(ButtonAction::Macro { name: "Gone".into(), repeat: false }));
        },
        "missing macro",
        None,
        |r| {
            r.app.config.general.macros[0].steps.pop();
        },
    );
}

#[test]
fn two_cycles_with_one_name_block_the_save() {
    blocks(
        Rig::new(),
        |r| {
            r.send(Message::NewCycle);
            r.send(Message::NewCycle);
            let name = r.app.config.general.cycles[0].name.clone();
            r.app.config.general.cycles[1].name = name;
        },
        "Two cycles are named",
        None,
        |r| r.app.config.general.cycles[1].name = "Other".into(),
    );
}

#[test]
fn a_menu_item_opening_a_missing_menu_blocks_the_save() {
    blocks(
        Rig::new(),
        |r| {
            r.send(Message::NewMenu(MenuKindTag::List));
            r.send(Message::AddMenuItem(0));
            r.app.config.general.menus[0].items[0].action = ButtonAction::OpenMenu("Gone".into());
        },
        "Gone",
        None,
        |r| r.app.config.general.menus[0].items[0].action = ButtonAction::Disabled,
    );
}

#[test]
fn a_rule_for_a_missing_profile_blocks_the_save() {
    blocks(
        Rig::with_game(),
        |r| r.app.config.games[0].rules[0].profile = "Nope".into(),
        "doesn't exist",
        None,
        |r| r.app.config.games[0].rules[0].profile = "Play".into(),
    );
}

#[test]
fn an_unnamed_setup_blocks_the_save() {
    blocks(Rig::with_game(), |r| r.app.config.games[0].name = String::new(), "A setup needs a name", None, |r| r.app.config.games[0].name = "Doom".into());
}

#[test]
fn a_blocked_save_blocks_only_itself() {
    // Warnings and unrelated state don't refuse: two button-menu items on one button just warn.
    let mut rig = Rig::new();
    rig.send(Message::NewMenu(MenuKindTag::Buttons));
    rig.send(Message::AddMenuItem(0));
    rig.send(Message::AddMenuItem(0));
    for i in 0..2 {
        rig.send(Message::SetMenuItemButton(0, i, QuickChoice(Some(Button::South))));
    }
    rig.save().unwrap();
    // A refused save doesn't stop a later, good one, and the first problem named is the one shown.
    rig.send(Message::SetAction(Target::Button(Button::East), keys("KEY_NOPE")));
    assert!(rig.save().is_err());
    rig.send(Message::SetAction(Target::Button(Button::East), keys("KEY_B")));
    rig.save().unwrap();
    rig.assert_round_trip();
}
