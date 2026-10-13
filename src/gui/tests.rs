//! The app as a whole: shared test helpers, undo, saving and closing.

use super::*;

/// The app on General's page, editing its "Gamepad" profile.
pub(super) fn app() -> App {
    let mut app = App::boot().0;
    let _ = app.update(Message::SelectPage(Page::Game(None)));
    app.editing = 0;
    app
}

/// The app with a game "Doom" (profiles "Play" and "Menus") shown.
pub(super) fn with_game() -> App {
    let mut app = app();
    // Guide plain, so the game starts without the Guide layer; tests add their own layers.
    let mut profiles = vec![Profile::pc_action("Play"), Profile::desktop("Menus")];
    profiles.iter_mut().for_each(|p| p.set_button(Button::Guide, ButtonAction::Gamepad(Button::Guide)));
    let mut game = Game::new("Doom", profiles);
    game.rules.push(Rule::new(RuleKind::Executable, "doom.exe", "Play"));
    app.config.games.push(game);
    let _ = app.update(Message::SelectPage(Page::Game(Some("Doom".into()))));
    app
}

pub(super) fn pick(app: &mut App, field: KeyField, single: bool, keys: &[&'static str]) {
    let _ = app.update(Message::OpenKeyPicker(field, Vec::new(), single));
    for k in keys {
        let _ = app.update(Message::PickerKey(k));
    }
    if !single {
        let _ = app.update(Message::PickerClose { apply: true });
    }
}

#[test]
fn the_picture_holds_for_its_minimum_time_before_switching() {
    let t0 = Instant::now();
    let pick = |model| PadPick { model: Some(model), family: PadFamily::Xbox };
    let ds = pick(PadModel::DualSense);
    let pro = pick(PadModel::ProController);
    let first = settle(None, ds, t0).unwrap();
    assert_eq!(first.pick, ds, "the first controller goes up at once");
    // Another controller used 0.5 s later waits for the hold.
    let early = settle(Some(first), pro, t0 + Duration::from_millis(500)).unwrap();
    assert_eq!(early.pick, ds);
    assert_eq!(early.since, t0, "the hold is measured from when the picture went up");
    // Once the hold has passed, it switches and starts a new hold.
    let late = settle(Some(first), pro, t0 + PAD_HOLD).unwrap();
    assert_eq!((late.pick, late.since), (pro, t0 + PAD_HOLD));
    // A controller that is no longer wanted before the hold ends doesn't cause a switch back.
    assert_eq!(settle(Some(first), ds, t0 + Duration::from_millis(100)), Some(first));
    // A family change alone counts as another controller.
    let generic = PadPick { model: None, family: PadFamily::Nintendo };
    assert_eq!(settle(Some(first), generic, t0 + PAD_HOLD).unwrap().pick, generic);
}

pub(super) fn device(name: &str, analog_triggers: bool, ignored: bool) -> ipc::DeviceInfo {
    ipc::DeviceInfo { name: name.into(), path: String::new(), managed: !ignored, ignored, analog_triggers, rumble: true, gyro: false, paddles: false, touchpad: false, family: None, model: None }
}

pub(super) fn snapshot(buttons: &[Button], right_stick: (f32, f32)) -> InputSnapshot {
    InputSnapshot {
        device: "Pad".into(),
        model: None,
        family: None,
        buttons: buttons.to_vec(),
        left_stick: (0.0, 0.0),
        right_stick,
        left_trigger: 0.0,
        right_trigger: 0.0,
        gyro: None,
        gyro_out: None,
        touches: Vec::new(),
        touch_click: false,
    }
}

#[test]
fn every_page_tab_and_dialog_builds() {
    let mut app = with_game();
    app.config.general.menus.push(Menu { name: "Wheel".into(), kind: MenuKind::List, items: vec![], cancel: None, no_cancel: false, keep_open: false, start: 0, style: OverlayStyle::default() });
    app.config.games[0].info.push(InfoOverlay { name: "Controls".into(), always: true, on_start: None, linger: None, title: None, current_input: Default::default(), style: OverlayStyle::info(), rows: vec![vec!["{south}".into()]] });
    app.config.games[0].macros.push(Macro { name: "Dodge".into(), steps: vec![MacroStep::Wait(5)] });
    app.config.games[0].layers.push(crate::config::Layer::new("Hotkeys"));
    app.config.games[0].modes.push(crate::config::Mode::new("Driving"));
    let status = Status { active_layers: vec!["Hotkeys".into()], active_mode: Some("Driving".into()), ..Default::default() };
    app.status = Some(Status { devices: vec![device("Pad", true, false)], ..status });
    let _ = app.view();
    for page in [Page::Overview, Page::Settings, Page::Manual, Page::Game(None), Page::Game(Some("Doom".into()))] {
        let _ = app.update(Message::SelectPage(page));
        for tab in GameTab::ALL {
            let _ = app.update(Message::SelectGameTab(tab));
            let _ = app.update(Message::ToggleMacro(0));
            let _ = app.update(Message::ToggleMenu(0));
            let _ = app.update(Message::ToggleInfo(0));
            let _ = app.view();
        }
    }
    let _ = app.update(Message::SelectPage(Page::Game(Some("Doom".into()))));
    let text = pack::export(app.game(), &pack::draft(app.game(), false)).pack.to_toml().unwrap();
    let dialogs = [
        Message::OpenAddGame,
        Message::Installed(launchers::Installed::default()),
        Message::PackFileRead(Some(Ok(text))),
        Message::OpenExport,
        Message::AskDeleteGame,
        Message::OpenBrowse(ItemKind::Menu),
        Message::BrowseFrom(BrowseSource::Game(None)),
    ];
    for m in dialogs {
        let _ = app.update(m);
        let _ = app.view();
    }
    let _ = app.update(Message::BrowseOpen(0));
    let _ = app.view();
}

#[test]
fn ctrl_z_takes_back_the_last_edit() {
    let mut app = app();
    let before = app.config.clone();
    let _ = app.update(Message::SetAction(Target::Button(Button::South), ButtonAction::Keys(vec!["KEY_A".into()])));
    assert_ne!(app.config, before);
    let _ = app.update(Message::Undo);
    assert_eq!(app.config, before);
    // Nothing left to undo: a no-op.
    let _ = app.update(Message::Undo);
    assert_eq!(app.config, before);
}

#[test]
fn quick_edits_of_one_kind_undo_together() {
    let mut app = app();
    let before = app.config.clone();
    let _ = app.update(Message::SetAction(Target::Button(Button::South), ButtonAction::Keys(vec!["KEY_A".into()])));
    let _ = app.update(Message::SetAction(Target::Button(Button::East), ButtonAction::Keys(vec!["KEY_B".into()])));
    let _ = app.update(Message::Undo);
    assert_eq!(app.config, before);
}

#[test]
fn closing_with_unsaved_edits_asks_first() {
    let mut app = app();
    let _ = app.update(Message::CloseRequested);
    assert!(app.dialog.is_none(), "nothing unsaved: closes without asking");
    let _ = app.update(Message::SetAction(Target::Button(Button::South), ButtonAction::Keys(vec!["KEY_A".into()])));
    let _ = app.update(Message::CloseRequested);
    assert!(matches!(app.dialog, Some(Dialog::ConfirmQuit)));
    let _ = app.update(Message::CloseDialog);
    assert!(app.dialog.is_none());
}

#[test]
fn a_config_that_failed_to_load_is_never_saved_over() {
    let mut app = app();
    let _ = app.update(Message::ConfigLoaded(Box::default(), Some("parsing setups/001-Doom/setup.toml".into())));
    let _ = app.update(Message::SetAction(Target::Button(Button::South), ButtonAction::Keys(vec!["KEY_A".into()])));
    let _ = app.update(Message::Save);
    assert!(app.message.as_ref().is_some_and(|(m, err)| *err && m.starts_with("Not saved")), "{:?}", app.message);
    // Once it reads, saving works again.
    let _ = app.update(Message::ConfigLoaded(Box::default(), None));
    app.message = None;
    let _ = app.update(Message::Save);
    assert!(!app.message.as_ref().is_some_and(|(m, _)| m.starts_with("Not saved")));
}

#[test]
fn a_blocked_save_can_jump_to_the_turbo_at_fault() {
    let mut app = app();
    app.config.general.macros.push(Macro { name: "Long".into(), steps: vec![MacroStep::Wait(300)] });
    let turbo = ButtonAction::Turbo { action: Box::new(ButtonAction::Macro { name: "Long".into(), repeat: false }), rate: 10.0, every_ms: 50 };
    app.config.general.profiles[0].set_button(Button::West, turbo);
    let _ = app.update(Message::Save);
    assert!(app.message.as_ref().is_some_and(|(m, err)| *err && m.contains("raise the turbo's gap")));
    assert_eq!(app.problem_at.as_ref().map(|p| (p.game.clone(), p.profile, p.target)), Some((None, 0, Target::Button(Button::West))));

    // Somewhere else first, then the jump brings the turbo's row up for editing.
    app.show_game(None);
    app.editing = 0;
    app.expanded.clear();
    let _ = app.update(Message::ShowProblem);
    assert!(app.expanded.contains(&Target::Button(Button::West)));
    assert_eq!(app.profile_tab, ProfileTab::Buttons);
}

#[test]
fn ignoring_a_controller_leaves_other_edits_unsaved() {
    let mut app = app();
    app.status = Some(Status { devices: vec![device("Pad", true, false)], ..Default::default() });
    let _ = app.update(Message::SetAction(Target::Button(Button::South), ButtonAction::Keys(vec!["KEY_A".into()])));
    // Ignoring sends the saved config with only that change, and the reply carries what was sent.
    let _ = app.update(Message::SetIgnored("Pad".into(), true));
    let sent = Box::new(app.saved.clone());
    let _ = app.update(Message::Saved(Ok(sent)));
    assert_eq!(app.saved.ignored_devices, ["Pad"]);
    assert_eq!(app.config.ignored_devices, ["Pad"]);
    assert_ne!(app.config, app.saved, "the key mapping was never sent, so it's still to save");
    assert!(app.saved.general.profiles[0].button(Button::South) != app.config.general.profiles[0].button(Button::South));
}

#[test]
fn an_edit_made_while_saving_stays_unsaved() {
    let mut app = app();
    let _ = app.update(Message::SetAction(Target::Button(Button::South), ButtonAction::Keys(vec!["KEY_A".into()])));
    let _ = app.update(Message::Save);
    let sent = Box::new(app.config.clone());
    let _ = app.update(Message::SetAction(Target::Button(Button::East), ButtonAction::Keys(vec!["KEY_B".into()])));
    let _ = app.update(Message::Saved(Ok(sent)));
    assert_ne!(app.config, app.saved);
    assert_eq!(app.saved.general.profiles[0].button(Button::South), &ButtonAction::Keys(vec!["KEY_A".into()]));
}

#[test]
fn ignoring_a_controller_never_saves_over_an_unreadable_config() {
    let mut app = app();
    let _ = app.update(Message::ConfigLoaded(Box::default(), Some("parsing setups/001-Doom/setup.toml".into())));
    app.status = Some(Status { devices: vec![device("Pad", true, false)], ..Default::default() });
    let _ = app.update(Message::SetIgnored("Pad".into(), true));
    assert!(app.saved.ignored_devices.is_empty() && app.config.ignored_devices.is_empty());
    assert!(app.message.as_ref().is_some_and(|(m, err)| *err && m.starts_with("Not saved")), "{:?}", app.message);
}

#[test]
fn the_notes_editor_follows_the_profile_shown() {
    use iced::widget::text_editor::{Action, Edit};
    let type_x = |app: &mut App| {
        let _ = app.update(Message::Guide(guide_tab::GuideMsg::Notes(Action::Move(iced::widget::text_editor::Motion::DocumentEnd))));
        let _ = app.update(Message::Guide(guide_tab::GuideMsg::Notes(Action::Edit(Edit::Insert('x')))));
    };
    let mut app = with_game();
    app.config.games[0].profiles[0].guide.notes = "Doom tips".into();
    let _ = app.update(Message::SelectProfileTab(ProfileTab::Guide));
    type_x(&mut app);
    assert_eq!(app.game().profiles[0].guide.notes, "Doom tipsx");

    // Another game's page, on the same tab: typing edits that profile's own notes.
    let _ = app.update(Message::SelectPage(Page::Game(None)));
    type_x(&mut app);
    assert_eq!(app.config.general.profiles[0].guide.notes, "x");
    assert_eq!(app.config.games[0].profiles[0].guide.notes, "Doom tipsx");

    // A new profile, and Undo, each show the notes they leave.
    let _ = app.update(Message::AddProfile(Template::Gamepad));
    type_x(&mut app);
    assert_eq!(app.profile().unwrap().guide.notes, "x");
    let _ = app.update(Message::Undo);
    let _ = app.update(Message::Undo);
    assert_eq!(app.guide_notes.text().trim_end(), app.profile().unwrap().guide.notes);
}

#[test]
fn undo_waits_while_a_picker_is_open() {
    let mut app = app();
    let _ = app.update(Message::NewInfo);
    let _ = app.update(Message::OpenTokenPicker(tokens::TokenTarget::InfoCell(0, 0, 0)));
    let _ = app.update(Message::Undo);
    assert_eq!(app.infos().len(), 1, "the overlay the picker writes into is still there");
    let _ = app.update(Message::CloseTokenPicker);
    let _ = app.update(Message::Undo);
    assert!(app.infos().is_empty());
}

#[test]
fn a_pack_file_goes_to_the_setup_it_was_added_from() {
    let mut app = with_game();
    let svg = b"<svg viewBox='0 0 8 8'></svg>".to_vec();
    // The page changed while the file dialog was open.
    let _ = app.update(Message::SelectPage(Page::Game(None)));
    let _ = app.update(Message::PackFilePicked(Some("Doom".into()), pack_files::FileKind::Icon, Some(Ok(("flame".into(), svg.clone())))));
    assert!(app.config.games[0].assets.icons.contains_key("flame"));
    assert!(app.config.general.assets.icons.is_empty());
    // A setup deleted meanwhile gets nothing, and says so.
    let _ = app.update(Message::PackFilePicked(Some("Gone".into()), pack_files::FileKind::Icon, Some(Ok(("leaf".into(), svg)))));
    assert!(app.message.as_ref().is_some_and(|(m, err)| *err && m.contains("leaf")));
}
