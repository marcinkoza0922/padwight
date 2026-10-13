use super::*;
use crate::config::{Combo, MouseButton, StickConfig, Zone};
use crate::pad_identity::PadIdentity;

#[test]
fn the_face_swap_sends_the_letter_on_each_nintendo_label() {
    let p = Profile::passthrough("swap");
    let mut e = Engine::default();
    assert_eq!(run(&mut e, &p, InputEvent::Button(Button::South, true)), [OutEvent::PadButton(Button::South, true)]);
    assert_eq!(run(&mut e, &p, InputEvent::Button(Button::South, false)), [OutEvent::PadButton(Button::South, false)]);
    e.set_face_swap(true);
    assert_eq!(run(&mut e, &p, InputEvent::Button(Button::South, true)), [OutEvent::PadButton(Button::East, true)]);
    assert_eq!(run(&mut e, &p, InputEvent::Button(Button::South, false)), [OutEvent::PadButton(Button::East, false)]);
    assert_eq!(run(&mut e, &p, InputEvent::Button(Button::North, true)), [OutEvent::PadButton(Button::West, true)]);
    assert_eq!(run(&mut e, &p, InputEvent::Button(Button::North, false)), [OutEvent::PadButton(Button::West, false)]);
    assert_eq!(run(&mut e, &p, InputEvent::Button(Button::LeftBumper, true)), [OutEvent::PadButton(Button::LeftBumper, true)]);
}

#[test]
fn a_reassigned_face_button_turns_the_swap_off() {
    let mut p = Profile::passthrough("swap");
    p.set_button(Button::South, ButtonAction::Mouse(MouseButton::Left));
    assert!(!p.faces_default());
    assert!(!PadIdentity::Xbox360.swaps_face_output(true, true, p.faces_default()));
}

fn run(engine: &mut Engine, profile: &Profile, ev: InputEvent) -> Vec<OutEvent> {
    let mut out = Vec::new();
    engine.handle(profile, ev, Instant::now(), &mut out);
    out
}

#[test]
fn passthrough_button_and_dpad() {
    let p = Profile::passthrough("p");
    let mut e = Engine::default();
    assert_eq!(
        run(&mut e, &p, InputEvent::Button(Button::South, true)),
        vec![OutEvent::PadButton(Button::South, true)]
    );
    assert_eq!(
        run(&mut e, &p, InputEvent::Button(Button::South, false)),
        vec![OutEvent::PadButton(Button::South, false)]
    );
}

#[test]
fn fired_actions_are_reported_once_per_press() {
    use crate::inputlog::FiredFrom;
    let mut p = Profile::passthrough("p");
    p.set_button(Button::South, ButtonAction::Mouse(MouseButton::Left));
    let mut e = Engine::default();
    run(&mut e, &p, InputEvent::Button(Button::South, true));
    assert_eq!(e.take_fired(), [(FiredFrom::Button(Button::South), ButtonAction::Mouse(MouseButton::Left))]);
    run(&mut e, &p, InputEvent::Button(Button::South, false));
    assert!(e.take_fired().is_empty());
}

/// A profile whose Guide holds the default Guide layer, and that layer applied over it.
fn guide_setup() -> (Profile, Profile) {
    let base = Profile::passthrough("p");
    let on = layered(&base, &crate::config::Layer::guide());
    (base, on)
}

#[test]
fn guide_tap_toggles_the_overlay_and_never_reaches_the_pad() {
    let (base, _) = guide_setup();
    let mut e = Engine::default();
    let t0 = Instant::now();
    let mut out = press(&mut e, &base, Button::Guide, true, t0);
    out.extend(press(&mut e, &base, Button::Guide, false, ms(t0, 80)));
    assert!(out.is_empty(), "the Guide button sends nothing to the game or Steam");
    assert_eq!(e.mode(), Some(GUIDE_MODE), "a tap opens the overlay");
    press(&mut e, &base, Button::Guide, true, ms(t0, 2000));
    press(&mut e, &base, Button::Guide, false, ms(t0, 2050));
    assert_eq!(e.mode(), None, "a second tap closes it");
}

#[test]
fn guide_chord_runs_its_shortcut_and_leaves_the_overlay_shut() {
    let (base, on) = guide_setup();
    let mut e = Engine::default();
    let t0 = Instant::now();
    press(&mut e, &base, Button::Guide, true, t0);
    press(&mut e, &on, Button::West, true, ms(t0, 50));
    assert!(e.take_overlay_toggle().is_some(), "Guide + X opens the keyboard");
    press(&mut e, &on, Button::West, false, ms(t0, 100));
    press(&mut e, &on, Button::Guide, false, ms(t0, 150));
    assert_eq!(e.mode(), None, "the chord doesn't open the overlay on release");
}

#[test]
fn guide_overlay_closes_on_the_next_press_and_that_press_runs() {
    let (base, on) = guide_setup();
    let mut e = Engine::default();
    let t0 = Instant::now();
    press(&mut e, &base, Button::Guide, true, t0);
    press(&mut e, &base, Button::Guide, false, ms(t0, 80));
    assert_eq!(e.mode(), Some(GUIDE_MODE));
    press(&mut e, &on, Button::North, true, ms(t0, 500));
    assert_eq!(e.take_overlay_toggle(), Some(crate::keyboard::Layout::Numpad), "Y opens the numpad");
    assert_eq!(e.mode(), None, "and closes the overlay");
}

/// A profile with the mouse-mode layer on top of the passthrough one.
fn mouse_setup() -> (Profile, Profile) {
    let base = Profile::passthrough("p");
    let on = layered(&base, &crate::config::Layer::mouse());
    (base, on)
}

#[test]
fn guide_and_right_stick_click_start_mouse_mode() {
    let (base, mouse) = mouse_setup();
    let (_, guide) = guide_setup();
    let mut e = Engine::default();
    let t0 = Instant::now();
    let mut out = press(&mut e, &base, Button::Guide, true, t0);
    out.extend(press(&mut e, &guide, Button::RightStick, true, ms(t0, 50)));
    out.extend(press(&mut e, &mouse, Button::RightStick, false, ms(t0, 100)));
    out.extend(press(&mut e, &base, Button::Guide, false, ms(t0, 150)));
    assert!(out.is_empty(), "the chord doesn't reach the game");
    assert_eq!(e.mode(), Some(MOUSE_MODE), "mouse mode stays on after the chord, with no overlay");
}

#[test]
fn right_stick_click_with_the_overlay_up_starts_mouse_mode() {
    let (base, on) = guide_setup();
    let mut e = Engine::default();
    let t0 = Instant::now();
    press(&mut e, &base, Button::Guide, true, t0);
    press(&mut e, &base, Button::Guide, false, ms(t0, 80));
    press(&mut e, &on, Button::RightStick, true, ms(t0, 500));
    assert_eq!(e.mode(), Some(MOUSE_MODE));
}

#[test]
fn guide_leaves_mouse_mode_and_does_not_open_the_overlay() {
    let (base, mouse) = mouse_setup();
    let (_, guide) = guide_setup();
    let mut e = Engine::default();
    let t0 = Instant::now();
    press(&mut e, &base, Button::Guide, true, t0);
    press(&mut e, &guide, Button::RightStick, true, ms(t0, 50));
    press(&mut e, &mouse, Button::RightStick, false, ms(t0, 60));
    press(&mut e, &mouse, Button::Guide, false, ms(t0, 70));
    press(&mut e, &mouse, Button::Guide, true, ms(t0, 500));
    press(&mut e, &mouse, Button::Guide, false, ms(t0, 580));
    assert_eq!(e.mode(), None);
}

#[test]
fn mouse_mode_keys_are_held_while_the_button_is_down() {
    let (_, mouse) = mouse_setup();
    let mut e = Engine::default();
    let t0 = Instant::now();
    e.mouse_mode = true;
    assert_eq!(press(&mut e, &mouse, Button::DpadUp, true, t0), vec![OutEvent::Key(KeyCode::KEY_LEFTSHIFT, true)]);
    assert_eq!(press(&mut e, &mouse, Button::DpadUp, false, ms(t0, 10)), vec![OutEvent::Key(KeyCode::KEY_LEFTSHIFT, false)]);
    assert_eq!(press(&mut e, &mouse, Button::West, true, ms(t0, 20)), vec![OutEvent::Key(KeyCode::KEY_LEFTCTRL, true)]);
    assert_eq!(press(&mut e, &mouse, Button::West, false, ms(t0, 30)), vec![OutEvent::Key(KeyCode::KEY_LEFTCTRL, false)]);
    assert_eq!(press(&mut e, &mouse, Button::North, true, ms(t0, 40)), vec![OutEvent::Key(KeyCode::KEY_LEFTALT, true)]);
    assert_eq!(press(&mut e, &mouse, Button::North, false, ms(t0, 50)), vec![OutEvent::Key(KeyCode::KEY_LEFTALT, false)]);
    assert_eq!(press(&mut e, &mouse, Button::East, true, ms(t0, 60)), vec![OutEvent::Key(KeyCode::KEY_DELETE, true)]);
    assert_eq!(press(&mut e, &mouse, Button::East, false, ms(t0, 70)), vec![OutEvent::Key(KeyCode::KEY_DELETE, false)]);
}

/// The profile the daemon hands the engine with `shortcuts` in force: the opener injected,
/// then the Guide mode (when `guide`) laid under it as the daemon's layering does.
fn with_shortcuts(shortcuts: &crate::config::Shortcuts, guide: bool) -> Profile {
    let base = Profile::passthrough("p");
    let mut p = if guide { layered(&base, &crate::config::Layer::guide_with(shortcuts)) } else { base };
    shortcuts.inject(&mut p);
    p
}

fn shortcuts_with(opener: crate::config::ShortcutInput) -> crate::config::Shortcuts {
    let mut s = crate::config::Shortcuts::default();
    s.set(crate::config::Shortcut::OpenGuide, Some(opener));
    s
}

#[test]
fn a_moved_opener_button_works_like_guide_and_guide_does_nothing() {
    let shortcuts = shortcuts_with(crate::config::ShortcutInput::button(Button::Select));
    let (base, on) = (with_shortcuts(&shortcuts, false), with_shortcuts(&shortcuts, true));
    let mut e = Engine::default();
    e.set_opener(shortcuts.engine_opener());
    let t0 = Instant::now();
    assert!(press(&mut e, &base, Button::Guide, true, t0).is_empty(), "Guide is swallowed");
    press(&mut e, &base, Button::Guide, false, ms(t0, 80));
    assert_eq!(e.mode(), None, "and opens nothing");
    let out = press(&mut e, &base, Button::Select, true, ms(t0, 200));
    assert!(out.is_empty(), "the opener never reaches the game");
    press(&mut e, &base, Button::Select, false, ms(t0, 280));
    assert_eq!(e.mode(), Some(GUIDE_MODE), "a tap opens the overlay");
    // Held, it makes the Guide shortcuts available like Guide does.
    press(&mut e, &on, Button::Select, true, ms(t0, 600));
    press(&mut e, &on, Button::Select, false, ms(t0, 680));
    assert_eq!(e.mode(), None, "tapped again, it closes");
}

#[test]
fn a_combo_opener_opens_and_closes_the_overlay_and_leaves_mouse_mode() {
    let shortcuts = shortcuts_with(crate::config::ShortcutInput::combo(&[Button::Start, Button::Select]));
    let (base, on) = (with_shortcuts(&shortcuts, false), with_shortcuts(&shortcuts, true));
    let mut e = Engine::default();
    e.set_opener(shortcuts.engine_opener());
    let t0 = Instant::now();
    press(&mut e, &base, Button::Start, true, t0);
    press(&mut e, &base, Button::Select, true, ms(t0, 10));
    press(&mut e, &base, Button::Select, false, ms(t0, 100));
    press(&mut e, &base, Button::Start, false, ms(t0, 110));
    assert_eq!(e.mode(), Some(GUIDE_MODE), "the combo opens the overlay");
    // The same combo with the overlay up puts it away, instead of its buttons closing it first.
    press(&mut e, &on, Button::Start, true, ms(t0, 500));
    press(&mut e, &on, Button::Select, true, ms(t0, 510));
    assert_eq!(e.mode(), None);
    press(&mut e, &on, Button::Select, false, ms(t0, 600));
    press(&mut e, &on, Button::Start, false, ms(t0, 610));
    // In mouse mode it leaves instead of opening.
    e.enter_mouse_mode();
    press(&mut e, &base, Button::Start, true, ms(t0, 900));
    press(&mut e, &base, Button::Select, true, ms(t0, 910));
    assert_eq!(e.mode(), None);
}

#[test]
fn a_gesture_opener_waits_for_its_gesture() {
    let shortcuts = shortcuts_with(crate::config::ShortcutInput::gesture(Button::Select, GestureKind::DoubleTap));
    let p = with_shortcuts(&shortcuts, false);
    let mut e = Engine::default();
    e.set_opener(shortcuts.engine_opener());
    let t0 = Instant::now();
    press(&mut e, &p, Button::Select, true, t0);
    press(&mut e, &p, Button::Select, false, ms(t0, 40));
    assert_eq!(e.mode(), None, "one tap is only a tap");
    press(&mut e, &p, Button::Select, true, ms(t0, 100));
    press(&mut e, &p, Button::Select, false, ms(t0, 140));
    fire_timers(&mut e, &p, ms(t0, 600));
    assert_eq!(e.mode(), Some(GUIDE_MODE), "two taps open it");
}

#[test]
fn with_the_opener_off_nothing_opens_the_overlay() {
    let mut shortcuts = crate::config::Shortcuts::default();
    shortcuts.set(crate::config::Shortcut::OpenGuide, None);
    let p = with_shortcuts(&shortcuts, false);
    let mut e = Engine::default();
    e.set_opener(shortcuts.engine_opener());
    let t0 = Instant::now();
    press(&mut e, &p, Button::Guide, true, t0);
    press(&mut e, &p, Button::Guide, false, ms(t0, 50));
    assert_eq!(e.mode(), None);
}

#[test]
fn mouse_mode_is_entered_on_whatever_the_shortcut_is_moved_to() {
    let mut shortcuts = crate::config::Shortcuts::default();
    shortcuts.set(crate::config::Shortcut::MouseMode, Some(crate::config::ShortcutInput::button(Button::Start)));
    let on = with_shortcuts(&shortcuts, true);
    let mut e = Engine::default();
    let t0 = Instant::now();
    press(&mut e, &on, Button::Guide, true, t0);
    press(&mut e, &on, Button::RightStick, true, ms(t0, 20));
    assert_eq!(e.mode(), Some(GUIDE_MODE), "R3 no longer does it");
    press(&mut e, &on, Button::Start, true, ms(t0, 40));
    press(&mut e, &on, Button::Guide, false, ms(t0, 80));
    assert_eq!(e.mode(), Some(MOUSE_MODE));
}

#[test]
fn quick_settings_is_a_guide_shortcut_that_closes_the_overlay() {
    let (base, on) = guide_setup();
    let mut e = Engine::default();
    let t0 = Instant::now();
    press(&mut e, &base, Button::Guide, true, t0);
    press(&mut e, &base, Button::Guide, false, ms(t0, 80));
    assert_eq!(e.mode(), Some(GUIDE_MODE));
    let out = press(&mut e, &on, Button::Start, true, ms(t0, 400));
    assert!(out.is_empty(), "Start doesn't reach the game");
    assert!(e.take_settings_request());
    assert_eq!(e.mode(), None, "the overlay is put away");
    // Held Guide + Start, the old chord, asks the same.
    press(&mut e, &on, Button::Start, false, ms(t0, 500));
    press(&mut e, &base, Button::Guide, true, ms(t0, 1000));
    press(&mut e, &on, Button::Start, true, ms(t0, 1050));
    assert!(e.take_settings_request());
}

#[test]
fn the_guide_menu_hears_a_and_b_where_the_menu_buttons_are() {
    let (base, on) = guide_setup();
    let mut e = Engine::default();
    e.set_menu_keys(crate::config::MenuKeys { confirm: Button::Select, back: Button::East });
    let t0 = Instant::now();
    press(&mut e, &base, Button::Guide, true, t0);
    press(&mut e, &base, Button::Guide, false, ms(t0, 80));
    press(&mut e, &on, Button::Select, true, ms(t0, 300));
    assert_eq!(e.take_guide_nav(), [Button::South], "Select chooses");
    assert_eq!(e.mode(), Some(GUIDE_MODE), "and doesn't close the overlay");
    press(&mut e, &on, Button::South, true, ms(t0, 400));
    assert!(e.take_guide_nav().is_empty(), "A is just another button now");
    assert_eq!(e.mode(), None);
}

#[test]
fn holding_a_menu_direction_repeats_it_until_let_go() {
    let (base, on) = guide_setup();
    let mut e = Engine::default();
    let t0 = Instant::now();
    press(&mut e, &base, Button::Guide, true, t0);
    press(&mut e, &base, Button::Guide, false, ms(t0, 10));
    press(&mut e, &on, Button::DpadDown, true, ms(t0, 100));
    assert_eq!(e.take_guide_nav(), [Button::DpadDown]);
    fire_timers(&mut e, &on, ms(t0, 300));
    assert!(e.take_guide_nav().is_empty(), "no repeat before the delay");
    fire_timers(&mut e, &on, ms(t0, 500));
    assert_eq!(e.take_guide_nav(), [Button::DpadDown], "repeats after the delay");
    fire_timers(&mut e, &on, ms(t0, 640));
    assert_eq!(e.take_guide_nav(), [Button::DpadDown], "then at the repeat rate");
    press(&mut e, &on, Button::DpadDown, false, ms(t0, 700));
    assert_eq!(e.next_deadline(), None, "letting go stops it");
    fire_timers(&mut e, &on, ms(t0, 2000));
    assert!(e.take_guide_nav().is_empty());
}

#[test]
fn letting_go_of_everything_closes_the_guide_overlay() {
    let (base, _) = guide_setup();
    let mut e = Engine::default();
    let t0 = Instant::now();
    press(&mut e, &base, Button::Guide, true, t0);
    press(&mut e, &base, Button::Guide, false, ms(t0, 80));
    e.release_all(false, &mut Vec::new());
    assert_eq!(e.mode(), None);
}

#[test]
fn screenshot_is_requested_on_press_only_once() {
    let mut p = Profile::passthrough("p");
    p.set_button(Button::RightBumper, ButtonAction::Screenshot);
    let mut e = Engine::default();
    let mut out = Vec::new();
    e.handle(&p, InputEvent::Button(Button::RightBumper, true), Instant::now(), &mut out);
    assert!(e.take_screenshot() && !e.take_screenshot());
    e.handle(&p, InputEvent::Button(Button::RightBumper, false), Instant::now(), &mut out);
    assert!(!e.take_screenshot());
}

#[test]
fn recording_is_toggled_on_each_press() {
    let mut p = Profile::passthrough("p");
    p.set_button(Button::LeftStick, ButtonAction::ToggleRecording);
    let mut e = Engine::default();
    let mut out = Vec::new();
    for _ in 0..2 {
        e.handle(&p, InputEvent::Button(Button::LeftStick, true), Instant::now(), &mut out);
        assert!(e.take_recording_toggle() && !e.take_recording_toggle());
        e.handle(&p, InputEvent::Button(Button::LeftStick, false), Instant::now(), &mut out);
        assert!(!e.take_recording_toggle());
    }
}

#[test]
fn force_quit_is_held_only_while_pressed() {
    let mut p = Profile::passthrough("p");
    p.set_button(Button::East, ButtonAction::ForceQuit);
    let mut e = Engine::default();
    let mut out = Vec::new();
    e.handle(&p, InputEvent::Button(Button::East, true), Instant::now(), &mut out);
    assert!(e.force_quit_held());
    e.handle(&p, InputEvent::Button(Button::East, false), Instant::now(), &mut out);
    assert!(!e.force_quit_held());
    e.handle(&p, InputEvent::Button(Button::East, true), Instant::now(), &mut out);
    e.release_all(false, &mut out);
    assert!(!e.force_quit_held(), "letting everything go cancels it");
}

#[test]
fn a_held_guide_swallows_the_d_pad_which_mouse_mode_owns() {
    let (base, on) = guide_setup();
    let mut e = Engine::default();
    let t0 = Instant::now();
    press(&mut e, &base, Button::Guide, true, t0);
    assert!(press(&mut e, &on, Button::DpadRight, true, ms(t0, 10)).is_empty());
    assert!(press(&mut e, &on, Button::DpadRight, false, ms(t0, 20)).is_empty());
}

#[test]
fn menu_buttons_drive_the_open_overlay_without_closing_it() {
    let (base, on) = guide_setup();
    let mut e = Engine::default();
    let t0 = Instant::now();
    press(&mut e, &base, Button::Guide, true, t0);
    press(&mut e, &base, Button::Guide, false, ms(t0, 80));
    press(&mut e, &on, Button::DpadDown, true, ms(t0, 200));
    press(&mut e, &on, Button::LeftStickUp, true, ms(t0, 300));
    press(&mut e, &on, Button::South, true, ms(t0, 400));
    assert_eq!(e.mode(), Some(GUIDE_MODE), "the menu buttons don't close the overlay");
    assert_eq!(e.take_guide_nav(), [Button::DpadDown, Button::LeftStickUp, Button::South]);
    assert!(e.take_guide_nav().is_empty());
    press(&mut e, &on, Button::Start, true, ms(t0, 500));
    assert_eq!(e.mode(), None, "any other button closes it");
}

#[test]
fn the_left_stick_moves_the_guide_menu_not_the_game() {
    let (base, on) = guide_setup();
    let mut e = Engine::default();
    let t0 = Instant::now();
    assert!(!axis(&mut e, &base, Axis::LeftY, 0.5).is_empty(), "the game gets the stick normally");
    axis(&mut e, &base, Axis::LeftY, 0.0);
    press(&mut e, &base, Button::Guide, true, t0);
    press(&mut e, &base, Button::Guide, false, ms(t0, 80));
    let out = axis(&mut e, &on, Axis::LeftY, 0.9);
    assert!(!out.iter().any(|o| matches!(o, OutEvent::PadAxis(_, v) if *v != 0.0)), "only recentered: {out:?}");
    assert_eq!(e.take_guide_nav(), [Button::LeftStickDown]);
}

#[test]
fn the_right_stick_sideways_does_not_close_the_guide() {
    let (base, on) = guide_setup();
    let mut e = Engine::default();
    let t0 = Instant::now();
    press(&mut e, &base, Button::Guide, true, t0);
    press(&mut e, &base, Button::Guide, false, ms(t0, 80));
    assert!(e.guide_overlay_open());
    press(&mut e, &on, Button::RightStickLeft, true, ms(t0, 200));
    press(&mut e, &on, Button::RightStickLeft, false, ms(t0, 240));
    press(&mut e, &on, Button::RightStickRight, true, ms(t0, 300));
    press(&mut e, &on, Button::RightStickRight, false, ms(t0, 340));
    assert!(e.guide_overlay_open(), "the menu stays up");
    assert_eq!(e.take_guide_nav(), [Button::RightStickLeft, Button::RightStickRight]);
}

#[test]
fn a_layer_can_swallow_the_buttons_it_does_not_set() {
    let (base, on) = guide_setup();
    assert_eq!(on.button(Button::South), &ButtonAction::Disabled);
    assert_eq!(on.button(Button::Guide), &ButtonAction::guide_hold(), "Guide itself stays");
    assert_eq!(on.button(Button::West), &ButtonAction::ToggleOverlay);
    assert_eq!(base.button(Button::South), &ButtonAction::Gamepad(Button::South));
}

#[test]
fn release_uses_action_from_press_time() {
    let mut p = Profile::passthrough("p");
    p.set_button(Button::South, ButtonAction::Mouse(MouseButton::Left));
    let mut e = Engine::default();
    run(&mut e, &p, InputEvent::Button(Button::South, true));
    // Profile changes while held: release must still go to the mouse button.
    p.set_button(Button::South, ButtonAction::Gamepad(Button::South));
    assert_eq!(
        run(&mut e, &p, InputEvent::Button(Button::South, false)),
        vec![OutEvent::MouseButton(MouseButton::Left, false)]
    );
}

#[test]
fn chord_releases_in_reverse() {
    let mut p = Profile::passthrough("p");
    p.set_button(Button::West, ButtonAction::Keys(vec!["KEY_LEFTCTRL".into(), "KEY_C".into()]));
    let mut e = Engine::default();
    run(&mut e, &p, InputEvent::Button(Button::West, true));
    assert_eq!(
        run(&mut e, &p, InputEvent::Button(Button::West, false)),
        vec![OutEvent::Key(KeyCode::KEY_C, false), OutEvent::Key(KeyCode::KEY_LEFTCTRL, false)]
    );
}

#[test]
fn trigger_threshold_with_hysteresis() {
    let p = Profile::desktop("d"); // right trigger = left click at 0.5
    let mut e = Engine::default();
    let click = |down| vec![OutEvent::MouseButton(MouseButton::Left, down)];
    assert!(run(&mut e, &p, InputEvent::Axis(Axis::RightTrigger, 0.4)).is_empty());
    assert_eq!(run(&mut e, &p, InputEvent::Axis(Axis::RightTrigger, 0.6)), click(true));
    assert!(run(&mut e, &p, InputEvent::Axis(Axis::RightTrigger, 0.48)).is_empty());
    assert_eq!(run(&mut e, &p, InputEvent::Axis(Axis::RightTrigger, 0.1)), click(false));
}

#[test]
fn stick_to_keys() {
    let mut p = Profile::passthrough("p");
    p.left_stick = StickConfig::new(StickAction::wasd(), 0.1, 1.0);
    let mut e = Engine::default();
    assert_eq!(
        run(&mut e, &p, InputEvent::Axis(Axis::LeftY, -1.0)),
        vec![OutEvent::Key(KeyCode::KEY_W, true)]
    );
    assert_eq!(
        run(&mut e, &p, InputEvent::Axis(Axis::LeftY, 1.0)),
        vec![OutEvent::Key(KeyCode::KEY_W, false), OutEvent::Key(KeyCode::KEY_S, true)]
    );
}

#[test]
fn mouse_tick_moves_and_accumulates_subpixels() {
    let p = Profile::desktop("d"); // left stick = mouse at 1200 px/s
    let mut e = Engine::default();
    run(&mut e, &p, InputEvent::Axis(Axis::LeftX, 1.0));
    assert!(e.needs_tick(&p));
    let mut out = Vec::new();
    for _ in 0..250 {
        e.tick(&p, 0.004, &mut out);
    }
    let total: i32 = out
        .iter()
        .map(|o| match o {
            OutEvent::MouseMove(dx, _) => *dx,
            _ => 0,
        })
        .sum();
    assert!((1195..=1200).contains(&total), "moved {total}px in 1s");
}

#[test]
fn deadzone_is_radial_and_rescaled() {
    assert_eq!(apply_deadzone(0.05, 0.05, 0.1), (0.0, 0.0));
    let (x, _) = apply_deadzone(1.0, 0.0, 0.2);
    assert!((x - 1.0).abs() < 1e-6);
    let (x, _) = apply_deadzone(0.6, 0.0, 0.2);
    assert!((x - 0.5).abs() < 1e-6);
}

fn combo_profile() -> Profile {
    let mut p = Profile::passthrough("p");
    p.combos.push(Combo {
        buttons: vec![Button::LeftBumper, Button::RightBumper],
        action: ButtonAction::Keys(vec!["KEY_LEFTALT".into(), "KEY_TAB".into()]),
    });
    p
}

fn step(e: &mut Engine, p: &Profile, ev: InputEvent, at: Instant) -> Vec<OutEvent> {
    let mut out = Vec::new();
    e.handle(p, ev, at, &mut out);
    out
}

#[test]
fn combo_fires_instead_of_members() {
    let p = combo_profile();
    let mut e = Engine::default();
    let t0 = Instant::now();
    assert!(step(&mut e, &p, InputEvent::Button(Button::LeftBumper, true), t0).is_empty());
    assert_eq!(
        step(&mut e, &p, InputEvent::Button(Button::RightBumper, true), t0 + Duration::from_millis(20)),
        vec![OutEvent::Key(KeyCode::KEY_LEFTALT, true), OutEvent::Key(KeyCode::KEY_TAB, true)]
    );
    // Window passing must not fire the consumed members.
    let mut out = Vec::new();
    e.timers(&p, t0 + Duration::from_secs(1), &mut out);
    assert!(out.is_empty());
    // Releasing either member releases the combo; the other release is silent.
    assert_eq!(
        step(&mut e, &p, InputEvent::Button(Button::RightBumper, false), t0),
        vec![OutEvent::Key(KeyCode::KEY_TAB, false), OutEvent::Key(KeyCode::KEY_LEFTALT, false)]
    );
    assert!(step(&mut e, &p, InputEvent::Button(Button::LeftBumper, false), t0).is_empty());
}

#[test]
fn combo_member_acts_alone_after_window() {
    let p = combo_profile();
    let mut e = Engine::default();
    let t0 = Instant::now();
    step(&mut e, &p, InputEvent::Button(Button::LeftBumper, true), t0);
    assert_eq!(e.next_deadline(), Some(t0 + Duration::from_millis(60)));
    let mut out = Vec::new();
    e.timers(&p, t0 + Duration::from_millis(59), &mut out);
    assert!(out.is_empty());
    e.timers(&p, t0 + Duration::from_millis(60), &mut out);
    assert_eq!(out, vec![OutEvent::PadButton(Button::LeftBumper, true)]);
    // Too late for the combo: RB acts on its own too (after its own window).
    assert!(step(&mut e, &p, InputEvent::Button(Button::RightBumper, true), t0 + Duration::from_millis(100)).is_empty());
    assert_eq!(
        step(&mut e, &p, InputEvent::Button(Button::LeftBumper, false), t0),
        vec![OutEvent::PadButton(Button::LeftBumper, false)]
    );
}

#[test]
fn quick_tap_of_combo_member_still_registers() {
    let p = combo_profile();
    let mut e = Engine::default();
    let t0 = Instant::now();
    step(&mut e, &p, InputEvent::Button(Button::LeftBumper, true), t0);
    assert_eq!(
        step(&mut e, &p, InputEvent::Button(Button::LeftBumper, false), t0 + Duration::from_millis(10)),
        vec![
            OutEvent::PadButton(Button::LeftBumper, true),
            OutEvent::PadButton(Button::LeftBumper, false)
        ]
    );
    assert_eq!(e.next_deadline(), None);
}

#[test]
fn disabled_member_works_as_modifier() {
    let mut p = Profile::passthrough("p");
    p.set_button(Button::Start, ButtonAction::Disabled);
    p.combos.push(Combo {
        buttons: vec![Button::Start, Button::South],
        action: ButtonAction::Screenshot,
    });
    let mut e = Engine::default();
    let t0 = Instant::now();
    step(&mut e, &p, InputEvent::Button(Button::Start, true), t0);
    assert_eq!(e.next_deadline(), None);
    let mut out = Vec::new();
    e.handle(&p, InputEvent::Button(Button::South, true), t0 + Duration::from_secs(5), &mut out);
    assert!(e.take_screenshot());
}

#[test]
fn multi_presses_in_order_and_releases_in_reverse() {
    let mut p = Profile::passthrough("p");
    p.set_button(
        Button::South,
        ButtonAction::Multi(vec![
            ButtonAction::Gamepad(Button::South),
            ButtonAction::Keys(vec!["KEY_X".into()]),
            ButtonAction::Mouse(MouseButton::Left),
        ]),
    );
    let mut e = Engine::default();
    assert_eq!(
        run(&mut e, &p, InputEvent::Button(Button::South, true)),
        vec![
            OutEvent::PadButton(Button::South, true),
            OutEvent::Key(KeyCode::KEY_X, true),
            OutEvent::MouseButton(MouseButton::Left, true)
        ]
    );
    assert_eq!(
        run(&mut e, &p, InputEvent::Button(Button::South, false)),
        vec![
            OutEvent::MouseButton(MouseButton::Left, false),
            OutEvent::Key(KeyCode::KEY_X, false),
            OutEvent::PadButton(Button::South, false)
        ]
    );
}

fn gesture_profile() -> Profile {
    let mut p = Profile::passthrough("p");
    p.gestures.insert(
        Button::North,
        crate::config::Gestures {
            double_tap: Some(ButtonAction::Keys(vec!["KEY_2".into()])),
            triple_tap: Some(ButtonAction::Keys(vec!["KEY_3".into()])),
            long_press: Some(ButtonAction::Keys(vec!["KEY_L".into()])),
        },
    );
    p
}

fn ms(t0: Instant, n: u64) -> Instant {
    t0 + Duration::from_millis(n)
}

fn press(e: &mut Engine, p: &Profile, b: Button, down: bool, at: Instant) -> Vec<OutEvent> {
    step(e, p, InputEvent::Button(b, down), at)
}

fn fire_timers(e: &mut Engine, p: &Profile, at: Instant) -> Vec<OutEvent> {
    let mut out = Vec::new();
    e.timers(p, at, &mut out);
    out
}

fn key_tap(k: KeyCode) -> Vec<OutEvent> {
    vec![OutEvent::Key(k, true), OutEvent::Key(k, false)]
}

#[test]
fn single_tap_fires_normal_action_after_window() {
    let p = gesture_profile();
    let mut e = Engine::default();
    let t0 = Instant::now();
    assert!(press(&mut e, &p, Button::North, true, t0).is_empty());
    assert!(press(&mut e, &p, Button::North, false, ms(t0, 80)).is_empty());
    assert_eq!(e.next_deadline(), Some(ms(t0, 330)));
    assert!(fire_timers(&mut e, &p, ms(t0, 329)).is_empty());
    assert_eq!(
        fire_timers(&mut e, &p, ms(t0, 330)),
        vec![OutEvent::PadButton(Button::North, true), OutEvent::PadButton(Button::North, false)]
    );
    assert_eq!(e.next_deadline(), None);
}

#[test]
fn holding_past_the_tap_window_presses_and_holds_the_buttons_own_action() {
    // e.g. RB: hold for a weapon wheel, double tap to holster.
    let mut p = Profile::passthrough("p");
    p.set_button(Button::RightBumper, ButtonAction::OpenMenu("Weapons".into()));
    p.gestures.insert(
        Button::RightBumper,
        crate::config::Gestures { double_tap: Some(ButtonAction::Keys(vec!["KEY_H".into()])), ..Default::default() },
    );
    let mut e = Engine::default();
    let t0 = Instant::now();
    press(&mut e, &p, Button::RightBumper, true, t0);
    assert_eq!(e.take_menu_request(), None, "undecided inside the tap window");
    fire_timers(&mut e, &p, ms(t0, p.tap_window_ms + 1));
    let (name, opener) = e.take_menu_request().expect("held past the window: the menu opens");
    assert_eq!((name.as_str(), opener.buttons), ("Weapons", vec![Button::RightBumper]));
    // The double tap still works.
    press(&mut e, &p, Button::RightBumper, false, ms(t0, 400));
    press(&mut e, &p, Button::RightBumper, true, ms(t0, 1000));
    press(&mut e, &p, Button::RightBumper, false, ms(t0, 1050));
    let out = press(&mut e, &p, Button::RightBumper, true, ms(t0, 1100));
    assert_eq!(out, vec![OutEvent::Key(KeyCode::KEY_H, true)]);
    assert_eq!(e.take_menu_request(), None);
}

#[test]
fn double_tap_resolves_after_window_when_triple_is_possible() {
    let p = gesture_profile();
    let mut e = Engine::default();
    let t0 = Instant::now();
    press(&mut e, &p, Button::North, true, t0);
    press(&mut e, &p, Button::North, false, ms(t0, 60));
    assert!(press(&mut e, &p, Button::North, true, ms(t0, 150)).is_empty());
    assert!(press(&mut e, &p, Button::North, false, ms(t0, 200)).is_empty());
    assert_eq!(fire_timers(&mut e, &p, ms(t0, 450)), key_tap(KeyCode::KEY_2));
}

#[test]
fn triple_tap_fires_on_third_press_and_holds() {
    let p = gesture_profile();
    let mut e = Engine::default();
    let t0 = Instant::now();
    for (down, at) in [(true, 0), (false, 50), (true, 120), (false, 170)] {
        assert!(press(&mut e, &p, Button::North, down, ms(t0, at)).is_empty());
    }
    assert_eq!(
        press(&mut e, &p, Button::North, true, ms(t0, 240)),
        vec![OutEvent::Key(KeyCode::KEY_3, true)]
    );
    assert!(fire_timers(&mut e, &p, ms(t0, 5000)).is_empty());
    assert_eq!(
        press(&mut e, &p, Button::North, false, ms(t0, 5000)),
        vec![OutEvent::Key(KeyCode::KEY_3, false)]
    );
}

#[test]
fn long_press_fires_while_held_and_releases_on_release() {
    let p = gesture_profile();
    let mut e = Engine::default();
    let t0 = Instant::now();
    press(&mut e, &p, Button::North, true, t0);
    assert!(fire_timers(&mut e, &p, ms(t0, 499)).is_empty());
    assert_eq!(fire_timers(&mut e, &p, ms(t0, 500)), vec![OutEvent::Key(KeyCode::KEY_L, true)]);
    assert_eq!(
        press(&mut e, &p, Button::North, false, ms(t0, 900)),
        vec![OutEvent::Key(KeyCode::KEY_L, false)]
    );
    assert_eq!(e.next_deadline(), None);
}

#[test]
fn long_press_only_button_taps_on_release() {
    let mut p = Profile::passthrough("p");
    p.gestures.insert(
        Button::South,
        crate::config::Gestures { long_press: Some(ButtonAction::Keys(vec!["KEY_F5".into()])), ..Default::default() },
    );
    let mut e = Engine::default();
    let t0 = Instant::now();
    press(&mut e, &p, Button::South, true, t0);
    assert_eq!(
        press(&mut e, &p, Button::South, false, ms(t0, 100)),
        vec![OutEvent::PadButton(Button::South, true), OutEvent::PadButton(Button::South, false)]
    );
    // Held past the threshold: the long press fires.
    press(&mut e, &p, Button::South, true, ms(t0, 1000));
    let mut out = Vec::new();
    e.timers(&p, ms(t0, 1500), &mut out);
    assert_eq!(out, vec![OutEvent::Key(KeyCode::KEY_F5, true)]);
}

#[test]
fn combo_member_with_gestures_double_taps_after_combo_window() {
    let mut p = combo_profile(); // LB+RB combo, 60 ms window
    p.gestures.insert(
        Button::LeftBumper,
        crate::config::Gestures { double_tap: Some(ButtonAction::Keys(vec!["KEY_D".into()])), ..Default::default() },
    );
    let mut e = Engine::default();
    let t0 = Instant::now();
    // Two quick taps, each released inside the combo window.
    press(&mut e, &p, Button::LeftBumper, true, t0);
    assert!(press(&mut e, &p, Button::LeftBumper, false, ms(t0, 30)).is_empty());
    press(&mut e, &p, Button::LeftBumper, true, ms(t0, 100));
    assert_eq!(press(&mut e, &p, Button::LeftBumper, false, ms(t0, 130)), key_tap(KeyCode::KEY_D));
}

fn axis(e: &mut Engine, p: &Profile, axis: Axis, v: f32) -> Vec<OutEvent> {
    run(e, p, InputEvent::Axis(axis, v))
}

#[test]
fn partial_push_walks_full_push_runs() {
    let mut p = Profile::passthrough("p");
    p.left_stick = StickConfig::new(StickAction::wasd().starting_at(0.2), 0.0, 1.0);
    p.left_stick.key_threshold = 0.2;
    p.left_stick.zones.push(crate::config::Zone {
        min: 0.0,
        max: 0.75,
        action: ButtonAction::Keys(vec!["KEY_LEFTSHIFT".into()]),
    });
    let mut e = Engine::default();
    let shift = KeyCode::KEY_LEFTSHIFT;
    assert_eq!(
        axis(&mut e, &p, Axis::LeftY, -0.4),
        vec![OutEvent::Key(KeyCode::KEY_W, true), OutEvent::Key(shift, true)]
    );
    // Just past the edge: hysteresis keeps walking.
    assert!(axis(&mut e, &p, Axis::LeftY, -0.76).is_empty());
    assert_eq!(axis(&mut e, &p, Axis::LeftY, -1.0), vec![OutEvent::Key(shift, false)]);
    assert_eq!(axis(&mut e, &p, Axis::LeftY, -0.5), vec![OutEvent::Key(shift, true)]);
    assert_eq!(
        axis(&mut e, &p, Axis::LeftY, 0.0),
        vec![OutEvent::Key(KeyCode::KEY_W, false), OutEvent::Key(shift, false)]
    );
}

#[test]
fn trigger_zones_layer_on_top_of_main_action() {
    let mut p = Profile::passthrough("p"); // RT = gamepad trigger
    p.right_trigger.zones = vec![
        crate::config::Zone { min: 0.3, max: 0.9, action: ButtonAction::Mouse(MouseButton::Right) },
        crate::config::Zone { min: 0.9, max: 1.0, action: ButtonAction::Mouse(MouseButton::Left) },
    ];
    let mut e = Engine::default();
    assert_eq!(
        axis(&mut e, &p, Axis::RightTrigger, 0.5),
        vec![OutEvent::PadAxis(Axis::RightTrigger, 0.5), OutEvent::MouseButton(MouseButton::Right, true)]
    );
    // Moving into the next zone releases the first before pressing the second.
    assert_eq!(
        axis(&mut e, &p, Axis::RightTrigger, 1.0),
        vec![
            OutEvent::PadAxis(Axis::RightTrigger, 1.0),
            OutEvent::MouseButton(MouseButton::Right, false),
            OutEvent::MouseButton(MouseButton::Left, true)
        ]
    );
    assert_eq!(
        axis(&mut e, &p, Axis::RightTrigger, 0.0),
        vec![OutEvent::PadAxis(Axis::RightTrigger, 0.0), OutEvent::MouseButton(MouseButton::Left, false)]
    );
}

#[test]
fn zone_from_zero_is_inactive_at_rest() {
    let mut p = Profile::passthrough("p");
    p.left_stick.deadzone = 0.2;
    p.left_stick.zones.push(crate::config::Zone { min: 0.0, max: 1.0, action: ButtonAction::Keys(vec!["KEY_Q".into()]) });
    let mut e = Engine::default();
    // Inside the deadzone counts as rest.
    assert!(!axis(&mut e, &p, Axis::LeftX, 0.1).contains(&OutEvent::Key(KeyCode::KEY_Q, true)));
    assert!(axis(&mut e, &p, Axis::LeftX, 0.5).contains(&OutEvent::Key(KeyCode::KEY_Q, true)));
}

#[test]
fn pc_action_template_drives_keyboard_and_mouse() {
    let p = Profile::pc_action("a");
    let mut e = Engine::default();
    let has = |out: &[OutEvent], ev: OutEvent| out.contains(&ev);

    assert!(has(&axis(&mut e, &p, Axis::RightTrigger, 1.0), OutEvent::MouseButton(MouseButton::Left, true)));
    assert!(has(&axis(&mut e, &p, Axis::LeftTrigger, 1.0), OutEvent::MouseButton(MouseButton::Right, true)));
    assert_eq!(run(&mut e, &p, InputEvent::Button(Button::South, true)), vec![OutEvent::Key(KeyCode::KEY_E, true)]);

    // Partial push walks forward; full push adds sprint.
    assert_eq!(axis(&mut e, &p, Axis::LeftY, -0.5), vec![OutEvent::Key(KeyCode::KEY_W, true)]);
    assert_eq!(axis(&mut e, &p, Axis::LeftY, -1.0), vec![OutEvent::Key(KeyCode::KEY_LEFTSHIFT, true)]);

    // Right stick is mouse look.
    axis(&mut e, &p, Axis::RightX, 1.0);
    assert!(e.needs_tick(&p));
    let mut out = Vec::new();
    e.tick(&p, 0.1, &mut out);
    assert!(matches!(out.as_slice(), [OutEvent::MouseMove(dx, 0)] if *dx > 0), "{out:?}");
}

fn wheel_total(out: &[OutEvent]) -> (i32, i32) {
    out.iter().fold((0, 0), |(v, h), ev| match ev {
        OutEvent::Wheel { vertical, horizontal } => (v + vertical, h + horizontal),
        _ => (v, h),
    })
}

fn hold_for(e: &mut Engine, p: &Profile, seconds: f32) -> Vec<OutEvent> {
    let mut out = Vec::new();
    let steps = (seconds / 0.004).round() as usize;
    for _ in 0..steps {
        e.tick(p, 0.004, &mut out);
    }
    out
}

#[test]
fn wheel_button_scrolls_a_notch_then_repeats_while_held() {
    use crate::config::WheelDirection;
    let mut p = Profile::passthrough("p");
    p.set_button(Button::RightBumper, ButtonAction::Wheel(WheelDirection::Down));
    let mut e = Engine::default();
    assert_eq!(
        run(&mut e, &p, InputEvent::Button(Button::RightBumper, true)),
        vec![OutEvent::Wheel { vertical: -120, horizontal: 0 }]
    );
    assert!(e.needs_tick(&p));
    // Nothing more during the repeat delay...
    assert_eq!(wheel_total(&hold_for(&mut e, &p, 0.3)), (0, 0));
    // ...then ~10 notches/s: 0.65 s more is ~0.6 s past the delay, about 6 notches.
    let (v, h) = wheel_total(&hold_for(&mut e, &p, 0.65));
    assert!((-720..=-600).contains(&v) && h == 0, "scrolled {v}");
    // Releasing stops it.
    assert!(run(&mut e, &p, InputEvent::Button(Button::RightBumper, false)).is_empty());
    assert!(!e.needs_tick(&p));
    assert_eq!(wheel_total(&hold_for(&mut e, &p, 1.0)), (0, 0));
}

#[test]
fn wheel_inside_multi_and_horizontal() {
    use crate::config::WheelDirection;
    let mut p = Profile::passthrough("p");
    p.set_button(
        Button::West,
        ButtonAction::Multi(vec![
            ButtonAction::Keys(vec!["KEY_LEFTSHIFT".into()]),
            ButtonAction::Wheel(WheelDirection::Right),
        ]),
    );
    let mut e = Engine::default();
    assert_eq!(
        run(&mut e, &p, InputEvent::Button(Button::West, true)),
        vec![OutEvent::Key(KeyCode::KEY_LEFTSHIFT, true), OutEvent::Wheel { vertical: 0, horizontal: 120 }]
    );
    let (v, h) = wheel_total(&hold_for(&mut e, &p, 1.0));
    assert!(v == 0 && h > 0, "{v} {h}");
}

fn toggle(inner: ButtonAction) -> ButtonAction {
    ButtonAction::toggle(inner)
}

fn turbo(inner: ButtonAction, rate: f32) -> ButtonAction {
    ButtonAction::Turbo { action: Box::new(inner), rate, every_ms: 0 }
}

fn c_key() -> ButtonAction {
    ButtonAction::Keys(vec!["KEY_C".into()])
}

fn clicks(out: &[OutEvent]) -> Vec<bool> {
    out.iter()
        .filter_map(|ev| match ev {
            OutEvent::MouseButton(MouseButton::Left, down) => Some(*down),
            _ => None,
        })
        .collect()
}

#[test]
fn toggle_holds_until_pressed_again() {
    let mut p = Profile::passthrough("p");
    p.set_button(Button::RightStick, toggle(c_key()));
    let mut e = Engine::default();
    let press = |e: &mut Engine, down| run(e, &p, InputEvent::Button(Button::RightStick, down));
    assert_eq!(press(&mut e, true), vec![OutEvent::Key(KeyCode::KEY_C, true)]);
    assert!(press(&mut e, false).is_empty(), "letting go keeps crouching");
    assert_eq!(press(&mut e, true), vec![OutEvent::Key(KeyCode::KEY_C, false)]);
    assert!(press(&mut e, false).is_empty());
    assert_eq!(press(&mut e, true), vec![OutEvent::Key(KeyCode::KEY_C, true)]);
}

#[test]
fn turbo_repeats_at_rate_while_held() {
    let mut p = Profile::passthrough("p");
    p.set_button(Button::West, turbo(ButtonAction::Mouse(MouseButton::Left), 10.0));
    let mut e = Engine::default();
    assert_eq!(clicks(&run(&mut e, &p, InputEvent::Button(Button::West, true))), [true]);
    assert!(e.needs_tick(&p));
    // 10/s for one second: alternating up/down every 50 ms, 20 flips.
    let flips = clicks(&hold_for(&mut e, &p, 1.0));
    assert!((19..=21).contains(&flips.len()), "{flips:?}");
    assert!(flips.windows(2).all(|w| w[0] != w[1]), "must alternate: {flips:?}");
    // Releasing mid-press releases the button and stops repeating.
    let mut all = flips;
    all.extend(clicks(&run(&mut e, &p, InputEvent::Button(Button::West, false))));
    assert_eq!(all.last(), Some(&false));
    assert!(!e.needs_tick(&p));
    assert!(clicks(&hold_for(&mut e, &p, 0.5)).is_empty());
}

#[test]
fn turbo_of_a_macro_plays_it_every_gap_but_never_faster_than_it_takes() {
    // A macro that takes 100 ms, so a turbo of it can't press more often than every 100 ms.
    let mut e = macro_engine(vec![MacroStep::Tap { action: ButtonAction::Mouse(MouseButton::Left), hold_ms: 100 }]);
    let looping = ButtonAction::Macro { name: "m".into(), repeat: true };
    let presses = |out: &[OutEvent]| clicks(out).into_iter().filter(|down| *down).count();

    let mut p = Profile::passthrough("p");
    p.set_button(Button::West, ButtonAction::Turbo { action: Box::new(looping.clone()), rate: 10.0, every_ms: 300 });
    let mut pressed = presses(&run(&mut e, &p, InputEvent::Button(Button::West, true)));
    pressed += presses(&hold_for(&mut e, &p, 1.0));
    assert!((3..=4).contains(&pressed), "300 ms gap: {pressed} presses in a second");
    run(&mut e, &p, InputEvent::Button(Button::West, false));

    // Asked for 50 ms, but the macro takes 100: about ten presses a second.
    let mut e = macro_engine(vec![MacroStep::Tap { action: ButtonAction::Mouse(MouseButton::Left), hold_ms: 100 }]);
    p.set_button(Button::West, ButtonAction::Turbo { action: Box::new(looping), rate: 10.0, every_ms: 50 });
    let mut pressed = presses(&run(&mut e, &p, InputEvent::Button(Button::West, true)));
    pressed += presses(&hold_for(&mut e, &p, 1.0));
    assert!((9..=11).contains(&pressed), "50 ms gap under a 100 ms macro: {pressed} presses in a second");
}

#[test]
fn toggled_turbo_keeps_firing_after_release_until_toggled_off() {
    let mut p = Profile::passthrough("p");
    p.set_button(Button::West, toggle(turbo(ButtonAction::Mouse(MouseButton::Left), 10.0)));
    let mut e = Engine::default();
    run(&mut e, &p, InputEvent::Button(Button::West, true));
    run(&mut e, &p, InputEvent::Button(Button::West, false));
    assert!(clicks(&hold_for(&mut e, &p, 0.5)).len() >= 9, "auto-fire continues hands-off");
    let mut out = run(&mut e, &p, InputEvent::Button(Button::West, true));
    out.extend(hold_for(&mut e, &p, 0.5));
    assert!(clicks(&out).iter().all(|down| !down), "only a final release: {out:?}");
    assert!(!e.needs_tick(&p));
}

#[test]
fn toggle_inside_multi_keeps_its_state_across_reverse_release() {
    let mut p = Profile::passthrough("p");
    p.set_button(
        Button::South,
        ButtonAction::Multi(vec![ButtonAction::Keys(vec!["KEY_LEFTSHIFT".into()]), toggle(c_key())]),
    );
    let mut e = Engine::default();
    let shift = KeyCode::KEY_LEFTSHIFT;
    assert_eq!(
        run(&mut e, &p, InputEvent::Button(Button::South, true)),
        vec![OutEvent::Key(shift, true), OutEvent::Key(KeyCode::KEY_C, true)]
    );
    assert_eq!(run(&mut e, &p, InputEvent::Button(Button::South, false)), vec![OutEvent::Key(shift, false)]);
    assert_eq!(
        run(&mut e, &p, InputEvent::Button(Button::South, true)),
        vec![OutEvent::Key(shift, true), OutEvent::Key(KeyCode::KEY_C, false)]
    );
}

#[test]
fn profile_switch_releases_toggles_and_stops_turbo() {
    let mut p = Profile::passthrough("p");
    p.set_button(Button::North, toggle(c_key()));
    p.set_button(Button::West, turbo(ButtonAction::Mouse(MouseButton::Left), 10.0));
    let mut e = Engine::default();
    run(&mut e, &p, InputEvent::Button(Button::North, true));
    run(&mut e, &p, InputEvent::Button(Button::North, false));
    run(&mut e, &p, InputEvent::Button(Button::West, true));
    let mut out = Vec::new();
    e.release_all(false, &mut out);
    assert!(out.contains(&OutEvent::Key(KeyCode::KEY_C, false)), "{out:?}");
    assert!(out.contains(&OutEvent::MouseButton(MouseButton::Left, false)), "{out:?}");
    assert!(!e.needs_tick(&p));
}

#[test]
fn double_tap_can_toggle() {
    let mut p = Profile::passthrough("p");
    p.gestures.insert(
        Button::East,
        crate::config::Gestures { double_tap: Some(toggle(c_key())), ..Default::default() },
    );
    let mut e = Engine::default();
    let t0 = Instant::now();
    let mut out = Vec::new();
    for (down, at) in [(true, 0), (false, 50), (true, 120)] {
        out.extend(press(&mut e, &p, Button::East, down, ms(t0, at)));
    }
    assert_eq!(out, vec![OutEvent::Key(KeyCode::KEY_C, true)]);
    // Releasing the double tap leaves the toggle on.
    assert!(press(&mut e, &p, Button::East, false, ms(t0, 200)).is_empty());
}

#[test]
fn angle_difference_wraps() {
    assert!((angle_diff(170.0, -170.0) + 20.0).abs() < 1e-4);
    assert!((angle_diff(-170.0, 170.0) - 20.0).abs() < 1e-4);
}

use crate::config::{Macro, MacroStep};

fn key(k: &str) -> ButtonAction {
    ButtonAction::Keys(vec![k.into()])
}

fn macro_engine(steps: Vec<MacroStep>) -> Engine {
    let mut e = Engine::default();
    e.set_macros(&[Macro { name: "m".into(), steps }]);
    e
}

fn mapped(repeat: bool) -> Profile {
    let mut p = Profile::passthrough("p");
    p.set_button(Button::West, ButtonAction::Macro { name: "m".into(), repeat });
    p
}

/// Key events with the time (ms since the press) they were sent.
fn timeline(e: &mut Engine, p: &Profile, seconds: f32, release_at: Option<f32>) -> Vec<(u32, OutEvent)> {
    let mut events: Vec<(u32, OutEvent)> =
        run(e, p, InputEvent::Button(Button::West, true)).into_iter().map(|ev| (0, ev)).collect();
    let steps = (seconds / 0.004).round() as u32;
    for i in 1..=steps {
        let mut out = Vec::new();
        if release_at.is_some_and(|r| (i as f32 * 0.004 - r).abs() < 0.002) {
            out.extend(run(e, p, InputEvent::Button(Button::West, false)));
        }
        e.tick(p, 0.004, &mut out);
        events.extend(out.into_iter().map(|ev| (i * 4, ev)));
    }
    events
}

#[test]
fn macro_plays_its_sequence_with_timing() {
    let mut e = macro_engine(vec![
        MacroStep::Tap { action: key("KEY_A"), hold_ms: 40 },
        MacroStep::Wait(100),
        MacroStep::Press(key("KEY_LEFTSHIFT")),
        MacroStep::Tap { action: ButtonAction::Mouse(MouseButton::Left), hold_ms: 20 },
        MacroStep::Release(key("KEY_LEFTSHIFT")),
    ]);
    let p = mapped(false);
    let events = timeline(&mut e, &p, 0.5, Some(0.01));
    let shift = KeyCode::KEY_LEFTSHIFT;
    assert_eq!(
        events,
        vec![
            (0, OutEvent::Key(KeyCode::KEY_A, true)),
            (40, OutEvent::Key(KeyCode::KEY_A, false)),
            (140, OutEvent::Key(shift, true)),
            (140, OutEvent::MouseButton(MouseButton::Left, true)),
            (160, OutEvent::MouseButton(MouseButton::Left, false)),
            (160, OutEvent::Key(shift, false)),
        ],
        "releasing the button early must not cut a play-once macro short"
    );
    assert!(!e.needs_tick(&p), "finished macros stop ticking");
}

#[test]
fn repeating_macro_loops_while_held_and_cleans_up_on_release() {
    let mut e = macro_engine(vec![
        MacroStep::Press(key("KEY_LEFTSHIFT")),
        MacroStep::Tap { action: key("KEY_A"), hold_ms: 50 },
        MacroStep::Wait(50),
    ]);
    let p = mapped(true);
    let events = timeline(&mut e, &p, 1.0, Some(0.32));
    let a_presses: Vec<u32> = events
        .iter()
        .filter(|(_, ev)| *ev == OutEvent::Key(KeyCode::KEY_A, true))
        .map(|(t, _)| *t)
        .collect();
    assert_eq!(a_presses, [0, 100, 200, 300], "loops every 100 ms until released at 320 ms");
    // Shift is never released by a step, so each pass lets go of it at its end.
    let shift_downs = events.iter().filter(|(_, ev)| *ev == OutEvent::Key(KeyCode::KEY_LEFTSHIFT, true)).count();
    let shift_ups = events.iter().filter(|(_, ev)| *ev == OutEvent::Key(KeyCode::KEY_LEFTSHIFT, false)).count();
    assert_eq!((shift_downs, shift_ups), (4, 4), "{events:?}");
    // Releasing at 320 ms, mid-tap, lets go of A and Shift together, and nothing follows.
    let after: Vec<&OutEvent> = events.iter().filter(|(t, _)| *t >= 320).map(|(_, ev)| ev).collect();
    assert_eq!(after, [&OutEvent::Key(KeyCode::KEY_A, false), &OutEvent::Key(KeyCode::KEY_LEFTSHIFT, false)]);
    assert!(!e.needs_tick(&p));
}

#[test]
fn toggled_repeating_macro_and_profile_switch_stop() {
    let mut e = macro_engine(vec![MacroStep::Tap { action: key("KEY_A"), hold_ms: 10 }, MacroStep::Wait(40)]);
    let mut p = Profile::passthrough("p");
    p.set_button(Button::West, toggle(ButtonAction::Macro { name: "m".into(), repeat: true }));
    run(&mut e, &p, InputEvent::Button(Button::West, true));
    run(&mut e, &p, InputEvent::Button(Button::West, false));
    let presses = |out: &[OutEvent]| out.iter().filter(|ev| **ev == OutEvent::Key(KeyCode::KEY_A, true)).count();
    assert_eq!(presses(&hold_for(&mut e, &p, 0.5)), 10, "keeps looping hands-off");
    let mut out = Vec::new();
    e.release_all(false, &mut out);
    assert!(!e.needs_tick(&p));
    assert_eq!(presses(&hold_for(&mut e, &p, 0.5)), 0);
}

#[test]
fn repeating_macro_without_waits_cannot_hang() {
    let mut e = macro_engine(vec![MacroStep::Press(key("KEY_A")), MacroStep::Release(key("KEY_A"))]);
    let p = mapped(true);
    run(&mut e, &p, InputEvent::Button(Button::West, true));
    // One pass per tick rather than an infinite loop.
    let out = hold_for(&mut e, &p, 0.04);
    assert_eq!(out.iter().filter(|ev| **ev == OutEvent::Key(KeyCode::KEY_A, true)).count(), 10);
}

#[test]
fn unknown_macro_does_nothing() {
    let mut e = Engine::default();
    assert!(run(&mut e, &mapped(false), InputEvent::Button(Button::West, true)).is_empty());
}

#[test]
fn stick_directions_are_buttons_with_any_action_alongside_the_stick() {
    // The right stick stays a gamepad stick; its directions also act as buttons.
    let mut p = Profile::passthrough("p");
    p.right_stick.deadzone = 0.0;
    p.right_stick.key_threshold = 0.5;
    p.set_button(Button::RightStickUp, toggle(key("KEY_C")));
    p.set_button(Button::RightStickDown, key("KEY_S"));
    p.set_button(Button::RightStickLeft, ButtonAction::Mouse(MouseButton::Left));
    let mut e = Engine::default();
    let c_down = OutEvent::Key(KeyCode::KEY_C, true);
    let stick_y = |v: f32| OutEvent::PadAxis(Axis::RightY, v);

    // Up is a toggle: flicking up and back turns it on, the next flick turns it off.
    assert_eq!(axis(&mut e, &p, Axis::RightY, -0.9), vec![OutEvent::PadAxis(Axis::RightX, 0.0), stick_y(-0.9), c_down]);
    assert!(!axis(&mut e, &p, Axis::RightY, -0.47).iter().any(|ev| matches!(ev, OutEvent::Key(..))), "inside the release margin");
    axis(&mut e, &p, Axis::RightY, 0.0);
    assert!(axis(&mut e, &p, Axis::RightY, -0.9).contains(&OutEvent::Key(KeyCode::KEY_C, false)));
    axis(&mut e, &p, Axis::RightY, 0.0);

    // A diagonal presses both directions; returning to center releases both.
    axis(&mut e, &p, Axis::RightX, -0.7);
    assert!(axis(&mut e, &p, Axis::RightY, 0.7).contains(&OutEvent::Key(KeyCode::KEY_S, true)));
    let mut out = axis(&mut e, &p, Axis::RightX, 0.0);
    out.extend(axis(&mut e, &p, Axis::RightY, 0.0));
    assert!(out.contains(&OutEvent::MouseButton(MouseButton::Left, false)), "{out:?}");
    assert!(out.contains(&OutEvent::Key(KeyCode::KEY_S, false)), "{out:?}");
}

#[test]
fn bumper_plus_stick_direction_combo() {
    let mut p = Profile::passthrough("p");
    p.right_stick.deadzone = 0.0;
    p.combos.push(Combo {
        buttons: vec![Button::LeftBumper, Button::RightStickRight],
        action: key("KEY_F"),
    });
    let mut e = Engine::default();
    let t0 = Instant::now();
    // Holding right on the stick (its own action is Disabled, so it waits as a modifier)...
    assert!(!axis(&mut e, &p, Axis::RightX, 1.0).iter().any(|ev| matches!(ev, OutEvent::Key(..))));
    // ...then pressing LB well afterwards fires the combo instead of LB's own action.
    assert_eq!(press(&mut e, &p, Button::LeftBumper, true, ms(t0, 2000)), vec![OutEvent::Key(KeyCode::KEY_F, true)]);
    assert_eq!(press(&mut e, &p, Button::LeftBumper, false, ms(t0, 2100)), vec![OutEvent::Key(KeyCode::KEY_F, false)]);
    // LB alone still works as a bumper.
    axis(&mut e, &p, Axis::RightX, 0.0);
    press(&mut e, &p, Button::LeftBumper, true, ms(t0, 3000));
    assert_eq!(fire_timers(&mut e, &p, ms(t0, 3100)), vec![OutEvent::PadButton(Button::LeftBumper, true)]);
}

#[test]
fn buttons_can_push_a_stick_direction() {
    let mut p = Profile::passthrough("p");
    p.left_stick.deadzone = 0.0;
    p.set_button(Button::DpadRight, ButtonAction::Gamepad(Button::LeftStickRight));
    p.set_button(Button::DpadUp, ButtonAction::Gamepad(Button::LeftStickUp));
    let mut e = Engine::default();
    let last_stick = |out: &[OutEvent]| -> (f32, f32) {
        let x = out.iter().rev().find_map(|ev| match ev { OutEvent::PadAxis(Axis::LeftX, v) => Some(*v), _ => None });
        let y = out.iter().rev().find_map(|ev| match ev { OutEvent::PadAxis(Axis::LeftY, v) => Some(*v), _ => None });
        (x.unwrap(), y.unwrap())
    };
    assert_eq!(last_stick(&run(&mut e, &p, InputEvent::Button(Button::DpadRight, true))), (1.0, 0.0));
    // Two directions make a diagonal on the stick's circle, not a square corner.
    let (x, y) = last_stick(&run(&mut e, &p, InputEvent::Button(Button::DpadUp, true)));
    let d = std::f32::consts::FRAC_1_SQRT_2;
    assert!((x - d).abs() < 1e-4 && (y + d).abs() < 1e-4, "{x} {y}");
    assert_eq!(last_stick(&run(&mut e, &p, InputEvent::Button(Button::DpadRight, false))), (0.0, -1.0));
    assert_eq!(last_stick(&run(&mut e, &p, InputEvent::Button(Button::DpadUp, false))), (0.0, 0.0));
    // Pushing against the physical stick adds, limited to full deflection.
    axis(&mut e, &p, Axis::LeftX, 0.5);
    assert_eq!(last_stick(&run(&mut e, &p, InputEvent::Button(Button::DpadRight, true))), (1.0, 0.0));
}

#[test]
fn macro_stick_steps_trace_a_motion_and_recenter() {
    let d = std::f32::consts::FRAC_1_SQRT_2;
    let mut e = macro_engine(vec![
        MacroStep::Stick { stick: Stick::Left, x: 0.0, y: 1.0 },
        MacroStep::Wait(16),
        MacroStep::Stick { stick: Stick::Left, x: d, y: d },
        MacroStep::Wait(16),
        MacroStep::Stick { stick: Stick::Left, x: 1.0, y: 0.0 },
        MacroStep::Tap { action: ButtonAction::Gamepad(Button::West), hold_ms: 16 },
    ]);
    let mut p = mapped(false);
    p.left_stick.deadzone = 0.0;
    let events = timeline(&mut e, &p, 0.2, None);
    let sticks: Vec<(u32, (Axis, f32))> = events
        .iter()
        .filter_map(|(t, ev)| match ev {
            OutEvent::PadAxis(a @ (Axis::LeftX | Axis::LeftY), v) => Some((*t, (*a, (v * 100.0).round() / 100.0))),
            _ => None,
        })
        .collect();
    assert_eq!(
        sticks,
        vec![
            (0, (Axis::LeftX, 0.0)),
            (0, (Axis::LeftY, 1.0)),
            (16, (Axis::LeftX, 0.71)),
            (16, (Axis::LeftY, 0.71)),
            (32, (Axis::LeftX, 1.0)),
            (32, (Axis::LeftY, 0.0)),
            // The pass ends 16 ms later and the stick recenters.
            (48, (Axis::LeftX, 0.0)),
            (48, (Axis::LeftY, 0.0)),
        ]
    );
    let punch: Vec<u32> = events
        .iter()
        .filter(|(_, ev)| matches!(ev, OutEvent::PadButton(Button::West, _)))
        .map(|(t, _)| *t)
        .collect();
    assert_eq!(punch, [32, 48], "pressed with the forward input, released 16 ms later");
}

#[test]
fn macro_stick_adds_to_the_physical_stick() {
    let mut e = macro_engine(vec![MacroStep::Stick { stick: Stick::Left, x: 0.5, y: 0.0 }, MacroStep::Wait(100)]);
    let mut p = mapped(false);
    p.left_stick.deadzone = 0.0;
    axis(&mut e, &p, Axis::LeftX, 0.25);
    let out = run(&mut e, &p, InputEvent::Button(Button::West, true));
    assert!(out.contains(&OutEvent::PadAxis(Axis::LeftX, 0.75)), "{out:?}");
}

#[test]
fn show_info_holds_while_pressed_and_toggles() {
    let mut p = Profile::passthrough("p");
    p.set_button(Button::Select, ButtonAction::ShowInfo("Keys".into()));
    p.set_button(Button::North, toggle(ButtonAction::ShowInfo("Stats".into())));
    let mut e = Engine::default();
    run(&mut e, &p, InputEvent::Button(Button::Select, true));
    assert!(e.take_info_changed());
    assert_eq!(e.shown_info().collect::<Vec<_>>(), ["Keys"]);
    run(&mut e, &p, InputEvent::Button(Button::Select, false));
    assert_eq!(e.shown_info().count(), 0);
    run(&mut e, &p, InputEvent::Button(Button::North, true));
    run(&mut e, &p, InputEvent::Button(Button::North, false));
    assert_eq!(e.shown_info().collect::<Vec<_>>(), ["Stats"], "toggled on stays up");
    run(&mut e, &p, InputEvent::Button(Button::North, true));
    assert_eq!(e.shown_info().count(), 0);
}

#[test]
fn open_menu_reports_its_opener_and_menu_items_tap() {
    let mut p = Profile::passthrough("p");
    p.set_button(Button::Select, ButtonAction::OpenMenu("Pause".into()));
    p.left_trigger = TriggerAction::Button { action: ButtonAction::OpenMenu("Wheel".into()), threshold: 0.5 }.into();
    let mut e = Engine::default();
    run(&mut e, &p, InputEvent::Button(Button::Select, true));
    assert_eq!(e.take_menu_request(), Some(("Pause".into(), Opener { buttons: vec![Button::Select], ..Opener::default() })));
    assert_eq!(e.take_menu_request(), None);
    axis(&mut e, &p, Axis::LeftTrigger, 1.0);
    assert_eq!(e.take_menu_request().map(|(n, o)| (n, o.trigger)), Some(("Wheel".into(), Some(Trigger::Left))));

    let mut out = Vec::new();
    let toggle_c = toggle(key("KEY_C"));
    e.tap_menu_item("Pause", 0, &key("KEY_M"), &mut out);
    assert_eq!(out, vec![OutEvent::Key(KeyCode::KEY_M, true), OutEvent::Key(KeyCode::KEY_M, false)]);
    out.clear();
    e.tap_menu_item("Pause", 1, &toggle_c, &mut out);
    e.tap_menu_item("Pause", 1, &toggle_c, &mut out);
    assert_eq!(out, vec![OutEvent::Key(KeyCode::KEY_C, true), OutEvent::Key(KeyCode::KEY_C, false)], "a toggle item flips each time");
}

#[test]
fn toggles_set_to_start_on_switch_on_when_the_game_starts() {
    let starting = |inner| ButtonAction::Toggle(Toggled { action: Box::new(inner), start_on: true });
    let mut p = Profile::passthrough("p");
    p.set_button(Button::North, starting(ButtonAction::ShowInfo("Controls".into())));
    // Inside a Multi, after a Turbo (which takes a slot of its own).
    p.set_button(
        Button::West,
        ButtonAction::Multi(vec![
            ButtonAction::Turbo { action: Box::new(ButtonAction::Mouse(MouseButton::Left)), rate: 5.0, every_ms: 0 },
            starting(ButtonAction::Keys(vec!["KEY_C".into()])),
        ]),
    );
    p.combos.push(Combo { buttons: vec![Button::RightBumper, Button::LeftBumper], action: starting(ButtonAction::Layer("L".into())) });
    let menu = Menu {
        name: "M".into(),
        kind: crate::config::MenuKind::List,
        items: vec![crate::config::MenuItem::new("x", starting(ButtonAction::Keys(vec!["KEY_M".into()])))],
        cancel: None, no_cancel: false, keep_open: false, start: 0,
        style: Default::default(),
    };
    let mut e = Engine::default();
    let mut out = Vec::new();
    e.start_toggles(&p, std::slice::from_ref(&menu), &mut out);
    assert_eq!(e.shown_info().collect::<Vec<_>>(), ["Controls"]);
    assert_eq!(e.layers(), ["L"]);
    assert!(out.contains(&OutEvent::Key(KeyCode::KEY_C, true)) && out.contains(&OutEvent::Key(KeyCode::KEY_M, true)));
    // Again does nothing: they're on already.
    let mut again = Vec::new();
    e.start_toggles(&p, std::slice::from_ref(&menu), &mut again);
    assert!(again.is_empty(), "{again:?}");
    // Pressing the button turns it off, like any toggle, and the Multi's toggle too.
    run(&mut e, &p, InputEvent::Button(Button::North, true));
    assert_eq!(e.shown_info().count(), 0);
    let out = run(&mut e, &p, InputEvent::Button(Button::West, true));
    assert!(out.contains(&OutEvent::Key(KeyCode::KEY_C, false)), "{out:?}");
    // The menu item turns off when chosen.
    let mut out = Vec::new();
    e.tap_menu_item("M", 0, &menu.items[0].action, &mut out);
    assert_eq!(out, [OutEvent::Key(KeyCode::KEY_M, false)]);
}

// Layers: the engine keeps the stack; the daemon hands it the profile with them on top.

fn layered(base: &Profile, layer: &crate::config::Layer) -> Profile {
    base.with_layers([layer])
}

/// Presses and lets go of `b`.
fn tap(e: &mut Engine, p: &Profile, b: Button) {
    run(e, p, InputEvent::Button(b, true));
    run(e, p, InputEvent::Button(b, false));
}

#[test]
fn layers_stack_and_count_their_holders() {
    let mut p = Profile::passthrough("p");
    p.set_button(Button::LeftBumper, ButtonAction::Layer("A".into()));
    p.set_button(Button::RightBumper, ButtonAction::Layer("B".into()));
    p.set_button(Button::West, ButtonAction::Layer("A".into()));
    let mut e = Engine::default();
    run(&mut e, &p, InputEvent::Button(Button::LeftBumper, true));
    run(&mut e, &p, InputEvent::Button(Button::RightBumper, true));
    assert_eq!(e.layers(), ["A", "B"], "oldest first");
    assert!(e.take_layers_changed() && !e.take_layers_changed());
    run(&mut e, &p, InputEvent::Button(Button::West, true));
    run(&mut e, &p, InputEvent::Button(Button::LeftBumper, false));
    assert_eq!(e.layers(), ["A", "B"], "West still holds A");
    run(&mut e, &p, InputEvent::Button(Button::West, false));
    assert_eq!(e.layers(), ["B"]);
}

#[test]
fn one_mode_is_on_at_a_time_and_pressing_it_again_goes_back() {
    let mut p = Profile::passthrough("p");
    p.set_button(Button::North, ButtonAction::Mode("Driving".into()));
    p.set_button(Button::West, ButtonAction::Mode("Flying".into()));
    let mut e = Engine::default();
    assert_eq!(e.mode(), None, "the default mode");
    tap(&mut e, &p, Button::North);
    assert_eq!(e.mode(), Some("Driving"));
    assert!(e.take_layers_changed());
    tap(&mut e, &p, Button::West);
    assert_eq!(e.mode(), Some("Flying"), "switching replaces the mode that was on");
    tap(&mut e, &p, Button::West);
    assert_eq!(e.mode(), None, "the same mode again goes back to the default");
    assert!(e.layers().is_empty(), "modes aren't layers");
}

#[test]
fn a_built_in_mode_stands_in_for_the_chosen_one_and_gives_it_back() {
    let mut p = Profile::passthrough("p");
    p.set_button(Button::North, ButtonAction::Mode("Driving".into()));
    p.set_button(Button::West, ButtonAction::Mode(MOUSE_MODE.into()));
    p.set_button(Button::LeftBumper, ButtonAction::Layer("Lean".into()));
    let mut e = Engine::default();
    tap(&mut e, &p, Button::North);
    run(&mut e, &p, InputEvent::Button(Button::LeftBumper, true));
    assert_eq!((e.mode(), e.layers()), (Some("Driving"), vec!["Lean".to_string()]));
    // Holding Guide: the Guide mode, without the held layer, which comes back after.
    run(&mut e, &p, InputEvent::Button(Button::Guide, true));
    assert_eq!((e.mode(), e.layers()), (Some(GUIDE_MODE), Vec::new()));
    assert_eq!(e.chosen_mode(), Some("Driving"));
    run(&mut e, &p, InputEvent::Button(Button::Guide, false));
    tap(&mut e, &p, Button::Guide);
    assert_eq!((e.mode(), e.layers()), (Some("Driving"), vec!["Lean".to_string()]), "the overlay closed again");
    run(&mut e, &p, InputEvent::Button(Button::LeftBumper, false));
    // Switching to mouse mode stands in for Driving; Guide ends it, and Driving is back.
    tap(&mut e, &p, Button::West);
    assert_eq!(e.mode(), Some(MOUSE_MODE));
    tap(&mut e, &p, Button::Guide);
    assert_eq!(e.mode(), Some("Driving"));
    // Switching to another mode ends mouse mode too: only one is on.
    tap(&mut e, &p, Button::West);
    let mut out = Vec::new();
    e.tap_menu_item("M", 0, &ButtonAction::Mode("Flying".into()), &mut out);
    assert_eq!(e.mode(), Some("Flying"));
    // The Guide mode can't be switched to: it's the Guide button's.
    e.tap_menu_item("M", 1, &ButtonAction::Mode(GUIDE_MODE.into()), &mut out);
    assert_eq!(e.mode(), Some("Flying"));
}

#[test]
fn the_mode_survives_menus_but_not_game_changes() {
    let mut p = Profile::passthrough("p");
    p.set_button(Button::North, ButtonAction::Mode("A".into()));
    p.set_button(Button::LeftBumper, ButtonAction::Layer("B".into()));
    let mut e = Engine::default();
    tap(&mut e, &p, Button::North);
    run(&mut e, &p, InputEvent::Button(Button::LeftBumper, true));
    let mut out = Vec::new();
    e.release_all(true, &mut out);
    assert!(e.layers().is_empty(), "held layers end");
    assert_eq!(e.mode(), Some("A"), "the mode stays");
    // A menu item switches modes like a button.
    e.tap_menu_item("M", 0, &ButtonAction::Mode("A".into()), &mut out);
    assert_eq!(e.mode(), None);
    tap(&mut e, &p, Button::North);
    e.release_all(false, &mut out);
    assert_eq!(e.mode(), None);
}

#[test]
fn a_held_button_keeps_its_action_across_a_layer_change() {
    let base = Profile::passthrough("p");
    let mut layer = crate::config::Layer::new("L");
    layer.buttons.insert(Button::South, ButtonAction::Keys(vec!["KEY_F1".into()]));
    let on = layered(&base, &layer);
    let mut e = Engine::default();
    assert_eq!(run(&mut e, &base, InputEvent::Button(Button::South, true)), [OutEvent::PadButton(Button::South, true)]);
    // The layer comes on while A is down: A still releases the pad button.
    assert_eq!(run(&mut e, &on, InputEvent::Button(Button::South, false)), [OutEvent::PadButton(Button::South, false)]);
    assert_eq!(run(&mut e, &on, InputEvent::Button(Button::South, true)), [OutEvent::Key(KeyCode::KEY_F1, true)]);
}

#[test]
fn analog_inputs_switch_modes_without_getting_stuck() {
    let base = Profile::passthrough("p");
    let mut layer = crate::config::Layer::new("L");
    layer.left_stick = Some(StickConfig::new(StickAction::wasd(), 0.1, 1.0));
    layer.right_stick = Some(StickConfig::new(StickAction::mouse(1000.0), 0.1, 1.0));
    layer.right_trigger = Some(TriggerAction::Gamepad(Trigger::Right).into());
    let on = layered(&base, &layer);
    let mut e = Engine::default();

    // Right stick: pad stick, then mouse; the virtual stick recenters.
    axis(&mut e, &base, Axis::RightX, 0.8);
    let mut out = Vec::new();
    e.resync(&on, &mut out);
    assert!(out.contains(&OutEvent::PadAxis(Axis::RightX, 0.0)), "{out:?}");

    // Left stick: a ring in the layer sends W while the stick points up. Leaving the layer
    // releases it, and letting go of the stick afterwards doesn't release it again.
    let held = axis(&mut e, &on, Axis::LeftY, -0.9);
    assert!(held.contains(&OutEvent::Key(KeyCode::KEY_W, true)));
    let mut out = Vec::new();
    e.resync(&base, &mut out);
    assert!(out.contains(&OutEvent::Key(KeyCode::KEY_W, false)), "still pushed up: {out:?}");
    assert!(!axis(&mut e, &base, Axis::LeftY, 0.0).contains(&OutEvent::Key(KeyCode::KEY_W, false)));

    // A trigger pull pressed as a button releases by its own threshold after the layer
    // made the trigger analog, and the analog trigger doesn't stay pulled afterwards.
    let mut pulled = Profile::passthrough("p");
    pulled.right_trigger = TriggerAction::Button { action: ButtonAction::Mouse(MouseButton::Left), threshold: 0.5 }.into();
    let pulled_on = layered(&pulled, &layer);
    // (The virtual trigger the resync above fed under the layer goes back to rest first.)
    let out = axis(&mut e, &pulled, Axis::RightTrigger, 0.9);
    assert_eq!(out, [OutEvent::PadAxis(Axis::RightTrigger, 0.0), OutEvent::MouseButton(MouseButton::Left, true)]);
    let out = axis(&mut e, &pulled_on, Axis::RightTrigger, 0.2);
    assert!(out.contains(&OutEvent::MouseButton(MouseButton::Left, false)), "{out:?}");
    let out = axis(&mut e, &pulled, Axis::RightTrigger, 0.1);
    assert!(out.contains(&OutEvent::PadAxis(Axis::RightTrigger, 0.0)), "{out:?}");
}

#[test]
fn active_zones_release_by_their_own_bounds() {
    let mut base = Profile::passthrough("p");
    base.left_stick.zones.push(Zone { min: 0.0, max: 0.5, action: ButtonAction::Keys(vec!["KEY_LEFTSHIFT".into()]) });
    let mut layer = crate::config::Layer::new("L");
    let mut stick = base.left_stick.clone();
    stick.zones = vec![Zone { min: 0.6, max: 1.0, action: ButtonAction::Keys(vec!["KEY_X".into()]) }];
    layer.left_stick = Some(stick);
    let on = layered(&base, &layer);
    let mut e = Engine::default();
    assert!(axis(&mut e, &base, Axis::LeftX, 0.3).contains(&OutEvent::Key(KeyCode::KEY_LEFTSHIFT, true)));
    // Under the layer, zone 0 means something else; Shift stays until 0.5 is passed.
    let out = axis(&mut e, &on, Axis::LeftX, 0.4);
    assert!(!out.iter().any(|o| matches!(o, OutEvent::Key(..))), "{out:?}");
    // Past it, Shift releases and the layer's zone presses, in that order.
    let out = axis(&mut e, &on, Axis::LeftX, 0.8);
    let keys: Vec<&OutEvent> = out.iter().filter(|o| matches!(o, OutEvent::Key(..))).collect();
    assert_eq!(keys, [&OutEvent::Key(KeyCode::KEY_LEFTSHIFT, false), &OutEvent::Key(KeyCode::KEY_X, true)]);
}

#[test]
fn a_trigger_pressing_at_the_lowest_setting_lets_go_at_rest() {
    let mut p = Profile::passthrough("p");
    p.right_trigger = TriggerAction::Button { action: ButtonAction::Mouse(MouseButton::Left), threshold: 0.05 }.into();
    let mut e = Engine::default();
    assert_eq!(run(&mut e, &p, InputEvent::Axis(Axis::RightTrigger, 0.5)), [OutEvent::MouseButton(MouseButton::Left, true)]);
    assert_eq!(run(&mut e, &p, InputEvent::Axis(Axis::RightTrigger, 0.0)), [OutEvent::MouseButton(MouseButton::Left, false)]);
}

#[test]
fn a_stick_direction_pressing_at_the_lowest_setting_lets_go_at_rest() {
    let mut p = Profile::passthrough("p");
    p.set_button(Button::LeftStickUp, ButtonAction::Keys(vec!["KEY_W".into()]));
    p.left_stick.key_threshold = 0.05;
    let mut e = Engine::default();
    let keys = |out: Vec<OutEvent>| out.into_iter().filter(|o| matches!(o, OutEvent::Key(..))).collect::<Vec<_>>();
    assert_eq!(keys(run(&mut e, &p, InputEvent::Axis(Axis::LeftY, -1.0))), [OutEvent::Key(KeyCode::KEY_W, true)]);
    assert_eq!(keys(run(&mut e, &p, InputEvent::Axis(Axis::LeftY, 0.0))), [OutEvent::Key(KeyCode::KEY_W, false)]);
}

#[test]
fn nothing_presses_at_rest_whatever_the_threshold() {
    let mut p = Profile::passthrough("p");
    p.right_trigger = TriggerAction::Button { action: ButtonAction::Mouse(MouseButton::Left), threshold: 0.0 }.into();
    p.set_button(Button::LeftStickUp, ButtonAction::Keys(vec!["KEY_W".into()]));
    p.set_button(Button::LeftStickDown, ButtonAction::Keys(vec!["KEY_S".into()]));
    p.left_stick.key_threshold = 0.0;
    let mut e = Engine::default();
    let mut out = run(&mut e, &p, InputEvent::Axis(Axis::RightTrigger, 0.0));
    out.extend(run(&mut e, &p, InputEvent::Axis(Axis::LeftY, 0.0)));
    out.retain(|o| !matches!(o, OutEvent::PadAxis(..)));
    assert!(out.is_empty(), "{out:?}");
}

#[test]
fn a_macro_that_plays_itself_is_refused_rather_than_crashing() {
    let mut p = Profile::passthrough("p");
    p.set_button(Button::West, ButtonAction::Macro { name: "a".into(), repeat: false });
    p.set_button(Button::North, ButtonAction::Macro { name: "loop".into(), repeat: false });
    p.set_button(Button::East, ButtonAction::Macro { name: "fine".into(), repeat: false });
    let plays = |name: &str| ButtonAction::Macro { name: name.into(), repeat: false };
    let key = ButtonAction::Keys(vec!["KEY_K".into()]);
    let mut e = Engine::default();
    e.set_macros(&[
        Macro { name: "loop".into(), steps: vec![MacroStep::Tap { action: crate::config::ButtonAction::toggle(plays("loop")), hold_ms: 0 }] },
        Macro { name: "a".into(), steps: vec![MacroStep::Press(key.clone()), MacroStep::Press(plays("b"))] },
        Macro { name: "b".into(), steps: vec![MacroStep::Press(plays("a"))] },
        // Playing another macro is fine.
        Macro { name: "fine".into(), steps: vec![MacroStep::Press(plays("inner"))] },
        Macro { name: "inner".into(), steps: vec![MacroStep::Press(key)] },
    ]);
    assert!(run(&mut e, &p, InputEvent::Button(Button::North, true)).is_empty());
    assert!(run(&mut e, &p, InputEvent::Button(Button::West, true)).is_empty());
    assert_eq!(run(&mut e, &p, InputEvent::Button(Button::East, true)), key_tap(KeyCode::KEY_K));
}

#[test]
fn the_face_swap_holds_for_a_face_button_with_gestures() {
    let mut p = Profile::passthrough("p");
    p.gestures.insert(Button::South, crate::config::Gestures { double_tap: Some(ButtonAction::Keys(vec!["KEY_2".into()])), ..Default::default() });
    let mut e = Engine::default();
    e.set_face_swap(true);
    let t0 = Instant::now();
    press(&mut e, &p, Button::South, true, t0);
    assert_eq!(fire_timers(&mut e, &p, ms(t0, 1000)), [OutEvent::PadButton(Button::East, true)], "held past the tap window");
    assert_eq!(press(&mut e, &p, Button::South, false, ms(t0, 1100)), [OutEvent::PadButton(Button::East, false)]);
    // A single tap, decided once the window runs out.
    press(&mut e, &p, Button::South, true, ms(t0, 2000));
    press(&mut e, &p, Button::South, false, ms(t0, 2050));
    assert_eq!(fire_timers(&mut e, &p, ms(t0, 3000)), [OutEvent::PadButton(Button::East, true), OutEvent::PadButton(Button::East, false)]);
}
