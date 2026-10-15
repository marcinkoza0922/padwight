//! A menu can't be put in a Toggle: its close button puts it away instead, so a menu button holds
//! its menu up like any other input.

use super::*;
use crate::config::{Combo, Game, Layer, Profile, TriggerAction};
use crate::engine::Engine;

/// A game opening menu "M" from every place an action goes.
fn opened_everywhere() -> Game {
    let mut profile = Profile::passthrough("p");
    profile.set_button(Button::North, open_menu("M"));
    profile.combos.push(Combo { buttons: vec![Button::LeftBumper, Button::RightBumper], action: ButtonAction::Multi(vec![key("KEY_B"), open_menu("M")]) });
    profile.left_trigger = TriggerAction::Button { action: open_menu("M"), threshold: 0.5 }.into();
    let mut game = Game::new("g", vec![profile]);
    let mut layer = Layer::new("L");
    layer.buttons.insert(Button::South, open_menu("M"));
    game.layers.push(layer);
    let mut items = numbers(2);
    items[1].action = open_menu("M");
    game.menus.push(menu("M", MenuKind::List, items));
    game
}

#[test]
fn a_menu_button_holds_its_menu_up_like_any_other() {
    let mut game = opened_everywhere();
    // A grid, as a list with a close button stays up once let go.
    game.menus[0].kind = MenuKind::Grid { columns: 2 };
    let profile = &game.profiles[0];
    let mut engine = Engine::default();
    let mut out = Vec::new();
    engine.handle(profile, InputEvent::Button(Button::North, true), Instant::now(), &mut out);
    let (name, opener) = engine.take_menu_request().expect("North opens the menu");
    assert_eq!((name.as_str(), opener.buttons.as_slice()), ("M", [Button::North].as_slice()), "held by the button, not toggled on");
    let mut pad = Pad::open_with(&game.menus, &name, opener);
    assert_eq!(pad.up(Button::North), Some(MenuOutcome::Close), "letting go puts it away");
    // Pressing it again opens it again: nothing was left toggled on.
    engine.handle(profile, InputEvent::Button(Button::North, false), Instant::now(), &mut out);
    engine.handle(profile, InputEvent::Button(Button::North, true), Instant::now(), &mut out);
    assert!(engine.take_menu_request().is_some());
}
