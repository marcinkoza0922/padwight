use super::*;

#[test]
fn overlay_size_and_offset_are_optional_and_unwritten_when_default() {
    // Older configs fit their contents, sit where their position puts them, and cap at 90%.
    let style: OverlayStyle = toml::from_str("position = \"center\"\n").unwrap();
    assert_eq!((style.width, style.height), (None, None));
    assert_eq!((style.x_offset, style.y_offset), (0.0, 0.0));
    assert_eq!((style.max_width, style.max_height), (90.0, 90.0));
    let written = toml::to_string(&OverlayStyle::default()).unwrap();
    for key in ["width", "height", "x_offset", "y_offset", "max_width", "max_height"] {
        assert!(!written.contains(key), "{key} is written by default: {written}");
    }
    let sized = OverlayStyle { width: Some(40.0), x_offset: -5.0, max_height: 60.0, ..OverlayStyle::default() };
    let back: OverlayStyle = toml::from_str(&toml::to_string(&sized).unwrap()).unwrap();
    assert_eq!(back, sized);
}

#[test]
fn overlay_corners_and_radial_options_are_optional() {
    // Older configs have none of the new keys: they load with today's look.
    let style: OverlayStyle = toml::from_str("position = \"center\"\n").unwrap();
    assert_eq!(style.corners, 0.4);
    let item: MenuItem = toml::from_str("label = \"x\"\naction = { keys = [\"KEY_X\"] }\n").unwrap();
    assert_eq!(item.weight, 1.0);
    #[derive(Deserialize)]
    struct Holder {
        kind: MenuKind,
    }
    let holder: Holder = toml::from_str("[kind.radial]\nstick = \"Right\"\n").unwrap();
    assert_eq!(holder.kind, MenuKind::Radial { stick: Stick::Right, boxes: false });
    // Defaults are left out of what gets written, so existing files and packs don't change.
    let written = toml::to_string(&OverlayStyle::default()).unwrap();
    assert!(!written.contains("corners"), "{written}");
    assert!(!toml::to_string(&MenuItem { weight: 1.0, ..item.clone() }).unwrap().contains("weight"));
    assert!(toml::to_string(&MenuKind::Radial { stick: Stick::Right, boxes: true }).unwrap().contains("boxes = true"));
}

#[test]
fn built_in_modes_are_never_stored_but_every_game_has_them() {
    // A stored copy of a built-in mode (as a layer, as older files have, or a mode) is
    // dropped, and the user's own stay.
    let mut own = Layer::new(GUIDE_MODE);
    own.buttons.insert(Button::East, ButtonAction::Keys(vec!["KEY_Q".into()]));
    let mut game = Game::new("Own", Vec::new());
    game.layers = vec![own, Layer::new("Mine")];
    game.modes = vec![Mode::new(MOUSE_MODE), Mode::new("Driving")];
    game.strip_built_in_modes();
    assert_eq!(game.layers.iter().map(|l| l.name.as_str()).collect::<Vec<_>>(), ["Mine"]);
    assert_eq!(game.modes.iter().map(|l| l.name.as_str()).collect::<Vec<_>>(), ["Driving"]);
    // Every game's scope has them as modes, unchanged, after its own.
    let mut config = Config::default();
    config.games.push(game);
    let scope = config.scope_of(&config.games[0]);
    let names: Vec<&str> = scope.modes.iter().map(|l| l.name.as_str()).collect();
    assert_eq!(names, ["Driving", GUIDE_MODE, MOUSE_MODE, MAGNIFIER_MODE]);
    assert!(scope.layers.iter().all(|l| built_in_mode(&l.name).is_none()), "they aren't layers");
    let guide = scope.modes.iter().find(|l| l.name == GUIDE_MODE).unwrap();
    assert_eq!(**guide, Layer::guide());
    assert_eq!(config.mode_for(&config.games[0], MOUSE_MODE).as_deref(), Some(&Layer::mouse()));
    let owned = Scope { modes: scope.modes.iter().map(|m| (**m).clone()).collect(), ..Scope::default() };
    assert_eq!(owned.game_modes().iter().map(|l| l.name.as_str()).collect::<Vec<_>>(), ["Driving"]);
}

#[test]
fn log_overlays_roundtrip_and_old_configs_load_without_them() {
    let mut config = Config::default();
    let mut log = LogOverlay::new("Inputs");
    let s = log.source.settings_mut();
    (s.tracking.device, s.newest, s.fade_after) = (Some(1), LogEnd::Bottom, 0.0);
    s.tracking.set_tracked(TrackedInput::Stick(Stick::Right), false);
    config.general.logs.push(log);
    config.general.profiles[0].set_button(Button::Select, ButtonAction::toggle(ButtonAction::ShowLog("Inputs".into())));
    let back: Config = toml::from_str(&toml::to_string_pretty(&config).unwrap()).unwrap();
    assert_eq!(back, config);
    let plain = toml::to_string_pretty(&Config::default()).unwrap();
    assert!(!plain.contains("log_overlays"), "nothing written when there are none");
    assert!(toml::from_str::<Config>(&plain).unwrap().general.logs.is_empty());
}

#[test]
fn default_config_roundtrips_through_toml() {
    let config = Config::default();
    let text = toml::to_string_pretty(&config).unwrap();
    let back: Config = toml::from_str(&text).unwrap();
    assert_eq!(config, back);
}

#[test]
fn setups_are_saved_as_setups() {
    let mut config = Config::default();
    config.games.push(Game::new("Doom", vec![Profile::passthrough("Play")]));
    config.active = ProfileRef::new(Some("Doom"), "Gamepad");
    let text = toml::to_string_pretty(&config).unwrap();
    assert!(text.contains("[[setups]]") && text.contains("setup = \"Doom\""), "{text}");
    assert!(!text.contains("[[games]]"), "{text}");
    assert_eq!(toml::from_str::<Config>(&text).unwrap(), config);
}

#[test]
fn multi_and_combos_roundtrip_through_toml() {
    let mut config = Config::default();
    config.general.profiles[0].set_button(
        Button::South,
        ButtonAction::Multi(vec![
            ButtonAction::Gamepad(Button::South),
            ButtonAction::Keys(vec!["KEY_X".into()]),
            ButtonAction::Mouse(MouseButton::Left),
        ]),
    );
    config.general.profiles[0].combos.push(Combo {
        buttons: vec![Button::Select, Button::Start],
        action: ButtonAction::Screenshot,
        window_ms: None,
    });
    config.general.profiles[0].gestures.insert(
        Button::North,
        Gestures {
            double_tap: Some(ButtonAction::Keys(vec!["KEY_F5".into()])),
            triple_tap: None,
            long_press: Some(ButtonAction::Screenshot),
            on_press: Some(ButtonAction::Keys(vec!["KEY_Z".into()])),
            on_release: None,
            tap_window_ms: Some(180),
            long_press_ms: Some(900),
        },
    );
    let back: Config = toml::from_str(&toml::to_string_pretty(&config).unwrap()).unwrap();
    assert_eq!(config, back);
}

#[test]
fn zones_roundtrip_and_legacy_trigger_format_loads() {
    let mut config = Config::default();
    let p = &mut config.general.profiles[0];
    p.left_stick.zones.push(Zone { min: 0.0, max: 0.75, action: ButtonAction::Keys(vec!["KEY_LEFTSHIFT".into()]) });
    p.right_trigger.zones.push(Zone { min: 0.95, max: 1.0, action: ButtonAction::Mouse(MouseButton::Left) });
    let back: Config = toml::from_str(&toml::to_string_pretty(&config).unwrap()).unwrap();
    assert_eq!(config, back);

    // Before zones existed, a trigger was stored as a bare TriggerAction.
    let mut value = toml::Value::try_from(Config::default()).unwrap();
    for profile in value["general"]["profiles"].as_array_mut().unwrap() {
        let table = profile.as_table_mut().unwrap();
        let action = table["left_trigger"]["action"].clone();
        table.insert("left_trigger".into(), action);
    }
    let config: Config = toml::from_str(&toml::to_string(&value).unwrap()).unwrap();
    assert_eq!(config.general.profiles[0].left_trigger, TriggerAction::Gamepad(Trigger::Left).into());
    assert_eq!(config.general.profiles[1].trigger(Trigger::Left), Config::default().general.profiles[1].trigger(Trigger::Left));
}

#[test]
fn old_configs_without_combos_still_load() {
    let mut value = toml::Value::try_from(Config::default()).unwrap();
    for profile in value["general"]["profiles"].as_array_mut().unwrap() {
        let table = profile.as_table_mut().unwrap();
        table.remove("combos");
        table.remove("combo_window_ms");
        table.remove("gestures");
        table.remove("tap_window_ms");
        table.remove("long_press_ms");
    }
    let config: Config = toml::from_str(&toml::to_string(&value).unwrap()).unwrap();
    assert!(config.general.profiles[0].combos.is_empty());
    assert_eq!(config.general.profiles[0].combo_window_ms, 60);
}

#[test]
fn auto_switch_roundtrips_and_is_optional() {
    let mut config = Config::default();
    config.auto_switch.default_profile = Some(ProfileRef::new(None, "Desktop"));
    let mut game = Game::new("Elden Ring", vec![Profile::pc_action("Gameplay")]);
    game.rules.push(Rule::new(RuleKind::SteamAppId, "1245620", "Gameplay"));
    game.rules.push(Rule { enabled: false, ..Rule::new(RuleKind::Executable, "eldenring.exe", "Gameplay") });
    config.games.push(game);
    let text = toml::to_string_pretty(&config).unwrap();
    let rules: Vec<String> = config.games[0].rules.iter().map(|r| toml::to_string(r).unwrap()).collect();
    assert!(!rules[0].contains("enabled") && rules[1].contains("enabled = false"), "rules only note being off");
    let back: Config = toml::from_str(&text).unwrap();
    assert_eq!(config, back);

    let mut value = toml::Value::try_from(Config::default()).unwrap();
    value.as_table_mut().unwrap().remove("auto_switch");
    let old: Config = toml::from_str(&toml::to_string(&value).unwrap()).unwrap();
    assert_eq!(old.auto_switch, AutoSwitch::default());
    assert_eq!(old.auto_switch.default_profile, Some(ProfileRef::new(None, "Gamepad")));

    // Keeping the current profile is written out, since a missing value means Gamepad.
    config.auto_switch.default_profile = None;
    let text = toml::to_string_pretty(&config).unwrap();
    assert!(text.contains("default_profile = \"keep\""), "{text}");
    assert_eq!(toml::from_str::<Config>(&text).unwrap().auto_switch.default_profile, None);
}

fn templates() -> Vec<Profile> {
    vec![
        Profile::passthrough("a"),
        Profile::desktop("b"),
        Profile::pc_action("c"),
        Profile::strategy("d"),
        Profile::platformer("e"),
        Profile::gyro_aim("f"),
        Profile::flick_stick("g"),
    ]
}

#[test]
fn gyro_templates_need_a_gyro() {
    let aim = Profile::gyro_aim("f");
    assert!(matches!(aim.gyro.mode, GyroMode::Mouse { .. }));
    assert!(matches!(aim.button(Button::LeftBumper), ButtonAction::Multi(list) if list.contains(&ButtonAction::GyroClutch)));
    assert!(!aim.usable_with(&[]) && aim.usable_with(&[Feature::Gyro]));
    let flick = Profile::flick_stick("g");
    assert!(matches!(flick.right_stick.action, StickAction::Flick { .. }));
    assert!(!flick.usable_with(&[]) && flick.usable_with(&[Feature::Gyro]));
    assert!(Profile::pc_action("c").usable_with(&[]), "PC action is playable without a gyro");
}

#[test]
fn templates_map_every_button_with_valid_keys() {
    use std::str::FromStr;
    for p in templates() {
        assert_eq!(p.buttons.len(), Button::ALL.len(), "{}: unmapped buttons", p.name);
        let mut keys: Vec<&String> = p.buttons.values().flat_map(|a| a.key_names()).collect();
        for t in [Trigger::Left, Trigger::Right] {
            if let TriggerAction::Button { action, .. } = p.trigger(t) {
                keys.extend(action.key_names());
            }
        }
        for s in [Stick::Left, Stick::Right] {
            let cfg = p.stick(s);
            keys.extend(cfg.action.ring_actions().iter().flat_map(|a| a.key_names()));
            keys.extend(cfg.zones.iter().flat_map(|z| z.action.key_names()));
        }
        for k in keys {
            assert!(crate::keycode::KeyCode::from_str(k).is_ok(), "{}: bad key {k}", p.name);
        }
        // Guide always opens the Guide layer, which holds the system shortcuts.
        assert_eq!(p.button(Button::Guide), &ButtonAction::guide_hold(), "{}", p.name);
        assert!(!p.gestures.contains_key(&Button::Guide), "{}: Guide has no gestures", p.name);
    }
}

#[test]
fn default_config_has_the_guide_mode_and_roundtrips() {
    let config = Config::default();
    assert!(config.general.modes.is_empty(), "built-in modes aren't stored");
    assert!(config.scope().modes.iter().any(|l| l.name == GUIDE_MODE));
    let text = toml::to_string_pretty(&config).unwrap();
    assert_eq!(toml::from_str::<Config>(&text).unwrap(), config);
}

#[test]
fn a_setup_s_guide_sounds_reach_the_scope_the_daemon_plays_from() {
    let mut config = Config::default();
    let mut set = SoundSet::default();
    let mut guide = set.get(crate::sound::SoundOverlay::Guide);
    guide.step.kind = crate::sound::SoundKind::Typewriter;
    set.set(crate::sound::SoundOverlay::Guide, guide);
    config.general.sounds = Some(set);
    let back: Config = toml::from_str(&toml::to_string_pretty(&config).unwrap()).unwrap();
    let cue = back.scope().sounds.cue(crate::sound::SoundOverlay::Guide, crate::sound::Feedback::Step);
    assert_eq!(cue.map(|s| s.kind), Some(crate::sound::SoundKind::Typewriter));
}

#[test]
fn the_guide_layer_leaves_profiles_alone() {
    assert_eq!(Layer::guide().buttons.get(&Button::DpadUp), None);
}

/// Actions, gyro, macros and menus through TOML.
mod actions;
/// How profiles are picked, layered and saved.
mod behaviour;
mod gyro_use;
mod refused;

#[test]
fn output_and_shifts_logs_roundtrip_and_keep_their_settings_when_switched() {
    let mut config = Config::default();
    for (name, kind) in [("Sent", LogKind::Output), ("Modes", LogKind::Shifts)] {
        config.general.logs.push(LogOverlay { source: LogSource::new(kind), ..LogOverlay::new(name) });
    }
    let back: Config = toml::from_str(&toml::to_string_pretty(&config).unwrap()).unwrap();
    assert_eq!(back, config);
    let mut source = LogSource::new(LogKind::Input);
    source.settings_mut().lines = 3;
    assert_eq!(source.with_kind(LogKind::Shifts), LogSource::Shifts(InputLogSettings { lines: 3, ..InputLogSettings::default() }));
}
