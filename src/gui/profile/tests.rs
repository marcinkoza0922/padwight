use super::*;
use crate::gui::tests::*;

#[test]
fn search_keeps_only_matching_buttons() {
    let app = app();
    let names = app.names();
    let mut ui = Ui { names: &names, expanded: &app.expanded, found: app.found, analog_triggers: true, any_gyro: false, any_paddles: false, any_touchpad: false, live_sticks: None, live_gyro: None, gyro_trail: &app.gyro_trail, gyro_note: None, stick_probe: 1.0, family: PadFamily::default(), in_use: PadFamily::default(), swap: Swap::Off, layer: None, menus: Vec::new(), live_touch: None, touchpad_height: 0.5, query: "" };
    let p = app.profile().unwrap();
    assert!(row_matches(p, Button::South, &ui));
    ui.query = "  ";
    assert!(row_matches(p, Button::South, &ui));
    ui.query = "zzzqqq";
    assert!(!row_matches(p, Button::South, &ui));
    assert!(button_rows(p, &ui).is_empty());
}

#[test]
fn trigger_depth_settings_hide_only_when_all_pads_are_digital() {
    let mut app = app();
    let status = |devices| Status { enabled: true, active_profile: "Gamepad".into(), devices, ..Default::default() };
    assert!(app.analog_triggers(), "no daemon: assume analog");

    app.status = Some(status(vec![device("Pro Controller", false, false)]));
    assert!(!app.analog_triggers());

    app.status = Some(status(vec![device("Pro Controller", false, false), device("Xbox", true, false)]));
    assert!(app.analog_triggers());

    // An ignored Xbox pad doesn't count.
    app.status = Some(status(vec![device("Pro Controller", false, false), device("Xbox", true, true)]));
    assert!(!app.analog_triggers());

    app.status = Some(status(vec![]));
    assert!(app.analog_triggers());
}

#[test]
fn renaming_a_profile_updates_rules_default_and_the_active_one() {
    let mut app = with_game();
    app.config.auto_switch.default_profile = Some(ProfileRef::new(Some("Doom"), "Play"));
    // The daemon says Doom › Play is active.
    let status = Status { active_profile: "Play".into(), active_game: Some("Doom".into()), ..Default::default() };
    let _ = app.update(Message::StatusLoaded(Ok(status.clone())));
    let _ = app.update(Message::RenameProfile("Fight".into()));
    assert_eq!(app.game().rules[0].profile, "Fight");
    assert_eq!(app.config.auto_switch.default_profile, Some(ProfileRef::new(Some("Doom"), "Fight")));
    // Until saved, the daemon keeps reporting the old name; it's followed.
    let _ = app.update(Message::StatusLoaded(Ok(status)));
    assert_eq!(app.config.active, ProfileRef::new(Some("Doom"), "Fight"));
    assert_eq!(app.saved.active, ProfileRef::new(Some("Doom"), "Play"));
    assert_eq!(app.validate(), None);

    let _ = app.update(Message::SetDefaultProfile(DefaultChoice(None)));
    assert_eq!(app.config.auto_switch.default_profile, None);
}

#[test]
fn new_profile_from_template_gets_unique_name_and_is_edited() {
    let mut app = app();
    let _ = app.update(Message::AddProfile(Template::Action));
    let _ = app.update(Message::AddProfile(Template::Action));
    let names: Vec<&str> = app.config.general.profiles.iter().map(|p| p.name.as_str()).collect();
    assert_eq!(names, ["Gamepad", "Desktop", "PC Action", "PC Action 2"]);
    assert_eq!(app.profile().unwrap().name, "PC Action 2");
    assert_eq!(app.validate(), None);
}

#[test]
fn stick_directions_are_combo_members_and_outputs_not_action_holders() {
    let mut app = app();
    // Combos accept stick directions as members.
    let _ = app.update(Message::AddCombo);
    let _ = app.update(Message::RemoveComboButton(0, Button::RightBumper));
    let _ = app.update(Message::AddComboButton(0, Button::RightStickRight));
    assert_eq!(app.config.general.profiles[0].combos[0].buttons, [Button::LeftBumper, Button::RightStickRight]);
    // Any button can output a stick direction.
    let _ = app.update(Message::SetAction(Target::Button(Button::South), ButtonAction::Gamepad(Button::RightStickUp)));
    assert_eq!(app.config.general.profiles[0].button(Button::South), &ButtonAction::Gamepad(Button::RightStickUp));
    // Validation still checks the actions that are editable.
    let _ = app.update(Message::SetAction(
        Target::Button(Button::South),
        ButtonAction::Keys(vec!["KEY_NOPE".into()]),
    ));
    assert!(app.validate().unwrap().contains("NOPE"), "{:?}", app.validate());
}

#[test]
fn find_by_pressing_detects_new_presses_and_firm_stick_pushes() {
    let held = snapshot(&[Button::South], (0.0, 0.0));
    assert_eq!(newly_pressed(None, &held), Some(Button::South));
    // Still held from before: not new. A second button is.
    assert_eq!(newly_pressed(Some(&held), &held), None);
    assert_eq!(newly_pressed(Some(&held), &snapshot(&[Button::South, Button::West], (0.0, 0.0))), Some(Button::West));
    // A light touch on a stick doesn't count; a firm push does.
    assert_eq!(newly_pressed(Some(&held), &snapshot(&[Button::South], (0.4, 0.0))), None);
    assert_eq!(newly_pressed(Some(&held), &snapshot(&[Button::South], (0.0, -0.9))), Some(Button::RightStickUp));
}

#[test]
fn the_gyro_pictures_say_when_they_dont_show_the_settings_on_screen() {
    let mut app = app();
    let note = |app: &App| app.gyro_note(app.profile().unwrap(), false);
    assert_eq!(note(&app), None, "the active profile, saved");
    let gyro = GyroConfig { mode: GyroMode::Mouse { sensitivity: 12.0 }, ..GyroConfig::default() };
    let _ = app.update(Message::SetGyro(gyro));
    assert!(note(&app).unwrap().contains("Save & apply"));
    app.saved = app.config.clone();
    assert_eq!(note(&app), None);
    app.editing = 1;
    assert!(note(&app).unwrap().contains("Make this profile active"));
}

#[test]
fn find_by_pressing_opens_and_highlights_the_row() {
    let mut app = app();
    let _ = app.update(Message::StartFind);
    assert!(app.finding);
    let _ = app.update(Message::LiveInput(Some(snapshot(&[Button::West], (0.0, 0.0)))));
    assert!(!app.finding, "one press ends find mode");
    assert_eq!((app.game_tab, app.profile_tab, app.found), (GameTab::Profiles, ProfileTab::Buttons, Some(Button::West)));
    let names = Names::default();
    let ui = Ui { names: &names, expanded: &app.expanded, found: app.found, analog_triggers: true, any_gyro: false, any_paddles: false, any_touchpad: false, live_sticks: None, live_gyro: None, gyro_trail: &app.gyro_trail, gyro_note: None, stick_probe: 1.0, family: PadFamily::default(), in_use: PadFamily::default(), swap: Swap::Off, layer: None, menus: Vec::new(), live_touch: None, touchpad_height: 0.5, query: "" };
    assert!(ui.is_open(Target::Button(Button::West)));
    // Presses while not finding don't move the editor.
    let _ = app.update(Message::LiveInput(Some(snapshot(&[Button::West, Button::North], (0.0, 0.0)))));
    assert_eq!(app.found, Some(Button::West));
    // A stick push jumps to the Sticks & triggers tab.
    let _ = app.update(Message::StartFind);
    let _ = app.update(Message::LiveInput(Some(snapshot(&[], (0.95, 0.0)))));
    assert_eq!((app.profile_tab, app.found), (ProfileTab::Sticks, Some(Button::RightStickRight)));
}

#[test]
fn rows_expand_and_collapse() {
    let mut app = app();
    let a = Target::Button(Button::South);
    let _ = app.update(Message::ToggleExpanded(a));
    assert!(app.expanded.contains(&a));
    let _ = app.update(Message::ToggleExpanded(a));
    assert!(!app.expanded.contains(&a));
    let _ = app.update(Message::ExpandAll(true));
    assert_eq!(app.expanded.len(), Button::ALL.len() + Button::PADDLES.len());
    let _ = app.update(Message::SelectProfileTab(ProfileTab::Sticks));
    // The Sticks tab has no button rows to open, so expanding it changes nothing.
    let _ = app.update(Message::ExpandAll(true));
    assert_eq!(app.expanded.len(), Button::ALL.len() + Button::PADDLES.len());
    let _ = app.update(Message::ExpandAll(false));
    assert_eq!(app.expanded.len(), Button::ALL.len() + Button::PADDLES.len());
    let _ = app.update(Message::SelectProfileTab(ProfileTab::Buttons));
    let _ = app.update(Message::ExpandAll(false));
    assert_eq!(app.expanded.len(), 0, "collapse all only touches the current section");
}

#[test]
fn a_new_combo_takes_buttons_no_other_combo_has() {
    let mut app = app();
    for _ in 0..3 {
        let _ = app.update(Message::AddCombo);
    }
    let combos = &app.profile().unwrap().combos;
    assert_eq!(combos[0].buttons, [Button::LeftBumper, Button::RightBumper]);
    assert_eq!(crate::config::duplicate_combo(combos), None, "{combos:?}");
    assert_eq!(app.validate(), None);
}

#[test]
fn the_pictures_tips_list_a_buttons_press_gestures_and_combos() {
    use crate::config::{Combo, GestureKind};
    let mut p = app().profile().unwrap().clone();
    p.combos = vec![Combo { buttons: vec![Button::LeftBumper, Button::South], action: ButtonAction::Keys(vec!["KEY_R".into()]), window_ms: None }];
    *p.gestures.entry(Button::South).or_default().slot(GestureKind::LongPress) = Some(ButtonAction::Keys(vec!["KEY_E".into()]));
    let tips = drawing_tips(&p);
    // Each line's lead, before its action.
    let tip = |b| {
        let (_, t) = tips.iter().find(|(s, _)| *s == pad_svg::Spot::Button(b))?;
        Some(t.lines().map(|l| l.split(':').next().unwrap_or_default().to_string()).collect::<Vec<_>>())
    };
    assert_eq!(tip(Button::South).unwrap(), ["Press", "Long press", "LB + A"]);
    assert_eq!(tip(Button::LeftBumper).unwrap(), ["Press", "LB + A"], "a combo alone earns a tip");
    assert_eq!(tip(Button::East), None, "a button that does one thing needs none");
}

#[test]
fn a_triggers_on_press_and_on_release_are_added_and_tipped() {
    use crate::config::{GestureKind, Trigger, TriggerAction};
    let mut app = app();
    let _ = app.update(Message::SetTrigger(Trigger::Left, TriggerAction::Button { action: ButtonAction::Layer("Aim".into()), threshold: 0.4 }));
    for kind in GestureKind::EDGES {
        let _ = app.update(Message::AddTriggerGesture(Trigger::Left, kind));
        let _ = app.update(Message::SetAction(Target::TriggerGesture(Trigger::Left, kind), ButtonAction::Mouse(crate::config::MouseButton::Right)));
    }
    let p = app.profile().unwrap().clone();
    assert_eq!(p.left_trigger.gestures.on_release, Some(ButtonAction::Mouse(crate::config::MouseButton::Right)));
    let tips = drawing_tips(&p);
    let (_, tip) = tips.iter().find(|(s, _)| *s == pad_svg::Spot::Trigger(Trigger::Left)).unwrap();
    assert_eq!(tip.lines().map(|l| l.split(':').next().unwrap()).collect::<Vec<_>>(), ["Press", "On press", "On release"]);
    let _ = app.update(Message::RemoveTriggerGesture(Trigger::Left, GestureKind::OnPress));
    assert_eq!(app.profile().unwrap().left_trigger.gestures.on_press, None);
}
