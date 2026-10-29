use super::*;
use crate::config::Gestures;

fn moved(pairs: &[(Shortcut, Option<ShortcutInput>)]) -> Shortcuts {
    let mut s = Shortcuts::default();
    for (shortcut, input) in pairs {
        s.set(*shortcut, input.clone());
    }
    s
}

#[test]
fn nothing_moved_builds_the_fixed_modes() {
    let s = Shortcuts::default();
    assert_eq!(Layer::guide_with(&s), Layer::guide());
    assert_eq!(Layer::mouse_with(&s, MouseModeExtras::default()), Layer::mouse());
    assert_eq!(s.engine_opener(), GuideOpener::Guide);
    assert!(s.all_warnings().is_empty());
}

#[test]
fn a_moved_shortcut_leaves_its_old_button_and_takes_a_gesture_or_a_combo() {
    let s = moved(&[
        (Shortcut::Screenshot, Some(ShortcutInput::gesture(Button::Start, GestureKind::DoubleTap))),
        (Shortcut::Keyboard, Some(ShortcutInput::combo(&[Button::Start, Button::East]))),
        (Shortcut::Media, None),
    ]);
    let guide = Layer::guide_with(&s);
    assert!(!guide.buttons.contains_key(&Button::RightBumper) && !guide.buttons.contains_key(&Button::LeftBumper));
    assert!(!guide.buttons.contains_key(&Button::West), "the keyboard left X");
    assert_eq!(guide.gestures[&Button::Start].double_tap, Some(ButtonAction::Screenshot));
    assert_eq!(guide.combos, vec![Combo { buttons: super::super::combo_key(&[Button::Start, Button::East]), action: ButtonAction::ToggleOverlay, window_ms: None }]);
}

#[test]
fn mouse_mode_entry_is_a_mode_switch_in_the_guide_mode() {
    let guide = Layer::guide_with(&Shortcuts::default());
    assert_eq!(guide.buttons[&Button::RightStick], ButtonAction::Mode(MOUSE_MODE.into()));
}

#[test]
fn back_on_its_default_is_not_moved() {
    let mut s = moved(&[(Shortcut::Tab, Some(ShortcutInput::button(Button::Start)))]);
    assert!(s.is_moved(Shortcut::Tab) && !s.is_default());
    s.set(Shortcut::Tab, Some(Shortcut::Tab.default_input()));
    assert!(s.is_default());
}

#[test]
fn guide_is_only_ever_the_opener() {
    let s = moved(&[
        (Shortcut::Tab, Some(ShortcutInput::button(Button::Guide))),
        (Shortcut::OpenGuide, Some(ShortcutInput::combo(&[Button::Guide, Button::Start]))),
        (Shortcut::Enter, Some(ShortcutInput::button(Button::Start))),
    ])
    .sanitized();
    assert!(!s.is_moved(Shortcut::Tab) && !s.is_moved(Shortcut::OpenGuide));
    assert!(s.is_moved(Shortcut::Enter));
}

#[test]
fn the_opener_has_three_forms_and_an_off_state() {
    let opener = |i: Option<ShortcutInput>| moved(&[(Shortcut::OpenGuide, i)]).engine_opener();
    assert_eq!(opener(Some(ShortcutInput::button(Button::Select))), GuideOpener::Button(Button::Select));
    assert_eq!(
        opener(Some(ShortcutInput::combo(&[Button::Start, Button::Select]))),
        GuideOpener::Triggered(super::super::combo_key(&[Button::Start, Button::Select]))
    );
    assert_eq!(opener(Some(ShortcutInput::gesture(Button::Start, GestureKind::LongPress))), GuideOpener::Triggered(vec![Button::Start]));
    assert_eq!(opener(None), GuideOpener::Triggered(Vec::new()));
    assert!(moved(&[(Shortcut::OpenGuide, None)]).opener_off());
}

#[test]
fn injecting_puts_the_opener_on_the_profile() {
    let s = moved(&[(Shortcut::OpenGuide, Some(ShortcutInput::combo(&[Button::Start, Button::Select])))]);
    let mut p = Profile::passthrough("p");
    p.combos.push(Combo { buttons: vec![Button::Select, Button::Start], action: ButtonAction::Disabled, window_ms: None });
    s.inject(&mut p);
    assert_eq!(p.combos.len(), 1, "the profile's own combo of them is replaced");
    assert_eq!(p.combos[0].action, ButtonAction::Mode(OPEN_GUIDE.into()));
    let s = moved(&[(Shortcut::OpenGuide, Some(ShortcutInput::gesture(Button::Select, GestureKind::DoubleTap)))]);
    s.inject(&mut p);
    assert_eq!(p.gestures[&Button::Select].double_tap, Some(ButtonAction::Mode(OPEN_GUIDE.into())));
}

#[test]
fn shortcuts_that_share_an_input_warn_each_other() {
    let s = moved(&[(Shortcut::Tab, Some(ShortcutInput::button(Button::West)))]);
    assert!(s.warnings(Shortcut::Tab).iter().any(|w| w.contains("Ctrl")), "Mouse mode's Ctrl is on X too");
    assert!(s.warnings(Shortcut::Keyboard).is_empty(), "the Guide mode is another mode");
    assert!(!s.all_warnings().is_empty());
}

#[test]
fn a_held_opener_button_cannot_also_be_a_shortcut() {
    let s = moved(&[(Shortcut::OpenGuide, Some(ShortcutInput::button(Button::Select)))]);
    assert!(s.warnings(Shortcut::ForceQuit).iter().any(|w| w.contains("Open the Guide overlay")));
    assert!(!s.warnings(Shortcut::OpenGuide).is_empty());
    let s = moved(&[(Shortcut::OpenGuide, Some(ShortcutInput::combo(&[Button::Start, Button::Select])))]);
    assert!(!s.warnings(Shortcut::ForceQuit).is_empty(), "Select is part of the combo");
    assert!(s.warnings(Shortcut::Keyboard).is_empty(), "X is not");
    assert!(moved(&[(Shortcut::OpenGuide, Some(ShortcutInput::button(Button::Start)))]).warnings(Shortcut::Shift).is_empty());
}

#[test]
fn a_profile_that_uses_the_openers_input_conflicts() {
    let p = Profile::passthrough("p");
    let conflict = |i: ShortcutInput| moved(&[(Shortcut::OpenGuide, Some(i))]).conflict(&p);
    assert!(Shortcuts::default().conflict(&p).is_none(), "Guide never conflicts");
    assert!(conflict(ShortcutInput::button(Button::Start)).is_some());
    assert!(conflict(ShortcutInput::gesture(Button::Start, GestureKind::DoubleTap)).is_none(), "no gesture there");
    let mut with_gesture = p.clone();
    with_gesture.gestures.insert(Button::Start, Gestures { double_tap: Some(ButtonAction::Disabled), ..Gestures::default() });
    let s = moved(&[(Shortcut::OpenGuide, Some(ShortcutInput::gesture(Button::Start, GestureKind::DoubleTap)))]);
    assert!(s.conflict(&with_gesture).is_some());
    let s = moved(&[(Shortcut::OpenGuide, Some(ShortcutInput::combo(&[Button::Start, Button::Select])))]);
    assert!(s.conflict(&p).is_none());
    let mut with_combo = p;
    with_combo.combos.push(Combo { buttons: vec![Button::Select, Button::Start], action: ButtonAction::Disabled, window_ms: None });
    assert!(s.conflict(&with_combo).is_some());
}

#[test]
fn shortcuts_survive_the_config_file() {
    let s = moved(&[
        (Shortcut::OpenGuide, Some(ShortcutInput::combo(&[Button::Start, Button::Select]))),
        (Shortcut::Escape, None),
        (Shortcut::Enter, Some(ShortcutInput::gesture(Button::East, GestureKind::LongPress))),
    ]);
    let text = toml::to_string(&s).unwrap();
    assert_eq!(toml::from_str::<Shortcuts>(&text).unwrap(), s, "{text}");
    assert_eq!(toml::to_string(&Shortcuts::default()).unwrap().trim(), "");
}

#[test]
fn the_menu_buttons_trade_places_with_a_and_b() {
    let keys = |confirm, back| MenuKeys { confirm, back };
    let default = keys(Button::South, Button::East);
    assert_eq!([Button::South, Button::East, Button::Select].map(|b| default.logical(b, true)), [Button::South, Button::East, Button::Select]);
    let swapped = keys(Button::East, Button::South);
    assert_eq!((swapped.logical(Button::East, true), swapped.logical(Button::South, true)), (Button::South, Button::East));
    let moved = keys(Button::Select, Button::LeftStick);
    assert_eq!(moved.logical(Button::Select, true), Button::South);
    assert_eq!(moved.logical(Button::South, true), Button::Select, "A takes the place of Select");
    assert_eq!(moved.logical(Button::LeftStick, true), Button::East);
    assert_eq!(moved.logical(Button::East, true), Button::LeftStick);
    assert_eq!(moved.logical(Button::LeftStick, false), Button::LeftStick, "back isn't in use");
    assert_eq!(moved.logical(Button::West, true), Button::West, "nothing else moves");
}

#[test]
fn the_menu_buttons_stay_among_the_free_buttons_and_are_never_off() {
    let s = moved(&[
        (Shortcut::MenuConfirm, Some(ShortcutInput::button(Button::DpadUp))),
        (Shortcut::MenuBack, Some(ShortcutInput::gesture(Button::Select, GestureKind::DoubleTap))),
    ])
    .sanitized();
    assert_eq!(s.menu_keys(), MenuKeys::default(), "the D-pad moves the menus; a gesture isn't a button");
    let mut s = moved(&[(Shortcut::MenuConfirm, Some(ShortcutInput::button(Button::Select)))]);
    assert_eq!(s.menu_keys().confirm, Button::Select);
    s.set(Shortcut::MenuConfirm, None);
    assert_eq!(s.menu_keys(), MenuKeys::default());
}

#[test]
fn the_menu_buttons_warn_when_they_share_or_meet_a_guide_shortcut() {
    let s = moved(&[(Shortcut::MenuConfirm, Some(ShortcutInput::button(Button::East)))]);
    assert!(!s.warnings(Shortcut::MenuBack).is_empty(), "both on B");
    let s = moved(&[(Shortcut::MenuConfirm, Some(ShortcutInput::button(Button::Select)))]);
    assert!(s.warnings(Shortcut::MenuConfirm).iter().any(|w| w.contains("Force quit")), "Select is the force quit");
    let s = moved(&[(Shortcut::OpenGuide, Some(ShortcutInput::button(Button::Select)))]);
    assert!(Shortcuts::default().all_warnings().is_empty() && !s.warnings(Shortcut::OpenGuide).is_empty());
}

#[test]
fn panel_keys_trade_places_within_their_panel() {
    let s = moved(&[(Shortcut::MediaPlay, Some(ShortcutInput::button(Button::LeftBumper)))]);
    assert_eq!(s.button(Shortcut::MediaPlay), Button::LeftBumper);
    assert_eq!(s.button(Shortcut::MediaPrevious), Button::South, "the key it took moved to its old button");
    assert_eq!(s.panel_press(Group::Media, Button::LeftBumper), PanelPress::Key(Button::South));
    assert_eq!(s.panel_press(Group::Media, Button::South), PanelPress::Key(Button::LeftBumper));
    assert_eq!(s.panel_press(Group::Media, Button::North), PanelPress::Key(Button::North), "untouched keys stay put");
    assert_eq!(s.panel_press(Group::Media, Button::DpadUp), PanelPress::Other);
}

#[test]
fn a_free_button_takes_the_key_and_the_old_one_does_nothing() {
    let s = moved(&[(Shortcut::MediaMute, Some(ShortcutInput::button(Button::Select)))]);
    assert_eq!(s.panel_press(Group::Media, Button::Select), PanelPress::Key(Button::West));
    assert_eq!(s.panel_press(Group::Media, Button::West), PanelPress::Ignored);
    assert_eq!(s.panel_press(Group::Numpad, Button::West), PanelPress::Key(Button::West), "numpad's backspace is its own");
}

#[test]
fn the_numpad_and_the_media_controls_trade_among_their_own_keys() {
    let s = moved(&[(Shortcut::NumpadBackspace, Some(ShortcutInput::button(Button::LeftBumper)))]);
    assert_eq!(s.button(Shortcut::NumpadBackspace), Button::LeftBumper);
    assert_eq!(s.button(Shortcut::NumpadEnter), Button::Start, "enter doesn't move");
    assert_eq!(s.button(Shortcut::MediaPrevious), Button::LeftBumper, "media keeps its own buttons");
    assert_eq!(s.button(Shortcut::MediaMute), Button::West);
    assert_eq!(s.panel_press(Group::Numpad, Button::LeftBumper), PanelPress::Key(Button::West));
    assert_eq!(s.panel_press(Group::Numpad, Button::West), PanelPress::Ignored);
}

#[test]
fn a_panel_key_is_always_on_a_button_and_back_on_its_default_is_unmoved() {
    let mut s = Shortcuts::default();
    s.set(Shortcut::MediaNext, None);
    assert_eq!(s, Shortcuts::default(), "off does nothing for a panel key");
    s.set(Shortcut::MediaNext, Some(ShortcutInput::button(Button::LeftBumper)));
    assert!(s.is_moved(Shortcut::MediaNext) && s.is_moved(Shortcut::MediaPrevious));
    s.set(Shortcut::MediaNext, Some(ShortcutInput::button(Button::RightBumper)));
    assert!(s.is_default(), "trading back to the defaults unmoves both");
}

#[test]
fn panel_keys_keep_to_their_choices() {
    let s = moved(&[
        (Shortcut::MediaPlay, Some(ShortcutInput::button(Button::DpadUp))),
        (Shortcut::MediaMute, Some(ShortcutInput::button(Button::Guide))),
        (Shortcut::NumpadEnter, Some(ShortcutInput::gesture(Button::Select, GestureKind::DoubleTap))),
    ])
    .sanitized();
    assert!(s.is_default(), "the D-pad, Guide and gestures aren't panel keys: {s:?}");
}

#[test]
fn the_menu_buttons_take_any_button_but_the_dpad_and_guide() {
    let s = moved(&[
        (Shortcut::MenuConfirm, Some(ShortcutInput::button(Button::Start))),
        (Shortcut::MenuBack, Some(ShortcutInput::button(Button::North))),
    ]);
    assert_eq!(s.menu_keys(), MenuKeys { confirm: Button::Start, back: Button::North });
    for b in [Button::DpadUp, Button::Guide] {
        assert_eq!(moved(&[(Shortcut::MenuBack, Some(ShortcutInput::button(b)))]).sanitized().menu_keys(), MenuKeys::default(), "{b:?}");
    }
}

#[test]
fn the_keyboard_keys_trade_among_themselves_and_leave_the_numpad_alone() {
    let s = moved(&[(Shortcut::KeyboardSpace, Some(ShortcutInput::button(Button::LeftStick)))]);
    assert_eq!(s.panel_press(Group::Keyboard, Button::LeftStick), PanelPress::Key(Button::North));
    assert_eq!(s.panel_press(Group::Keyboard, Button::North), PanelPress::Ignored, "the space's old button is free");
    assert_eq!(s.panel_press(Group::Numpad, Button::North), PanelPress::Other, "the numpad has no North key");
    let s = moved(&[(Shortcut::KeyboardLeft, Some(ShortcutInput::button(Button::RightBumper)))]);
    assert_eq!(s.button(Shortcut::KeyboardRight), Button::LeftBumper, "the cursor keys trade");
}

#[test]
fn an_input_listing_a_button_twice_is_tidied_when_loaded() {
    let text = "[open_guide]\nbuttons = [\"Start\", \"Select\", \"Start\"]\n";
    let shortcuts: Shortcuts = toml::from_str::<Shortcuts>(text).unwrap().sanitized();
    assert_eq!(shortcuts.opener().unwrap().buttons, [Button::Start, Button::Select]);
}
