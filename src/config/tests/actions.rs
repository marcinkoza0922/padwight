use crate::config::*;

#[test]
fn toggle_and_turbo_roundtrip_through_toml() {
    let mut config = Config::default();
    let crouch = ButtonAction::toggle(ButtonAction::Keys(vec!["KEY_C".into()]));
    let auto_fire = ButtonAction::toggle(ButtonAction::Turbo {
        action: Box::new(ButtonAction::Mouse(MouseButton::Left)),
        rate: 12.0,
        every_ms: 0,
    });
    config.general.profiles[0].set_button(Button::RightStick, crouch);
    config.general.profiles[0].set_button(Button::West, auto_fire);
    let back: Config = toml::from_str(&toml::to_string_pretty(&config).unwrap()).unwrap();
    assert_eq!(config, back);
    assert_eq!(config.general.profiles[0].button(Button::RightStick).key_names(), [&"KEY_C".to_string()]);
}

#[test]
fn toggles_that_start_on_add_a_field_others_read_as_before() {
    let plain = ButtonAction::toggle(ButtonAction::ShowInfo("Controls".into()));
    let starting = ButtonAction::Toggle(Toggled { action: Box::new(ButtonAction::ShowInfo("Controls".into())), start_on: true });
    let mut config = Config::default();
    config.general.profiles[0].set_button(Button::North, plain.clone());
    config.general.profiles[0].set_button(Button::West, starting.clone());
    let text = toml::to_string_pretty(&config).unwrap();
    assert!(text.contains("[general.profiles.buttons.North.toggle]\nshow_info = \"Controls\""), "{text}");
    assert!(text.contains("start_on = true"));
    let back: Config = toml::from_str(&text).unwrap();
    assert_eq!(back.general.profiles[0].button(Button::North), &plain);
    assert_eq!(back.general.profiles[0].button(Button::West), &starting);
    // Over IPC too.
    let json = serde_json::to_string(&config).unwrap();
    assert_eq!(serde_json::from_str::<Config>(&json).unwrap(), config);
}

#[test]
fn gyro_config_roundtrips_and_is_optional() {
    let mut config = Config::default();
    config.general.profiles[0].gyro = GyroConfig {
        mode: GyroMode::Stick { stick: Stick::Right, full_rate: 300.0, anti_deadzone: 0.2, game_curve: 1.0 },
        horizontal: GyroHorizontal::YawAndRoll,
        invert_x: true,
        invert_y: false,
        enabled: true, starts_active: false,
        noise_threshold: 2.0,
        axes: GyroAxes::Vertical,
        y_scale: 1.5,
        curve: CurvePreset::Wide.speed_curve(1.0).unwrap(),
        pause_with_stick: Some(Stick::Right),
        ..GyroConfig::default()
    };
    config.gyro_calibration.insert("Pad".into(), [0.1, -0.2, 0.05]);
    let back: Config = toml::from_str(&toml::to_string_pretty(&config).unwrap()).unwrap();
    assert_eq!(config, back);

    let mut value = toml::Value::try_from(Config::default()).unwrap();
    for profile in value["general"]["profiles"].as_array_mut().unwrap() {
        profile.as_table_mut().unwrap().remove("gyro");
    }
    let old: Config = toml::from_str(&toml::to_string(&value).unwrap()).unwrap();
    assert_eq!(old.general.profiles[0].gyro, GyroConfig::default());
}

#[test]
fn macros_roundtrip_and_walk_finds_nested_mappings() {
    let mut config = Config::default();
    let key = |k: &str| ButtonAction::Keys(vec![k.into()]);
    config.general.macros.push(Macro {
        name: "Combo".into(),
        steps: vec![
            MacroStep::Tap { action: key("KEY_A"), hold_ms: 40 },
            MacroStep::Wait(100),
            MacroStep::Press(key("KEY_LEFTSHIFT")),
            MacroStep::Tap { action: ButtonAction::Mouse(MouseButton::Left), hold_ms: 20 },
            MacroStep::Release(key("KEY_LEFTSHIFT")),
        ],
    });
    let mapped = ButtonAction::toggle(ButtonAction::Macro { name: "Combo".into(), repeat: true });
    config.general.profiles[0].set_button(Button::West, mapped);
    let back: Config = toml::from_str(&toml::to_string_pretty(&config).unwrap()).unwrap();
    assert_eq!(config, back);

    let mut names = Vec::new();
    for a in config.general.profiles[0].actions() {
        a.walk(&mut |a| {
            if let ButtonAction::Macro { name, .. } = a {
                names.push(name.clone());
            }
        });
    }
    assert_eq!(names, ["Combo"]);
    assert_eq!(config.general.macros[0].steps.iter().map(MacroStep::duration_ms).sum::<u64>(), 160);
}

#[test]
fn stick_direction_buttons_and_macro_stick_steps_roundtrip() {
    let mut config = Config::default();
    config.general.profiles[0].set_button(Button::RightStickUp, ButtonAction::Macro { name: "Jump".into(), repeat: false });
    config.general.profiles[0].set_button(Button::DpadRight, ButtonAction::Gamepad(Button::LeftStickRight));
    config.general.profiles[0].combos.push(Combo {
        buttons: vec![Button::LeftBumper, Button::RightStickRight],
        action: ButtonAction::Keys(vec!["KEY_F".into()]),
        window_ms: None,
    });
    config.general.macros.push(Macro {
        name: "Jump".into(),
        steps: vec![MacroStep::Stick { stick: Stick::Left, x: 0.0, y: -1.0 }, MacroStep::Wait(17)],
    });
    let back: Config = toml::from_str(&toml::to_string_pretty(&config).unwrap()).unwrap();
    assert_eq!(config, back);
    let found = config.general.profiles[0].actions().into_iter().any(|a| matches!(a, ButtonAction::Macro { .. }));
    assert!(found, "stick-direction mappings are visited like any button");
}

#[test]
fn stick_direction_buttons_cover_every_direction_once() {
    assert_eq!(Button::EVERY.len(), Button::ALL.len() + 8);
    for s in [Stick::Left, Stick::Right] {
        let dirs = Button::stick_directions(s);
        assert!(dirs.iter().all(|b| b.stick_direction().is_some_and(|(stick, _)| stick == s)));
    }
    assert!(Button::ALL.iter().all(|b| b.stick_direction().is_none()), "ALL stays physical buttons only");
}

#[test]
fn grid_spans_roundtrip() {
    let mut game = Game::new("g", vec![Profile::passthrough("p")]);
    let mut item = MenuItem::new("x", ButtonAction::OpenMenu("M".into()));
    item.span = Span { columns: 2, rows: 3 };
    game.menus.push(Menu { name: "M".into(), kind: MenuKind::Grid { columns: 3 }, items: vec![item], cancel: None, no_cancel: false, keep_open: false, start: 4, style: OverlayStyle::default() });
    let back: Game = toml::from_str(&toml::to_string_pretty(&game).unwrap()).unwrap();
    assert_eq!(back, game);
}

#[test]
fn menus_roundtrip_and_pick_a_sensible_cancel_button() {
    let item = |label: &str, action| MenuItem::new(label, action);
    let mut config = Config::default();
    config.general.menus = vec![
        Menu {
            name: "Weapons".into(),
                kind: MenuKind::Radial { stick: Stick::Right, boxes: false },
            items: (1..=4).map(|n| item(&format!("Slot {n}"), ButtonAction::Keys(vec![format!("KEY_{n}")]))).collect(),
            cancel: None, no_cancel: false, keep_open: false, start: 0,
            style: OverlayStyle::default(),
        },
        Menu {
            name: "Pause".into(),
                kind: MenuKind::Buttons,
            items: vec![MenuItem { button: Some(Button::LeftBumper), ..item("Map", ButtonAction::Keys(vec!["KEY_M".into()])) }],
            cancel: Some(Button::Start), no_cancel: false, keep_open: false, start: 0,
            style: OverlayStyle { position: ScreenPosition::TopRight, scale: 1.5, ..OverlayStyle::default() },
        },
        Menu {
            name: "Faces".into(),
                kind: MenuKind::Directional { cluster: Cluster::FaceButtons },
            items: vec![item("More", ButtonAction::OpenMenu("Pause".into()))],
            cancel: None, no_cancel: false, keep_open: false, start: 0,
            style: OverlayStyle::default(),
        },
        ];
    config.general.profiles[0].set_button(Button::Select, ButtonAction::OpenMenu("Pause".into()));
    let back: Config = toml::from_str(&toml::to_string_pretty(&config).unwrap()).unwrap();
    assert_eq!(config, back);
    assert_eq!(config.general.menus[0].cancel_button(), None, "a radial menu goes as it's let go");
    assert_eq!(config.general.menus[1].cancel_button(), Some(Button::Start));
    assert_eq!(config.general.menus[2].cancel_button(), None, "nor does a directional one");
    let list = Menu { kind: MenuKind::List, ..config.general.menus[0].clone() };
    assert_eq!(list.cancel_button(), Some(Button::East), "a list closes with B");
    let none = Menu { no_cancel: true, ..config.general.menus[1].clone() };
    assert_eq!(none.cancel_button(), None, "a chosen button doesn't count once there's none");
}

#[test]
fn unstyled_menus_load_with_the_default_style() {
    let text = r#"
        name = "Plain"
        items = []
        [kind.directional]
        cluster = "DPad"
    "#;
    let menu: Menu = toml::from_str(text).unwrap();
    assert_eq!(menu.kind, MenuKind::Directional { cluster: Cluster::DPad });
    assert_eq!(menu.style, OverlayStyle::default());
    assert_eq!(Config::default().keyboard_style.position, ScreenPosition::BottomCenter);
    assert_eq!(Config::default().numpad_style.position, ScreenPosition::BottomCenter);
    // A game follows the global styles until it sets its own.
    let mut config = Config::default();
    config.games.push(Game::new("Doom", vec![Profile::passthrough("Play")]));
    config.active = ProfileRef::new(Some("Doom"), "Play");
    assert_eq!(config.active_keyboard_style(), &config.keyboard_style);
    config.games[0].numpad_style = Some(OverlayStyle { scale: 1.5, ..OverlayStyle::numpad() });
    assert_eq!(config.active_numpad_style().scale, 1.5);
    assert_eq!(config.active_keyboard_style(), &config.keyboard_style);
    let again: Config = toml::from_str(&toml::to_string(&config).unwrap()).unwrap();
    assert_eq!(again.games[0].numpad_style, config.games[0].numpad_style);
}

#[test]
fn hex_colors_parse_strictly() {
    assert_eq!(parse_hex("#2f5db0"), Some([0x2f, 0x5d, 0xb0]));
    assert_eq!(parse_hex(" #FFFFFF "), Some([255, 255, 255]));
    for bad in ["2f5db0", "#2f5db", "#2f5dbz", "", "#"] {
        assert_eq!(parse_hex(bad), None, "{bad}");
    }
}

#[test]
fn templates_roundtrip_through_toml() {
    let config = Config { general: Game::new("General", super::templates()), ..Config::default() };
    let back: Config = toml::from_str(&toml::to_string_pretty(&config).unwrap()).unwrap();
    assert_eq!(config, back);
}

#[test]
fn pad_pushes_read_as_full_unless_an_amount_is_given() {
    let full: ButtonAction = toml::from_str::<std::collections::BTreeMap<String, ButtonAction>>("a = { pad_trigger = { trigger = \"Left\" } }\nb = { pad_stick = { stick = \"Right\", angle = 47.0 } }")
        .unwrap()
        .into_values()
        .next()
        .unwrap();
    assert_eq!(full, ButtonAction::PadTrigger { trigger: Trigger::Left, amount: 1.0 });
    let stick = ButtonAction::PadStick { stick: Stick::Right, angle: 47.0, amount: 0.5 };
    let text = toml::to_string(&std::collections::BTreeMap::from([("x", stick.clone())])).unwrap();
    assert_eq!(toml::from_str::<std::collections::BTreeMap<String, ButtonAction>>(&text).unwrap()["x"], stick);
    assert_eq!(crate::config::stick_vector(90.0, 1.0), (1.0, 0.0));
}

#[test]
fn cycles_follow_renames_of_themselves_and_the_macros_in_their_steps() {
    let mut game = Game::new("Anno", vec![Profile::passthrough("Pad")]);
    game.macros.push(Macro { name: "Fast".into(), steps: vec![] });
    game.cycles.push(Cycle { name: "Speed".into(), steps: vec![ButtonAction::Macro { name: "Fast".into(), repeat: false }, ButtonAction::Keys(vec!["KEY_1".into()])] });
    game.profiles[0].set_button(Button::North, ButtonAction::Cycle { name: "Speed".into(), back: false });
    game.rename_item(ItemKind::Macro, "Fast", "Faster");
    game.rename_refs(ItemKind::Macro, "Fast", "Faster");
    assert_eq!(game.cycles[0].steps[0], ButtonAction::Macro { name: "Faster".into(), repeat: false });
    game.rename_item(ItemKind::Cycle, "Speed", "Game speed");
    game.rename_refs(ItemKind::Cycle, "Speed", "Game speed");
    assert_eq!(game.profiles[0].button(Button::North), &ButtonAction::Cycle { name: "Game speed".into(), back: false });
    assert_eq!(game.names(ItemKind::Cycle), ["Game speed"]);
    assert!(crate::pack::game_references(&game).contains(&(ItemKind::Macro, "Faster".into())), "a cycle's steps count as references");
}

#[test]
fn a_cycle_action_writes_back_only_when_it_steps_back() {
    let mut config = Config::default();
    config.general.profiles[0].set_button(Button::RightBumper, ButtonAction::Cycle { name: "Speed".into(), back: false });
    config.general.profiles[0].set_button(Button::LeftBumper, ButtonAction::Cycle { name: "Speed".into(), back: true });
    config.general.cycles.push(Cycle { name: "Speed".into(), steps: vec![ButtonAction::Keys(vec!["KEY_1".into()])] });
    let text = toml::to_string_pretty(&config).unwrap();
    assert_eq!(text.matches("back = true").count(), 1, "{text}");
    assert!(!text.contains("back = false"), "{text}");
    let back: Config = toml::from_str(&text).unwrap();
    assert_eq!(back, config);
}

#[test]
fn saving_refuses_a_cycle_step_that_isnt_an_input_and_a_missing_cycle() {
    let mut config = Config::default();
    config.general.cycles.push(Cycle { name: "Speed".into(), steps: vec![ButtonAction::GyroToggle] });
    let problem = crate::config::problems::game_problem(&config, &config.general);
    assert_eq!(problem.as_deref(), Some("Cycle “Speed”: a step can only press inputs or play macros."));
    config.general.cycles[0].steps[0] = ButtonAction::Multi(vec![ButtonAction::Keys(vec!["KEY_1".into()]), ButtonAction::Gamepad(Button::South)]);
    assert_eq!(crate::config::problems::game_problem(&config, &config.general), None);
    config.general.profiles[0].set_button(Button::North, ButtonAction::Cycle { name: "Gone".into(), back: false });
    let problem = crate::config::problems::game_problem(&config, &config.general).unwrap();
    assert!(problem.contains("missing cycle “Gone”"), "{problem}");
}

#[test]
fn dpad_ring_presses_pad_directions_with_diagonals_pressing_both() {
    let actions = StickAction::dpad().ring_actions().to_vec();
    assert_eq!(actions.len(), 8);
    assert_eq!(actions[0], ButtonAction::Gamepad(Button::DpadUp));
    assert_eq!(actions[1], ButtonAction::Multi(vec![ButtonAction::Gamepad(Button::DpadUp), ButtonAction::Gamepad(Button::DpadRight)]));
}
