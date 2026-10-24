use super::*;
use crate::config::{GyroMode, Layer, MouseResponse, Stick, StickAction, StickConfig};

fn press(b: Button) -> InputEvent {
    InputEvent::Button(b, true)
}

impl SystemMenu {
    fn handle_now(&mut self, ev: InputEvent, config: &mut Config) -> Option<Outcome> {
        self.handle(ev, Instant::now(), config)
    }
}

/// A config whose active profile has a mouse-moving left stick.
fn mouse_config() -> Config {
    let mut config = Config::default();
    let profile = &mut config.general.profiles[0];
    profile.left_stick.action = StickAction::Mouse { speed: 1000.0, response: MouseResponse::default(), invert_y: false };
    config
}

fn speed(config: &Config) -> f32 {
    match config.active().unwrap().left_stick.action {
        StickAction::Mouse { speed, .. } => speed,
        _ => panic!("left stick is not a mouse"),
    }
}

#[test]
fn the_guide_rows_open_their_page_directly() {
    let config = mouse_config();
    assert_eq!(SystemMenu::new(&config).on_page(Page::Quick).screen, Screen::Quick);
    assert!(matches!(SystemMenu::new(&config).on_page(Page::EditControls).screen, Screen::Editor(_)));
    assert_eq!(SystemMenu::new(&config).on_page(Page::Main).screen, Screen::Main);
}

#[test]
fn b_on_a_page_opened_from_the_guide_goes_back_to_the_guide() {
    let mut config = mouse_config();
    let mut quick = SystemMenu::new(&config).on_page(Page::Quick);
    assert_eq!(quick.handle_now(press(Button::East), &mut config), Some(Outcome::ReturnToGuide));
    let mut editor = SystemMenu::new(&config).on_page(Page::EditControls);
    assert_eq!(editor.handle_now(press(Button::East), &mut config), Some(Outcome::ReturnToGuide));
}

#[test]
fn main_menu_opens_quick_settings_first_and_edit_controls_last() {
    let mut config = mouse_config();
    let mut menu = SystemMenu::new(&config);
    assert_eq!(menu.handle_now(press(Button::South), &mut config), None);
    assert_eq!(menu.screen, Screen::Quick);
    assert_eq!(menu.handle_now(press(Button::East), &mut config), None);
    assert_eq!(menu.screen, Screen::Main);
    assert_eq!(menu.handle_now(press(Button::DpadDown), &mut config), None);
    assert_eq!(menu.handle_now(press(Button::South), &mut config), Some(Outcome::EditControls));
    // B leaves the editor for the main page, and B again closes the menu.
    assert_eq!(menu.handle_now(press(Button::East), &mut config), None);
    assert_eq!(menu.handle_now(press(Button::East), &mut config), Some(Outcome::Close));
}

#[test]
fn quick_settings_tunes_the_speeds_a_layer_or_mode_sets_itself() {
    let mut config = mouse_config();
    config.general.profiles.truncate(1);
    let base = config.general.profiles[0].left_stick.clone();
    let mut aim = Layer::new("Aim");
    aim.left_stick = Some(StickConfig::new(StickAction::Mouse { speed: 500.0, response: MouseResponse::default(), invert_y: false }, 0.1, 1.0));
    aim.gyro = Some(crate::config::GyroConfig { mode: GyroMode::Mouse { sensitivity: 5.0 }, ..Default::default() });
    config.general.layers.push(aim);
    let mut driving = Layer::new("Driving");
    driving.left_stick = Some(StickConfig::new(StickAction::Mouse { speed: 300.0, response: MouseResponse::default(), invert_y: false }, 0.1, 1.0));
    config.general.modes.push(driving);
    let rows = quick::rows(&config, &config);
    let (layer, mode) = (quick::Scope::Layer(0), quick::Scope::Mode(0));
    for row in [Row::ScopedStickSpeed(layer, Stick::Left), Row::ScopedGyroSensitivity(layer), Row::ScopedStickSpeed(mode, Stick::Left)] {
        assert!(rows.contains(&row), "{row:?} in {rows:?}");
    }
    assert_eq!(quick::label(Row::ScopedStickSpeed(layer, Stick::Left), &config, &config), "Left Stick speed, Aim layer: 500 px/s");
    assert_eq!(quick::label(Row::ScopedStickSpeed(mode, Stick::Left), &config, &config), "Left Stick speed, Driving mode: 300 px/s");
    let saved = config.clone();
    assert_eq!(quick::adjust(Row::ScopedStickSpeed(layer, Stick::Left), -1, false, &mut config, &saved), Some(Outcome::Changed));
    assert_eq!(quick::adjust(Row::ScopedGyroSensitivity(layer), 1, false, &mut config, &saved), Some(Outcome::Changed));
    assert!(quick::label(Row::ScopedStickSpeed(layer, Stick::Left), &config, &saved).contains("(was 500 px/s)"));
    assert_eq!(config.general.profiles[0].left_stick, base, "the base speed stays as it was");
    assert_eq!(config.general.modes[0], saved.general.modes[0]);
}

#[test]
fn quick_settings_only_lists_what_the_profile_uses() {
    let mut config = mouse_config();
    config.general.profiles.truncate(1);
    assert_eq!(quick::rows(&config, &config), [Row::StickSpeed(Stick::Left), Row::InvertY, Row::Deadzone(Stick::Left), Row::Deadzone(Stick::Right), Row::Rumble, Row::OverlaySize, Row::NintendoLayout, Row::Remapping]);
    // A second profile adds the profile switch.
    config.general.profiles.push(config.general.profiles[0].clone());
    config.general.profiles[1].name = "Other".into();
    assert_eq!(quick::rows(&config, &config).last(), Some(&Row::Remapping));
    assert!(quick::rows(&config, &config).contains(&Row::Profile));
}

#[test]
fn quick_nintendo_layout_sets_the_active_game_only() {
    let mut config = mouse_config();
    assert!(!config.active_nintendo_layout());
    // Picking the row turns it on for the game the menu is over, and leaves Settings alone.
    assert_eq!(quick::adjust(Row::NintendoLayout, 1, false, &mut config, &Config::default()), Some(Outcome::Changed));
    assert!(config.active_nintendo_layout());
    assert_eq!(config.general.nintendo_layout, Some(true));
    assert_eq!(quick::label(Row::NintendoLayout, &config, &config), "Nintendo button layout: on");
    assert_eq!(quick::adjust(Row::NintendoLayout, 1, false, &mut config, &Config::default()), Some(Outcome::Changed));
    assert_eq!(config.general.nintendo_layout, Some(false));
}

#[test]
fn closing_with_unsaved_changes_asks_first() {
    let mut config = mouse_config();
    let saved = config.clone();
    let mut menu = SystemMenu::new(&saved);
    // Change the stick speed: now the config differs from what was saved.
    menu.handle_now(press(Button::South), &mut config);
    menu.handle_now(press(Button::DpadRight), &mut config);
    menu.handle_now(press(Button::East), &mut config);
    assert_eq!(menu.screen, Screen::Main);
    assert_eq!(menu.handle_now(press(Button::East), &mut config), None);
    assert!(matches!(menu.screen, Screen::Confirm(..)));
    // B keeps editing, back on the main page.
    assert_eq!(menu.handle_now(press(Button::East), &mut config), None);
    assert_eq!(menu.screen, Screen::Main);
    // Closing again asks again: A saves and X discards, handing back what was saved.
    menu.handle_now(press(Button::East), &mut config);
    assert_eq!(menu.handle_now(press(Button::South), &mut config), Some(Outcome::Save { back: false }));
    assert_eq!(
        menu.handle_now(press(Button::West), &mut config),
        Some(Outcome::Discard { saved: Box::new(saved.clone()), back: false })
    );
}

#[test]
fn the_question_is_drawn_over_the_page_it_was_asked_on() {
    let mut config = mouse_config();
    let mut menu = SystemMenu::new(&config).on_page(Page::Quick);
    let plain = menu.view(&config);
    assert!(plain.prompt.is_none());
    menu.handle_now(press(Button::DpadRight), &mut config);
    menu.handle_now(press(Button::East), &mut config);
    let asked = menu.view(&config);
    assert_eq!((asked.title.as_str(), asked.crumbs.as_slice()), ("Quick Settings", &["Guide".to_string()][..]), "the page stays");
    let prompt = asked.prompt.expect("the question is on top of it");
    assert_eq!(prompt.answers.iter().map(|(_, t)| t.as_str()).collect::<Vec<_>>(), ["Save", "Discard", "Keep editing"]);
}

#[test]
fn answering_after_back_returns_to_the_guide_but_after_guide_it_closes() {
    let mut config = mouse_config();
    let mut menu = SystemMenu::new(&config).on_page(Page::Quick);
    menu.handle_now(press(Button::DpadRight), &mut config);
    menu.handle_now(press(Button::East), &mut config);
    assert_eq!(menu.handle_now(press(Button::South), &mut config), Some(Outcome::Save { back: true }));

    let mut menu = SystemMenu::new(&config).on_page(Page::Quick);
    menu.handle_now(press(Button::DpadRight), &mut config);
    assert_eq!(menu.guide_pressed(&config), None);
    assert_eq!(menu.handle_now(press(Button::South), &mut config), Some(Outcome::Save { back: false }));

    // Opened with Guide + Start, there is no Guide overlay to go back to.
    let mut menu = SystemMenu::new(&Config::default());
    menu.handle_now(press(Button::East), &mut config);
    assert_eq!(menu.handle_now(press(Button::South), &mut config), Some(Outcome::Save { back: false }));
}

#[test]
fn keep_editing_goes_back_to_the_page_that_was_left() {
    let mut config = mouse_config();
    let saved = config.clone();
    let mut menu = SystemMenu::new(&saved).on_page(Page::Quick);
    assert_eq!(menu.view(&config).crumbs, ["Guide"], "opened from the Guide overlay");
    menu.handle_now(press(Button::DpadDown), &mut config);
    menu.handle_now(press(Button::South), &mut config);
    assert_eq!(menu.handle_now(press(Button::East), &mut config), None, "unsaved, so it asks");
    assert!(matches!(menu.screen, Screen::Confirm(..)));
    assert_eq!(menu.view(&config).crumbs, ["Guide"]);
    menu.handle_now(press(Button::East), &mut config);
    assert_eq!(menu.screen, Screen::Quick, "B keeps editing on Quick Settings");
    assert_eq!(menu.cursor, 1, "on the row it was on");
}

#[test]
fn a_profile_switch_keeps_the_cursor_on_the_profile_row() {
    let mut config = mouse_config();
    config.general.profiles.truncate(1);
    let mut other = config.general.profiles[0].clone();
    other.name = "Other".into();
    other.left_stick.action = StickAction::Disabled;
    config.general.profiles.push(other);
    let mut menu = SystemMenu::new(&config).on_page(Page::Quick);
    menu.cursor = quick::rows(&config, &config).iter().position(|r| *r == Row::Profile).unwrap();
    let Some(Outcome::Switch(target)) = menu.handle_now(press(Button::South), &mut config) else { panic!("no switch") };
    config.active = target;
    menu.profile_switched(&config);
    assert_eq!(quick::rows(&config, &config)[menu.cursor], Row::Profile, "fewer rows now, still on Profile");
}

#[test]
fn guide_closes_the_menu_but_asks_about_unsaved_changes_first() {
    let mut config = mouse_config();
    let mut menu = SystemMenu::new(&config).on_page(Page::Quick);
    assert_eq!(menu.guide_pressed(&config), Some(Outcome::Close), "closes, not back to the Guide overlay");
    menu.handle_now(press(Button::DpadRight), &mut config);
    assert_eq!(menu.guide_pressed(&config), None);
    assert!(matches!(menu.screen, Screen::Confirm(..)));
    assert_eq!(menu.guide_pressed(&config), None, "the question needs an answer");
}

#[test]
fn stick_speed_steps_up_and_down() {
    let mut config = mouse_config();
    let mut menu = SystemMenu::new(&config);
    menu.handle_now(press(Button::South), &mut config);
    assert_eq!(menu.handle_now(press(Button::DpadRight), &mut config), Some(Outcome::Changed));
    assert!((speed(&config) - 1100.0).abs() < 0.01);
    assert_eq!(menu.handle_now(press(Button::DpadLeft), &mut config), Some(Outcome::Changed));
    assert!((speed(&config) - 1000.0).abs() < 0.01);
}

#[test]
fn holding_right_keeps_raising_the_speed() {
    let mut config = mouse_config();
    let mut menu = SystemMenu::new(&config).on_page(Page::Quick);
    let t0 = Instant::now();
    assert_eq!(menu.handle(press(Button::DpadRight), t0, &mut config), Some(Outcome::Changed));
    assert_eq!(menu.tick(t0, &mut config), None, "not before the delay");
    let delay = crate::nav::REPEAT_DELAY;
    assert_eq!(menu.tick(t0 + delay, &mut config), Some(Outcome::Changed));
    assert!((speed(&config) - 1210.0).abs() < 0.1, "{}", speed(&config));
    menu.handle(InputEvent::Button(Button::DpadRight, false), t0 + delay, &mut config);
    assert_eq!(menu.next_deadline(), None);
}

#[test]
fn rb_makes_big_steps_and_a_changed_number_says_what_it_was() {
    let mut config = mouse_config();
    let saved = config.clone();
    let mut menu = SystemMenu::new(&saved).on_page(Page::Quick);
    menu.handle_now(press(Button::RightBumper), &mut config);
    menu.handle_now(press(Button::DpadRight), &mut config);
    assert!((speed(&config) - 1500.0).abs() < 0.1, "{}", speed(&config));
    assert_eq!(menu.view(&config).items[0].label, "Left Stick speed: 1500 px/s (was 1000 px/s)");
    menu.handle_now(InputEvent::Button(Button::RightBumper, false), &mut config);
    menu.handle_now(press(Button::DpadRight), &mut config);
    assert!((speed(&config) - 1650.0).abs() < 0.1);
}

#[test]
fn invert_y_flips_every_flag_at_once() {
    let mut config = mouse_config();
    let mut menu = SystemMenu::new(&config);
    menu.handle_now(press(Button::South), &mut config);
    menu.handle_now(press(Button::DpadDown), &mut config);
    assert_eq!(menu.handle_now(press(Button::South), &mut config), Some(Outcome::Changed));
    let profile = config.active().unwrap();
    assert!(quick::inverted_y(profile));
    assert!(matches!(profile.left_stick.action, StickAction::Mouse { invert_y: true, .. }));
    assert!(profile.gyro.invert_y);
    menu.handle_now(press(Button::South), &mut config);
    assert!(!quick::inverted_y(config.active().unwrap()));
}

#[test]
fn stick_push_steps_once_until_let_go() {
    let mut config = mouse_config();
    let mut menu = SystemMenu::new(&config);
    menu.handle_now(InputEvent::Axis(Axis::LeftY, 0.9), &mut config);
    assert_eq!(menu.cursor, 1);
    menu.handle_now(InputEvent::Axis(Axis::LeftY, 1.0), &mut config);
    assert_eq!(menu.cursor, 1);
    menu.handle_now(InputEvent::Axis(Axis::LeftY, 0.0), &mut config);
    menu.handle_now(InputEvent::Axis(Axis::LeftY, -0.9), &mut config);
    assert_eq!(menu.cursor, 0);
}

#[test]
fn put_aside_while_asking_it_comes_back_where_the_changes_were_made() {
    let mut config = mouse_config();
    let mut menu = SystemMenu::new(&config).on_page(Page::Quick);
    menu.handle_now(press(Button::DpadDown), &mut config);
    menu.handle_now(press(Button::DpadRight), &mut config);
    menu.handle_now(press(Button::East), &mut config);
    assert!(matches!(menu.screen, Screen::Confirm(..)));
    menu.hold(Hold::RemappingOff);
    menu.put_aside();
    assert_eq!((menu.screen.clone(), menu.cursor, menu.on_hold()), (Screen::Quick, 1, None));
    assert!(menu.unsaved(&config), "the changes are still the menu's to save");
}

#[test]
fn on_hold_with_remapping_off_a_saves_and_x_discards() {
    let mut config = mouse_config();
    let mut menu = SystemMenu::new(&config).on_page(Page::Quick);
    menu.handle_now(press(Button::DpadRight), &mut config);
    // Turned off from the tray, which saves it: that alone isn't a change of the menu's.
    config.enabled = false;
    menu.remapping_set(false);
    assert!(menu.unsaved(&config), "the stick speed still is");
    menu.hold(Hold::RemappingOff);
    assert_eq!(menu.handle_now(press(Button::East), &mut config), None, "no keep editing");
    assert_eq!(menu.guide_pressed(&config), None);
    assert_eq!(menu.handle_now(press(Button::South), &mut config), Some(Outcome::Save { back: false }));
    let Some(Outcome::Discard { saved, back: false }) = menu.handle_now(press(Button::West), &mut config) else {
        panic!("X discards");
    };
    assert!(!saved.enabled, "Discard leaves remapping off");
    let mut unchanged = mouse_config();
    unchanged.enabled = false;
    assert!(!menu.unsaved(&unchanged));
}

/// General's Gamepad, its left stick moving the mouse, and Desktop.
fn two_profiles() -> Config {
    mouse_config()
}

#[test]
fn switch_profile_switches_then_leaves_without_asking_to_save() {
    let mut config = two_profiles();
    let mut menu = SystemMenu::new(&config).on_page(Page::Profiles);
    menu.handle_now(press(Button::DpadDown), &mut config);
    let Some(Outcome::Switch(target)) = menu.handle_now(press(Button::South), &mut config) else { panic!("no switch") };
    assert_eq!(target.profile, "Desktop");
    // The daemon switches, then asks the menu what follows.
    config.active = target;
    assert_eq!(menu.profile_switched(&config), Some(Outcome::Close));
    assert!(!menu.unsaved(&config));
}

#[test]
fn choosing_the_current_profile_just_leaves() {
    let mut config = two_profiles();
    let mut menu = SystemMenu::new(&config).on_page(Page::Profiles);
    assert_eq!(menu.handle_now(press(Button::South), &mut config), Some(Outcome::Close));
}

#[test]
fn a_new_profile_is_an_unsaved_change_and_back_returns_to_the_list() {
    let mut config = two_profiles();
    let mut menu = SystemMenu::new(&config).on_page(Page::Profiles);
    // Up wraps to "+ New profile", the last row.
    menu.handle_now(press(Button::DpadUp), &mut config);
    assert_eq!(menu.handle_now(press(Button::South), &mut config), None);
    assert_eq!(menu.screen, Screen::NewProfile);
    let Some(Outcome::Added(at)) = menu.handle_now(press(Button::South), &mut config) else { panic!("nothing added") };
    config.active = at.clone();
    assert_eq!(menu.profile_switched(&config), None);
    menu.profile_added(&config);
    assert_eq!((menu.screen.clone(), menu.cursor), (Screen::Profiles, config.general.profiles.len() - 1));
    assert!(menu.unsaved(&config));
    // Leaving asks about the new profile, and Discard goes back to the profile it was on.
    assert_eq!(menu.handle_now(press(Button::East), &mut config), None);
    let Some(Outcome::Discard { saved, .. }) = menu.handle_now(press(Button::West), &mut config) else { panic!("no discard") };
    assert_eq!(saved.active_ref().profile, config.general.profiles[0].name);
}

#[test]
fn a_name_is_typed_on_the_keyboard_and_set_with_enter() {
    let mut config = mouse_config();
    config.general.macros.push(crate::config::Macro { name: "Jumps".into(), steps: Vec::new() });
    let mut menu = SystemMenu::new(&config).on_page(Page::EditControls);
    // Macros, then the macro, then its name.
    let rows = menu.view(&config).items;
    let macros = rows.iter().position(|r| r.label.starts_with("Macros")).unwrap();
    for _ in 0..macros {
        menu.handle_now(press(Button::DpadDown), &mut config);
    }
    menu.handle_now(press(Button::South), &mut config);
    menu.handle_now(press(Button::South), &mut config);
    assert_eq!(menu.handle_now(press(Button::South), &mut config), None);
    assert!(menu.typing());
    assert!(matches!(menu.overlay_view(&config, &crate::figures::Live::default()), crate::overlay::OverlayView::Keyboard(k) if k.typed == "Jumps" && k.field.is_some()));
    // X deletes the last letter, and Start (Enter) sets it.
    menu.handle_now(press(Button::West), &mut config);
    menu.handle_now(InputEvent::Button(Button::West, false), &mut config);
    assert_eq!(menu.handle_now(press(Button::Start), &mut config), Some(Outcome::Changed));
    assert!(!menu.typing());
    assert_eq!(config.general.macros[0].name, "Jump");
}

#[test]
fn guide_puts_the_keyboard_away_without_setting_the_text() {
    let mut config = mouse_config();
    config.general.macros.push(crate::config::Macro { name: "Jumps".into(), steps: Vec::new() });
    let mut menu = SystemMenu::new(&config).on_page(Page::EditControls);
    let macros = menu.view(&config).items.iter().position(|r| r.label.starts_with("Macros")).unwrap();
    for _ in 0..macros {
        menu.handle_now(press(Button::DpadDown), &mut config);
    }
    for _ in 0..3 {
        menu.handle_now(press(Button::South), &mut config);
    }
    menu.handle_now(press(Button::West), &mut config);
    assert_eq!(menu.guide_pressed(&config), None);
    assert!(!menu.typing());
    assert_eq!(config.general.macros[0].name, "Jumps");
}

#[test]
fn on_a_left_stick_page_the_left_stick_is_left_to_the_picture() {
    let mut config = mouse_config();
    let mut menu = SystemMenu::new(&config).on_page(Page::EditControls);
    // Sticks, then the left stick.
    menu.handle_now(press(Button::DpadDown), &mut config);
    menu.handle_now(press(Button::South), &mut config);
    menu.handle_now(press(Button::South), &mut config);
    let before = menu.view(&config).selected;
    menu.handle_now(InputEvent::Axis(Axis::LeftY, 1.0), &mut config);
    assert_eq!(menu.view(&config).selected, before, "pushing it doesn't move the cursor");
    menu.handle_now(press(Button::DpadDown), &mut config);
    assert_ne!(menu.view(&config).selected, before, "the D-pad still does");
}
