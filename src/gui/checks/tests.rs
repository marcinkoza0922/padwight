use super::*;

#[test]
fn button_menu_items_nothing_can_pick_are_warned_about() {
    let item = |button| MenuItem { button, ..MenuItem::new("x", ButtonAction::Disabled) };
    let menu = |name: &str, items| Menu { name: name.into(), kind: MenuKind::Buttons, items, cancel: None, no_cancel: false, keep_open: false, start: 0, style: OverlayStyle::default() };
    let quick = menu("Quick", vec![item(Some(Button::LeftBumper)), item(Some(Button::South)), item(Some(Button::South)), item(None)]);
    let sub = menu("Sub", vec![item(Some(Button::RightBumper))]);
    let mut profile = Profile::passthrough("p");
    profile.set_button(Button::LeftBumper, ButtonAction::OpenMenu("Quick".into()));
    profile.combos.push(Combo { buttons: vec![Button::Select, Button::RightBumper], action: ButtonAction::OpenMenu("Quick".into()) });
    let mut game = Game::new("g", vec![profile]);
    game.menus = vec![quick.clone(), sub.clone()];
    game.menus[0].items[1].action = ButtonAction::OpenMenu("Sub".into());

    let openers = menu_openers(&game, "Quick");
    assert_eq!(openers, [vec![Button::LeftBumper], vec![Button::Select, Button::RightBumper]]);
    assert!(button_item_warning(&quick, 0, &openers).is_some_and(|w| w.contains("opens this menu")));
    assert_eq!(button_item_warning(&quick, 1, &openers), None);
    assert!(button_item_warning(&quick, 2, &openers).is_some_and(|w| w.contains("item 2")));
    assert!(button_item_warning(&quick, 3, &openers).is_some_and(|w| w.contains("no button")));
    // A submenu is held up by its parent's openers too.
    assert!(button_item_warning(&sub, 0, &menu_openers(&game, "Sub")).is_some_and(|w| w.contains("RB opens")));
    // Other kinds of menu have no item buttons to warn about.
    let list = Menu { kind: MenuKind::List, ..quick };
    assert_eq!(button_item_warning(&list, 3, &openers), None);
}

#[test]
fn problems_are_found_inside_nested_actions_and_flag_their_section() {
    let names = Names { macros: vec!["Known".to_string()], modes: vec!["Guide".to_string()], ..Names::default() };
    let nested = ButtonAction::Multi(vec![
        ButtonAction::Mouse(MouseButton::Left),
        ButtonAction::toggle(ButtonAction::Keys(vec!["KEY_NOPE".into()])),
    ]);
    assert_eq!(action_problem(&nested, &names).as_deref(), Some("unknown key \u{201c}NOPE\u{201d}"));
    let missing = ButtonAction::Macro { name: "Gone".into(), repeat: false };
    assert_eq!(action_problem(&missing, &names).as_deref(), Some("missing macro \u{201c}Gone\u{201d}"));
    assert_eq!(action_problem(&ButtonAction::Macro { name: "Known".into(), repeat: false }, &names), None);
    let toggled_menu = ButtonAction::toggle(ButtonAction::OpenMenu("M".into()));
    assert!(action_problem(&toggled_menu, &Names { menus: vec!["M".into()], ..Names::default() }).is_some_and(|p| p.contains("can't be toggled")));
    // A turbo's gap must cover its macro's length; a longer macro is caught here.
    let turbo = |every_ms| ButtonAction::Turbo {
        action: Box::new(ButtonAction::Macro { name: "Long".into(), repeat: false }),
        rate: 10.0,
        every_ms,
    };
    let long = Names { macros: vec!["Long".into()], macro_ms: [("Long".to_string(), 300)].into(), ..Names::default() };
    assert!(action_problem(&turbo(200), &long).is_some_and(|p| p.contains("raise the turbo's gap")));
    assert_eq!(action_problem(&turbo(300), &long), None);

    let mut p = Profile::passthrough("p");
    assert!(ProfileTab::ALL.iter().all(|t| !section_has_problem(&p, *t, &names)));
    p.set_button(Button::RightStickUp, missing.clone());
    assert!(!section_has_problem(&p, ProfileTab::Sticks, &names), "stick directions have no actions to check");
    assert!(!section_has_problem(&p, ProfileTab::Buttons, &names));
    // A button ring's sector does, and flags the Sticks tab.
    p.stick_mut(Stick::Right).action = StickAction::ring(8);
    let StickAction::Ring { actions, .. } = &mut p.stick_mut(Stick::Right).action else { unreachable!() };
    actions[0] = missing;
    assert!(section_has_problem(&p, ProfileTab::Sticks, &names), "ring sectors live on the Sticks tab");
    p.combos.push(Combo { buttons: vec![Button::South], action: ButtonAction::Disabled });
    assert!(section_has_problem(&p, ProfileTab::Combos, &names));
}

#[test]
fn two_combos_of_the_same_buttons_are_refused() {
    let names = Names::default();
    let combo = |buttons: Vec<Button>, key: &str| Combo { buttons, action: ButtonAction::Keys(vec![key.into()]) };
    let mut p = Profile::passthrough("p");
    p.combos = vec![combo(vec![Button::LeftBumper, Button::RightBumper], "KEY_A"), combo(vec![Button::South, Button::East], "KEY_B")];
    assert!(!section_has_problem(&p, ProfileTab::Combos, &names));
    // The same buttons in another order are the same combo.
    p.combos.push(combo(vec![Button::RightBumper, Button::LeftBumper], "KEY_C"));
    assert!(section_has_problem(&p, ProfileTab::Combos, &names));
    assert_eq!(combo_problem(&p.combos, 0), None, "the first one is fine");
    assert!(combo_problem(&p.combos, 2).is_some_and(|w| w.contains("Several at once")));
    let mut game = Game::new("g", vec![p.clone()]);
    assert_eq!(game_problem(&Config::default(), &game).as_deref(), Some("Profile “p”: two combos are RB + LB. Give one of them “Several at once…” to do both things."));

    // One combo doing both is the way.
    let both = ButtonAction::Multi(vec![ButtonAction::Keys(vec!["KEY_A".into()]), ButtonAction::Keys(vec!["KEY_C".into()])]);
    game.profiles[0].combos.remove(2);
    game.profiles[0].combos[0].action = both;
    assert_eq!(game_problem(&Config::default(), &game), None);

    // A layer's combos are held to the same rule; one with a profile's buttons replaces it.
    let mut layer = crate::config::Layer::new("Lean");
    layer.combos = vec![combo(vec![Button::LeftBumper, Button::RightBumper], "KEY_D")];
    game.layers.push(layer);
    assert_eq!(game_problem(&Config::default(), &game), None);
    game.layers[0].combos.push(combo(vec![Button::LeftBumper, Button::RightBumper], "KEY_E"));
    assert!(game_problem(&Config::default(), &game).is_some_and(|w| w.starts_with("Layer “Lean”: two combos")));
}
