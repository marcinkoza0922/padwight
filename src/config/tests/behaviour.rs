use std::path::PathBuf;

use crate::config::*;

#[test]
fn profiles_needing_missing_features_are_skipped() {
    let mut gyro = Profile::passthrough("Gyro");
    gyro.requires = vec![Feature::Gyro];
    let mut config = Config::default();
    config.games.push(Game::new("Doom", vec![gyro, Profile::passthrough("Plain"), Profile::passthrough("Other")]));
    config.active = ProfileRef::new(Some("Doom"), "Other");
    let plain: &[Feature] = &[];
    assert_eq!(config.next_profile(None), Some(ProfileRef::new(Some("Doom"), "Gyro")));
    assert_eq!(config.next_profile(Some(plain)), Some(ProfileRef::new(Some("Doom"), "Plain")));
    assert_eq!(config.next_profile(Some(&[Feature::Gyro])), Some(ProfileRef::new(Some("Doom"), "Gyro")));
    let gyro_ref = ProfileRef::new(Some("Doom"), "Gyro");
    assert_eq!(config.usable_profile(gyro_ref.clone(), Some(plain)), ProfileRef::new(Some("Doom"), "Plain"));
    assert_eq!(config.usable_profile(gyro_ref.clone(), Some(&[Feature::Gyro])), gyro_ref);
    assert_eq!(config.usable_profile(gyro_ref.clone(), None), gyro_ref);
    assert!(Profile::passthrough("P").usable_with(&[]));
}

#[test]
fn profile_follows_the_controllers_features() {
    let mut gyro = Profile::passthrough("Gyro");
    gyro.requires = vec![Feature::Gyro];
    let mut config = Config::default();
    config.games.push(Game::new("Doom", vec![gyro, Profile::passthrough("Plain")]));
    let (gyro_ref, plain_ref) = (ProfileRef::new(Some("Doom"), "Gyro"), ProfileRef::new(Some("Doom"), "Plain"));
    let with = [Feature::Gyro];
    config.active = gyro_ref.clone();
    assert_eq!(config.refit(&with, None), Refit::Keep);
    // The DualSense is swapped for an Xbox pad: the game's plain profile takes over.
    assert_eq!(config.refit(&[], None), Refit::Leave { missing: vec![Feature::Gyro], to: plain_ref.clone() });
    config.active = plain_ref.clone();
    let fell = (gyro_ref.clone(), plain_ref.clone());
    assert_eq!(config.refit(&[], Some(&fell)), Refit::Keep, "still without gyro");
    assert_eq!(config.refit(&with, Some(&fell)), Refit::Return(gyro_ref.clone()), "and back with it");
    config.active = ProfileRef::new(None, "Gamepad");
    assert_eq!(config.refit(&with, Some(&fell)), Refit::Forget, "chosen by hand since");
    // A game with nothing playable gives way to the default profile.
    config.games[0].profiles.remove(1);
    config.active = gyro_ref;
    assert_eq!(config.refit(&[], None), Refit::Leave { missing: vec![Feature::Gyro], to: config.fallback_profile() });
}

#[test]
fn next_profile_wraps_within_the_game() {
    let mut config = Config::default();
    assert_eq!(config.next_profile(None), Some(ProfileRef::new(None, "Desktop")));
    config.active = ProfileRef::new(None, "Desktop");
    assert_eq!(config.next_profile(None), Some(ProfileRef::new(None, "Gamepad")));
    config.games.push(Game::new("Doom", vec![Profile::pc_action("Play"), Profile::desktop("Menus")]));
    config.active = ProfileRef::new(Some("Doom"), "Menus");
    assert_eq!(config.next_profile(None), Some(ProfileRef::new(Some("Doom"), "Play")));
}

#[test]
fn replacing_the_config_keeps_follows_or_replaces_the_active_profile() {
    let mut config = Config::default();
    config.games.push(Game::new("Doom", vec![Profile::pc_action("Play")]));
    let playing = ProfileRef::new(Some("Doom"), "Play");
    // Unchanged: stays, whatever the editor's copy says.
    let mut new = config.clone();
    new.active = ProfileRef::new(None, "Desktop");
    new.carry_active(&playing);
    assert_eq!(new.active, playing);
    // Renamed: the editor's copy says where it went.
    let mut renamed = config.clone();
    renamed.games[0].name = "Doom II".into();
    renamed.active = ProfileRef::new(Some("Doom II"), "Play");
    renamed.carry_active(&playing);
    assert_eq!(renamed.active, ProfileRef::new(Some("Doom II"), "Play"));
    // Deleted: the default profile, else General's first.
    let mut deleted = config.clone();
    deleted.games.clear();
    deleted.active = playing.clone();
    deleted.auto_switch.default_profile = Some(ProfileRef::new(None, "Desktop"));
    deleted.carry_active(&playing);
    assert_eq!(deleted.active, ProfileRef::new(None, "Desktop"));
    deleted.auto_switch.default_profile = None;
    deleted.carry_active(&ProfileRef::new(Some("Gone"), "x"));
    assert_eq!(deleted.active, ProfileRef::new(None, "Desktop"), "Desktop exists, so it's kept");
    deleted.active = ProfileRef::new(Some("Gone"), "x");
    deleted.carry_active(&ProfileRef::new(Some("Gone"), "x"));
    assert_eq!(deleted.active, ProfileRef::new(None, "Gamepad"));
}

#[test]
fn a_missing_active_profile_falls_back_to_its_game_then_general() {
    let mut config = Config::default();
    config.games.push(Game::new("Doom", vec![Profile::pc_action("Play")]));
    config.active = ProfileRef::new(Some("Doom"), "Gone");
    assert_eq!(config.active_ref(), ProfileRef::new(Some("Doom"), "Play"));
    config.active = ProfileRef::new(Some("No such setup"), "Play");
    assert_eq!(config.active_ref(), ProfileRef::new(None, "Gamepad"));
}

#[test]
fn scope_holds_only_the_active_games_items() {
    let mut config = Config::default();
    let m = |name: &str, steps: usize| Macro { name: name.into(), steps: vec![MacroStep::Wait(1); steps] };
    config.general.macros = vec![m("Dodge", 1), m("Screenshot", 1)];
    let mut game = Game::new("Doom", vec![Profile::pc_action("Play")]);
    game.macros = vec![m("Dodge", 2)];
    config.games.push(game);
    assert_eq!(config.scope().macros, [m("Dodge", 1), m("Screenshot", 1)]);
    config.active = ProfileRef::new(Some("Doom"), "Play");
    assert_eq!(config.scope().macros, [m("Dodge", 2)], "General's items aren't usable in other setups");
}

#[test]
fn finding_a_profile_by_name_prefers_the_active_game() {
    let mut config = Config::default();
    config.games.push(Game::new("A", vec![Profile::desktop("Desktop"), Profile::pc_action("Play")]));
    config.games.push(Game::new("B", vec![Profile::pc_action("Play")]));
    assert_eq!(config.find_profile("Desktop"), Some(ProfileRef::new(None, "Desktop")));
    assert_eq!(config.find_profile("Play"), Some(ProfileRef::new(Some("A"), "Play")));
    config.active = ProfileRef::new(Some("B"), "Play");
    assert_eq!(config.find_profile("Play"), Some(ProfileRef::new(Some("B"), "Play")));
    assert_eq!(config.find_profile("Nope"), None);
}

#[test]
fn layers_override_on_top_of_a_profile_newest_last() {
    let mut base = Profile::desktop("Desktop");
    base.gestures.insert(Button::North, Gestures { double_tap: Some(ButtonAction::Screenshot), ..Gestures::default() });
    let f = |k: &str| ButtonAction::Keys(vec![k.into()]);
    let mut a = Layer::new("A");
    a.buttons.insert(Button::South, f("KEY_F1"));
    a.gestures.insert(Button::North, Gestures::default());
    a.disabled_combos.push(vec![Button::RightBumper, Button::LeftBumper]);
    a.right_stick = Some(StickConfig::new(StickAction::Disabled, 0.1, 1.0));
    let mut b = Layer::new("B");
    b.buttons.insert(Button::South, f("KEY_F2"));
    b.combos.push(Combo { buttons: vec![Button::Select, Button::Start], action: f("KEY_F3") });

    let p = base.with_layers([&a, &b]);
    assert_eq!(p.button(Button::South), &f("KEY_F2"), "the newest layer wins");
    assert_eq!(p.button(Button::East), base.button(Button::East), "unset inputs fall through");
    assert!(p.gestures(Button::North).is_none(), "empty gestures switch the profile's off");
    assert_eq!(p.combos.len(), 1, "LB+RB switched off (in any order), Select+Start added");
    assert_eq!(p.combos[0].buttons, [Button::Select, Button::Start]);
    assert_eq!(p.right_stick.action, StickAction::Disabled);
    assert_eq!(p.left_stick, base.left_stick);
    assert_eq!(a.overrides(), 4);

    // Layers belong to games and roundtrip with them.
    let mut config = Config::default();
    config.general.layers = vec![a, b];
    config.general.profiles[0].set_button(Button::LeftBumper, ButtonAction::Layer("A".into()));
    let back: Config = toml::from_str(&toml::to_string_pretty(&config).unwrap()).unwrap();
    assert_eq!(back, config);
    assert_eq!(config.general.layers.len(), 2);
    assert_eq!(config.general.layers_named(&["B".into(), "Nope".into()]).map(|l| l.name.as_str()).collect::<Vec<_>>(), ["B"]);
    config.general.rename_refs(ItemKind::Layer, "A", "Alpha");
    assert_eq!(config.general.profiles[0].button(Button::LeftBumper), &ButtonAction::Layer("Alpha".into()));
}

#[test]
fn renames_follow_into_profiles_menus_and_macros() {
    let mut game = Game::new("G", vec![Profile::passthrough("P")]);
    game.profiles[0].set_button(Button::West, ButtonAction::toggle(ButtonAction::Macro { name: "Old".into(), repeat: false }));
    game.menus.push(Menu {
        name: "M".into(),
        kind: MenuKind::List,
        items: vec![MenuItem::new("x", ButtonAction::Macro { name: "Old".into(), repeat: false })],
        cancel: None, no_cancel: false, keep_open: false, start: 0,
        style: OverlayStyle::default(),
    });
    game.rename_refs(ItemKind::Macro, "Old", "New");
    game.rename_refs(ItemKind::Menu, "New", "Wrong kind");
    assert_eq!(game.profiles[0].button(Button::West), &ButtonAction::toggle(ButtonAction::Macro { name: "New".into(), repeat: false }));
    assert_eq!(game.menus[0].items[0].action, ButtonAction::Macro { name: "New".into(), repeat: false });
}

pub(super) fn scratch_dir(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("padwight-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

#[test]
#[cfg(target_os = "linux")]
fn a_saved_config_is_readable_only_by_its_owner() {
    use std::os::unix::fs::PermissionsExt;
    let mode = |path: &std::path::Path| std::fs::metadata(path).unwrap().permissions().mode() & 0o777;
    let dir = scratch_dir("private-save");
    let path = dir.join("config.toml");

    Config::default().save_to(&path).unwrap();
    assert_eq!(mode(&path), 0o600, "a new config is private");

    // A temporary file left loose by an earlier run must not keep its mode.
    let tmp = dir.join("config.toml.tmp");
    std::fs::write(&tmp, "left over").unwrap();
    std::fs::set_permissions(&tmp, std::fs::Permissions::from_mode(0o644)).unwrap();
    Config::default().save_to(&path).unwrap();
    assert_eq!(mode(&path), 0o600, "a save over a loose temporary file is private too");
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn a_missing_config_is_written_as_the_default() {
    let dir = scratch_dir("missing-config");
    let path = dir.join("nested").join("config.toml");
    assert!(Config::load_from(&path).unwrap() == Config::default());
    assert!(Config::load_from(&path).unwrap() == Config::default());
    assert!(!dir.join("nested").join("config.toml.old").exists());
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn paddle_buttons_round_trip_and_profiles_need_the_feature_for_them() {
    let mut p = Profile::passthrough("Paddles");
    p.set_button(Button::LeftPaddle, ButtonAction::Gamepad(Button::South));
    p.requires = vec![Feature::BackPaddles];
    let text = toml::to_string(&p).unwrap();
    assert_eq!(toml::from_str::<Profile>(&text).unwrap(), p);
    assert!(!p.usable_with(&[Feature::Gyro]) && p.usable_with(&[Feature::BackPaddles]));
    assert_eq!(Feature::BackPaddles.label(), "back paddles");
    let text = toml::to_string(&Profile { requires: vec![Feature::Touchpad], ..Profile::passthrough("t") }).unwrap();
    assert!(text.contains("requires = [\"touchpad\"]"), "{text}");
    assert!(!Profile { requires: vec![Feature::Touchpad], ..Profile::passthrough("t") }.usable_with(&[Feature::Gyro, Feature::BackPaddles]));
}

#[test]
fn the_built_in_modes_let_nothing_of_the_profile_through_but_a_way_back() {
    let key = |k: &str| ButtonAction::Keys(vec![k.into()]);
    let mut p = Profile::passthrough("Paddles");
    p.set_button(Button::LeftPaddle, ButtonAction::Gamepad(Button::South));
    p.gestures.insert(Button::RightPaddle, Gestures { long_press: Some(key("KEY_R")), ..Gestures::default() });
    p.set_button(Button::RightStickUp, key("KEY_W"));
    p.combos.push(Combo { buttons: vec![Button::LeftBumper, Button::RightBumper], action: key("KEY_G") });
    // What switches modes stays, so there's a way back out.
    p.set_button(Button::RightPaddle2, ButtonAction::Mode("Driving".into()));
    p.combos.push(Combo { buttons: vec![Button::Select, Button::Start], action: ButtonAction::Layer("Map".into()) });
    for mode in [Layer::mouse(), Layer::guide()] {
        let on = p.with_layers([&mode]);
        for b in [Button::LeftPaddle, Button::RightPaddle, Button::RightStickUp] {
            assert_eq!(on.button(b), &ButtonAction::Disabled, "{}: {b}", mode.name);
            assert!(on.gestures(b).is_none(), "{}: {b}", mode.name);
        }
        assert!(!on.combos.iter().any(|c| c.action == key("KEY_G")), "{}: the profile's combo goes", mode.name);
        assert!(on.combos.iter().any(|c| c.action == ButtonAction::Layer("Map".into())), "{}", mode.name);
        assert_eq!(on.button(Button::RightPaddle2), &ButtonAction::Mode("Driving".into()), "{}", mode.name);
    }
}

#[test]
fn the_built_in_modes_let_none_of_the_profile_s_sticks_triggers_gyro_or_touchpad_through() {
    let key = |k: &str| ButtonAction::Keys(vec![k.into()]);
    let mut p = Profile::passthrough("Everything");
    p.left_stick.zones.push(Zone { min: 0.9, max: 1.0, action: key("KEY_LEFTSHIFT") });
    p.left_stick.deadzone = 0.3;
    p.left_trigger.zones.push(Zone { min: 0.5, max: 1.0, action: key("KEY_E") });
    p.gyro.mode = GyroMode::Mouse { sensitivity: 10.0 };
    p.touchpad = TouchpadConfig::default();
    assert!(p.left_stick.action != StickAction::Disabled && p.left_trigger.action != TriggerAction::Disabled);
    let off = TouchpadConfig { motion: TouchpadMotion::Off, click: ButtonAction::Disabled, right: None };
    for mode in [Layer::mouse(), Layer::guide()] {
        let on = p.with_layers([&mode]);
        assert_eq!(on.gyro.mode, GyroMode::Off, "{}", mode.name);
        assert_eq!(on.touchpad, off, "{}", mode.name);
        assert!(on.left_stick.zones.is_empty() && on.left_trigger.zones.is_empty(), "{}", mode.name);
    }
    let guide = p.with_layers([&Layer::guide()]);
    for s in [&guide.left_stick, &guide.right_stick] {
        assert_eq!(s.action, StickAction::Disabled);
    }
    assert_eq!(guide.left_stick.deadzone, 0.3, "the direction buttons keep the profile's deadzone");
    for t in [&guide.left_trigger, &guide.right_trigger] {
        assert_eq!(t.action, TriggerAction::Disabled);
    }
    let mouse = p.with_layers([&Layer::mouse()]);
    assert_eq!(mouse.right_stick, Layer::mouse().right_stick.unwrap(), "mouse mode's own sticks stay");
    // A game's own mode that does nothing with unset inputs still keeps the profile's analog ones.
    let mut menus = Layer::new("Menus");
    menus.swallow_unbound = true;
    let own = p.with_layers([&menus]);
    assert_eq!((own.left_stick, own.gyro, own.touchpad), (p.left_stick.clone(), p.gyro.clone(), p.touchpad.clone()));
}

#[test]
fn mouse_mode_can_let_the_gyro_and_touchpad_move_the_pointer() {
    let mut p = Profile::passthrough("Aim");
    p.gyro.mode = GyroMode::Stick { stick: Stick::Right, full_rate: 300.0, anti_deadzone: 0.1, game_curve: 1.0 };
    p.gyro.start_off = true;
    p.gyro.full_turn_px = Some(4000.0);
    p.touchpad.motion = TouchpadMotion::Off;
    let config = Config { mouse_mode: MouseModeExtras { gyro: true, touchpad: true }, ..Config::default() };
    let game = Game::new("G", vec![]);
    let mouse = config.mode_for(&game, MOUSE_MODE).unwrap();
    let on = p.with_layers([&*mouse]);
    assert_eq!(on.gyro.mode, GyroMode::Mouse { sensitivity: 15.0 }, "a gyro aiming a stick moves the pointer at the default speed");
    assert_eq!((on.gyro.start_off, on.gyro.full_turn_px), (true, None), "it keeps whether it starts on");
    assert_eq!(on.touchpad, TouchpadConfig::default(), "the touchpad works as a laptop's");
    p.gyro.mode = GyroMode::Mouse { sensitivity: 8.0 };
    assert_eq!(p.with_layers([&*mouse]).gyro, p.gyro, "a gyro that already moves the pointer is kept as it is");
    // The Guide mode still lets neither through.
    let guide = config.mode_for(&game, GUIDE_MODE).unwrap();
    assert_eq!(p.with_layers([&*guide]).gyro.mode, GyroMode::Off);

    let dir = scratch_dir("mouse-mode-extras");
    let path = dir.join("config.toml");
    config.save_to(&path).unwrap();
    assert_eq!(Config::load_from(&path).unwrap().mouse_mode, config.mouse_mode);
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn a_lost_config_file_keeps_the_setups_saved_beside_it() {
    let dir = scratch_dir("lost-settings");
    let path = dir.join("config.toml");
    let mut config = Config::default();
    config.games.push(Game::new("Doom", vec![Profile::pc_action("Play")]));
    config.general.profiles.push(Profile::platformer("Retro"));
    config.save_to(&path).unwrap();
    std::fs::remove_file(&path).unwrap();
    let loaded = Config::load_from(&path).unwrap();
    assert_eq!((loaded.games.clone(), loaded.general.clone()), (config.games.clone(), config.general.clone()));
    assert!(path.exists(), "the settings are written again");
    assert_eq!(Config::load_from(&path).unwrap().games, config.games);
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn combos_listing_a_button_twice_are_tidied_when_loaded() {
    let dir = scratch_dir("repeated-combo");
    let path = dir.join("config.toml");
    let mut config = Config::default();
    let doubled = Combo { buttons: vec![Button::South, Button::East, Button::South], action: ButtonAction::Screenshot };
    config.general.profiles[0].combos.push(doubled);
    let mut layer = Layer::new("L");
    layer.combos.push(Combo { buttons: vec![Button::Start, Button::Select, Button::Start], action: ButtonAction::Screenshot });
    layer.disabled_combos.push(vec![Button::East, Button::East]);
    config.general.layers.push(layer);
    config.save_to(&path).unwrap();
    let loaded = Config::load_from(&path).unwrap();
    assert_eq!(loaded.general.profiles[0].combos[0].buttons, [Button::South, Button::East]);
    assert_eq!(loaded.general.layers[0].combos[0].buttons, [Button::Start, Button::Select]);
    assert_eq!(loaded.general.layers[0].disabled_combos[0], [Button::East]);
    assert!(loaded.unreadable.is_empty());
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn a_game_s_mode_named_like_a_built_in_one_never_replaces_it() {
    let mut game = Game::new("Doom", vec![Profile::passthrough("Play")]);
    game.modes.push(Mode::new(GUIDE_MODE));
    let config = Config::default();
    assert_eq!(config.mode_for(&game, GUIDE_MODE).as_deref(), Some(&Layer::guide()));
}

#[test]
fn a_number_that_isn_t_finite_is_refused_naming_where() {
    let dir = scratch_dir("not-finite");
    let path = dir.join("config.toml");
    let mut config = Config::default();
    config.general.info.push(InfoOverlay {
        name: "Sheet".into(),
        always: false,
        on_start: None,
        linger: Some(f32::INFINITY),
        title: None,
        current_input: Default::default(),
        style: OverlayStyle::info(),
        rows: Vec::new(),
    });
    config.save_to(&path).unwrap();
    // An item file is left out; the rest loads.
    let loaded = Config::load_from(&path).unwrap();
    assert!(loaded.general.info.is_empty());
    let err = &loaded.unreadable[0].error;
    assert!(err.contains("linger") && err.contains("Sheet"), "{err}");

    config.general.info.clear();
    config.rumble_strength = f32::NAN;
    config.save_to(&path).unwrap();
    let err = format!("{:#}", Config::load_from(&path).unwrap_err());
    assert!(err.contains("rumble_strength") && err.contains("config.toml"), "{err}");
    let _ = std::fs::remove_dir_all(&dir);
}
