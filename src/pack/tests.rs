use super::*;
use crate::config::{Button, MenuItem, MenuKind, OverlayStyle, ProfileRef, RuleKind};

fn mac(name: &str) -> Macro {
    Macro { name: name.into(), steps: vec![MacroStep::Wait(10)] }
}

fn menu(name: &str, items: Vec<ButtonAction>) -> Menu {
    Menu {
        name: name.into(),
        kind: MenuKind::List,
        items: items.into_iter().map(|action| MenuItem::new("x", action)).collect(),
        cancel: None, no_cancel: false, keep_open: false, start: 0,
        style: OverlayStyle::default(),
    }
}

fn macro_ref(name: &str) -> ButtonAction {
    ButtonAction::Macro { name: name.into(), repeat: false }
}

/// A game whose profile opens its menu "Wheel", which plays its macro "Heal".
fn setup() -> Config {
    let mut config = Config::default();
    let mut game = Game::new("Doom", vec![Profile::pc_action("Play")]);
    game.profiles[0].set_button(Button::Select, ButtonAction::OpenMenu("Wheel".into()));
    game.macros = vec![mac("Dodge"), mac("Heal")];
    game.menus = vec![menu("Wheel", vec![macro_ref("Heal")])];
    game.rules = vec![Rule::new(RuleKind::Executable, "doom.exe", "Play")];
    config.games.push(game);
    config
}

#[test]
fn profiles_state_their_own_needs_and_old_packs_are_upgraded() {
    let mut config = setup();
    let game = &mut config.games[0];
    game.profiles[0].requires = vec![Feature::Gyro];
    game.profiles.push(Profile::passthrough("Plain"));
    let out = export(game, &draft(game, false));
    assert_eq!(out.features, [FeatureUse { feature: Feature::Gyro, place: "profile “Play”".into(), required: true }]);
    assert_eq!(out.pack.required_by_all(), []);
    assert_eq!(out.pack.needing(Feature::Gyro), ["Play"]);
    let back = parse(&out.pack.to_toml().unwrap()).unwrap();
    assert_eq!(back.profiles[0].requires, [Feature::Gyro]);
    assert!(back.profiles[1].requires.is_empty());

    // Format 7 listed the need for the whole pack: it lands on the gyro profiles.
    let mut old = out.pack.clone();
    old.format = 7;
    old.profiles.iter_mut().for_each(|p| p.requires.clear());
    let text = old.to_toml().unwrap().replacen("[pack]\n", "[pack]\nrequires = [\"gyro\"]\n", 1);
    let upgraded = parse(&text).unwrap();
    assert_eq!(upgraded.profiles[0].requires, [Feature::Gyro]);
    assert!(upgraded.profiles[1].requires.is_empty());
    assert!(upgraded.pack.requires.is_empty());
}

#[test]
fn export_reports_problems() {
    let mut config = setup();
    config.games[0].profiles[0].set_button(Button::North, macro_ref("Gone"));
    let game = &config.games[0];
    let info = draft(game, false);
    let out = export(game, &info);
    assert_eq!(out.dangling, ["macro \"Gone\""]);
    assert!(out.pack.pack.requires.is_empty(), "a pack doesn't state needs; its profiles do");
    let uses = FeatureUse { feature: Feature::Gyro, place: "profile “Play”".into(), required: false };
    assert_eq!(out.features, [uses], "the action template aims with gyro, which it can do without");
    assert_eq!(out.pack.pack.name, "Doom");
}

#[test]
fn packs_roundtrip_and_newer_formats_are_refused() {
    let config = setup();
    let out = export(&config.games[0], &draft(&config.games[0], false));
    let text = out.pack.to_toml().unwrap();
    assert_eq!(parse(&text).unwrap(), out.pack);

    let newer = text.replacen(&format!("format = {FORMAT}"), &format!("format = {}", FORMAT + 1), 1);
    assert!(parse(&newer).unwrap_err().to_string().contains("newer version of the app"));
    let extra = format!("surprise = 1\n{text}");
    assert!(parse(&extra).is_err(), "strict within a format");
    assert!(parse("[pack]\nid = 'x'").unwrap_err().to_string().contains("no format number"));
}

#[test]
#[expect(clippy::cognitive_complexity, reason = "predates the size lints")]
fn importing_twice_renames_then_updates_by_id() {
    let mut config = setup();
    let mut pack = export(&config.games[0], &draft(&config.games[0], false)).pack;
    pack.pack.version = "1.0".into();

    // "Doom" exists (made by the user, no origin): a name clash, renamed by default.
    let p = plan(&config, pack.clone(), false);
    assert!(p.name_clash && p.update_of.is_none());
    assert_eq!(p.renamed, "Doom (2)");
    // Its rule matches the user's own Doom rule.
    assert_eq!(p.rule_clashes, [RuleClash { rule: 0, other_game: "Doom".into() }]);
    let name = apply(&mut config, &p, &Choices::default());
    assert_eq!(name, "Doom (2)");
    assert!(!config.games[0].rules[0].enabled, "the import's rule took over");
    assert!(config.games[1].rules[0].enabled);
    // Its items keep their names: they're the setup's own.
    let imported = &config.games[1];
    assert_eq!(imported.menus[0].items[0].action, macro_ref("Heal"));
    assert_eq!(imported.profiles[0].button(Button::Select), &ButtonAction::OpenMenu("Wheel".into()));

    // The same ID again is an update of "Doom (2)", which the user has since edited.
    config.games[1].macros[0].steps.push(MacroStep::Wait(5));
    config.games[1].rules[0].enabled = false;
    pack.pack.version = "1.1".into();
    let p = plan(&config, pack.clone(), false);
    assert_eq!(p.update_of.as_deref(), Some("Doom (2)"));
    assert_eq!(p.update_kind, Some(UpdateKind::Update));
    assert_eq!(p.edited, ["macro “Dodge”"]);
    assert!(p.rule_clashes.is_empty(), "rules the setup had keep their decision");
    apply(&mut config, &p, &Choices::default());
    assert_eq!(config.games.len(), 2);
    assert_eq!(config.games[1].origin.as_ref().unwrap().version, "1.1");
    assert!(!config.games[1].rules[0].enabled, "switched off before, stays off");
    assert!(edited_items(&config.games[1]).is_empty());

    pack.pack.version = "1.0.9".into();
    assert_eq!(plan(&config, pack, false).update_kind, Some(UpdateKind::Downgrade));
}

#[test]
fn replacing_and_keeping_my_rule() {
    let mut config = setup();
    let mut pack = export(&config.games[0], &draft(&config.games[0], false)).pack;
    pack.pack.id = new_id();
    let p = plan(&config, pack, false);
    assert_eq!(p.rule_clashes.len(), 1, "with the same-named setup, until it's replaced");
    let choices = Choices { replace: true, keep_mine: vec![true] };
    assert_eq!(live_clashes(&p, &choices).count(), 0);
    assert_eq!(apply(&mut config, &p, &choices), "Doom");
    assert_eq!(config.games.len(), 1, "replaced in place");
    assert!(config.games[0].rules[0].enabled, "no clash left with the setup it replaced");
    assert!(config.games[0].origin.is_some());
}

#[test]
fn forks_get_a_new_id_and_credit_the_original() {
    let mut game = Game::new("Doom", vec![Profile::passthrough("P")]);
    let own = draft(&game, false);
    assert!(!own.id.is_empty() && own.based_on.is_none());
    game.pack = own.clone();
    assert_eq!(draft(&game, false).id, own.id, "my own pack keeps its ID");

    game.pack = PackInfo { id: "abc".into(), author: "them".into(), version: "2.0".into(), ..PackInfo::default() };
    game.origin = Some(Origin { id: "abc".into(), version: "2.0".into(), ..Origin::default() });
    let fork = draft(&game, false);
    assert_ne!(fork.id, "abc");
    assert_eq!(fork.based_on, Some(PackRef { id: "abc".into(), name: "Doom".into(), author: "them".into(), version: "2.0".into() }));
    assert_eq!(fork.author, "");
    assert_eq!(draft(&game, true).id, "abc", "the library keeps its pack IDs");

    // Imported next to another Doom, it was renamed; the credit uses the pack's own name.
    game.name = "Doom (2)".into();
    game.origin.as_mut().unwrap().name = "Doom".into();
    assert_eq!(draft(&game, false).based_on.unwrap().name, "Doom");
}

#[test]
fn copies_bring_what_they_refer_to_unless_already_there() {
    let mut source = Game::new("Source", Vec::new());
    source.macros = vec![mac("Heal"), mac("Dodge")];
    source.menus = vec![menu("Wheel", vec![macro_ref("Heal"), ButtonAction::OpenMenu("More".into())]), menu("More", vec![macro_ref("Dodge")])];
    let none = |_: ItemKind, _: &str| false;
    let all = dependencies(&source, ItemKind::Menu, "Wheel", none);
    let want = |list: &[(ItemKind, &str)]| list.iter().map(|(k, n)| (*k, n.to_string())).collect::<Vec<_>>();
    assert_eq!(all, want(&[(ItemKind::Menu, "Wheel"), (ItemKind::Macro, "Heal"), (ItemKind::Menu, "More"), (ItemKind::Macro, "Dodge")]));
    let has_heal = |k: ItemKind, n: &str| k == ItemKind::Macro && n == "Heal";
    assert_eq!(
        dependencies(&source, ItemKind::Menu, "Wheel", has_heal),
        want(&[(ItemKind::Menu, "Wheel"), (ItemKind::Menu, "More"), (ItemKind::Macro, "Dodge")])
    );
    assert!(dependencies(&source, ItemKind::Macro, "Nope", none).is_empty());
}

#[test]
fn layers_travel_in_packs_with_what_they_use() {
    let mut config = setup();
    config.games[0].profiles[0].gyro = crate::config::GyroConfig::default();
    let mut layer = Layer::new("Hotkeys");
    layer.buttons.insert(Button::South, macro_ref("Heal"));
    layer.buttons.insert(Button::East, ButtonAction::Layer("Deeper".into()));
    layer.indicator = Indicator::Info("Cheat sheet".into());
    let mut deeper = Layer::new("Deeper");
    deeper.gyro = Some(crate::config::GyroConfig { mode: crate::config::GyroMode::Mouse { sensitivity: 10.0 }, ..Default::default() });
    config.games[0].layers = vec![layer.clone(), deeper];
    config.games[0].info.push(InfoOverlay { name: "Cheat sheet".into(), always: false, on_start: None, linger: None, title: None, current_input: Default::default(), style: Default::default(), rows: vec![] });

    let out = export(&config.games[0], &draft(&config.games[0], false));
    assert!(out.dangling.is_empty(), "the indicator's info overlay is found: {:?}", out.dangling);
    assert_eq!(out.features, [FeatureUse { feature: Feature::Gyro, place: "layer “Deeper”".into(), required: false }]);
    let text = out.pack.to_toml().unwrap();
    assert!(text.contains(&format!("format = {FORMAT}")));
    let back = parse(&text).unwrap();
    assert_eq!(back.layers.len(), 2);
    assert_eq!(back.to_game().layers[0], layer);

    // Copying the layer brings the macro it plays, its indicator and the layer it holds.
    let needs = dependencies(&config.games[0], ItemKind::Layer, "Hotkeys", |_, _| false);
    let want = [(ItemKind::Layer, "Hotkeys"), (ItemKind::Macro, "Heal"), (ItemKind::Info, "Cheat sheet"), (ItemKind::Layer, "Deeper")];
    assert_eq!(needs, want.map(|(k, n)| (k, n.to_string())));
}

#[test]
fn paddles_are_listed_when_used_and_when_required() {
    let mut config = setup();
    let game = &mut config.games[0];
    game.profiles[0].set_button(Button::LeftPaddle, ButtonAction::Gamepad(Button::South));
    let paddle = |required| FeatureUse { feature: Feature::BackPaddles, place: "profile “Play”".into(), required };
    let out = export(game, &draft(game, false));
    assert!(out.features.contains(&paddle(false)), "a paddle mapping is listed as an undeclared use");
    assert!(!out.pack.needing(Feature::BackPaddles).contains(&"Play"));

    game.profiles[0].requires = vec![Feature::BackPaddles];
    let out = export(game, &draft(game, false));
    assert!(out.features.contains(&paddle(true)));
    assert_eq!(out.pack.needing(Feature::BackPaddles), ["Play"]);

    // A paddle set to nothing isn't a use.
    game.profiles[0].requires.clear();
    game.profiles[0].set_button(Button::LeftPaddle, ButtonAction::Disabled);
    let out = export(game, &draft(game, false));
    assert!(!out.features.iter().any(|u| u.feature == Feature::BackPaddles));
}

#[test]
fn a_touchpad_setup_is_listed_and_a_declared_need_travels() {
    let mut config = setup();
    let game = &mut config.games[0];
    let out = export(game, &draft(game, false));
    assert!(!out.features.iter().any(|u| u.feature == Feature::Touchpad), "the default touchpad isn't a use");

    game.profiles[0].touchpad.motion = crate::config::TouchpadMotion::Off;
    let touchpad = |required| FeatureUse { feature: Feature::Touchpad, place: "profile “Play”".into(), required };
    let out = export(game, &draft(game, false));
    assert!(out.features.contains(&touchpad(false)), "a changed touchpad is listed as an undeclared use");

    game.profiles[0].requires = vec![Feature::Touchpad];
    let out = export(game, &draft(game, false));
    assert!(out.features.contains(&touchpad(true)));
    assert_eq!(out.pack.needing(Feature::Touchpad), ["Play"]);
    let back = parse(&out.pack.to_toml().unwrap()).unwrap();
    assert_eq!(back.profiles[0].requires, [Feature::Touchpad]);
}

#[test]
fn the_overlay_font_travels_in_packs() {
    let mut config = setup();
    config.games[0].overlay_font = Some("Comfortaa".into());
    let out = export(&config.games[0], &draft(&config.games[0], false));
    let pack = parse(&out.pack.to_toml().unwrap()).unwrap();
    assert_eq!(pack.to_game().overlay_font.as_deref(), Some("Comfortaa"));
}

#[test]
fn controller_support_travels_in_packs() {
    use crate::pad_identity::ControllerSupport;
    let mut config = setup();
    let plain = export(&config.games[0], &draft(&config.games[0], false)).pack.to_toml().unwrap();
    assert!(!plain.contains("controllers"), "a game that ticks nothing leaves it out");
    let support = ControllerSupport { dualsense: true, switch_pro: true, ..Default::default() };
    config.games[0].controllers = Some(support);
    let out = export(&config.games[0], &draft(&config.games[0], false));
    let pack = parse(&out.pack.to_toml().unwrap()).unwrap();
    assert_eq!(pack.to_game().controllers, Some(support));
}

#[test]
fn keyboard_and_numpad_styles_travel_in_packs() {
    let mut config = setup();
    let style = OverlayStyle { scale: 1.5, ..OverlayStyle::keyboard() };
    config.games[0].keyboard_style = Some(style.clone());
    let out = export(&config.games[0], &draft(&config.games[0], false));
    let pack = parse(&out.pack.to_toml().unwrap()).unwrap();
    assert_eq!(pack.to_game().keyboard_style, Some(style));
    assert_eq!(pack.to_game().numpad_style, None);

    // Importing over a game that has its own numpad look keeps it when the pack sets none.
    let mut config = setup();
    let mut mine = pack.to_game();
    mine.name = "Mine".into();
    mine.numpad_style = Some(OverlayStyle { scale: 0.5, ..OverlayStyle::numpad() });
    config.games.push(mine.clone());
    let mut plan = plan(&config, pack, false);
    plan.update_of = Some("Mine".into());
    apply(&mut config, &plan, &Choices::default());
    let updated = config.games.iter().find(|g| g.name == "Mine").unwrap();
    assert_eq!(updated.numpad_style, mine.numpad_style);
    assert_eq!(updated.keyboard_style.as_ref().unwrap().scale, 1.5);
}

#[test]
fn media_and_menu_styles_travel_in_packs() {
    let mut config = setup();
    let style = OverlayStyle { scale: 1.2, ..OverlayStyle::media() };
    config.games[0].media_style = Some(style.clone());
    config.games[0].menu_style = Some(OverlayStyle::default());
    let out = export(&config.games[0], &draft(&config.games[0], false));
    let pack = parse(&out.pack.to_toml().unwrap()).unwrap();
    assert_eq!(pack.to_game().media_style, Some(style));
    assert_eq!(pack.to_game().menu_style, Some(OverlayStyle::default()));
    assert_eq!(pack.to_game().keyboard_style, None);
}

#[test]
fn version_order_and_ids() {
    use std::cmp::Ordering::*;
    assert_eq!(compare_versions("1.10", "1.9"), Greater);
    assert_eq!(compare_versions("1.0", "1"), Equal);
    assert_eq!(compare_versions("v2", "1.9.9"), Greater);
    assert_eq!(compare_versions("1.0-beta", "1.0"), Less, "pre-releases come before the release");
    assert_eq!(compare_versions("1.0-beta.2", "1.0-beta.10"), Less);
    assert_eq!(compare_versions("1.1-rc1", "1.0"), Greater);
    assert_eq!(compare_versions("1.0+linux", "1.0"), Equal);
    let id = new_id();
    assert_eq!(id.len(), 36);
    assert_eq!(&id[14..15], "4");
    assert_ne!(id, new_id());
}

#[test]
fn imported_games_can_become_active() {
    let mut config = setup();
    let mut pack = export(&config.games[0], &draft(&config.games[0], false)).pack;
    pack.pack.name = "Quake".into();
    pack.rules.clear();
    let plan = plan(&config, pack, false);
    let name = apply(&mut config, &plan, &Choices::default());
    config.active = ProfileRef::new(Some(&name), "Play");
    assert_eq!(config.active_ref(), ProfileRef::new(Some("Quake"), "Play"));
    assert!(config.scope().menus.iter().any(|m| m.name == "Wheel"));
}

#[test]
fn icons_and_sounds_travel_in_packs_and_bad_ones_are_refused() {
    let mut config = setup();
    config.games[0].assets.add_icon("bow-2", "<svg viewBox='0 0 8 8'><path d='M0 0h8v8z' fill='currentColor'/></svg>").unwrap();
    let wav: Vec<u8> = {
        let data: Vec<u8> = [0i16, 8000, -8000, 0].iter().flat_map(|s| s.to_le_bytes()).collect();
        let mut out = b"RIFF".to_vec();
        out.extend((36 + data.len() as u32).to_le_bytes());
        out.extend(b"WAVEfmt ");
        out.extend(16u32.to_le_bytes());
        out.extend([1, 0, 1, 0]);
        out.extend(44_100u32.to_le_bytes());
        out.extend(88_200u32.to_le_bytes());
        out.extend([2, 0, 16, 0]);
        out.extend(b"data");
        out.extend((data.len() as u32).to_le_bytes());
        out.extend(data);
        out
    };
    config.games[0].assets.add_sound("zap", &wav).unwrap();
    let text = export(&config.games[0], &draft(&config.games[0], false)).pack.to_toml().unwrap();
    let game = parse(&text).unwrap().to_game();
    assert_eq!(game.assets, config.games[0].assets);
    let plain = export(&setup().games[0], &draft(&setup().games[0], false)).pack.to_toml().unwrap();
    assert!(!plain.contains("assets"), "a game with none leaves them out");
    let bad = text.replace("<svg viewBox", "<script></script><svg viewBox");
    assert!(parse(&bad).is_err(), "an icon with a script in it is refused");
}

#[test]
fn a_pack_can_t_bring_its_own_built_in_modes_or_repeat_a_combo_button() {
    use crate::config::{GUIDE_MODE, MOUSE_MODE};
    let mut game = setup().games[0].clone();
    game.layers.push(Layer::new(MOUSE_MODE));
    game.modes = vec![Mode::new(GUIDE_MODE), Mode::new("Driving")];
    game.profiles[0].combos.push(Combo { buttons: vec![Button::West, Button::South, Button::West], action: ButtonAction::Screenshot });
    let pack = parse(&export(&game, &draft(&game, false)).pack.to_toml().unwrap()).unwrap();
    assert!(pack.layers.is_empty());
    assert_eq!(pack.modes.iter().map(|m| m.name.as_str()).collect::<Vec<_>>(), ["Driving"]);
    assert!(pack.profiles[0].combos.iter().any(|c| c.buttons == [Button::West, Button::South]));
}

#[test]
fn an_update_keeps_the_icons_the_player_added() {
    let mut old = setup().games[0].clone();
    old.assets.add_icon("mine", "<svg></svg>").unwrap();
    old.assets.add_icon("shared", "<svg>old</svg>").unwrap();
    let mut new = setup().games[0].clone();
    new.assets.add_icon("shared", "<svg>new</svg>").unwrap();
    let kept = keep_looks(new, &old);
    assert!(kept.assets.icons.contains_key("mine"));
    assert_eq!(kept.assets.icons["shared"], "<svg>new</svg>");
}

#[test]
fn a_pack_with_a_number_that_isn_t_finite_is_refused() {
    let mut config = setup();
    config.games[0].menu_style = Some(OverlayStyle { scale: f32::NAN, ..OverlayStyle::default() });
    let text = export(&config.games[0], &draft(&config.games[0], false)).pack.to_toml().unwrap();
    assert!(parse(&text).unwrap_err().to_string().contains("menu_style.scale"));
}
