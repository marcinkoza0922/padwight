use super::*;
use crate::gui::tests::*;

fn opener(app: &App) -> Option<ShortcutInput> {
    app.config.shortcuts.opener()
}

#[test]
fn moving_the_opener_warns_the_setups_that_use_the_input() {
    let mut app = with_game();
    assert!(app.opener_conflicts().is_empty(), "Guide conflicts with nothing");
    let _ = app.update(Message::SetShortcut(Shortcut::OpenGuide, Some(ShortcutInput::button(Button::Start))));
    let conflicts = app.opener_conflicts();
    assert!(conflicts.iter().any(|(setup, _)| setup == "Doom"), "{conflicts:?}");
    assert!(conflicts.iter().any(|(setup, _)| setup == "General"));
    // A gesture nothing uses is clear again.
    let _ = app.update(Message::SetShortcut(Shortcut::OpenGuide, Some(ShortcutInput::gesture(Button::Start, GestureKind::LongPress))));
    assert!(app.opener_conflicts().is_empty());
    let _ = app.update(Message::ResetShortcut(Shortcut::OpenGuide));
    assert!(app.config.shortcuts.is_default());
}

#[test]
fn leaving_the_opener_unset_is_allowed_with_a_warning() {
    let mut app = with_game();
    let _ = app.update(Message::SetShortcut(Shortcut::OpenGuide, None));
    assert_eq!(opener(&app), None);
    assert!(app.config.shortcuts.opener_off());
    assert!(app.config.shortcuts.all_warnings().iter().any(|w| w.contains("Nothing opens")));
    let _ = app.update(Message::SelectPage(Page::Settings));
    let _ = app.update(Message::SelectSettingsTab(SettingsTab::Shortcuts));
    let _ = app.view();
}

#[test]
fn a_shortcut_set_by_pressing_is_its_own_undo_step() {
    let mut app = with_game();
    let _ = app.update(Message::SetAction(Target::Button(Button::South), ButtonAction::Keys(vec!["KEY_A".into()])));
    let mapped = app.config.clone();
    let _ = app.update(Message::ListenShortcut(Some((Shortcut::Tab, 0))));
    let _ = app.update(Message::LiveInput(Some(snapshot(&[Button::Start], (0.0, 0.0)))));
    assert!(app.config.shortcuts.is_moved(Shortcut::Tab));
    let _ = app.update(Message::Undo);
    assert_eq!(app.config, mapped, "undo takes back the shortcut alone");
}

#[test]
fn listening_takes_the_next_press_and_keeps_guide_for_the_opener() {
    let mut app = with_game();
    let _ = app.update(Message::ListenShortcut(Some((Shortcut::Tab, 0))));
    let _ = app.update(Message::LiveInput(Some(snapshot(&[Button::Guide], (0.0, 0.0)))));
    assert!(app.shortcuts_ui.listening.is_some(), "Guide can't be given to a Tab key");
    assert!(!app.config.shortcuts.is_moved(Shortcut::Tab));
    let _ = app.update(Message::LiveInput(Some(snapshot(&[], (0.0, 0.0)))));
    let _ = app.update(Message::LiveInput(Some(snapshot(&[Button::Start], (0.0, 0.0)))));
    assert!(app.shortcuts_ui.listening.is_none());
    assert_eq!(app.config.shortcuts.input(Shortcut::Tab), Some(ShortcutInput::button(Button::Start)));
}

#[test]
fn changing_the_kind_keeps_the_buttons_that_fit() {
    let combo = ShortcutInput::combo(&[Button::Start, Button::Select]);
    assert_eq!(InputKind::DoubleTap.apply(Some(&combo), Button::South), Some(ShortcutInput::gesture(Button::Select, GestureKind::DoubleTap)));
    let button = ShortcutInput::button(Button::East);
    let as_combo = InputKind::Combo.apply(Some(&button), Button::South).unwrap();
    assert!(as_combo.is_combo() && as_combo.uses(Button::East));
    assert_eq!(InputKind::Off.apply(Some(&button), Button::South), None);
    assert_eq!(InputKind::Button.apply(None, Button::South), Some(ShortcutInput::button(Button::South)));
    assert_eq!(InputKind::of(Some(&combo)), InputKind::Combo);
}
