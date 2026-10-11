//! A menu can't be put in a Toggle: its close button puts it away instead. Toggled menus in old
//! configs and packs come out of the Toggle as they load, and then open like any other.

use super::*;
use crate::config::{Combo, Config, Game, Layer, Profile, TriggerAction};
use crate::engine::Engine;

fn toggled(action: ButtonAction) -> ButtonAction {
    ButtonAction::toggle(action)
}

fn scratch_dir(name: &str) -> std::path::PathBuf {
    let dir = std::env::temp_dir().join(format!("padwight-suite-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

/// A game opening menu "M" through a Toggle in every place an action goes, and a toggled key
/// that should stay as it is.
fn toggled_everywhere() -> Game {
    let mut profile = Profile::passthrough("p");
    profile.set_button(Button::North, toggled(open_menu("M")));
    profile.set_button(Button::West, toggled(key("KEY_A")));
    profile.combos.push(Combo { buttons: vec![Button::LeftBumper, Button::RightBumper], action: ButtonAction::Multi(vec![key("KEY_B"), toggled(open_menu("M"))]) });
    profile.left_trigger = TriggerAction::Button { action: toggled(open_menu("M")), threshold: 0.5 }.into();
    let mut game = Game::new("g", vec![profile]);
    let mut layer = Layer::new("L");
    layer.buttons.insert(Button::South, toggled(open_menu("M")));
    game.layers.push(layer);
    let mut items = numbers(2);
    items[1].action = toggled(open_menu("M"));
    game.menus.push(menu("M", MenuKind::List, items));
    game
}

fn assert_untoggled(game: &Game) {
    let p = &game.profiles[0];
    assert_eq!(p.buttons[&Button::North], open_menu("M"), "a button");
    assert!(matches!(p.buttons[&Button::West], ButtonAction::Toggle(_)), "other toggles stay");
    assert_eq!(p.combos[0].action, ButtonAction::Multi(vec![key("KEY_B"), open_menu("M")]), "inside a Multi on a combo");
    assert!(matches!(&p.left_trigger.action, TriggerAction::Button { action, .. } if *action == open_menu("M")), "a trigger");
    assert_eq!(game.layers[0].buttons[&Button::South], open_menu("M"), "a layer");
    assert_eq!(game.menus[0].items[1].action, open_menu("M"), "a menu item");
}

#[test]
fn a_saved_config_with_toggled_menus_loads_with_them_untoggled() {
    let dir = scratch_dir("untoggle");
    let path = dir.join("config.toml");
    let mut config = Config::default();
    let game = toggled_everywhere();
    config.general.profiles = game.profiles;
    config.general.layers = game.layers;
    config.general.menus = game.menus;
    config.games.push(toggled_everywhere());
    config.save_to(&path).unwrap();
    let back = Config::load_from(&path).unwrap();
    assert_untoggled(&back.general);
    assert_untoggled(&back.games[0]);
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn an_imported_pack_with_toggled_menus_comes_in_untoggled() {
    let game = toggled_everywhere();
    let text = crate::pack::export(&game, &crate::pack::draft(&game, false)).pack.to_toml().unwrap();
    assert!(text.contains("toggle"), "the pack really has the toggles");
    let pack = crate::pack::parse(&text).unwrap();
    assert_untoggled(&pack.to_game());
}

#[test]
fn a_menu_button_holds_its_menu_up_like_any_other_once_untoggled() {
    let mut game = toggled_everywhere();
    game.untoggle_menus();
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

#[test]
fn the_controller_layout_editor_wont_wrap_a_menu_in_a_toggle() {
    use crate::layout_editor::buttons::{ActionRow, Wrap, action_row_label, wrapped};
    assert_eq!(wrapped(&open_menu("M"), Wrap::Toggle), open_menu("M"));
    assert_eq!(action_row_label(ActionRow::Wrap(Wrap::Toggle), &open_menu("M")), "A menu can't be a toggle");
    assert!(matches!(wrapped(&key("KEY_A"), Wrap::Toggle), ButtonAction::Toggle(_)), "anything else still can be");
}
