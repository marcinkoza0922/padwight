use super::*;
use crate::gui::tests::*;

#[test]
#[expect(clippy::cognitive_complexity, reason = "predates the size lints")]
fn editing_a_layer_records_only_overrides() {
    let mut app = with_game();
    let _ = app.update(Message::SelectGameTab(GameTab::Layers));
    let _ = app.update(Message::NewLayer);
    assert_eq!(app.game().layers[0].name, "Layer");
    assert!(app.editing_layer());
    let base_south = app.game().profiles[0].button(Button::South).clone();

    // Editing a button through the profile editor overrides it in the layer only.
    let f1 = ButtonAction::Keys(vec!["KEY_F1".into()]);
    let _ = app.update(Message::SetAction(Target::Button(Button::South), f1.clone()));
    let layer = &app.game().layers[0];
    assert_eq!(layer.buttons.get(&Button::South), Some(&f1));
    assert_eq!(layer.buttons.len(), 1);
    assert_eq!(app.game().profiles[0].button(Button::South), &base_south);
    assert_eq!(app.profile().unwrap().button(Button::South), &f1, "the editor shows the layer over the profile");

    // A gesture added in the layer, and the key picker writing into the layer.
    let _ = app.update(Message::AddGesture(Button::North, GestureKind::DoubleTap));
    assert!(app.game().layers[0].gestures.get(&Button::North).is_some_and(|g| g.double_tap.is_some()));
    pick(&mut app, KeyField::root(Target::Gesture(Button::North, GestureKind::DoubleTap)), false, &["KEY_F2"]);
    assert_eq!(
        app.game().layers[0].gestures[&Button::North].double_tap,
        Some(ButtonAction::Keys(vec!["KEY_F2".into()]))
    );

    // Override copies the profile's stick; editing it changes the layer's copy.
    let _ = app.update(Message::OverrideInput(LayerPart::Stick(Stick::Right)));
    assert_eq!(app.game().layers[0].right_stick.as_ref(), Some(&app.game().profiles[0].right_stick));
    let scroll = StickConfig::new(StickAction::Scroll { speed: 15.0, invert_y: false }, 0.15, 2.0);
    let _ = app.update(Message::SetStick(Stick::Right, scroll.clone()));
    assert_eq!(app.game().layers[0].right_stick, Some(scroll));
    assert_ne!(app.game().profiles[0].right_stick.action, StickAction::Scroll { speed: 15.0, invert_y: false });
    let _ = app.update(Message::RevertInput(LayerPart::Stick(Stick::Right)));
    let _ = app.update(Message::RevertInput(LayerPart::Button(Button::North)));
    assert!(app.game().layers[0].right_stick.is_none() && app.game().layers[0].gestures.is_empty());

    // Its own combos, and switching off one of the profile's.
    app.config.games[0].profiles[0].combos.push(Combo { buttons: vec![Button::LeftBumper, Button::RightBumper], action: ButtonAction::Disabled, window_ms: None });
    let _ = app.update(Message::AddCombo);
    assert_eq!(app.game().layers[0].combos.len(), 1);
    assert_eq!(app.game().profiles[0].combos.len(), 1);
    let _ = app.update(Message::ToggleBaseCombo(vec![Button::LeftBumper, Button::RightBumper]));
    assert_eq!(app.game().layers[0].disabled_combos, [vec![Button::LeftBumper, Button::RightBumper]]);
    let _ = app.update(Message::RemoveCombo(0));
    assert_eq!(app.validate(), None);
    let _ = app.view();
}

#[test]
fn layer_actions_make_follow_and_check_layers() {
    let mut app = with_game();
    // "+ New layer" in a button's picker makes one and points the button at it.
    let _ = app.update(Message::SetAction(Target::Button(Button::LeftBumper), ButtonAction::Layer(NEW_LAYER.into())));
    assert_eq!(app.game().layers[0].name, "Layer");
    assert_eq!(app.game().profiles[0].button(Button::LeftBumper), &ButtonAction::Layer("Layer".into()));
    // Renaming the layer follows.
    let _ = app.update(Message::SelectGameTab(GameTab::Layers));
    let _ = app.update(Message::RenameLayer("Hotkeys".into()));
    assert_eq!(app.game().profiles[0].button(Button::LeftBumper), &ButtonAction::Layer("Hotkeys".into()));
    assert_eq!(app.validate(), None);

    // A layer can't be toggled: one that stays on is a mode.
    let _ = app.update(Message::SetAction(Target::Button(Button::North), ButtonAction::toggle(ButtonAction::Layer("Hotkeys".into()))));
    assert!(app.validate().unwrap().contains("can't be toggled"), "{:?}", app.validate());
    let _ = app.update(Message::SetAction(Target::Button(Button::North), ButtonAction::Disabled));

    // A menu item can't hold a layer, toggled or not, but it can switch modes.
    let _ = app.update(Message::SelectGameTab(GameTab::Menus));
    let _ = app.update(Message::NewMenu(MenuKindTag::List));
    let _ = app.update(Message::SetAction(Target::MenuItem(0, 0), ButtonAction::Layer("Hotkeys".into())));
    assert!(app.validate().unwrap().contains("can't hold a layer"), "{:?}", app.validate());
    let _ = app.update(Message::SetAction(Target::MenuItem(0, 0), ButtonAction::toggle(ButtonAction::Layer("Hotkeys".into()))));
    assert!(app.validate().unwrap().contains("can't hold a layer"), "{:?}", app.validate());
    let _ = app.update(Message::SetAction(Target::MenuItem(0, 0), ButtonAction::Mode(NEW_MODE.into())));
    assert_eq!(app.game().modes[0].name, "Mode", "+ New mode makes one");
    assert_eq!(app.game().menus[0].items[0].action, ButtonAction::Mode("Mode".into()));
    assert_eq!(app.validate(), None);

    // Deleting it leaves the mappings flagged.
    let _ = app.update(Message::SelectGameTab(GameTab::Layers));
    let _ = app.update(Message::DeleteLayer);
    assert!(app.validate().unwrap().contains("missing layer"), "{:?}", app.validate());
}

#[test]
fn layers_are_copied_from_other_games_with_what_they_use() {
    let mut app = with_game();
    let mut plain = Profile::passthrough("P");
    plain.set_button(Button::Guide, ButtonAction::Gamepad(Button::Guide));
    let mut quake = Game::new("Quake", vec![plain]);
    quake.macros.push(Macro { name: "Lean".into(), steps: vec![MacroStep::Wait(5)] });
    let mut lean = crate::config::Layer::new("Lean");
    lean.buttons.insert(Button::West, ButtonAction::Macro { name: "Lean".into(), repeat: false });
    lean.indicator = crate::config::Indicator::Off;
    quake.layers.push(lean);
    app.config.games.push(quake);
    let _ = app.update(Message::SelectGameTab(GameTab::Layers));
    let _ = app.update(Message::OpenBrowse(ItemKind::Layer));
    let _ = app.update(Message::BrowseFrom(BrowseSource::Game(Some("Quake".into()))));
    let _ = app.view();
    let _ = app.update(Message::CopyItem(0));
    let doom = app.game();
    assert_eq!(doom.layers[0].name, "Lean");
    assert_eq!(doom.macros[0].name, "Lean");
    assert_eq!(app.validate(), None);
}

#[test]
fn the_modes_tab_edits_modes_like_the_layers_tab_edits_layers() {
    let mut app = with_game();
    let _ = app.update(Message::SelectGameTab(GameTab::Modes));
    let _ = app.view();
    let _ = app.update(Message::NewLayer);
    assert_eq!(app.game().modes[0].name, "Mode");
    assert!(app.game().layers.is_empty(), "the Modes tab makes modes");
    assert!(app.editing_layer());

    let f1 = ButtonAction::Keys(vec!["KEY_F1".into()]);
    let _ = app.update(Message::SetAction(Target::Button(Button::South), f1.clone()));
    assert_eq!(app.game().modes[0].buttons.get(&Button::South), Some(&f1));
    assert_ne!(app.game().profiles[0].button(Button::South), &f1, "the profile keeps its own");

    // A button switches to it, and renaming follows.
    let _ = app.update(Message::SelectGameTab(GameTab::Profiles));
    let _ = app.update(Message::SetAction(Target::Button(Button::North), ButtonAction::Mode("Mode".into())));
    let _ = app.update(Message::SelectGameTab(GameTab::Modes));
    let _ = app.update(Message::RenameLayer("Driving".into()));
    assert_eq!(app.game().profiles[0].button(Button::North), &ButtonAction::Mode("Driving".into()));
    assert_eq!(app.validate(), None);
    let _ = app.view();

    // Deleting it leaves the switch flagged.
    let _ = app.update(Message::DeleteLayer);
    assert!(app.game().modes.is_empty());
    assert!(app.validate().unwrap().contains("missing mode"), "{:?}", app.validate());
}
