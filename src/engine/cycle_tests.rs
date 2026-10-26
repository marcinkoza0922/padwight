use super::*;

fn key(name: &str) -> ButtonAction {
    ButtonAction::Keys(vec![name.into()])
}

/// An engine knowing a "Speed" cycle of keys 1, 2 and 3, and a profile stepping it forward on
/// West and back on North.
fn speed() -> (Engine, Profile) {
    let mut e = Engine::default();
    e.set_cycles(&[Cycle { name: "Speed".into(), steps: vec![key("KEY_1"), key("KEY_2"), key("KEY_3")] }]);
    let mut p = Profile::passthrough("p");
    p.set_button(Button::West, ButtonAction::Cycle { name: "Speed".into(), back: false });
    p.set_button(Button::North, ButtonAction::Cycle { name: "Speed".into(), back: true });
    (e, p)
}

fn run(e: &mut Engine, p: &Profile, button: Button, pressed: bool) -> Vec<OutEvent> {
    let mut out = Vec::new();
    e.handle(p, InputEvent::Button(button, pressed), Instant::now(), &mut out);
    out
}

/// The key a full press and release of `button` taps.
fn tap(e: &mut Engine, p: &Profile, button: Button) -> KeyCode {
    let down = run(e, p, button, true);
    let [OutEvent::Key(k, true)] = down[..] else { panic!("pressed {down:?}") };
    assert_eq!(run(e, p, button, false), [OutEvent::Key(k, false)]);
    k
}

#[test]
fn each_press_presses_the_next_step_and_starts_over_after_the_last() {
    let (mut e, p) = speed();
    let keys: Vec<KeyCode> = (0..4).map(|_| tap(&mut e, &p, Button::West)).collect();
    assert_eq!(keys, [KeyCode::KEY_1, KeyCode::KEY_2, KeyCode::KEY_3, KeyCode::KEY_1]);
}

#[test]
fn stepping_back_goes_the_other_way_from_the_same_place() {
    let (mut e, p) = speed();
    assert_eq!(tap(&mut e, &p, Button::North), KeyCode::KEY_3, "back from the start is the last step");
    assert_eq!(tap(&mut e, &p, Button::West), KeyCode::KEY_1);
    assert_eq!(tap(&mut e, &p, Button::West), KeyCode::KEY_2);
    assert_eq!(tap(&mut e, &p, Button::North), KeyCode::KEY_1);
}

#[test]
fn a_step_is_held_while_the_input_is_and_let_go_on_a_profile_switch() {
    let (mut e, p) = speed();
    assert_eq!(run(&mut e, &p, Button::West, true), [OutEvent::Key(KeyCode::KEY_1, true)]);
    let mut out = Vec::new();
    e.tick(&p, 0.5, &mut out);
    assert!(out.is_empty(), "nothing repeats while held");
    e.release_all(false, &mut out);
    assert_eq!(out.iter().filter(|ev| matches!(ev, OutEvent::Key(..))).collect::<Vec<_>>(), [&OutEvent::Key(KeyCode::KEY_1, false)]);
    assert!(run(&mut e, &p, Button::West, false).is_empty());
    assert_eq!(tap(&mut e, &p, Button::West), KeyCode::KEY_2);
}

#[test]
fn several_inputs_in_a_step_press_together() {
    let mut e = Engine::default();
    let both = ButtonAction::Multi(vec![key("KEY_LEFTCTRL"), ButtonAction::Gamepad(Button::South)]);
    e.set_cycles(&[Cycle { name: "c".into(), steps: vec![both, key("KEY_X")] }]);
    let mut p = Profile::passthrough("p");
    p.set_button(Button::West, ButtonAction::Cycle { name: "c".into(), back: false });
    assert_eq!(run(&mut e, &p, Button::West, true), [OutEvent::Key(KeyCode::KEY_LEFTCTRL, true), OutEvent::PadButton(Button::South, true)]);
    assert_eq!(run(&mut e, &p, Button::West, false), [OutEvent::PadButton(Button::South, false), OutEvent::Key(KeyCode::KEY_LEFTCTRL, false)]);
}

#[test]
fn a_step_can_play_a_macro() {
    let mut e = Engine::default();
    e.set_macros(&[Macro { name: "m".into(), steps: vec![MacroStep::Tap { action: key("KEY_A"), hold_ms: 0 }] }]);
    e.set_cycles(&[Cycle { name: "c".into(), steps: vec![ButtonAction::Macro { name: "m".into(), repeat: false }, key("KEY_B")] }]);
    let mut p = Profile::passthrough("p");
    p.set_button(Button::West, ButtonAction::Cycle { name: "c".into(), back: false });
    assert_eq!(run(&mut e, &p, Button::West, true), [OutEvent::Key(KeyCode::KEY_A, true), OutEvent::Key(KeyCode::KEY_A, false)]);
    run(&mut e, &p, Button::West, false);
    assert_eq!(tap(&mut e, &p, Button::West), KeyCode::KEY_B);
}

#[test]
fn the_game_starting_puts_cycles_back_at_their_start() {
    let (mut e, p) = speed();
    tap(&mut e, &p, Button::West);
    e.reset_cycles();
    assert_eq!(tap(&mut e, &p, Button::West), KeyCode::KEY_1);
}

#[test]
fn a_cycle_keeps_its_place_until_its_steps_change() {
    let (mut e, p) = speed();
    tap(&mut e, &p, Button::West);
    let same = Cycle { name: "Speed".into(), steps: vec![key("KEY_1"), key("KEY_2"), key("KEY_3")] };
    e.set_cycles(std::slice::from_ref(&same));
    assert_eq!(tap(&mut e, &p, Button::West), KeyCode::KEY_2, "a config reload that left it alone");
    e.set_cycles(&[Cycle { steps: vec![key("KEY_4"), key("KEY_5")], ..same }]);
    assert_eq!(tap(&mut e, &p, Button::West), KeyCode::KEY_4, "edited, it starts over");
}

#[test]
fn a_missing_or_empty_cycle_and_a_step_that_isnt_an_input_do_nothing() {
    let mut e = Engine::default();
    let nested = ButtonAction::Cycle { name: "loop".into(), back: false };
    e.set_cycles(&[Cycle { name: "empty".into(), steps: vec![] }, Cycle { name: "loop".into(), steps: vec![nested.clone()] }]);
    let mut p = Profile::passthrough("p");
    for (b, name) in [(Button::West, "gone"), (Button::North, "empty"), (Button::East, "loop")] {
        p.set_button(b, ButtonAction::Cycle { name: name.into(), back: false });
        assert!(run(&mut e, &p, b, true).is_empty(), "{name}");
        assert!(run(&mut e, &p, b, false).is_empty(), "{name}");
    }
}
