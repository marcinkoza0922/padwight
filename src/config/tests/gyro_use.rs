//! A profile whose gyro is disabled can't hold gyro actions, nor open what holds them.

use crate::config::problems::{Names, game_problem, menu_gyro_warning};
use crate::config::*;

fn menu(name: &str, items: Vec<ButtonAction>) -> Menu {
    Menu {
        name: name.into(),
        kind: MenuKind::Buttons,
        items: items.into_iter().map(|a| MenuItem::new("Item", a)).collect(),
        cancel: Some(Button::Start),
        no_cancel: false,
        keep_open: false,
        start: 0,
        style: OverlayStyle::default(),
    }
}

/// General with one profile, whose gyro is `enabled`.
fn setup(enabled: bool) -> Config {
    let mut config = Config::default();
    config.general.profiles.truncate(1);
    config.general.profiles[0].gyro.enabled = enabled;
    config
}

fn problem(config: &Config) -> Option<String> {
    game_problem(config, &config.general)
}

#[test]
fn a_gyro_action_needs_gyro_enabled() {
    let mut config = setup(true);
    config.general.profiles[0].set_button(Button::North, ButtonAction::GyroToggle);
    assert_eq!(problem(&config), None);
    config.general.profiles[0].gyro.enabled = false;
    let why = problem(&config).unwrap();
    assert!(why.starts_with("Profile “") && why.contains("needs gyro"), "{why}");
}

#[test]
fn every_gyro_action_and_every_place_for_one_is_checked() {
    for action in [ButtonAction::GyroToggle, ButtonAction::GyroClutch, ButtonAction::GyroHold, ButtonAction::GyroRecenter] {
        let nested = ButtonAction::Multi(vec![ButtonAction::Keys(vec!["KEY_1".into()]), action.clone()]);
        let mut config = setup(false);
        let p = &mut config.general.profiles[0];
        p.combos.push(Combo { buttons: vec![Button::LeftBumper, Button::RightBumper], action: nested, window_ms: None });
        assert!(problem(&config).is_some_and(|w| w.contains("needs gyro")), "{action:?} in a combo");
        let mut config = setup(false);
        config.general.profiles[0].left_trigger = TriggerAction::Button { action, threshold: 0.3 }.into();
        assert!(problem(&config).is_some(), "on a trigger");
    }
}

#[test]
fn a_menu_with_gyro_is_only_a_problem_where_a_profile_without_gyro_opens_it() {
    let mut config = setup(false);
    config.general.menus = vec![menu("Aim", vec![ButtonAction::GyroToggle])];
    assert_eq!(problem(&config), None, "nothing opens it, so nothing is wrong");
    config.general.profiles[0].set_button(Button::North, ButtonAction::OpenMenu("Aim".into()));
    let why = problem(&config).unwrap();
    assert!(why.contains("menu “Aim” uses gyro"), "{why}");
    config.general.profiles[0].gyro.enabled = true;
    assert_eq!(problem(&config), None);
}

#[test]
fn a_profile_with_gyro_may_open_it_while_another_without_may_not() {
    let mut config = setup(true);
    config.general.menus = vec![menu("Aim", vec![ButtonAction::GyroHold])];
    let mut plain = Profile::passthrough("Plain");
    plain.gyro.enabled = false;
    config.general.profiles.push(plain);
    config.general.profiles[0].set_button(Button::North, ButtonAction::OpenMenu("Aim".into()));
    assert_eq!(problem(&config), None);
    config.general.profiles[1].set_button(Button::North, ButtonAction::OpenMenu("Aim".into()));
    assert!(problem(&config).is_some_and(|w| w.starts_with("Profile “Plain”")));
}

#[test]
fn gyro_is_found_through_other_menus_macros_and_layers() {
    let mut config = setup(false);
    config.general.menus = vec![menu("Top", vec![ButtonAction::OpenMenu("Deep".into())]), menu("Deep", vec![ButtonAction::Macro { name: "Aim".into(), repeat: false }])];
    config.general.macros = vec![Macro { name: "Aim".into(), steps: vec![MacroStep::Press(ButtonAction::GyroRecenter)] }];
    let names = Names::for_game(&config, &config.general);
    for (kind, name) in [(ItemKind::Menu, "Top"), (ItemKind::Menu, "Deep"), (ItemKind::Macro, "Aim")] {
        assert!(names.uses_gyro(kind, name), "{name}");
    }
    config.general.profiles[0].set_button(Button::North, ButtonAction::OpenMenu("Top".into()));
    assert!(problem(&config).is_some());
}

#[test]
fn a_layer_that_sets_its_own_gyro_supplies_it() {
    let mut config = setup(false);
    let mut layer = Layer::new("Aim");
    layer.buttons.insert(Button::South, ButtonAction::GyroHold);
    config.general.layers = vec![layer];
    config.general.profiles[0].set_button(Button::North, ButtonAction::Layer("Aim".into()));
    assert!(problem(&config).is_some_and(|w| w.contains("layer “Aim” uses gyro")), "it relies on the profile's gyro");
    config.general.layers[0].gyro = Some(GyroConfig { enabled: true, ..GyroConfig::default() });
    assert_eq!(problem(&config), None, "its own gyro is on");
    config.general.layers[0].gyro = Some(GyroConfig { enabled: false, ..GyroConfig::default() });
    assert!(problem(&config).is_some_and(|w| w.starts_with("Layer “Aim”")), "its own gyro is off, and it holds a gyro action");
}

#[test]
fn a_menu_with_gyro_where_no_profile_has_it_is_a_warning() {
    let mut config = setup(false);
    let aim = menu("Aim", vec![ButtonAction::GyroToggle]);
    config.general.menus = vec![aim.clone(), menu("Plain", vec![ButtonAction::Disabled])];
    let names = Names::for_game(&config, &config.general);
    assert!(menu_gyro_warning(&config.general, &names, &aim).is_some());
    assert_eq!(menu_gyro_warning(&config.general, &names, &config.general.menus[1]), None);
    config.general.profiles[0].gyro.enabled = true;
    assert_eq!(menu_gyro_warning(&config.general, &names, &aim), None, "some profile has gyro");
}

#[test]
fn the_touchpad_cant_lay_out_a_gyro_menu_without_gyro() {
    let mut config = setup(false);
    config.general.menus = vec![menu("Aim", vec![ButtonAction::GyroToggle])];
    config.general.profiles[0].touchpad.motion = TouchpadMotion::Menu { menu: "Aim".into(), choose: TouchChoose::Lift, overlay: false };
    assert!(problem(&config).is_some_and(|w| w.contains("touchpad")), "{:?}", problem(&config));
}
