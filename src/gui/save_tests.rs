//! Tests that drive the settings window and check what Save & apply writes (see `harness.rs`):
//! one change per editor, saved and compared with the files before; Revert and Undo; and each
//! save check blocking the save it should. Every saved config is also read back and compared with
//! what the window held, so an editor can't save something other than what it showed.

use super::harness::Rig;
use super::*;

mod blocks;

const GAMEPAD: &str = "general/profiles/00001-Gamepad.toml";

fn keys(k: &str) -> ButtonAction {
    ButtonAction::Keys(vec![k.into()])
}

/// Checks a save changed only the files whose path contains one of `files`, and that each
/// fragment in `lines` is among the changed lines.
#[track_caller]
fn assert_diff(diff: &[String], files: &[&str], lines: &[&str]) {
    assert!(!diff.is_empty(), "the save changed nothing");
    for d in diff {
        assert!(files.iter().any(|f| d.contains(f)), "unexpected change: {d}\nall: {diff:#?}");
    }
    for l in lines {
        assert!(diff.iter().any(|d| d.contains(l)), "no change with {l:?}: {diff:#?}");
    }
}

/// An edit through one message, saved: the diff, after checking the files read back as shown.
#[track_caller]
fn saved_after(rig: &mut Rig, messages: Vec<Message>) -> Vec<String> {
    for m in messages {
        rig.send(m);
    }
    let diff = rig.save_diff();
    rig.assert_round_trip();
    diff
}

#[test]
fn changing_a_button_changes_that_line_and_nothing_else() {
    let mut rig = Rig::new();
    let diff = saved_after(&mut rig, vec![Message::SetAction(Target::Button(Button::South), keys("KEY_A"))]);
    assert_eq!(diff, [format!("{GAMEPAD}: - gamepad = \"South\""), format!("{GAMEPAD}: + keys = [\"KEY_A\"]")]);
    // Saving again with no edit writes the same files.
    rig.save().unwrap();
    assert_eq!(rig.save_diff(), Vec::<String>::new());
}

#[test]
fn a_library_setup_added_and_one_button_changed() {
    let mut rig = Rig::new();
    rig.send(Message::PreviewLibrary(0));
    rig.send(Message::ConfirmImport);
    let first = rig.save_diff();
    assert!(!first.is_empty() && first.iter().all(|d| d.starts_with("setups/") && d.contains("(new file)")), "{first:#?}");
    rig.assert_round_trip();
    // One button changed in the new setup: one line in one file.
    let before = rig.app.profile().unwrap().button(Button::South).clone();
    let after = keys("KEY_F13");
    assert_ne!(before, after);
    let diff = saved_after(&mut rig, vec![Message::SetAction(Target::Button(Button::South), after)]);
    assert_eq!(diff.len(), 2, "{diff:#?}");
    assert!(diff.iter().all(|d| d.starts_with("setups/") && d.contains("profiles/")), "{diff:#?}");
    assert!(diff.iter().any(|d| d.contains("+ keys = [\"KEY_F13\"]")), "{diff:#?}");
}

#[test]
fn sticks_save_their_change() {
    let mut rig = Rig::new();
    let stick = rig.app.profile().unwrap().stick(Stick::Left).clone();
    let changed = StickConfig { deadzone: 0.31, ..stick.clone() };
    assert_ne!(stick.deadzone, changed.deadzone);
    let diff = saved_after(&mut rig, vec![Message::SetStick(Stick::Left, changed)]);
    assert_diff(&diff, &[GAMEPAD], &["deadzone = 0.31"]);
    assert_eq!(diff.len(), 2, "{diff:#?}");
}

#[test]
fn triggers_save_their_change() {
    let mut rig = Rig::new();
    let action = TriggerAction::Button { action: keys("KEY_A"), threshold: 0.4 };
    let diff = saved_after(&mut rig, vec![Message::SetTrigger(Trigger::Left, action)]);
    assert_diff(&diff, &[GAMEPAD], &["KEY_A", "0.4"]);
}

#[test]
fn gyro_saves_its_change() {
    let mut rig = Rig::new();
    let gyro = rig.app.profile().unwrap().gyro.clone();
    let diff = saved_after(&mut rig, vec![Message::SetGyro(GyroConfig { invert_y: !gyro.invert_y, ..gyro })]);
    assert_diff(&diff, &[GAMEPAD], &["invert_y"]);
    assert_eq!(diff.len(), 2, "{diff:#?}");
}

#[test]
fn the_touchpad_saves_its_change() {
    let mut rig = Rig::new();
    let touchpad = rig.app.profile().unwrap().touchpad.clone();
    let diff = saved_after(&mut rig, vec![Message::SetTouchpad(TouchpadConfig { click: keys("KEY_B"), ..touchpad })]);
    assert_diff(&diff, &[GAMEPAD], &["KEY_B"]);
}

#[test]
fn combos_save_their_change() {
    let mut rig = Rig::new();
    let diff = saved_after(
        &mut rig,
        vec![
            Message::AddCombo,
            Message::AddComboButton(0, Button::LeftBumper),
            Message::AddComboButton(0, Button::RightBumper),
            Message::SetAction(Target::Combo(0), keys("KEY_C")),
        ],
    );
    assert_diff(&diff, &[GAMEPAD], &["[[combos]]", "KEY_C"]);
    rig.send(Message::RemoveCombo(0));
    let diff = saved_after(&mut rig, vec![]);
    assert_diff(&diff, &[GAMEPAD], &["- [[combos]]", "- keys = [\"KEY_C\"]"]);
}

#[test]
fn layers_and_modes_save_their_change() {
    let mut rig = Rig::with_game();
    let diff = saved_after(&mut rig, vec![Message::SelectGameTab(GameTab::Layers), Message::NewLayer]);
    assert_diff(&diff, &["setups/00001-Doom/"], &["(new file)"]);
    assert!(diff.iter().any(|d| d.contains("layers/")), "{diff:#?}");
    let diff = saved_after(&mut rig, vec![Message::SelectGameTab(GameTab::Modes), Message::NewLayer]);
    assert_diff(&diff, &["setups/00001-Doom/"], &["(new file)"]);
    assert!(diff.iter().any(|d| d.contains("modes/")), "{diff:#?}");
    // A change inside the layer touches that layer's file only.
    let diff = saved_after(&mut rig, vec![Message::SelectGameTab(GameTab::Layers), Message::OverrideInput(LayerPart::Button(Button::South))]);
    assert_diff(&diff, &["layers/"], &[]);
}

#[test]
fn macros_save_their_change() {
    let mut rig = Rig::new();
    let diff = saved_after(&mut rig, vec![Message::NewMacro]);
    assert_diff(&diff, &["general/macros/"], &["(new file)"]);
    let diff = saved_after(&mut rig, vec![Message::AddMacroStep(0, StepKind::Wait)]);
    assert_diff(&diff, &["general/macros/"], &["wait = 100"]);
    let diff = saved_after(&mut rig, vec![Message::DeleteMacro(0)]);
    assert_diff(&diff, &["general/macros/"], &["(removed file)"]);
}

#[test]
fn cycles_save_their_change() {
    let mut rig = Rig::new();
    let diff = saved_after(&mut rig, vec![Message::NewCycle]);
    assert_diff(&diff, &["general/cycles/"], &["(new file)"]);
    let diff = saved_after(&mut rig, vec![Message::AddCycleStep(0)]);
    assert_diff(&diff, &["general/cycles/"], &[]);
    assert_eq!(rig.reloaded().general.cycles[0].steps.len(), 3);
}

#[test]
fn menus_save_their_change() {
    let mut rig = Rig::new();
    let diff = saved_after(&mut rig, vec![Message::NewMenu(MenuKindTag::List)]);
    assert_diff(&diff, &["general/menus/"], &["(new file)"]);
    let diff = saved_after(&mut rig, vec![Message::AddMenuItem(0), Message::SetMenuItemLabel(0, 0, "Heal".into())]);
    assert_diff(&diff, &["general/menus/"], &["Heal"]);
}

#[test]
fn info_and_log_overlays_save_their_change() {
    let mut rig = Rig::new();
    let diff = saved_after(&mut rig, vec![Message::NewInfo]);
    assert_diff(&diff, &["general/info/"], &["(new file)"]);
    let diff = saved_after(&mut rig, vec![Message::SetInfoTitle(0, "Controls".into())]);
    assert_diff(&diff, &["general/info/"], &["Controls"]);
    let diff = saved_after(&mut rig, vec![Message::NewLog]);
    assert_diff(&diff, &["general/logs/"], &["(new file)"]);
    let diff = saved_after(&mut rig, vec![Message::RenameLog(0, "Combos".into())]);
    assert!(!diff.is_empty(), "{diff:#?}");
    assert_eq!(rig.reloaded().general.logs[0].name, "Combos");
}

#[test]
fn revert_puts_the_saved_config_back_exactly() {
    let mut rig = Rig::new();
    let before = rig.files();
    let config = rig.app.config.clone();
    rig.send(Message::SetAction(Target::Button(Button::South), keys("KEY_A")));
    rig.send(Message::NewMacro);
    rig.send(Message::NewMenu(MenuKindTag::Grid));
    assert_ne!(rig.app.config, config);
    rig.send(Message::Revert);
    assert_eq!(rig.app.config, config);
    assert_eq!(rig.app.config, rig.app.saved);
    rig.save().unwrap();
    assert_eq!(rig.files(), before, "saving after Revert writes what was there");
}

#[test]
fn undo_after_a_change_restores_the_config_exactly() {
    let mut rig = Rig::new();
    let before = rig.files();
    let config = rig.app.config.clone();
    for m in [Message::NewCycle, Message::NewInfo, Message::AddCombo] {
        rig.send(m);
        rig.send(Message::Undo);
        assert_eq!(rig.app.config, config);
    }
    rig.send(Message::SetAction(Target::Button(Button::East), keys("KEY_E")));
    rig.send(Message::Undo);
    rig.save().unwrap();
    assert_eq!(rig.files(), before);
    // Undo after a save goes back to before the edit; saving that writes the old lines again.
    rig.send(Message::SetAction(Target::Button(Button::East), keys("KEY_E")));
    rig.save().unwrap();
    rig.send(Message::Undo);
    assert_eq!(rig.app.config, config);
    rig.save().unwrap();
    assert_eq!(rig.files(), before);
}
